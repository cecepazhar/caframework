use crate::error::{CafError, DbError};
use rusqlite::Transaction;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Appends a change to `change_log` in the given transaction.
/// Schema: `(seq, entity, entity_id, op, rev, at, profile_id, device_id)`.
pub fn append_change_log(
    tx: &Transaction,
    entity: &str,
    entity_id: &str,
    op: &str,
    rev: i64,
    profile_id: &str,
    device_id: &str,
) -> Result<i64, CafError> {
    let now = now_epoch_secs();
    tx.execute(
        "INSERT INTO change_log (entity, entity_id, op, rev, at, profile_id, device_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![entity, entity_id, op, rev, now, profile_id, device_id],
    )
    .map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to append change_log: {e}"
        )))
    })?;

    Ok(tx.last_insert_rowid())
}

/// Central write helper for inserting a record into a sync table.
/// Bumps revision to 1 and logs the 'insert' operation to `change_log`.
pub fn insert(
    tx: &Transaction,
    entity: &str,
    entity_id: &str,
    profile_id: &str,
    device_id: &str,
) -> Result<i64, CafError> {
    append_change_log(tx, entity, entity_id, "insert", 1, profile_id, device_id)
}

/// Central write helper for updating a record in a sync table.
/// Records the new revision and logs the 'update' operation to `change_log`.
pub fn update(
    tx: &Transaction,
    entity: &str,
    entity_id: &str,
    new_rev: i64,
    profile_id: &str,
    device_id: &str,
) -> Result<i64, CafError> {
    append_change_log(
        tx, entity, entity_id, "update", new_rev, profile_id, device_id,
    )
}

/// Central write helper for soft-deleting a record in a sync table.
/// Logs the 'delete' operation to `change_log`.
pub fn soft_delete(
    tx: &Transaction,
    entity: &str,
    entity_id: &str,
    new_rev: i64,
    profile_id: &str,
    device_id: &str,
) -> Result<i64, CafError> {
    append_change_log(
        tx, entity, entity_id, "delete", new_rev, profile_id, device_id,
    )
}

/// Central write helper for restoring a soft-deleted record in a sync table.
/// Logs the 'update' operation to `change_log`.
pub fn restore(
    tx: &Transaction,
    entity: &str,
    entity_id: &str,
    new_rev: i64,
    profile_id: &str,
    device_id: &str,
) -> Result<i64, CafError> {
    append_change_log(
        tx, entity, entity_id, "update", new_rev, profile_id, device_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_log_append_and_atomicity() {
        let temp_dir = std::env::temp_dir().join(format!("caf_sync_test_{}", uuid::Uuid::new_v4()));
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let mut conn = crate::db::open_encrypted(&temp_dir, key).expect("open encrypted db");

        // 1. Success atomicity in transaction
        {
            let tx = conn.transaction().expect("start tx");
            let seq = insert(&tx, "notes", "note-1", "user-1", "dev-1").expect("insert sync");
            assert!(seq > 0);
            tx.commit().expect("commit");
        }

        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM change_log WHERE entity_id = 'note-1'",
                [],
                |r| r.get(0),
            )
            .expect("query count");
        assert_eq!(count, 1);

        // 2. Rollback atomicity: if transaction fails or is dropped without commit, row is not saved
        {
            let tx = conn.transaction().expect("start tx");
            let _ = insert(&tx, "notes", "note-rollback", "user-1", "dev-1").expect("insert");
            // Intentionally drop transaction without commit
        }

        let rb_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM change_log WHERE entity_id = 'note-rollback'",
                [],
                |r| r.get(0),
            )
            .expect("query rb count");
        assert_eq!(rb_count, 0, "Uncommitted transaction must roll back");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
