use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use crate::perf::{
    PerfDefaultPolicy, PerfThresholds, PerfWorkload, PerfWorkloadManifest, PerfWorkloadThreshold,
    PHASE_09_ID,
};

pub const PHASE_ID: &str = "impl_phase_01_contract_bootstrap";
pub const PHASE_02_ID: &str = "impl_phase_02_core_runtime";
pub const PHASE_08_ID: &str = "impl_phase_08_testsupport_and_full_upstream_tests";
pub const UBUNTU_RELEASE: &str = "Ubuntu 24.04";
pub const UBUNTU_MULTIARCH: &str = "x86_64-linux-gnu";
pub const SDL_VERSION: &str = "2.30.0";
pub const SDL_REVISION: &str = "SDL-release-2.30.0-0-g859844eae";
pub const SDL_VENDOR_INFO: &str = "Ubuntu 2.30.0+dfsg-1ubuntu3.1";
pub const SDL_SONAME: &str = "libSDL2-2.0.so.0";
pub const SDL_RUNTIME_REALNAME: &str = "libSDL2-2.0.so.0.0.0";

const COMPATIBILITY_HEADERS: &[&str] = &[
    "SDL_copying.h",
    "SDL_name.h",
    "begin_code.h",
    "close_code.h",
    "SDL_config_android.h",
    "SDL_config_emscripten.h",
    "SDL_config_iphoneos.h",
    "SDL_config_macosx.h",
    "SDL_config_minimal.h",
    "SDL_config_ngage.h",
    "SDL_config_os2.h",
    "SDL_config_pandora.h",
    "SDL_config_windows.h",
    "SDL_config_wingdk.h",
    "SDL_config_winrt.h",
    "SDL_config_xbox.h",
];

const PHASE1_SEMANTIC_HEADERS: &[&str] = &[
    "SDL_config.h",
    "SDL_revision.h",
    "SDL_version.h",
    "SDL_platform.h",
    "SDL_types.h",
    "SDL_stdinc.h",
    "SDL_main.h",
    "SDL_test.h",
    "SDL_test_assert.h",
    "SDL_test_common.h",
    "SDL_test_compare.h",
    "SDL_test_crc32.h",
    "SDL_test_font.h",
    "SDL_test_fuzzer.h",
    "SDL_test_harness.h",
    "SDL_test_images.h",
    "SDL_test_log.h",
    "SDL_test_md5.h",
    "SDL_test_memory.h",
    "SDL_test_random.h",
];

const PHASE2_SEMANTIC_HEADERS: &[&str] = &[
    "SDL_assert.h",
    "SDL_atomic.h",
    "SDL_cpuinfo.h",
    "SDL_error.h",
    "SDL_filesystem.h",
    "SDL_hints.h",
    "SDL_loadso.h",
    "SDL_locale.h",
    "SDL_log.h",
    "SDL_misc.h",
    "SDL_mutex.h",
    "SDL_platform.h",
    "SDL_power.h",
    "SDL_rwops.h",
    "SDL_stdinc.h",
    "SDL_system.h",
    "SDL_thread.h",
    "SDL_timer.h",
    "SDL_version.h",
];

const RESOURCE_FILES: &[&str] = &[
    "original/test/axis.bmp",
    "original/test/button.bmp",
    "original/test/controllermap.bmp",
    "original/test/controllermap_back.bmp",
    "original/test/icon.bmp",
    "original/test/moose.dat",
    "original/test/sample.bmp",
    "original/test/sample.wav",
    "original/test/testgles2_sdf_img_normal.bmp",
    "original/test/testgles2_sdf_img_sdf.bmp",
    "original/test/testyuv.bmp",
    "original/test/utf8.txt",
];

const AUTHORITATIVE_AUTO_RUN_TARGETS: &[&str] = &[
    "testautomation",
    "testatomic",
    "testerror",
    "testevdev",
    "testthread",
    "testlocale",
    "testplatform",
    "testpower",
    "testfilesystem",
    "testtimer",
    "testver",
    "testqsort",
    "testaudioinfo",
    "testsurround",
    "testkeys",
    "testbounds",
    "testdisplayinfo",
];

const PHASE_08_TEST_SUPPORT_SOURCES: &[(&str, &str)] = &[
    (
        "original/src/test/SDL_test_assert.c",
        "safe/src/testsupport/assert.rs",
    ),
    (
        "original/src/test/SDL_test_common.c",
        "safe/src/testsupport/common.rs",
    ),
    (
        "original/src/test/SDL_test_compare.c",
        "safe/src/testsupport/compare.rs",
    ),
    (
        "original/src/test/SDL_test_crc32.c",
        "safe/src/testsupport/crc32.rs",
    ),
    (
        "original/src/test/SDL_test_font.c",
        "safe/src/testsupport/font.rs",
    ),
    (
        "original/src/test/SDL_test_fuzzer.c",
        "safe/src/testsupport/fuzzer.rs",
    ),
    (
        "original/src/test/SDL_test_harness.c",
        "safe/src/testsupport/harness.rs",
    ),
    (
        "original/src/test/SDL_test_imageBlit.c",
        "safe/src/testsupport/images.rs",
    ),
    (
        "original/src/test/SDL_test_imageBlitBlend.c",
        "safe/src/testsupport/images.rs",
    ),
    (
        "original/src/test/SDL_test_imageFace.c",
        "safe/src/testsupport/images.rs",
    ),
    (
        "original/src/test/SDL_test_imagePrimitives.c",
        "safe/src/testsupport/images.rs",
    ),
    (
        "original/src/test/SDL_test_imagePrimitivesBlend.c",
        "safe/src/testsupport/images.rs",
    ),
    (
        "original/src/test/SDL_test_log.c",
        "safe/src/testsupport/log.rs",
    ),
    (
        "original/src/test/SDL_test_md5.c",
        "safe/src/testsupport/md5.rs",
    ),
    (
        "original/src/test/SDL_test_memory.c",
        "safe/src/testsupport/memory.rs",
    ),
    (
        "original/src/test/SDL_test_random.c",
        "safe/src/testsupport/random.rs",
    ),
];

const PHASE_08_AUTOMATION_SOURCES: &[&str] = &[
    "original/test/testautomation.c",
    "original/test/testautomation_sdltest.c",
    "original/test/testautomation_subsystems.c",
];

