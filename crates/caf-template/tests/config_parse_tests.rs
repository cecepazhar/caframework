use caf_template::config::{AppConfigFile, NavigationSection};
use caf_template::module::{ModuleManifestFile, validate_module_manifest};
use caf_template::validation::validate_app_toml;
use caf_template::variant::{VariantConfigFile, validate_variant_config};

#[test]
fn test_prd_fr1_example_parses_byte_for_byte() {
    let fixture_str = include_str!("fixtures/prd_fr1_example.toml");
    let config: AppConfigFile =
        toml::from_str(fixture_str).expect("FR-1 example must parse into AppConfigFile");

    assert_eq!(config.app.name, "My App");
    assert_eq!(config.app.slug, "myapp");
    assert_eq!(config.app.code, "MYA");
    assert_eq!(config.app.identifier, "com.example.myapp");
    assert_eq!(config.app.accent, "indigo");
    assert_eq!(config.brand.developer, "Cecep Saeful Azhar");
    assert_eq!(config.rbac.super_role, "super_admin");
    assert_eq!(config.menus.len(), 1);
    assert_eq!(config.menus[0].key, "notes");

    // Also run validation
    let validated =
        validate_app_toml(fixture_str, None).expect("FR-1 fixture must validate successfully");
    assert_eq!(validated.app.slug, "myapp");
}

#[test]
fn test_a04_navigation_parses() {
    #[derive(serde::Deserialize)]
    struct NavigationWrapper {
        navigation: NavigationSection,
    }
    let fixture_str = include_str!("fixtures/a04_navigation_example.toml");
    let nav: NavigationWrapper = toml::from_str(fixture_str).expect("A-04 navigation must parse");
    assert_eq!(nav.navigation.primary_action, Some("orders".into()));
    assert_eq!(nav.navigation.collapsed_default, Some(false));
}

#[test]
fn test_a01_module_manifest_parses() {
    let fixture_str = include_str!("fixtures/a01_module_example.toml");
    let manifest: ModuleManifestFile =
        validate_module_manifest(fixture_str).expect("A-01 module manifest must validate");
    assert_eq!(manifest.module.key, "orders");
    assert_eq!(manifest.module.cargo_feature, "mod-orders");
    assert!(
        manifest.module.extra.is_some(),
        "module.extra must be preserved"
    );
}

#[test]
fn test_a02_variant_config_parses() {
    let fixture_str = include_str!("fixtures/a02_variant_example.toml");
    let variant: VariantConfigFile =
        validate_variant_config(fixture_str).expect("A-02 variant config must validate");
    assert_eq!(variant.variant.key, "lite");
    assert_eq!(variant.variant.extends, "app.toml");
    let app = variant.variant.app.expect("app override present");
    assert_eq!(app.name, "Example Lite");
}
