use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use tempfile::tempdir;

use crate::contracts::{
    generate_real_sdl_config, generate_sdl_revision_header, load_original_test_object_manifest,
    load_original_test_port_map, load_standalone_test_manifest, PortCompletionState,
    UBUNTU_MULTIARCH,
};

const DEFAULT_CTEST_TARGETS: &[&str] = &["testatomic", "testplatform", "testqsort"];
const HEADLESS_ORIGINAL_SUITE_UNSUPPORTED_TARGETS: &[&str] = &[
    "testautomation",
    "testlocale",
    "testkeys",
    "testbounds",
    "testdisplayinfo",
];
const ORIGINAL_SUITE_TIMEOUT: Duration = Duration::from_secs(120);

pub struct CompileOriginalTestObjectsArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub object_manifest: Option<PathBuf>,
    pub output_dir: PathBuf,
}

pub struct RelinkOriginalTestObjectsArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub object_manifest: Option<PathBuf>,
    pub standalone_manifest: Option<PathBuf>,
    pub objects_dir: PathBuf,
    pub output_dir: PathBuf,
    pub library_path: PathBuf,
}

pub struct RunRelinkedOriginalTestsArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub standalone_manifest: PathBuf,
    pub bin_dir: PathBuf,
    pub filter: Option<String>,
    pub validation_modes: Vec<String>,
    pub skip_if_empty: bool,
}

pub struct BuildOriginalStandaloneArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub standalone_manifest: PathBuf,
    pub stage_root: PathBuf,
    pub build_dir: PathBuf,
    pub phase: String,
}

pub struct RunOriginalStandaloneArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub standalone_manifest: PathBuf,
    pub build_dir: PathBuf,
    pub phase: String,
    pub validation_mode: String,
    pub skip_if_empty: bool,
}

pub struct RunFixtureBackedOriginalTestsArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub standalone_manifest: PathBuf,
    pub build_dir: PathBuf,
    pub phase: String,
    pub skip_if_empty: bool,
}

pub struct BuildOriginalCmakeSuiteArgs {
    pub repo_root: PathBuf,
    pub original_dir: PathBuf,
    pub stage_root: PathBuf,
    pub build_dir: PathBuf,
}

pub struct RunOriginalCtestArgs {
    pub repo_root: PathBuf,
    pub build_dir: PathBuf,
    pub stage_root: Option<PathBuf>,
    pub filter: Option<String>,
    pub test_list: Option<String>,
}

pub struct BuildOriginalAutotoolsSuiteArgs {
    pub repo_root: PathBuf,
    pub original_dir: PathBuf,
    pub stage_root: PathBuf,
    pub build_dir: PathBuf,
}

pub struct RunOriginalAutotoolsCheckArgs {
    pub repo_root: PathBuf,
    pub stage_root: Option<PathBuf>,
    pub build_dir: PathBuf,
}

