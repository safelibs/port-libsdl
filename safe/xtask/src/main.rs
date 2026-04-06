mod contracts;
mod dependents;
mod final_phase;
mod original_tests;
mod perf;
mod stage_install;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use contracts::{
    abi_check, capture_contracts, verify_captured_contracts, verify_test_port_coverage,
    verify_test_port_map, AbiCheckArgs as ContractsAbiCheckArgs, ContractArgs, PHASE_08_ID,
    UBUNTU_MULTIARCH,
};
use dependents::{verify_dependent_regressions, VerifyDependentRegressionsArgs};
use final_phase::{
    final_check, verify_install_contract, verify_unsafe_allowlist, FinalCheckArgs,
    VerifyInstallContractArgs,
};
use original_tests::{
    build_original_autotools_suite, build_original_cmake_suite, build_original_standalone,
    compile_original_test_objects, relink_original_test_objects, run_evdev_fixture_tests,
    run_fixture_backed_original_tests, run_gesture_replay, run_original_autotools_check,
    run_original_ctest, run_original_standalone, run_relinked_original_tests, run_xvfb,
    run_xvfb_window_smoke, BuildOriginalAutotoolsSuiteArgs, BuildOriginalCmakeSuiteArgs,
    BuildOriginalStandaloneArgs, CompileOriginalTestObjectsArgs, RelinkOriginalTestObjectsArgs,
    RunFixtureBackedOriginalTestsArgs, RunOriginalAutotoolsCheckArgs, RunOriginalCtestArgs,
    RunOriginalStandaloneArgs, RunRelinkedOriginalTestsArgs,
};
use perf::{
    build_original_reference, perf_assert, perf_capture, BuildOriginalReferenceArgs,
    PerfAssertArgs, PerfCaptureArgs, DEFAULT_ORIGINAL_BUILD_DIR, DEFAULT_ORIGINAL_PREFIX,
    DEFAULT_PERF_MANIFEST, DEFAULT_PERF_REPORT, DEFAULT_PERF_RUNNER_DIR, DEFAULT_PERF_THRESHOLDS,
    DEFAULT_PERF_WAIVERS, DEFAULT_SAFE_STAGE_ROOT,
};
use stage_install::{
    stage_install, verify_bootstrap_stage, verify_driver_contract, StageInstallArgs,
    StageInstallMode, VerifyBootstrapStageArgs, VerifyDriverContractArgs,
};

