use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VariantConfigFile {
    pub variant: VariantHeader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VariantHeader {
    pub key: String,
    pub extends: String,
    #[serde(default)]
    pub app: Option<VariantAppOverride>,
    #[serde(default)]
    pub modules: Option<VariantModulesOverride>,
    #[serde(default)]
    pub defaults: Option<BTreeMap<String, bool>>,
    #[serde(default)]
    pub locked: Option<VariantLockedOverride>,
    #[serde(default)]
    pub pro: Option<VariantProOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VariantAppOverride {
    pub name: String,
    pub identifier: String,
    pub accent: String,
    pub platforms: Vec<String>,
    pub icon: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VariantModulesOverride {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub framework: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VariantLockedOverride {
    #[serde(default)]
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VariantProOverride {
    #[serde(default)]
    pub gcc_product_id: Option<String>,
    #[serde(default)]
    pub features: Option<BTreeMap<String, String>>,
}

use crate::error::ValidationError;
use crate::validation::{find_location, safe_prefix};
use regex::Regex;
use std::sync::LazyLock;

static RE_VAR_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]{1,30}$").expect("valid regex"));

pub fn validate_variant_config(source: &str) -> Result<VariantConfigFile, Vec<ValidationError>> {
    let mut errors = Vec::new();
    let config: Result<VariantConfigFile, toml::de::Error> = toml::from_str(source);
    let config = match config {
        Ok(c) => c,
        Err(e) => {
            let mut err = ValidationError::new("variant.toml", e.message());
            if let Some(span) = e.span() {
                let prefix = safe_prefix(source, span.start);
                let line = prefix.matches('\n').count() + 1;
                let col = prefix
                    .rfind('\n')
                    .map_or(prefix.len() + 1, |nl| prefix.len() - nl);
                err.line = Some(line);
                err.column = Some(col);
            }
            errors.push(err);
            return Err(errors);
        }
    };

    if !RE_VAR_KEY.is_match(&config.variant.key) {
        let (l, c) = find_location(source, &format!("key = \"{}\"", config.variant.key));
        errors.push(
            ValidationError::new(
                "variant.key",
                format!(
                    "variant key '{}' must match ^[a-z][a-z0-9-]{{1,30}}$",
                    config.variant.key
                ),
            )
            .with_location(l, c),
        );
    }

    if config.variant.extends.trim().is_empty() {
        let (l, c) = find_location(source, "extends =");
        errors.push(
            ValidationError::new("variant.extends", "extends must not be empty")
                .with_location(l, c),
        );
    }

    if errors.is_empty() {
        Ok(config)
    } else {
        Err(errors)
    }
}
