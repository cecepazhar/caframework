use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::codegen::discover_modules;
use crate::config::AppConfigFile;
use crate::error::ValidationError;
use crate::module::{ModuleManifestFile, validate_module_graph_full};
use crate::validation::{find_location, safe_prefix, validate_app_toml};
use regex::Regex;

static RE_VAR_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]{1,30}$").expect("valid regex"));

pub const FRAMEWORK_MODULES: &[(&str, &str)] = &[
    ("profiles", "mod-profiles"),
    ("ai", "mod-ai"),
    ("pro", "mod-pro"),
    ("feedback", "mod-feedback"),
    ("updater", "mod-updater"),
    ("crash", "mod-crash"),
    ("integrations", "mod-integrations"),
    ("scheduler", "mod-scheduler"),
    ("support_page", "mod-support-page"),
    ("background", "mod-background"),
    ("builder", "builder"),
];

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

/// Fully resolved variant with base config merged, modules discovered and features computed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedVariant {
    pub key: String,
    pub variant_path: Option<PathBuf>,
    pub extends: String,
    pub base_config: AppConfigFile,
    pub resolved_config: AppConfigFile,
    pub variant_config: VariantConfigFile,
    pub enabled_modules: Vec<String>,
    pub cargo_features: Vec<String>,
    pub defaults: BTreeMap<String, bool>,
    pub locked_keys: Vec<String>,
}

/// Validates syntax and basic schema of a variant TOML file.
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