fn main() -> Result<()> {
    let repo_root = env::current_dir()?;
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return usage();
    };
    let remaining = args.collect::<Vec<_>>();

    match command.as_str() {
        "capture-contracts" => {
            let parsed = CommonArgs::parse(&remaining)?;
            capture_contracts(parsed.into_contract_args(repo_root))
        }
        "verify-captured-contracts" => {
            let parsed = CommonArgs::parse(&remaining)?;
            verify_captured_contracts(parsed.into_contract_args(repo_root))
        }
        "abi-check" => {
            let parsed = AbiCheckArgs::parse(&remaining)?;
            let symbols_manifest = parsed
                .symbols
                .unwrap_or_else(|| parsed.generated.join("linux_symbol_manifest.json"));
            let dynapi_manifest = parsed
                .dynapi
                .unwrap_or_else(|| parsed.generated.join("dynapi_manifest.json"));
            let exports_source = parsed
                .exports
                .as_ref()
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
                .cloned()
                .unwrap_or_else(|| PathBuf::from("safe/src/exports/generated_linux_stubs.rs"));
            let exports_contract = parsed
                .exports
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) != Some("rs"));
            let dynapi_source = PathBuf::from("safe/src/dynapi/generated.rs");
            abi_check(ContractsAbiCheckArgs {
                repo_root: &repo_root,
                symbols_manifest_path: &symbols_manifest,
                dynapi_manifest_path: &dynapi_manifest,
                exports_source_path: &exports_source,
                dynapi_source_path: &dynapi_source,
                library: parsed.library.as_deref(),
                require_soname: parsed.require_soname.as_deref(),
                exports_contract_path: exports_contract.as_deref(),
            })
        }
        "verify-test-port-map" => {
            let parsed = VerifyTestPortMapArgs::parse(&remaining)?;
            let map_path = parsed
                .map
                .unwrap_or_else(|| parsed.generated.join("original_test_port_map.json"));
            verify_test_port_map(
                &repo_root,
                &map_path,
                &parsed.original,
                parsed.expect_source_files,
                parsed.expect_executable_targets,
            )
        }
        "verify-test-port-coverage" => {
            let parsed = VerifyTestPortCoverageArgs::parse(&remaining)?;
            let map_path = parsed
                .map
                .unwrap_or_else(|| parsed.generated.join("original_test_port_map.json"));
            verify_test_port_coverage(
                &repo_root,
                &map_path,
                &parsed.original,
                &parsed.phase,
                parsed.require_complete,
                parsed.expect_source_files,
                parsed.expect_executable_targets,
            )
        }
        "stage-install" => {
            let parsed = StageInstallCliArgs::parse(&remaining)?;
            stage_install(StageInstallArgs {
                repo_root,
                generated_dir: parsed.generated,
                original_dir: parsed.original,
                stage_root: parsed.root,
                library_path: parsed.library,
                mode: parsed.mode,
            })
        }
        "build-original-cmake-suite" => {
            let parsed = OriginalSuiteCliArgs::parse(&remaining, "build-phase8-upstream-cmake")?;
            build_original_cmake_suite(BuildOriginalCmakeSuiteArgs {
                repo_root,
                original_dir: parsed.original,
                stage_root: parsed.root,
                build_dir: parsed.build_dir,
            })
        }
        "run-original-ctest" => {
            let parsed = OriginalCtestCliArgs::parse(&remaining, "build-phase8-upstream-cmake")?;
            run_original_ctest(RunOriginalCtestArgs {
                repo_root,
                build_dir: parsed.build_dir,
                stage_root: parsed.root,
                filter: parsed.filter,
                test_list: parsed.test_list,
            })
        }
        "build-original-autotools-suite" => {
            let parsed =
                OriginalSuiteCliArgs::parse(&remaining, "build-phase8-upstream-autotools")?;
            build_original_autotools_suite(BuildOriginalAutotoolsSuiteArgs {
                repo_root,
                original_dir: parsed.original,
                stage_root: parsed.root,
                build_dir: parsed.build_dir,
            })
        }
        "run-original-autotools-check" => {
            let parsed = RunOriginalAutotoolsCheckCliArgs::parse(
                &remaining,
                "build-phase8-upstream-autotools",
            )?;
            run_original_autotools_check(RunOriginalAutotoolsCheckArgs {
                repo_root,
                stage_root: parsed.root,
                build_dir: parsed.build_dir,
            })
        }
        "build-original-reference" => {
            let parsed = BuildOriginalReferenceCliArgs::parse(&remaining)?;
            build_original_reference(BuildOriginalReferenceArgs {
                repo_root,
                original_dir: parsed.original,
                build_dir: parsed.build_dir,
                prefix_dir: parsed.prefix,
            })
        }
        "perf-capture" => {
            let parsed = PerfCaptureCliArgs::parse(&remaining)?;
            perf_capture(PerfCaptureArgs {
                repo_root,
                generated_dir: parsed.generated,
                original_dir: parsed.original,
                original_prefix_dir: parsed.original_prefix,
                safe_stage_root: parsed.safe_stage,
                runner_dir: parsed.runner_dir,
                workload_manifest: parsed.manifest,
                thresholds_path: parsed.thresholds,
                report_path: parsed.report,
                waivers_path: parsed.waivers,
            })
        }
        "perf-assert" => {
            let parsed = PerfAssertCliArgs::parse(&remaining)?;
            perf_assert(PerfAssertArgs {
                repo_root,
                thresholds_path: parsed.thresholds,
                report_path: parsed.report,
                waivers_path: parsed.waivers,
            })
        }
        "verify-bootstrap-stage" => {
            let parsed = VerifyBootstrapStageCliArgs::parse(&remaining)?;
            verify_bootstrap_stage(VerifyBootstrapStageArgs {
                repo_root,
                generated_dir: parsed.generated,
                stage_root: parsed.root,
            })
        }
        "verify-install-contract" => {
            let parsed = VerifyInstallContractCliArgs::parse(&remaining)?;
            verify_install_contract(VerifyInstallContractArgs {
                repo_root,
                generated_dir: parsed.generated,
                original_dir: parsed.original,
                package_root: parsed.root,
                install_contract_path: parsed.contract,
                public_header_inventory_path: parsed.public_header_inventory,
                mode: parsed.mode,
            })
        }
        "verify-driver-contract" => {
            let parsed = VerifyDriverContractCliArgs::parse(&remaining)?;
            verify_driver_contract(VerifyDriverContractArgs {
                repo_root,
                contract_path: parsed.contract,
                stage_root: parsed.root,
                kind: parsed.kind,
            })
        }
        "compile-original-test-objects" => {
            let parsed = CompileOriginalCliArgs::parse(&remaining)?;
            compile_original_test_objects(CompileOriginalTestObjectsArgs {
                repo_root,
                generated_dir: parsed.generated,
                object_manifest: parsed.object_manifest,
                output_dir: parsed.output_dir,
            })
        }
        "relink-original-test-objects" => {
            let parsed = RelinkOriginalCliArgs::parse(&remaining)?;
            relink_original_test_objects(RelinkOriginalTestObjectsArgs {
                repo_root,
                generated_dir: parsed.generated,
                object_manifest: parsed.object_manifest,
                standalone_manifest: parsed.standalone_manifest,
                objects_dir: parsed.objects_dir,
                output_dir: parsed.output_dir,
                library_path: parsed.library,
            })
        }
        "build-original-standalone" => {
            let parsed = BuildOriginalStandaloneCliArgs::parse(&remaining)?;
            build_original_standalone(BuildOriginalStandaloneArgs {
                repo_root,
                generated_dir: parsed.generated,
                standalone_manifest: parsed.manifest,
                stage_root: parsed.destdir,
                build_dir: parsed.build_dir,
                phase: parsed.phase,
            })
        }
        "run-relinked-original-tests" => {
            let parsed = RunRelinkedCliArgs::parse(&remaining)?;
            run_relinked_original_tests(RunRelinkedOriginalTestsArgs {
                repo_root,
                generated_dir: parsed.generated,
                standalone_manifest: parsed.manifest,
                bin_dir: parsed.bin_dir,
                filter: parsed.target,
                validation_modes: parsed.validation_modes,
                skip_if_empty: parsed.skip_if_empty,
            })
        }
        "run-original-standalone" => {
            let parsed = RunOriginalStandaloneCliArgs::parse(&remaining)?;
            run_original_standalone(RunOriginalStandaloneArgs {
                repo_root,
                generated_dir: parsed.generated,
                standalone_manifest: parsed.manifest,
                build_dir: parsed.build_dir,
                phase: parsed.phase,
                validation_mode: parsed.validation_mode,
                skip_if_empty: parsed.skip_if_empty,
            })
        }
        "run-evdev-fixture-tests" => run_evdev_fixture_tests(repo_root),
        "run-fixture-backed-original-tests" => {
            let parsed = RunFixtureBackedOriginalTestsCliArgs::parse(&remaining)?;
            run_fixture_backed_original_tests(RunFixtureBackedOriginalTestsArgs {
                repo_root,
                generated_dir: parsed.generated,
                standalone_manifest: parsed.manifest,
                build_dir: parsed.build_dir,
                phase: parsed.phase,
                skip_if_empty: parsed.skip_if_empty,
            })
        }
        "run-gesture-replay" => run_gesture_replay(repo_root),
        "run-xvfb" => {
            let parsed = RunXvfbCliArgs::parse(&remaining)?;
            run_xvfb(repo_root, parsed.command)
        }
        "run-xvfb-window-smoke" => run_xvfb_window_smoke(repo_root),
        "final-check" => {
            let parsed = FinalCheckCliArgs::parse(&remaining)?;
            final_check(FinalCheckArgs {
                repo_root,
                generated_dir: parsed.generated,
                original_dir: parsed.original,
                dependents_path: parsed.dependents,
                cves_path: parsed.cves,
                relink_objects_dir: parsed.relink_objects_dir,
                relink_bin_dir: parsed.relink_bin_dir,
                unsafe_allowlist: parsed.unsafe_allowlist,
                phase_report: parsed.phase_report,
                unsafe_report: parsed.unsafe_report,
                dependent_regression_manifest: parsed.dependent_regression_manifest,
                dependent_matrix_results: parsed.dependent_matrix_results,
                dependent_matrix_artifact_dir: parsed.dependent_matrix_artifact_dir,
            })
        }
        "verify-dependent-regressions" => {
            let parsed = VerifyDependentRegressionsCliArgs::parse(&remaining)?;
            verify_dependent_regressions(VerifyDependentRegressionsArgs {
                repo_root,
                dependents_path: parsed.dependents,
                manifest_path: parsed.manifest,
                results_path: parsed.results,
            })
        }
        "security-regressions" => security_regressions(&repo_root),
        "unsafe-audit" | "verify-unsafe-allowlist" => {
            let parsed = VerifyUnsafeAllowlistCliArgs::parse(&remaining)?;
            verify_unsafe_allowlist(&repo_root, &parsed.allowlist, &parsed.report).map(|_| ())
        }
        _ => usage(),
    }
}

