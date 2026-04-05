#[path = "common/testutils.rs"]
mod testutils;

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn load_json(path: &PathBuf) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read json artifact"))
        .expect("parse json artifact")
}

#[test]
fn perf_artifacts_cover_the_phase_09_workloads() {
    let repo_root = testutils::repo_root();
    let manifest_path = repo_root.join("safe/generated/perf_workload_manifest.json");
    let thresholds_path = repo_root.join("safe/generated/perf_thresholds.json");
    let report_path = repo_root.join("safe/generated/reports/perf-baseline-vs-safe.json");
    let waivers_path = repo_root.join("safe/generated/reports/perf-waivers.md");

    let manifest = load_json(&manifest_path);
    let thresholds = load_json(&thresholds_path);
    let report = load_json(&report_path);

    assert_eq!(
        manifest["phase_id"].as_str(),
        Some("impl_phase_09_performance")
    );
    assert_eq!(
        thresholds["phase_id"].as_str(),
        Some("impl_phase_09_performance")
    );
    assert_eq!(
        report["phase_id"].as_str(),
        Some("impl_phase_09_performance")
    );

    let workload_ids = manifest["workloads"]
        .as_array()
        .expect("manifest workloads array")
        .iter()
        .map(|entry| entry["workload_id"].as_str().expect("workload id"))
        .collect::<BTreeSet<_>>();
    let threshold_ids = thresholds["workloads"]
        .as_array()
        .expect("threshold workloads array")
        .iter()
        .map(|entry| {
            entry["workload_id"]
                .as_str()
                .expect("threshold workload id")
        })
        .collect::<BTreeSet<_>>();
    let report_ids = report["workloads"]
        .as_array()
        .expect("report workloads array")
        .iter()
        .map(|entry| entry["workload_id"].as_str().expect("report workload id"))
        .collect::<BTreeSet<_>>();

    assert_eq!(workload_ids, threshold_ids);
    assert_eq!(workload_ids, report_ids);
    assert_eq!(
        workload_ids,
        BTreeSet::from([
            "audio_stream_convert_resample_wave",
            "controller_mapping_guid",
            "event_queue_throughput",
            "renderer_queue_copy_texture_upload",
            "surface_create_fill_convert_blit",
        ])
    );

    for resource in manifest["workloads"]
        .as_array()
        .expect("manifest workloads")
        .iter()
        .flat_map(|entry| {
            entry["resource_paths"]
                .as_array()
                .expect("resource_paths array")
                .iter()
        })
    {
        let path = repo_root.join(resource.as_str().expect("resource path"));
        assert!(path.exists(), "missing perf resource {}", path.display());
    }

    let default_policy = thresholds["default_policy"]
        .as_object()
        .expect("default policy object");
    assert_eq!(
        default_policy
            .get("samples_per_workload")
            .and_then(Value::as_u64),
        Some(5)
    );
    assert_eq!(
        default_policy
            .get("max_median_cpu_regression_ratio")
            .and_then(Value::as_f64),
        Some(1.2)
    );
    assert_eq!(
        default_policy
            .get("max_peak_allocation_regression_ratio")
            .and_then(Value::as_f64),
        Some(1.25)
    );

    let waivers = fs::read_to_string(&waivers_path).expect("read perf waivers");
    assert!(
        waivers.contains("No active waivers.") || waivers.contains("## `"),
        "unexpected waiver document contents"
    );
}
