use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use tempfile::tempdir;

use crate::contracts::{
    generate_real_sdl_config, generate_sdl_revision_header, load_driver_contract,
    load_install_contract, load_public_header_inventory, DriverFamilyContract,
    PublicHeaderInventory, SDL_RUNTIME_REALNAME, SDL_SONAME, SDL_VERSION, UBUNTU_MULTIARCH,
};
use crate::original_tests::{
    compile_original_test_objects, relink_original_test_objects, CompileOriginalTestObjectsArgs,
    RelinkOriginalTestObjectsArgs,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageInstallMode {
    Bootstrap,
    Runtime,
    Full,
}

pub struct StageInstallArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub original_dir: PathBuf,
    pub stage_root: PathBuf,
    pub library_path: Option<PathBuf>,
    pub mode: StageInstallMode,
}

pub struct VerifyBootstrapStageArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub stage_root: PathBuf,
}

pub struct VerifyDriverContractArgs {
    pub repo_root: PathBuf,
    pub contract_path: PathBuf,
    pub stage_root: PathBuf,
    pub kind: String,
}

pub fn stage_install(args: StageInstallArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let original_dir = absolutize(&args.repo_root, &args.original_dir);
    let stage_root = absolutize(&args.repo_root, &args.stage_root);
    let inventory =
        load_public_header_inventory(&generated_dir.join("public_header_inventory.json"))?;
    let install_contract = load_install_contract(&generated_dir.join("install_contract.json"))?;

    if stage_root.exists() {
        fs::remove_dir_all(&stage_root)
            .with_context(|| format!("remove {}", stage_root.display()))?;
    }
    fs::create_dir_all(&stage_root)?;

    install_public_headers(&args.repo_root, &original_dir, &stage_root, &inventory)?;
    install_multiarch_headers(&stage_root)?;
    install_pkg_config(&original_dir, &stage_root)?;
    install_sdl2_config_script(&stage_root)?;
    install_m4(&original_dir, &stage_root)?;
    install_cmake_surface(&original_dir, &stage_root)?;
    install_helper_archives(&args.repo_root, &stage_root)?;
    install_library_artifacts(&args.repo_root, &stage_root, args.library_path.as_deref())?;
    if args.mode == StageInstallMode::Full {
        install_installed_tests(&args.repo_root, &generated_dir, &stage_root)?;
    }

    let _ = install_contract;
    Ok(())
}

pub fn verify_bootstrap_stage(args: VerifyBootstrapStageArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let stage_root = absolutize(&args.repo_root, &args.stage_root);
    let inventory =
        load_public_header_inventory(&generated_dir.join("public_header_inventory.json"))?;
    let install_contract = load_install_contract(&generated_dir.join("install_contract.json"))?;

    verify_headers(&stage_root, &inventory)?;

    for required in install_contract
        .dev_paths
        .iter()
        .chain(install_contract.runtime_paths.iter())
    {
        ensure_exists(&stage_root.join(required))?;
    }

    for cmake_path in &install_contract.cmake_surface {
        ensure_exists(&stage_root.join(cmake_path))?;
    }

    Ok(())
}

pub fn verify_driver_contract(args: VerifyDriverContractArgs) -> Result<()> {
    let contract_path = absolutize(&args.repo_root, &args.contract_path);
    let stage_root = absolutize(&args.repo_root, &args.stage_root);
    let driver_contract = load_driver_contract(&contract_path)?;
    match args.kind.as_str() {
        "video" => {
            verify_video_driver_contract(&args.repo_root, &stage_root, &driver_contract.video)
        }
        "audio" => {
            verify_audio_driver_contract(&args.repo_root, &stage_root, &driver_contract.audio)
        }
        other => bail!("unsupported driver contract kind {other}"),
    }
}