fn usage<T>() -> Result<T> {
    bail!(
        "usage: xtask <capture-contracts|verify-captured-contracts|abi-check|verify-test-port-map|verify-test-port-coverage|stage-install|build-original-cmake-suite|run-original-ctest|build-original-autotools-suite|run-original-autotools-check|build-original-reference|perf-capture|perf-assert|verify-bootstrap-stage|verify-install-contract|verify-driver-contract|compile-original-test-objects|relink-original-test-objects|build-original-standalone|run-relinked-original-tests|run-original-standalone|run-evdev-fixture-tests|run-fixture-backed-original-tests|run-gesture-replay|run-xvfb|run-xvfb-window-smoke|verify-dependent-regressions|security-regressions|final-check|unsafe-audit|verify-unsafe-allowlist> ..."
    )
}

fn security_regressions(repo_root: &Path) -> Result<()> {
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
        let status = Command::new("cargo")
            .current_dir(repo_root)
            .args([
                "test",
                "--manifest-path",
                "safe/Cargo.toml",
                "--test",
                &test,
            ])
            .status()
            .with_context(|| format!("run security regression test {test}"))?;
        if !status.success() {
            bail!("security regression test {test} failed with status {status}");
        }
    }

    Ok(())
}

#[derive(Debug)]
struct CommonArgs {
    generated: PathBuf,
    original: PathBuf,
    dependents: PathBuf,
    cves: PathBuf,
}

impl CommonArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut original = PathBuf::from("original");
        let mut dependents = PathBuf::from("dependents.json");
        let mut cves = PathBuf::from("relevant_cves.json");
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--original" => original = PathBuf::from(require_value(&mut iter, "--original")?),
                "--dependents" => {
                    dependents = PathBuf::from(require_value(&mut iter, "--dependents")?)
                }
                "--cves" => cves = PathBuf::from(require_value(&mut iter, "--cves")?),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            original,
            dependents,
            cves,
        })
    }

    fn into_contract_args(self, repo_root: PathBuf) -> ContractArgs {
        ContractArgs {
            repo_root,
            generated_dir: self.generated,
            original_dir: self.original,
            dependents_path: self.dependents,
            cves_path: self.cves,
        }
    }
}

#[derive(Debug)]
struct AbiCheckArgs {
    generated: PathBuf,
    symbols: Option<PathBuf>,
    dynapi: Option<PathBuf>,
    exports: Option<PathBuf>,
    library: Option<PathBuf>,
    require_soname: Option<String>,
}

impl AbiCheckArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut symbols = None;
        let mut dynapi = None;
        let mut exports = None;
        let mut library = None;
        let mut require_soname = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--symbols" => {
                    symbols = Some(PathBuf::from(require_value(&mut iter, "--symbols")?))
                }
                "--dynapi" => dynapi = Some(PathBuf::from(require_value(&mut iter, "--dynapi")?)),
                "--exports" => {
                    exports = Some(PathBuf::from(require_value(&mut iter, "--exports")?))
                }
                "--library" => {
                    library = Some(PathBuf::from(require_value(&mut iter, "--library")?))
                }
                "--require-soname" => {
                    require_soname = Some(require_value(&mut iter, "--require-soname")?.to_string())
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            symbols,
            dynapi,
            exports,
            library,
            require_soname,
        })
    }
}

#[derive(Debug)]
struct VerifyTestPortMapArgs {
    generated: PathBuf,
    original: PathBuf,
    map: Option<PathBuf>,
    expect_source_files: Option<usize>,
    expect_executable_targets: Option<usize>,
}

impl VerifyTestPortMapArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut original = PathBuf::from("original");
        let mut map = None;
        let mut expect_source_files = None;
        let mut expect_executable_targets = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--original" => original = PathBuf::from(require_value(&mut iter, "--original")?),
                "--map" => map = Some(PathBuf::from(require_value(&mut iter, "--map")?)),
                "--expect-source-files" => {
                    expect_source_files =
                        Some(require_value(&mut iter, "--expect-source-files")?.parse()?)
                }
                "--expect-executable-targets" => {
                    expect_executable_targets =
                        Some(require_value(&mut iter, "--expect-executable-targets")?.parse()?)
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            original,
            map,
            expect_source_files,
            expect_executable_targets,
        })
    }
}

#[derive(Debug)]
struct VerifyTestPortCoverageArgs {
    generated: PathBuf,
    original: PathBuf,
    map: Option<PathBuf>,
    phase: String,
    require_complete: bool,
    expect_source_files: Option<usize>,
    expect_executable_targets: Option<usize>,
}

impl VerifyTestPortCoverageArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut original = PathBuf::from("original");
        let mut map = None;
        let mut phase = PHASE_08_ID.to_string();
        let mut require_complete = false;
        let mut expect_source_files = None;
        let mut expect_executable_targets = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--original" => original = PathBuf::from(require_value(&mut iter, "--original")?),
                "--map" => map = Some(PathBuf::from(require_value(&mut iter, "--map")?)),
                "--phase" => phase = require_value(&mut iter, "--phase")?.to_string(),
                "--require-complete" => require_complete = true,
                "--expect-source-files" => {
                    expect_source_files =
                        Some(require_value(&mut iter, "--expect-source-files")?.parse()?)
                }
                "--expect-executable-targets" => {
                    expect_executable_targets =
                        Some(require_value(&mut iter, "--expect-executable-targets")?.parse()?)
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            original,
            map,
            phase,
            require_complete,
            expect_source_files,
            expect_executable_targets,
        })
    }
}