/// Resolves a variant TOML source on top of its base config (`extends`), validating overrides.
pub fn resolve_variant(
    source: &str,
    variant_path: Option<&Path>,
    project_root: &Path,
) -> Result<ResolvedVariant, Vec<ValidationError>> {
    let variant_file = validate_variant_config(source)?;
    let mut errors = Vec::new();

    // 1. Locate and read extends base config
    let extends_rel = &variant_file.variant.extends;
    let base_path = if let Some(vp) = variant_path {
        let candidate = vp
            .parent()
            .map(|p| p.join(extends_rel))
            .unwrap_or_else(|| project_root.join(extends_rel));
        if candidate.exists() {
            candidate
        } else {
            project_root.join(extends_rel)
        }
    } else {
        project_root.join(extends_rel)
    };

    let base_content = match std::fs::read_to_string(&base_path) {
        Ok(c) => c,
        Err(e) => {
            let (l, c) = find_location(source, "extends =");
            errors.push(
                ValidationError::new(
                    "variant.extends",
                    format!(
                        "base config '{}' not found at {}: {e}",
                        extends_rel,
                        base_path.display()
                    ),
                )
                .with_location(l, c),
            );
            return Err(errors);
        }
    };

    let base_config = match validate_app_toml(&base_content, Some(project_root)) {
        Ok(c) => c,
        Err(errs) => {
            errors.extend(errs);
            return Err(errors);
        }
    };

    let mut resolved_config = base_config.clone();

    // 2. Overlay [variant.app]
    if let Some(app_ov) = &variant_file.variant.app {
        // A-02: identifier MUST differ per variant
        if app_ov.identifier == base_config.app.identifier {
            let (l, c) = find_location(source, &format!("identifier = \"{}\"", app_ov.identifier));
            errors.push(
                ValidationError::new(
                    "variant.app.identifier",
                    format!(
                        "variant identifier '{}' must differ from base app.identifier '{}'",
                        app_ov.identifier, base_config.app.identifier
                    ),
                )
                .with_location(l, c),
            );
        }

        resolved_config.app.name = app_ov.name.clone();
        resolved_config.app.identifier = app_ov.identifier.clone();
        resolved_config.app.accent = app_ov.accent.clone();
        resolved_config.app.platforms = app_ov.platforms.clone();
        resolved_config.brand.logo = app_ov.icon.clone();
    }

    // 3. Overlay [variant.modules]
    let all_discovered_modules = discover_modules(project_root);
    let mut included_manifests: Vec<ModuleManifestFile> = Vec::new();

    if let Some(mod_ov) = &variant_file.variant.modules {
        // Framework modules override
        for (k, v) in &mod_ov.framework {
            if k == "backup" {
                let (l, c) = find_location(source, "backup");
                errors.push(
                    ValidationError::new(
                        "variant.modules.framework",
                        "backup is mandatory and cannot be configured under framework modules",
                    )
                    .with_location(l, c),
                );
                continue;
            }

            let is_known = FRAMEWORK_MODULES.iter().any(|(name, _)| name == k);
            if !is_known {
                let (l, c) = find_location(source, k);
                errors.push(
                    ValidationError::new(
                        "variant.modules.framework",
                        format!("unknown framework module '{}'", k),
                    )
                    .with_location(l, c),
                );
            } else {
                let mods = resolved_config
                    .modules
                    .get_or_insert(crate::config::ModulesSection {
                        profiles: Some(true),
                        ai: Some(true),
                        pro: Some(true),
                        feedback: Some(true),
                        updater: Some(true),
                        crash: Some(true),
                        integrations: Some(true),
                        scheduler: Some(true),
                        support_page: Some(false),
                        builder: Some(false),
                        background: Some(false),
                    });
                match k.as_str() {
                    "profiles" => mods.profiles = Some(*v),
                    "ai" => mods.ai = Some(*v),
                    "pro" => mods.pro = Some(*v),
                    "feedback" => mods.feedback = Some(*v),
                    "updater" => mods.updater = Some(*v),
                    "crash" => mods.crash = Some(*v),
                    "integrations" => mods.integrations = Some(*v),
                    "scheduler" => mods.scheduler = Some(*v),
                    "support_page" => mods.support_page = Some(*v),
                    "builder" => mods.builder = Some(*v),
                    "background" => mods.background = Some(*v),
                    _ => {}
                }
            }
        }

        // Product modules include
        for mod_key in &mod_ov.include {
            if let Some(m) = all_discovered_modules
                .iter()
                .find(|m| &m.module.key == mod_key)
            {
                included_manifests.push(m.clone());
            } else {
                let (l, c) = find_location(source, &format!("\"{}\"", mod_key));
                errors.push(
                    ValidationError::new(
                        "variant.modules.include",
                        format!("unknown module '{}'", mod_key),
                    )
                    .with_location(l, c),
                );
            }
        }
    }

    // 4. Overlay [variant.pro]
    if let Some(pro_ov) = &variant_file.variant.pro {
        let pro_sec = resolved_config
            .pro
            .get_or_insert(crate::config::ProSection {
                gcc_product_id: None,
                trial_days: Some(14),
                device_limit: Some(3),
                offline_grace_days: Some(7),
                refresh_hours: Some(24),
                currencies: None,
                features: None,
                public_keys: None,
            });
        if let Some(gid) = &pro_ov.gcc_product_id {
            pro_sec.gcc_product_id = Some(gid.clone());
        }
        if let Some(feats) = &pro_ov.features {
            pro_sec.features = Some(feats.clone());
        }
    }

    // 5. Re-validate resolved AppConfigFile through validate_app_toml
    match toml::to_string_pretty(&resolved_config) {
        Ok(resolved_toml) => {
            if let Err(res_errs) = validate_app_toml(&resolved_toml, Some(project_root)) {
                for mut re in res_errs {
                    // Try to map line/col back to variant source if possible
                    let (vl, vc) = find_location(source, &re.path);
                    if vl > 0 {
                        re.line = Some(vl);
                        re.column = Some(vc);
                    }
                    errors.push(re);
                }
            }
        }
        Err(e) => {
            errors.push(ValidationError::new(
                "resolved_config",
                format!("serialization error: {e}"),
            ));
        }
    }

    // 6. Validate module graph of included manifests
    if !included_manifests.is_empty()
        && let Err(mod_errs) = validate_module_graph_full(&included_manifests, None, None)
    {
        errors.extend(mod_errs);
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // 7. Compute enabled modules and cargo features
    let mut enabled_modules = Vec::new();
    let mut cargo_features = Vec::new();

    if let Some(ref m) = resolved_config.modules {
        if m.profiles.unwrap_or(true) {
            enabled_modules.push("profiles".into());
            cargo_features.push("mod-profiles".into());
        }
        if m.ai.unwrap_or(true) {
            enabled_modules.push("ai".into());
            cargo_features.push("mod-ai".into());
            cargo_features.push("mod-ai-advanced".into());
        }
        if m.pro.unwrap_or(true) {
            enabled_modules.push("pro".into());
            cargo_features.push("mod-pro".into());
        }
        if m.feedback.unwrap_or(true) {
            enabled_modules.push("feedback".into());
            cargo_features.push("mod-feedback".into());
        }
        if m.updater.unwrap_or(true) {
            enabled_modules.push("updater".into());
            cargo_features.push("mod-updater".into());
        }
        if m.crash.unwrap_or(true) {
            enabled_modules.push("crash".into());
            cargo_features.push("mod-crash".into());
        }
        if m.integrations.unwrap_or(true) {
            enabled_modules.push("integrations".into());
            cargo_features.push("mod-integrations".into());
        }
        if m.scheduler.unwrap_or(true) {
            enabled_modules.push("scheduler".into());
            cargo_features.push("mod-scheduler".into());
        }
        if m.support_page.unwrap_or(false) {
            enabled_modules.push("support_page".into());
            cargo_features.push("mod-support-page".into());
        }
        if m.background.unwrap_or(false) {
            enabled_modules.push("background".into());
            cargo_features.push("mod-background".into());
        }
        if m.builder.unwrap_or(false) {
            enabled_modules.push("builder".into());
            cargo_features.push("builder".into());
        }
    }

    for manifest in &included_manifests {
        enabled_modules.push(manifest.module.key.clone());
        cargo_features.push(manifest.module.cargo_feature.clone());
    }

    enabled_modules.sort();
    enabled_modules.dedup();
    cargo_features.sort();
    cargo_features.dedup();

    let defaults = variant_file.variant.defaults.clone().unwrap_or_default();
    let locked_keys = variant_file
        .variant
        .locked
        .as_ref()
        .map(|l| l.keys.clone())
        .unwrap_or_default();

    Ok(ResolvedVariant {
        key: variant_file.variant.key.clone(),
        variant_path: variant_path.map(|p| p.to_path_buf()),
        extends: extends_rel.clone(),
        base_config,
        resolved_config,
        variant_config: variant_file,
        enabled_modules,
        cargo_features,
        defaults,
        locked_keys,
    })
}

/// Resolves a variant from file path.
pub fn resolve_variant_from_file(
    variant_path: &Path,
    project_root: &Path,
) -> Result<ResolvedVariant, Vec<ValidationError>> {
    let content = std::fs::read_to_string(variant_path).map_err(|e| {
        vec![ValidationError::new(
            variant_path.display().to_string(),
            format!("failed to read file: {e}"),
        )]
    })?;
    resolve_variant(&content, Some(variant_path), project_root)
}

/// Validates all variant files in `<project_root>/variants/`.
pub fn validate_all_variants(
    project_root: &Path,
) -> Result<Vec<ResolvedVariant>, Vec<ValidationError>> {
    let variants_dir = project_root.join("variants");
    if !variants_dir.exists() {
        return Ok(Vec::new());
    }

    let mut resolved_variants = Vec::new();
    let mut errors = Vec::new();

    let mut entries = match std::fs::read_dir(&variants_dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect::<Vec<_>>(),
        Err(e) => {
            return Err(vec![ValidationError::new(
                "variants",
                format!("failed to read variants directory: {e}"),
            )]);
        }
    };
    entries.sort_by_key(|e| e.file_name());

    let mut keys = HashSet::new();
    let mut identifiers = HashSet::new();

    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(ValidationError::new(
                        path.display().to_string(),
                        format!("failed to read file: {e}"),
                    ));
                    continue;
                }
            };

            match resolve_variant(&content, Some(&path), project_root) {
                Ok(rv) => {
                    // Check duplicate key
                    if !keys.insert(rv.key.clone()) {
                        let (l, c) = find_location(&content, &format!("key = \"{}\"", rv.key));
                        errors.push(
                            ValidationError::new(
                                "variant.key",
                                format!("duplicate variant key '{}'", rv.key),
                            )
                            .with_location(l, c),
                        );
                    }

                    // Check duplicate identifier across variants
                    let id = rv.resolved_config.app.identifier.clone();
                    if !identifiers.insert(id.clone()) {
                        let (l, c) = find_location(&content, &format!("identifier = \"{}\"", id));
                        errors.push(
                            ValidationError::new(
                                "variant.app.identifier",
                                format!("duplicate variant identifier '{}' across variants", id),
                            )
                            .with_location(l, c),
                        );
                    }

                    resolved_variants.push(rv);
                }
                Err(errs) => {
                    errors.extend(errs);
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(resolved_variants)
    } else {
        Err(errors)
    }
}

