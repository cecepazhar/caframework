use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModuleManifestFile {
    pub module: ModuleDefinition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModuleDefinition {
    pub key: String,
    pub label_en: String,
    pub label_id: String,
    pub cargo_feature: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    #[serde(default)]
    pub features: Vec<ModuleFeatureConfig>,
    #[serde(default)]
    pub provides: Option<ModuleProvidesConfig>,
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModuleFeatureConfig {
    pub key: String,
    #[serde(default = "default_feature_default")]
    pub default: bool,
    #[serde(default)]
    pub locked_when: Option<String>,
}

fn default_feature_default() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModuleProvidesConfig {
    #[serde(default)]
    pub menus: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub entities: Vec<String>,
    #[serde(default)]
    pub jobs: Vec<String>,
    #[serde(default)]
    pub integrations: Vec<String>,
    #[serde(default)]
    pub settings_sections: Vec<String>,
    #[serde(default)]
    pub ai_context_providers: Vec<String>,
}

use crate::error::ValidationError;
use crate::validation::{find_location, safe_prefix};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static RE_MOD_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_]{1,31}$").expect("valid regex"));

pub fn validate_module_manifest(source: &str) -> Result<ModuleManifestFile, Vec<ValidationError>> {
    let mut errors = Vec::new();
    let manifest: Result<ModuleManifestFile, toml::de::Error> = toml::from_str(source);
    let manifest = match manifest {
        Ok(m) => m,
        Err(e) => {
            let mut err = ValidationError::new("module.toml", e.message());
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

    if !RE_MOD_KEY.is_match(&manifest.module.key) {
        let (l, c) = find_location(source, &format!("key = \"{}\"", manifest.module.key));
        errors.push(
            ValidationError::new(
                "module.key",
                format!(
                    "module key '{}' must match ^[a-z][a-z0-9_]{{1,31}}$",
                    manifest.module.key
                ),
            )
            .with_location(l, c),
        );
    }

    if !manifest.module.cargo_feature.starts_with("mod-") {
        let (l, c) = find_location(source, &manifest.module.cargo_feature);
        errors.push(
            ValidationError::new(
                "module.cargo_feature",
                format!(
                    "cargo_feature '{}' must start with 'mod-'",
                    manifest.module.cargo_feature
                ),
            )
            .with_location(l, c),
        );
    }

    if errors.is_empty() {
        Ok(manifest)
    } else {
        Err(errors)
    }
}

pub fn validate_module_graph(manifests: &[ModuleManifestFile]) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    let mut keys = HashSet::new();
    let mut key_map = HashMap::new();

    for m in manifests {
        if !keys.insert(m.module.key.clone()) {
            errors.push(ValidationError::new(
                "module.key",
                format!("duplicate module key '{}'", m.module.key),
            ));
        }
        key_map.insert(m.module.key.clone(), m);
    }

    for m in manifests {
        for dep in &m.module.depends_on {
            if !key_map.contains_key(dep) {
                errors.push(ValidationError::new(
                    format!("module.{}.depends_on", m.module.key),
                    format!(
                        "module '{}' depends on unknown module '{}'",
                        m.module.key, dep
                    ),
                ));
            }
        }
        for conf in &m.module.conflicts_with {
            if key_map.contains_key(conf) {
                errors.push(ValidationError::new(
                    format!("module.{}.conflicts_with", m.module.key),
                    format!(
                        "module '{}' conflicts with enabled module '{}'",
                        m.module.key, conf
                    ),
                ));
            }
        }
    }

    // Cycle detection using DFS
    let mut visited = HashMap::new();
    fn has_cycle(
        node: &str,
        key_map: &HashMap<String, &ModuleManifestFile>,
        visited: &mut HashMap<String, u8>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string(), 0);
        path.push(node.to_string());
        if let Some(m) = key_map.get(node) {
            for dep in &m.module.depends_on {
                if let Some(&state) = visited.get(dep) {
                    if state == 0 {
                        path.push(dep.to_string());
                        return Some(path.clone());
                    }
                } else if let Some(cycle) = has_cycle(dep, key_map, visited, path) {
                    return Some(cycle);
                }
            }
        }
        path.pop();
        visited.insert(node.to_string(), 1);
        None
    }

    for key in key_map.keys() {
        if !visited.contains_key(key) {
            let mut path = Vec::new();
            if let Some(cycle) = has_cycle(key, &key_map, &mut visited, &mut path) {
                errors.push(ValidationError::new(
                    "module.graph",
                    format!("cyclic module dependency detected: {}", cycle.join(" -> ")),
                ));
                break;
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
