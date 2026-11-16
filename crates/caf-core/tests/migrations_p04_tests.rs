//! P04 Acceptance Tests: Migrations, sync-ready data, central write helper, and device id.

use caf_core::error::{CafError, DbError};
use caf_core::migrations::{
    FRAMEWORK_MIGRATION_MAX, FRAMEWORK_MIGRATION_MIN, validate_framework_migrations_range,
    validate_product_migrations_range,
};
use caf_core::vault;
use rusqlite::Connection;
use std::collections::HashSet;
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
        let dir = std::env::temp_dir().join(format!("caf_p04_{label}_{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::set_var("CAF_DATA_DIR", &dir);
        }
        Self { dir, _guard: guard }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        vault::lock();
        let _ = fs::remove_dir_all(&self.dir);
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::remove_var("CAF_DATA_DIR");
        }
    }
}

// --------------------------------------------------------------------------
// P04.1: Range enforcement and encrypted connection
// --------------------------------------------------------------------------

#[test]
fn test_p04_1_range_enforcement_and_encrypted_connection() {
    let env = TestEnv::new("p04_1_range");

    // 1. Framework range validation (0001..0999)
    assert!(validate_framework_migrations_range().is_ok());

    // Product range validation (1000+)
    let bad_product = &[(999, "bad_under_1000", "SELECT 1;")];
    assert_eq!(
        validate_product_migrations_range(bad_product).unwrap_err(),
        CafError::Db(DbError::MigrationFailed)
    );

    let bad_product_zero = &[(0, "bad_zero", "SELECT 1;")];
    assert_eq!(
        validate_product_migrations_range(bad_product_zero).unwrap_err(),
        CafError::Db(DbError::MigrationFailed)
    );

    let valid_product = &[
        (
            1000,
            "first_product_mig",
            "CREATE TABLE IF NOT EXISTS prod_test (id TEXT PRIMARY KEY);",
        ),
        (
            1001,
            "second_product_mig",
            "CREATE TABLE IF NOT EXISTS prod_test2 (id TEXT PRIMARY KEY);",
        ),
    ];
    assert!(validate_product_migrations_range(valid_product).is_ok());

    // 2. Encrypted connection migration test
    let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let mut conn = caf_core::db::open_encrypted(&env.dir, key).expect("open encrypted db");

    let cv: i32 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query max version");
    assert!(
        (FRAMEWORK_MIGRATION_MIN..=FRAMEWORK_MIGRATION_MAX).contains(&cv),
        "Max framework migration must be in range 0001..0999, got {cv}"
    );

    // Apply product migrations on the same encrypted connection
    caf_core::migrations::run_product_migrations(&mut conn, valid_product)
        .expect("run product migrations");
    let max_prod_v: i32 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query prod version");
    assert_eq!(max_prod_v, 1001);
}

// --------------------------------------------------------------------------
// P04.2: Exact change_log schema
// --------------------------------------------------------------------------

#[test]
fn test_p04_2_change_log_exact_columns() {
    let env = TestEnv::new("p04_2_changelog_schema");
    let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let conn = caf_core::db::open_encrypted(&env.dir, key).expect("open encrypted db");

    let mut stmt = conn
        .prepare("PRAGMA table_info(change_log);")
        .expect("pragma table_info");
    let rows = stmt
        .query_map([], |row| {
            let col_name: String = row.get(1)?;
            let col_type: String = row.get(2)?;
            let not_null: i32 = row.get(3)?;
            let is_pk: i32 = row.get(5)?;
            Ok((col_name, col_type, not_null, is_pk))
        })
        .expect("query_map");

    let cols: Vec<(String, String, i32, i32)> = rows.map(|r| r.unwrap()).collect();
    let col_names: Vec<String> = cols.iter().map(|c| c.0.clone()).collect();

    let expected_names = vec![
        "seq".to_string(),
        "entity".to_string(),
        "entity_id".to_string(),
        "op".to_string(),
        "rev".to_string(),
        "at".to_string(),
        "profile_id".to_string(),
        "device_id".to_string(),
    ];

    assert_eq!(
        col_names, expected_names,
        "change_log columns must match exact PRD FR-5 specification"
    );

    // Assert seq is primary key integer
    let seq_col = cols.iter().find(|c| c.0 == "seq").unwrap();
    assert_eq!(seq_col.3, 1, "seq must be primary key");

    // Assert entity, entity_id, op, rev, at, device_id are NOT NULL
    let non_nulls = ["entity", "entity_id", "op", "rev", "at", "device_id"];
    for name in &non_nulls {
        let col = cols.iter().find(|c| c.0 == *name).unwrap();
        assert_eq!(col.2, 1, "column {} must be NOT NULL", name);
    }
}