#[derive(Debug)]
struct StageInstallCliArgs {
    generated: PathBuf,
    original: PathBuf,
    root: PathBuf,
    library: Option<PathBuf>,
    mode: StageInstallMode,
}

impl StageInstallCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut original = PathBuf::from("original");
        let mut root = None;
        let mut library = None;
        let mut mode = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--original" => original = PathBuf::from(require_value(&mut iter, "--original")?),
                "--mode" => mode = Some(require_value(&mut iter, "--mode")?.to_string()),
                "--root" | "--destdir" => {
                    root = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                "--library" => {
                    library = Some(PathBuf::from(require_value(&mut iter, "--library")?))
                }
                other => bail!("unknown argument {other}"),
            }
        }
        let mode = mode.unwrap_or_else(|| "full".to_string());
        let mode = match mode.as_str() {
            "bootstrap" => StageInstallMode::Bootstrap,
            "runtime" => StageInstallMode::Runtime,
            "full" => StageInstallMode::Full,
            _ => bail!("unsupported --mode {mode}"),
        };
        Ok(Self {
            generated,
            original,
            root: root.ok_or_else(|| anyhow!("--root or --destdir is required"))?,
            library,
            mode,
        })
    }
}

#[derive(Debug)]
struct FinalCheckCliArgs {
    generated: PathBuf,
    original: PathBuf,
    dependents: PathBuf,
    cves: PathBuf,
    relink_objects_dir: PathBuf,
    relink_bin_dir: PathBuf,
    unsafe_allowlist: PathBuf,
    phase_report: PathBuf,
    unsafe_report: PathBuf,
    dependent_regression_manifest: PathBuf,
    dependent_matrix_results: PathBuf,
    dependent_matrix_artifact_dir: PathBuf,
}

impl FinalCheckCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut original = PathBuf::from("original");
        let mut dependents = PathBuf::from("dependents.json");
        let mut cves = PathBuf::from("relevant_cves.json");
        let mut relink_objects_dir = PathBuf::from("build-phase10-relinked-objects");
        let mut relink_bin_dir = PathBuf::from("build-phase10-relinked-bins");
        let mut unsafe_allowlist = PathBuf::from("safe/docs/unsafe-allowlist.md");
        let mut phase_report = PathBuf::from("safe/generated/reports/phase10-final-check.json");
        let mut unsafe_report = PathBuf::from("safe/generated/reports/unsafe-audit.json");
        let mut dependent_regression_manifest =
            PathBuf::from("safe/generated/dependent_regression_manifest.json");
        let mut dependent_matrix_results =
            PathBuf::from("safe/generated/reports/dependent-matrix-results.json");
        let mut dependent_matrix_artifact_dir =
            PathBuf::from("safe/generated/reports/dependent-matrix");
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--original" => original = PathBuf::from(require_value(&mut iter, "--original")?),
                "--dependents" => {
                    dependents = PathBuf::from(require_value(&mut iter, "--dependents")?)
                }
                "--cves" => cves = PathBuf::from(require_value(&mut iter, "--cves")?),
                "--relink-objects-dir" => {
                    relink_objects_dir =
                        PathBuf::from(require_value(&mut iter, "--relink-objects-dir")?)
                }
                "--relink-bin-dir" => {
                    relink_bin_dir = PathBuf::from(require_value(&mut iter, "--relink-bin-dir")?)
                }
                "--unsafe-allowlist" => {
                    unsafe_allowlist =
                        PathBuf::from(require_value(&mut iter, "--unsafe-allowlist")?)
                }
                "--phase-report" => {
                    phase_report = PathBuf::from(require_value(&mut iter, "--phase-report")?)
                }
                "--unsafe-report" => {
                    unsafe_report = PathBuf::from(require_value(&mut iter, "--unsafe-report")?)
                }
                "--dependent-regression-manifest" => {
                    dependent_regression_manifest =
                        PathBuf::from(require_value(&mut iter, "--dependent-regression-manifest")?)
                }
                "--dependent-matrix-results" => {
                    dependent_matrix_results =
                        PathBuf::from(require_value(&mut iter, "--dependent-matrix-results")?)
                }
                "--dependent-matrix-artifact-dir" => {
                    dependent_matrix_artifact_dir =
                        PathBuf::from(require_value(&mut iter, "--dependent-matrix-artifact-dir")?)
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            original,
            dependents,
            cves,
            relink_objects_dir,
            relink_bin_dir,
            unsafe_allowlist,
            phase_report,
            unsafe_report,
            dependent_regression_manifest,
            dependent_matrix_results,
            dependent_matrix_artifact_dir,
        })
    }
}

#[derive(Debug)]
struct VerifyDependentRegressionsCliArgs {
    dependents: PathBuf,
    manifest: PathBuf,
    results: PathBuf,
}

impl VerifyDependentRegressionsCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut dependents = PathBuf::from("dependents.json");
        let mut manifest = PathBuf::from("safe/generated/dependent_regression_manifest.json");
        let mut results = PathBuf::from("safe/generated/reports/dependent-matrix-results.json");
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--dependents" => {
                    dependents = PathBuf::from(require_value(&mut iter, "--dependents")?)
                }
                "--manifest" => manifest = PathBuf::from(require_value(&mut iter, "--manifest")?),
                "--results" => results = PathBuf::from(require_value(&mut iter, "--results")?),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            dependents,
            manifest,
            results,
        })
    }
}

#[derive(Debug)]
struct VerifyUnsafeAllowlistCliArgs {
    allowlist: PathBuf,
    report: PathBuf,
}

impl VerifyUnsafeAllowlistCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut allowlist = PathBuf::from("safe/docs/unsafe-allowlist.md");
        let mut report = PathBuf::from("safe/generated/reports/unsafe-audit.json");
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--allowlist" => {
                    allowlist = PathBuf::from(require_value(&mut iter, "--allowlist")?)
                }
                "--report" => report = PathBuf::from(require_value(&mut iter, "--report")?),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self { allowlist, report })
    }
}

#[derive(Debug)]
struct BuildOriginalStandaloneCliArgs {
    generated: PathBuf,
    manifest: PathBuf,
    destdir: PathBuf,
    build_dir: PathBuf,
    phase: String,
}