fn validate_video_contract(contract: &DriverFamilyContract) -> Result<()> {
    let derived_no_hint = contract
        .registry_order
        .iter()
        .filter(|entry| entry.demand_only != Some(true))
        .map(|entry| entry.driver_name.clone())
        .collect::<Vec<_>>();
    if contract.no_hint_probe_order != derived_no_hint {
        bail!(
            "video driver contract no_hint_probe_order mismatch\nexpected: {:?}\nactual: {:?}",
            derived_no_hint,
            contract.no_hint_probe_order
        );
    }

    if contract.single_backend_expectations.len() != contract.registry_order.len() {
        bail!(
            "video driver contract single_backend_expectations count mismatch: expected {}, got {}",
            contract.registry_order.len(),
            contract.single_backend_expectations.len()
        );
    }

    for entry in &contract.registry_order {
        let expectation = contract
            .single_backend_expectations
            .iter()
            .find(|expectation| expectation.driver_name == entry.driver_name)
            .ok_or_else(|| {
                anyhow!(
                    "video driver contract missing single_backend_expectations entry for {}",
                    entry.driver_name
                )
            })?;
        let expected_selected = if entry.demand_only == Some(true) {
            None
        } else {
            Some(entry.driver_name.clone())
        };
        if expectation.selected_without_hint != expected_selected {
            bail!(
                "video driver contract selected_without_hint mismatch for {}\nexpected: {:?}\nactual: {:?}",
                entry.driver_name,
                expected_selected,
                expectation.selected_without_hint
            );
        }
        if expectation.rationale.trim().is_empty() {
            bail!(
                "video driver contract rationale missing for {}",
                entry.driver_name
            );
        }
    }

    let evdev = contract
        .registry_order
        .iter()
        .find(|entry| entry.driver_name == "evdev")
        .ok_or_else(|| anyhow!("video driver contract missing evdev entry"))?;
    if !evdev
        .feature_predicates
        .iter()
        .any(|predicate| predicate.contains("SDL_INPUT_LINUXEV"))
    {
        bail!("video driver contract evdev entry must preserve SDL_INPUT_LINUXEV gating");
    }
    if !contract
        .no_hint_probe_order
        .iter()
        .any(|driver| driver == "evdev")
    {
        bail!("video driver contract no_hint_probe_order missing evdev");
    }
    if contract_selected_without_hint(contract, "evdev")? != "evdev" {
        bail!("video driver contract selected_without_hint mismatch for evdev");
    }

    Ok(())
}

fn verify_video_driver_contract(
    repo_root: &Path,
    stage_root: &Path,
    contract: &DriverFamilyContract,
) -> Result<()> {
    validate_video_contract(contract)?;

    let expected = contract
        .registry_order
        .iter()
        .map(|entry| entry.driver_name.clone())
        .collect::<Vec<_>>();
    let probe = build_driver_probe(repo_root, stage_root)?;

    let listed = run_driver_probe(&probe, &["list-video"], &[])?;
    if listed != expected {
        bail!(
            "video driver registry mismatch\nexpected: {:?}\nactual: {:?}",
            expected,
            listed
        );
    }

    let dummy_expected = contract_selected_without_hint(contract, "dummy")?;
    let dummy = run_driver_probe(
        &probe,
        &["init-video-nohint"],
        &[("SDL_VIDEODRIVER", "dummy")],
    )?;
    if dummy != [dummy_expected] {
        bail!("explicit dummy driver probe failed: {:?}", dummy);
    }

    let offscreen_expected = contract_selected_without_hint(contract, "offscreen")?;
    let offscreen = run_driver_probe(
        &probe,
        &["init-video-nohint"],
        &[("SDL_VIDEODRIVER", "offscreen")],
    )?;
    if offscreen != [offscreen_expected] {
        bail!("explicit offscreen driver probe failed: {:?}", offscreen);
    }

    let x_display = if let Ok(display) = std::env::var("DISPLAY") {
        Some((None, display))
    } else {
        spawn_xvfb()
    };

    if let Some((_guard, display)) = x_display {
        let x11_expected = contract_selected_without_hint(contract, "x11")?;
        let env = [("DISPLAY", display.as_str())];
        let explicit_x11 = run_driver_probe_capture(
            &probe,
            &["init-video-explicit", "x11"],
            &[("DISPLAY", display.as_str()), ("SDL_VIDEODRIVER", "x11")],
        )?;
        if explicit_x11.success {
            if explicit_x11.stdout != [x11_expected.clone()] {
                bail!(
                    "explicit x11 driver probe did not match contract under X11/Xvfb: {:?}",
                    explicit_x11.stdout
                );
            }
            let no_hint = run_driver_probe(&probe, &["init-video-nohint"], &env)?;
            if no_hint != [x11_expected] {
                bail!(
                    "no-hint video probe did not match contract under X11/Xvfb: {:?}",
                    no_hint
                );
            }
        } else {
            let fallback = first_packaged_video_fallback(contract)?;
            let no_hint = run_driver_probe(&probe, &["init-video-nohint"], &env)?;
            if no_hint != [fallback.clone()] {
                bail!(
                    "no-hint video probe did not fall back to {fallback} when host backends were unavailable under X11/Xvfb: {:?}",
                    no_hint
                );
            }
        }
    }

    Ok(())
}

fn first_packaged_video_fallback(contract: &DriverFamilyContract) -> Result<String> {
    for driver in &contract.no_hint_probe_order {
        if matches!(driver.as_str(), "x11" | "wayland" | "KMSDRM") {
            continue;
        }
        return contract_selected_without_hint(contract, driver);
    }
    bail!("video driver contract has no packaged fallback backend");
}