/// Generates a diff summary between two variants.
pub fn diff_variants(v1: &ResolvedVariant, v2: &ResolvedVariant) -> String {
    let mut out = String::new();
    out.push_str(&format!("Variant Diff: '{}' vs '{}'\n", v1.key, v2.key));
    out.push_str("==============================================\n");

    if v1.resolved_config.app.name != v2.resolved_config.app.name {
        out.push_str(&format!(
            "App Name:       - {}\n                + {}\n",
            v1.resolved_config.app.name, v2.resolved_config.app.name
        ));
    }
    if v1.resolved_config.app.identifier != v2.resolved_config.app.identifier {
        out.push_str(&format!(
            "Identifier:     - {}\n                + {}\n",
            v1.resolved_config.app.identifier, v2.resolved_config.app.identifier
        ));
    }
    if v1.resolved_config.app.accent != v2.resolved_config.app.accent {
        out.push_str(&format!(
            "Accent:         - {}\n                + {}\n",
            v1.resolved_config.app.accent, v2.resolved_config.app.accent
        ));
    }
    if v1.resolved_config.app.platforms != v2.resolved_config.app.platforms {
        out.push_str(&format!(
            "Platforms:      - {:?}\n                + {:?}\n",
            v1.resolved_config.app.platforms, v2.resolved_config.app.platforms
        ));
    }
    if v1.resolved_config.brand.logo != v2.resolved_config.brand.logo {
        out.push_str(&format!(
            "Icon/Logo:      - {}\n                + {}\n",
            v1.resolved_config.brand.logo, v2.resolved_config.brand.logo
        ));
    }

    let p1 = v1
        .resolved_config
        .pro
        .as_ref()
        .and_then(|p| p.gcc_product_id.as_deref())
        .unwrap_or("");
    let p2 = v2
        .resolved_config
        .pro
        .as_ref()
        .and_then(|p| p.gcc_product_id.as_deref())
        .unwrap_or("");
    if p1 != p2 {
        out.push_str(&format!(
            "GCC Product ID: - {}\n                + {}\n",
            p1, p2
        ));
    }

    // Modules difference
    let mods1: HashSet<_> = v1.enabled_modules.iter().collect();
    let mods2: HashSet<_> = v2.enabled_modules.iter().collect();
    let removed: Vec<_> = mods1.difference(&mods2).map(|s| s.as_str()).collect();
    let added: Vec<_> = mods2.difference(&mods1).map(|s| s.as_str()).collect();
    if !removed.is_empty() || !added.is_empty() {
        out.push_str("Modules:\n");
        for r in removed {
            out.push_str(&format!("  - {}\n", r));
        }
        for a in added {
            out.push_str(&format!("  + {}\n", a));
        }
    }

    // Cargo features difference
    let feat1: HashSet<_> = v1.cargo_features.iter().collect();
    let feat2: HashSet<_> = v2.cargo_features.iter().collect();
    let feat_rem: Vec<_> = feat1.difference(&feat2).map(|s| s.as_str()).collect();
    let feat_add: Vec<_> = feat2.difference(&feat1).map(|s| s.as_str()).collect();
    if !feat_rem.is_empty() || !feat_add.is_empty() {
        out.push_str("Cargo Features:\n");
        for r in feat_rem {
            out.push_str(&format!("  - {}\n", r));
        }
        for a in feat_add {
            out.push_str(&format!("  + {}\n", a));
        }
    }

    out
}