pub fn compile_original_test_objects(args: CompileOriginalTestObjectsArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let output_dir = absolutize(&args.repo_root, &args.output_dir);
    let object_manifest = args
        .object_manifest
        .as_ref()
        .map(|path| absolutize(&args.repo_root, path))
        .unwrap_or_else(|| generated_dir.join("original_test_object_manifest.json"));
    let manifest = load_original_test_object_manifest(&object_manifest)?;

    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    let include_temp = tempdir().context("create generated include tempdir")?;
    let generated_include_dir = include_temp.path();
    prepare_original_generated_include_dir(&args.repo_root, generated_include_dir)?;

    for unit in manifest
        .translation_units
        .iter()
        .filter(|unit| unit.ubuntu_24_04_enabled)
    {
        let object_path = output_dir.join(&unit.output_object_relpath);
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut cmd = Command::new(&manifest.toolchain_defaults.compiler);
        cmd.current_dir(&args.repo_root)
            .arg("-c")
            .arg(&unit.source_path)
            .arg("-o")
            .arg(&object_path);

        for include in &unit.include_dirs {
            cmd.arg("-I")
                .arg(resolve_token(include, generated_include_dir, None)?);
        }
        for include in &unit.system_include_dirs {
            cmd.arg("-isystem").arg(include);
        }
        for definition in &unit.compile_definitions {
            cmd.arg(format!("-D{definition}"));
        }
        for flag in &manifest.toolchain_defaults.baseline_compiler_flags {
            cmd.arg(flag);
        }
        for flag in &unit.compile_flags {
            cmd.arg(flag);
        }

        let output = cmd
            .output()
            .with_context(|| format!("compile {}", unit.source_path))?;
        if !output.status.success() {
            bail!(
                "compiling {} failed:\n{}",
                unit.source_path,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    Ok(())
}

pub fn relink_original_test_objects(args: RelinkOriginalTestObjectsArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let objects_dir = absolutize(&args.repo_root, &args.objects_dir);
    let output_dir = absolutize(&args.repo_root, &args.output_dir);
    let library_path = absolutize(&args.repo_root, &args.library_path);
    let stage_libdir = library_path
        .parent()
        .ok_or_else(|| anyhow!("library path {} has no parent", library_path.display()))?;
    let object_manifest = args
        .object_manifest
        .as_ref()
        .map(|path| absolutize(&args.repo_root, path))
        .unwrap_or_else(|| generated_dir.join("original_test_object_manifest.json"));
    let manifest = load_original_test_object_manifest(&object_manifest)?;
    if let Some(standalone_manifest) = args.standalone_manifest.as_ref() {
        let standalone_manifest = absolutize(&args.repo_root, standalone_manifest);
        let _ = load_standalone_test_manifest(&standalone_manifest)?;
    }

    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    for target in manifest
        .targets
        .iter()
        .filter(|target| target.ubuntu_24_04_enabled)
    {
        let output_path = output_dir.join(&target.output_name);
        let mut cmd = Command::new(&manifest.toolchain_defaults.linker);
        cmd.current_dir(&args.repo_root).arg("-o").arg(&output_path);
        for object_id in &target.object_ids {
            let unit = manifest
                .translation_units
                .iter()
                .find(|unit| &unit.object_id == object_id)
                .ok_or_else(|| anyhow!("missing translation unit {}", object_id))?;
            cmd.arg(objects_dir.join(&unit.output_object_relpath));
        }
        for search in &target.link_search_paths {
            cmd.arg("-L")
                .arg(resolve_token(search, Path::new(""), Some(stage_libdir))?);
        }
        for search in &manifest.toolchain_defaults.baseline_linker_flags {
            cmd.arg(search);
        }
        for library in &manifest.toolchain_defaults.baseline_link_libraries {
            cmd.arg(render_library_arg(library));
        }
        for library in &target.link_libraries {
            cmd.arg(render_library_arg(library));
        }
        for option in &target.link_options {
            cmd.arg(option);
        }
        let output = cmd
            .output()
            .with_context(|| format!("link {}", target.target_name))?;
        if !output.status.success() {
            bail!(
                "linking {} failed:\n{}",
                target.target_name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        copy_target_resources(&args.repo_root, &output_dir, &target.resource_paths)?;
    }

    Ok(())
}

pub fn run_relinked_original_tests(args: RunRelinkedOriginalTestsArgs) -> Result<()> {
    let _generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let bin_dir = absolutize(&args.repo_root, &args.bin_dir);
    let standalone_manifest = absolutize(&args.repo_root, &args.standalone_manifest);
    let standalone = load_standalone_test_manifest(&standalone_manifest)?;
    let mut ran_any = false;

    for target in standalone.targets.iter().filter(|target| {
        args.validation_modes
            .iter()
            .any(|mode| mode == &target.ci_validation_mode)
    }) {
        if let Some(filter) = &args.filter {
            if &target.target_name != filter {
                continue;
            }
        }
        ran_any = true;
        let executable = bin_dir.join(&target.target_name);
        if !executable.exists() {
            bail!("missing relinked binary {}", executable.display());
        }
        let mut cmd = Command::new(&executable);
        cmd.current_dir(&bin_dir)
            .env_remove(real_runtime_env_key())
            .env("SDL_AUDIODRIVER", "dummy")
            .env("SDL_VIDEODRIVER", "dummy")
            .env("SDL_TESTS_QUICK", "1");
        for (key, value) in &target.checker_runner_contract.environment {
            cmd.env(key, value);
        }
        let description = format!("run {}", target.target_name);
        if target.timeout_seconds > 0 {
            run_command_with_timeout(
                &mut cmd,
                &description,
                Duration::from_secs(target.timeout_seconds as u64),
            )?;
        } else {
            run_command(&mut cmd, &description)?;
        }
    }

    if !ran_any && !args.skip_if_empty {
        bail!("no relinked tests matched the requested filter/modes");
    }

    Ok(())
}

pub fn build_original_standalone(args: BuildOriginalStandaloneArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let stage_root = absolutize(&args.repo_root, &args.stage_root);
    let build_dir = absolutize(&args.repo_root, &args.build_dir);
    let standalone_manifest =
        load_standalone_test_manifest(&absolutize(&args.repo_root, &args.standalone_manifest))?;
    let object_manifest = load_original_test_object_manifest(
        &generated_dir.join("original_test_object_manifest.json"),
    )?;
    let port_map = load_original_test_port_map(&generated_dir.join("original_test_port_map.json"))?;

    let owned_targets = port_map
        .target_ownership
        .iter()
        .filter(|entry| entry.owning_phase == args.phase && entry.linux_buildable)
        .map(|entry| entry.target_name.clone())
        .collect::<BTreeSet<_>>();
    if owned_targets.is_empty() {
        bail!(
            "phase {} owns no Linux-buildable standalone targets",
            args.phase
        );
    }

    let selected_targets = object_manifest
        .targets
        .iter()
        .filter(|target| {
            target.ubuntu_24_04_enabled
                && owned_targets.contains(&target.target_name)
                && standalone_manifest.targets.iter().any(|standalone| {
                    standalone.target_name == target.target_name && standalone.linux_buildable
                })
        })
        .collect::<Vec<_>>();
    if selected_targets.is_empty() {
        bail!("phase {} selected no standalone build targets", args.phase);
    }

    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)
            .with_context(|| format!("remove {}", build_dir.display()))?;
    }
    fs::create_dir_all(&build_dir)?;

    let stage_include_root = stage_root.join("usr/include");
    let stage_header_dir = stage_include_root.join("SDL2");
    let stage_multiarch_include = stage_include_root.join(UBUNTU_MULTIARCH);
    let stage_libdir = stage_root.join(format!("usr/lib/{UBUNTU_MULTIARCH}"));
    let objects_dir = build_dir.join("objects");
    fs::create_dir_all(&objects_dir)?;

    let needed_object_ids = selected_targets
        .iter()
        .flat_map(|target| target.object_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    for unit in object_manifest
        .translation_units
        .iter()
        .filter(|unit| unit.ubuntu_24_04_enabled && needed_object_ids.contains(&unit.object_id))
    {
        let output_path = objects_dir.join(&unit.output_object_relpath);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut cmd = Command::new(&object_manifest.toolchain_defaults.compiler);
        cmd.current_dir(&args.repo_root)
            .arg("-c")
            .arg(&unit.source_path)
            .arg("-o")
            .arg(&output_path)
            .arg("-I")
            .arg(&stage_header_dir)
            .arg("-I")
            .arg(&stage_multiarch_include)
            .arg("-I")
            .arg(args.repo_root.join("original/test"));

        for definition in &unit.compile_definitions {
            cmd.arg(format!("-D{definition}"));
        }
        for flag in &object_manifest.toolchain_defaults.baseline_compiler_flags {
            cmd.arg(flag);
        }
        for flag in &unit.compile_flags {
            cmd.arg(flag);
        }

        let output = cmd
            .output()
            .with_context(|| format!("compile {}", unit.source_path))?;
        if !output.status.success() {
            bail!(
                "compiling {} failed:\n{}",
                unit.source_path,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    for target in &selected_targets {
        let standalone = standalone_manifest
            .targets
            .iter()
            .find(|entry| entry.target_name == target.target_name)
            .ok_or_else(|| anyhow!("missing standalone manifest target {}", target.target_name))?;
        let output_path = build_dir.join(&target.output_name);
        let mut cmd = Command::new(&object_manifest.toolchain_defaults.linker);
        cmd.current_dir(&args.repo_root).arg("-o").arg(&output_path);
        for object_id in &target.object_ids {
            let unit = object_manifest
                .translation_units
                .iter()
                .find(|unit| &unit.object_id == object_id)
                .ok_or_else(|| anyhow!("missing translation unit {}", object_id))?;
            cmd.arg(objects_dir.join(&unit.output_object_relpath));
        }
        cmd.arg(format!("-L{}", stage_libdir.display()))
            .arg(format!("-Wl,-rpath,{}", stage_libdir.display()));
        for flag in &object_manifest.toolchain_defaults.baseline_linker_flags {
            cmd.arg(flag);
        }
        for library in &object_manifest.toolchain_defaults.baseline_link_libraries {
            cmd.arg(render_library_arg(library));
        }
        for library in &target.link_libraries {
            cmd.arg(render_library_arg(library));
        }
        for option in &target.link_options {
            cmd.arg(option);
        }

        let output = cmd
            .output()
            .with_context(|| format!("link {}", target.target_name))?;
        if !output.status.success() {
            bail!(
                "linking {} failed:\n{}",
                target.target_name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        copy_target_resources(&args.repo_root, &build_dir, &standalone.resource_paths)?;
    }

    Ok(())
}

pub fn run_original_standalone(args: RunOriginalStandaloneArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let build_dir = absolutize(&args.repo_root, &args.build_dir);
    let standalone_manifest =
        load_standalone_test_manifest(&absolutize(&args.repo_root, &args.standalone_manifest))?;
    let port_map = load_original_test_port_map(&generated_dir.join("original_test_port_map.json"))?;
    let owned_targets = port_map
        .target_ownership
        .iter()
        .filter(|entry| entry.owning_phase == args.phase && entry.linux_buildable)
        .map(|entry| entry.target_name.clone())
        .collect::<BTreeSet<_>>();

    let selected_targets = standalone_manifest
        .targets
        .iter()
        .filter(|target| {
            owned_targets.contains(&target.target_name)
                && target.linux_runnable
                && target.ci_validation_mode == args.validation_mode
        })
        .collect::<Vec<_>>();

    if selected_targets.is_empty() {
        if args.skip_if_empty {
            return Ok(());
        }
        bail!(
            "phase {} has no runnable standalone targets for validation mode {}",
            args.phase,
            args.validation_mode
        );
    }

    for target in selected_targets {
        let executable = build_dir.join(&target.target_name);
        if !executable.exists() {
            bail!("missing standalone binary {}", executable.display());
        }

        let mut cmd = Command::new(&executable);
        cmd.current_dir(&build_dir).env("SDL_TESTS_QUICK", "1");
        for (key, value) in &target.checker_runner_contract.environment {
            cmd.env(key, value);
        }
        let status = cmd
            .status()
            .with_context(|| format!("run {}", target.target_name))?;
        if !status.success() {
            bail!("standalone target {} failed", target.target_name);
        }
    }

    Ok(())
}

pub fn run_evdev_fixture_tests(repo_root: PathBuf) -> Result<()> {
    run_safe_test_target(&repo_root, "evdev_fixtures")?;
    run_safe_test_binary_ignored(
        &repo_root,
        "evdev_fixtures",
        "hinted_evdev_devices_appear_in_hint_order_with_probed_fixture_metadata",
    )?;
    run_safe_test_binary_ignored(
        &repo_root,
        "evdev_fixtures",
        "hinted_device_directory_expands_through_linux_discovery_order",
    )?;
    run_safe_test_binary(
        &repo_root,
        "original_apps_input",
        "controllermap_gamecontroller_and_testevdev_ports_cover_mapping_and_fixture_behavior",
    )
}

pub fn run_fixture_backed_original_tests(args: RunFixtureBackedOriginalTestsArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let build_dir = absolutize(&args.repo_root, &args.build_dir);
    let standalone_manifest =
        load_standalone_test_manifest(&absolutize(&args.repo_root, &args.standalone_manifest))?;
    let port_map = load_original_test_port_map(&generated_dir.join("original_test_port_map.json"))?;
    let owned_targets = port_map
        .target_ownership
        .iter()
        .filter(|entry| entry.owning_phase == args.phase && entry.linux_buildable)
        .map(|entry| entry.target_name.clone())
        .collect::<BTreeSet<_>>();
    let selected_targets = standalone_manifest
        .targets
        .iter()
        .filter(|target| {
            owned_targets.contains(&target.target_name)
                && matches!(
                    target.ci_validation_mode.as_str(),
                    "build_only" | "fixture_run"
                )
        })
        .collect::<Vec<_>>();

    if selected_targets.is_empty() {
        if args.skip_if_empty {
            return Ok(());
        }
        bail!(
            "phase {} has no fixture-backed standalone targets",
            args.phase
        );
    }

    for target in &selected_targets {
        let executable = build_dir.join(&target.target_name);
        if !executable.exists() {
            bail!(
                "fixture-backed standalone target {} is missing built binary {}",
                target.target_name,
                executable.display()
            );
        }
    }

    let mut rust_targets = BTreeSet::new();
    for target in &selected_targets {
        let matching_entries = port_map
            .entries
            .iter()
            .filter(|entry| {
                entry.source_kind == "standalone executable source"
                    && entry.owning_phase == args.phase
                    && entry.completion_state == PortCompletionState::Complete
                    && entry
                        .upstream_targets
                        .iter()
                        .any(|upstream| upstream == &target.target_name)
            })
            .collect::<Vec<_>>();
        if matching_entries.is_empty() {
            bail!(
                "fixture-backed standalone target {} has no complete phase-owned port entries",
                target.target_name
            );
        }
        for entry in matching_entries {
            rust_targets.insert(entry.rust_target_path.clone());
        }
    }

    for rust_target in rust_targets {
        let test_name = Path::new(&rust_target)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| anyhow!("unable to derive test target name from {}", rust_target))?;
        run_safe_test_target(&args.repo_root, test_name)?;
    }

    Ok(())
}

pub fn run_gesture_replay(repo_root: PathBuf) -> Result<()> {
    let generated_dir = repo_root.join("safe/generated");
    let port_map = load_original_test_port_map(&generated_dir.join("original_test_port_map.json"))?;
    let standalone =
        load_standalone_test_manifest(&generated_dir.join("standalone_test_manifest.json"))?;

    let gesture_entry = port_map
        .entries
        .iter()
        .find(|entry| entry.original_path == "original/test/testgesture.c")
        .ok_or_else(|| anyhow!("missing testgesture entry in original_test_port_map.json"))?;
    if gesture_entry.completion_state != PortCompletionState::Complete {
        bail!(
            "testgesture port map entry must be complete, found {:?}",
            gesture_entry.completion_state
        );
    }
    if gesture_entry.rust_target_path != "safe/tests/original_apps_video.rs" {
        bail!(
            "testgesture port map entry points at unexpected Rust target {}",
            gesture_entry.rust_target_path
        );
    }

    let gesture_target = standalone
        .targets
        .iter()
        .find(|target| target.target_name == "testgesture")
        .ok_or_else(|| anyhow!("missing testgesture target in standalone_test_manifest.json"))?;
    if gesture_target.ci_validation_mode != "build_only" {
        bail!(
            "testgesture standalone manifest must remain build_only, found {}",
            gesture_target.ci_validation_mode
        );
    }

    run_safe_test_binary(
        &repo_root,
        "original_apps_video",
        "gesture_replay_roundtrip_is_deterministic",
    )
}

pub fn run_xvfb_window_smoke(repo_root: PathBuf) -> Result<()> {
    run_xvfb(
        repo_root,
        vec![
            "cargo".to_string(),
            "test".to_string(),
            "--manifest-path".to_string(),
            "safe/Cargo.toml".to_string(),
            "--test".to_string(),
            "xvfb_window_smoke".to_string(),
            "xvfb_backed_x11_window_smoke_replaces_manual_window_demos".to_string(),
            "--".to_string(),
            "--exact".to_string(),
        ],
    )
}

pub fn run_xvfb(repo_root: PathBuf, command: Vec<String>) -> Result<()> {
    if command.is_empty() {
        bail!("run-xvfb requires a command");
    }

    let (_guard, display_name) = spawn_xvfb()?;
    let mut child = Command::new(&command[0]);
    child
        .current_dir(&repo_root)
        .args(&command[1..])
        .env("DISPLAY", &display_name);
    let status = child
        .status()
        .with_context(|| format!("run command under Xvfb: {}", command.join(" ")))?;
    if !status.success() {
        bail!("command under Xvfb failed: {}", command.join(" "));
    }
    Ok(())
}

pub fn build_original_cmake_suite(args: BuildOriginalCmakeSuiteArgs) -> Result<()> {
    let original_test_dir = resolve_original_test_dir(&args.repo_root, &args.original_dir)?;
    let stage_root = absolutize(&args.repo_root, &args.stage_root);
    let build_dir = absolutize(&args.repo_root, &args.build_dir);

    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)
            .with_context(|| format!("remove {}", build_dir.display()))?;
    }

    let mut configure_cmd = Command::new("cmake");
    configure_cmd
        .current_dir(&args.repo_root)
        .arg("-S")
        .arg(&original_test_dir)
        .arg("-B")
        .arg(&build_dir)
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg(format!(
            "-DCMAKE_PREFIX_PATH={}",
            stage_root.join("usr").display()
        ))
        .arg("-DSDL_INSTALL_TESTS=ON")
        .arg("-DSDL_DUMMYAUDIO=ON")
        .arg("-DSDL_DUMMYVIDEO=ON");
    run_command(&mut configure_cmd, "configure original CMake suite")?;

    let mut build_cmd = Command::new("cmake");
    build_cmd
        .current_dir(&args.repo_root)
        .arg("--build")
        .arg(&build_dir)
        .arg("--parallel")
        .arg(parallelism().to_string());
    run_command(&mut build_cmd, "build original CMake suite")
}

pub fn run_original_ctest(args: RunOriginalCtestArgs) -> Result<()> {
    let build_dir = absolutize(&args.repo_root, &args.build_dir);
    let mut cmd = Command::new("ctest");
    cmd.current_dir(&args.repo_root)
        .arg("--test-dir")
        .arg(&build_dir)
        .arg("--output-on-failure")
        .arg("--timeout")
        .arg("120")
        .env("SDL_TESTS_QUICK", "1");
    if let Some(stage_root) = args.stage_root.as_ref() {
        apply_stage_suite_env(&mut cmd, &absolutize(&args.repo_root, stage_root))?;
    }
    let filter = match (&args.filter, &args.test_list) {
        (Some(filter), _) => Some(filter.clone()),
        (None, Some(test_list)) => ctest_filter_from_test_list(&args.repo_root, test_list)?,
        (None, None) => default_ctest_filter(&args.repo_root)?,
    };
    if let Some(filter) = &filter {
        cmd.arg("-R").arg(filter);
    }
    run_command(&mut cmd, "run original CMake ctest suite")
}

pub fn build_original_autotools_suite(args: BuildOriginalAutotoolsSuiteArgs) -> Result<()> {
    let original_test_dir = resolve_original_test_dir(&args.repo_root, &args.original_dir)?;
    let stage_root = absolutize(&args.repo_root, &args.stage_root);
    let build_dir = absolutize(&args.repo_root, &args.build_dir);

    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)
            .with_context(|| format!("remove {}", build_dir.display()))?;
    }
    fs::create_dir_all(&build_dir)?;

    let configure = original_test_dir.join("configure");
    let mut configure_cmd = Command::new(&configure);
    configure_cmd.current_dir(&build_dir);
    apply_stage_suite_env(&mut configure_cmd, &stage_root)?;
    run_command(&mut configure_cmd, "configure original autotools suite")?;

    let mut make_cmd = Command::new("make");
    make_cmd
        .current_dir(&build_dir)
        .arg(format!("-j{}", parallelism()));
    apply_stage_suite_env(&mut make_cmd, &stage_root)?;
    run_command(&mut make_cmd, "build original autotools suite")?;
    record_autotools_stage_root(&build_dir, &stage_root)
}

pub fn run_original_autotools_check(args: RunOriginalAutotoolsCheckArgs) -> Result<()> {
    let build_dir = absolutize(&args.repo_root, &args.build_dir);
    let stage_root = resolve_autotools_stage_root(&args.repo_root, &build_dir, args.stage_root)?;
    let targets = default_headless_original_suite_targets(&args.repo_root)?;
    if targets.is_empty() {
        bail!("original autotools check selected no runnable targets");
    }

    for target in targets {
        let executable = build_dir.join(&target);
        if !executable.exists() {
            bail!(
                "original autotools test target {} is missing binary {}",
                target,
                executable.display()
            );
        }

        let mut cmd = Command::new(&executable);
        cmd.current_dir(&build_dir)
            .env("SDL_AUDIODRIVER", "dummy")
            .env("SDL_VIDEODRIVER", "dummy")
            .env("SDL_TESTS_QUICK", "1");
        apply_stage_suite_env(&mut cmd, &stage_root)?;
        run_command_with_timeout(
            &mut cmd,
            &format!("run original autotools test {target}"),
            ORIGINAL_SUITE_TIMEOUT,
        )?;
    }

    Ok(())
}

fn resolve_token(
    value: &str,
    generated_include_dir: &Path,
    stage_libdir: Option<&Path>,
) -> Result<String> {
    match value {
        "$GENERATED_INCLUDE_DIR" => Ok(generated_include_dir.display().to_string()),
        "$STAGE_LIBDIR" => Ok(stage_libdir
            .ok_or_else(|| anyhow!("missing stage libdir token value"))?
            .display()
            .to_string()),
        _ => Ok(value.to_string()),
    }
}

fn prepare_original_generated_include_dir(
    repo_root: &Path,
    generated_include_dir: &Path,
) -> Result<()> {
    let original_dir = repo_root.join("original");
    fs::write(
        generated_include_dir.join("SDL_config.h"),
        fs::read(original_dir.join("debian/SDL_config.h"))
            .context("read original Debian SDL_config.h wrapper")?,
    )?;
    fs::write(
        generated_include_dir.join("SDL_revision.h"),
        generate_sdl_revision_header(),
    )?;

    let multiarch_dir = generated_include_dir.join("SDL2");
    fs::create_dir_all(&multiarch_dir)?;
    fs::write(
        multiarch_dir.join("_real_SDL_config.h"),
        generate_real_sdl_config(),
    )?;
    for header in ["SDL_platform.h", "begin_code.h", "close_code.h"] {
        let link = multiarch_dir.join(header);
        if link.exists() {
            fs::remove_file(&link)?;
        }
        std::os::unix::fs::symlink(original_dir.join("include").join(header), &link)
            .with_context(|| format!("symlink {}", link.display()))?;
    }

    Ok(())
}

fn resolve_original_test_dir(repo_root: &Path, original_dir: &Path) -> Result<PathBuf> {
    let original_dir = absolutize(repo_root, original_dir);
    let nested = original_dir.join("test");
    if nested.join("CMakeLists.txt").exists() && nested.join("configure").exists() {
        return Ok(nested);
    }

    let direct = original_dir.join("CMakeLists.txt");
    let direct_configure = original_dir.join("configure");
    if direct.exists() && direct_configure.exists() {
        return Ok(original_dir);
    }

    bail!(
        "unable to locate original test suite under {}",
        original_dir.display()
    );
}

fn ctest_filter_from_test_list(repo_root: &Path, test_list: &str) -> Result<Option<String>> {
    let candidate_path = absolutize(repo_root, Path::new(test_list));
    if candidate_path.exists() {
        let targets = load_test_list_targets(&candidate_path)?;
        return test_names_to_ctest_regex(&filter_headless_original_suite_targets(targets));
    }

    let targets = test_list
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    test_names_to_ctest_regex(&targets)
}

fn default_ctest_filter(repo_root: &Path) -> Result<Option<String>> {
    let targets = load_noninteractive_test_targets(repo_root)?;
    let standalone = load_standalone_test_manifest(
        &repo_root.join("safe/generated/standalone_test_manifest.json"),
    )?;
    let filtered = targets
        .into_iter()
        .filter(|target| {
            DEFAULT_CTEST_TARGETS.contains(&target.as_str())
                && standalone
                    .targets
                    .iter()
                    .find(|entry| entry.target_name == *target)
                    .map(|entry| entry.timeout_seconds <= 60)
                    .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    test_names_to_ctest_regex(&filtered)
}

fn test_names_to_ctest_regex(targets: &[String]) -> Result<Option<String>> {
    if targets.is_empty() {
        return Ok(None);
    }
    if targets.iter().any(|target| target.is_empty()) {
        bail!("CTest test list contains an empty target name");
    }
    Ok(Some(format!("^({})$", targets.join("|"))))
}

fn default_headless_original_suite_targets(repo_root: &Path) -> Result<Vec<String>> {
    Ok(filter_headless_original_suite_targets(
        load_noninteractive_test_targets(repo_root)?,
    ))
}

fn load_noninteractive_test_targets(repo_root: &Path) -> Result<Vec<String>> {
    load_test_list_targets(&repo_root.join("safe/generated/noninteractive_test_list.json"))
}

fn load_test_list_targets(path: &Path) -> Result<Vec<String>> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)
        .with_context(|| format!("parse test list {}", path.display()))?;
    value
        .get("targets")
        .and_then(|targets| targets.as_array())
        .ok_or_else(|| anyhow!("{} is missing targets", path.display()))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| anyhow!("{} contains a non-string target", path.display()))
        })
        .collect::<Result<Vec<_>>>()
}

fn filter_headless_original_suite_targets(targets: Vec<String>) -> Vec<String> {
    targets
        .into_iter()
        .filter(|target| !HEADLESS_ORIGINAL_SUITE_UNSUPPORTED_TARGETS.contains(&target.as_str()))
        .collect()
}

fn record_autotools_stage_root(build_dir: &Path, stage_root: &Path) -> Result<()> {
    fs::write(
        autotools_stage_root_path(build_dir),
        format!("{}\n", stage_root.display()),
    )
    .with_context(|| format!("write autotools stage root for {}", build_dir.display()))
}

fn resolve_autotools_stage_root(
    repo_root: &Path,
    build_dir: &Path,
    explicit_stage_root: Option<PathBuf>,
) -> Result<PathBuf> {
    if let Some(stage_root) = explicit_stage_root {
        return Ok(absolutize(repo_root, &stage_root));
    }

    let stage_root_path = autotools_stage_root_path(build_dir);
    let contents = fs::read_to_string(&stage_root_path).with_context(|| {
        format!(
            "read autotools stage root metadata {}; rerun build-original-autotools-suite with --root or pass --root explicitly",
            stage_root_path.display()
        )
    })?;
    let stage_root = contents.trim();
    if stage_root.is_empty() {
        bail!(
            "autotools stage root metadata {} is empty",
            stage_root_path.display()
        );
    }
    Ok(PathBuf::from(stage_root))
}

fn autotools_stage_root_path(build_dir: &Path) -> PathBuf {
    build_dir.join(".xtask-stage-root")
}

fn apply_stage_suite_env(cmd: &mut Command, stage_root: &Path) -> Result<()> {
    let stage_bin = stage_root.join("usr/bin");
    let stage_libdir = stage_root.join(format!("usr/lib/{UBUNTU_MULTIARCH}"));
    let stage_pkgconfig = stage_libdir.join("pkgconfig");
    let ttf_cflags = pkg_config_cflags("SDL2_ttf")?;

    cmd.env("PATH", joined_env_path(&stage_bin, env::var_os("PATH"))?)
        .env("SDL2_CONFIG", stage_bin.join("sdl2-config"))
        .env(
            "PKG_CONFIG_PATH",
            joined_env_path(&stage_pkgconfig, env::var_os("PKG_CONFIG_PATH"))?,
        )
        .env(
            "LD_LIBRARY_PATH",
            joined_env_path(&stage_libdir, env::var_os("LD_LIBRARY_PATH"))?,
        );
    if let Some(ttf_cflags) = ttf_cflags {
        cmd.env(
            "CFLAGS",
            prepend_shell_flags(&ttf_cflags, env::var_os("CFLAGS")),
        );
    }

    Ok(())
}

fn pkg_config_cflags(package: &str) -> Result<Option<String>> {
    let output = Command::new("pkg-config")
        .args(["--cflags", package])
        .output()
        .with_context(|| format!("query pkg-config --cflags {package}"))?;
    if !output.status.success() {
        return Ok(None);
    }

    let flags = String::from_utf8(output.stdout)
        .with_context(|| format!("decode pkg-config --cflags {package} output"))?
        .trim()
        .to_string();
    if flags.is_empty() {
        Ok(None)
    } else {
        Ok(Some(flags))
    }
}

fn prepend_shell_flags(prefix: &str, existing: Option<OsString>) -> String {
    match existing {
        Some(existing) if !existing.is_empty() => {
            format!("{prefix} {}", existing.to_string_lossy())
        }
        _ => prefix.to_string(),
    }
}

fn real_runtime_env_key() -> &'static str {
    concat!("SAFE_SDL_REAL_", "SDL_PATH")
}

