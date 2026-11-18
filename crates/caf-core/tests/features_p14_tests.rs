//! P14 Acceptance Tests: Runtime feature toggles (A-03).
//!
//! Tests:
//! - P14.1: `feature_flags` table, sync-ready schema, seeding and core API.
//! - P14.2: Enforcement in `guarded` (toggled-off feature fails in core even if UI is bypassed).
//! - P14.3: `locked_when` predicates and static locked features.
//! - P14.4: Settings → Features audit events (`FEATURE_TOGGLE`) and `change_log` tracking.

use caf_core::api::policy::guarded;
use caf_core::error::{CafError, FeatureError};
use caf_core::features::{
    FeatureDefinition, get_feature, is_feature_enabled, list_features, register_feature,
    register_locked_when_predicate, reset_registry, set_feature_enabled,
};
use caf_core::session::{Session, clear_current_session, set_current_session};
use caf_core::vault;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct TestEnv {
    dir: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl TestEnv {
    fn new(label: &str) -> Self {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        vault::lock();
        clear_current_session().ok();
        reset_registry();
        let dir = std::env::temp_dir().join(format!("caf_p14_{label}_{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::set_var("CAF_DATA_DIR", &dir);
        }
        vault::setup_vault("test-p14-password-123").expect("setup vault");
        Self { dir, _guard: guard }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        vault::lock();
        clear_current_session().ok();
        reset_registry();
        let _ = fs::remove_dir_all(&self.dir);
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::remove_var("CAF_DATA_DIR");
        }
    }
}

fn set_test_session(profile_id: &str, permissions: &[&str]) {
    let mut session = Session::new(profile_id, "owner");
    for &p in permissions {
        session.permissions.insert(p.to_string());
    }
    set_current_session(session).expect("set session");
}

// --------------------------------------------------------------------------
// P14.1: feature_flags table, sync-ready schema, seeding and core API
// --------------------------------------------------------------------------

#[test]
fn test_p14_1_table_and_core_api() {
    let _env = TestEnv::new("p14_1_api");
    set_test_session("user-p14-1", &["settings:manage", "settings:view"]);

    // 1. Verify seeded features
    let features = list_features().expect("list_features");
    assert!(
        features.len() >= 8,
        "Expected at least 8 built-in features, got {}",
        features.len()
    );

    let profiles = get_feature("profiles").expect("get_feature profiles");
    assert_eq!(profiles.key, "profiles");
    assert!(profiles.enabled);
    assert!(profiles.locked, "profiles must be locked");

    let ai = get_feature("ai").expect("get_feature ai");
    assert_eq!(ai.key, "ai");
    assert!(ai.enabled);
    assert!(!ai.locked, "ai should not be locked");

    // 2. Direct database query to verify sync-ready columns
    let conn = caf_core::db::open().expect("open db");
    let (enabled, locked, rev, owner, visibility): (i64, i64, i64, String, String) = conn
        .query_row(
            "SELECT enabled, locked, rev, owner_profile_id, visibility FROM feature_flags WHERE key = 'profiles'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .expect("query profiles row");

    assert_eq!(enabled, 1);
    assert_eq!(locked, 1);
    assert_eq!(rev, 1);
    assert_eq!(owner, "system");
    assert_eq!(visibility, "shared");

    // 3. Register custom product feature
    register_feature(FeatureDefinition {
        key: "custom.orders".to_string(),
        label_en: "Orders Module".to_string(),
        label_id: "Modul Pesanan".to_string(),
        description_en: "Manage customer orders and checkout".to_string(),
        description_id: "Kelola pesanan dan pembayaran pelanggan".to_string(),
        default_enabled: true,
        locked: false,
        locked_reason: None,
        locked_when_predicate: None,
    });

    let custom_feat = get_feature("custom.orders").expect("get custom feature");
    assert_eq!(custom_feat.key, "custom.orders");
    assert!(custom_feat.enabled);
    assert!(is_feature_enabled("custom.orders").expect("is_feature_enabled"));
}

// --------------------------------------------------------------------------
// P14.2: Enforcement in guarded
// --------------------------------------------------------------------------

#[test]
fn test_p14_2_enforcement_in_guarded() {
    let _env = TestEnv::new("p14_2_guarded");
    // Give permissions to call feedback / updater / crash
    set_test_session(
        "user-p14-2",
        &["settings:manage", "settings:view", "integrations:manage"],
    );

    // Check module feature is initially enabled
    assert!(is_feature_enabled("integrations").expect("integrations is enabled initially"));

    // Guarded execution succeeds when feature is enabled
    let result = guarded("list_integrations", || Ok("integrations_ran"));
    assert_eq!(result.expect("guarded succeeds"), "integrations_ran");

    // Toggle off the feature at runtime
    let updated = set_feature_enabled("integrations", false).expect("disable integrations");
    assert!(!updated.enabled);
    assert!(!is_feature_enabled("integrations").expect("integrations is disabled"));

    // Guarded execution must now fail with FeatureError::FeatureDisabled
    let err = guarded("list_integrations", || Ok("should_not_run")).unwrap_err();
    match err {
        CafError::Feature(FeatureError::FeatureDisabled) => {}
        other => panic!("Expected FeatureDisabled error, got {other:?}"),
    }
    assert_eq!(err.code(), "CAF-FEATURE-001");

    // Re-enable the feature
    let re_enabled = set_feature_enabled("integrations", true).expect("re-enable integrations");
    assert!(re_enabled.enabled);
    assert!(is_feature_enabled("integrations").expect("integrations is re-enabled"));

    // Guarded execution succeeds again
    let result2 = guarded("list_integrations", || Ok("integrations_ran_again"));
    assert_eq!(
        result2.expect("guarded succeeds again"),
        "integrations_ran_again"
    );
}

// --------------------------------------------------------------------------
// P14.3: locked_when predicates and static locked features
// --------------------------------------------------------------------------

#[test]
fn test_p14_3_locked_when_predicates_and_static_locks() {
    let _env = TestEnv::new("p14_3_predicates");
    set_test_session("user-p14-3", &["settings:manage", "settings:view"]);

    // 1. Static locked feature cannot be disabled
    let profiles_err = set_feature_enabled("profiles", false).unwrap_err();
    match profiles_err {
        CafError::Feature(FeatureError::FeatureLocked) => {}
        other => panic!("Expected FeatureLocked, got {other:?}"),
    }
    assert_eq!(profiles_err.code(), "CAF-FEATURE-002");

    // 2. Register feature with a locked_when predicate
    register_locked_when_predicate("has_open_reservation", |conn| {
        let count: i64 = conn
            .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
            .unwrap_or(0);
        if count > 0 {
            Ok(Some(format!(
                "Cannot disable reservation: {} active items exist in the database",
                count
            )))
        } else {
            Ok(None)
        }
    });

    register_feature(FeatureDefinition {
        key: "orders.reservation".to_string(),
        label_en: "Table Reservation".to_string(),
        label_id: "Reservasi Meja".to_string(),
        description_en: "Book tables in advance".to_string(),
        description_id: "Pesan meja lebih awal".to_string(),
        default_enabled: true,
        locked: false,
        locked_reason: None,
        locked_when_predicate: Some("has_open_reservation".to_string()),
    });

    // When notes table has no rows: disabling succeeds
    let disabled = set_feature_enabled("orders.reservation", false)
        .expect("disable should succeed when predicate passes");
    assert!(!disabled.enabled);

    // Re-enable
    set_feature_enabled("orders.reservation", true).expect("re-enable");

    // Insert a note row into the database (simulating live state)
    let conn = caf_core::db::open().expect("open db");
    conn.execute(
        "INSERT INTO notes (id, title, content, tags, rev, created_at, updated_at, origin_device_id, owner_profile_id, visibility)
         VALUES ('note-live-state', 'Live item', '', '[]', 1, 1000, 1000, 'dev-1', 'user-p14-3', 'shared')",
        [],
    )
    .expect("insert note");

    // Attempting to disable now must be rejected with the predicate's reason
    let pred_err = set_feature_enabled("orders.reservation", false).unwrap_err();
    let err_str = pred_err.to_string();
    assert!(
        err_str.contains("Cannot disable reservation: 1 active items exist in the database"),
        "Error message must contain predicate reason, got: {err_str}"
    );

    // Feature should remain enabled
    let current = get_feature("orders.reservation").expect("get_feature");
    assert!(
        current.enabled,
        "Feature must stay enabled after rejected disable attempt"
    );
}

// --------------------------------------------------------------------------
// P14.4: Settings → Features audit events and sync tracking
// --------------------------------------------------------------------------

#[test]
fn test_p14_4_audit_and_change_log() {
    let _env = TestEnv::new("p14_4_audit");
    set_test_session("audit-actor-p14", &["settings:manage", "settings:view"]);

    // Toggle feedback off
    set_feature_enabled("feedback", false).expect("toggle off");

    let conn = caf_core::db::open().expect("open db");

    // 1. Verify FEATURE_TOGGLE audit log event
    let (event_type, host_id, details): (String, Option<String>, String) = conn
        .query_row(
            "SELECT event_type, host_id, details FROM command_logs WHERE event_type = 'FEATURE_TOGGLE' ORDER BY timestamp DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("query audit event");

    assert_eq!(event_type, "FEATURE_TOGGLE");
    assert_eq!(host_id.as_deref(), Some("audit-actor-p14"));
    assert!(details.contains("feedback: disabled"));

    // 2. Verify change_log sync tracking
    let (entity, entity_id, op, profile_id): (String, String, String, String) = conn
        .query_row(
            "SELECT entity, entity_id, op, profile_id FROM change_log WHERE entity = 'feature_flags' AND entity_id = 'feedback' ORDER BY seq DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("query change_log entry");

    assert_eq!(entity, "feature_flags");
    assert_eq!(entity_id, "feedback");
    assert!(op == "insert" || op == "update");
    assert_eq!(profile_id, "audit-actor-p14");

    // Toggle feedback on again
    set_feature_enabled("feedback", true).expect("toggle on");

    let details_on: String = conn
        .query_row(
            "SELECT details FROM command_logs WHERE event_type = 'FEATURE_TOGGLE' ORDER BY timestamp DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .expect("query latest audit event");

    assert!(details_on.contains("feedback: enabled"));
}