// --------------------------------------------------------------------------
// P04.3: Central write helper & atomicity
// --------------------------------------------------------------------------

#[test]
fn test_p04_3_central_write_helper_and_atomicity() {
    let env = TestEnv::new("p04_3_central_write");
    let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let mut conn = caf_core::db::open_encrypted(&env.dir, key).expect("open encrypted db");

    // 1. Insert helper
    {
        let tx = conn.transaction().expect("start tx");
        let seq = caf_core::sync::insert(&tx, "notes", "note-p04-1", "user-1", "dev-1").unwrap();
        assert!(seq > 0);
        tx.commit().expect("commit");
    }

    let op: String = conn
        .query_row(
            "SELECT op FROM change_log WHERE entity_id = 'note-p04-1'",
            [],
            |r| r.get(0),
        )
        .expect("get op");
    assert_eq!(op, "insert");

    // 2. Update helper
    {
        let tx = conn.transaction().expect("start tx");
        let seq = caf_core::sync::update(&tx, "notes", "note-p04-1", 2, "user-1", "dev-1").unwrap();
        assert!(seq > 0);
        tx.commit().expect("commit");
    }

    let rev: i64 = conn
        .query_row(
            "SELECT rev FROM change_log WHERE entity_id = 'note-p04-1' AND op = 'update'",
            [],
            |r| r.get(0),
        )
        .expect("get rev");
    assert_eq!(rev, 2);

    // 3. Soft delete helper
    {
        let tx = conn.transaction().expect("start tx");
        let seq =
            caf_core::sync::soft_delete(&tx, "notes", "note-p04-1", 3, "user-1", "dev-1").unwrap();
        assert!(seq > 0);
        tx.commit().expect("commit");
    }

    let del_op: String = conn
        .query_row(
            "SELECT op FROM change_log WHERE entity_id = 'note-p04-1' AND rev = 3",
            [],
            |r| r.get(0),
        )
        .expect("get del op");
    assert_eq!(del_op, "delete");

    // 4. Restore helper
    {
        let tx = conn.transaction().expect("start tx");
        let seq =
            caf_core::sync::restore(&tx, "notes", "note-p04-1", 4, "user-1", "dev-1").unwrap();
        assert!(seq > 0);
        tx.commit().expect("commit");
    }

    let restore_op: String = conn
        .query_row(
            "SELECT op FROM change_log WHERE entity_id = 'note-p04-1' AND rev = 4",
            [],
            |r| r.get(0),
        )
        .expect("get restore op");
    assert_eq!(restore_op, "update");

    // 5. Atomicity: failure / rollback rolls back both entity row and change_log
    {
        let tx = conn.transaction().expect("start tx");
        tx.execute(
            "INSERT INTO notes (id, title, rev, created_at, updated_at, origin_device_id, owner_profile_id, visibility)
             VALUES (?1, ?2, 1, 1000, 1000, 'dev-1', 'user-1', 'shared')",
            ["note-fail-atomicity", "Failing note"],
        )
        .expect("insert note");

        caf_core::sync::insert(&tx, "notes", "note-fail-atomicity", "user-1", "dev-1").unwrap();
        // Drop without commit (simulating transaction abort / change_log failure)
    }

    let note_exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM notes WHERE id = 'note-fail-atomicity'",
            [],
            |r| r.get(0),
        )
        .expect("query note");
    assert_eq!(note_exists, 0, "Aborted transaction must roll back note");

    let cl_exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM change_log WHERE entity_id = 'note-fail-atomicity'",
            [],
            |r| r.get(0),
        )
        .expect("query change_log");
    assert_eq!(
        cl_exists, 0,
        "Aborted transaction must roll back change_log"
    );
}