fn validate_audio_contract(contract: &DriverFamilyContract) -> Result<()> {
    let derived_no_hint = contract
        .registry_order
        .iter()
        .filter(|entry| entry.demand_only != Some(true))
        .map(|entry| entry.driver_name.clone())
        .collect::<Vec<_>>();
    if contract.no_hint_probe_order != derived_no_hint {
        bail!(
            "audio driver contract no_hint_probe_order mismatch\nexpected: {:?}\nactual: {:?}",
            derived_no_hint,
            contract.no_hint_probe_order
        );
    }

    if contract.single_backend_expectations.len() != contract.registry_order.len() {
        bail!(
            "audio driver contract single_backend_expectations count mismatch: expected {}, got {}",
            contract.registry_order.len(),
            contract.single_backend_expectations.len()
        );
    }

    for entry in &contract.registry_order {
        let expectation = contract
            .single_backend_expectations
            .iter()
            .find(|expectation| expectation.driver_name == entry.driver_name)
            .ok_or_else(|| {
                anyhow!(
                    "audio driver contract missing single_backend_expectations entry for {}",
                    entry.driver_name
                )
            })?;
        let expected_selected = if entry.demand_only == Some(true) {
            None
        } else {
            Some(entry.driver_name.clone())
        };
        if expectation.selected_without_hint != expected_selected {
            bail!(
                "audio driver contract selected_without_hint mismatch for {}\nexpected: {:?}\nactual: {:?}",
                entry.driver_name,
                expected_selected,
                expectation.selected_without_hint
            );
        }
        if expectation.rationale.trim().is_empty() {
            bail!(
                "audio driver contract rationale missing for {}",
                entry.driver_name
            );
        }
    }

    let names = contract
        .registry_order
        .iter()
        .map(|entry| entry.driver_name.as_str())
        .collect::<Vec<_>>();
    if names
        != [
            "pulseaudio",
            "alsa",
            "sndio",
            "pipewire",
            "dsp",
            "disk",
            "dummy",
        ]
    {
        bail!("audio driver contract registry_order mismatch: {:?}", names);
    }
    if !contract
        .registry_order
        .iter()
        .any(|entry| entry.driver_name == "dsp" && entry.demand_only != Some(true))
    {
        bail!("audio driver contract must preserve non-demand dsp entry");
    }
    for driver in ["disk", "dummy"] {
        if !contract
            .registry_order
            .iter()
            .any(|entry| entry.driver_name == driver && entry.demand_only == Some(true))
        {
            bail!("audio driver contract must preserve demand_only semantics for {driver}");
        }
    }

    Ok(())
}

fn verify_audio_driver_contract(
    repo_root: &Path,
    stage_root: &Path,
    contract: &DriverFamilyContract,
) -> Result<()> {
    validate_audio_contract(contract)?;

    let expected = contract
        .registry_order
        .iter()
        .map(|entry| entry.driver_name.clone())
        .collect::<Vec<_>>();
    let probe = build_driver_probe(repo_root, stage_root)?;

    let listed = run_driver_probe(&probe, &["list-audio"], &[])?;
    if listed != expected {
        bail!(
            "audio driver registry mismatch\nexpected: {:?}\nactual: {:?}",
            expected,
            listed
        );
    }

    let default_driver = contract
        .no_hint_probe_order
        .first()
        .ok_or_else(|| anyhow!("audio driver contract missing no_hint_probe_order"))?;
    let no_hint_expected = contract_selected_without_hint(contract, default_driver)?;
    let no_hint = run_driver_probe(&probe, &["init-audio-nohint"], &[("SDL_AUDIODRIVER", "")])?;
    if no_hint != [no_hint_expected] {
        bail!("no-hint audio driver probe failed: {:?}", no_hint);
    }

    for driver in ["pulseaudio", "dsp", "disk", "dummy"] {
        let explicit = run_driver_probe(&probe, &["init-audio-explicit", driver], &[])?;
        if explicit != [driver.to_string()] {
            bail!(
                "explicit audio driver probe for {driver} failed: {:?}",
                explicit
            );
        }
    }

    Ok(())
}

fn contract_selected_without_hint(
    contract: &DriverFamilyContract,
    driver_name: &str,
) -> Result<String> {
    contract
        .single_backend_expectations
        .iter()
        .find(|expectation| expectation.driver_name == driver_name)
        .ok_or_else(|| anyhow!("missing single_backend_expectations entry for {driver_name}"))?
        .selected_without_hint
        .clone()
        .ok_or_else(|| anyhow!("selected_without_hint missing for {driver_name}"))
}

fn install_public_headers(
    repo_root: &Path,
    original_dir: &Path,
    stage_root: &Path,
    inventory: &PublicHeaderInventory,
) -> Result<()> {
    for header in &inventory.headers {
        let destination = stage_root.join(&header.install_relpath);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = match header.header_name.as_str() {
            "SDL_config.h" => fs::read(original_dir.join("debian/SDL_config.h"))?,
            "SDL_revision.h" => generate_sdl_revision_header().into_bytes(),
            _ => {
                let source = repo_root.join(&header.source_path);
                fs::read(&source).with_context(|| format!("read {}", source.display()))?
            }
        };
        fs::write(&destination, contents)
            .with_context(|| format!("write {}", destination.display()))?;
    }
    Ok(())
}

