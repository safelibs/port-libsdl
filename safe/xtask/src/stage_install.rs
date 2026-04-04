use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::contracts::{
    generate_real_sdl_config, generate_sdl_revision_header, load_install_contract,
    load_public_header_inventory, rel, PublicHeaderInventory, SDL_RUNTIME_REALNAME, SDL_SONAME,
    SDL_VERSION, UBUNTU_MULTIARCH,
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

pub fn stage_install(args: StageInstallArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let original_dir = absolutize(&args.repo_root, &args.original_dir);
    let stage_root = absolutize(&args.repo_root, &args.stage_root);
    let inventory = load_public_header_inventory(&generated_dir.join("public_header_inventory.json"))?;
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
    let inventory = load_public_header_inventory(&generated_dir.join("public_header_inventory.json"))?;
    let install_contract = load_install_contract(&generated_dir.join("install_contract.json"))?;

    verify_headers(&stage_root, &inventory)?;

    for required in [
        format!("usr/lib/{UBUNTU_MULTIARCH}/pkgconfig/sdl2.pc"),
        "usr/bin/sdl2-config".to_string(),
        "usr/share/aclocal/sdl2.m4".to_string(),
        format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2main.a"),
        format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2_test.a"),
    ] {
        ensure_exists(&stage_root.join(&required))?;
    }

    for cmake_path in &install_contract.cmake_surface {
        ensure_exists(&stage_root.join(cmake_path))?;
    }

    Ok(())
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
                fs::read(&source)
                    .with_context(|| format!("read {}", source.display()))?
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
        .replace("@SDL_STATIC_LIBS@", "")
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
        "#!/bin/sh\nset -eu\nprefix=$(CDPATH= cd -- \"$(dirname -- \"$0\")/..\" && pwd)\nexec_prefix=\"$prefix\"\nlibdir=\"$prefix/lib/{triplet}\"\nincludedir=\"$prefix/include\"\ncase \"${{1:-}}\" in\n  --prefix) echo \"$prefix\" ;;\n  --exec-prefix) echo \"$exec_prefix\" ;;\n  --version) echo \"{version}\" ;;\n  --cflags) echo \"-I${{includedir}} -I${{includedir}}/SDL2\" ;;\n  --libs) echo \"-L${{libdir}} -lSDL2\" ;;\n  --static-libs) echo \"-L${{libdir}} -lSDL2main -lSDL2\" ;;\n  *) echo \"usage: $0 [--prefix|--exec-prefix|--version|--cflags|--libs|--static-libs]\" >&2; exit 1 ;;\nesac\n",
        triplet = UBUNTU_MULTIARCH,
        version = SDL_VERSION,
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
        "include(\"${CMAKE_CURRENT_LIST_DIR}/sdl2-config.cmake\")\n",
    )?;
    fs::write(cmake_dir.join("SDL2ConfigVersion.cmake"), &lower_version)?;
    fs::copy(
        original_dir.join("cmake/sdlfind.cmake"),
        cmake_dir.join("sdlfind.cmake"),
    )?;

    Ok(())
}

fn render_lowercase_cmake_config(original_dir: &Path) -> Result<String> {
    let template = fs::read_to_string(original_dir.join("sdl2-config.cmake.in"))?;
    Ok(template
        .replace("@cmake_prefix_relpath@", "../../..")
        .replace("@exec_prefix@", "${prefix}")
        .replace("@bindir@", "${prefix}/bin")
        .replace("@libdir@", &format!("${{prefix}}/lib/{UBUNTU_MULTIARCH}"))
        .replace("@includedir@", "${prefix}/include")
        .replace("@SDL_LIBS@", "-lSDL2")
        .replace("@SDL_STATIC_LIBS@", "")
        .replace("@SDL_VERSION@", SDL_VERSION))
}

fn render_lowercase_cmake_version(original_dir: &Path) -> Result<String> {
    let template = fs::read_to_string(original_dir.join("sdl2-config-version.cmake.in"))?;
    Ok(template.replace("@SDL_VERSION@", SDL_VERSION))
}

fn install_helper_archives(repo_root: &Path, original_dir: &Path, stage_root: &Path) -> Result<()> {
    let tempdir = tempfile::tempdir().context("create helper archive tempdir")?;
    let include_dir = tempdir.path().join("include");
    fs::create_dir_all(&include_dir)?;
    fs::write(include_dir.join("SDL_config.h"), generate_real_sdl_config())?;
    fs::write(include_dir.join("SDL_revision.h"), generate_sdl_revision_header())?;

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

fn compile_c_object(repo_root: &Path, source: &Path, output: &Path, includes: &[String]) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut cmd = Command::new("cc");
    cmd.current_dir(repo_root).arg("-c").arg(source).arg("-o").arg(output);
    for include in includes {
        cmd.arg(include);
    }
    let output_result = cmd.output().with_context(|| format!("compile {}", source.display()))?;
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
    let output = cmd.output().with_context(|| format!("archive {}", archive.display()))?;
    if !output.status.success() {
        bail!(
            "archiving {} failed:\n{}",
            archive.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn install_library_artifacts(repo_root: &Path, stage_root: &Path, library_path: Option<&Path>) -> Result<()> {
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
