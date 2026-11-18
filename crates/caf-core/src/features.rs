//! Runtime Feature Toggles (A-03 / P14)
//!
//! Provides the runtime feature toggles table, definitions registry,
//! `locked_when` predicates, and enforcement logic.

use crate::error::{CafError, DbError, FeatureError};
use parking_lot::RwLock;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// A predicate function called before a feature can be toggled off.
/// If it returns `Ok(Some(reason))`, the toggle is rejected with that reason.
pub type LockedWhenPredicate =
    Arc<dyn Fn(&Connection) -> Result<Option<String>, CafError> + Send + Sync>;

/// Static definition and metadata of a feature or module toggle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureDefinition {
    pub key: String,
    pub label_en: String,
    pub label_id: String,
    pub description_en: String,
    pub description_id: String,
    pub default_enabled: bool,
    pub locked: bool,
    pub locked_reason: Option<String>,
    pub locked_when_predicate: Option<String>,
}

/// Sync-ready record stored in the `feature_flags` database table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureFlagRecord {
    pub key: String,
    pub enabled: bool,
    pub locked: bool,
    pub locked_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub rev: i64,
    pub origin_device_id: String,
    pub owner_profile_id: String,
    pub visibility: String,
}

/// Item exposed to IPC and Frontend Settings UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureToggleItem {
    pub key: String,
    pub label_en: String,
    pub label_id: String,
    pub description_en: String,
    pub description_id: String,
    pub enabled: bool,
    pub locked: bool,
    pub locked_reason: Option<String>,
    pub is_compiled: bool,
    pub is_licensed: bool,
    pub has_predicate: bool,
}

static CUSTOM_DEFINITIONS: LazyLock<RwLock<HashMap<String, FeatureDefinition>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static PREDICATES: LazyLock<RwLock<HashMap<String, LockedWhenPredicate>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static RUNTIME_CACHE: LazyLock<RwLock<HashMap<String, bool>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn current_timestamp() -> Result<i64, CafError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| CafError::generic(format!("system clock error: {e}")))
}

/// Built-in framework definitions matching amendment A-01 and A-03.
pub fn builtin_definitions() -> Vec<FeatureDefinition> {
    vec![
        FeatureDefinition {
            key: "profiles".to_string(),
            label_en: "Profiles & Security".to_string(),
            label_id: "Profil & Keamanan".to_string(),
            description_en: "Multi-user authentication, roles, and PIN verification".to_string(),
            description_id: "Autentikasi multi-pengguna, peran, dan verifikasi PIN".to_string(),
            default_enabled: true,
            locked: true,
            locked_reason: Some("Profiles module is mandatory and cannot be disabled".to_string()),
            locked_when_predicate: None,
        },
        FeatureDefinition {
            key: "ai".to_string(),
            label_en: "AI Assistant".to_string(),
            label_id: "Asisten AI".to_string(),
            description_en: "Local and cloud-based AI reasoning, copilot, and habits".to_string(),
            description_id: "Penalaran AI lokal dan cloud, kopilot, dan kebiasaan".to_string(),
            default_enabled: true,
            locked: false,
            locked_reason: None,
            locked_when_predicate: None,
        },
        FeatureDefinition {
            key: "integrations".to_string(),
            label_en: "External Integrations".to_string(),
            label_id: "Integrasi Eksternal".to_string(),
            description_en: "Outbound webhook integrations and synchronized delivery".to_string(),
            description_id: "Integrasi webhook keluar dan pengiriman tersinkronisasi".to_string(),
            default_enabled: true,
            locked: false,
            locked_reason: None,
            locked_when_predicate: None,
        },
        FeatureDefinition {
            key: "scheduler".to_string(),
            label_en: "Background Scheduler".to_string(),
            label_id: "Penjadwal Latar Belakang".to_string(),
            description_en: "Automated cron jobs and background tasks execution".to_string(),
            description_id: "Tugas cron otomatis dan eksekusi tugas latar belakang".to_string(),
            default_enabled: true,
            locked: false,
            locked_reason: None,
            locked_when_predicate: None,
        },
        FeatureDefinition {
            key: "feedback".to_string(),
            label_en: "User Feedback".to_string(),
            label_id: "Umpan Balik Pengguna".to_string(),
            description_en: "Submit diagnostics, issues, and user feedback".to_string(),
            description_id: "Kirimkan diagnostik, kendala, dan masukan pengguna".to_string(),
            default_enabled: true,
            locked: false,
            locked_reason: None,
            locked_when_predicate: None,
        },
        FeatureDefinition {
            key: "updater".to_string(),
            label_en: "Auto Updater".to_string(),
            label_id: "Pembaruan Otomatis".to_string(),
            description_en: "Automatic checking and installation of updates".to_string(),
            description_id: "Pemeriksaan dan pemasangan pembaruan otomatis".to_string(),
            default_enabled: true,
            locked: false,
            locked_reason: None,
            locked_when_predicate: None,
        },
        FeatureDefinition {
            key: "crash".to_string(),
            label_en: "Crash Reporting".to_string(),
            label_id: "Pelaporan Kerusakan".to_string(),
            description_en: "Crash diagnosis and panic logging".to_string(),
            description_id: "Diagnosis kerusakan dan pencatatan kepanikan".to_string(),
            default_enabled: true,
            locked: false,
            locked_reason: None,
            locked_when_predicate: None,
        },
        FeatureDefinition {
            key: "pro".to_string(),
            label_en: "Pro Licensing".to_string(),
            label_id: "Lisensi Pro".to_string(),
            description_en: "Professional features and enterprise licensing gating".to_string(),
            description_id: "Fitur profesional dan pembatasan lisensi korporasi".to_string(),
            default_enabled: true,
            locked: true,
            locked_reason: Some("Pro licensing system cannot be disabled at runtime".to_string()),
            locked_when_predicate: None,
        },
    ]
}

