use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use tempfile::tempdir;

use crate::contracts::{
    generate_real_sdl_config, generate_sdl_revision_header, load_original_test_object_manifest,
    load_original_test_port_map, load_standalone_test_manifest, UBUNTU_MULTIARCH,
};

pub struct CompileOriginalTestObjectsArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub output_dir: PathBuf,
}

pub struct RelinkOriginalTestObjectsArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub objects_dir: PathBuf,
    pub output_dir: PathBuf,
    pub library_path: PathBuf,
}

pub struct RunRelinkedOriginalTestsArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub filter: Option<String>,
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

pub fn compile_original_test_objects(args: CompileOriginalTestObjectsArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let output_dir = absolutize(&args.repo_root, &args.output_dir);
    let manifest = load_original_test_object_manifest(
        &generated_dir.join("original_test_object_manifest.json"),
    )?;

    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    let include_temp = tempdir().context("create generated include tempdir")?;
    let generated_include_dir = include_temp.path();
    fs::write(
        generated_include_dir.join("SDL_config.h"),
        generate_real_sdl_config(),
    )?;
    fs::write(
        generated_include_dir.join("SDL_revision.h"),
        generate_sdl_revision_header(),
    )?;

    for unit in manifest
        .translation_units
        .iter()
        .filter(|unit| unit.ubuntu_24_04_enabled)
    {
        let object_path = output_dir.join(&unit.output_object_relpath);
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut cmd = Command::new("cc");
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
    let manifest = load_original_test_object_manifest(
        &generated_dir.join("original_test_object_manifest.json"),
    )?;

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
        let mut cmd = Command::new("cc");
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
        cmd.arg("-lSDL2_test").arg("-lSDL2");
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
    }

    Ok(())
}

pub fn run_relinked_original_tests(args: RunRelinkedOriginalTestsArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let bin_dir = absolutize(&args.repo_root, &args.bin_dir);
    let standalone =
        load_standalone_test_manifest(&generated_dir.join("standalone_test_manifest.json"))?;

    for target in standalone
        .targets
        .iter()
        .filter(|target| target.ci_validation_mode == "auto_run")
    {
        if let Some(filter) = &args.filter {
            if &target.target_name != filter {
                continue;
            }
        }
        let status = Command::new(bin_dir.join(&target.target_name))
            .current_dir(&bin_dir)
            .env("SDL_AUDIODRIVER", "dummy")
            .env("SDL_VIDEODRIVER", "dummy")
            .status()
            .with_context(|| format!("run {}", target.target_name))?;
        if !status.success() {
            bail!("relinked test {} failed", target.target_name);
        }
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

pub fn run_gesture_replay(repo_root: PathBuf) -> Result<()> {
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
