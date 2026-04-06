use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use serde::Serialize;
use tempfile::tempdir;

use crate::contracts::{
    abi_check, load_install_contract, load_public_header_inventory, verify_captured_contracts,
    verify_test_port_map, AbiCheckArgs, ContractArgs, PublicHeaderInventory, SDL_SONAME,
    UBUNTU_MULTIARCH,
};
use crate::dependents::{verify_dependent_regressions, VerifyDependentRegressionsArgs};
use crate::original_tests::{
    compile_original_test_objects, relink_original_test_objects, run_relinked_original_tests,
    CompileOriginalTestObjectsArgs, RelinkOriginalTestObjectsArgs, RunRelinkedOriginalTestsArgs,
};
use crate::stage_install::{verify_driver_contract, VerifyDriverContractArgs};

const PHASE_10_ID: &str = "impl_phase_10_packaging_dependents_final";
const DISTRO_SDL_PACKAGE_VERSION: &str = "2.30.0+dfsg-1ubuntu3.1";
#[derive(Debug)]
pub struct FinalCheckArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub original_dir: PathBuf,
    pub dependents_path: PathBuf,
    pub cves_path: PathBuf,
    pub relink_objects_dir: PathBuf,
    pub relink_bin_dir: PathBuf,
    pub unsafe_allowlist: PathBuf,
    pub phase_report: PathBuf,
    pub unsafe_report: PathBuf,
    pub dependent_regression_manifest: PathBuf,
    pub dependent_matrix_results: PathBuf,
    pub dependent_matrix_artifact_dir: PathBuf,
}

#[derive(Debug)]
pub struct VerifyInstallContractArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub original_dir: PathBuf,
    pub package_root: PathBuf,
    pub install_contract_path: Option<PathBuf>,
    pub public_header_inventory_path: Option<PathBuf>,
    pub mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UnsafeAuditRule {
    pattern: String,
    category: String,
    justification: String,
}

#[derive(Debug, Serialize)]
pub struct UnsafeAuditEntry {
    path: String,
    category: String,
    matched_pattern: String,
}

