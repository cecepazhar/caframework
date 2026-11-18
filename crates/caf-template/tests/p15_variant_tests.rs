use caf_template::variant::{diff_variants, generate_variant_matrix, resolve_variant};
use std::path::Path;

#[test]
fn test_p15_1_variant_resolution_valid() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root");

    let fixture_str = include_str!("fixtures/a02_variant_example.toml");
    let resolved = resolve_variant(fixture_str, None, repo_root)
        .expect("a02_variant_example must resolve cleanly");

    assert_eq!(resolved.key, "lite");
    assert_eq!(resolved.resolved_config.app.name, "Example Lite");
    assert_eq!(
        resolved.resolved_config.app.identifier,
        "com.example.app.lite"
    );
    assert_eq!(resolved.resolved_config.app.accent, "sky");
    assert_eq!(
        resolved.resolved_config.app.platforms,
        vec!["windows", "android"]
    );
    assert_eq!(
        resolved.resolved_config.brand.logo,
        "assets/variants/lite/icon.png"
    );

    // AI is turned off in variant, scheduler is turned on
    let mods = resolved.resolved_config.modules.expect("modules section");
    assert_eq!(mods.ai, Some(false));
    assert_eq!(mods.scheduler, Some(true));

    assert!(
        !resolved.cargo_features.iter().any(|f| f == "mod-ai"),
        "mod-ai should not be compiled in lite variant"
    );
    assert!(
        resolved.cargo_features.iter().any(|f| f == "mod-scheduler"),
        "mod-scheduler should be compiled"
    );

    // Defaults and locked
    assert_eq!(
        resolved.defaults.get("orders.reservation"),
        Some(&false),
        "orders.reservation default must be false"
    );
    assert_eq!(resolved.locked_keys, vec!["orders"]);
}

#[test]
fn test_p15_1_duplicate_identifier_fails_with_location() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root");

    let fixture_str = include_str!("fixtures/variant_duplicate_identifier.toml");
    let errs = resolve_variant(fixture_str, None, repo_root)
        .expect_err("duplicate identifier must be rejected");

    let dup_err = errs
        .iter()
        .find(|e| e.path == "variant.app.identifier")
        .expect("must contain variant.app.identifier error");

    assert!(
        dup_err.message.contains("must differ from base"),
        "expected error message about identifier difference, got: {}",
        dup_err.message
    );
    assert!(
        dup_err.line.is_some() && dup_err.column.is_some(),
        "must report line and column location: line={:?}, col={:?}",
        dup_err.line,
        dup_err.column
    );
}

#[test]
fn test_p15_1_unknown_module_fails_with_location() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root");

    let fixture_str = include_str!("fixtures/variant_unknown_module.toml");
    let errs = resolve_variant(fixture_str, None, repo_root)
        .expect_err("unknown module in include must be rejected");

    let mod_err = errs
        .iter()
        .find(|e| e.path == "variant.modules.include")
        .expect("must contain variant.modules.include error");

    assert!(
        mod_err
            .message
            .contains("unknown module 'non_existent_module_xyz'"),
        "expected error message about unknown module, got: {}",
        mod_err.message
    );
    assert!(
        mod_err.line.is_some() && mod_err.column.is_some(),
        "must report line and column location"
    );
}

#[test]
fn test_p15_1_invalid_override_fails_with_location() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root");

    let fixture_str = include_str!("fixtures/variant_invalid_override.toml");
    let errs = resolve_variant(fixture_str, None, repo_root)
        .expect_err("invalid override must be rejected");

    let fw_err = errs
        .iter()
        .find(|e| e.path == "variant.modules.framework")
        .expect("must contain variant.modules.framework error");

    assert!(
        fw_err
            .message
            .contains("unknown framework module 'unknown_framework_mod'"),
        "expected error message about unknown framework module, got: {}",
        fw_err.message
    );
    assert!(
        fw_err.line.is_some() && fw_err.column.is_some(),
        "must report line and column location"
    );

    let accent_err = errs
        .iter()
        .find(|e| e.path == "app.accent")
        .expect("must contain invalid accent error");
    assert!(
        accent_err.message.contains("invalid_rainbow_color")
            || accent_err.message.contains("accent"),
        "expected error message about accent, got: {}",
        accent_err.message
    );
}

#[test]
fn test_p15_1_diff_and_matrix_generation() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root");

    let fixture_str = include_str!("fixtures/a02_variant_example.toml");
    let v1 = resolve_variant(fixture_str, None, repo_root).expect("v1 resolve");

    // Second variant
    let v2_str = r#"
[variant]
key = "pro"
extends = "app.toml"

[variant.app]
name = "Example Pro"
identifier = "com.example.app.pro"
accent = "emerald"
platforms = ["linux", "windows", "macos"]
icon = "assets/variants/pro/icon.png"

[variant.modules]
framework = { ai = true, scheduler = true }

[variant.pro]
gcc_product_id = "gcc-prod-pro-2026"
"#;
    let v2 = resolve_variant(v2_str, None, repo_root).expect("v2 resolve");

    // Diff
    let diff = diff_variants(&v1, &v2);
    assert!(diff.contains("Variant Diff: 'lite' vs 'pro'"));
    assert!(diff.contains("App Name:"));
    assert!(diff.contains("Identifier:"));
    assert!(diff.contains("com.example.app.lite"));
    assert!(diff.contains("com.example.app.pro"));
    assert!(diff.contains("GCC Product ID:"));

    // Matrix (markdown)
    let matrix_md = generate_variant_matrix(&[v1.clone(), v2.clone()], "md");
    assert!(matrix_md.contains("| Module | Kind | lite | pro |"));
    assert!(matrix_md.contains("| profiles | framework | ✓ | ✓ |"));
    assert!(matrix_md.contains("| ai | framework | - | ✓ |"));

    // Matrix (text)
    let matrix_txt = generate_variant_matrix(&[v1, v2], "text");
    assert!(matrix_txt.contains("MODULE MATRIX"));
    assert!(matrix_txt.contains("ai"));
}