fn joined_env_path(first: &Path, existing: Option<OsString>) -> Result<OsString> {
    let mut entries = vec![first.to_path_buf()];
    if let Some(existing) = existing {
        entries.extend(env::split_paths(&existing));
    }
    env::join_paths(entries).context("join environment search path")
}

fn parallelism() -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}

fn run_command(cmd: &mut Command, description: &str) -> Result<()> {
    let status = cmd.status().with_context(|| description.to_string())?;
    if !status.success() {
        bail!("{description} failed with status {status}");
    }
    Ok(())
}

fn run_command_with_timeout(cmd: &mut Command, description: &str, timeout: Duration) -> Result<()> {
    let mut child = cmd.spawn().with_context(|| description.to_string())?;
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child
            .try_wait()
            .with_context(|| format!("{description}: poll child process"))?
        {
            if status.success() {
                return Ok(());
            }
            bail!("{description} failed with status {status}");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("{description} timed out after {}s", timeout.as_secs());
        }
        thread::sleep(Duration::from_millis(200));
    }
}

fn render_library_arg(name: &str) -> String {
    match name {
        "GL" => "-lGL".to_string(),
        "GLESv1_CM" => "-lGLESv1_CM".to_string(),
        "X11" => "-lX11".to_string(),
        "m" => "-lm".to_string(),
        _ => format!("-l{name}"),
    }
}