#[derive(Debug, Serialize)]
pub struct UnsafeAuditSummary {
    generated_at_unix_seconds: u64,
    rule_count: usize,
    total_unsafe_files: usize,
    undocumented_files: usize,
    category_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
pub struct UnsafeAuditReport {
    phase_id: String,
    allowlist_path: String,
    summary: UnsafeAuditSummary,
    rules: Vec<UnsafeAuditRule>,
    entries: Vec<UnsafeAuditEntry>,
}

#[derive(Debug, Serialize)]
struct FinalPhaseReport {
    phase_id: String,
    package_version: String,
    built_packages: BTreeMap<String, String>,
    installed_library: String,
    unsafe_audit_report: String,
    dependent_regression_manifest: String,
    dependent_matrix_results: String,
    dependent_matrix_artifact_dir: String,
}

pub fn final_check(args: FinalCheckArgs) -> Result<()> {
    ensure_root()?;

    verify_captured_contracts(ContractArgs {
        repo_root: args.repo_root.clone(),
        generated_dir: args.generated_dir.clone(),
        original_dir: args.original_dir.clone(),
        dependents_path: args.dependents_path.clone(),
        cves_path: args.cves_path.clone(),
    })?;
    verify_test_port_map(
        &args.repo_root,
        &args.generated_dir.join("original_test_port_map.json"),
        &args.original_dir,
        Some(116),
        Some(71),
    )?;

    run_command(
        command_in(&args.repo_root, "apt-get", ["update"]),
        "apt-get update",
    )?;
    run_command(
        command_in(
            &args.repo_root,
            "apt-get",
            [
                "install",
                "-y",
                "--no-install-recommends",
                "build-essential",
                "clang",
                "cmake",
                "pkg-config",
                "dpkg-dev",
                "jq",
            ],
        ),
        "install checker prerequisites",
    )?;
    run_command(
        command_in(&args.repo_root, "apt-get", ["build-dep", "-y", "./safe"]),
        "install safe build-deps",
    )?;
    restore_system_sdl_packages(&args.repo_root)?;
    run_cargo_test(&args.repo_root, "upstream_port_core")?;

    build_safe_packages(&args.repo_root)?;
    let built_packages = locate_built_packages(&args.repo_root)?;
    let package_version = read_deb_field(
        built_packages
            .get("libsdl2-2.0-0")
            .ok_or_else(|| anyhow!("missing libsdl2-2.0-0 package"))?,
        "Version",
    )?;
    install_safe_packages(&args.repo_root, &built_packages)?;

    verify_packaged_install(&args.repo_root, &args.generated_dir, &args.original_dir)?;
    verify_driver_contract(VerifyDriverContractArgs {
        repo_root: args.repo_root.clone(),
        contract_path: args.generated_dir.join("driver_contract.json"),
        stage_root: PathBuf::from("/"),
        kind: "video".to_string(),
    })?;
    verify_driver_contract(VerifyDriverContractArgs {
        repo_root: args.repo_root.clone(),
        contract_path: args.generated_dir.join("driver_contract.json"),
        stage_root: PathBuf::from("/"),
        kind: "audio".to_string(),
    })?;
    run_original_consumer_script(&args.repo_root, &args.original_dir, "build")?;
    run_original_consumer_script(&args.repo_root, &args.original_dir, "cmake")?;
    run_safe_consumer_script(&args.repo_root, "build")?;
    run_safe_consumer_script(&args.repo_root, "cmake")?;
    run_safe_consumer_script(&args.repo_root, "deprecated-use")?;
    run_packaged_autopkgtests(&args.repo_root)?;

    compile_original_test_objects(CompileOriginalTestObjectsArgs {
        repo_root: args.repo_root.clone(),
        generated_dir: args.generated_dir.clone(),
        object_manifest: None,
        output_dir: args.relink_objects_dir.clone(),
    })?;
    relink_original_test_objects(RelinkOriginalTestObjectsArgs {
        repo_root: args.repo_root.clone(),
        generated_dir: args.generated_dir.clone(),
        object_manifest: None,
        standalone_manifest: None,
        objects_dir: args.relink_objects_dir.clone(),
        output_dir: args.relink_bin_dir.clone(),
        library_path: PathBuf::from(format!("/usr/lib/{UBUNTU_MULTIARCH}/{SDL_SONAME}")),
    })?;
    run_relinked_original_tests(RunRelinkedOriginalTestsArgs {
        repo_root: args.repo_root.clone(),
        generated_dir: args.generated_dir.clone(),
        standalone_manifest: args.generated_dir.join("standalone_test_manifest.json"),
        bin_dir: args.relink_bin_dir.clone(),
        filter: None,
        validation_modes: vec!["auto_run".to_string(), "fixture_run".to_string()],
        skip_if_empty: false,
    })?;
    run_test_original_matrix(
        &args.repo_root,
        &args.dependent_matrix_results,
        &args.dependent_matrix_artifact_dir,
    )?;
    verify_dependent_regressions(VerifyDependentRegressionsArgs {
        repo_root: args.repo_root.clone(),
        dependents_path: args.dependents_path.clone(),
        manifest_path: args.dependent_regression_manifest.clone(),
        results_path: args.dependent_matrix_results.clone(),
    })?;
    run_security_regressions(&args.repo_root)?;

    let unsafe_report =
        verify_unsafe_allowlist(&args.repo_root, &args.unsafe_allowlist, &args.unsafe_report)?;
    write_json_report(
        &args.phase_report,
        &FinalPhaseReport {
            phase_id: PHASE_10_ID.to_string(),
            package_version,
            built_packages: built_packages
                .iter()
                .map(|(name, path)| (name.clone(), rel(&args.repo_root, path)))
                .collect(),
            installed_library: format!("/usr/lib/{UBUNTU_MULTIARCH}/{SDL_SONAME}"),
            unsafe_audit_report: rel(&args.repo_root, &args.unsafe_report),
            dependent_regression_manifest: rel(
                &args.repo_root,
                &args.dependent_regression_manifest,
            ),
            dependent_matrix_results: rel(&args.repo_root, &args.dependent_matrix_results),
            dependent_matrix_artifact_dir: rel(
                &args.repo_root,
                &args.dependent_matrix_artifact_dir,
            ),
        },
    )?;

    let _ = unsafe_report;
    Ok(())
}

pub fn verify_unsafe_allowlist(
    repo_root: &Path,
    allowlist_path: &Path,
    report_path: &Path,
) -> Result<UnsafeAuditReport> {
    let allowlist = parse_unsafe_allowlist(allowlist_path)?;
    let files = collect_unsafe_files(repo_root)?;
    let mut entries = Vec::new();
    let mut category_counts = BTreeMap::<String, usize>::new();

    for file in files {
        let Some(rule) = allowlist
            .iter()
            .find(|rule| pattern_matches(&rule.pattern, &file))
        else {
            bail!(
                "unsafe occurrence in {} is not covered by {}",
                file,
                allowlist_path.display()
            );
        };
        entries.push(UnsafeAuditEntry {
            path: file,
            category: rule.category.clone(),
            matched_pattern: rule.pattern.clone(),
        });
        *category_counts.entry(rule.category.clone()).or_default() += 1;
    }

    let report = UnsafeAuditReport {
        phase_id: PHASE_10_ID.to_string(),
        allowlist_path: allowlist_path.display().to_string(),
        summary: UnsafeAuditSummary {
            generated_at_unix_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("compute unsafe audit timestamp")?
                .as_secs(),
            rule_count: allowlist.len(),
            total_unsafe_files: entries.len(),
            undocumented_files: 0,
            category_counts,
        },
        rules: allowlist,
        entries,
    };
    write_json_report(report_path, &report)?;
    Ok(report)
}

fn build_safe_packages(repo_root: &Path) -> Result<()> {
    let mut cmd = Command::new("dpkg-buildpackage");
    cmd.current_dir(repo_root.join("safe"))
        .args(["-us", "-uc", "-b"]);
    apply_repo_rust_toolchain_env(&mut cmd);
    run_command(cmd, "build safe Debian packages")
}

fn install_safe_packages(repo_root: &Path, packages: &BTreeMap<String, PathBuf>) -> Result<()> {
    let runtime = packages
        .get("libsdl2-2.0-0")
        .ok_or_else(|| anyhow!("missing built libsdl2-2.0-0 package"))?;
    let dev = packages
        .get("libsdl2-dev")
        .ok_or_else(|| anyhow!("missing built libsdl2-dev package"))?;
    let tests = packages
        .get("libsdl2-tests")
        .ok_or_else(|| anyhow!("missing built libsdl2-tests package"))?;
    let install_prereqs_once = |repo_root: &Path| {
        let mut cmd = Command::new("apt-get");
        cmd.current_dir(repo_root).args([
            "install",
            "-y",
            "--no-install-recommends",
            "libsdl2-ttf-dev",
            "gnome-desktop-testing",
        ]);
        run_command(cmd, "install autopkgtest prerequisites")
    };
    let install_packages_once = |repo_root: &Path| {
        let mut cmd = Command::new("dpkg");
        cmd.current_dir(repo_root)
            .arg("-i")
            .arg(runtime)
            .arg(dev)
            .arg(tests);
        run_command(cmd, "install safe Debian packages")
    };

    install_prereqs_once(repo_root)?;
    if let Err(error) = install_packages_once(repo_root) {
        let mut repair = Command::new("dpkg");
        repair.current_dir(repo_root).args(["--configure", "-a"]);
        let _ = run_command(repair, "repair dpkg state before retrying package install");
        install_packages_once(repo_root).with_context(|| format!("{error:#}"))?;
    }
    Ok(())
}

fn installed_distro_sdl_archive(package: &str) -> Result<PathBuf> {
    let mut arch_cmd = Command::new("dpkg");
    arch_cmd.arg("--print-architecture");
    let arch = output_command(arch_cmd, "read Debian architecture")?;
    Ok(PathBuf::from("/var/cache/apt/archives").join(format!(
        "{package}_{DISTRO_SDL_PACKAGE_VERSION}_{}.deb",
        arch.trim()
    )))
}

fn restore_system_sdl_packages(repo_root: &Path) -> Result<()> {
    let runtime_version = installed_package_version("libsdl2-2.0-0")?;
    let dev_version = installed_package_version("libsdl2-dev")?;
    let tests_version = installed_package_version("libsdl2-tests")?;
    let needs_restore = runtime_version
        .as_deref()
        .is_some_and(|version| version.contains("safelibs"))
        || dev_version
            .as_deref()
            .is_some_and(|version| version.contains("safelibs"))
        || tests_version
            .as_deref()
            .is_some_and(|version| version.contains("safelibs"));
    if !needs_restore {
        return Ok(());
    }

    let remove_tests_once = |repo_root: &Path| {
        let mut cmd = Command::new("dpkg");
        cmd.current_dir(repo_root)
            .args(["--remove", "libsdl2-tests"]);
        run_command(
            cmd,
            "remove previously installed safe SDL tests package before final staged verification",
        )
    };
    if tests_version
        .as_deref()
        .is_some_and(|version| version.contains("safelibs"))
    {
        if let Err(error) = remove_tests_once(repo_root) {
            let mut repair = Command::new("dpkg");
            repair.current_dir(repo_root).args(["--configure", "-a"]);
            let _ = run_command(
                repair,
                "repair dpkg state before retrying SDL package reset",
            );
            remove_tests_once(repo_root).with_context(|| format!("{error:#}"))?;
        }
    }

    let runtime_archive = installed_distro_sdl_archive("libsdl2-2.0-0")?;
    let dev_archive = installed_distro_sdl_archive("libsdl2-dev")?;
    let download_once = |repo_root: &Path| {
        let mut cmd = Command::new("apt-get");
        cmd.current_dir(repo_root).args([
            "install",
            "-y",
            "--download-only",
            "--allow-downgrades",
            "--no-install-recommends",
            &format!("libsdl2-2.0-0={DISTRO_SDL_PACKAGE_VERSION}"),
            &format!("libsdl2-dev={DISTRO_SDL_PACKAGE_VERSION}"),
        ]);
        run_command(
            cmd,
            "download distro SDL runtime and development packages before final staged verification",
        )
    };
    let restore_once = |repo_root: &Path| {
        let mut cmd = Command::new("dpkg");
        cmd.current_dir(repo_root)
            .arg("-i")
            .arg(&runtime_archive)
            .arg(&dev_archive);
        run_command(
            cmd,
            "restore distro SDL runtime and development packages before final staged verification",
        )
    };
    download_once(repo_root)?;
    if let Err(error) = restore_once(repo_root) {
        let mut repair = Command::new("dpkg");
        repair.current_dir(repo_root).args(["--configure", "-a"]);
        let _ = run_command(
            repair,
            "repair dpkg state before retrying SDL package restore",
        );
        restore_once(repo_root).with_context(|| format!("{error:#}"))?;
    }
    Ok(())
}

fn verify_packaged_install(
    repo_root: &Path,
    generated_dir: &Path,
    original_dir: &Path,
) -> Result<()> {
    let install_contract = load_install_contract(&generated_dir.join("install_contract.json"))?;
    let header_inventory =
        load_public_header_inventory(&generated_dir.join("public_header_inventory.json"))?;
    let runtime_files = dpkg_list("libsdl2-2.0-0")?;
    let dev_files = dpkg_list("libsdl2-dev")?;
    let tests_files = dpkg_list("libsdl2-tests")?;

    verify_original_install_patterns(
        original_dir,
        "debian/libsdl2-2.0-0.install",
        &runtime_files,
        UBUNTU_MULTIARCH,
    )?;
    verify_original_install_patterns(
        original_dir,
        "debian/libsdl2-dev.install",
        &dev_files,
        UBUNTU_MULTIARCH,
    )?;
    verify_original_install_patterns(
        original_dir,
        "debian/libsdl2-tests.install",
        &tests_files,
        UBUNTU_MULTIARCH,
    )?;

    for path in &install_contract.runtime_paths {
        ensure_packaged_path(&runtime_files, path)?;
    }
    for path in &install_contract.dev_paths {
        ensure_packaged_path(&dev_files, path)?;
    }
    for path in &install_contract.cmake_surface {
        ensure_packaged_path(&dev_files, path)?;
    }
    for path in &install_contract.multiarch_include_paths {
        ensure_packaged_path(&dev_files, path)?;
    }
    verify_public_headers(&dev_files, &header_inventory)?;
    verify_authoritative_test_headers(original_dir, &dev_files)?;
    verify_installed_sdl_config(original_dir)?;
    verify_debian_multiarch_symlinks(original_dir)?;
    verify_cmake_surface()?;
    verify_installed_tests_payload(&tests_files, &install_contract)?;
    verify_static_link_surface(repo_root)?;
    verify_installed_abi(repo_root, original_dir)?;
    Ok(())
}

pub fn verify_install_contract(args: VerifyInstallContractArgs) -> Result<()> {
    let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
    let original_dir = absolutize(&args.repo_root, &args.original_dir);
    let package_root = absolutize(&args.repo_root, &args.package_root);
    if let Some(mode) = args.mode.as_deref() {
        match mode {
            "packaged" | "package" | "staged" | "stage" => {}
            _ => bail!("unsupported verify-install-contract mode {mode}"),
        }
    }
    let install_contract = load_install_contract(
        &args
            .install_contract_path
            .as_ref()
            .map(|path| absolutize(&args.repo_root, path))
            .unwrap_or_else(|| generated_dir.join("install_contract.json")),
    )?;
    let header_inventory = load_public_header_inventory(
        &args
            .public_header_inventory_path
            .as_ref()
            .map(|path| absolutize(&args.repo_root, path))
            .unwrap_or_else(|| generated_dir.join("public_header_inventory.json")),
    )?;
    let installed_files =
        collect_relevant_rooted_files(&package_root, &install_contract, &header_inventory)?;

    verify_original_install_patterns(
        &original_dir,
        "debian/libsdl2-2.0-0.install",
        &installed_files,
        UBUNTU_MULTIARCH,
    )?;
    verify_original_install_patterns(
        &original_dir,
        "debian/libsdl2-dev.install",
        &installed_files,
        UBUNTU_MULTIARCH,
    )?;
    verify_original_install_patterns(
        &original_dir,
        "debian/libsdl2-tests.install",
        &installed_files,
        UBUNTU_MULTIARCH,
    )?;

    for path in &install_contract.runtime_paths {
        ensure_rooted_packaged_path(&package_root, &installed_files, path)?;
    }
    for path in &install_contract.dev_paths {
        ensure_rooted_packaged_path(&package_root, &installed_files, path)?;
    }
    for path in &install_contract.cmake_surface {
        ensure_rooted_packaged_path(&package_root, &installed_files, path)?;
    }
    for path in &install_contract.multiarch_include_paths {
        ensure_rooted_packaged_path(&package_root, &installed_files, path)?;
    }

    verify_public_headers(&installed_files, &header_inventory)?;
    verify_authoritative_test_headers(&original_dir, &installed_files)?;
    verify_rooted_installed_sdl_config(&original_dir, &package_root)?;
    verify_rooted_debian_multiarch_symlinks(&original_dir, &package_root)?;
    verify_rooted_cmake_surface(&package_root)?;
    verify_rooted_installed_tests_payload(&package_root, &installed_files, &install_contract)?;
    verify_rooted_static_link_surface(&package_root)?;
    Ok(())
}

fn verify_public_headers(
    dev_files: &BTreeSet<String>,
    inventory: &PublicHeaderInventory,
) -> Result<()> {
    for header in &inventory.headers {
        ensure_packaged_path(dev_files, &header.install_relpath)?;
    }
    Ok(())
}

fn verify_authoritative_test_headers(
    original_dir: &Path,
    dev_files: &BTreeSet<String>,
) -> Result<()> {
    let mut authoritative = Vec::new();
    for entry in fs::read_dir(original_dir.join("include"))
        .context("read original include directory for SDL_test headers")?
    {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("SDL_test") && name.ends_with(".h") {
            authoritative.push(name);
        }
    }
    authoritative.sort();
    if authoritative.len() != 13 {
        bail!(
            "expected 13 authoritative SDL_test headers, found {}",
            authoritative.len()
        );
    }
    for header in authoritative {
        ensure_packaged_path(dev_files, &format!("usr/include/SDL2/{header}"))?;
    }
    Ok(())
}

fn verify_installed_sdl_config(original_dir: &Path) -> Result<()> {
    let installed = fs::read("/usr/include/SDL2/SDL_config.h")
        .context("read installed /usr/include/SDL2/SDL_config.h")?;
    let expected = fs::read(original_dir.join("debian/SDL_config.h"))
        .context("read original Debian SDL_config.h wrapper")?;
    if installed != expected {
        bail!("/usr/include/SDL2/SDL_config.h no longer matches Debian wrapper semantics");
    }
    ensure_path_exists(format!(
        "usr/include/{UBUNTU_MULTIARCH}/SDL2/_real_SDL_config.h"
    ))?;
    Ok(())
}

fn verify_rooted_installed_sdl_config(original_dir: &Path, root: &Path) -> Result<()> {
    let installed = fs::read(root.join("usr/include/SDL2/SDL_config.h")).with_context(|| {
        format!(
            "read {}",
            root.join("usr/include/SDL2/SDL_config.h").display()
        )
    })?;
    let expected = fs::read(original_dir.join("debian/SDL_config.h"))
        .context("read original Debian SDL_config.h wrapper")?;
    if installed != expected {
        bail!("usr/include/SDL2/SDL_config.h no longer matches Debian wrapper semantics");
    }
    ensure_rooted_path_exists(
        root,
        format!("usr/include/{UBUNTU_MULTIARCH}/SDL2/_real_SDL_config.h"),
    )?;
    Ok(())
}

fn verify_debian_multiarch_symlinks(original_dir: &Path) -> Result<()> {
    let rules = fs::read_to_string(original_dir.join("debian/rules"))
        .context("read original Debian rules for include symlink layout")?;
    let link_re = Regex::new(
        r#"ln -s (?P<target>\.\./\.\./SDL2/[A-Za-z0-9_\.]+) debian/tmp/usr/include/\$\(DEB_HOST_MULTIARCH\)/SDL2/"#,
    )?;
    let mut expected = BTreeMap::new();
    for captures in link_re.captures_iter(&rules) {
        let target = captures["target"].to_string();
        let name = Path::new(&target)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("unable to derive basename from {target}"))?;
        expected.insert(name.to_string(), target);
    }
    if expected.len() != 3 {
        bail!(
            "failed to recover Debian multiarch include symlink contract from original/debian/rules"
        );
    }
    for (name, target) in expected {
        let installed = PathBuf::from(format!("/usr/include/{UBUNTU_MULTIARCH}/SDL2/{name}"));
        let metadata = fs::symlink_metadata(&installed)
            .with_context(|| format!("stat {}", installed.display()))?;
        if !metadata.file_type().is_symlink() {
            bail!("{} must remain a symlink", installed.display());
        }
        let actual_target = fs::read_link(&installed)
            .with_context(|| format!("readlink {}", installed.display()))?;
        if actual_target.as_path() != Path::new(&target) {
            bail!(
                "{} target mismatch: expected {}, found {}",
                installed.display(),
                target,
                actual_target.display()
            );
        }
    }
    Ok(())
}