#[derive(Debug)]
struct OriginalSuiteCliArgs {
    original: PathBuf,
    root: PathBuf,
    build_dir: PathBuf,
}

#[derive(Debug)]
struct BuildOriginalReferenceCliArgs {
    original: PathBuf,
    build_dir: PathBuf,
    prefix: PathBuf,
}

impl BuildOriginalReferenceCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut original = PathBuf::from("original");
        let mut build_dir = PathBuf::from(DEFAULT_ORIGINAL_BUILD_DIR);
        let mut prefix = PathBuf::from(DEFAULT_ORIGINAL_PREFIX);
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--original" | "--source" => {
                    original = PathBuf::from(require_value(&mut iter, arg)?)
                }
                "--build-dir" => {
                    build_dir = PathBuf::from(require_value(&mut iter, "--build-dir")?)
                }
                "--prefix" | "--root" | "--destdir" => {
                    prefix = PathBuf::from(require_value(&mut iter, arg)?)
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            original,
            build_dir,
            prefix,
        })
    }
}

#[derive(Debug)]
struct PerfCaptureCliArgs {
    generated: PathBuf,
    original: PathBuf,
    original_prefix: PathBuf,
    safe_stage: PathBuf,
    runner_dir: PathBuf,
    manifest: PathBuf,
    thresholds: PathBuf,
    report: PathBuf,
    waivers: PathBuf,
}

impl PerfCaptureCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut original = PathBuf::from("original");
        let mut original_prefix = PathBuf::from(DEFAULT_ORIGINAL_PREFIX);
        let mut safe_stage = PathBuf::from(DEFAULT_SAFE_STAGE_ROOT);
        let mut runner_dir = PathBuf::from(DEFAULT_PERF_RUNNER_DIR);
        let mut manifest = PathBuf::from(DEFAULT_PERF_MANIFEST);
        let mut thresholds = PathBuf::from(DEFAULT_PERF_THRESHOLDS);
        let mut report = PathBuf::from(DEFAULT_PERF_REPORT);
        let mut waivers = PathBuf::from(DEFAULT_PERF_WAIVERS);
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--original" | "--source" => {
                    original = PathBuf::from(require_value(&mut iter, arg)?)
                }
                "--original-prefix" | "--reference-prefix" | "--prefix" => {
                    original_prefix = PathBuf::from(require_value(&mut iter, arg)?)
                }
                "--safe-stage" | "--safe-root" | "--stage-root" | "--candidate-prefix" => {
                    safe_stage = PathBuf::from(require_value(&mut iter, arg)?)
                }
                "--runner-dir" => {
                    runner_dir = PathBuf::from(require_value(&mut iter, "--runner-dir")?)
                }
                "--manifest" => manifest = PathBuf::from(require_value(&mut iter, "--manifest")?),
                "--thresholds" => {
                    thresholds = PathBuf::from(require_value(&mut iter, "--thresholds")?)
                }
                "--report" | "--output" => report = PathBuf::from(require_value(&mut iter, arg)?),
                "--waivers" => waivers = PathBuf::from(require_value(&mut iter, "--waivers")?),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            original,
            original_prefix,
            safe_stage,
            runner_dir,
            manifest,
            thresholds,
            report,
            waivers,
        })
    }
}

#[derive(Debug)]
struct PerfAssertCliArgs {
    thresholds: PathBuf,
    report: PathBuf,
    waivers: PathBuf,
}

impl PerfAssertCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut thresholds = PathBuf::from(DEFAULT_PERF_THRESHOLDS);
        let mut report = PathBuf::from(DEFAULT_PERF_REPORT);
        let mut waivers = PathBuf::from(DEFAULT_PERF_WAIVERS);
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--thresholds" => {
                    thresholds = PathBuf::from(require_value(&mut iter, "--thresholds")?)
                }
                "--report" | "--output" => report = PathBuf::from(require_value(&mut iter, arg)?),
                "--waivers" => waivers = PathBuf::from(require_value(&mut iter, "--waivers")?),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            thresholds,
            report,
            waivers,
        })
    }
}

impl OriginalSuiteCliArgs {
    fn parse(args: &[String], default_build_dir: &str) -> Result<Self> {
        let mut original = PathBuf::from("original");
        let mut root = PathBuf::from("build-phase8-stage");
        let mut build_dir = PathBuf::from(default_build_dir);
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--original" => original = PathBuf::from(require_value(&mut iter, "--original")?),
                "--root" | "--stage-root" | "--destdir" => {
                    root = PathBuf::from(require_value(&mut iter, arg)?)
                }
                "--build-dir" => {
                    build_dir = PathBuf::from(require_value(&mut iter, "--build-dir")?)
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            original,
            root,
            build_dir,
        })
    }
}

#[derive(Debug)]
struct RunOriginalAutotoolsCheckCliArgs {
    root: Option<PathBuf>,
    build_dir: PathBuf,
}

impl RunOriginalAutotoolsCheckCliArgs {
    fn parse(args: &[String], default_build_dir: &str) -> Result<Self> {
        let mut root = None;
        let mut build_dir = PathBuf::from(default_build_dir);
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--root" | "--stage-root" | "--destdir" => {
                    root = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                "--build-dir" => {
                    build_dir = PathBuf::from(require_value(&mut iter, "--build-dir")?)
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self { root, build_dir })
    }
}

#[derive(Debug)]
struct OriginalCtestCliArgs {
    build_dir: PathBuf,
    root: Option<PathBuf>,
    filter: Option<String>,
    test_list: Option<String>,
}

impl OriginalCtestCliArgs {
    fn parse(args: &[String], default_build_dir: &str) -> Result<Self> {
        let mut build_dir = PathBuf::from(default_build_dir);
        let mut root = None;
        let mut filter = None;
        let mut test_list = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--build-dir" => {
                    build_dir = PathBuf::from(require_value(&mut iter, "--build-dir")?)
                }
                "--root" => root = Some(PathBuf::from(require_value(&mut iter, "--root")?)),
                "--filter" | "--regex" => filter = Some(require_value(&mut iter, arg)?.to_string()),
                "--test-list" => {
                    test_list = Some(require_value(&mut iter, "--test-list")?.to_string())
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            build_dir,
            root,
            filter,
            test_list,
        })
    }
}

impl BuildOriginalStandaloneCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut manifest = None;
        let mut destdir = None;
        let mut build_dir = None;
        let mut phase = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--manifest" => {
                    manifest = Some(PathBuf::from(require_value(&mut iter, "--manifest")?))
                }
                "--destdir" | "--root" => {
                    destdir = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                "--build-dir" => {
                    build_dir = Some(PathBuf::from(require_value(&mut iter, "--build-dir")?))
                }
                "--phase" => phase = Some(require_value(&mut iter, "--phase")?.to_string()),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            manifest: manifest
                .unwrap_or_else(|| PathBuf::from("safe/generated/standalone_test_manifest.json")),
            destdir: destdir.ok_or_else(|| anyhow!("--destdir or --root is required"))?,
            build_dir: build_dir.ok_or_else(|| anyhow!("--build-dir is required"))?,
            phase: phase.ok_or_else(|| anyhow!("--phase is required"))?,
        })
    }
}

#[derive(Debug)]
struct VerifyBootstrapStageCliArgs {
    generated: PathBuf,
    root: PathBuf,
}

impl VerifyBootstrapStageCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut root = None;
        let mut require = Vec::new();
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--require" => require.push(require_value(&mut iter, "--require")?.to_string()),
                "--root" | "--destdir" => {
                    root = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                other => bail!("unknown argument {other}"),
            }
        }
        let _ = require;
        Ok(Self {
            generated,
            root: root.ok_or_else(|| anyhow!("--root or --destdir is required"))?,
        })
    }
}

#[derive(Debug)]
struct VerifyInstallContractCliArgs {
    generated: PathBuf,
    original: PathBuf,
    root: PathBuf,
    contract: Option<PathBuf>,
    public_header_inventory: Option<PathBuf>,
    mode: Option<String>,
}

impl VerifyInstallContractCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut original = PathBuf::from("original");
        let mut root = None;
        let mut contract = None;
        let mut public_header_inventory = None;
        let mut mode = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--original" => original = PathBuf::from(require_value(&mut iter, "--original")?),
                "--contract" => {
                    contract = Some(PathBuf::from(require_value(&mut iter, "--contract")?))
                }
                "--public-header-inventory" => {
                    public_header_inventory = Some(PathBuf::from(require_value(
                        &mut iter,
                        "--public-header-inventory",
                    )?))
                }
                "--mode" => mode = Some(require_value(&mut iter, "--mode")?.to_string()),
                "--package-root" | "--root" | "--destdir" => {
                    root = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            original,
            root: root
                .ok_or_else(|| anyhow!("--package-root, --root, or --destdir is required"))?,
            contract,
            public_header_inventory,
            mode,
        })
    }
}

#[derive(Debug)]
struct VerifyDriverContractCliArgs {
    contract: PathBuf,
    root: PathBuf,
    kind: String,
}

impl VerifyDriverContractCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut contract = None;
        let mut root = None;
        let mut kind = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--contract" => {
                    contract = Some(PathBuf::from(require_value(&mut iter, "--contract")?))
                }
                "--package-root" | "--root" | "--destdir" => {
                    root = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                "--kind" => kind = Some(require_value(&mut iter, "--kind")?.to_string()),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            contract: contract.unwrap_or_else(|| generated.join("driver_contract.json")),
            root: root.ok_or_else(|| anyhow!("--root or --destdir is required"))?,
            kind: kind.ok_or_else(|| anyhow!("--kind is required"))?,
        })
    }
}

#[derive(Debug)]
struct RunXvfbCliArgs {
    command: Vec<String>,
}

impl RunXvfbCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let command = if let Some(separator) = args.iter().position(|arg| arg == "--") {
            args[separator + 1..].to_vec()
        } else {
            args.to_vec()
        };
        if command.is_empty() {
            bail!("run-xvfb requires a command after --");
        }
        Ok(Self { command })
    }
}

#[derive(Debug)]
struct CompileOriginalCliArgs {
    generated: PathBuf,
    object_manifest: Option<PathBuf>,
    output_dir: PathBuf,
}

impl CompileOriginalCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut object_manifest = None;
        let mut output_dir = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--object-manifest" | "--manifest" => {
                    object_manifest = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                "--output-dir" | "--out" => {
                    output_dir = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            object_manifest,
            output_dir: output_dir.ok_or_else(|| anyhow!("--output-dir or --out is required"))?,
        })
    }
}

#[derive(Debug)]
struct RelinkOriginalCliArgs {
    generated: PathBuf,
    object_manifest: Option<PathBuf>,
    standalone_manifest: Option<PathBuf>,
    objects_dir: PathBuf,
    output_dir: PathBuf,
    library: PathBuf,
}

impl RelinkOriginalCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut object_manifest = None;
        let mut standalone_manifest = None;
        let mut objects_dir = None;
        let mut output_dir = PathBuf::from("build-phase10-relinked-bins");
        let mut library = None;
        let mut package_root = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--object-manifest" | "--manifest" => {
                    object_manifest = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                "--standalone-manifest" => {
                    standalone_manifest = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                "--objects-dir" => {
                    objects_dir = Some(PathBuf::from(require_value(&mut iter, "--objects-dir")?))
                }
                "--output-dir" | "--out" | "--build-dir" => {
                    output_dir = PathBuf::from(require_value(&mut iter, arg)?)
                }
                "--package-root" | "--root" | "--destdir" => {
                    package_root = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                "--library" => {
                    library = Some(PathBuf::from(require_value(&mut iter, "--library")?))
                }
                other => bail!("unknown argument {other}"),
            }
        }
        let library = library.or_else(|| {
            package_root.map(|root| root.join(format!("usr/lib/{UBUNTU_MULTIARCH}/libSDL2-2.0.so")))
        });
        Ok(Self {
            generated,
            object_manifest,
            standalone_manifest,
            objects_dir: objects_dir.ok_or_else(|| anyhow!("--objects-dir is required"))?,
            output_dir,
            library: library.ok_or_else(|| anyhow!("--library or --package-root is required"))?,
        })
    }
}

#[derive(Debug)]
struct RunRelinkedCliArgs {
    generated: PathBuf,
    manifest: PathBuf,
    bin_dir: PathBuf,
    target: Option<String>,
    validation_modes: Vec<String>,
    skip_if_empty: bool,
}

