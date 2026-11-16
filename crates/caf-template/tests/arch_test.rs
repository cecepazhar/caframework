use std::process::Command;

#[test]
fn test_caf_template_has_no_gui_dependencies() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "caf-template", "--prefix", "none"])
        .output()
        .expect("cargo tree must succeed");

    assert!(output.status.success(), "cargo tree failed: {:?}", output);
    let tree = String::from_utf8_lossy(&output.stdout);

    let forbidden_gui_crates = [
        "tauri",
        "gtk",
        "webkit2gtk",
        "tao",
        "wry",
        "gdk",
        "glib-sys",
        "cairo-rs",
        "pango",
        "caf-app",
    ];

    for pkg in tree.lines() {
        let pkg_name = pkg.split_whitespace().next().unwrap_or("").trim();
        for forbidden in forbidden_gui_crates {
            assert!(
                pkg_name != forbidden && !pkg_name.starts_with(&format!("{}-", forbidden)),
                "Architecture violation: caf-template depends on GUI/Tauri crate '{}' (found '{}')",
                forbidden,
                pkg_name
            );
        }
    }
}