fn verify_rooted_debian_multiarch_symlinks(original_dir: &Path, root: &Path) -> Result<()> {
    let rules = fs::read_to_string(original_dir.join("debian/rules"))
        .context("read original Debian rules for include symlink layout")?;
    let link_re = Regex::new(
        r#"ln -s (?P<target>\.\./\.\./SDL2/[A-Za-z0-9_\.]+) debian/tmp/usr/include/\$\(DEB_HOST_MULTIARCH\)/SDL2/"#,
    )?;
    let mut expected = BTreeMap::new();
    for captures in link_re.captures_iter(&rules) {
        let target = captures["target"].to_string();
        let name = Path::new(&target)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("unable to derive basename from {target}"))?;
        expected.insert(name.to_string(), target);
    }
    if expected.len() != 3 {
        bail!(
            "failed to recover Debian multiarch include symlink contract from original/debian/rules"
        );
    }
    for (name, target) in expected {
        let installed = root.join(format!("usr/include/{UBUNTU_MULTIARCH}/SDL2/{name}"));
        let metadata = fs::symlink_metadata(&installed)
            .with_context(|| format!("stat {}", installed.display()))?;
        if !metadata.file_type().is_symlink() {
            bail!("{} must remain a symlink", installed.display());
        }
        let actual_target = fs::read_link(&installed)
            .with_context(|| format!("readlink {}", installed.display()))?;
        if actual_target.as_path() != Path::new(&target) {
            bail!(
                "{} target mismatch: expected {}, found {}",
                installed.display(),
                target,
                actual_target.display()
            );
        }
    }
    Ok(())
}