impl RunRelinkedCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut manifest = PathBuf::from("safe/generated/standalone_test_manifest.json");
        let mut bin_dir = None;
        let mut target = None;
        let mut validation_modes = vec!["auto_run".to_string(), "fixture_run".to_string()];
        let mut skip_if_empty = false;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--standalone-manifest" | "--manifest" => {
                    manifest = PathBuf::from(require_value(&mut iter, arg)?)
                }
                "--object-manifest" => {
                    let _ = require_value(&mut iter, "--object-manifest")?;
                }
                "--bin-dir" | "--build-dir" => {
                    bin_dir = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                "--validation-mode" | "--validation-modes" => {
                    validation_modes = require_value(&mut iter, arg)?
                        .split(',')
                        .map(str::trim)
                        .filter(|mode| !mode.is_empty())
                        .map(ToOwned::to_owned)
                        .collect();
                }
                "--skip-if-empty" => skip_if_empty = true,
                "--target" => target = Some(require_value(&mut iter, "--target")?.to_string()),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            manifest,
            bin_dir: bin_dir.ok_or_else(|| anyhow!("--bin-dir is required"))?,
            target,
            validation_modes,
            skip_if_empty,
        })
    }
}

#[derive(Debug)]
struct RunOriginalStandaloneCliArgs {
    generated: PathBuf,
    manifest: PathBuf,
    build_dir: PathBuf,
    phase: String,
    validation_mode: String,
    skip_if_empty: bool,
}

impl RunOriginalStandaloneCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut manifest = None;
        let mut build_dir = None;
        let mut phase = None;
        let mut validation_mode = "auto_run".to_string();
        let mut skip_if_empty = false;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--manifest" => {
                    manifest = Some(PathBuf::from(require_value(&mut iter, "--manifest")?))
                }
                "--build-dir" => {
                    build_dir = Some(PathBuf::from(require_value(&mut iter, "--build-dir")?))
                }
                "--phase" => phase = Some(require_value(&mut iter, "--phase")?.to_string()),
                "--validation-mode" => {
                    validation_mode = require_value(&mut iter, "--validation-mode")?.to_string()
                }
                "--skip-if-empty" => skip_if_empty = true,
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            manifest: manifest
                .unwrap_or_else(|| PathBuf::from("safe/generated/standalone_test_manifest.json")),
            build_dir: build_dir.ok_or_else(|| anyhow!("--build-dir is required"))?,
            phase: phase.ok_or_else(|| anyhow!("--phase is required"))?,
            validation_mode,
            skip_if_empty,
        })
    }
}

#[derive(Debug)]
struct RunFixtureBackedOriginalTestsCliArgs {
    generated: PathBuf,
    manifest: PathBuf,
    build_dir: PathBuf,
    phase: String,
    skip_if_empty: bool,
}

impl RunFixtureBackedOriginalTestsCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut manifest = PathBuf::from("safe/generated/standalone_test_manifest.json");
        let mut build_dir = PathBuf::from("build-phase7-standalone");
        let mut phase = "impl_phase_07_input_devices".to_string();
        let mut skip_if_empty = false;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--manifest" => manifest = PathBuf::from(require_value(&mut iter, "--manifest")?),
                "--build-dir" => {
                    build_dir = PathBuf::from(require_value(&mut iter, "--build-dir")?)
                }
                "--phase" => phase = require_value(&mut iter, "--phase")?.to_string(),
                "--skip-if-empty" => skip_if_empty = true,
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            manifest,
            build_dir,
            phase,
            skip_if_empty,
        })
    }
}

