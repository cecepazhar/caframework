//! Encrypted SQLite storage engine for CAFramework. Every record persisted lives inside a
//! single SQLCipher-encrypted file at `<data_dir>/caframework.db` — see [`crate::vault`].

use crate::error::{CafError, DbError};
use rusqlite::Connection;
use std::path::Path;

/// Opens (creating if absent) the SQLCipher-encrypted database at `<data_dir>/caframework.db`,
/// keys it with `passphrase`, and ensures the schema exists.
pub fn open_encrypted(data_dir: &Path, passphrase: &str) -> Result<Connection, CafError> {
    std::fs::create_dir_all(data_dir)
        .map_err(|e| CafError::Db(DbError::Generic(format!("failed to create data dir: {e}"))))?;
    let db_path = crate::paths::db_path(data_dir);

    let mut conn = if let Some(raw_key) = raw_key_literal(passphrase) {
        match open_keyed(&db_path, &raw_key) {
            Ok(conn) => conn,
            Err(_) => {
                let conn = open_keyed(&db_path, &passphrase_literal(passphrase))?;
                conn.execute_batch(&format!("PRAGMA rekey = {raw_key};"))
                    .map_err(|e| {
                        CafError::Db(DbError::Generic(format!(
                            "failed to migrate database to raw-key mode: {e}"
                        )))
                    })?;
                conn
            }
        }
    } else {
        open_keyed(&db_path, &passphrase_literal(passphrase))?
    };

    crate::migrations::run_migrations(&mut conn)?;

    // Seed default builtin skills if empty
    let skills_count: i64 = conn
        .query_row("SELECT count(*) FROM ai_custom_skills", [], |r| r.get(0))
        .unwrap_or(0);
    if skills_count == 0 {
        for s in crate::ai::skills::builtin_skills() {
            let _ = crate::ai::db::save_skill(&conn, &s);
        }
    }

    Ok(conn)
}

fn passphrase_literal(passphrase: &str) -> String {
    format!("'{}'", passphrase.replace('\'', "''"))
}

fn raw_key_literal(passphrase: &str) -> Option<String> {
    if passphrase.len() == 64 && passphrase.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(format!("\"x'{passphrase}'\""))
    } else {
        None
    }
}

fn open_keyed(db_path: &Path, key_literal: &str) -> Result<Connection, CafError> {
    if db_path.exists()
        && let Ok(metadata) = std::fs::metadata(db_path)
        && metadata.len() >= 16
        && let Ok(mut file) = std::fs::File::open(db_path)
    {
        use std::io::Read;
        let mut header = [0u8; 16];
        if file.read_exact(&mut header).is_ok() && &header == b"SQLite format 3\0" {
            return Err(CafError::Vault(crate::error::VaultError::CorruptedHeader));
        }
    }

    let conn = Connection::open(db_path)
        .map_err(|e| CafError::Db(DbError::Generic(format!("failed to open database: {e}"))))?;

    conn.execute_batch(&format!("PRAGMA key = {key_literal};"))
        .map_err(|e| CafError::Db(DbError::Generic(format!("failed to set vault key: {e}"))))?;

    conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(|_| CafError::Vault(crate::error::VaultError::CorruptedHeader))?;

    // PRAGMA integrity_check on unlock (P03.4)
    let integrity: Result<String, _> = conn.query_row("PRAGMA integrity_check;", [], |r| r.get(0));
    match integrity {
        Ok(res) if res == "ok" => {}
        _ => return Err(CafError::Vault(crate::error::VaultError::CorruptedHeader)),
    }

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;\nPRAGMA synchronous = NORMAL;\nPRAGMA foreign_keys = ON;",
    )
    .map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to configure database pragmas: {e}"
        )))
    })?;

    Ok(conn)
}

/// Convenience helper to open the encrypted DB with the current vault key.
pub fn open() -> Result<Connection, CafError> {
    let data_dir = crate::paths::data_dir()?;
    let key = crate::vault::ensure_unlocked_key()?;
    open_encrypted(&data_dir, &key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_encrypted_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("caf_db_test_{}", uuid::Uuid::new_v4()));
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let conn = open_encrypted(&temp_dir, key).expect("open encrypted db");

        conn.execute(
            "INSERT INTO app_kv (key, value) VALUES (?1, ?2)",
            ["test_k", "test_v"],
        )
        .unwrap();
        let val: String = conn
            .query_row("SELECT value FROM app_kv WHERE key = ?1", ["test_k"], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(val, "test_v");

        drop(conn);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
