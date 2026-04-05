mod contracts;
mod original_tests;
mod stage_install;

use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};

use contracts::{
    abi_check, capture_contracts, verify_captured_contracts, verify_test_port_coverage,
    verify_test_port_map, ContractArgs,
};
use original_tests::{
    build_original_standalone, compile_original_test_objects, relink_original_test_objects,
    run_evdev_fixture_tests, run_fixture_backed_original_tests, run_gesture_replay,
    run_original_standalone, run_relinked_original_tests, run_xvfb, run_xvfb_window_smoke,
    BuildOriginalStandaloneArgs, CompileOriginalTestObjectsArgs, RelinkOriginalTestObjectsArgs,
    RunFixtureBackedOriginalTestsArgs, RunOriginalStandaloneArgs, RunRelinkedOriginalTestsArgs,
};
use stage_install::{
    stage_install, verify_bootstrap_stage, verify_driver_contract, StageInstallArgs,
    VerifyBootstrapStageArgs, VerifyDriverContractArgs,
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
                .unwrap_or_else(|| PathBuf::from("safe/src/exports/generated_linux_stubs.rs"));
            let dynapi_source = PathBuf::from("safe/src/dynapi/generated.rs");
            abi_check(
                &repo_root,
                &symbols_manifest,
                &dynapi_manifest,
                &exports_source,
                &dynapi_source,
                parsed.library.as_deref(),
                parsed.require_soname.as_deref(),
            )
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
            verify_test_port_coverage(&repo_root, &map_path, &parsed.phase)
        }
        "stage-install" => {
            let parsed = StageInstallCliArgs::parse(&remaining)?;
            stage_install(StageInstallArgs {
                repo_root,
                generated_dir: parsed.generated,
                original_dir: parsed.original,
                stage_root: parsed.root,
                library_path: parsed.library,
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
                output_dir: parsed.output_dir,
            })
        }
        "relink-original-test-objects" => {
            let parsed = RelinkOriginalCliArgs::parse(&remaining)?;
            relink_original_test_objects(RelinkOriginalTestObjectsArgs {
                repo_root,
                generated_dir: parsed.generated,
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
                bin_dir: parsed.bin_dir,
                filter: parsed.target,
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
        _ => usage(),
    }
}

fn usage<T>() -> Result<T> {
    bail!(
        "usage: xtask <capture-contracts|verify-captured-contracts|abi-check|verify-test-port-map|verify-test-port-coverage|stage-install|verify-bootstrap-stage|verify-driver-contract|compile-original-test-objects|relink-original-test-objects|build-original-standalone|run-relinked-original-tests|run-original-standalone|run-evdev-fixture-tests|run-fixture-backed-original-tests|run-gesture-replay|run-xvfb|run-xvfb-window-smoke> ..."
    )
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
    map: Option<PathBuf>,
    phase: String,
}

impl VerifyTestPortCoverageArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut map = None;
        let mut phase = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--map" => map = Some(PathBuf::from(require_value(&mut iter, "--map")?)),
                "--phase" => phase = Some(require_value(&mut iter, "--phase")?.to_string()),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            map,
            phase: phase.ok_or_else(|| anyhow!("--phase is required"))?,
        })
    }
}

#[derive(Debug)]
struct StageInstallCliArgs {
    generated: PathBuf,
    original: PathBuf,
    root: PathBuf,
    library: Option<PathBuf>,
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
        let mode = mode.unwrap_or_else(|| "bootstrap".to_string());
        if mode != "bootstrap" && mode != "runtime" {
            bail!("unsupported --mode {mode}");
        }
        Ok(Self {
            generated,
            original,
            root: root.ok_or_else(|| anyhow!("--root or --destdir is required"))?,
            library,
        })
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
                "--root" | "--destdir" => {
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
    output_dir: PathBuf,
}

impl CompileOriginalCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut output_dir = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--output-dir" => {
                    output_dir = Some(PathBuf::from(require_value(&mut iter, "--output-dir")?))
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            output_dir: output_dir.ok_or_else(|| anyhow!("--output-dir is required"))?,
        })
    }
}

#[derive(Debug)]
struct RelinkOriginalCliArgs {
    generated: PathBuf,
    objects_dir: PathBuf,
    output_dir: PathBuf,
    library: PathBuf,
}

impl RelinkOriginalCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut objects_dir = None;
        let mut output_dir = None;
        let mut library = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--objects-dir" => {
                    objects_dir = Some(PathBuf::from(require_value(&mut iter, "--objects-dir")?))
                }
                "--output-dir" => {
                    output_dir = Some(PathBuf::from(require_value(&mut iter, "--output-dir")?))
                }
                "--library" => {
                    library = Some(PathBuf::from(require_value(&mut iter, "--library")?))
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            objects_dir: objects_dir.ok_or_else(|| anyhow!("--objects-dir is required"))?,
            output_dir: output_dir.ok_or_else(|| anyhow!("--output-dir is required"))?,
            library: library.ok_or_else(|| anyhow!("--library is required"))?,
        })
    }
}

#[derive(Debug)]
struct RunRelinkedCliArgs {
    generated: PathBuf,
    bin_dir: PathBuf,
    target: Option<String>,
}

impl RunRelinkedCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut bin_dir = None;
        let mut target = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => {
                    generated = PathBuf::from(require_value(&mut iter, "--generated")?)
                }
                "--bin-dir" => {
                    bin_dir = Some(PathBuf::from(require_value(&mut iter, "--bin-dir")?))
                }
                "--target" => target = Some(require_value(&mut iter, "--target")?.to_string()),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            bin_dir: bin_dir.ok_or_else(|| anyhow!("--bin-dir is required"))?,
            target,
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