fn install_multiarch_headers(stage_root: &Path) -> Result<()> {
    let multiarch_dir = stage_root.join(format!("usr/include/{UBUNTU_MULTIARCH}/SDL2"));
    fs::create_dir_all(&multiarch_dir)?;
    fs::write(
        multiarch_dir.join("_real_SDL_config.h"),
        generate_real_sdl_config(),
    )?;
    for name in ["SDL_platform.h", "begin_code.h", "close_code.h"] {
        let link = multiarch_dir.join(name);
        if link.exists() {
            fs::remove_file(&link)?;
        }
        std::os::unix::fs::symlink(format!("../../SDL2/{name}"), &link)?;
    }
    Ok(())
}

fn install_pkg_config(original_dir: &Path, stage_root: &Path) -> Result<()> {
    let template = fs::read_to_string(original_dir.join("sdl2.pc.in"))?;
    let rendered = template
        .replace("@prefix@", "${pcfiledir}/../../..")
        .replace("@exec_prefix@", "${prefix}")
        .replace("@libdir@", &format!("${{prefix}}/lib/{UBUNTU_MULTIARCH}"))
        .replace("@includedir@", "${prefix}/include")
        .replace("@SDL_VERSION@", SDL_VERSION)
        .replace("@PKGCONFIG_DEPENDS@", "")
        .replace("@SDL_RLD_FLAGS@", "")
        .replace("@SDL_LIBS@", "-lSDL2")
        .replace("@PKGCONFIG_LIBS_PRIV@", "")
        .replace("@SDL_STATIC_LIBS@", &static_private_link_flags())
        .replace("@SDL_CFLAGS@", "");
    let destination = stage_root.join(format!("usr/lib/{UBUNTU_MULTIARCH}/pkgconfig/sdl2.pc"));
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(destination, rendered)?;
    Ok(())
}

fn install_sdl2_config_script(stage_root: &Path) -> Result<()> {
    let destination = stage_root.join("usr/bin/sdl2-config");
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let script = format!(
        "#!/bin/sh\nset -eu\nbindir=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\nprefix=$(CDPATH= cd -- \"$bindir/..\" && pwd)\nexec_prefix=\"$prefix\"\nexec_prefix_set=no\nusage='Usage: $0 [--prefix[=DIR]] [--exec-prefix[=DIR]] [--version] [--cflags] [--libs] [--static-libs]'\nif [ \"$#\" -eq 0 ]; then\n  echo \"$usage\" >&2\n  exit 1\nfi\noutput=''\nappend_output() {{\n  if [ -n \"$output\" ]; then\n    output=\"$output $1\"\n  else\n    output=\"$1\"\n  fi\n}}\nwhile [ \"$#\" -gt 0 ]; do\n  case \"$1\" in\n    --prefix=*)\n      prefix=${{1#--prefix=}}\n      if [ \"$exec_prefix_set\" = no ]; then\n        exec_prefix=\"$prefix\"\n      fi\n      ;;\n    --prefix)\n      append_output \"$prefix\"\n      ;;\n    --exec-prefix=*)\n      exec_prefix=${{1#--exec-prefix=}}\n      exec_prefix_set=yes\n      ;;\n    --exec-prefix)\n      append_output \"$exec_prefix\"\n      ;;\n    --version)\n      append_output '{version}'\n      ;;\n    --cflags)\n      append_output \"-I$prefix/include/SDL2\"\n      ;;\n    --libs)\n      append_output \"-L$prefix/lib/{triplet} -lSDL2\"\n      ;;\n    --static-libs)\n      append_output \"$prefix/lib/{triplet}/libSDL2.a {static_private}\"\n      ;;\n    *)\n      echo \"$usage\" >&2\n      exit 1\n      ;;\n  esac\n  shift\ndone\nprintf '%s\\n' \"$output\"\n",
        triplet = UBUNTU_MULTIARCH,
        version = SDL_VERSION,
        static_private = static_private_link_flags(),
    );
    fs::write(&destination, script)?;
    let mut perms = fs::metadata(&destination)?.permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    fs::set_permissions(destination, perms)?;
    Ok(())
}

fn install_m4(original_dir: &Path, stage_root: &Path) -> Result<()> {
    let destination = stage_root.join("usr/share/aclocal/sdl2.m4");
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(original_dir.join("sdl2.m4"), destination)?;
    Ok(())
}

