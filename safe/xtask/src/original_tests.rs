use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use tempfile::tempdir;

use crate::contracts::{
    generate_real_sdl_config, generate_sdl_revision_header, load_original_test_object_manifest,
    load_standalone_test_manifest,
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

pub fn compile_original_test_objects(args: CompileOriginalTestObjectsArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let output_dir = absolutize(&args.repo_root, &args.output_dir);
    let manifest = load_original_test_object_manifest(&generated_dir.join("original_test_object_manifest.json"))?;

    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    let include_temp = tempdir().context("create generated include tempdir")?;
    let generated_include_dir = include_temp.path();
    fs::write(generated_include_dir.join("SDL_config.h"), generate_real_sdl_config())?;
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
    let manifest = load_original_test_object_manifest(&generated_dir.join("original_test_object_manifest.json"))?;

    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    for target in manifest.targets.iter().filter(|target| target.ubuntu_24_04_enabled) {
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
    let standalone = load_standalone_test_manifest(&generated_dir.join("standalone_test_manifest.json"))?;

    for target in standalone.targets.iter().filter(|target| target.ci_validation_mode == "auto_run") {
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

fn resolve_token(value: &str, generated_include_dir: &Path, stage_libdir: Option<&Path>) -> Result<String> {
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