/// Look up a feature definition from custom registered items or built-ins.
pub fn get_feature_definition(key: &str) -> Option<FeatureDefinition> {
    if let Some(def) = CUSTOM_DEFINITIONS.read().get(key) {
        return Some(def.clone());
    }
    builtin_definitions().into_iter().find(|d| d.key == key)
}

/// Register a product-level or test feature definition at runtime.
pub fn register_feature(def: FeatureDefinition) {
    let mut custom = CUSTOM_DEFINITIONS.write();
    custom.insert(def.key.clone(), def);
    RUNTIME_CACHE.write().clear();
}

/// Register a `locked_when` predicate function.
pub fn register_locked_when_predicate<F>(name: impl Into<String>, predicate: F)
where
    F: Fn(&Connection) -> Result<Option<String>, CafError> + Send + Sync + 'static,
{
    let mut map = PREDICATES.write();
    map.insert(name.into(), Arc::new(predicate));
}

/// Reset custom definitions and predicates (used in tests).
pub fn reset_registry() {
    CUSTOM_DEFINITIONS.write().clear();
    PREDICATES.write().clear();
    RUNTIME_CACHE.write().clear();
}

/// Clear in-memory toggle state cache.
pub fn clear_cache() {
    RUNTIME_CACHE.write().clear();
}

/// Check if the module is compiled into this binary via Cargo feature (A-01).
pub fn is_module_compiled(key: &str) -> bool {
    match key {
        "profiles" => cfg!(feature = "mod-profiles"),
        "ai" => cfg!(feature = "mod-ai"),
        "integrations" => cfg!(feature = "mod-integrations"),
        "scheduler" => cfg!(feature = "mod-scheduler"),
        "feedback" => cfg!(feature = "mod-feedback"),
        "updater" => cfg!(feature = "mod-updater"),
        "crash" => cfg!(feature = "mod-crash"),
        "pro" => cfg!(feature = "mod-pro"),
        _ => true, // Product modules default to true if registered
    }
}

/// Check if the feature is licensed (entitlements check).
pub fn is_module_licensed(key: &str) -> bool {
    if key == "pro" {
        #[cfg(feature = "mod-pro")]
        {
            crate::pro::get_pro_status()
                .map(|s| s.is_pro)
                .unwrap_or(false)
        }
        #[cfg(not(feature = "mod-pro"))]
        {
            false
        }
    } else {
        true
    }
}

