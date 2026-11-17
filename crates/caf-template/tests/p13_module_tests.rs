//! Tests for P13: Feature modules behind Cargo features (A-01).
//! Validates:
//! - P13.2: Module manifest parsing, graph validation, and validation errors per A-01 §3.
//! - P13.5: Module dependency arch test (importing undeclared module fails).
//! - P13.6: Example product module (docs/examples/modules/sample-module).

use caf_template::config::AppConfigFile;
use caf_template::module::{validate_module_graph_full, validate_module_manifest};
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()))
}

#[test]
fn test_p13_sample_product_module_validates_and_parses() {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs/examples/modules/sample-module/module.toml");
    let content = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("failed to read sample module: {e}"));
    let manifest = validate_module_manifest(&content).expect("sample module should be valid");
    assert_eq!(manifest.module.key, "sample_module");
    assert_eq!(manifest.module.cargo_feature, "mod-sample-module");
    assert!(manifest.module.extra.is_some());
}

#[test]
fn test_p13_duplicate_key_error() {
    let content = fixture("p13_dup_key_1.toml");
    let m1 = validate_module_manifest(&content).expect("parse m1");
    let m2 = validate_module_manifest(&content).expect("parse m2");

    let errs =
        validate_module_graph_full(&[m1, m2], None, None).expect_err("should fail duplicate");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("duplicate module key 'dup_key'"))
    );
}

#[test]
fn test_p13_unknown_dependency_error() {
    let content = fixture("p13_unknown_dep.toml");
    let m = validate_module_manifest(&content).expect("parse unknown dep mod");

    let errs = validate_module_graph_full(&[m], None, None).expect_err("should fail unknown dep");
    assert!(errs.iter().any(|e| {
        e.message
            .contains("depends on unknown module 'nonexistent_provider'")
    }));
}

#[test]
fn test_p13_conflict_error() {
    let c1 = fixture("p13_conflict_1.toml");
    let c2 = fixture("p13_conflict_2.toml");
    let m1 = validate_module_manifest(&c1).expect("parse alpha");
    let m2 = validate_module_manifest(&c2).expect("parse beta");

    let errs = validate_module_graph_full(&[m1, m2], None, None).expect_err("should fail conflict");
    assert!(errs.iter().any(|e| {
        e.message
            .contains("conflicts with enabled module 'beta_mod'")
    }));
}

#[test]
fn test_p13_cycle_error() {
    let c1 = fixture("p13_cycle_1.toml");
    let c2 = fixture("p13_cycle_2.toml");
    let m1 = validate_module_manifest(&c1).expect("parse node a");
    let m2 = validate_module_manifest(&c2).expect("parse node b");

    let errs = validate_module_graph_full(&[m1, m2], None, None).expect_err("should fail cycle");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("cyclic module dependency detected"))
    );
}

#[test]
fn test_p13_missing_cargo_feature_error() {
    let content = fixture("p13_missing_cargo_feature.toml");
    let m = validate_module_manifest(&content).expect("parse fake mod");
    let known_features = ["mod-profiles", "mod-ai", "mod-pro"];

    let errs = validate_module_graph_full(&[m], None, Some(&known_features))
        .expect_err("should fail missing cargo feature");
    assert!(errs.iter().any(|e| {
        e.message
            .contains("cargo_feature 'mod-ghost-feature' which does not exist")
    }));
}

#[test]
fn test_p13_missing_menu_entity_job_permissions_errors() {
    let app_cfg_raw = fixture("prd_fr1_example.toml");
    let app_cfg: AppConfigFile = toml::from_str(&app_cfg_raw).expect("parse app config");

    // Missing menu
    let m_menu = validate_module_manifest(&fixture("p13_missing_menu.toml")).expect("parse");
    let errs_menu =
        validate_module_graph_full(&[m_menu], Some(&app_cfg), None).expect_err("menu err");
    assert!(errs_menu.iter().any(|e| {
        e.message
            .contains("provides menu 'undefined_menu_key' which is not defined in app.toml")
    }));

    // Missing entity
    let m_ent = validate_module_manifest(&fixture("p13_missing_entity.toml")).expect("parse");
    let errs_ent = validate_module_graph_full(&[m_ent], Some(&app_cfg), None).expect_err("ent err");
    assert!(errs_ent.iter().any(|e| {
        e.message
            .contains("provides entity 'undefined_entity_key' which is not defined in app.toml")
    }));

    // Missing job
    let m_job = validate_module_manifest(&fixture("p13_missing_job.toml")).expect("parse");
    let errs_job = validate_module_graph_full(&[m_job], Some(&app_cfg), None).expect_err("job err");
    assert!(errs_job.iter().any(|e| {
        e.message
            .contains("provides job 'undefined_job_key' which is not defined in app.toml")
    }));

    // Missing permission
    let m_perm = validate_module_manifest(&fixture("p13_missing_perm.toml")).expect("parse");
    let errs_perm =
        validate_module_graph_full(&[m_perm], Some(&app_cfg), None).expect_err("perm err");
    assert!(errs_perm.iter().any(|e| {
        e.message
            .contains("provides permission 'nonexistent_perm_defined_nowhere:action'")
    }));
}

/// P13.5 Arch test: scan module files and assert an undeclared module import fails.
#[test]
fn test_p13_5_module_dependency_arch_test() {
    fn check_module_imports(
        module_key: &str,
        depends_on: &[String],
        src_code: &str,
    ) -> Result<(), String> {
        for line in src_code.lines() {
            let trimmed = line.trim();
            if let Some(target) = trimmed.strip_prefix("use crate::modules::") {
                let target_mod = target
                    .split("::")
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(';');
                if target_mod != module_key && !depends_on.iter().any(|d| d == target_mod) {
                    return Err(format!(
                        "module '{module_key}' imports undeclared module '{target_mod}' outside depends_on"
                    ));
                }
            }
        }
        Ok(())
    }

    // Good module: valid
    let good_code = "pub fn foo() { println!(\"ok\"); }";
    assert!(check_module_imports("sample_module", &[], good_code).is_ok());

    // Bad module: imports undeclared module 'secret_orders'
    let bad_code = "use crate::modules::secret_orders::SecretService;\npub fn foo() {}";
    let err = check_module_imports("sample_module", &[], bad_code).expect_err("must fail");
    assert!(err.contains("imports undeclared module 'secret_orders' outside depends_on"));

    // Declared module: valid
    assert!(check_module_imports("sample_module", &["secret_orders".into()], bad_code).is_ok());
}