// --------------------------------------------------------------------------
// P04.4: Convention test on the real schema
// --------------------------------------------------------------------------

#[test]
fn test_p04_4_schema_conventions_and_negative_fixture() {
    let env = TestEnv::new("p04_4_conventions");
    let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let conn = caf_core::db::open_encrypted(&env.dir, key).expect("open encrypted db");

    let required_columns = ["owner_profile_id", "visibility", "updated_at", "deleted_at"];

    // Documented exclusions with reasons:
    let excluded_tables: HashSet<&str> = [
        "schema_version",    // Framework migration tracking table
        "app_kv",            // Framework key-value configuration store
        "change_log",        // Sync audit log itself
        "ai_providers",      // Global AI configuration
        "ai_user_personas",  // Global AI personas
        "ai_routing_matrix", // Global AI routing rules
        "ai_habits",         // AI local memory learning habits table
        "ai_custom_skills",  // AI custom skill catalog
        "ai_habits_fts",     // Virtual full-text search table
        "command_logs",      // Audit event log table
        "sqlite_sequence",   // SQLite internal sequence counter
    ]
    .into_iter()
    .collect();

    fn check_table_conventions(
        conn: &Connection,
        table: &str,
        required_cols: &[&str],
    ) -> Result<(), String> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table});"))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| e.to_string())?;
        let col_names: HashSet<String> = rows.filter_map(|r| r.ok()).collect();

        for &req in required_cols {
            if !col_names.contains(req) {
                return Err(format!(
                    "Table '{table}' is missing required column '{req}'"
                ));
            }
        }
        Ok(())
    }

    // Check all tables in real migrated database
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';")
        .expect("prepare");
    let table_names: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .expect("query_map")
        .filter_map(|r| r.ok())
        .collect();

    for table in &table_names {
        if table.starts_with("ai_habits_fts") || excluded_tables.contains(table.as_str()) {
            continue;
        }
        check_table_conventions(&conn, table, &required_columns)
            .unwrap_or_else(|e| panic!("Convention failure: {e}"));
    }

    // Negative fixture test: adding a table with missing columns must fail convention check
    conn.execute_batch(
        "CREATE TABLE invalid_sync_table (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL
         );",
    )
    .expect("create bad table");

    let check_bad = check_table_conventions(&conn, "invalid_sync_table", &required_columns);
    assert!(
        check_bad.is_err(),
        "Table without FR-5 columns must fail convention check"
    );
}

// --------------------------------------------------------------------------
// P04.5: Stable device ID stored inside the vault
// --------------------------------------------------------------------------

#[test]
fn test_p04_5_device_id_stable_and_isolated() {
    let _env1 = TestEnv::new("p04_5_dev_id_1");
    let _setup = vault::setup_vault("test-master-password-123").expect("setup");

    // 1. Device ID is created and stored in vault
    let id_1 = caf_core::paths::device_id().expect("device_id");
    assert!(!id_1.is_empty());

    // Verify stored inside vault (app_kv)
    let conn = caf_core::db::open().expect("open db");
    let stored_in_vault: String = conn
        .query_row(
            "SELECT value FROM app_kv WHERE key = 'origin_device_id'",
            [],
            |r| r.get(0),
        )
        .expect("query app_kv");
    assert_eq!(
        stored_in_vault, id_1,
        "Device ID must be stored in app_kv inside vault"
    );
    drop(conn);

    // 2. Restart simulation: lock and unlock, id remains identical
    vault::lock();
    vault::unlock_with_password("test-master-password-123").expect("unlock");
    let id_after_restart = caf_core::paths::device_id().expect("device_id after restart");
    assert_eq!(
        id_1, id_after_restart,
        "Device ID must be stable across restarts in the same vault"
    );

    // Drop first environment before creating second environment to release TEST_LOCK
    drop(_env1);

    // 3. Different data dir produces different device ID
    let _env2 = TestEnv::new("p04_5_dev_id_2");
    let _setup2 = vault::setup_vault("test-master-password-123").expect("setup 2");
    let id_2 = caf_core::paths::device_id().expect("device_id 2");
    assert_ne!(
        id_1, id_2,
        "Different data dirs must have different device IDs"
    );
}
