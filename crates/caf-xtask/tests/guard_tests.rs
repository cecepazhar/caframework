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
fn test_guard_passes_on_repo() {
    let root = get_workspace_root();
    let output = Command::new(bin_path())
        .arg("guard")
        .current_dir(&root)
        .output()
        .expect("Failed to run caf-xtask guard");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(
        output.status.success(),
        "guard should exit 0 on repo baseline"
    );
    assert!(stdout.contains("All guards passed."));
}

#[test]
fn test_guard_fails_on_bad_fixtures() {
    let root = get_workspace_root();

    // 1. Check absolute_paths guard
    let output = Command::new(bin_path())
        .args(["guard", "--only", "absolute_paths"])
        .current_dir(&root)
        .output()
        .expect("Failed to run caf-xtask guard");
    assert!(output.status.success());

    // 2. Check invoke_registry guard
    let output = Command::new(bin_path())
        .args(["guard", "--only", "invoke_registry"])
        .current_dir(&root)
        .output()
        .expect("Failed to run caf-xtask guard");
    assert!(output.status.success());

    // 3. Check raw_sync_writes guard
    let output = Command::new(bin_path())
        .args(["guard", "--only", "raw_sync_writes"])
        .current_dir(&root)
        .output()
        .expect("Failed to run caf-xtask guard");
    assert!(output.status.success());

    // 4. Check genericity guard
    let output = Command::new(bin_path())
        .args(["guard", "--only", "genericity"])
        .current_dir(&root)
        .output()
        .expect("Failed to run caf-xtask guard");
    assert!(output.status.success());
}

#[test]
fn test_repository_hygiene_fails_on_bad_fixtures() {
    let temp_root = std::env::temp_dir().join(format!("caf_hygiene_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("create temp fixture dir");

    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(temp_root.clone());

    // 1. Create a 2 MB file (oversized)
    let large_file = temp_root.join("huge_blob.bin");
    let two_mb = vec![1u8; 2 * 1024 * 1024];
    std::fs::write(&large_file, two_mb).expect("write large file");

    // 2. Create a dummy vault.key file (forbidden key extension)
    let dummy_key = temp_root.join("vault.key");
    std::fs::write(&dummy_key, "0123456789abcdef0123456789abcdef").expect("write dummy key");

    // 3. Create a dummy task.md (internal plan inside repository)
    let plan_file = temp_root.join("task.md");
    std::fs::write(&plan_file, "# Plan inside repo").expect("write plan file");

    let output = Command::new(bin_path())
        .args(["guard", "--only", "repository_hygiene"])
        .env("CAF_GUARD_ROOT", &temp_root)
        .current_dir(&temp_root)
        .output()
        .expect("Failed to run caf-xtask guard");

    assert!(
        !output.status.success(),
        "Guard must exit non-zero when fixture tree has 2MB file and dummy vault.key"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Failing fixture output:\n{}", stderr);

    assert!(
        stderr.contains("Tracked file is over 1MB") || stderr.contains("1048576"),
        "Must flag >1MB file in fixture: {stderr}"
    );
    assert!(
        stderr.contains("forbidden key"),
        "Must flag vault.key in fixture: {stderr}"
    );
    assert!(
        stderr.contains("internal notes/plans"),
        "Must flag task.md in fixture: {stderr}"
    );
}

#[test]
fn test_ledger_guard_fails_on_tamper() {
    let temp_root =
        std::env::temp_dir().join(format!("caf_ledger_guard_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("create temp fixture dir");

    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(temp_root.clone());

    // 1. Valid task.md
    let task_content = "\
# Task Plan
**P00.1 Original Title**
**P00.2 Second Task**
";
    std::fs::write(temp_root.join("task.md"), task_content).expect("write task.md");

    // 2. Tampered ledger-v3.md (P00.1 renamed, P00.2 marked DONE without evidence)
    let bad_ledger = "\
# Ledger
| Task | Title | Status | Evidence anchor |
|---|---|---|---|
| P00.1 | Renamed Fake Title | DONE | |
| P00.2 | Second Task | DONE | P00.md#p002 |
";
    std::fs::write(temp_root.join("ledger-v3.md"), bad_ledger).expect("write bad ledger");

    let output = Command::new(bin_path())
        .args(["guard", "--only", "ledger"])
        .env("CAF_PLAN_DIR", &temp_root)
        .output()
        .expect("Failed to run caf-xtask guard");

    assert!(
        !output.status.success(),
        "Ledger guard must fail on renamed task or missing evidence file"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Ledger failing fixture output:\n{}", stderr);

    assert!(
        stderr.contains("title mismatch"),
        "Must detect title mismatch: {stderr}"
    );
    assert!(
        stderr.contains("without evidence anchor") || stderr.contains("does not exist"),
        "Must detect missing evidence anchor/file: {stderr}"
    );
}

#[test]
fn test_invoke_guard_fails_on_fixture() {
    let temp_root =
        std::env::temp_dir().join(format!("caf_invoke_guard_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_root);
    let fe_src = temp_root.join("frontend/src");
    std::fs::create_dir_all(&fe_src).expect("create temp fixture dir");

    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(temp_root.clone());

    // Create a non-generated file that calls invoke()
    let bad_file = fe_src.join("BadComponent.svelte");
    std::fs::write(&bad_file, "<script>\n  invoke('bad_command');\n</script>")
        .expect("write bad fixture");

    let output = Command::new(bin_path())
        .args(["guard", "--only", "invoke_registry"])
        .env("CAF_GUARD_ROOT", &temp_root)
        .output()
        .expect("Failed to run caf-xtask guard");

    assert!(
        !output.status.success(),
        "Guard must fail when invoke() is called outside generated bindings"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Invoke guard fixture output:\n{}", stderr);

    assert!(
        stderr.contains("invoke() is only allowed in generated bindings"),
        "Must flag forbidden invoke() call in non-generated file: {stderr}"
    );
}