fn install_cmake_surface(original_dir: &Path, stage_root: &Path) -> Result<()> {
    let cmake_dir = stage_root.join(format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2"));
    fs::create_dir_all(&cmake_dir)?;

    let lower = render_lowercase_cmake_config(original_dir)?;
    let lower_version = render_lowercase_cmake_version(original_dir)?;
    fs::write(cmake_dir.join("sdl2-config.cmake"), &lower)?;
    fs::write(cmake_dir.join("sdl2-config-version.cmake"), &lower_version)?;
    fs::write(
        cmake_dir.join("SDL2Config.cmake"),
        render_uppercase_cmake_config(),
    )?;
    fs::write(cmake_dir.join("SDL2ConfigVersion.cmake"), &lower_version)?;
    fs::write(
        cmake_dir.join("SDL2Targets.cmake"),
        render_imported_target_export(
            "SDL2::SDL2",
            "SHARED",
            "${CMAKE_CURRENT_LIST_DIR}/../../libSDL2.so",
            &[],
        ),
    )?;
    fs::write(
        cmake_dir.join("SDL2staticTargets.cmake"),
        render_imported_target_export(
            "SDL2::SDL2-static",
            "STATIC",
            "${CMAKE_CURRENT_LIST_DIR}/../../libSDL2.a",
            &["INTERFACE_LINK_LIBRARIES \"dl;m;pthread;rt\""],
        ),
    )?;
    fs::write(
        cmake_dir.join("SDL2mainTargets.cmake"),
        render_imported_target_export(
            "SDL2::SDL2main",
            "STATIC",
            "${CMAKE_CURRENT_LIST_DIR}/../../libSDL2main.a",
            &["INTERFACE_LINK_LIBRARIES \"SDL2::SDL2\""],
        ),
    )?;
    fs::write(
        cmake_dir.join("SDL2testTargets.cmake"),
        render_imported_target_export(
            "SDL2::SDL2test",
            "STATIC",
            "${CMAKE_CURRENT_LIST_DIR}/../../libSDL2_test.a",
            &["INTERFACE_LINK_LIBRARIES \"SDL2::SDL2\""],
        ),
    )?;
    fs::copy(
        original_dir.join("cmake/sdlfind.cmake"),
        cmake_dir.join("sdlfind.cmake"),
    )?;

    Ok(())
}

fn render_lowercase_cmake_config(original_dir: &Path) -> Result<String> {
    let template = fs::read_to_string(original_dir.join("sdl2-config.cmake.in"))?;
    Ok(template
        .replace("@cmake_prefix_relpath@", "../../../..")
        .replace("@exec_prefix@", "${prefix}")
        .replace("@bindir@", "${prefix}/bin")
        .replace("@libdir@", &format!("${{prefix}}/lib/{UBUNTU_MULTIARCH}"))
        .replace("@includedir@", "${prefix}/include")
        .replace("@SDL_LIBS@", "-lSDL2")
        .replace("@SDL_STATIC_LIBS@", &static_private_link_flags())
        .replace("@SDL_VERSION@", SDL_VERSION))
}

fn render_lowercase_cmake_version(original_dir: &Path) -> Result<String> {
    let template = fs::read_to_string(original_dir.join("sdl2-config-version.cmake.in"))?;
    Ok(template.replace("@SDL_VERSION@", SDL_VERSION))
}

fn render_uppercase_cmake_config() -> String {
    [
        "set(SDL2_FOUND TRUE)",
        "",
        "if(EXISTS \"${CMAKE_CURRENT_LIST_DIR}/SDL2Targets.cmake\")",
        "  include(\"${CMAKE_CURRENT_LIST_DIR}/SDL2Targets.cmake\")",
        "  set(SDL2_SDL2_FOUND TRUE)",
        "endif()",
        "if(EXISTS \"${CMAKE_CURRENT_LIST_DIR}/SDL2staticTargets.cmake\")",
        "  if(ANDROID)",
        "    enable_language(CXX)",
        "  endif()",
        "  include(\"${CMAKE_CURRENT_LIST_DIR}/SDL2staticTargets.cmake\")",
        "  set(SDL2_SDL2-static_FOUND TRUE)",
        "endif()",
        "if(EXISTS \"${CMAKE_CURRENT_LIST_DIR}/SDL2mainTargets.cmake\")",
        "  include(\"${CMAKE_CURRENT_LIST_DIR}/SDL2mainTargets.cmake\")",
        "  set(SDL2_SDL2main_FOUND TRUE)",
        "endif()",
        "if(EXISTS \"${CMAKE_CURRENT_LIST_DIR}/SDL2testTargets.cmake\")",
        "  include(\"${CMAKE_CURRENT_LIST_DIR}/SDL2testTargets.cmake\")",
        "  set(SDL2_SDL2test_FOUND TRUE)",
        "endif()",
        "",
        "include(\"${CMAKE_CURRENT_LIST_DIR}/sdlfind.cmake\")",
        "",
        "if(TARGET SDL2::SDL2-static AND NOT TARGET SDL2::SDL2)",
        "  if(CMAKE_VERSION VERSION_LESS \"3.18\")",
        "    add_library(SDL2::SDL2 INTERFACE IMPORTED)",
        "    set_target_properties(SDL2::SDL2 PROPERTIES INTERFACE_LINK_LIBRARIES \"SDL2::SDL2-static\")",
        "  else()",
        "    add_library(SDL2::SDL2 ALIAS SDL2::SDL2-static)",
        "  endif()",
        "endif()",
        "",
        "if(TARGET SDL2::SDL2 AND NOT TARGET SDL2)",
        "  add_library(SDL2 INTERFACE IMPORTED)",
        "  set_target_properties(SDL2 PROPERTIES INTERFACE_LINK_LIBRARIES \"SDL2::SDL2\")",
        "endif()",
        "",
        "get_filename_component(SDL2_PREFIX \"${CMAKE_CURRENT_LIST_DIR}/../../../..\" ABSOLUTE)",
        "set(SDL2_EXEC_PREFIX \"${SDL2_PREFIX}\")",
        "set(SDL2_INCLUDE_DIR \"${SDL2_PREFIX}/include/SDL2\")",
        "set(SDL2_INCLUDE_DIRS \"${SDL2_PREFIX}/include;${SDL2_INCLUDE_DIR}\")",
        "set(SDL2_BINDIR \"${SDL2_PREFIX}/bin\")",
        &format!("set(SDL2_LIBDIR \"${{SDL2_PREFIX}}/lib/{UBUNTU_MULTIARCH}\")"),
        "set(SDL2_LIBRARIES SDL2::SDL2)",
        "set(SDL2_STATIC_LIBRARIES SDL2::SDL2-static)",
        "set(SDL2_STATIC_PRIVATE_LIBS)",
        "",
        "set(SDL2MAIN_LIBRARY)",
        "if(TARGET SDL2::SDL2main)",
        "  set(SDL2MAIN_LIBRARY SDL2::SDL2main)",
        "  list(INSERT SDL2_LIBRARIES 0 SDL2::SDL2main)",
        "  list(INSERT SDL2_STATIC_LIBRARIES 0 SDL2::SDL2main)",
        "endif()",
        "",
        "set(SDL2TEST_LIBRARY)",
        "if(TARGET SDL2::SDL2test)",
        "  set(SDL2TEST_LIBRARY SDL2::SDL2test)",
        "endif()",
        "",
    ]
    .join("\n")
}

fn render_imported_target_export(
    target_name: &str,
    library_kind: &str,
    imported_location: &str,
    extra_properties: &[&str],
) -> String {
    let mut properties = vec![
        format!("    IMPORTED_LOCATION \"{imported_location}\""),
        format!(
            "    INTERFACE_INCLUDE_DIRECTORIES \"${{CMAKE_CURRENT_LIST_DIR}}/../../../../include/SDL2\""
        ),
        "    IMPORTED_LINK_INTERFACE_LANGUAGES \"C\"".to_string(),
    ];
    if target_name == "SDL2::SDL2" {
        properties.push(format!("    IMPORTED_SONAME \"{SDL_SONAME}\""));
    }
    for property in extra_properties {
        properties.push(format!("    {property}"));
    }

    format!(
        "if(NOT TARGET {target_name})\n  add_library({target_name} {library_kind} IMPORTED)\n  set_target_properties({target_name} PROPERTIES\n{}\n  )\nendif()\n",
        properties.join("\n")
    )
}

fn install_helper_archives(repo_root: &Path, stage_root: &Path) -> Result<()> {
    let libdir = stage_root.join(format!("usr/lib/{UBUNTU_MULTIARCH}"));
    fs::create_dir_all(&libdir)?;
    let status = Command::new("cargo")
        .current_dir(repo_root)
        .args([
            "build",
            "--manifest-path",
            "safe/Cargo.toml",
            "-p",
            "safe-sdl2-test",
            "-p",
            "safe-sdl2main",
            "--release",
        ])
        .status()
        .context("run cargo build for safe SDL helper archives")?;
    if !status.success() {
        bail!("cargo build --release for safe SDL helper archives failed");
    }

    fs::copy(
        repo_root.join("safe/target/release/libsafe_sdl2_test.a"),
        libdir.join("libSDL2_test.a"),
    )
    .context("copy Rust-built libSDL2_test.a into stage root")?;
    fs::copy(
        repo_root.join("safe/target/release/libsafe_sdl2main.a"),
        libdir.join("libSDL2main.a"),
    )
    .context("copy Rust-built libSDL2main.a into stage root")?;
    Ok(())
}

fn install_library_artifacts(
    repo_root: &Path,
    stage_root: &Path,
    library_path: Option<&Path>,
) -> Result<()> {
    let (cdylib, staticlib) = match library_path {
        Some(path) => {
            let cdylib = absolutize(repo_root, path);
            let staticlib = cdylib
                .parent()
                .ok_or_else(|| anyhow!("library path has no parent"))?
                .join("libsafe_sdl.a");
            (cdylib, staticlib)
        }
        None => {
            let status = Command::new("cargo")
                .current_dir(repo_root)
                .args([
                    "build",
                    "--manifest-path",
                    "safe/Cargo.toml",
                    "-p",
                    "safe-sdl",
                    "--release",
                ])
                .status()
                .context("run cargo build for safe-sdl")?;
            if !status.success() {
                bail!("cargo build --release for safe-sdl failed");
            }
            (
                repo_root.join("safe/target/release/libsafe_sdl.so"),
                repo_root.join("safe/target/release/libsafe_sdl.a"),
            )
        }
    };

    let libdir = stage_root.join(format!("usr/lib/{UBUNTU_MULTIARCH}"));
    fs::create_dir_all(&libdir)?;

    let runtime_real = libdir.join(SDL_RUNTIME_REALNAME);
    fs::copy(&cdylib, &runtime_real)
        .with_context(|| format!("copy {} to {}", cdylib.display(), runtime_real.display()))?;

    for (link_name, target) in [
        (SDL_SONAME, SDL_RUNTIME_REALNAME),
        ("libSDL2-2.0.so", SDL_SONAME),
        ("libSDL2.so", SDL_SONAME),
    ] {
        let link = libdir.join(link_name);
        if link.exists() {
            fs::remove_file(&link)?;
        }
        std::os::unix::fs::symlink(target, &link)?;
    }

    if staticlib.exists() {
        fs::copy(&staticlib, libdir.join("libSDL2.a"))
            .with_context(|| format!("copy {}", staticlib.display()))?;
    }

    Ok(())
}

fn install_installed_tests(
    repo_root: &Path,
    generated_dir: &Path,
    stage_root: &Path,
) -> Result<()> {
    let build_root = tempdir().context("create installed-tests build tempdir")?;
    let objects_dir = build_root.path().join("objects");
    let linked_dir = build_root.path().join("linked");

    compile_original_test_objects(CompileOriginalTestObjectsArgs {
        repo_root: repo_root.to_path_buf(),
        generated_dir: generated_dir.to_path_buf(),
        object_manifest: None,
        output_dir: objects_dir.clone(),
    })?;
    relink_original_test_objects(RelinkOriginalTestObjectsArgs {
        repo_root: repo_root.to_path_buf(),
        generated_dir: generated_dir.to_path_buf(),
        object_manifest: None,
        standalone_manifest: None,
        objects_dir,
        output_dir: linked_dir.clone(),
        library_path: stage_root.join(format!("usr/lib/{UBUNTU_MULTIARCH}/{SDL_SONAME}")),
    })?;

    let installed_tests_dir = stage_root.join("usr/libexec/installed-tests/SDL2");
    copy_dir_contents(&linked_dir, &installed_tests_dir)?;

    let metadata_dir = stage_root.join("usr/share/installed-tests/SDL2");
    copy_dir_contents(
        &repo_root.join("safe/upstream-tests/installed-tests/usr/share/installed-tests/SDL2"),
        &metadata_dir,
    )?;

    Ok(())
}

fn verify_headers(stage_root: &Path, inventory: &PublicHeaderInventory) -> Result<()> {
    for header in &inventory.headers {
        ensure_exists(&stage_root.join(&header.install_relpath))?;
    }
    Ok(())
}

fn ensure_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("missing staged path {}", path.display());
    }
    Ok(())
}

