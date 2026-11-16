use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn bin_path() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_caf-xtask") {
        return PathBuf::from(p);
    }
    let mut p = get_workspace_root();
    p.push("target");
    p.push("debug");
    p.push("caf-xtask");
    p
}

#[test]
fn test_guard_raw_sync_writes_runs() {
    let root = get_workspace_root();
    let output = Command::new(bin_path())
        .args(["guard", "--only", "raw_sync_writes"])
        .current_dir(&root)
        .output()
        .expect("Failed to run guard");
    assert!(output.status.success());
}

#[test]
fn test_guard_raw_sync_writes_fails_on_bad_fixture() {
    let temp_root =
        std::env::temp_dir().join(format!("caf_guard_fixture_{}", uuid::Uuid::new_v4()));
    let fake_core_src = temp_root.join("crates/caf-core/src");
    fs::create_dir_all(&fake_core_src).expect("create fake core src");

    // Create a fake Rust file with direct SQL write on a sync table outside sync.rs
    let bad_file = fake_core_src.join("bad_module.rs");
    fs::write(
        &bad_file,
        r#"
pub fn raw_write_forbidden() {
    let query = "INSERT INTO notes (id, title) VALUES ('n1', 'forbidden');";
}
"#,
    )
    .expect("write bad fixture file");

    let mut failures = Vec::new();
    let res = caf_xtask::scanner::check_raw_sync_writes(&temp_root, &mut failures);
    assert!(res.is_ok());
    assert!(
        !failures.is_empty(),
        "Guard must flag raw sync write outside sync.rs"
    );
    assert!(
        failures[0].contains("Direct SQL write on sync table outside sync.rs"),
        "Failure message must match expected guard message: {}",
        failures[0]
    );

    let _ = fs::remove_dir_all(&temp_root);
}