/// Seeds feature definitions into `feature_flags` table if not present.
pub fn seed_features(conn: &mut Connection) -> Result<(), CafError> {
    let mut all_defs = builtin_definitions();
    for custom_def in CUSTOM_DEFINITIONS.read().values() {
        if !all_defs.iter().any(|d| d.key == custom_def.key) {
            all_defs.push(custom_def.clone());
        }
    }

    let now = current_timestamp()?;
    let device_id: String = conn
        .query_row(
            "SELECT value FROM app_kv WHERE key = 'origin_device_id'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "device-local".into());

    let tx = conn.transaction().map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to begin seed transaction: {e}"
        )))
    })?;

    for def in &all_defs {
        let count: i64 = tx
            .query_row(
                "SELECT count(*) FROM feature_flags WHERE key = ?1",
                params![def.key],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if count == 0 {
            tx.execute(
                "INSERT INTO feature_flags (
                    key, enabled, locked, locked_reason,
                    created_at, updated_at, deleted_at, rev,
                    origin_device_id, owner_profile_id, visibility
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?5, NULL, 1, ?6, 'system', 'shared')",
                params![
                    def.key,
                    if def.default_enabled { 1 } else { 0 },
                    if def.locked { 1 } else { 0 },
                    def.locked_reason,
                    now,
                    device_id,
                ],
            )
            .map_err(|e| {
                CafError::Db(DbError::Generic(format!(
                    "failed to seed feature flag {}: {e}",
                    def.key
                )))
            })?;

            // Append to change_log
            let _ = crate::sync::insert(&tx, "feature_flags", &def.key, "system", &device_id);
        }
    }

    tx.commit().map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to commit seed transaction: {e}"
        )))
    })?;

    Ok(())
}

/// Returns whether a feature flag is currently active.
///
/// A feature is active only when all conditions match:
/// 1. Compiled behind Cargo feature (A-01)
/// 2. Licensed / Entitled
/// 3. Runtime toggle switched on in `feature_flags` table
pub fn is_feature_enabled(key: &str) -> Result<bool, CafError> {
    if !is_module_compiled(key) {
        return Ok(false);
    }
    if !is_module_licensed(key) {
        return Ok(false);
    }

    // Fast-path: memory cache
    if let Some(&cached) = RUNTIME_CACHE.read().get(key) {
        return Ok(cached);
    }

    if !crate::vault::is_unlocked().unwrap_or(false) {
        // Vault locked: fall back to static definition default
        let default_val = get_feature_definition(key)
            .map(|d| d.default_enabled)
            .unwrap_or(true);
        return Ok(default_val);
    }

    let conn = crate::db::open()?;
    let row_enabled: Option<i64> = conn
        .query_row(
            "SELECT enabled FROM feature_flags WHERE key = ?1 AND deleted_at IS NULL",
            params![key],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| {
            CafError::Db(DbError::Generic(format!(
                "failed to read feature flag {key}: {e}"
            )))
        })?;

    let is_on = match row_enabled {
        Some(val) => val != 0,
        None => get_feature_definition(key)
            .map(|d| d.default_enabled)
            .unwrap_or(true),
    };

    RUNTIME_CACHE.write().insert(key.to_string(), is_on);
    Ok(is_on)
}

