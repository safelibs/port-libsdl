#![allow(clippy::all)]

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn installed_tests_metadata_is_materialized_from_the_upstream_template() {
    let root = repo_root();
    let base = root.join("safe/upstream-tests/installed-tests/usr/share/installed-tests/SDL2");
    for target in [
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
    ] {
        let path = base.join(format!("{target}.test"));
        let contents = std::fs::read_to_string(&path).expect("read .test file");
        assert!(contents.contains("[Test]"));
        assert!(contents.contains(&format!("Exec=/usr/libexec/installed-tests/SDL2/{target}")));
    }
    assert!(root
        .join("safe/upstream-tests/installed-tests/debian/tests/installed-tests")
        .exists());
}

#[test]
fn original_test_port_map_is_closed_out() {
    let root = repo_root();
    let path = root.join("safe/generated/original_test_port_map.json");
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("read original_test_port_map"))
            .expect("parse original_test_port_map");
    for entry in value["entries"].as_array().expect("entries array") {
        assert_eq!(entry["completion_state"].as_str(), Some("complete"));
    }
    for entry in value["target_ownership"]
        .as_array()
        .expect("target_ownership array")
    {
        assert_eq!(entry["completion_state"].as_str(), Some("complete"));
    }
}