/// Generates a markdown or text matrix table of Module x Variant.
pub fn generate_variant_matrix(variants: &[ResolvedVariant], format: &str) -> String {
    let mut all_modules = BTreeMap::new();
    for (name, _) in FRAMEWORK_MODULES {
        all_modules.insert(name.to_string(), "framework");
    }
    for v in variants {
        if let Some(mod_ov) = &v.variant_config.variant.modules {
            for inc in &mod_ov.include {
                all_modules.insert(inc.clone(), "product");
            }
        }
    }

    if format == "md" || format == "markdown" {
        let mut md = String::new();
        md.push_str("| Module | Kind |");
        for v in variants {
            md.push_str(&format!(" {} |", v.key));
        }
        md.push('\n');
        md.push_str("|---|---|");
        for _ in variants {
            md.push_str("---|");
        }
        md.push('\n');

        for (mod_key, kind) in &all_modules {
            md.push_str(&format!("| {} | {} |", mod_key, kind));
            for v in variants {
                let is_enabled = v.enabled_modules.contains(mod_key);
                let mark = if is_enabled { "✓" } else { "-" };
                md.push_str(&format!(" {} |", mark));
            }
            md.push('\n');
        }
        md
    } else {
        let mut txt = String::new();
        txt.push_str("MODULE MATRIX\n");
        txt.push_str("----------------------------------------------\n");
        for (mod_key, kind) in &all_modules {
            txt.push_str(&format!("{:<20} ({:<9}): ", mod_key, kind));
            for v in variants {
                let is_enabled = v.enabled_modules.contains(mod_key);
                let mark = if is_enabled { "✓" } else { "-" };
                txt.push_str(&format!("{}: {}  ", v.key, mark));
            }
            txt.push('\n');
        }
        txt
    }
}