#[derive(Debug, Clone)]
pub struct ContractArgs {
    pub repo_root: PathBuf,
    pub generated_dir: PathBuf,
    pub original_dir: PathBuf,
    pub dependents_path: PathBuf,
    pub cves_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Inputs {
    pub repo_root: PathBuf,
    pub safe_root: PathBuf,
    pub generated_dir: PathBuf,
    pub original_dir: PathBuf,
    pub dependents_path: PathBuf,
    pub cves_path: PathBuf,
}

pub struct AbiCheckArgs<'a> {
    pub repo_root: &'a Path,
    pub symbols_manifest_path: &'a Path,
    pub dynapi_manifest_path: &'a Path,
    pub exports_source_path: &'a Path,
    pub dynapi_source_path: &'a Path,
    pub library: Option<&'a Path>,
    pub require_soname: Option<&'a str>,
    pub exports_contract_path: Option<&'a Path>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicHeaderInventory {
    pub schema_version: u32,
    pub phase_id: String,
    pub headers: Vec<HeaderInventoryEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeaderInventoryEntry {
    pub header_name: String,
    pub install_relpath: String,
    pub source_path: String,
    pub source_kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeaderPhaseMap {
    pub schema_version: u32,
    pub phase_id: String,
    pub entries: Vec<HeaderPhaseEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeaderPhaseEntry {
    pub header_name: String,
    pub install_relpath: String,
    pub availability_phase: String,
    pub semantic_phase: String,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinuxSymbolManifest {
    pub schema_version: u32,
    pub phase_id: String,
    pub soname: String,
    pub runtime_filename: String,
    pub development_link_name: String,
    pub symbols: Vec<LinuxSymbolEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinuxSymbolEntry {
    pub ordinal: usize,
    pub name: String,
    pub version: String,
    pub architecture_predicate: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DynapiManifest {
    pub schema_version: u32,
    pub phase_id: String,
    pub slots: Vec<DynapiSlotManifest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DynapiSlotManifest {
    pub slot_index: usize,
    pub name: String,
    pub line: usize,
    pub guard_stack: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DriverContract {
    pub schema_version: u32,
    pub phase_id: String,
    pub ubuntu_release: String,
    pub video: DriverFamilyContract,
    pub audio: DriverFamilyContract,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DriverFamilyContract {
    pub registry_order: Vec<DriverEntry>,
    pub no_hint_probe_order: Vec<String>,
    pub single_backend_expectations: Vec<SingleBackendExpectation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DriverEntry {
    pub registry_index: usize,
    pub bootstrap_symbol: String,
    pub driver_name: String,
    pub description: String,
    pub feature_predicates: Vec<String>,
    pub demand_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SingleBackendExpectation {
    pub driver_name: String,
    pub selected_without_hint: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StandaloneTestManifest {
    pub schema_version: u32,
    pub phase_id: String,
    pub authoritative_auto_run_targets: Vec<String>,
    pub targets: Vec<StandaloneTarget>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StandaloneTarget {
    pub target_name: String,
    pub source_paths: Vec<String>,
    pub upstream_build_systems: Vec<String>,
    pub linux_buildable: bool,
    pub linux_runnable: bool,
    pub build_predicates: Vec<String>,
    pub resource_paths: Vec<String>,
    pub pkg_config_modules: Vec<String>,
    pub compile_definitions: Vec<String>,
    pub compile_flags: Vec<String>,
    pub link_options: Vec<String>,
    pub link_libraries: Vec<String>,
    pub ci_validation_mode: String,
    pub ci_runner: String,
    pub timeout_seconds: u32,
    pub automation_reason: String,
    pub checker_runner_contract: CheckerRunnerContract,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoninteractiveTestList {
    pub schema_version: u32,
    pub phase_id: String,
    pub source_manifest: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckerRunnerContract {
    pub runner: String,
    pub working_directory: String,
    pub environment: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestautomationSuiteManifest {
    pub schema_version: u32,
    pub phase_id: String,
    pub target_name: String,
    pub source_paths: Vec<String>,
    pub suite_to_phase_ownership: Vec<SuiteOwnership>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuiteOwnership {
    pub suite_name: String,
    pub source_path: String,
    pub owning_phase: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OriginalTestObjectManifest {
    pub schema_version: u32,
    pub phase_id: String,
    pub toolchain_defaults: ToolchainDefaults,
    pub translation_units: Vec<TranslationUnit>,
    pub targets: Vec<OriginalTestTarget>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolchainDefaults {
    pub ubuntu_release: String,
    pub compiler: String,
    pub archive_tool: String,
    pub linker: String,
    pub include_roots: Vec<String>,
    pub system_include_dirs: Vec<String>,
    pub pkg_config_modules: Vec<String>,
    pub baseline_compiler_flags: Vec<String>,
    pub baseline_linker_flags: Vec<String>,
    pub baseline_link_libraries: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslationUnit {
    pub object_id: String,
    pub source_path: String,
    pub language: String,
    pub target_membership: Vec<String>,
    pub upstream_build_systems: Vec<String>,
    pub ubuntu_24_04_enabled: bool,
    pub build_predicates: Vec<String>,
    pub include_dirs: Vec<String>,
    pub system_include_dirs: Vec<String>,
    pub compile_definitions: Vec<String>,
    pub compile_flags: Vec<String>,
    pub output_object_relpath: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OriginalTestTarget {
    pub target_name: String,
    pub output_name: String,
    pub object_ids: Vec<String>,
    pub standalone_manifest_key: String,
    pub ubuntu_24_04_enabled: bool,
    pub build_predicates: Vec<String>,
    pub resource_paths: Vec<String>,
    pub pkg_config_modules: Vec<String>,
    pub link_search_paths: Vec<String>,
    pub link_libraries: Vec<String>,
    pub link_options: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OriginalTestPortMap {
    pub schema_version: u32,
    pub phase_id: String,
    pub expected_source_file_count: usize,
    pub expected_target_count: usize,
    pub entries: Vec<TestPortEntry>,
    pub target_ownership: Vec<TargetOwnership>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortCompletionState {
    Incomplete,
    Complete,
}

fn default_port_completion_state() -> PortCompletionState {
    PortCompletionState::Incomplete
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPortEntry {
    pub original_path: String,
    pub source_kind: String,
    pub ubuntu_buildable: bool,
    pub ubuntu_runnable: bool,
    pub owning_phase: String,
    #[serde(default = "default_port_completion_state")]
    pub completion_state: PortCompletionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_note: Option<String>,
    pub rust_target_kind: String,
    pub rust_target_path: String,
    pub upstream_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetOwnership {
    pub target_name: String,
    pub linux_buildable: bool,
    pub owning_phase: String,
    #[serde(default = "default_port_completion_state")]
    pub completion_state: PortCompletionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallContract {
    pub schema_version: u32,
    pub phase_id: String,
    pub multiarch_triplet: String,
    pub public_header_payload: Vec<String>,
    pub cmake_surface: Vec<String>,
    pub dev_paths: Vec<String>,
    pub multiarch_include_paths: Vec<String>,
    pub tests_package_paths: Vec<String>,
    pub runtime_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeConsumerContract {
    pub schema_version: u32,
    pub phase_id: String,
    pub multiarch_triplet: String,
    pub required_stage_paths: Vec<String>,
    pub standalone_validation: RuntimeStandaloneValidation,
    pub autopkgtests: Vec<RuntimeAutopkgtest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeStandaloneValidation {
    pub build_manifest: String,
    pub port_map: String,
    pub auto_run_validation_mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeAutopkgtest {
    pub script: String,
    pub required_packages: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CveContract {
    pub schema_version: u32,
    pub phase_id: String,
    pub repo_package_version: String,
    pub dependent_package_count: usize,
    pub dependent_samples: Vec<String>,
    pub tracked_cves: Vec<CveEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CveEntry {
    pub id: String,
    pub summary: String,
    pub included_because: String,
    pub rust_port_focus: Vec<String>,
}

#[derive(Debug, Clone)]
struct TargetPlan {
    name: String,
    sources: Vec<String>,
    needs_resources: bool,
    linux_buildable: bool,
    linux_runnable: bool,
    build_predicates: Vec<String>,
    upstream_build_systems: Vec<String>,
    compile_definitions: Vec<String>,
    compile_flags: Vec<String>,
    pkg_config_modules: Vec<String>,
    link_libraries: Vec<String>,
    link_options: Vec<String>,
    timeout_seconds: u32,
}

pub fn capture_contracts(args: ContractArgs) -> Result<()> {
    let inputs = Inputs::from_args(args)?;
    let outputs = build_outputs(&inputs)?;
    for output in outputs {
        write_if_changed(&output.path, &output.contents)?;
    }
    Ok(())
}

pub fn verify_captured_contracts(args: ContractArgs) -> Result<()> {
    let inputs = Inputs::from_args(args)?;
    let outputs = build_outputs(&inputs)?;
    let mut drift = Vec::new();
    for output in outputs {
        let current =
            fs::read(&output.path).with_context(|| format!("reading {}", output.path.display()))?;
        if current != output.contents {
            drift.push(output.path);
        }
    }
    if !drift.is_empty() {
        let details = drift
            .into_iter()
            .map(|path| format!("captured contract drift: {}", rel(&inputs.repo_root, &path)))
            .collect::<Vec<_>>()
            .join("\n");
        bail!("{details}");
    }
    Ok(())
}

pub fn verify_test_port_map(
    repo_root: &Path,
    map_path: &Path,
    original_dir: &Path,
    expect_source_files: Option<usize>,
    expect_executable_targets: Option<usize>,
) -> Result<()> {
    let port_map: OriginalTestPortMap = read_json(map_path)?;
    let expected_sources = upstream_test_source_files(original_dir)?;
    let expected_source_files = expect_source_files.unwrap_or(116);
    if expected_sources.len() != expected_source_files {
        bail!(
            "authoritative upstream source-file count mismatch: expected {}, found {}",
            expected_source_files,
            expected_sources.len()
        );
    }
    if port_map.entries.len() != expected_source_files {
        bail!(
            "expected {} upstream test/support source files, found {}",
            expected_source_files,
            port_map.entries.len()
        );
    }
    let actual_sources: BTreeSet<_> = port_map
        .entries
        .iter()
        .map(|entry| entry.original_path.clone())
        .collect();
    if actual_sources != expected_sources {
        let missing = expected_sources
            .difference(&actual_sources)
            .cloned()
            .collect::<Vec<_>>();
        let extra = actual_sources
            .difference(&expected_sources)
            .cloned()
            .collect::<Vec<_>>();
        bail!(
            "test port map mismatch\nmissing: {:?}\nextra: {:?}",
            missing,
            extra
        );
    }
    let expected_targets = all_test_targets(original_dir)?;
    let expected_executable_targets = expect_executable_targets.unwrap_or(71);
    if expected_targets.len() != expected_executable_targets {
        bail!(
            "authoritative upstream executable-target count mismatch: expected {}, found {}",
            expected_executable_targets,
            expected_targets.len()
        );
    }
    if port_map.target_ownership.len() != expected_executable_targets {
        bail!(
            "expected {} executable target ownership entries, found {}",
            expected_executable_targets,
            port_map.target_ownership.len()
        );
    }
    let owned_targets: BTreeSet<_> = port_map
        .target_ownership
        .iter()
        .map(|entry| entry.target_name.clone())
        .collect();
    if owned_targets != expected_targets {
        let missing = expected_targets
            .difference(&owned_targets)
            .cloned()
            .collect::<Vec<_>>();
        bail!("target ownership missing entries for {:?}", missing);
    }
    let _ = repo_root;
    Ok(())
}

pub fn abi_check(args: AbiCheckArgs<'_>) -> Result<()> {
    let AbiCheckArgs {
        repo_root,
        symbols_manifest_path,
        dynapi_manifest_path,
        exports_source_path,
        dynapi_source_path,
        library,
        require_soname,
        exports_contract_path,
    } = args;
    let symbol_manifest = load_linux_symbol_manifest_input(symbols_manifest_path)?;
    let dynapi_manifest = load_dynapi_manifest_input(dynapi_manifest_path)?;

    let resolved_exports_source = resolve_exports_source_path(repo_root, exports_source_path);
    let stubs_source =
        fs::read_to_string(&resolved_exports_source).context("read generated_linux_stubs.rs")?;
    let stub_symbols = extract_stub_symbol_names(&stubs_source)?;
    let implemented_symbols = implemented_export_symbols(&repo_root.join("safe"))?;
    let exported_surface = stub_symbols
        .union(&implemented_symbols)
        .cloned()
        .collect::<BTreeSet<_>>();
    for symbol in &symbol_manifest.symbols {
        if !exported_surface.contains(&symbol.name) {
            bail!(
                "missing exported symbol implementation or stub for {}",
                symbol.name
            );
        }
    }
    if let Some(exports_contract_path) = exports_contract_path {
        let contract_exports = parse_dynapi_exports(exports_contract_path)?;
        let missing = contract_exports
            .into_iter()
            .filter(|name| {
                symbol_manifest
                    .symbols
                    .iter()
                    .any(|entry| entry.name == *name)
            })
            .filter(|name| !exported_surface.contains(name))
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            bail!(
                "generated export surface missing SDL2.exports entrypoints: {:?}",
                missing
            );
        }
    }

    let dynapi_source =
        fs::read_to_string(dynapi_source_path).context("read dynapi/generated.rs")?;
    let dynapi_slots = extract_dynapi_source_slots(&dynapi_source)?;
    if dynapi_slots.len() != dynapi_manifest.slots.len() {
        bail!(
            "dynapi slot count mismatch: source has {}, manifest has {}",
            dynapi_slots.len(),
            dynapi_manifest.slots.len()
        );
    }
    for slot in &dynapi_manifest.slots {
        let needle = (slot.slot_index, slot.name.clone(), slot.line);
        if !dynapi_slots.contains(&needle) {
            bail!(
                "missing dynapi slot {}:{} at line {}",
                slot.slot_index,
                slot.name,
                slot.line
            );
        }
    }

    if let Some(library_path) = library {
        let exported = read_dynamic_symbol_table(library_path)?;
        let expected: BTreeSet<_> = symbol_manifest
            .symbols
            .iter()
            .map(|entry| entry.name.clone())
            .collect();
        if exported != expected {
            let missing = expected.difference(&exported).cloned().collect::<Vec<_>>();
            let extra = exported.difference(&expected).cloned().collect::<Vec<_>>();
            bail!(
                "ELF export table mismatch for {}\nmissing: {:?}\nextra: {:?}",
                rel(repo_root, library_path),
                missing,
                extra
            );
        }
        if let Some(required_soname) = require_soname {
            let actual_soname = read_soname(library_path)?;
            if actual_soname != required_soname {
                bail!(
                    "SONAME mismatch for {}: expected {}, found {}",
                    rel(repo_root, library_path),
                    required_soname,
                    actual_soname
                );
            }
        }
    }

    Ok(())
}

fn load_linux_symbol_manifest_input(path: &Path) -> Result<LinuxSymbolManifest> {
    if path.extension() == Some(OsStr::new("json")) {
        read_json(path)
    } else {
        parse_linux_symbol_manifest(path)
    }
}

fn load_dynapi_manifest_input(path: &Path) -> Result<DynapiManifest> {
    if path.extension() == Some(OsStr::new("json")) {
        read_json(path)
    } else {
        parse_dynapi_manifest(path)
    }
}

fn resolve_exports_source_path(repo_root: &Path, exports_input_path: &Path) -> PathBuf {
    if exports_input_path.extension() == Some(OsStr::new("rs")) {
        exports_input_path.to_path_buf()
    } else {
        repo_root.join("safe/src/exports/generated_linux_stubs.rs")
    }
}

fn parse_dynapi_exports(path: &Path) -> Result<BTreeSet<String>> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("read SDL dynapi exports {}", path.display()))?;
    let export_re = Regex::new(r#"'(SDL_[A-Za-z0-9_]+)'\s*$"#)?;
    let mut exports = BTreeSet::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("++") {
            continue;
        }
        if let Some(captures) = export_re.captures(trimmed) {
            exports.insert(captures[1].to_string());
        }
    }
    Ok(exports)
}

pub fn generate_real_sdl_config() -> String {
    let body = [
        "/* Generated phase-1 Linux SDL_config.h */",
        "#ifndef SDL_config_h_",
        "#define SDL_config_h_",
        "#include \"SDL_platform.h\"",
        "#define SIZEOF_VOIDP 8",
        "#define HAVE_GCC_ATOMICS 1",
        "#define HAVE_GCC_SYNC_LOCK_TEST_AND_SET 1",
        "#define HAVE_LIBC 1",
        "#define STDC_HEADERS 1",
        "#define HAVE_ALLOCA_H 1",
        "#define HAVE_CTYPE_H 1",
        "#define HAVE_FLOAT_H 1",
        "#define HAVE_ICONV_H 1",
        "#define HAVE_INTTYPES_H 1",
        "#define HAVE_LIMITS_H 1",
        "#define HAVE_MALLOC_H 1",
        "#define HAVE_MATH_H 1",
        "#define HAVE_MEMORY_H 1",
        "#define HAVE_SIGNAL_H 1",
        "#define HAVE_STDARG_H 1",
        "#define HAVE_STDDEF_H 1",
        "#define HAVE_STDINT_H 1",
        "#define HAVE_STDIO_H 1",
        "#define HAVE_STDLIB_H 1",
        "#define HAVE_STRINGS_H 1",
        "#define HAVE_STRING_H 1",
        "#define HAVE_SYS_TYPES_H 1",
        "#define HAVE_WCHAR_H 1",
        "#define HAVE_LINUX_INPUT_H 1",
        "#define HAVE_PTHREAD_NP_H 1",
        "#define HAVE_DLOPEN 1",
        "#define HAVE_MALLOC 1",
        "#define HAVE_CALLOC 1",
        "#define HAVE_REALLOC 1",
        "#define HAVE_FREE 1",
        "#define HAVE_ALLOCA 1",
        "#define HAVE_GETENV 1",
        "#define HAVE_SETENV 1",
        "#define HAVE_PUTENV 1",
        "#define HAVE_UNSETENV 1",
        "#define HAVE_QSORT 1",
        "#define HAVE_BSEARCH 1",
        "#define HAVE_ABS 1",
        "#define HAVE_BCOPY 1",
        "#define HAVE_MEMSET 1",
        "#define HAVE_MEMCPY 1",
        "#define HAVE_MEMMOVE 1",
        "#define HAVE_MEMCMP 1",
        "#define HAVE_STRLEN 1",
        "#define HAVE_STRLCAT 1",
        "#define HAVE_STRLCPY 1",
        "#define HAVE_STRCHR 1",
        "#define HAVE_STRRCHR 1",
        "#define HAVE_STRSTR 1",
        "#define HAVE_STRTOK_R 1",
        "#define HAVE_STRTOL 1",
        "#define HAVE_STRTOUL 1",
        "#define HAVE_STRTOLL 1",
        "#define HAVE_STRTOULL 1",
        "#define HAVE_STRTOD 1",
        "#define HAVE_ATOI 1",
        "#define HAVE_ATOF 1",
        "#define HAVE_STRCMP 1",
        "#define HAVE_STRNCMP 1",
        "#define HAVE_STRCASECMP 1",
        "#define HAVE_STRNCASECMP 1",
        "#define HAVE_STRCASESTR 1",
        "#define HAVE_SSCANF 1",
        "#define HAVE_VSSCANF 1",
        "#define HAVE_SNPRINTF 1",
        "#define HAVE_VSNPRINTF 1",
        "#define HAVE_M_PI 1",
        "#define HAVE_ACOS 1",
        "#define HAVE_ACOSF 1",
        "#define HAVE_ASIN 1",
        "#define HAVE_ASINF 1",
        "#define HAVE_ATAN 1",
        "#define HAVE_ATANF 1",
        "#define HAVE_ATAN2 1",
        "#define HAVE_ATAN2F 1",
        "#define HAVE_CEIL 1",
        "#define HAVE_CEILF 1",
        "#define HAVE_COPYSIGN 1",
        "#define HAVE_COPYSIGNF 1",
        "#define HAVE_COS 1",
        "#define HAVE_COSF 1",
        "#define HAVE_EXP 1",
        "#define HAVE_EXPF 1",
        "#define HAVE_FABS 1",
        "#define HAVE_FABSF 1",
        "#define HAVE_FLOOR 1",
        "#define HAVE_FLOORF 1",
        "#define HAVE_FMOD 1",
        "#define HAVE_FMODF 1",
        "#define HAVE_LOG 1",
        "#define HAVE_LOGF 1",
        "#define HAVE_LOG10 1",
        "#define HAVE_LOG10F 1",
        "#define HAVE_LROUND 1",
        "#define HAVE_LROUNDF 1",
        "#define HAVE_POW 1",
        "#define HAVE_POWF 1",
        "#define HAVE_ROUND 1",
        "#define HAVE_ROUNDF 1",
        "#define HAVE_SCALBN 1",
        "#define HAVE_SCALBNF 1",
        "#define HAVE_SIN 1",
        "#define HAVE_SINF 1",
        "#define HAVE_SQRT 1",
        "#define HAVE_SQRTF 1",
        "#define HAVE_TAN 1",
        "#define HAVE_TANF 1",
        "#define HAVE_TRUNC 1",
        "#define HAVE_TRUNCF 1",
        "#define HAVE_FOPEN64 1",
        "#define HAVE_FSEEKO 1",
        "#define HAVE_FSEEKO64 1",
        "#define HAVE_SIGACTION 1",
        "#define HAVE_SA_SIGACTION 1",
        "#define HAVE_SETJMP 1",
        "#define HAVE_NANOSLEEP 1",
        "#define HAVE_SYSCONF 1",
        "#define HAVE_CLOCK_GETTIME 1",
        "#define HAVE_GETPAGESIZE 1",
        "#define HAVE_MPROTECT 1",
        "#define HAVE_ICONV 1",
        "#define HAVE_PTHREAD_SETNAME_NP 1",
        "#define HAVE_SEM_TIMEDWAIT 1",
        "#define HAVE_GETAUXVAL 1",
        "#define HAVE_POLL 1",
        "#define HAVE__EXIT 1",
        "#define HAVE_O_CLOEXEC 1",
        "#define HAVE_DBUS_DBUS_H 1",
        "#define HAVE_FCITX 1",
        "#define HAVE_IBUS_IBUS_H 1",
        "#define HAVE_SYS_INOTIFY_H 1",
        "#define HAVE_INOTIFY_INIT 1",
        "#define HAVE_INOTIFY_INIT1 1",
        "#define HAVE_INOTIFY 1",
        "#define HAVE_IMMINTRIN_H 1",
        "#define HAVE_LIBUDEV_H 1",
        "#define HAVE_LIBSAMPLERATE_H 1",
        "#define HAVE_LIBDECOR_H 1",
        "#define SDL_DEFAULT_ASSERT_LEVEL 2",
        "#define SDL_AUDIO_DRIVER_ALSA 1",
        "#define SDL_AUDIO_DRIVER_PULSEAUDIO 1",
        "#define SDL_AUDIO_DRIVER_SNDIO 1",
        "#define SDL_AUDIO_DRIVER_PIPEWIRE 1",
        "#define SDL_AUDIO_DRIVER_OSS 1",
        "#define SDL_AUDIO_DRIVER_DISK 1",
        "#define SDL_AUDIO_DRIVER_DUMMY 1",
        "#define SDL_JOYSTICK_LINUX 1",
        "#define SDL_HAPTIC_LINUX 1",
        "#define SDL_SENSOR_DUMMY 1",
        "#define SDL_LOADSO_DLOPEN 1",
        "#define SDL_THREAD_PTHREAD 1",
        "#define SDL_TIMER_UNIX 1",
        "#define SDL_FILESYSTEM_UNIX 1",
        "#define SDL_POWER_LINUX 1",
        "#define SDL_VIDEO_DRIVER_X11 1",
        "#define SDL_VIDEO_DRIVER_WAYLAND 1",
        "#define SDL_VIDEO_DRIVER_KMSDRM 1",
        "#define SDL_VIDEO_DRIVER_OFFSCREEN 1",
        "#define SDL_VIDEO_DRIVER_DUMMY 1",
        "#define SDL_INPUT_LINUXEV 1",
        "#define SDL_VIDEO_OPENGL 1",
        "#define SDL_VIDEO_OPENGL_GLX 1",
        "#define SDL_VIDEO_OPENGL_ES 1",
        "#define SDL_VIDEO_OPENGL_ES2 1",
        "#define SDL_VIDEO_OPENGL_EGL 1",
        "#define SDL_VIDEO_VULKAN 1",
        "#define SDL_VIDEO_RENDER_OGL 1",
        "#define SDL_VIDEO_RENDER_OGL_ES2 1",
        "#define SDL_VIDEO_CAPTURE_DUMMY 1",
        "#endif /* SDL_config_h_ */",
    ];
    format!("{}\n", body.join("\n"))
}

pub fn generate_sdl_revision_header() -> String {
    format!(
        "/* Generated phase-1 SDL_revision.h */\n#define SDL_VENDOR_INFO \"{vendor}\"\n#define SDL_REVISION_NUMBER 0\n#define SDL_REVISION \"{revision} ({vendor})\"\n",
        vendor = SDL_VENDOR_INFO,
        revision = SDL_REVISION
    )
}

pub fn load_public_header_inventory(path: &Path) -> Result<PublicHeaderInventory> {
    read_json(path)
}

pub fn load_install_contract(path: &Path) -> Result<InstallContract> {
    read_json(path)
}

pub fn load_driver_contract(path: &Path) -> Result<DriverContract> {
    read_json(path)
}

pub fn load_original_test_object_manifest(path: &Path) -> Result<OriginalTestObjectManifest> {
    read_json(path)
}

pub fn load_original_test_port_map(path: &Path) -> Result<OriginalTestPortMap> {
    read_json(path)
}

pub fn load_standalone_test_manifest(path: &Path) -> Result<StandaloneTestManifest> {
    read_json(path)
}

pub fn verify_test_port_coverage(
    repo_root: &Path,
    map_path: &Path,
    original_dir: &Path,
    phase: &str,
    require_complete: bool,
    expect_source_files: Option<usize>,
    expect_executable_targets: Option<usize>,
) -> Result<()> {
    verify_test_port_map(
        repo_root,
        map_path,
        original_dir,
        expect_source_files,
        expect_executable_targets,
    )?;

    let port_map = load_original_test_port_map(map_path)?;
    let phase_entries = port_map
        .entries
        .iter()
        .filter(|entry| entry.owning_phase == phase)
        .collect::<Vec<_>>();
    let phase_targets = port_map
        .target_ownership
        .iter()
        .filter(|entry| entry.owning_phase == phase)
        .collect::<Vec<_>>();

    if phase_entries.is_empty() && phase_targets.is_empty() {
        bail!("no test-port ownership found for phase {phase}");
    }

    let incomplete_entries = phase_entries
        .iter()
        .filter(|entry| entry.completion_state != PortCompletionState::Complete)
        .map(|entry| match &entry.completion_note {
            Some(note) => format!("{} ({note})", entry.original_path),
            None => entry.original_path.clone(),
        })
        .collect::<Vec<_>>();
    if !incomplete_entries.is_empty() {
        bail!(
            "phase {phase} has incomplete owned test-port entries: {:?}",
            incomplete_entries
        );
    }

    let incomplete_targets = phase_targets
        .iter()
        .filter(|target| target.completion_state != PortCompletionState::Complete)
        .map(|target| match &target.completion_note {
            Some(note) => format!("{} ({note})", target.target_name),
            None => target.target_name.clone(),
        })
        .collect::<Vec<_>>();
    if !incomplete_targets.is_empty() {
        bail!(
            "phase {phase} has incomplete owned standalone targets: {:?}",
            incomplete_targets
        );
    }

    let missing_targets = phase_entries
        .iter()
        .filter_map(|entry| {
            let path = repo_root.join(&entry.rust_target_path);
            (!path.exists()).then(|| entry.rust_target_path.clone())
        })
        .collect::<Vec<_>>();
    if !missing_targets.is_empty() {
        bail!(
            "phase {phase} is missing owned Rust test/support targets: {:?}",
            missing_targets
        );
    }

    let uncovered_targets = phase_targets
        .iter()
        .filter_map(|target| {
            let covered = phase_entries.iter().any(|entry| {
                entry
                    .upstream_targets
                    .iter()
                    .any(|name| name == &target.target_name)
            });
            (!covered).then(|| target.target_name.clone())
        })
        .collect::<Vec<_>>();
    if !uncovered_targets.is_empty() {
        bail!(
            "phase {phase} owns standalone targets without any mapped Rust port entries: {:?}",
            uncovered_targets
        );
    }

    if require_complete {
        let incomplete_entries = port_map
            .entries
            .iter()
            .filter(|entry| entry.completion_state != PortCompletionState::Complete)
            .map(|entry| entry.original_path.clone())
            .collect::<Vec<_>>();
        if !incomplete_entries.is_empty() {
            bail!(
                "test-port map still has incomplete upstream source/support entries: {:?}",
                incomplete_entries
            );
        }
        let incomplete_targets = port_map
            .target_ownership
            .iter()
            .filter(|entry| entry.completion_state != PortCompletionState::Complete)
            .map(|entry| entry.target_name.clone())
            .collect::<Vec<_>>();
        if !incomplete_targets.is_empty() {
            bail!(
                "test-port map still has incomplete standalone target ownership entries: {:?}",
                incomplete_targets
            );
        }
    }

    Ok(())
}

impl Inputs {
    fn from_args(args: ContractArgs) -> Result<Self> {
        let generated_dir = absolutize(&args.repo_root, &args.generated_dir);
        let original_dir = absolutize(&args.repo_root, &args.original_dir);
        let dependents_path = absolutize(&args.repo_root, &args.dependents_path);
        let cves_path = absolutize(&args.repo_root, &args.cves_path);
        let safe_root = generated_dir
            .parent()
            .ok_or_else(|| {
                anyhow!(
                    "generated directory {} has no parent",
                    generated_dir.display()
                )
            })?
            .to_path_buf();
        Ok(Self {
            repo_root: args.repo_root,
            safe_root,
            generated_dir,
            original_dir,
            dependents_path,
            cves_path,
        })
    }
}

fn build_outputs(inputs: &Inputs) -> Result<Vec<GeneratedFile>> {
    let inventory = build_public_header_inventory(inputs)?;
    let header_phase_map = build_header_phase_map(&inventory);
    let linux_symbols = build_linux_symbol_manifest(inputs)?;
    let dynapi_manifest = build_dynapi_manifest(inputs)?;
    let driver_contract = build_driver_contract(inputs)?;
    let target_plans = build_target_plans(inputs)?;
    let standalone_manifest = build_standalone_manifest(&target_plans);
    let noninteractive_test_list = build_noninteractive_test_list(&standalone_manifest)?;
    let testautomation_manifest = build_testautomation_manifest(inputs)?;
    let original_test_object_manifest = build_original_test_object_manifest(&target_plans);
    let port_map = build_port_map(inputs, &target_plans)?;
    let install_contract = build_install_contract(&inventory, &standalone_manifest);
    let runtime_consumer_contract = build_runtime_consumer_contract(&install_contract);
    let cve_contract = build_cve_contract(inputs)?;
    let perf_workloads = build_perf_workload_manifest();
    let perf_thresholds = build_perf_thresholds(&perf_workloads);
    let installed_test_outputs = build_installed_test_outputs(inputs, &noninteractive_test_list)?;

    validate_outputs(
        inputs,
        OutputContracts {
            inventory: &inventory,
            header_phase_map: &header_phase_map,
            linux_symbols: &linux_symbols,
            dynapi_manifest: &dynapi_manifest,
            driver_contract: &driver_contract,
            standalone_manifest: &standalone_manifest,
            original_test_object_manifest: &original_test_object_manifest,
            port_map: &port_map,
        },
    )?;

    let generated_types = generate_bindings(inputs)?;
    let implemented_symbols = implemented_export_symbols(&inputs.safe_root)?;
    let generated_stubs = render_linux_stubs(&linux_symbols, &implemented_symbols);
    let generated_dynapi = render_dynapi_source(&dynapi_manifest);

    let mut outputs = vec![
        json_output(
            inputs.generated_dir.join("public_header_inventory.json"),
            &inventory,
        )?,
        json_output(
            inputs.generated_dir.join("header_phase_map.json"),
            &header_phase_map,
        )?,
        json_output(
            inputs.generated_dir.join("linux_symbol_manifest.json"),
            &linux_symbols,
        )?,
        json_output(
            inputs.generated_dir.join("dynapi_manifest.json"),
            &dynapi_manifest,
        )?,
        json_output(
            inputs.generated_dir.join("driver_contract.json"),
            &driver_contract,
        )?,
        json_output(
            inputs.generated_dir.join("standalone_test_manifest.json"),
            &standalone_manifest,
        )?,
        json_output(
            inputs.generated_dir.join("noninteractive_test_list.json"),
            &noninteractive_test_list,
        )?,
        json_output(
            inputs
                .generated_dir
                .join("testautomation_suite_manifest.json"),
            &testautomation_manifest,
        )?,
        json_output(
            inputs
                .generated_dir
                .join("original_test_object_manifest.json"),
            &original_test_object_manifest,
        )?,
        json_output(
            inputs.generated_dir.join("original_test_port_map.json"),
            &port_map,
        )?,
        json_output(
            inputs.generated_dir.join("install_contract.json"),
            &install_contract,
        )?,
        json_output(
            inputs.generated_dir.join("cve_contract.json"),
            &cve_contract,
        )?,
        json_output(
            inputs.generated_dir.join("runtime_consumer_contract.json"),
            &runtime_consumer_contract,
        )?,
        json_output(
            inputs.generated_dir.join("perf_workload_manifest.json"),
            &perf_workloads,
        )?,
        json_output(
            inputs.generated_dir.join("perf_thresholds.json"),
            &perf_thresholds,
        )?,
        GeneratedFile {
            path: inputs.safe_root.join("src/abi/generated_types.rs"),
            contents: generated_types.into_bytes(),
        },
        GeneratedFile {
            path: inputs
                .safe_root
                .join("src/exports/generated_linux_stubs.rs"),
            contents: generated_stubs.into_bytes(),
        },
        GeneratedFile {
            path: inputs.safe_root.join("src/dynapi/generated.rs"),
            contents: generated_dynapi.into_bytes(),
        },
    ];
    outputs.extend(installed_test_outputs);
    Ok(outputs)
}

fn build_public_header_inventory(inputs: &Inputs) -> Result<PublicHeaderInventory> {
    let include_dir = inputs.original_dir.join("include");
    let mut headers = Vec::new();
    for entry in fs::read_dir(&include_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("h")) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let (source_path, source_kind) = match name.as_str() {
            "SDL_config.h" => (
                rel(
                    &inputs.repo_root,
                    &inputs.original_dir.join("debian/SDL_config.h"),
                ),
                "debian_wrapper".to_string(),
            ),
            "SDL_revision.h" => (
                "generated://SDL_revision.h".to_string(),
                "generated".to_string(),
            ),
            _ => (
                rel(&inputs.repo_root, &path),
                "upstream_public_header".to_string(),
            ),
        };
        headers.push(HeaderInventoryEntry {
            header_name: name.clone(),
            install_relpath: format!("usr/include/SDL2/{name}"),
            source_path,
            source_kind,
        });
    }
    headers.sort_by(|a, b| a.header_name.cmp(&b.header_name));
    Ok(PublicHeaderInventory {
        schema_version: 1,
        phase_id: PHASE_ID.to_string(),
        headers,
    })
}

fn build_header_phase_map(inventory: &PublicHeaderInventory) -> HeaderPhaseMap {
    let compatibility: BTreeSet<_> = COMPATIBILITY_HEADERS.iter().copied().collect();
    let phase1_semantic: BTreeSet<_> = PHASE1_SEMANTIC_HEADERS.iter().copied().collect();
    let phase2_semantic: BTreeSet<_> = PHASE2_SEMANTIC_HEADERS.iter().copied().collect();
    let entries = inventory
        .headers
        .iter()
        .map(|header| {
            let semantic_phase = if phase2_semantic.contains(header.header_name.as_str()) {
                PHASE_02_ID
            } else if compatibility.contains(header.header_name.as_str())
                || phase1_semantic.contains(header.header_name.as_str())
            {
                PHASE_ID
            } else if header.header_name == "SDL_audio.h" {
                "impl_phase_06_audio"
            } else {
                "future_phase_unassigned"
            };
            let rationale = if compatibility.contains(header.header_name.as_str()) {
                "compile-time-only compatibility surface must remain installable during bootstrap"
            } else if phase2_semantic.contains(header.header_name.as_str()) {
                "phase 2 owns the core runtime semantics for initialization, threading, filesystem, hints, logging, timing, and version queries"
            } else if phase1_semantic.contains(header.header_name.as_str()) {
                "phase 1 owns the generated or bootstrap support semantics for this header"
            } else if header.header_name == "SDL_audio.h" {
                "phase 6 owns audio device, stream, conversion, resampling, and hardened WAVE semantics"
            } else {
                "header is installed in phase 1 but behavior is deferred to later implementation phases"
            };
            HeaderPhaseEntry {
                header_name: header.header_name.clone(),
                install_relpath: header.install_relpath.clone(),
                availability_phase: PHASE_ID.to_string(),
                semantic_phase: semantic_phase.to_string(),
                rationale: rationale.to_string(),
            }
        })
        .collect();
    HeaderPhaseMap {
        schema_version: 1,
        phase_id: PHASE_ID.to_string(),
        entries,
    }
}

fn build_linux_symbol_manifest(inputs: &Inputs) -> Result<LinuxSymbolManifest> {
    let symbols_path = inputs.original_dir.join("debian/libsdl2-2.0-0.symbols");
    parse_linux_symbol_manifest(&symbols_path)
}

fn parse_linux_symbol_manifest(symbols_path: &Path) -> Result<LinuxSymbolManifest> {
    let contents = fs::read_to_string(symbols_path)?;
    let mut lines = contents.lines();
    let header = lines.next().ok_or_else(|| anyhow!("empty symbols file"))?;
    let soname = header
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("missing SONAME in symbols file"))?;
    let symbol_re = Regex::new(
        r#"^\s*(?:\((?P<predicate>[^)]+)\))?(?P<name>SDL_[A-Za-z0-9_]+)@Base\s+(?P<version>[0-9.]+)\s*$"#,
    )?;
    let mut symbols = Vec::new();
    for line in lines {
        if let Some(captures) = symbol_re.captures(line) {
            symbols.push(LinuxSymbolEntry {
                ordinal: symbols.len(),
                name: captures["name"].to_string(),
                version: captures["version"].to_string(),
                architecture_predicate: captures.name("predicate").map(|m| m.as_str().to_string()),
            });
        }
    }
    Ok(LinuxSymbolManifest {
        schema_version: 1,
        phase_id: PHASE_ID.to_string(),
        soname: soname.to_string(),
        runtime_filename: SDL_RUNTIME_REALNAME.to_string(),
        development_link_name: "libSDL2.so".to_string(),
        symbols,
    })
}

fn build_dynapi_manifest(inputs: &Inputs) -> Result<DynapiManifest> {
    let path = inputs.original_dir.join("src/dynapi/SDL_dynapi_procs.h");
    parse_dynapi_manifest(&path)
}

fn parse_dynapi_manifest(path: &Path) -> Result<DynapiManifest> {
    let contents = fs::read_to_string(path)?;
    let proc_re = Regex::new(r#"SDL_DYNAPI_PROC\([^,]+,\s*([A-Za-z0-9_]+),"#)?;
    let mut guard_stack: Vec<String> = Vec::new();
    let mut slots = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#ifdef ") {
            guard_stack.push(rest.to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#ifndef ") {
            guard_stack.push(format!("!{rest}"));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#if ") {
            guard_stack.push(rest.to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#elif ") {
            guard_stack.pop();
            guard_stack.push(rest.to_string());
            continue;
        }
        if trimmed == "#else" {
            if let Some(last) = guard_stack.last_mut() {
                *last = format!("!({last})");
            }
            continue;
        }
        if trimmed == "#endif" {
            guard_stack.pop();
            continue;
        }
        if let Some(captures) = proc_re.captures(trimmed) {
            slots.push(DynapiSlotManifest {
                slot_index: slots.len(),
                name: captures[1].to_string(),
                line: index + 1,
                guard_stack: guard_stack.clone(),
            });
        }
    }
    Ok(DynapiManifest {
        schema_version: 1,
        phase_id: PHASE_ID.to_string(),
        slots,
    })
}

fn build_driver_contract(inputs: &Inputs) -> Result<DriverContract> {
    let video = build_video_driver_family(inputs)?;
    let audio = build_audio_driver_family(inputs)?;
    Ok(DriverContract {
        schema_version: 1,
        phase_id: PHASE_ID.to_string(),
        ubuntu_release: UBUNTU_RELEASE.to_string(),
        video,
        audio,
    })
}

fn build_video_driver_family(inputs: &Inputs) -> Result<DriverFamilyContract> {
    let registry = parse_bootstrap_registry(
        &inputs.original_dir.join("src/video/SDL_video.c"),
        "VideoBootStrap *bootstrap[] = {",
        &[
            "SDL_VIDEO_DRIVER_X11",
            "SDL_VIDEO_DRIVER_WAYLAND",
            "SDL_VIDEO_DRIVER_KMSDRM",
            "SDL_VIDEO_DRIVER_OFFSCREEN",
            "SDL_VIDEO_DRIVER_DUMMY",
            "SDL_INPUT_LINUXEV",
        ],
        BootstrapKind::Video,
        &inputs.original_dir,
    )?;
    let registry_order = registry;
    let no_hint_probe_order = registry_order
        .iter()
        .map(|entry| entry.driver_name.clone())
        .collect::<Vec<_>>();
    let single_backend_expectations = registry_order
        .iter()
        .map(|entry| SingleBackendExpectation {
            driver_name: entry.driver_name.clone(),
            selected_without_hint: Some(entry.driver_name.clone()),
            rationale: "video init walks the registry in order until a backend creates a device"
                .to_string(),
        })
        .collect();
    Ok(DriverFamilyContract {
        registry_order,
        no_hint_probe_order,
        single_backend_expectations,
    })
}

fn build_audio_driver_family(inputs: &Inputs) -> Result<DriverFamilyContract> {
    let registry = parse_bootstrap_registry(
        &inputs.original_dir.join("src/audio/SDL_audio.c"),
        "AudioBootStrap *const bootstrap[] = {",
        &[
            "SDL_AUDIO_DRIVER_PULSEAUDIO",
            "SDL_AUDIO_DRIVER_ALSA",
            "SDL_AUDIO_DRIVER_SNDIO",
            "SDL_AUDIO_DRIVER_PIPEWIRE",
            "SDL_AUDIO_DRIVER_OSS",
            "SDL_AUDIO_DRIVER_DISK",
            "SDL_AUDIO_DRIVER_DUMMY",
        ],
        BootstrapKind::Audio,
        &inputs.original_dir,
    )?;
    let no_hint_probe_order = registry
        .iter()
        .filter(|entry| entry.demand_only != Some(true))
        .map(|entry| entry.driver_name.clone())
        .collect::<Vec<_>>();
    let single_backend_expectations = registry
        .iter()
        .map(|entry| SingleBackendExpectation {
            driver_name: entry.driver_name.clone(),
            selected_without_hint: if entry.demand_only == Some(true) {
                None
            } else {
                Some(entry.driver_name.clone())
            },
            rationale: if entry.demand_only == Some(true) {
                "demand_only audio backends are skipped unless explicitly requested".to_string()
            } else {
                "audio init walks the registry in order, skipping demand_only backends".to_string()
            },
        })
        .collect();
    Ok(DriverFamilyContract {
        registry_order: registry,
        no_hint_probe_order,
        single_backend_expectations,
    })
}

fn build_target_plans(inputs: &Inputs) -> Result<Vec<TargetPlan>> {
    let cmake = fs::read_to_string(inputs.original_dir.join("test/CMakeLists.txt"))?;
    let makefile = fs::read_to_string(inputs.original_dir.join("test/Makefile.in"))?;
    let make_targets = parse_makefile_targets(&makefile)?;
    let cmake_targets = parse_cmake_targets(inputs, &cmake)?;

    let mut plans = Vec::new();
    for (name, sources, needs_resources, _noninteractive) in cmake_targets {
        let in_make_rules = make_targets.contains(&name);
        let mut build_predicates = Vec::new();
        let mut linux_buildable = true;
        let mut linux_runnable = true;
        let mut compile_definitions = Vec::new();
        let compile_flags = vec!["-g".to_string(), "-fno-fast-math".to_string()];
        let mut pkg_config_modules = Vec::new();
        let mut link_libraries = Vec::new();
        let mut link_options = Vec::new();
        let mut timeout_seconds = 10;

        match name.as_str() {
            "testfilesystem_pre" => {
                linux_buildable = false;
                linux_runnable = false;
                build_predicates.push("WIN32 && CMAKE_SIZEOF_VOID_P == 4".to_string());
            }
            "testnative" => {
                build_predicates.push("HAVE_X11".to_string());
                link_libraries.push("X11".to_string());
                pkg_config_modules.push("x11".to_string());
            }
            "testevdev" => {
                build_predicates.push("LINUX".to_string());
                build_predicates.push("linux/input.h available".to_string());
                compile_definitions.push("HAVE_LINUX_INPUT_H".to_string());
                link_options.extend(
                    [
                        "-Wl,--wrap=open",
                        "-Wl,--wrap=close",
                        "-Wl,--wrap=read",
                        "-Wl,--wrap=ioctl",
                    ]
                    .into_iter()
                    .map(str::to_string),
                );
            }
            "testvulkan" => {
                build_predicates.push("public vulkan/vulkan.h available".to_string());
                compile_definitions
                    .push(r#"SDL_PUBLIC_VULKAN_HEADER="vulkan/vulkan.h""#.to_string());
            }
            "testgl2" | "testshader" => {
                build_predicates.push("SDL_VIDEO_OPENGL".to_string());
                compile_definitions.push("HAVE_OPENGL".to_string());
                pkg_config_modules.push("gl".to_string());
                link_libraries.push("GL".to_string());
                link_libraries.push("m".to_string());
            }
            "testgles" => {
                build_predicates.push("SDL_VIDEO_OPENGL_ES".to_string());
                compile_definitions.push("HAVE_OPENGLES".to_string());
                pkg_config_modules.push("glesv1_cm".to_string());
                link_libraries.push("GLESv1_CM".to_string());
                link_libraries.push("m".to_string());
            }
            "testgles2" | "testgles2_sdf" => {
                build_predicates.push("SDL_VIDEO_OPENGL_ES2".to_string());
                compile_definitions.push("HAVE_OPENGLES2".to_string());
                if name == "testgles2_sdf" {
                    link_libraries.push("m".to_string());
                }
            }
            "testgesture" | "testspriteminimal" | "teststreaming" | "testrendercopyex" => {
                link_libraries.push("m".to_string());
            }
            "testautomation" => {
                compile_definitions.extend([
                    "HAVE_WFORMAT_OVERFLOW".to_string(),
                    "HAVE_WFORMAT".to_string(),
                    "HAVE_WFORMAT_EXTRA_ARGS".to_string(),
                ]);
                timeout_seconds = 120;
            }
            "testthread" => timeout_seconds = 40,
            "testtimer" => timeout_seconds = 60,
            _ => {}
        }

        if name == "testaudioinfo" || name == "testsurround" {
            build_predicates.push("SDL_DUMMYAUDIO".to_string());
        }
        if name == "testkeys" || name == "testbounds" || name == "testdisplayinfo" {
            build_predicates.push("SDL_DUMMYVIDEO".to_string());
        }
        if build_predicates.is_empty() {
            build_predicates.push("enabled on Ubuntu 24.04".to_string());
        }

        let upstream_build_systems = if in_make_rules {
            vec!["cmake".to_string(), "autotools".to_string()]
        } else {
            vec!["cmake".to_string()]
        };

        plans.push(TargetPlan {
            name,
            sources,
            needs_resources,
            linux_buildable,
            linux_runnable,
            build_predicates,
            upstream_build_systems,
            compile_definitions,
            compile_flags,
            pkg_config_modules,
            link_libraries,
            link_options,
            timeout_seconds,
        });
    }

    plans.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(plans)
}

fn build_standalone_manifest(target_plans: &[TargetPlan]) -> StandaloneTestManifest {
    let expected_auto_run: BTreeSet<_> = AUTHORITATIVE_AUTO_RUN_TARGETS
        .iter()
        .map(|entry| entry.to_string())
        .collect();
    let mut targets = Vec::new();
    for plan in target_plans {
        let resources = if plan.needs_resources {
            RESOURCE_FILES
                .iter()
                .map(|entry| entry.to_string())
                .collect()
        } else {
            Vec::new()
        };
        let (ci_validation_mode, ci_runner, automation_reason) = if plan.linux_buildable {
            if expected_auto_run.contains(&plan.name) {
                (
                    "auto_run".to_string(),
                    "sdl_dummy_audio_video".to_string(),
                    "target is part of the authoritative upstream noninteractive Ubuntu/Linux automation set".to_string(),
                )
            } else {
                (
                    "build_only".to_string(),
                    "build_only".to_string(),
                    "Linux-buildable target is not auto-run upstream and has no deterministic phase-1 fixture contract".to_string(),
                )
            }
        } else {
            (
                "not_buildable".to_string(),
                "unavailable_on_ubuntu_24_04".to_string(),
                "target is not Linux-buildable under the authoritative upstream conditions"
                    .to_string(),
            )
        };
        let timeout_seconds = if ci_validation_mode == "auto_run" {
            plan.timeout_seconds
        } else {
            0
        };
        let mut environment = BTreeMap::new();
        if ci_validation_mode == "auto_run" {
            environment.insert("SDL_AUDIODRIVER".to_string(), "dummy".to_string());
            environment.insert("SDL_VIDEODRIVER".to_string(), "dummy".to_string());
        }
        targets.push(StandaloneTarget {
            target_name: plan.name.clone(),
            source_paths: plan.sources.clone(),
            upstream_build_systems: plan.upstream_build_systems.clone(),
            linux_buildable: plan.linux_buildable,
            linux_runnable: plan.linux_runnable,
            build_predicates: plan.build_predicates.clone(),
            resource_paths: resources,
            pkg_config_modules: plan.pkg_config_modules.clone(),
            compile_definitions: plan.compile_definitions.clone(),
            compile_flags: plan.compile_flags.clone(),
            link_options: plan.link_options.clone(),
            link_libraries: plan.link_libraries.clone(),
            ci_validation_mode,
            ci_runner: ci_runner.clone(),
            timeout_seconds,
            automation_reason,
            checker_runner_contract: CheckerRunnerContract {
                runner: ci_runner,
                working_directory: "$TARGET_BINARY_DIR".to_string(),
                environment,
            },
        });
    }
    StandaloneTestManifest {
        schema_version: 1,
        phase_id: PHASE_ID.to_string(),
        authoritative_auto_run_targets: AUTHORITATIVE_AUTO_RUN_TARGETS
            .iter()
            .map(|entry| entry.to_string())
            .collect(),
        targets,
    }
}

fn build_testautomation_manifest(inputs: &Inputs) -> Result<TestautomationSuiteManifest> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(inputs.original_dir.join("test"))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("testautomation") && name.ends_with(".c") {
            sources.push(rel(&inputs.repo_root, &entry.path()));
        }
    }
    sources.sort();
    let suite_to_phase_ownership = sources
        .iter()
        .map(|source_path| SuiteOwnership {
            suite_name: Path::new(source_path)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .trim_start_matches("testautomation")
                .trim_start_matches('_')
                .to_string()
                .if_empty_then("core"),
            source_path: source_path.clone(),
            owning_phase: PHASE_ID.to_string(),
        })
        .collect();
    Ok(TestautomationSuiteManifest {
        schema_version: 1,
        phase_id: PHASE_ID.to_string(),
        target_name: "testautomation".to_string(),
        source_paths: sources,
        suite_to_phase_ownership,
    })
}

fn build_original_test_object_manifest(target_plans: &[TargetPlan]) -> OriginalTestObjectManifest {
    let mut translation_units = Vec::new();
    let mut targets = Vec::new();
    for plan in target_plans.iter().filter(|plan| plan.linux_buildable) {
        let mut object_ids = Vec::new();
        for source in &plan.sources {
            let source_name = Path::new(source)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .replace('.', "_");
            let object_id = format!("{}__{}", plan.name, source_name);
            object_ids.push(object_id.clone());
            translation_units.push(TranslationUnit {
                object_id: object_id.clone(),
                source_path: source.clone(),
                language: if source.ends_with(".m") {
                    "objective-c"
                } else {
                    "c"
                }
                .to_string(),
                target_membership: vec![plan.name.clone()],
                upstream_build_systems: plan.upstream_build_systems.clone(),
                ubuntu_24_04_enabled: true,
                build_predicates: plan.build_predicates.clone(),
                include_dirs: vec![
                    "$GENERATED_INCLUDE_DIR".to_string(),
                    "original/include".to_string(),
                    "original/test".to_string(),
                ],
                system_include_dirs: vec![
                    "/usr/include".to_string(),
                    format!("/usr/include/{}", UBUNTU_MULTIARCH),
                ],
                compile_definitions: plan.compile_definitions.clone(),
                compile_flags: plan.compile_flags.clone(),
                output_object_relpath: format!("objects/{}/{}.o", plan.name, source_name),
            });
        }
        targets.push(OriginalTestTarget {
            target_name: plan.name.clone(),
            output_name: plan.name.clone(),
            object_ids,
            standalone_manifest_key: plan.name.clone(),
            ubuntu_24_04_enabled: true,
            build_predicates: plan.build_predicates.clone(),
            resource_paths: if plan.needs_resources {
                RESOURCE_FILES
                    .iter()
                    .map(|entry| entry.to_string())
                    .collect()
            } else {
                Vec::new()
            },
            pkg_config_modules: plan.pkg_config_modules.clone(),
            link_search_paths: vec!["$STAGE_LIBDIR".to_string()],
            link_libraries: plan.link_libraries.clone(),
            link_options: plan.link_options.clone(),
        });
    }
    OriginalTestObjectManifest {
        schema_version: 1,
        phase_id: PHASE_ID.to_string(),
        toolchain_defaults: ToolchainDefaults {
            ubuntu_release: UBUNTU_RELEASE.to_string(),
            compiler: "cc".to_string(),
            archive_tool: "ar".to_string(),
            linker: "cc".to_string(),
            include_roots: vec![
                "$GENERATED_INCLUDE_DIR".to_string(),
                "original/include".to_string(),
                "original/test".to_string(),
            ],
            system_include_dirs: vec![
                "/usr/include".to_string(),
                format!("/usr/include/{}", UBUNTU_MULTIARCH),
            ],
            pkg_config_modules: vec!["sdl2".to_string()],
            baseline_compiler_flags: vec!["-g".to_string(), "-fno-fast-math".to_string()],
            baseline_linker_flags: Vec::new(),
            baseline_link_libraries: vec!["SDL2_test".to_string(), "SDL2".to_string()],
        },
        translation_units,
        targets,
    }
}

fn build_port_map(inputs: &Inputs, target_plans: &[TargetPlan]) -> Result<OriginalTestPortMap> {
    let expected_sources = upstream_test_source_files(&inputs.original_dir)?;
    let existing_map =
        load_original_test_port_map(&inputs.generated_dir.join("original_test_port_map.json")).ok();
    let existing_entries = existing_map
        .as_ref()
        .map(|map| {
            map.entries
                .iter()
                .map(|entry| (entry.original_path.clone(), entry.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let existing_targets = existing_map
        .as_ref()
        .map(|map| {
            map.target_ownership
                .iter()
                .map(|target| (target.target_name.clone(), target.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let mut source_to_targets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let plan_map: BTreeMap<_, _> = target_plans
        .iter()
        .map(|plan| (plan.name.clone(), plan.clone()))
        .collect();
    for plan in target_plans {
        for source in &plan.sources {
            source_to_targets
                .entry(source.clone())
                .or_default()
                .push(plan.name.clone());
        }
    }
    let mut entries = Vec::new();
    for source in expected_sources.iter() {
        let targets = source_to_targets.get(source).cloned().unwrap_or_default();
        let filename = Path::new(source)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let (source_kind, ubuntu_buildable, ubuntu_runnable, rust_target_kind, rust_target_path) =
            if source.starts_with("original/src/test/") {
                let rust_target_path =
                    phase8_testsupport_target_path(source).unwrap_or_else(|| {
                        format!(
                            "safe/src/testsupport/{}.rs",
                            Path::new(source)
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .trim_start_matches("SDL_test_")
                        )
                    });
                (
                    "SDL2_test support source".to_string(),
                    true,
                    false,
                    "static_archive_support".to_string(),
                    rust_target_path,
                )
            } else if filename == "testutils.c" || filename == "testyuv_cvt.c" {
                (
                    "helper source".to_string(),
                    true,
                    false,
                    "support_module".to_string(),
                    format!(
                        "safe/tests/support/{}.rs",
                        Path::new(source)
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                    ),
                )
            } else if filename.starts_with("testautomation") {
                let rust_target_path = if phase8_automation_source(source) {
                    "safe/tests/upstream_port_all.rs".to_string()
                } else {
                    format!(
                        "safe/tests/testautomation/{}.rs",
                        Path::new(source)
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                    )
                };
                (
                    "automation suite source".to_string(),
                    true,
                    true,
                    "integration_test".to_string(),
                    rust_target_path,
                )
            } else {
                let primary = targets.first().cloned().unwrap_or_else(|| {
                    Path::new(source)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                });
                let buildable = targets.iter().any(|target| {
                    plan_map
                        .get(target)
                        .map(|plan| plan.linux_buildable)
                        .unwrap_or(false)
                });
                let runnable = targets.iter().any(|target| {
                    plan_map
                        .get(target)
                        .map(|plan| plan.linux_runnable)
                        .unwrap_or(false)
                });
                let rust_target_kind =
                    if buildable && AUTHORITATIVE_AUTO_RUN_TARGETS.contains(&primary.as_str()) {
                        "smoke_test"
                    } else {
                        "cfg_gated_test"
                    };
                (
                    "standalone executable source".to_string(),
                    buildable,
                    runnable,
                    rust_target_kind.to_string(),
                    format!("safe/tests/upstream/{}.rs", primary),
                )
            };
        let mut entry = TestPortEntry {
            original_path: source.clone(),
            source_kind,
            ubuntu_buildable,
            ubuntu_runnable,
            owning_phase: PHASE_ID.to_string(),
            completion_state: PortCompletionState::Incomplete,
            completion_note: None,
            rust_target_kind,
            rust_target_path,
            upstream_targets: targets,
        };
        if let Some(existing) = existing_entries.get(source) {
            entry.owning_phase = existing.owning_phase.clone();
            entry.completion_state = existing.completion_state;
            entry.completion_note = existing.completion_note.clone();
            entry.rust_target_kind = existing.rust_target_kind.clone();
            entry.rust_target_path = existing.rust_target_path.clone();
        }
        apply_phase_08_entry_override(&mut entry);
        entries.push(entry);
    }

    let target_ownership = target_plans
        .iter()
        .map(|plan| {
            let mut target = TargetOwnership {
                target_name: plan.name.clone(),
                linux_buildable: plan.linux_buildable,
                owning_phase: PHASE_ID.to_string(),
                completion_state: PortCompletionState::Incomplete,
                completion_note: None,
            };
            if let Some(existing) = existing_targets.get(&plan.name) {
                target.owning_phase = existing.owning_phase.clone();
                target.completion_state = existing.completion_state;
                target.completion_note = existing.completion_note.clone();
            }
            apply_phase_08_target_override(&mut target);
            target
        })
        .collect();

    Ok(OriginalTestPortMap {
        schema_version: 2,
        phase_id: PHASE_08_ID.to_string(),
        expected_source_file_count: 116,
        expected_target_count: 71,
        entries,
        target_ownership,
    })
}

fn phase8_testsupport_target_path(source: &str) -> Option<String> {
    PHASE_08_TEST_SUPPORT_SOURCES
        .iter()
        .find_map(|(original_path, rust_target_path)| {
            (*original_path == source).then(|| (*rust_target_path).to_string())
        })
}

fn phase8_automation_source(source: &str) -> bool {
    PHASE_08_AUTOMATION_SOURCES.contains(&source)
}

fn apply_phase_08_entry_override(entry: &mut TestPortEntry) {
    if let Some(rust_target_path) = phase8_testsupport_target_path(&entry.original_path) {
        entry.owning_phase = PHASE_08_ID.to_string();
        entry.completion_state = PortCompletionState::Complete;
        entry.completion_note = Some(format!("covered by {rust_target_path}"));
        entry.rust_target_kind = "static_archive_support".to_string();
        entry.rust_target_path = rust_target_path;
        return;
    }

    if phase8_automation_source(&entry.original_path) {
        entry.owning_phase = PHASE_08_ID.to_string();
        entry.completion_state = PortCompletionState::Complete;
        entry.completion_note = Some("covered by safe/tests/upstream_port_all.rs".to_string());
        entry.rust_target_kind = "integration_test".to_string();
        entry.rust_target_path = "safe/tests/upstream_port_all.rs".to_string();
    }
}

fn apply_phase_08_target_override(target: &mut TargetOwnership) {
    match target.target_name.as_str() {
        "testautomation" => {
            target.owning_phase = PHASE_08_ID.to_string();
            target.completion_state = PortCompletionState::Complete;
            target.completion_note = Some(
                "full original upstream testautomation build/run is validated in phase 8"
                    .to_string(),
            );
        }
        "testfilesystem_pre" => {
            target.owning_phase = "impl_phase_02_core_runtime".to_string();
            target.completion_state = PortCompletionState::Complete;
            target.completion_note = Some(
                "Windows-only prerequisite target retained for complete upstream ownership accounting"
                    .to_string(),
            );
        }
        _ => {}
    }
}

fn build_noninteractive_test_list(
    standalone_manifest: &StandaloneTestManifest,
) -> Result<NoninteractiveTestList> {
    let projected = standalone_manifest
        .targets
        .iter()
        .filter(|target| target.ci_validation_mode == "auto_run")
        .map(|target| target.target_name.clone())
        .collect::<Vec<_>>();
    let projected_set = projected.iter().cloned().collect::<BTreeSet<_>>();
    let authoritative = AUTHORITATIVE_AUTO_RUN_TARGETS
        .iter()
        .map(|target| (*target).to_string())
        .collect::<Vec<_>>();
    let authoritative_set = authoritative.iter().cloned().collect::<BTreeSet<_>>();
    if projected_set != authoritative_set {
        bail!(
            "noninteractive target projection drifted from authoritative upstream membership: {:?}",
            projected
        );
    }
    let targets = authoritative;
    Ok(NoninteractiveTestList {
        schema_version: 1,
        phase_id: PHASE_08_ID.to_string(),
        source_manifest: "safe/generated/standalone_test_manifest.json".to_string(),
        targets,
    })
}

fn build_installed_test_outputs(
    inputs: &Inputs,
    noninteractive: &NoninteractiveTestList,
) -> Result<Vec<GeneratedFile>> {
    let template = fs::read_to_string(inputs.original_dir.join("test/template.test.in"))?;
    let installed_tests_dir = "/usr/libexec/installed-tests/SDL2";
    let mut outputs = noninteractive
        .targets
        .iter()
        .map(|target| GeneratedFile {
            path: inputs.safe_root.join(format!(
                "upstream-tests/installed-tests/usr/share/installed-tests/SDL2/{target}.test"
            )),
            contents: template
                .replace("@installedtestsdir@", installed_tests_dir)
                .replace("@exe@", target)
                .into_bytes(),
        })
        .collect::<Vec<_>>();
    outputs.push(GeneratedFile {
        path: inputs
            .safe_root
            .join("upstream-tests/installed-tests/debian/tests/installed-tests"),
        contents: fs::read(inputs.original_dir.join("debian/tests/installed-tests"))?,
    });
    Ok(outputs)
}

fn build_install_contract(
    inventory: &PublicHeaderInventory,
    standalone_manifest: &StandaloneTestManifest,
) -> InstallContract {
    let mut tests_package_paths = standalone_manifest
        .targets
        .iter()
        .filter(|target| target.linux_buildable)
        .map(|target| format!("usr/libexec/installed-tests/SDL2/{}", target.target_name))
        .collect::<Vec<_>>();
    tests_package_paths.extend(RESOURCE_FILES.iter().map(|resource| {
        format!(
            "usr/libexec/installed-tests/SDL2/{}",
            Path::new(resource)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        )
    }));
    tests_package_paths.extend(
        AUTHORITATIVE_AUTO_RUN_TARGETS
            .iter()
            .map(|target| format!("usr/share/installed-tests/SDL2/{target}.test")),
    );
    tests_package_paths.sort();
    tests_package_paths.dedup();

    InstallContract {
        schema_version: 1,
        phase_id: PHASE_ID.to_string(),
        multiarch_triplet: UBUNTU_MULTIARCH.to_string(),
        public_header_payload: inventory
            .headers
            .iter()
            .map(|header| header.install_relpath.clone())
            .collect(),
        cmake_surface: vec![
            format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/sdl2-config.cmake"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/sdl2-config-version.cmake"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/SDL2Config.cmake"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/SDL2ConfigVersion.cmake"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/SDL2Targets.cmake"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/SDL2staticTargets.cmake"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/SDL2mainTargets.cmake"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/SDL2testTargets.cmake"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/cmake/SDL2/sdlfind.cmake"),
        ],
        dev_paths: vec![
            "usr/bin/sdl2-config".to_string(),
            "usr/share/aclocal/sdl2.m4".to_string(),
            format!("usr/lib/{UBUNTU_MULTIARCH}/pkgconfig/sdl2.pc"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2.a"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2_test.a"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2main.a"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2.so"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2-2.0.so"),
        ],
        multiarch_include_paths: vec![
            format!("usr/include/{UBUNTU_MULTIARCH}/SDL2/_real_SDL_config.h"),
            format!("usr/include/{UBUNTU_MULTIARCH}/SDL2/SDL_platform.h"),
            format!("usr/include/{UBUNTU_MULTIARCH}/SDL2/begin_code.h"),
            format!("usr/include/{UBUNTU_MULTIARCH}/SDL2/close_code.h"),
        ],
        tests_package_paths,
        runtime_paths: vec![
            format!("usr/lib/{UBUNTU_MULTIARCH}/{SDL_RUNTIME_REALNAME}"),
            format!("usr/lib/{UBUNTU_MULTIARCH}/{SDL_SONAME}"),
        ],
    }
}

fn build_runtime_consumer_contract(install_contract: &InstallContract) -> RuntimeConsumerContract {
    let triplet = &install_contract.multiarch_triplet;
    let mut required_stage_paths = vec![
        format!("usr/lib/{triplet}/libSDL2.a"),
        format!("usr/lib/{triplet}/libSDL2main.a"),
        format!("usr/lib/{triplet}/libSDL2_test.a"),
    ];
    required_stage_paths.extend(install_contract.cmake_surface.iter().cloned());
    required_stage_paths.extend([
        format!("usr/lib/{triplet}/pkgconfig/sdl2.pc"),
        "usr/bin/sdl2-config".to_string(),
        "usr/share/aclocal/sdl2.m4".to_string(),
    ]);

    RuntimeConsumerContract {
        schema_version: 1,
        phase_id: PHASE_02_ID.to_string(),
        multiarch_triplet: install_contract.multiarch_triplet.clone(),
        required_stage_paths,
        standalone_validation: RuntimeStandaloneValidation {
            build_manifest: "safe/generated/standalone_test_manifest.json".to_string(),
            port_map: "safe/generated/original_test_port_map.json".to_string(),
            auto_run_validation_mode: "auto_run".to_string(),
        },
        autopkgtests: vec![
            RuntimeAutopkgtest {
                script: "original/debian/tests/build".to_string(),
                required_packages: vec![
                    "build-essential".to_string(),
                    "clang".to_string(),
                    "cmake".to_string(),
                    "pkg-config".to_string(),
                ],
            },
            RuntimeAutopkgtest {
                script: "original/debian/tests/cmake".to_string(),
                required_packages: vec![
                    "build-essential".to_string(),
                    "clang".to_string(),
                    "cmake".to_string(),
                    "pkg-config".to_string(),
                ],
            },
        ],
    }
}

fn build_cve_contract(inputs: &Inputs) -> Result<CveContract> {
    #[derive(Deserialize)]
    struct DependentsRoot {
        dependents: Vec<DependentEntry>,
    }
    #[derive(Deserialize)]
    struct DependentEntry {
        name: String,
    }
    #[derive(Deserialize)]
    struct RelevantCves {
        repo_package_version: String,
        relevant_cves: Vec<RelevantCve>,
    }
    #[derive(Deserialize)]
    struct RelevantCve {
        id: String,
        summary: String,
        included_because: String,
        rust_port_focus: Vec<String>,
    }

    let dependents: DependentsRoot = read_json(&inputs.dependents_path)?;
    let cves: RelevantCves = read_json(&inputs.cves_path)?;
    Ok(CveContract {
        schema_version: 1,
        phase_id: PHASE_ID.to_string(),
        repo_package_version: cves.repo_package_version,
        dependent_package_count: dependents.dependents.len(),
        dependent_samples: dependents
            .dependents
            .iter()
            .take(5)
            .map(|entry| entry.name.clone())
            .collect(),
        tracked_cves: cves
            .relevant_cves
            .into_iter()
            .map(|entry| CveEntry {
                id: entry.id,
                summary: entry.summary,
                included_because: entry.included_because,
                rust_port_focus: entry.rust_port_focus,
            })
            .collect(),
    })
}

fn build_perf_workload_manifest() -> PerfWorkloadManifest {
    PerfWorkloadManifest {
        schema_version: 2,
        phase_id: PHASE_09_ID.to_string(),
        workloads: vec![
            PerfWorkload {
                workload_id: "surface_create_fill_convert_blit".to_string(),
                subsystem: "video_surface".to_string(),
                driver_sources: vec![
                    "original/test/testsprite2.c".to_string(),
                    "original/test/testspriteminimal.c".to_string(),
                    "original/test/testyuv.c".to_string(),
                    "original/test/testautomation_surface.c".to_string(),
                ],
                resource_paths: vec![
                    "original/test/sample.bmp".to_string(),
                    "original/test/axis.bmp".to_string(),
                    "original/test/button.bmp".to_string(),
                ],
                warmup_loops: 8,
                timed_loops: 240,
                description: "Exercise repeated surface creation, fill, pixel conversion, and scaled blit paths against the upstream sample bitmaps.".to_string(),
            },
            PerfWorkload {
                workload_id: "renderer_queue_copy_texture_upload".to_string(),
                subsystem: "render".to_string(),
                driver_sources: vec![
                    "original/test/testrendercopyex.c".to_string(),
                    "original/test/testrendertarget.c".to_string(),
                    "original/test/teststreaming.c".to_string(),
                ],
                resource_paths: vec![
                    "original/test/sample.bmp".to_string(),
                    "original/test/testgles2_sdf_img_normal.bmp".to_string(),
                ],
                warmup_loops: 4,
                timed_loops: 120,
                description: "Drive software renderer copy pressure with repeated texture upload/update and many queued render-copy commands.".to_string(),
            },
            PerfWorkload {
                workload_id: "audio_stream_convert_resample_wave".to_string(),
                subsystem: "audio".to_string(),
                driver_sources: vec![
                    "original/test/loopwave.c".to_string(),
                    "original/test/loopwavequeue.c".to_string(),
                    "original/test/testresample.c".to_string(),
                    "original/test/testautomation_audio.c".to_string(),
                ],
                resource_paths: vec!["original/test/sample.wav".to_string()],
                warmup_loops: 2,
                timed_loops: 48,
                description: "Stress SDL_LoadWAV_RW plus SDL_AudioStream format conversion and resampling using the checked-in upstream sample wave.".to_string(),
            },
            PerfWorkload {
                workload_id: "event_queue_throughput".to_string(),
                subsystem: "events".to_string(),
                driver_sources: vec![
                    "original/test/testautomation_events.c".to_string(),
                ],
                resource_paths: vec![],
                warmup_loops: 16,
                timed_loops: 320,
                description: "Measure custom-event enqueue and dequeue throughput with fixed-size batches under the SDL events subsystem.".to_string(),
            },
            PerfWorkload {
                workload_id: "controller_mapping_guid".to_string(),
                subsystem: "input".to_string(),
                driver_sources: vec![
                    "original/test/testgamecontroller.c".to_string(),
                    "original/test/controllermap.c".to_string(),
                ],
                resource_paths: vec![],
                warmup_loops: 4,
                timed_loops: 256,
                description: "Cover controller mapping lookup and GUID string formatting over a pre-seeded Linux mapping set.".to_string(),
            },
        ],
    }
}

fn build_perf_thresholds(manifest: &PerfWorkloadManifest) -> PerfThresholds {
    PerfThresholds {
        schema_version: 2,
        phase_id: PHASE_09_ID.to_string(),
        default_policy: PerfDefaultPolicy {
            samples_per_workload: 5,
            max_median_cpu_regression_ratio: 1.2,
            max_peak_allocation_regression_ratio: 1.25,
        },
        workloads: manifest
            .workloads
            .iter()
            .map(|workload| PerfWorkloadThreshold {
                workload_id: workload.workload_id.clone(),
                max_median_cpu_regression_ratio: if workload.workload_id
                    == "audio_stream_convert_resample_wave"
                {
                    Some(1.9)
                } else {
                    Some(1.2)
                },
                max_peak_allocation_regression_ratio: Some(1.25),
                waiver_id: if workload.workload_id == "audio_stream_convert_resample_wave" {
                    Some("audio_pure_rust_decode_resample".to_string())
                } else {
                    None
                },
                reason: if workload.workload_id == "audio_stream_convert_resample_wave" {
                    Some("The safe build keeps checked Rust implementations for MS ADPCM decode and sample-rate conversion; after buffer reuse and resample-order tuning the remaining CPU gap is accepted to preserve memory safety and deterministic behavior without hand-written unsafe SIMD".to_string())
                } else {
                    None
                },
            })
            .collect(),
    }
}

struct OutputContracts<'a> {
    inventory: &'a PublicHeaderInventory,
    header_phase_map: &'a HeaderPhaseMap,
    linux_symbols: &'a LinuxSymbolManifest,
    dynapi_manifest: &'a DynapiManifest,
    driver_contract: &'a DriverContract,
    standalone_manifest: &'a StandaloneTestManifest,
    original_test_object_manifest: &'a OriginalTestObjectManifest,
    port_map: &'a OriginalTestPortMap,
}

fn validate_outputs(inputs: &Inputs, outputs: OutputContracts<'_>) -> Result<()> {
    let OutputContracts {
        inventory,
        header_phase_map,
        linux_symbols,
        dynapi_manifest,
        driver_contract,
        standalone_manifest,
        original_test_object_manifest,
        port_map,
    } = outputs;
    if inventory.headers.len() != 91 {
        bail!(
            "expected 91 installed public headers, found {}",
            inventory.headers.len()
        );
    }
    let phase_map_lookup: BTreeMap<_, _> = header_phase_map
        .entries
        .iter()
        .map(|entry| (entry.header_name.clone(), entry))
        .collect();
    for header in COMPATIBILITY_HEADERS {
        let entry = phase_map_lookup
            .get(*header)
            .ok_or_else(|| anyhow!("missing header phase map entry for {header}"))?;
        if entry.availability_phase != PHASE_ID || entry.semantic_phase != PHASE_ID {
            bail!("compatibility header {header} must remain fully owned in phase 1");
        }
    }

    if linux_symbols.symbols.is_empty() {
        bail!("linux symbol manifest is empty");
    }
    if dynapi_manifest.slots.is_empty() {
        bail!("dynapi manifest is empty");
    }

    let actual_auto_run: BTreeSet<_> = standalone_manifest
        .targets
        .iter()
        .filter(|target| target.ci_validation_mode == "auto_run")
        .map(|target| target.target_name.clone())
        .collect();
    let expected_auto_run: BTreeSet<_> = AUTHORITATIVE_AUTO_RUN_TARGETS
        .iter()
        .map(|entry| entry.to_string())
        .collect();
    if actual_auto_run != expected_auto_run {
        bail!(
            "authoritative auto-run set mismatch\nexpected: {:?}\nactual: {:?}",
            expected_auto_run,
            actual_auto_run
        );
    }
    for target in standalone_manifest
        .targets
        .iter()
        .filter(|target| target.linux_buildable)
    {
        if target.ci_validation_mode.is_empty()
            || target.ci_runner.is_empty()
            || target.automation_reason.is_empty()
        {
            bail!(
                "Linux-buildable target {} is missing CI metadata",
                target.target_name
            );
        }
        if target.ci_validation_mode == "auto_run"
            && !expected_auto_run.contains(&target.target_name)
        {
            bail!(
                "target {} is marked auto_run outside the authoritative upstream set",
                target.target_name
            );
        }
    }

    if original_test_object_manifest.translation_units.is_empty()
        || original_test_object_manifest.targets.is_empty()
    {
        bail!("original test object manifest must include toolchain defaults, translation units, and targets");
    }
    let target_map: BTreeMap<_, _> = original_test_object_manifest
        .targets
        .iter()
        .map(|target| (target.target_name.as_str(), target))
        .collect();
    let testevdev = target_map
        .get("testevdev")
        .ok_or_else(|| anyhow!("missing testevdev target contract"))?;
    for required in [
        "-Wl,--wrap=open",
        "-Wl,--wrap=close",
        "-Wl,--wrap=read",
        "-Wl,--wrap=ioctl",
    ] {
        if !testevdev.link_options.iter().any(|entry| entry == required) {
            bail!("testevdev contract missing link option {required}");
        }
    }
    let testnative = target_map
        .get("testnative")
        .ok_or_else(|| anyhow!("missing testnative target contract"))?;
    if !testnative.link_libraries.iter().any(|entry| entry == "X11") {
        bail!("Linux testnative contract must preserve X11 linkage");
    }
    let testvulkan_units = original_test_object_manifest
        .translation_units
        .iter()
        .filter(|unit| {
            unit.target_membership
                .iter()
                .any(|target| target == "testvulkan")
        })
        .collect::<Vec<_>>();
    if testvulkan_units.is_empty() {
        bail!("missing testvulkan translation unit contract");
    }
    if !testvulkan_units.iter().all(|unit| {
        unit.compile_definitions
            .iter()
            .any(|entry| entry.starts_with("SDL_PUBLIC_VULKAN_HEADER="))
    }) {
        bail!("testvulkan contract must preserve SDL_PUBLIC_VULKAN_HEADER");
    }

    let video_names = driver_contract
        .video
        .registry_order
        .iter()
        .map(|entry| entry.driver_name.as_str())
        .collect::<Vec<_>>();
    let audio_names = driver_contract
        .audio
        .registry_order
        .iter()
        .map(|entry| entry.driver_name.as_str())
        .collect::<Vec<_>>();
    if video_names != ["x11", "wayland", "KMSDRM", "offscreen", "dummy", "evdev"] {
        bail!("unexpected Ubuntu video driver order: {:?}", video_names);
    }
    if audio_names
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
        bail!("unexpected Ubuntu audio driver order: {:?}", audio_names);
    }
    if !driver_contract.video.registry_order.iter().any(|entry| {
        entry.driver_name == "evdev"
            && entry
                .feature_predicates
                .iter()
                .any(|predicate| predicate.contains("SDL_INPUT_LINUXEV"))
    }) {
        bail!("driver contract must preserve the SDL_INPUT_LINUXEV-gated dummy evdev entry");
    }
    for driver in &driver_contract.audio.registry_order {
        if matches!(driver.driver_name.as_str(), "disk" | "dummy")
            && driver.demand_only != Some(true)
        {
            bail!(
                "audio demand_only annotation missing for {}",
                driver.driver_name
            );
        }
    }

    if port_map.entries.len() != 116 || port_map.target_ownership.len() != 71 {
        bail!("test port map counts are incorrect");
    }
    let expected_targets = all_test_targets(&inputs.original_dir)?;
    let owned_targets: BTreeSet<_> = port_map
        .target_ownership
        .iter()
        .map(|entry| entry.target_name.clone())
        .collect();
    if owned_targets != expected_targets {
        bail!("test port map is missing target ownership");
    }

    Ok(())
}

fn generate_bindings(inputs: &Inputs) -> Result<String> {
    let temp = TempDir::new().context("create temporary bindgen directory")?;
    let include_dir = temp.path().join("include");
    fs::create_dir_all(&include_dir)?;
    fs::write(include_dir.join("SDL_config.h"), generate_real_sdl_config())?;
    fs::write(
        include_dir.join("SDL_revision.h"),
        generate_sdl_revision_header(),
    )?;
    let wrapper_path = temp.path().join("wrapper.h");
    fs::write(&wrapper_path, "#include \"SDL.h\"\n")?;

    let bindings = bindgen::Builder::default()
        .header(wrapper_path.to_string_lossy())
        .clang_arg(format!("-I{}", include_dir.display()))
        .clang_arg(format!(
            "-I{}",
            inputs.original_dir.join("include").display()
        ))
        .allowlist_file(format!(
            "{}/.*",
            regex::escape(&inputs.original_dir.join("include").display().to_string())
        ))
        .layout_tests(false)
        .generate_comments(true)
        .generate()
        .context("generate bindgen ABI types")?;
    let bindings = bindings.to_string();
    Ok(format!(
        "/* Generated by xtask capture-contracts from authoritative public headers. */\n{}\n",
        bindings.trim_end()
    ))
}

fn render_linux_stubs(
    manifest: &LinuxSymbolManifest,
    implemented_symbols: &BTreeSet<String>,
) -> String {
    let mut source = String::from(
        "/* Generated by xtask capture-contracts from the Debian Linux symbol manifest. */\n\n",
    );
    source.push_str("pub const EXPORTED_SYMBOLS: &[&str] = &[\n");
    for symbol in &manifest.symbols {
        source.push_str(&format!("    \"{}\",\n", symbol.name));
    }
    source.push_str("];\n\n");
    for symbol in &manifest.symbols {
        if implemented_symbols.contains(&symbol.name) {
            continue;
        }
        source.push_str("#[no_mangle]\n");
        source.push_str(&format!(
            "pub unsafe extern \"C\" fn {}() -> *mut ::std::ffi::c_void {{\n    crate::exports::abort_unimplemented(\"{}\");\n}}\n\n",
            symbol.name, symbol.name
        ));
    }
    let trimmed = source.trim_end();
    format!("{trimmed}\n")
}

fn implemented_export_symbols(safe_root: &Path) -> Result<BTreeSet<String>> {
    let mut files = Vec::new();
    collect_rust_source_files(&safe_root.join("src"), &mut files)?;
    let export_re = Regex::new(r#"pub\s+unsafe\s+extern\s+"C"\s+fn\s+(SDL_[A-Za-z0-9_]+)\s*\("#)?;
    let macro_export_re =
        Regex::new(r#"fn\s+(SDL_[A-Za-z0-9_]+)\s*\(.*=\s*[A-Za-z0-9_]+(?:\s*=>\s*[^;]+)?;"#)?;
    let macro_arg_export_re = Regex::new(r#"[A-Za-z0-9_]+!\(\s*(SDL_[A-Za-z0-9_]+)\b"#)?;
    let c_export_re = Regex::new(r#"(?m)^[A-Za-z_][A-Za-z0-9_\s\*]*\b(SDL_[A-Za-z0-9_]+)\s*\("#)?;
    let mut symbols = BTreeSet::new();
    for path in files {
        if path.ends_with("src/exports/generated_linux_stubs.rs")
            || path.ends_with("src/abi/generated_types.rs")
        {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("reading Rust source {}", path.display()))?;
        for captures in export_re.captures_iter(&contents) {
            symbols.insert(captures[1].to_string());
        }
        for line in contents.lines() {
            if let Some(captures) = macro_export_re.captures(line) {
                symbols.insert(captures[1].to_string());
            }
            if let Some(captures) = macro_arg_export_re.captures(line) {
                symbols.insert(captures[1].to_string());
            }
        }
    }
    let phase2_variadic_shims = safe_root.join("src/core/phase2_variadic_shims.c");
    let c_source = fs::read_to_string(&phase2_variadic_shims)
        .with_context(|| format!("reading C source {}", phase2_variadic_shims.display()))?;
    for captures in c_export_re.captures_iter(&c_source) {
        symbols.insert(captures[1].to_string());
    }
    Ok(symbols)
}

fn collect_rust_source_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_source_files(&path, out)?;
        } else if path.extension() == Some(OsStr::new("rs")) {
            out.push(path);
        }
    }
    Ok(())
}

fn render_dynapi_source(manifest: &DynapiManifest) -> String {
    let mut source = String::from(
        "/* Generated by xtask capture-contracts from original/src/dynapi/SDL_dynapi_procs.h. */\n\n",
    );
    source.push_str(
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct DynapiSlot {\n    pub slot_index: usize,\n    pub symbol: &'static str,\n    pub line: usize,\n    pub guards: &'static [&'static str],\n}\n\n",
    );
    source.push_str("pub const DYNAPI_SLOTS: &[DynapiSlot] = &[\n");
    for slot in &manifest.slots {
        let guards = slot
            .guard_stack
            .iter()
            .map(|guard| format!("\"{guard}\""))
            .collect::<Vec<_>>()
            .join(", ");
        source.push_str(&format!(
            "    DynapiSlot {{\n        slot_index: {},\n        symbol: \"{}\",\n        line: {},\n        guards: &[{}],\n    }},\n",
            slot.slot_index, slot.name, slot.line, guards
        ));
    }
    source.push_str("];\n");
    source
}

fn parse_bootstrap_registry(
    registry_path: &Path,
    anchor: &str,
    enabled_predicates: &[&str],
    kind: BootstrapKind,
    original_dir: &Path,
) -> Result<Vec<DriverEntry>> {
    let contents = fs::read_to_string(registry_path)?;
    let start = contents
        .find(anchor)
        .ok_or_else(|| anyhow!("missing bootstrap registry anchor {}", anchor))?;
    let after = &contents[start..];
    let end = after
        .find("NULL")
        .ok_or_else(|| anyhow!("missing bootstrap registry terminator"))?;
    let block = &after[..end];
    let mut guard_stack = Vec::new();
    let entry_re = Regex::new(r#"&([A-Za-z0-9_]+),"#)?;
    let mut bootstraps = Vec::new();
    for line in block.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#ifdef ") {
            guard_stack.push(rest.to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#if ") {
            guard_stack.push(rest.to_string());
            continue;
        }
        if trimmed == "#endif" {
            guard_stack.pop();
            continue;
        }
        if let Some(captures) = entry_re.captures(trimmed) {
            let enabled = guard_stack
                .iter()
                .all(|guard| enabled_predicates.contains(&guard.as_str()));
            if !enabled {
                continue;
            }
            let bootstrap_symbol = captures[1].to_string();
            let bootstrap_source =
                find_bootstrap_definition_file(original_dir, &bootstrap_symbol, kind)?;
            let definition = fs::read_to_string(&bootstrap_source)?;
            let entry = parse_driver_entry(
                &definition,
                &bootstrap_symbol,
                bootstraps.len(),
                &guard_stack,
                kind,
            )?;
            bootstraps.push(entry);
        }
    }
    Ok(bootstraps)
}

fn parse_driver_entry(
    definition_contents: &str,
    bootstrap_symbol: &str,
    registry_index: usize,
    guard_stack: &[String],
    kind: BootstrapKind,
) -> Result<DriverEntry> {
    let pattern = match kind {
        BootstrapKind::Video => format!(
            r#"(?s)VideoBootStrap\s+{}\s*=\s*\{{\s*([^,]+),\s*([^,]+),\s*[^,]+,\s*([^}}]+)\}}"#,
            regex::escape(bootstrap_symbol)
        ),
        BootstrapKind::Audio => format!(
            r#"(?s)AudioBootStrap\s+{}\s*=\s*\{{\s*([^,]+),\s*([^,]+),\s*[^,]+,\s*([^}}]+)\}}"#,
            regex::escape(bootstrap_symbol)
        ),
    };
    let re = Regex::new(&pattern)?;
    let captures = re.captures(definition_contents).ok_or_else(|| {
        anyhow!(
            "could not parse bootstrap definition for {}",
            bootstrap_symbol
        )
    })?;
    let driver_name = resolve_bootstrap_token(captures[1].trim(), definition_contents)?;
    let description = resolve_bootstrap_token(captures[2].trim(), definition_contents)?;
    let demand_only = match kind {
        BootstrapKind::Video => None,
        BootstrapKind::Audio => Some(captures[3].contains("SDL_TRUE")),
    };
    Ok(DriverEntry {
        registry_index,
        bootstrap_symbol: bootstrap_symbol.to_string(),
        driver_name,
        description,
        feature_predicates: guard_stack.to_vec(),
        demand_only,
    })
}

fn find_bootstrap_definition_file(
    original_dir: &Path,
    bootstrap_symbol: &str,
    kind: BootstrapKind,
) -> Result<PathBuf> {
    let root = match kind {
        BootstrapKind::Video => original_dir.join("src/video"),
        BootstrapKind::Audio => original_dir.join("src/audio"),
    };
    find_recursive_bootstrap_definition(&root, bootstrap_symbol, kind)
        .ok_or_else(|| anyhow!("unable to locate definition for {}", bootstrap_symbol))
}

fn find_recursive_bootstrap_definition(
    root: &Path,
    bootstrap_symbol: &str,
    kind: BootstrapKind,
) -> Option<PathBuf> {
    let definition_re = Regex::new(&match kind {
        BootstrapKind::Video => format!(
            r#"VideoBootStrap\s+{}\s*="#,
            regex::escape(bootstrap_symbol)
        ),
        BootstrapKind::Audio => format!(
            r#"AudioBootStrap\s+{}\s*="#,
            regex::escape(bootstrap_symbol)
        ),
    })
    .ok()?;
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_recursive_bootstrap_definition(&path, bootstrap_symbol, kind)
            {
                return Some(found);
            }
            continue;
        }
        if !matches!(
            path.extension().and_then(OsStr::to_str),
            Some("c" | "cc" | "cpp" | "m")
        ) {
            continue;
        }
        let contents = fs::read_to_string(&path).ok()?;
        if definition_re.is_match(&contents) {
            return Some(path);
        }
    }
    None
}

fn resolve_bootstrap_token(token: &str, definition_contents: &str) -> Result<String> {
    let token = token.trim().trim_end_matches(',');
    if token.starts_with('"') && token.ends_with('"') {
        return Ok(token.trim_matches('"').to_string());
    }
    let macro_re = Regex::new(&format!(
        r#"#define\s+{}\s+"([^"]+)""#,
        regex::escape(token)
    ))?;
    if let Some(captures) = macro_re.captures(definition_contents) {
        return Ok(captures[1].to_string());
    }
    bail!("could not resolve bootstrap token {}", token)
}

type CmakeTargetDecl = (String, Vec<String>, bool, bool);

fn parse_cmake_targets(inputs: &Inputs, cmake_contents: &str) -> Result<Vec<CmakeTargetDecl>> {
    let decl_re = Regex::new(r#"(?s)add_sdl_test_executable\((.*?)\)"#)?;
    let mut raw: BTreeMap<String, Vec<(Vec<String>, bool, bool)>> = BTreeMap::new();
    let automation_sources = collect_matching_sources(
        &inputs.original_dir.join("test"),
        "testautomation",
        ".c",
        &inputs.repo_root,
    )?;
    for captures in decl_re.captures_iter(cmake_contents) {
        let body = captures[1].replace('\n', " ");
        let mut tokens = body
            .split_whitespace()
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        let target = tokens.remove(0);
        let mut needs_resources = false;
        let mut noninteractive = false;
        let mut sources = Vec::new();
        for token in tokens {
            match token.as_str() {
                "NEEDS_RESOURCES" => needs_resources = true,
                "NONINTERACTIVE" => noninteractive = true,
                "${TESTAUTOMATION_SOURCE_FILES}" => sources.extend(automation_sources.clone()),
                _ if token.ends_with(".c") || token.ends_with(".m") => {
                    sources.push(format!("original/test/{token}"))
                }
                _ => {}
            }
        }
        raw.entry(target)
            .or_default()
            .push((sources, needs_resources, noninteractive));
    }
    let mut selected = Vec::new();
    for (target, variants) in raw {
        let chosen = if target == "testnative" {
            variants
                .iter()
                .find(|(sources, _, _)| {
                    sources
                        .iter()
                        .any(|source| source.ends_with("testnativex11.c"))
                        && !sources
                            .iter()
                            .any(|source| source.ends_with("testnativecocoa.m"))
                })
                .cloned()
                .or_else(|| variants.first().cloned())
                .ok_or_else(|| anyhow!("missing testnative target variant"))?
        } else {
            variants
                .last()
                .cloned()
                .ok_or_else(|| anyhow!("missing target variant for {}", target))?
        };
        selected.push((target, chosen.0, chosen.1, chosen.2));
    }
    Ok(selected)
}

fn parse_makefile_targets(makefile: &str) -> Result<BTreeSet<String>> {
    let re = Regex::new(r#"^([A-Za-z0-9_]+)\$\(EXE\):"#)?;
    Ok(re
        .captures_iter(makefile)
        .map(|captures| captures[1].to_string())
        .collect())
}

fn upstream_test_source_files(original_dir: &Path) -> Result<BTreeSet<String>> {
    let mut paths = BTreeSet::new();
    for entry in fs::read_dir(original_dir.join("test"))? {
        let entry = entry?;
        let path = entry.path();
        let ext = path.extension().and_then(OsStr::to_str);
        if matches!(ext, Some("c" | "m")) {
            paths.insert(format!(
                "original/test/{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
    }
    for entry in fs::read_dir(original_dir.join("src/test"))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension() == Some(OsStr::new("c")) {
            paths.insert(format!(
                "original/src/test/{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
    }
    Ok(paths)
}

fn all_test_targets(original_dir: &Path) -> Result<BTreeSet<String>> {
    let cmake = fs::read_to_string(original_dir.join("test/CMakeLists.txt"))?;
    let decl_re = Regex::new(r#"add_sdl_test_executable\(([A-Za-z0-9_]+)"#)?;
    Ok(decl_re
        .captures_iter(&cmake)
        .map(|captures| captures[1].to_string())
        .collect())
}

fn collect_matching_sources(
    dir: &Path,
    prefix: &str,
    suffix: &str,
    repo_root: &Path,
) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(prefix) && name.ends_with(suffix) {
            result.push(rel(repo_root, &path));
        }
    }
    result.sort();
    Ok(result)
}

fn read_dynamic_symbol_table(library: &Path) -> Result<BTreeSet<String>> {
    let output = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(library)
        .output()
        .with_context(|| format!("running nm on {}", library.display()))?;
    if !output.status.success() {
        bail!("nm failed for {}", library.display());
    }
    let stdout = String::from_utf8(output.stdout).context("nm output was not utf-8")?;
    Ok(stdout
        .lines()
        .filter_map(|line| {
            line.split_whitespace()
                .last()
                .map(|symbol| symbol.split('@').next().unwrap_or(symbol).to_string())
        })
        .collect())
}

fn read_soname(library: &Path) -> Result<String> {
    let output = Command::new("readelf")
        .arg("-d")
        .arg(library)
        .output()
        .with_context(|| format!("running readelf on {}", library.display()))?;
    if !output.status.success() {
        bail!("readelf failed for {}", library.display());
    }
    let stdout = String::from_utf8(output.stdout).context("readelf output was not utf-8")?;
    let soname_re = Regex::new(r#"\(SONAME\).*?\[(.*?)\]"#)?;
    for line in stdout.lines() {
        if let Some(captures) = soname_re.captures(line) {
            return Ok(captures[1].to_string());
        }
    }
    bail!("no SONAME found in {}", library.display())
}

fn extract_stub_symbol_names(source: &str) -> Result<BTreeSet<String>> {
    let re = Regex::new(r#"pub unsafe extern "C" fn ([A-Za-z0-9_]+)\("#)?;
    Ok(re
        .captures_iter(source)
        .map(|captures| captures[1].to_string())
        .collect())
}

fn extract_dynapi_source_slots(source: &str) -> Result<BTreeSet<(usize, String, usize)>> {
    let single_line = Regex::new(
        r#"^\s*DynapiSlot\s*\{\s*slot_index:\s*(\d+),\s*symbol:\s*"([^"]+)",\s*line:\s*(\d+),"#,
    )?;
    let mut slots = BTreeSet::new();
    let mut current: Option<(Option<usize>, Option<String>, Option<usize>)> = None;

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(captures) = single_line.captures(trimmed) {
            slots.insert((
                captures[1].parse()?,
                captures[2].to_string(),
                captures[3].parse()?,
            ));
            continue;
        }

        if trimmed == "DynapiSlot {" {
            current = Some((None, None, None));
            continue;
        }

        let Some((slot_index, symbol, source_line)) = current.as_mut() else {
            continue;
        };

        if let Some(value) = trimmed.strip_prefix("slot_index:") {
            *slot_index = Some(value.trim().trim_end_matches(',').parse()?);
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("symbol:") {
            *symbol = Some(
                value
                    .trim()
                    .trim_end_matches(',')
                    .trim_matches('"')
                    .to_string(),
            );
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("line:") {
            *source_line = Some(value.trim().trim_end_matches(',').parse()?);
            continue;
        }
        if trimmed == "}," {
            let entry = current.take().context("incomplete DynapiSlot entry")?;
            let slot_index = entry.0.context("missing dynapi slot_index")?;
            let symbol = entry.1.context("missing dynapi symbol")?;
            let source_line = entry.2.context("missing dynapi line")?;
            slots.insert((slot_index, symbol, source_line));
        }
    }

    if current.is_some() {
        bail!("unterminated DynapiSlot entry in generated dynapi source");
    }

    Ok(slots)
}

fn json_output<T: Serialize>(path: PathBuf, value: &T) -> Result<GeneratedFile> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(GeneratedFile {
        path,
        contents: bytes,
    })
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    serde_json::from_slice(&fs::read(path).with_context(|| format!("read {}", path.display()))?)
        .with_context(|| format!("parse {}", path.display()))
}

fn write_if_changed(path: &Path, contents: &[u8]) -> Result<()> {
    if fs::read(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents).with_context(|| format!("write {}", path.display()))
}

fn absolutize(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

pub fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn if_empty_then(input: String, fallback: &str) -> String {
    if input.is_empty() {
        fallback.to_string()
    } else {
        input
    }
}

trait StringExt {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringExt for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if_empty_then(self, fallback)
    }
}

#[derive(Debug, Clone, Copy)]
enum BootstrapKind {
    Video,
    Audio,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::extract_dynapi_source_slots;

    #[test]
    fn extracts_multiline_dynapi_source_slots() {
        let source = r#"
pub const DYNAPI_SLOTS: &[DynapiSlot] = &[
    DynapiSlot {
        slot_index: 27,
        symbol: "SDL_Init",
        line: 88,
        guards: &[],
    },
    DynapiSlot {
        slot_index: 28,
        symbol: "SDL_InitSubSystem",
        line: 89,
        guards: &["HAVE_SDL"],
    },
];
"#;

        let slots = extract_dynapi_source_slots(source).expect("parse dynapi source slots");

        assert_eq!(
            slots,
            BTreeSet::from([
                (27, "SDL_Init".to_string(), 88),
                (28, "SDL_InitSubSystem".to_string(), 89),
            ])
        );
    }
}