fn verify_cmake_surface() -> Result<()> {
    ensure_path_exists(format!(
        "usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/sdlfind.cmake"
    ))?;
    let config = fs::read_to_string(format!(
        "/usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/SDL2Config.cmake"
    ))
    .context("read installed SDL2Config.cmake")?;
    if !config.contains("sdlfind.cmake") {
        bail!("installed SDL2Config.cmake must include sdlfind.cmake");
    }
    Ok(())
}

fn verify_rooted_cmake_surface(root: &Path) -> Result<()> {
    ensure_rooted_path_exists(
        root,
        format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/sdlfind.cmake"),
    )?;
    let config = fs::read_to_string(root.join(format!(
        "usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/SDL2Config.cmake"
    )))
    .with_context(|| {
        format!(
            "read {}",
            root.join(format!(
                "usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/SDL2Config.cmake"
            ))
            .display()
        )
    })?;
    if !config.contains("sdlfind.cmake") {
        bail!("installed SDL2Config.cmake must include sdlfind.cmake");
    }
    Ok(())
}

fn verify_installed_tests_payload(
    tests_files: &BTreeSet<String>,
    install_contract: &crate::contracts::InstallContract,
) -> Result<()> {
    let contract_paths = install_contract
        .tests_package_paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let actual = walk_tree("/usr/libexec/installed-tests/SDL2")?
        .into_iter()
        .chain(walk_tree("/usr/share/installed-tests/SDL2")?)
        .collect::<BTreeSet<_>>();
    let missing = contract_paths
        .difference(&actual)
        .cloned()
        .collect::<Vec<_>>();
    let extra = actual
        .difference(&contract_paths)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !extra.is_empty() {
        bail!(
            "installed-tests payload mismatch\nmissing: {:?}\nextra: {:?}",
            missing,
            extra
        );
    }
    for path in &install_contract.tests_package_paths {
        ensure_packaged_path(tests_files, path)?;
        let full_path = Path::new("/").join(path);
        if path.starts_with("usr/libexec/installed-tests/SDL2/")
            && full_path.is_file()
            && !path.ends_with(".bmp")
            && !path.ends_with(".wav")
            && !path.ends_with(".dat")
            && !path.ends_with(".txt")
        {
            let mode = fs::metadata(&full_path)
                .with_context(|| format!("stat {}", full_path.display()))?
                .permissions()
                .mode();
            if mode & 0o111 == 0 {
                bail!("installed test {} is not executable", full_path.display());
            }
        }
    }
    Ok(())
}

