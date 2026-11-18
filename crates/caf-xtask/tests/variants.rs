use caf_xtask::variant_cmd::{VariantCommand, run_variant_cmd};
use std::path::Path;
use std::process::Command;

#[test]
fn test_variant_validate_all() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root");

    let res = run_variant_cmd(
        &VariantCommand::Validate {
            variant: None,
            all: true,
        },
        repo_root,
    );
    assert!(res.is_ok(), "validate --all should succeed, got: {:?}", res);
}

#[test]
fn test_variant_list_and_diff() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root");

    let list_res = run_variant_cmd(&VariantCommand::List, repo_root);
    assert!(list_res.is_ok(), "list command should succeed");

    let diff_res = run_variant_cmd(
        &VariantCommand::Diff {
            a: "lite".into(),
            b: "pro".into(),
        },
        repo_root,
    );
    assert!(diff_res.is_ok(), "diff command should succeed");
}

#[test]
fn test_variant_matrix() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root");

    let matrix_res = run_variant_cmd(
        &VariantCommand::Matrix {
            format: "md".into(),
        },
        repo_root,
    );
    assert!(matrix_res.is_ok(), "matrix command should succeed");
}

#[test]
fn test_variant_build_leaves_git_status_clean() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root");

    // Build lite variant
    let res_lite = run_variant_cmd(
        &VariantCommand::Build {
            variant: "lite".into(),
            target: "linux".into(),
            release: false,
        },
        repo_root,
    );
    assert!(
        res_lite.is_ok(),
        "variant build lite should succeed: {:?}",
        res_lite
    );

    // Build pro variant
    let res_pro = run_variant_cmd(
        &VariantCommand::Build {
            variant: "pro".into(),
            target: "linux".into(),
            release: false,
        },
        repo_root,
    );
    assert!(
        res_pro.is_ok(),
        "variant build pro should succeed: {:?}",
        res_pro
    );

    // Verify git status is clean on generated directories
    let diff_core = Command::new("git")
        .args(["diff", "--quiet", "crates/caf-core/src/generated"])
        .current_dir(repo_root)
        .status()
        .expect("git diff status");
    assert!(
        diff_core.success(),
        "crates/caf-core/src/generated must have zero unstaged diff after variant build"
    );

    let diff_frontend = Command::new("git")
        .args(["diff", "--quiet", "frontend/src/lib/generated"])
        .current_dir(repo_root)
        .status()
        .expect("git diff status");
    assert!(
        diff_frontend.success(),
        "frontend/src/lib/generated must have zero unstaged diff after variant build"
    );
}