fn require_value<'a, I>(iter: &mut I, flag: &str) -> Result<&'a str>
where
    I: Iterator<Item = &'a String>,
{
    iter.next()
        .map(|value| value.as_str())
        .ok_or_else(|| anyhow!("{flag} requires a value"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{
        CompileOriginalCliArgs, OriginalCtestCliArgs, RelinkOriginalCliArgs,
        RunOriginalAutotoolsCheckCliArgs, RunRelinkedCliArgs, VerifyDriverContractCliArgs,
        VerifyInstallContractCliArgs, VerifyTestPortCoverageArgs, PHASE_08_ID,
    };
    use crate::contracts::{load_original_test_port_map, verify_test_port_coverage};
    use tempfile::tempdir;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("workspace root")
            .to_path_buf()
    }

    #[test]
    fn verify_test_port_coverage_defaults_phase_08() {
        let parsed = VerifyTestPortCoverageArgs::parse(&["--require-complete".to_string()])
            .expect("parse verify-test-port-coverage args");
        assert_eq!(parsed.phase, PHASE_08_ID);
    }

    #[test]
    fn run_original_ctest_accepts_test_list_flag() {
        let parsed = OriginalCtestCliArgs::parse(
            &[
                "--build-dir".to_string(),
                "build-phase8-upstream-cmake".to_string(),
                "--test-list".to_string(),
                "safe/generated/noninteractive_test_list.json".to_string(),
            ],
            "ignored",
        )
        .expect("parse run-original-ctest args");
        assert_eq!(
            parsed.test_list.as_deref(),
            Some("safe/generated/noninteractive_test_list.json")
        );
    }

    #[test]
    fn run_original_autotools_check_does_not_default_stage_root() {
        let parsed = RunOriginalAutotoolsCheckCliArgs::parse(
            &[
                "--build-dir".to_string(),
                "/tmp/libsdl-safe-autotools".to_string(),
            ],
            "ignored",
        )
        .expect("parse run-original-autotools-check args");
        assert_eq!(parsed.root, None);
        assert_eq!(
            parsed.build_dir,
            PathBuf::from("/tmp/libsdl-safe-autotools")
        );
    }

    #[test]
    fn verify_install_contract_accepts_package_root_flag() {
        let parsed = VerifyInstallContractCliArgs::parse(&[
            "--package-root".to_string(),
            "/tmp/pkgroot".to_string(),
        ])
        .expect("parse verify-install-contract args");
        assert_eq!(parsed.root, PathBuf::from("/tmp/pkgroot"));
    }

    #[test]
    fn verify_install_contract_accepts_checker_compatibility_flags() {
        let parsed = VerifyInstallContractCliArgs::parse(&[
            "--contract".to_string(),
            "safe/generated/install_contract.json".to_string(),
            "--public-header-inventory".to_string(),
            "safe/generated/public_header_inventory.json".to_string(),
            "--package-root".to_string(),
            "/".to_string(),
            "--mode".to_string(),
            "packaged".to_string(),
        ])
        .expect("parse checker-compatible verify-install-contract args");
        assert_eq!(
            parsed.contract,
            Some(PathBuf::from("safe/generated/install_contract.json"))
        );
        assert_eq!(
            parsed.public_header_inventory,
            Some(PathBuf::from("safe/generated/public_header_inventory.json"))
        );
        assert_eq!(parsed.mode.as_deref(), Some("packaged"));
    }

    #[test]
    fn verify_driver_contract_accepts_package_root_flag() {
        let parsed = VerifyDriverContractCliArgs::parse(&[
            "--package-root".to_string(),
            "/tmp/pkgroot".to_string(),
            "--kind".to_string(),
            "video".to_string(),
        ])
        .expect("parse verify-driver-contract args");
        assert_eq!(parsed.root, PathBuf::from("/tmp/pkgroot"));
        assert_eq!(parsed.kind, "video");
    }

    #[test]
    fn relink_original_cli_derives_library_from_package_root() {
        let parsed = RelinkOriginalCliArgs::parse(&[
            "--objects-dir".to_string(),
            "build-phase10-relinked-objects".to_string(),
            "--out".to_string(),
            "build-phase10-relinked-bins".to_string(),
            "--package-root".to_string(),
            "/tmp/pkgroot".to_string(),
            "--object-manifest".to_string(),
            "safe/generated/original_test_object_manifest.json".to_string(),
            "--standalone-manifest".to_string(),
            "safe/generated/standalone_test_manifest.json".to_string(),
        ])
        .expect("parse relink-original-test-objects args");
        assert_eq!(
            parsed.library,
            PathBuf::from("/tmp/pkgroot/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so")
        );
        assert_eq!(
            parsed.object_manifest,
            Some(PathBuf::from(
                "safe/generated/original_test_object_manifest.json"
            ))
        );
        assert_eq!(
            parsed.standalone_manifest,
            Some(PathBuf::from(
                "safe/generated/standalone_test_manifest.json"
            ))
        );
    }

    #[test]
    fn relink_original_cli_defaults_output_dir() {
        let parsed = RelinkOriginalCliArgs::parse(&[
            "--objects-dir".to_string(),
            "build-phase10-relinked-objects".to_string(),
            "--package-root".to_string(),
            "/tmp/pkgroot".to_string(),
        ])
        .expect("parse relink-original-test-objects args");
        assert_eq!(
            parsed.output_dir,
            PathBuf::from("build-phase10-relinked-bins")
        );
    }

    #[test]
    fn relink_original_cli_accepts_manifest_as_object_manifest() {
        let parsed = RelinkOriginalCliArgs::parse(&[
            "--manifest".to_string(),
            "safe/generated/original_test_object_manifest.json".to_string(),
            "--objects-dir".to_string(),
            "build-phase10-relinked-objects".to_string(),
            "--package-root".to_string(),
            "/tmp/pkgroot".to_string(),
        ])
        .expect("parse relink-original-test-objects args");
        assert_eq!(
            parsed.object_manifest,
            Some(PathBuf::from(
                "safe/generated/original_test_object_manifest.json"
            ))
        );
        assert_eq!(parsed.standalone_manifest, None);
    }

    #[test]
    fn run_relinked_cli_accepts_standalone_manifest() {
        let parsed = RunRelinkedCliArgs::parse(&[
            "--object-manifest".to_string(),
            "safe/generated/original_test_object_manifest.json".to_string(),
            "--bin-dir".to_string(),
            "build-phase10-relinked-bins".to_string(),
            "--standalone-manifest".to_string(),
            "safe/generated/standalone_test_manifest.json".to_string(),
        ])
        .expect("parse run-relinked-original-tests args");
        assert_eq!(
            parsed.manifest,
            PathBuf::from("safe/generated/standalone_test_manifest.json")
        );
    }

    #[test]
    fn run_relinked_cli_accepts_build_dir_alias() {
        let parsed = RunRelinkedCliArgs::parse(&[
            "--build-dir".to_string(),
            "build-phase10-relinked-bins".to_string(),
        ])
        .expect("parse run-relinked-original-tests args");
        assert_eq!(parsed.bin_dir, PathBuf::from("build-phase10-relinked-bins"));
    }

    #[test]
    fn run_relinked_cli_accepts_validation_modes_and_skip_if_empty() {
        let parsed = RunRelinkedCliArgs::parse(&[
            "--build-dir".to_string(),
            "build-phase10-relinked-bins".to_string(),
            "--validation-modes".to_string(),
            "auto_run,fixture_run".to_string(),
            "--skip-if-empty".to_string(),
        ])
        .expect("parse run-relinked-original-tests args");
        assert_eq!(
            parsed.validation_modes,
            vec!["auto_run".to_string(), "fixture_run".to_string()]
        );
        assert!(parsed.skip_if_empty);
    }

    #[test]
    fn compile_original_cli_accepts_out_alias() {
        let parsed = CompileOriginalCliArgs::parse(&[
            "--out".to_string(),
            "build-phase10-relinked-objects".to_string(),
        ])
        .expect("parse compile-original-test-objects args");
        assert_eq!(
            parsed.output_dir,
            PathBuf::from("build-phase10-relinked-objects")
        );
    }

    #[test]
    fn verify_test_port_coverage_rejects_removed_source_entry() {
        let repo_root = repo_root();
        let source_map = repo_root.join("safe/generated/original_test_port_map.json");
        let mut port_map = load_original_test_port_map(&source_map).expect("load port map");
        port_map.entries.pop().expect("remove one source entry");

        let temp = tempdir().expect("temporary map dir");
        let temp_map = temp.path().join("original_test_port_map.json");
        let mut bytes = serde_json::to_vec_pretty(&port_map).expect("serialize temp port map");
        bytes.push(b'\n');
        fs::write(&temp_map, bytes).expect("write temp port map");

        let err = verify_test_port_coverage(
            &repo_root,
            &temp_map,
            &repo_root.join("original"),
            PHASE_08_ID,
            true,
            Some(116),
            Some(71),
        )
        .expect_err("removed source entry must fail coverage verification");

        assert!(
            err.to_string()
                .contains("expected 116 upstream test/support source files, found 115"),
            "unexpected error: {err:#}"
        );
    }
}