fn copy_dir_contents(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in
        fs::read_dir(source).with_context(|| format!("read directory {}", source.display()))?
    {
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_contents(&entry_path, &target_path)?;
        } else {
            fs::copy(&entry_path, &target_path).with_context(|| {
                format!("copy {} to {}", entry_path.display(), target_path.display())
            })?;
        }
    }
    Ok(())
}

fn absolutize(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn build_driver_probe(repo_root: &Path, stage_root: &Path) -> Result<PathBuf> {
    let temp = tempdir().context("create driver probe tempdir")?;
    let temp_path = temp.path().to_path_buf();
    std::mem::forget(temp);
    let source = temp_path.join("driver_probe.c");
    let binary = temp_path.join("driver_probe");
    let stage_include_root = stage_root.join("usr/include");
    let stage_header_dir = stage_include_root.join("SDL2");
    let stage_multiarch_include = stage_include_root.join(UBUNTU_MULTIARCH);
    let stage_libdir = stage_root.join(format!("usr/lib/{UBUNTU_MULTIARCH}"));

    fs::write(
        &source,
        r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "SDL.h"

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "missing mode\n");
        return 64;
    }
    if (strcmp(argv[1], "list-video") == 0) {
        const int count = SDL_GetNumVideoDrivers();
        for (int i = 0; i < count; ++i) {
            const char *name = SDL_GetVideoDriver(i);
            puts(name ? name : "");
        }
        return 0;
    }
    if (strcmp(argv[1], "init-video-nohint") == 0) {
        if (SDL_Init(SDL_INIT_VIDEO) != 0) {
            fprintf(stderr, "%s\n", SDL_GetError());
            return 2;
        }
        puts(SDL_GetCurrentVideoDriver());
        SDL_Quit();
        return 0;
    }
    if (strcmp(argv[1], "init-video-explicit") == 0) {
        if (argc < 3) {
            fprintf(stderr, "missing driver name\n");
            return 64;
        }
        if (SDL_VideoInit(argv[2]) != 0) {
            fprintf(stderr, "%s\n", SDL_GetError());
            return 3;
        }
        puts(SDL_GetCurrentVideoDriver());
        SDL_VideoQuit();
        return 0;
    }
    if (strcmp(argv[1], "list-audio") == 0) {
        const int count = SDL_GetNumAudioDrivers();
        for (int i = 0; i < count; ++i) {
            const char *name = SDL_GetAudioDriver(i);
            puts(name ? name : "");
        }
        return 0;
    }
    if (strcmp(argv[1], "init-audio-nohint") == 0) {
        if (SDL_AudioInit(NULL) != 0) {
            fprintf(stderr, "%s\n", SDL_GetError());
            return 4;
        }
        puts(SDL_GetCurrentAudioDriver());
        SDL_AudioQuit();
        return 0;
    }
    if (strcmp(argv[1], "init-audio-explicit") == 0) {
        if (argc < 3) {
            fprintf(stderr, "missing driver name\n");
            return 64;
        }
        if (SDL_AudioInit(argv[2]) != 0) {
            fprintf(stderr, "%s\n", SDL_GetError());
            return 5;
        }
        puts(SDL_GetCurrentAudioDriver());
        SDL_AudioQuit();
        return 0;
    }
    fprintf(stderr, "unknown mode: %s\n", argv[1]);
    return 64;
}
"#,
    )?;

    let output = Command::new("cc")
        .current_dir(repo_root)
        .arg("-o")
        .arg(&binary)
        .arg(&source)
        .arg("-I")
        .arg(&stage_header_dir)
        .arg("-I")
        .arg(&stage_multiarch_include)
        .arg(format!("-L{}", stage_libdir.display()))
        .arg(format!("-Wl,-rpath,{}", stage_libdir.display()))
        .arg("-lSDL2")
        .output()
        .context("compile driver probe")?;
    if !output.status.success() {
        bail!(
            "compiling driver probe failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(binary)
}

fn run_driver_probe(probe: &Path, args: &[&str], envs: &[(&str, &str)]) -> Result<Vec<String>> {
    let result = run_driver_probe_capture(probe, args, envs)?;
    if !result.success {
        bail!("driver probe {:?} failed:\n{}", args, result.stderr);
    }
    Ok(result.stdout)
}

struct DriverProbeOutput {
    success: bool,
    stdout: Vec<String>,
    stderr: String,
}

fn run_driver_probe_capture(
    probe: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<DriverProbeOutput> {
    let mut cmd = Command::new(probe);
    if args.is_empty() {
        cmd.arg("list-video");
    } else {
        cmd.args(args);
    }
    cmd.env_remove("SDL_AUDIODRIVER");
    cmd.env_remove("SDL_VIDEODRIVER");
    cmd.env_remove(real_runtime_env_key());
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let output = cmd
        .output()
        .with_context(|| format!("run {}", probe.display()))?;
    Ok(DriverProbeOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn real_runtime_env_key() -> &'static str {
    concat!("SAFE_SDL_REAL_", "SDL_PATH")
}

struct XvfbGuard {
    child: std::process::Child,
}

impl Drop for XvfbGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_xvfb() -> Option<(Option<XvfbGuard>, String)> {
    for display in 91..100 {
        let display_name = format!(":{display}");
        let child = match Command::new("Xvfb")
            .arg(&display_name)
            .arg("-screen")
            .arg("0")
            .arg("1024x768x24")
            .arg("-nolisten")
            .arg("tcp")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => continue,
        };
        thread::sleep(Duration::from_millis(500));
        return Some((Some(XvfbGuard { child }), display_name));
    }
    None
}

fn static_private_link_flags() -> String {
    "-ldl -lm -pthread -lrt".to_string()
}

#[cfg(test)]
mod tests {
    use super::render_uppercase_cmake_config;

    #[test]
    fn uppercase_cmake_config_is_prefix_relative_and_defines_plain_sdl2_target() {
        let config = render_uppercase_cmake_config();

        assert!(
            config.contains(
                "get_filename_component(SDL2_PREFIX \"${CMAKE_CURRENT_LIST_DIR}/../../../..\" ABSOLUTE)"
            ),
            "config should derive SDL2_PREFIX from its own install location: {config}"
        );
        assert!(
            config.contains("set(SDL2_INCLUDE_DIR \"${SDL2_PREFIX}/include/SDL2\")"),
            "config should expose prefix-relative include directories: {config}"
        );
        assert!(
            config.contains("set(SDL2_LIBDIR \"${SDL2_PREFIX}/lib/"),
            "config should expose a prefix-relative library directory: {config}"
        );
        assert!(
            !config.contains("set(SDL2_PREFIX \"/usr\")"),
            "config must not hardcode /usr: {config}"
        );
        assert!(
            config.contains("if(TARGET SDL2::SDL2 AND NOT TARGET SDL2)"),
            "config should define a plain SDL2 compatibility target: {config}"
        );
        assert!(
            config.contains(
                "set_target_properties(SDL2 PROPERTIES INTERFACE_LINK_LIBRARIES \"SDL2::SDL2\")"
            ),
            "plain SDL2 target should forward to the imported namespaced target: {config}"
        );
    }
}
