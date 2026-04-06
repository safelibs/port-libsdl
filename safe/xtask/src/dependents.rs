use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

const PHASE_10_ID: &str = "impl_phase_10_packaging_dependents_final";

#[derive(Debug)]
pub struct VerifyDependentRegressionsArgs {
    pub repo_root: PathBuf,
    pub dependents_path: PathBuf,
    pub manifest_path: PathBuf,
    pub results_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct DependentsInventory {
    dependents: Vec<DependentDefinition>,
}

#[derive(Debug, Deserialize)]
struct DependentDefinition {
    slug: String,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DependentMatrixResults {
    pub schema_version: u32,
    pub phase_id: String,
    pub only_filter: Option<String>,
    pub dependents: Vec<DependentMatrixEntry>,
    pub summary: DependentMatrixSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DependentMatrixEntry {
    pub slug: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub duration_seconds: f64,
    #[serde(default)]
    pub log_path: Option<String>,
    #[serde(default)]
    pub json_path: Option<String>,
    #[serde(default)]
    pub artifact_dir: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DependentMatrixSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DependentRegressionManifest {
    pub schema_version: u32,
    pub phase_id: String,
    #[serde(default)]
    pub issues: Vec<DependentRegressionIssue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DependentRegressionIssue {
    pub slug: String,
    pub dependent_name: String,
    pub status: String,
    pub summary: String,
    pub reproducer: DependentReproducer,
    #[serde(default)]
    pub fix_commit: String,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DependentReproducer {
    pub path: String,
    #[serde(default)]
    pub test_name: String,
    #[serde(default)]
    pub command: String,
}

pub fn verify_dependent_regressions(args: VerifyDependentRegressionsArgs) -> Result<()> {
    let dependents_path = absolutize(&args.repo_root, &args.dependents_path);
    let manifest_path = absolutize(&args.repo_root, &args.manifest_path);
    let results_path = absolutize(&args.repo_root, &args.results_path);

    let inventory: DependentsInventory = read_json(&dependents_path)?;
    let manifest: DependentRegressionManifest = read_json(&manifest_path)?;
    let results: DependentMatrixResults = read_json(&results_path)?;

    if manifest.schema_version != 1 {
        bail!(
            "{} uses unsupported schema_version {}",
            manifest_path.display(),
            manifest.schema_version
        );
    }
    if manifest.phase_id != PHASE_10_ID {
        bail!(
            "{} has unexpected phase_id {}",
            manifest_path.display(),
            manifest.phase_id
        );
    }
    if results.schema_version != 1 {
        bail!(
            "{} uses unsupported schema_version {}",
            results_path.display(),
            results.schema_version
        );
    }
    if results.phase_id != PHASE_10_ID {
        bail!(
            "{} has unexpected phase_id {}",
            results_path.display(),
            results.phase_id
        );
    }

    let mut expected = BTreeMap::new();
    for dependent in inventory.dependents {
        if dependent.slug.trim().is_empty() {
            bail!(
                "{} contains an empty dependent slug",
                dependents_path.display()
            );
        }
        let previous = expected.insert(dependent.slug.clone(), dependent.name);
        if previous.is_some() {
            bail!(
                "{} contains duplicate dependent slug {}",
                dependents_path.display(),
                dependent.slug
            );
        }
    }
    if expected.len() != 12 {
        bail!(
            "{} must remain authoritative for 12 dependent checks, found {}",
            dependents_path.display(),
            expected.len()
        );
    }

    validate_results_inventory(&results_path, &results, &expected)?;
    validate_results_summary(&results_path, &results)?;

    let mut issues_by_slug = BTreeMap::new();
    for issue in manifest.issues {
        if issue.summary.trim().is_empty() {
            bail!("manifest issue {} is missing a summary", issue.slug);
        }
        let expected_name = expected.get(&issue.slug).ok_or_else(|| {
            anyhow!(
                "manifest issue {} is not present in dependents.json",
                issue.slug
            )
        })?;
        if &issue.dependent_name != expected_name {
            bail!(
                "manifest issue {} names {} but dependents.json expects {}",
                issue.slug,
                issue.dependent_name,
                expected_name
            );
        }
        if !matches!(issue.status.as_str(), "open" | "fixed") {
            bail!(
                "manifest issue {} has unsupported status {}",
                issue.slug,
                issue.status
            );
        }
        if issue.status == "fixed" && issue.fix_commit.trim().is_empty() {
            bail!(
                "manifest issue {} is fixed but fix_commit is empty",
                issue.slug
            );
        }
        validate_reproducer(&args.repo_root, &issue)?;
        let previous = issues_by_slug.insert(issue.slug.clone(), issue);
        if previous.is_some() {
            bail!(
                "manifest contains duplicate issue slug {}",
                previous.unwrap().slug
            );
        }
    }

    for result in &results.dependents {
        match result.status.as_str() {
            "passed" => {
                if let Some(issue) = issues_by_slug.get(&result.slug) {
                    if issue.status == "open" {
                        bail!(
                            "dependent {} passed but manifest issue remains open",
                            result.slug
                        );
                    }
                }
            }
            "failed" => {
                let issue = issues_by_slug.get(&result.slug).ok_or_else(|| {
                    anyhow!(
                        "dependent {} failed in {} but has no entry in {}",
                        result.slug,
                        results_path.display(),
                        manifest_path.display()
                    )
                })?;
                if issue.status == "fixed" {
                    bail!(
                        "dependent {} is marked fixed in {} but still failed in {}",
                        result.slug,
                        manifest_path.display(),
                        results_path.display()
                    );
                }
            }
            other => bail!(
                "dependent {} has unsupported status {} in {}",
                result.slug,
                other,
                results_path.display()
            ),
        }
    }

    Ok(())
}

fn validate_results_inventory(
    results_path: &Path,
    results: &DependentMatrixResults,
    expected: &BTreeMap<String, String>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for result in &results.dependents {
        let expected_name = expected.get(&result.slug).ok_or_else(|| {
            anyhow!(
                "{} contains unexpected dependent slug {}",
                results_path.display(),
                result.slug
            )
        })?;
        if &result.name != expected_name {
            bail!(
                "{} recorded {} for {} but dependents.json expects {}",
                results_path.display(),
                result.name,
                result.slug,
                expected_name
            );
        }
        if !seen.insert(result.slug.clone()) {
            bail!(
                "{} contains duplicate dependent slug {}",
                results_path.display(),
                result.slug
            );
        }
    }

    if results.only_filter.is_none() {
        let missing = expected
            .keys()
            .filter(|slug| !seen.contains(*slug))
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            bail!(
                "{} is missing dependent results for {:?}",
                results_path.display(),
                missing
            );
        }
        let extra = seen
            .iter()
            .filter(|slug| !expected.contains_key(*slug))
            .cloned()
            .collect::<Vec<_>>();
        if !extra.is_empty() {
            bail!(
                "{} contains unexpected dependent results {:?}",
                results_path.display(),
                extra
            );
        }
    }

    Ok(())
}

fn validate_results_summary(results_path: &Path, results: &DependentMatrixResults) -> Result<()> {
    let passed = results
        .dependents
        .iter()
        .filter(|entry| entry.status == "passed")
        .count();
    let failed = results
        .dependents
        .iter()
        .filter(|entry| entry.status == "failed")
        .count();
    if results.summary.total != results.dependents.len()
        || results.summary.passed != passed
        || results.summary.failed != failed
    {
        bail!(
            "{} summary mismatch: total={}, passed={}, failed={} but entries imply total={}, passed={}, failed={}",
            results_path.display(),
            results.summary.total,
            results.summary.passed,
            results.summary.failed,
            results.dependents.len(),
            passed,
            failed
        );
    }
    Ok(())
}

fn validate_reproducer(repo_root: &Path, issue: &DependentRegressionIssue) -> Result<()> {
    if issue.reproducer.path.trim().is_empty() {
        bail!("manifest issue {} is missing reproducer.path", issue.slug);
    }
    let path = absolutize(repo_root, Path::new(&issue.reproducer.path));
    if !path.exists() {
        bail!(
            "manifest issue {} references missing reproducer path {}",
            issue.slug,
            path.display()
        );
    }
    if !issue.reproducer.test_name.trim().is_empty() {
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("read reproducer source {}", path.display()))?;
        let needle = format!("fn {}", issue.reproducer.test_name);
        if !contents.contains(&needle) {
            bail!(
                "manifest issue {} expects test {} in {}",
                issue.slug,
                issue.reproducer.test_name,
                path.display()
            );
        }
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    serde_json::from_slice(&fs::read(path).with_context(|| format!("read {}", path.display()))?)
        .with_context(|| format!("parse {}", path.display()))
}

fn absolutize(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        verify_dependent_regressions, DependentMatrixEntry, DependentMatrixResults,
        DependentMatrixSummary, DependentRegressionIssue, DependentRegressionManifest,
        DependentReproducer, VerifyDependentRegressionsArgs,
    };
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn verify_dependent_regressions_accepts_fixed_passed_issue() {
        let repo = tempdir().expect("repo tempdir");
        fs::write(
            repo.path().join("dependents.json"),
            r#"{"dependents":[
{"slug":"a","name":"A"},{"slug":"b","name":"B"},{"slug":"c","name":"C"},{"slug":"d","name":"D"},
{"slug":"e","name":"E"},{"slug":"f","name":"F"},{"slug":"g","name":"G"},{"slug":"h","name":"H"},
{"slug":"i","name":"I"},{"slug":"j","name":"J"},{"slug":"k","name":"K"},{"slug":"l","name":"L"}]}"#,
        )
        .expect("write dependents");
        let tests_dir = repo.path().join("safe/tests");
        fs::create_dir_all(&tests_dir).expect("create tests dir");
        fs::write(
            tests_dir.join("dependent_regressions.rs"),
            "fn fixed_issue_reproducer() {}\n",
        )
        .expect("write reproducer");
        fs::write(
            repo.path().join("manifest.json"),
            serde_json::to_vec_pretty(&DependentRegressionManifest {
                schema_version: 1,
                phase_id: "impl_phase_10_packaging_dependents_final".to_string(),
                issues: vec![DependentRegressionIssue {
                    slug: "a".to_string(),
                    dependent_name: "A".to_string(),
                    status: "fixed".to_string(),
                    summary: "resolved".to_string(),
                    reproducer: DependentReproducer {
                        path: "safe/tests/dependent_regressions.rs".to_string(),
                        test_name: "fixed_issue_reproducer".to_string(),
                        command: String::new(),
                    },
                    fix_commit: "abc123".to_string(),
                    notes: Vec::new(),
                }],
            })
            .expect("serialize manifest"),
        )
        .expect("write manifest");
        fs::write(
            repo.path().join("results.json"),
            serde_json::to_vec_pretty(&DependentMatrixResults {
                schema_version: 1,
                phase_id: "impl_phase_10_packaging_dependents_final".to_string(),
                only_filter: None,
                dependents: vec![
                    DependentMatrixEntry {
                        slug: "a".to_string(),
                        name: "A".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "b".to_string(),
                        name: "B".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "c".to_string(),
                        name: "C".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "d".to_string(),
                        name: "D".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "e".to_string(),
                        name: "E".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "f".to_string(),
                        name: "F".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "g".to_string(),
                        name: "G".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "h".to_string(),
                        name: "H".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "i".to_string(),
                        name: "I".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "j".to_string(),
                        name: "J".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "k".to_string(),
                        name: "K".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                    DependentMatrixEntry {
                        slug: "l".to_string(),
                        name: "L".to_string(),
                        status: "passed".to_string(),
                        duration_seconds: 0.1,
                        log_path: None,
                        json_path: None,
                        artifact_dir: None,
                        notes: Vec::new(),
                    },
                ],
                summary: DependentMatrixSummary {
                    total: 12,
                    passed: 12,
                    failed: 0,
                },
            })
            .expect("serialize results"),
        )
        .expect("write results");

        verify_dependent_regressions(VerifyDependentRegressionsArgs {
            repo_root: repo.path().to_path_buf(),
            dependents_path: "dependents.json".into(),
            manifest_path: "manifest.json".into(),
            results_path: "results.json".into(),
        })
        .expect("verify dependents");
    }
}