fn verify_rooted_installed_tests_payload(
    root: &Path,
    installed_files: &BTreeSet<String>,
    install_contract: &crate::contracts::InstallContract,
) -> Result<()> {
    let contract_paths = install_contract
        .tests_package_paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let actual = walk_tree_relative(&root.join("usr/libexec/installed-tests/SDL2"), root)?
        .into_iter()
        .chain(walk_tree_relative(
            &root.join("usr/share/installed-tests/SDL2"),
            root,
        )?)
        .collect::<BTreeSet<_>>();
    let missing = contract_paths
        .difference(&actual)
        .cloned()
        .collect::<Vec<_>>();
    let extra = actual
        .difference(&contract_paths)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !extra.is_empty() {
        bail!(
            "installed-tests payload mismatch\nmissing: {:?}\nextra: {:?}",
            missing,
            extra
        );
    }
    for path in &install_contract.tests_package_paths {
        ensure_rooted_packaged_path(root, installed_files, path)?;
        let full_path = root.join(path);
        if path.starts_with("usr/libexec/installed-tests/SDL2/")
            && full_path.is_file()
            && !path.ends_with(".bmp")
            && !path.ends_with(".wav")
            && !path.ends_with(".dat")
            && !path.ends_with(".txt")
        {
            let mode = fs::metadata(&full_path)
                .with_context(|| format!("stat {}", full_path.display()))?
                .permissions()
                .mode();
            if mode & 0o111 == 0 {
                bail!("installed test {} is not executable", full_path.display());
            }
        }
    }
    Ok(())
}

