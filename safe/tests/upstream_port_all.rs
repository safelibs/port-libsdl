#![allow(clippy::all)]

#[path = "common/testutils.rs"]
mod testutils;

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn noninteractive_manifest_projection_matches_authoritative_upstream_set() {
    let root = repo_root();
    let path = root.join("safe/generated/noninteractive_test_list.json");
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("read noninteractive_test_list"))
            .expect("parse noninteractive_test_list");
    let targets = value["targets"]
        .as_array()
        .expect("targets array")
        .iter()
        .map(|entry| entry.as_str().expect("target string"))
        .collect::<Vec<_>>();
    assert_eq!(
        targets,
        vec![
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
        ]
    );
}

#[test]
fn xtask_verify_test_port_coverage_requires_a_fully_completed_map() {
    let root = repo_root();
    let status = Command::new("cargo")
        .current_dir(&root)
        .args([
            "run",
            "--manifest-path",
            "safe/Cargo.toml",
            "-p",
            "xtask",
            "--",
            "verify-test-port-coverage",
            "--phase",
            "impl_phase_08_testsupport_and_full_upstream_tests",
            "--require-complete",
        ])
        .status()
        .expect("run xtask verify-test-port-coverage");
    assert!(status.success());
}

#[test]
fn sdl2_test_symbols_stay_in_the_support_archive() {
    let root = repo_root();
    let release_dir = root.join("safe/target/release");
    let runtime_library = release_dir.join("libsafe_sdl.so");
    let support_archive = release_dir.join("libsafe_sdl2_test.a");

    let status = Command::new("cargo")
        .current_dir(&root)
        .args(["build", "--manifest-path", "safe/Cargo.toml", "--release"])
        .status()
        .expect("build release artifacts for symbol boundary check");
    assert!(status.success());

    let runtime_symbols = Command::new("nm")
        .arg("-D")
        .arg(&runtime_library)
        .output()
        .expect("inspect runtime library symbols");
    assert!(runtime_symbols.status.success());
    let runtime_stdout = String::from_utf8(runtime_symbols.stdout).expect("runtime nm stdout");
    assert!(
        !runtime_stdout.contains(" SDLTest_"),
        "libsafe_sdl.so unexpectedly exports SDLTest_* symbols"
    );

    let support_symbols = Command::new("nm")
        .args(["-g", "--defined-only"])
        .arg(&support_archive)
        .output()
        .expect("inspect SDL2_test archive symbols");
    assert!(support_symbols.status.success());
    let support_stdout = String::from_utf8(support_symbols.stdout).expect("support nm stdout");
    assert!(
        support_stdout.contains(" SDLTest_"),
        "libsafe_sdl2_test.a is missing SDLTest_* exports"
    );
}
