mod contracts;
mod original_tests;
mod stage_install;

use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};

use contracts::{abi_check, capture_contracts, verify_captured_contracts, verify_test_port_map, ContractArgs};
use original_tests::{
    compile_original_test_objects, relink_original_test_objects, run_relinked_original_tests,
    CompileOriginalTestObjectsArgs, RelinkOriginalTestObjectsArgs, RunRelinkedOriginalTestsArgs,
};
use stage_install::{
    stage_install, verify_bootstrap_stage, StageInstallArgs, VerifyBootstrapStageArgs,
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
            verify_test_port_map(&repo_root, &map_path, &parsed.original)
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
        "run-relinked-original-tests" => {
            let parsed = RunRelinkedCliArgs::parse(&remaining)?;
            run_relinked_original_tests(RunRelinkedOriginalTestsArgs {
                repo_root,
                generated_dir: parsed.generated,
                bin_dir: parsed.bin_dir,
                filter: parsed.target,
            })
        }
        _ => usage(),
    }
}

fn usage<T>() -> Result<T> {
    bail!(
        "usage: xtask <capture-contracts|verify-captured-contracts|abi-check|verify-test-port-map|stage-install|verify-bootstrap-stage|compile-original-test-objects|relink-original-test-objects|run-relinked-original-tests> ..."
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
                "--generated" => generated = PathBuf::from(require_value(&mut iter, "--generated")?),
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
                "--generated" => generated = PathBuf::from(require_value(&mut iter, "--generated")?),
                "--symbols" => symbols = Some(PathBuf::from(require_value(&mut iter, "--symbols")?)),
                "--dynapi" => dynapi = Some(PathBuf::from(require_value(&mut iter, "--dynapi")?)),
                "--exports" => exports = Some(PathBuf::from(require_value(&mut iter, "--exports")?)),
                "--library" => library = Some(PathBuf::from(require_value(&mut iter, "--library")?)),
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
}

impl VerifyTestPortMapArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut original = PathBuf::from("original");
        let mut map = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => generated = PathBuf::from(require_value(&mut iter, "--generated")?),
                "--original" => original = PathBuf::from(require_value(&mut iter, "--original")?),
                "--map" => map = Some(PathBuf::from(require_value(&mut iter, "--map")?)),
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            original,
            map,
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
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => generated = PathBuf::from(require_value(&mut iter, "--generated")?),
                "--original" => original = PathBuf::from(require_value(&mut iter, "--original")?),
                "--root" | "--destdir" => {
                    root = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                "--library" => library = Some(PathBuf::from(require_value(&mut iter, "--library")?)),
                other => bail!("unknown argument {other}"),
            }
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
struct VerifyBootstrapStageCliArgs {
    generated: PathBuf,
    root: PathBuf,
}

impl VerifyBootstrapStageCliArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let mut generated = PathBuf::from("safe/generated");
        let mut root = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--generated" => generated = PathBuf::from(require_value(&mut iter, "--generated")?),
                "--root" | "--destdir" => {
                    root = Some(PathBuf::from(require_value(&mut iter, arg)?))
                }
                other => bail!("unknown argument {other}"),
            }
        }
        Ok(Self {
            generated,
            root: root.ok_or_else(|| anyhow!("--root or --destdir is required"))?,
        })
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
                "--generated" => generated = PathBuf::from(require_value(&mut iter, "--generated")?),
                "--output-dir" => output_dir = Some(PathBuf::from(require_value(&mut iter, "--output-dir")?)),
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
                "--generated" => generated = PathBuf::from(require_value(&mut iter, "--generated")?),
                "--objects-dir" => objects_dir = Some(PathBuf::from(require_value(&mut iter, "--objects-dir")?)),
                "--output-dir" => output_dir = Some(PathBuf::from(require_value(&mut iter, "--output-dir")?)),
                "--library" => library = Some(PathBuf::from(require_value(&mut iter, "--library")?)),
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
                "--generated" => generated = PathBuf::from(require_value(&mut iter, "--generated")?),
                "--bin-dir" => bin_dir = Some(PathBuf::from(require_value(&mut iter, "--bin-dir")?)),
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

fn require_value<'a, I>(iter: &mut I, flag: &str) -> Result<&'a str>
where
    I: Iterator<Item = &'a String>,
{
    iter.next()
        .map(|value| value.as_str())
        .ok_or_else(|| anyhow!("{flag} requires a value"))
}