fn verify_static_link_surface(repo_root: &Path) -> Result<()> {
    ensure_path_exists(format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2.a"))?;
    ensure_path_exists(format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2main.a"))?;
    ensure_path_exists(format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2_test.a"))?;
    let mut cmd = Command::new("sdl2-config");
    cmd.current_dir(repo_root).arg("--static-libs");
    let output = output_command(cmd, "probe sdl2-config --static-libs")?;
    let expected_static = format!("/usr/lib/{UBUNTU_MULTIARCH}/libSDL2.a");
    if !output.contains(&expected_static) {
        bail!(
            "sdl2-config --static-libs did not resolve {}: {}",
            expected_static,
            output.trim()
        );
    }
    if output.contains(&repo_root.display().to_string()) {
        bail!("sdl2-config --static-libs leaked a build-tree path");
    }
    Ok(())
}

fn verify_rooted_static_link_surface(root: &Path) -> Result<()> {
    ensure_rooted_path_exists(root, format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2.a"))?;
    ensure_rooted_path_exists(root, format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2main.a"))?;
    ensure_rooted_path_exists(root, format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2_test.a"))?;
    Ok(())
}

fn verify_installed_abi(repo_root: &Path, original_dir: &Path) -> Result<()> {
    let installed_library = PathBuf::from(format!("/usr/lib/{UBUNTU_MULTIARCH}/{SDL_SONAME}"));
    abi_check(AbiCheckArgs {
        repo_root,
        symbols_manifest_path: &original_dir.join("debian/libsdl2-2.0-0.symbols"),
        dynapi_manifest_path: &original_dir.join("src/dynapi/SDL_dynapi_procs.h"),
        exports_source_path: &repo_root.join("safe/src/exports/generated_linux_stubs.rs"),
        dynapi_source_path: &repo_root.join("safe/src/dynapi/generated.rs"),
        library: Some(&installed_library),
        require_soname: Some(SDL_SONAME),
        exports_contract_path: Some(&original_dir.join("src/dynapi/SDL2.exports")),
    })
}

fn run_packaged_autopkgtests(repo_root: &Path) -> Result<()> {
    let safe_root = repo_root.join("safe");
    for script in ["build", "cmake", "deprecated-use", "installed-tests"] {
        let temp = tempdir().with_context(|| format!("create tempdir for autopkgtest {script}"))?;
        let mut cmd = Command::new(safe_root.join(format!("debian/tests/{script}")));
        cmd.current_dir(&safe_root)
            .env_remove(real_runtime_env_key())
            .env("AUTOPKGTEST_TMP", temp.path())
            .env("HOME", temp.path());
        run_command(cmd, &format!("run safe/debian/tests/{script}"))?;
    }
    Ok(())
}

fn run_original_consumer_script(repo_root: &Path, original_dir: &Path, script: &str) -> Result<()> {
    let original_root = absolutize(repo_root, original_dir);
    let mut cmd = Command::new("sh");
    cmd.current_dir(&original_root)
        .arg(original_root.join(format!("debian/tests/{script}")))
        .env_remove(real_runtime_env_key());
    run_command(cmd, &format!("run original/debian/tests/{script}"))
}

fn run_safe_consumer_script(repo_root: &Path, script: &str) -> Result<()> {
    let safe_root = repo_root.join("safe");
    let mut cmd = Command::new("sh");
    cmd.current_dir(&safe_root)
        .arg(safe_root.join(format!("debian/tests/{script}")))
        .env_remove(real_runtime_env_key());
    run_command(cmd, &format!("run safe/debian/tests/{script}"))
}

fn run_test_original_matrix(
    repo_root: &Path,
    results_path: &Path,
    artifact_dir: &Path,
) -> Result<()> {
    let mut cmd = Command::new(repo_root.join("test-original.sh"));
    cmd.current_dir(repo_root)
        .env_remove(real_runtime_env_key())
        .arg("--json-out")
        .arg(absolutize(repo_root, results_path))
        .arg("--artifact-dir")
        .arg(absolutize(repo_root, artifact_dir));
    run_command(cmd, "run dependent Docker matrix")
}

fn run_security_regressions(repo_root: &Path) -> Result<()> {
    let tests_dir = repo_root.join("safe/tests");
    let mut tests = fs::read_dir(&tests_dir)
        .with_context(|| format!("read {}", tests_dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("rs")).then_some(path)
        })
        .filter_map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .filter(|stem| stem.starts_with("security_"))
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    tests.sort();
    if tests.is_empty() {
        bail!(
            "no security regression tests found under {}",
            tests_dir.display()
        );
    }
    for test in tests {
        run_cargo_test(repo_root, &test)?;
    }
    Ok(())
}

fn verify_original_install_patterns(
    original_dir: &Path,
    relative_path: &str,
    installed_files: &BTreeSet<String>,
    triplet: &str,
) -> Result<()> {
    let contents = fs::read_to_string(original_dir.join(relative_path))
        .with_context(|| format!("read {}", original_dir.join(relative_path).display()))?;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("debian/SDL_config.h ") {
            continue;
        }
        let pattern = trimmed
            .replace("usr/include/*/", &format!("usr/include/{triplet}/"))
            .replace("usr/lib/*/", &format!("usr/lib/{triplet}/"));
        if !install_pattern_matches(installed_files, &pattern) {
            bail!(
                "{} pattern {} did not match installed package contents",
                relative_path,
                trimmed
            );
        }
    }
    Ok(())
}

fn install_pattern_matches(installed_files: &BTreeSet<String>, pattern: &str) -> bool {
    if pattern.contains('*') {
        let regex = Regex::new(&format!(
            "^{}$",
            regex::escape(pattern).replace("\\*", "[^/]*")
        ))
        .expect("valid install pattern regex");
        installed_files.iter().any(|path| regex.is_match(path))
    } else {
        installed_files.contains(pattern)
            || installed_files
                .iter()
                .any(|path| path.starts_with(&(pattern.to_string() + "/")))
    }
}

fn ensure_packaged_path(installed_files: &BTreeSet<String>, relative_path: &str) -> Result<()> {
    if !installed_files.contains(relative_path) {
        bail!("package is missing {}", relative_path);
    }
    ensure_path_exists(relative_path)?;
    Ok(())
}

fn ensure_rooted_packaged_path(
    root: &Path,
    installed_files: &BTreeSet<String>,
    relative_path: &str,
) -> Result<()> {
    if !installed_files.contains(relative_path) {
        bail!("package root is missing {}", relative_path);
    }
    ensure_rooted_path_exists(root, relative_path)?;
    Ok(())
}

fn ensure_path_exists<P: AsRef<Path>>(relative_path: P) -> Result<()> {
    let full_path = Path::new("/").join(relative_path.as_ref());
    if !full_path.exists() {
        bail!("missing installed path {}", full_path.display());
    }
    Ok(())
}

fn ensure_rooted_path_exists<P: AsRef<Path>>(root: &Path, relative_path: P) -> Result<()> {
    let full_path = root.join(relative_path.as_ref());
    if !full_path.exists() {
        bail!("missing installed path {}", full_path.display());
    }
    Ok(())
}

fn walk_tree(root: &str) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    walk_tree_inner(Path::new(root), &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_relevant_rooted_files(
    root: &Path,
    install_contract: &crate::contracts::InstallContract,
    header_inventory: &PublicHeaderInventory,
) -> Result<BTreeSet<String>> {
    let mut scan_roots = BTreeSet::new();
    for path in install_contract
        .runtime_paths
        .iter()
        .chain(&install_contract.dev_paths)
        .chain(&install_contract.cmake_surface)
        .chain(&install_contract.multiarch_include_paths)
        .chain(&install_contract.tests_package_paths)
    {
        if let Some(parent) = Path::new(path).parent() {
            scan_roots.insert(parent.to_path_buf());
        }
    }
    for header in &header_inventory.headers {
        if let Some(parent) = Path::new(&header.install_relpath).parent() {
            scan_roots.insert(parent.to_path_buf());
        }
    }

    let mut installed_files = BTreeSet::new();
    for scan_root in scan_roots {
        let full_root = root.join(&scan_root);
        if !full_root.exists() {
            continue;
        }
        installed_files.extend(walk_tree_relative(&full_root, root)?);
    }
    Ok(installed_files)
}

fn walk_tree_relative(root: &Path, base: &Path) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    walk_tree_relative_inner(root, base, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn walk_tree_inner(root: &Path, paths: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            walk_tree_inner(&path, paths)?;
        } else if file_type.is_file() || file_type.is_symlink() {
            paths.push(strip_leading_slash(&path));
        }
    }
    Ok(())
}

fn walk_tree_relative_inner(root: &Path, base: &Path, paths: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            walk_tree_relative_inner(&path, base, paths)?;
        } else if file_type.is_file() || file_type.is_symlink() {
            paths.push(
                path.strip_prefix(base)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn locate_built_packages(repo_root: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let mut packages = BTreeMap::new();
    for entry in fs::read_dir(repo_root).context("read repo root for built .deb files")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("deb") {
            continue;
        }
        let package_name = read_deb_field(&path, "Package")?;
        if !matches!(
            package_name.as_str(),
            "libsdl2-2.0-0" | "libsdl2-dev" | "libsdl2-tests"
        ) {
            continue;
        }
        match packages.get(&package_name) {
            Some(existing) => {
                let current_mtime = fs::metadata(&path)?.modified()?;
                let existing_mtime = fs::metadata(existing)?.modified()?;
                if current_mtime > existing_mtime {
                    packages.insert(package_name, path);
                }
            }
            None => {
                packages.insert(package_name, path);
            }
        }
    }
    if packages.len() != 3 {
        bail!(
            "expected 3 built SDL2 Debian packages, found {}",
            packages.len()
        );
    }
    Ok(packages)
}

fn dpkg_list(package: &str) -> Result<BTreeSet<String>> {
    let mut cmd = Command::new("dpkg");
    cmd.args(["-L", package]);
    let output = output_command(cmd, &format!("list installed files for {package}"))?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('/'))
        .map(PathBuf::from)
        .map(|path| strip_leading_slash(&path))
        .collect())
}

fn read_deb_field(path: &Path, field: &str) -> Result<String> {
    let mut cmd = Command::new("dpkg-deb");
    cmd.arg("-f").arg(path).arg(field);
    output_command(cmd, &format!("read {field} from {}", path.display()))
        .map(|value| value.trim().to_string())
}

fn installed_package_version(package: &str) -> Result<Option<String>> {
    let mut cmd = Command::new("dpkg-query");
    cmd.args(["-W", "-f=${Status}\t${Version}", package]);
    let output = cmd.output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8(output.stdout)?;
    let mut parts = stdout.trim().split('\t');
    let status = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default().trim();
    if status == "install ok installed" && !version.is_empty() {
        Ok(Some(version.to_string()))
    } else {
        Ok(None)
    }
}

fn ensure_root() -> Result<()> {
    let mut cmd = Command::new("id");
    cmd.arg("-u");
    let uid = output_command(cmd, "read uid")?;
    if uid.trim() != "0" {
        bail!("final-check requires root privileges to install build dependencies and packages");
    }
    Ok(())
}

fn run_cargo_test(repo_root: &Path, target: &str) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(repo_root).args([
        "test",
        "--manifest-path",
        "safe/Cargo.toml",
        "--test",
        target,
    ]);
    apply_repo_rust_toolchain_env(&mut cmd);
    cmd.env_remove(real_runtime_env_key());
    run_command(cmd, &format!("run cargo test {target}"))
}

fn real_runtime_env_key() -> &'static str {
    concat!("SAFE_SDL_REAL_", "SDL_PATH")
}

fn apply_repo_rust_toolchain_env(cmd: &mut Command) {
    let Some(sudo_user) = env::var_os("SUDO_USER") else {
        return;
    };
    let user_home = PathBuf::from(format!("/home/{}", sudo_user.to_string_lossy()));
    let cargo_home = user_home.join(".cargo");
    let rustup_home = user_home.join(".rustup");
    let cargo_bin = cargo_home.join("bin");
    if !cargo_bin.join("cargo").is_file() || !rustup_home.is_dir() {
        return;
    }

    let mut path_entries = vec![cargo_bin];
    if let Some(existing) = env::var_os("PATH") {
        path_entries.extend(env::split_paths(&existing));
    }
    if let Ok(path) = env::join_paths(path_entries) {
        cmd.env("PATH", path);
    }
    cmd.env("CARGO_HOME", cargo_home);
    cmd.env("RUSTUP_HOME", rustup_home);
}

fn command_in<const N: usize>(cwd: &Path, program: &str, args: [&str; N]) -> Command {
    let mut cmd = Command::new(program);
    cmd.current_dir(cwd).args(args);
    cmd
}

fn run_command(mut cmd: Command, description: &str) -> Result<()> {
    let status = cmd.status().with_context(|| description.to_string())?;
    if !status.success() {
        bail!("{description} failed with status {status}");
    }
    Ok(())
}

fn output_command(mut cmd: Command, description: &str) -> Result<String> {
    let output = cmd.output().with_context(|| description.to_string())?;
    if !output.status.success() {
        bail!(
            "{description} failed with status {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout).context("command output was not utf-8")
}

fn parse_unsafe_allowlist(path: &Path) -> Result<Vec<UnsafeAuditRule>> {
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let entry_re =
        Regex::new(r#"^- `(?P<pattern>[^`]+)` \[(?P<category>[a-z]+)\]: (?P<justification>.+)$"#)?;
    let mut rules = Vec::new();
    for line in contents.lines() {
        let Some(captures) = entry_re.captures(line.trim()) else {
            continue;
        };
        let category = captures["category"].to_string();
        if !matches!(
            category.as_str(),
            "ffi" | "os" | "generated" | "performance" | "tests"
        ) {
            bail!("unsupported unsafe allowlist category {category}");
        }
        rules.push(UnsafeAuditRule {
            pattern: captures["pattern"].to_string(),
            category,
            justification: captures["justification"].to_string(),
        });
    }
    if rules.is_empty() {
        bail!(
            "{} does not contain any parseable unsafe allowlist rules",
            path.display()
        );
    }
    Ok(rules)
}

fn collect_unsafe_files(repo_root: &Path) -> Result<Vec<String>> {
    let mut cmd = Command::new("rg");
    cmd.current_dir(repo_root).args([
        "-l",
        r"\bunsafe(\s+extern|\s+fn|\s+impl|\s+trait|\s*\{)",
        "safe/src",
        "safe/tests",
        "safe/sdl2main",
        "safe/xtask/src",
    ]);
    let output = output_command(cmd, "scan safe/ for unsafe usage")?;
    let mut files = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn pattern_matches(pattern: &str, path: &str) -> bool {
    let regex = Regex::new(&format!(
        "^{}$",
        regex::escape(pattern).replace("\\*", "[^/]*")
    ))
    .expect("valid unsafe allowlist regex");
    regex.is_match(path)
}

fn write_json_report<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create report directory {}", parent.display()))?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)
        .with_context(|| format!("write {}", path.display()))
}

fn rel(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn absolutize(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn strip_leading_slash(path: &Path) -> String {
    path.strip_prefix(Path::new("/"))
        .unwrap_or(path)
        .display()
        .to_string()
}
