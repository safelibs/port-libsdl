#[path = "common/testutils.rs"]
mod testutils;

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
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
