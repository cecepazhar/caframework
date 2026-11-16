use crate::error::{CafError, DbError};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub const FRAMEWORK_MIGRATION_MIN: i32 = 1;
pub const FRAMEWORK_MIGRATION_MAX: i32 = 999;
pub const PRODUCT_MIGRATION_MIN: i32 = 1000;

pub const FRAMEWORK_MIGRATIONS: &[(i32, &str, &str)] = &[
    (
        1,
        "0001_baseline_schema",
        include_str!("migrations/0001_baseline_schema.sql"),
    ),
    (
        2,
        "0002_ai_routing_and_memory",
        include_str!("migrations/0002_ai_routing_and_memory.sql"),
    ),
];

fn compute_checksum(sql: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sql.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn validate_framework_migrations_range() -> Result<(), CafError> {
    for &(version, name, _) in FRAMEWORK_MIGRATIONS {
        if !(FRAMEWORK_MIGRATION_MIN..=FRAMEWORK_MIGRATION_MAX).contains(&version) {
            return Err(CafError::Db(DbError::MigrationFailed));
        }
        if version <= 0 {
            return Err(CafError::Db(DbError::Generic(format!(
                "invalid framework migration version {version} for {name}"
            ))));
        }
    }
    Ok(())
}

pub fn validate_product_migrations_range(migrations: &[(i32, &str, &str)]) -> Result<(), CafError> {
    for &(version, name, _) in migrations {
        if version < PRODUCT_MIGRATION_MIN {
            return Err(CafError::Db(DbError::MigrationFailed));
        }
        if version <= 0 {
            return Err(CafError::Db(DbError::Generic(format!(
                "invalid product migration version {version} for {name}"
            ))));
        }
    }
    Ok(())
}

pub fn run_migrations(conn: &mut Connection) -> Result<(), CafError> {
    validate_framework_migrations_range()?;
    apply_migrations_list(conn, FRAMEWORK_MIGRATIONS)
}

pub fn run_product_migrations(
    conn: &mut Connection,
    product_migrations: &[(i32, &str, &str)],
) -> Result<(), CafError> {
    validate_product_migrations_range(product_migrations)?;
    apply_migrations_list(conn, product_migrations)
}

fn apply_migrations_list(
    conn: &mut Connection,
    migrations: &[(i32, &str, &str)],
) -> Result<(), CafError> {
    // Bootstrap schema_version if not present
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            checksum TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        );",
    )
    .map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to bootstrap schema_version: {e}"
        )))
    })?;

    let tx = conn.transaction().map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to start migration transaction: {e}"
        )))
    })?;

    let max_version: i32 = tx
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let mut applied_any = false;

    for &(version, name, sql) in migrations {
        let expected_checksum = compute_checksum(sql);

        if version <= max_version {
            // Verify checksum for already applied migrations
            let stored_checksum: Result<String, _> = tx.query_row(
                "SELECT checksum FROM schema_version WHERE version = ?1",
                [&version],
                |row| row.get(0),
            );

            if let Ok(stored) = stored_checksum {
                if stored != expected_checksum {
                    return Err(CafError::Db(DbError::MigrationChecksum));
                }
            } else {
                return Err(CafError::Db(DbError::Generic(format!(
                    "migration {} is marked applied but no checksum found",
                    version
                ))));
            }
            continue;
        }

        // Apply new migration
        tx.execute_batch(sql).map_err(|e| {
            CafError::Db(DbError::Generic(format!(
                "failed to apply migration {}: {}",
                version, e
            )))
        })?;

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        tx.execute(
            "INSERT INTO schema_version (version, name, checksum, applied_at) VALUES (?1, ?2, ?3, ?4)",
            (&version, &name, &expected_checksum, &now_ms),
        )
        .map_err(|e| {
            CafError::Db(DbError::Generic(format!(
                "failed to record migration {}: {}",
                version, e
            )))
        })?;

        applied_any = true;
    }

    if applied_any {
        tx.commit().map_err(|e| {
            CafError::Db(DbError::Generic(format!(
                "failed to commit migrations: {e}"
            )))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_range_enforcement() {
        assert!(validate_framework_migrations_range().is_ok());

        // Reject invalid framework migration ranges
        let bad_framework = &[(1001, "bad_fw", "SELECT 1;")];
        for &(v, _, _) in bad_framework {
            assert!(
                !(FRAMEWORK_MIGRATION_MIN..=FRAMEWORK_MIGRATION_MAX).contains(&v),
                "Framework migration must be strictly between 0001 and 0999"
            );
        }

        // Reject product migrations in framework range
        let bad_product = &[(500, "bad_prod", "SELECT 1;")];
        assert_eq!(
            validate_product_migrations_range(bad_product).unwrap_err(),
            CafError::Db(DbError::MigrationFailed)
        );

        let good_product = &[(1000, "good_prod", "SELECT 1;")];
        assert!(validate_product_migrations_range(good_product).is_ok());
    }

    #[test]
    fn test_migrations_forward_encrypted() {
        let temp_dir = std::env::temp_dir().join(format!("caf_mig_test_{}", uuid::Uuid::new_v4()));
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let conn = crate::db::open_encrypted(&temp_dir, key).expect("open encrypted db");

        let cv: i32 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query max version");
        assert_eq!(cv, 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