fn absolutize(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn copy_target_resources(repo_root: &Path, build_dir: &Path, resources: &[String]) -> Result<()> {
    for resource in resources {
        let source = repo_root.join(resource);
        let destination = build_dir.join(
            source
                .file_name()
                .ok_or_else(|| anyhow!("resource path {} has no filename", source.display()))?,
        );
        fs::copy(&source, &destination)
            .with_context(|| format!("copy {} to {}", source.display(), destination.display()))?;
    }
    Ok(())
}

fn run_safe_test_binary(repo_root: &Path, test_name: &str, filter: &str) -> Result<()> {
    let status = Command::new("cargo")
        .current_dir(repo_root)
        .arg("test")
        .arg("--manifest-path")
        .arg("safe/Cargo.toml")
        .arg("--test")
        .arg(test_name)
        .arg(filter)
        .arg("--")
        .arg("--exact")
        .status()
        .with_context(|| format!("run cargo test {test_name}::{filter}"))?;
    if !status.success() {
        bail!("cargo test {test_name} {filter} failed");
    }
    Ok(())
}

fn run_safe_test_binary_ignored(repo_root: &Path, test_name: &str, filter: &str) -> Result<()> {
    let status = Command::new("cargo")
        .current_dir(repo_root)
        .arg("test")
        .arg("--manifest-path")
        .arg("safe/Cargo.toml")
        .arg("--test")
        .arg(test_name)
        .arg(filter)
        .arg("--")
        .arg("--ignored")
        .arg("--exact")
        .status()
        .with_context(|| format!("run cargo test --ignored {test_name}::{filter}"))?;
    if !status.success() {
        bail!("cargo test --ignored {test_name} {filter} failed");
    }
    Ok(())
}

fn run_safe_test_target(repo_root: &Path, test_name: &str) -> Result<()> {
    let status = Command::new("cargo")
        .current_dir(repo_root)
        .arg("test")
        .arg("--manifest-path")
        .arg("safe/Cargo.toml")
        .arg("--test")
        .arg(test_name)
        .status()
        .with_context(|| format!("run cargo test {test_name}"))?;
    if !status.success() {
        bail!("cargo test {test_name} failed");
    }
    Ok(())
}

struct XvfbGuard {
    child: Child,
}

impl Drop for XvfbGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_xvfb() -> Result<(XvfbGuard, String)> {
    for display in 91..100 {
        let display_name = format!(":{display}");
        let child = Command::new("Xvfb")
            .arg(&display_name)
            .arg("-screen")
            .arg("0")
            .arg("1024x768x24")
            .arg("-nolisten")
            .arg("tcp")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let Ok(child) = child else {
            continue;
        };
        thread::sleep(Duration::from_millis(500));
        return Ok((XvfbGuard { child }, display_name));
    }

    bail!("unable to start Xvfb")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        filter_headless_original_suite_targets, record_autotools_stage_root,
        resolve_autotools_stage_root,
    };
    use tempfile::tempdir;

    #[test]
    fn run_original_autotools_check_uses_recorded_stage_root() {
        let repo_root = tempdir().expect("repo root tempdir");
        let build_dir = repo_root.path().join("build");
        std::fs::create_dir_all(&build_dir).expect("create build dir");
        let stage_root = repo_root.path().join("stage");
        record_autotools_stage_root(&build_dir, &stage_root).expect("record stage root");

        let resolved =
            resolve_autotools_stage_root(repo_root.path(), &build_dir, None).expect("resolve");

        assert_eq!(resolved, stage_root);
    }

    #[test]
    fn explicit_autotools_stage_root_overrides_metadata() {
        let repo_root = tempdir().expect("repo root tempdir");
        let build_dir = repo_root.path().join("build");
        std::fs::create_dir_all(&build_dir).expect("create build dir");
        record_autotools_stage_root(&build_dir, &repo_root.path().join("stale"))
            .expect("record stale stage root");

        let resolved = resolve_autotools_stage_root(
            repo_root.path(),
            &build_dir,
            Some(PathBuf::from("/tmp/libsdl-safe-root")),
        )
        .expect("resolve explicit stage root");

        assert_eq!(resolved, PathBuf::from("/tmp/libsdl-safe-root"));
    }

    #[test]
    fn headless_original_suite_filter_removes_known_host_video_blockers() {
        let filtered = filter_headless_original_suite_targets(vec![
            "testatomic".to_string(),
            "testlocale".to_string(),
            "testaudioinfo".to_string(),
            "testdisplayinfo".to_string(),
        ]);

        assert_eq!(
            filtered,
            vec!["testatomic".to_string(), "testaudioinfo".to_string()]
        );
    }
}
