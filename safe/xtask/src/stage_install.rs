use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use tempfile::tempdir;

use crate::contracts::{
    generate_real_sdl_config, generate_sdl_revision_header, load_driver_contract,
    load_install_contract, load_public_header_inventory, rel, DriverFamilyContract,
    PublicHeaderInventory, SDL_RUNTIME_REALNAME, SDL_SONAME, SDL_VERSION, UBUNTU_MULTIARCH,
};

pub struct StageInstallArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub original_dir: PathBuf,
    pub stage_root: PathBuf,
    pub library_path: Option<PathBuf>,
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
    install_helper_archives(&args.repo_root, &original_dir, &stage_root)?;
    install_library_artifacts(&args.repo_root, &stage_root, args.library_path.as_deref())?;

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
        .cloned()
    {
        ensure_exists(&stage_root.join(&required))?;
    }

    for cmake_path in &install_contract.cmake_surface {
        ensure_exists(&stage_root.join(cmake_path))?;
    }

    Ok(())
}

pub fn verify_driver_contract(args: VerifyDriverContractArgs) -> Result<()> {
    if args.kind != "video" {
        bail!("unsupported driver contract kind {}", args.kind);
    }

    let contract_path = absolutize(&args.repo_root, &args.contract_path);
    let stage_root = absolutize(&args.repo_root, &args.stage_root);
    let driver_contract = load_driver_contract(&contract_path)?;
    validate_video_contract(&driver_contract.video)?;

    let expected = driver_contract
        .video
        .registry_order
        .iter()
        .map(|entry| entry.driver_name.clone())
        .collect::<Vec<_>>();
    let probe = build_driver_probe(&args.repo_root, &stage_root)?;

    let listed = run_driver_probe(&probe, &[], &[])?;
    if listed != expected {
        bail!(
            "video driver registry mismatch\nexpected: {:?}\nactual: {:?}",
            expected,
            listed
        );
    }

    let dummy_expected = contract_selected_without_hint(&driver_contract.video, "dummy")?;
    let dummy = run_driver_probe(&probe, &["init-nohint"], &[("SDL_VIDEODRIVER", "dummy")])?;
    if dummy != [dummy_expected] {
        bail!("explicit dummy driver probe failed: {:?}", dummy);
    }

    let offscreen_expected = contract_selected_without_hint(&driver_contract.video, "offscreen")?;
    let offscreen = run_driver_probe(
        &probe,
        &["init-nohint"],
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
        let x11_expected = contract_selected_without_hint(&driver_contract.video, "x11")?;
        let env = [("DISPLAY", display.as_str())];
        let no_hint = run_driver_probe(&probe, &["init-nohint"], &env)?;
        if no_hint != [x11_expected.clone()] {
            bail!(
                "no-hint video probe did not match contract under X11/Xvfb: {:?}",
                no_hint
            );
        }

        let explicit_x11 = run_driver_probe(
            &probe,
            &["init-nohint"],
            &[("DISPLAY", display.as_str()), ("SDL_VIDEODRIVER", "x11")],
        )?;
        if explicit_x11 != [x11_expected] {
            bail!(
                "explicit x11 driver probe did not match contract under X11/Xvfb: {:?}",
                explicit_x11
            );
        }
    }

    Ok(())
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
        "include(\"${CMAKE_CURRENT_LIST_DIR}/sdl2-config.cmake\")",
        "foreach(_sdl2_targets_file SDL2Targets.cmake SDL2staticTargets.cmake SDL2mainTargets.cmake SDL2testTargets.cmake)",
        "  if(EXISTS \"${CMAKE_CURRENT_LIST_DIR}/${_sdl2_targets_file}\")",
        "    include(\"${CMAKE_CURRENT_LIST_DIR}/${_sdl2_targets_file}\")",
        "  endif()",
        "endforeach()",
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

fn install_helper_archives(repo_root: &Path, original_dir: &Path, stage_root: &Path) -> Result<()> {
    let tempdir = tempfile::tempdir().context("create helper archive tempdir")?;
    let include_dir = tempdir.path().join("include");
    fs::create_dir_all(&include_dir)?;
    fs::write(include_dir.join("SDL_config.h"), generate_real_sdl_config())?;
    fs::write(
        include_dir.join("SDL_revision.h"),
        generate_sdl_revision_header(),
    )?;

    let object_dir = tempdir.path().join("objects");
    fs::create_dir_all(&object_dir)?;
    let libdir = stage_root.join(format!("usr/lib/{UBUNTU_MULTIARCH}"));
    fs::create_dir_all(&libdir)?;

    let dummy_main_obj = object_dir.join("SDL_dummy_main.o");
    compile_c_object(
        repo_root,
        &original_dir.join("src/main/dummy/SDL_dummy_main.c"),
        &dummy_main_obj,
        &[
            format!("-I{}", include_dir.display()),
            format!("-I{}", original_dir.join("include").display()),
            format!("-I{}", original_dir.join("src").display()),
        ],
    )?;
    archive_objects(&libdir.join("libSDL2main.a"), &[dummy_main_obj])?;

    let mut test_objects = Vec::new();
    for entry in fs::read_dir(original_dir.join("src/test"))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(std::ffi::OsStr::new("c")) {
            continue;
        }
        let object = object_dir.join(format!(
            "{}.o",
            path.file_stem().unwrap_or_default().to_string_lossy()
        ));
        compile_c_object(
            repo_root,
            &path,
            &object,
            &[
                format!("-I{}", include_dir.display()),
                format!("-I{}", original_dir.join("include").display()),
                format!("-I{}", original_dir.join("src/test").display()),
            ],
        )?;
        test_objects.push(object);
    }
    test_objects.sort();
    archive_objects(&libdir.join("libSDL2_test.a"), &test_objects)?;
    Ok(())
}