/// Retrieve the toggle item for a given key.
pub fn get_feature(key: &str) -> Result<FeatureToggleItem, CafError> {
    let def =
        get_feature_definition(key).ok_or(CafError::Feature(FeatureError::FeatureNotFound))?;

    let is_compiled = is_module_compiled(key);
    let is_licensed = is_module_licensed(key);
    let has_predicate = def.locked_when_predicate.is_some();

    if !crate::vault::is_unlocked().unwrap_or(false) {
        return Ok(FeatureToggleItem {
            key: def.key,
            label_en: def.label_en,
            label_id: def.label_id,
            description_en: def.description_en,
            description_id: def.description_id,
            enabled: def.default_enabled,
            locked: def.locked,
            locked_reason: def.locked_reason,
            is_compiled,
            is_licensed,
            has_predicate,
        });
    }

    let conn = crate::db::open()?;
    let row_data: Option<(i64, i64, Option<String>)> = conn
        .query_row(
            "SELECT enabled, locked, locked_reason FROM feature_flags WHERE key = ?1 AND deleted_at IS NULL",
            params![key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| CafError::Db(DbError::Generic(format!("failed to query feature flag {key}: {e}"))))?;

    let (enabled, locked, locked_reason) = match row_data {
        Some((e, l, r)) => (
            e != 0,
            l != 0 || def.locked,
            r.or(def.locked_reason.clone()),
        ),
        None => (def.default_enabled, def.locked, def.locked_reason.clone()),
    };

    Ok(FeatureToggleItem {
        key: def.key,
        label_en: def.label_en,
        label_id: def.label_id,
        description_en: def.description_en,
        description_id: def.description_id,
        enabled,
        locked,
        locked_reason,
        is_compiled,
        is_licensed,
        has_predicate,
    })
}

/// List all feature toggle items.
pub fn list_features() -> Result<Vec<FeatureToggleItem>, CafError> {
    let mut all_defs = builtin_definitions();
    for custom_def in CUSTOM_DEFINITIONS.read().values() {
        if !all_defs.iter().any(|d| d.key == custom_def.key) {
            all_defs.push(custom_def.clone());
        }
    }

    let mut items = Vec::with_capacity(all_defs.len());
    for def in all_defs {
        match get_feature(&def.key) {
            Ok(item) => items.push(item),
            Err(_) => {
                let is_compiled = is_module_compiled(&def.key);
                let is_licensed = is_module_licensed(&def.key);
                items.push(FeatureToggleItem {
                    key: def.key,
                    label_en: def.label_en,
                    label_id: def.label_id,
                    description_en: def.description_en,
                    description_id: def.description_id,
                    enabled: def.default_enabled,
                    locked: def.locked,
                    locked_reason: def.locked_reason,
                    is_compiled,
                    is_licensed,
                    has_predicate: def.locked_when_predicate.is_some(),
                });
            }
        }
    }

    Ok(items)
}

/// Set a feature toggle's enabled state.
///
/// If disabling (`target_enabled == false`), enforces:
/// 1. `locked` must be false
/// 2. `locked_when` predicate check (if defined) must return `Ok(None)`
pub fn set_feature_enabled(key: &str, target_enabled: bool) -> Result<FeatureToggleItem, CafError> {
    let def =
        get_feature_definition(key).ok_or(CafError::Feature(FeatureError::FeatureNotFound))?;

    let actor = match crate::session::get_current_session() {
        Ok(s) => s.profile_id,
        Err(_) => "system".to_string(),
    };
    let device_id = crate::paths::device_id().unwrap_or_else(|_| "device-local".into());

    let mut conn = crate::db::open()?;

    // Read current database state
    let row: Option<(i64, i64, Option<String>, i64)> = conn
        .query_row(
            "SELECT enabled, locked, locked_reason, rev FROM feature_flags WHERE key = ?1 AND deleted_at IS NULL",
            params![key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(|e| CafError::Db(DbError::Generic(format!("failed to query feature flag {key}: {e}"))))?;

    let had_existing_row = row.is_some();
    let (_current_enabled, is_locked, _reason, current_rev) = match &row {
        Some((e, l, r, rev)) => (*e != 0, *l != 0 || def.locked, r.clone(), *rev),
        None => (
            def.default_enabled,
            def.locked,
            def.locked_reason.clone(),
            1,
        ),
    };

    if !target_enabled {
        if is_locked {
            return Err(CafError::Feature(FeatureError::FeatureLocked));
        }

        // Check locked_when predicate
        if let Some(pred_name) = &def.locked_when_predicate {
            let predicate = PREDICATES.read().get(pred_name).cloned().ok_or_else(|| {
                CafError::generic(format!(
                    "registered predicate '{pred_name}' not found for feature '{key}'"
                ))
            })?;

            if let Some(reason) = predicate(&conn)? {
                return Err(CafError::Feature(FeatureError::Generic(reason)));
            }
        }
    }

    let now = current_timestamp()?;
    let new_rev = current_rev + 1;

    let tx = conn.transaction().map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to begin transaction: {e}"
        )))
    })?;

    tx.execute(
        "INSERT INTO feature_flags (
            key, enabled, locked, locked_reason,
            created_at, updated_at, deleted_at, rev,
            origin_device_id, owner_profile_id, visibility
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?5, NULL, ?6, ?7, ?8, 'shared')
         ON CONFLICT(key) DO UPDATE SET
            enabled = excluded.enabled,
            updated_at = excluded.updated_at,
            rev = excluded.rev",
        params![
            key,
            if target_enabled { 1 } else { 0 },
            if is_locked { 1 } else { 0 },
            def.locked_reason,
            now,
            new_rev,
            device_id,
            actor,
        ],
    )
    .map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to update feature flag {key}: {e}"
        )))
    })?;

    // Record sync change_log
    if had_existing_row {
        crate::sync::update(&tx, "feature_flags", key, new_rev, &actor, &device_id)?;
    } else {
        crate::sync::insert(&tx, "feature_flags", key, &actor, &device_id)?;
    }

    tx.commit().map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to commit transaction: {e}"
        )))
    })?;

    // Invalidate/update cache
    RUNTIME_CACHE
        .write()
        .insert(key.to_string(), target_enabled);

    // Audit log event
    let state_str = if target_enabled {
        "enabled"
    } else {
        "disabled"
    };
    let _ = crate::audit::log_event(
        "FEATURE_TOGGLE",
        Some(&actor),
        &format!("{key}: {state_str}"),
    );

    get_feature(key)
}
