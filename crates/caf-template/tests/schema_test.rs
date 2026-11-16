use caf_template::config::AppConfigFile;
use caf_template::schema::generate_app_config_schema;

#[test]
fn test_schema_generation_and_validation() {
    let schema_json = generate_app_config_schema();
    let schema_val: serde_json::Value =
        serde_json::from_str(&schema_json).expect("generated schema must be valid JSON");

    assert_eq!(schema_val["title"], "AppConfigFile");
    // Ensure deny_unknown_fields causes additionalProperties to be false
    assert_eq!(schema_val["additionalProperties"], false);

    // Verify FR-1 fixture serializes into JSON matching schema expectations
    let fr1_fixture = include_str!("fixtures/prd_fr1_example.toml");
    let config: AppConfigFile = toml::from_str(fr1_fixture).expect("valid toml");
    let config_json = serde_json::to_value(&config).expect("valid json value");
    assert!(config_json.is_object());
    assert_eq!(config_json["app"]["slug"], "myapp");
}