fn compile_c_object(
    repo_root: &Path,
    source: &Path,
    output: &Path,
    includes: &[String],
) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut cmd = Command::new("cc");
    cmd.current_dir(repo_root)
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(output);
    for include in includes {
        cmd.arg(include);
    }
    let output_result = cmd
        .output()
        .with_context(|| format!("compile {}", source.display()))?;
    if !output_result.status.success() {
        bail!(
            "compiling {} failed:\n{}",
            rel(repo_root, source),
            String::from_utf8_lossy(&output_result.stderr)
        );
    }
    Ok(())
}

fn archive_objects(archive: &Path, objects: &[PathBuf]) -> Result<()> {
    let mut cmd = Command::new("ar");
    cmd.arg("crs").arg(archive);
    for object in objects {
        cmd.arg(object);
    }
    let output = cmd
        .output()
        .with_context(|| format!("archive {}", archive.display()))?;
    if !output.status.success() {
        bail!(
            "archiving {} failed:\n{}",
            archive.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
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
    if (strcmp(argv[1], "list") == 0) {
        const int count = SDL_GetNumVideoDrivers();
        for (int i = 0; i < count; ++i) {
            const char *name = SDL_GetVideoDriver(i);
            puts(name ? name : "");
        }
        return 0;
    }
    if (strcmp(argv[1], "init-nohint") == 0) {
        if (SDL_Init(SDL_INIT_VIDEO) != 0) {
            fprintf(stderr, "%s\n", SDL_GetError());
            return 2;
        }
        puts(SDL_GetCurrentVideoDriver());
        SDL_Quit();
        return 0;
    }
    if (strcmp(argv[1], "init-explicit") == 0) {
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
    let mut cmd = Command::new(probe);
    if args.is_empty() {
        cmd.arg("list");
    } else {
        cmd.args(args);
    }
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let output = cmd
        .output()
        .with_context(|| format!("run {}", probe.display()))?;
    if !output.status.success() {
        bail!(
            "driver probe {:?} failed:\n{}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
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
        let child = Command::new("Xvfb")
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
            .ok()?;
        thread::sleep(Duration::from_millis(500));
        return Some((Some(XvfbGuard { child }), display_name));
    }
    None
}

fn static_private_link_flags() -> String {
    "-ldl -lm -pthread -lrt".to_string()
}
