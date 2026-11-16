use crate::config::AppConfigFile;
use schemars::schema_for;

pub fn generate_app_config_schema() -> String {
    let schema = schema_for!(AppConfigFile);
    serde_json::to_string_pretty(&schema).expect("schema must serialize to JSON")
}
