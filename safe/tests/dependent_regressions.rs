use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[path = "common/testutils.rs"]
mod testutils;

use safe_sdl::abi::generated_types::SDL_GLattr_SDL_GL_CONTEXT_MAJOR_VERSION;
use safe_sdl::render::gl::{SDL_GL_GetAttribute, SDL_GL_ResetAttributes, SDL_GL_SetAttribute};

#[derive(Debug, Deserialize)]
struct DependentRegressionManifest {
    schema_version: u32,
    phase_id: String,
    #[serde(default)]
    issues: Vec<DependentRegressionIssue>,
}

#[derive(Debug, Deserialize)]
struct DependentRegressionIssue {
    slug: String,
    dependent_name: String,
    status: String,
    summary: String,
    reproducer: DependentReproducer,
    #[serde(default)]
    fix_commit: String,
}

#[derive(Debug, Deserialize)]
struct DependentReproducer {
    path: String,
    #[serde(default)]
    test_name: String,
    #[serde(default)]
    command: String,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn manifest_path() -> PathBuf {
    repo_root().join("generated/dependent_regression_manifest.json")
}

#[test]
fn manifest_entries_are_bound_to_real_tests() {
    let manifest: DependentRegressionManifest = serde_json::from_slice(
        &fs::read(manifest_path()).expect("read dependent regression manifest"),
    )
    .expect("parse dependent regression manifest");

    assert_eq!(manifest.schema_version, 1);
    assert_eq!(
        manifest.phase_id,
        "impl_phase_10_packaging_dependents_final"
    );

    for issue in manifest.issues {
        assert!(
            !issue.slug.trim().is_empty(),
            "issue slug must not be empty"
        );
        assert!(
            !issue.dependent_name.trim().is_empty(),
            "issue {} must name the dependent",
            issue.slug
        );
        assert!(
            !issue.summary.trim().is_empty(),
            "issue {} needs a summary",
            issue.slug
        );
        assert!(
            matches!(issue.status.as_str(), "open" | "fixed"),
            "issue {} has unsupported status {}",
            issue.slug,
            issue.status
        );
        if issue.status == "fixed" {
            assert!(
                !issue.fix_commit.trim().is_empty(),
                "issue {} is fixed but fix_commit is empty",
                issue.slug
            );
        }
        assert!(
            !issue.reproducer.path.trim().is_empty(),
            "issue {} needs a reproducer path",
            issue.slug
        );
        let reproducer_path = repo_root()
            .parent()
            .expect("workspace root")
            .join(&issue.reproducer.path);
        assert!(
            reproducer_path.exists(),
            "issue {} reproducer path {} does not exist",
            issue.slug,
            reproducer_path.display()
        );
        if !issue.reproducer.test_name.trim().is_empty() {
            let contents =
                fs::read_to_string(&reproducer_path).expect("read reproducer source file");
            assert!(
                contents.contains(&format!("fn {}", issue.reproducer.test_name)),
                "issue {} reproducer test {} missing from {}",
                issue.slug,
                issue.reproducer.test_name,
                reproducer_path.display()
            );
        }
        if !issue.reproducer.command.trim().is_empty() {
            assert!(
                issue.reproducer.command.contains("cargo test")
                    || issue.reproducer.command.contains("./test-original.sh"),
                "issue {} reproducer command should remain reviewable",
                issue.slug
            );
        }
    }
}

#[test]
fn gl_attributes_do_not_require_a_host_runtime() {
    unsafe {
        SDL_GL_ResetAttributes();
        assert_eq!(
            SDL_GL_SetAttribute(SDL_GLattr_SDL_GL_CONTEXT_MAJOR_VERSION, 3),
            0,
            "{}",
            testutils::current_error()
        );

        let mut major_version = -1;
        assert_eq!(
            SDL_GL_GetAttribute(SDL_GLattr_SDL_GL_CONTEXT_MAJOR_VERSION, &mut major_version,),
            0,
            "{}",
            testutils::current_error()
        );
        assert_eq!(major_version, 3);
    }
}
