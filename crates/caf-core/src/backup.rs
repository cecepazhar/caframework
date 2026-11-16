//! Encrypted Vault Backup & Restore v2 (ADR 0002, FR-9, F-09).
//! Binary format:
//! `b"CAFBAK\0\0"` (8 bytes) | `format_version: u16 = 2` | `header_len: u32` | `header_json` | `nonce (12 bytes)` | `ciphertext_and_tag`

use crate::error::{CafError, DbError, VaultError};
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Nonce};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::Path;
use zeroize::Zeroizing;

pub const MAGIC: &[u8; 8] = b"CAFBAK\0\0";
pub const FORMAT_VERSION: u16 = 2;
pub const APP_ID: &str = "com.caframework.app";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupAuthMode {
    Passphrase,
    Keyring,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupHeader {
    pub app_id: String,
    pub created_at: u64,
    pub schema_version: i32,
    pub salt: String, // hex-encoded 32-byte salt
    pub auth_mode: BackupAuthMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDump {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPayload {
    pub app_id: String,
    pub schema_version: i32,
    pub created_at: u64,
    pub tables: Vec<TableDump>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPrefs {
    pub folder_path: String,
    pub retention_count: u32,
    pub last_successful_backup: Option<u64>,
}

impl Default for BackupPrefs {
    fn default() -> Self {
        Self {
            folder_path: String::new(),
            retention_count: 5,
            last_successful_backup: None,
        }
    }
}

fn parse_header_meta(data: &[u8]) -> Result<(BackupHeader, usize), CafError> {
    if data.len() < 14 {
        return Err(CafError::Vault(VaultError::Generic(
            "Backup data too short".into(),
        )));
    }
    let magic = data
        .get(0..8)
        .ok_or_else(|| CafError::Vault(VaultError::Generic("data too short".into())))?;
    if magic != MAGIC {
        return Err(CafError::Vault(VaultError::Generic(
            "Invalid backup magic bytes".into(),
        )));
    }
    let v0 = *data
        .get(8)
        .ok_or_else(|| CafError::Vault(VaultError::Generic("data too short".into())))?;
    let v1 = *data
        .get(9)
        .ok_or_else(|| CafError::Vault(VaultError::Generic("data too short".into())))?;
    let version = u16::from_be_bytes([v0, v1]);
    if version != FORMAT_VERSION {
        return Err(CafError::Vault(VaultError::Generic(format!(
            "Unsupported backup format version: {version}"
        ))));
    }
    let h0 = *data
        .get(10)
        .ok_or_else(|| CafError::Vault(VaultError::Generic("data too short".into())))?;
    let h1 = *data
        .get(11)
        .ok_or_else(|| CafError::Vault(VaultError::Generic("data too short".into())))?;
    let h2 = *data
        .get(12)
        .ok_or_else(|| CafError::Vault(VaultError::Generic("data too short".into())))?;
    let h3 = *data
        .get(13)
        .ok_or_else(|| CafError::Vault(VaultError::Generic("data too short".into())))?;
    let header_len = u32::from_be_bytes([h0, h1, h2, h3]) as usize;

    let header_raw = data
        .get(14..14 + header_len)
        .ok_or_else(|| CafError::Vault(VaultError::Generic("Truncated backup header".into())))?;
    let header_str = std::str::from_utf8(header_raw)
        .map_err(|e| CafError::Vault(VaultError::Generic(format!("Invalid header UTF-8: {e}"))))?;
    let header: BackupHeader = serde_json::from_str(header_str)
        .map_err(|e| CafError::Vault(VaultError::Generic(format!("Corrupt backup header: {e}"))))?;

    if header.app_id != APP_ID {
        return Err(CafError::Vault(VaultError::Generic(format!(
            "Incompatible application backup: expected {}, found {}",
            APP_ID, header.app_id
        ))));
    }

    Ok((header, header_len))
}

/// Reads the unencrypted header from raw backup bytes without requiring secrets.
pub fn inspect_backup_header(data: &[u8]) -> Result<BackupHeader, CafError> {
    parse_header_meta(data).map(|(h, _)| h)
}

/// Export backup encrypted with user passphrase (P07.1, P07.2).
pub fn export_backup(passphrase: &str) -> Result<Vec<u8>, CafError> {
    if passphrase.len() < crate::vault::MIN_PASSWORD_LEN {
        return Err(CafError::Vault(VaultError::Generic(format!(
            "Backup passphrase minimal {} karakter",
            crate::vault::MIN_PASSWORD_LEN
        ))));
    }

    let conn = crate::db::open()?;
    let payload = dump_payload_from_conn(&conn)?;

    let mut salt = [0u8; 32];
    use aes_gcm::aead::rand_core::RngCore;
    OsRng.fill_bytes(&mut salt);

    let key = derive_key_argon2(passphrase, &salt)?;
    let backup_bytes = pack_and_encrypt(&payload, &salt, BackupAuthMode::Passphrase, &key)?;

    record_backup_success(payload.created_at)?;
    Ok(backup_bytes)
}

/// Export backup encrypted with the keyring backup key (P07.4: automated backup).
pub fn export_backup_with_keyring(data_dir: &Path) -> Result<Vec<u8>, CafError> {
    let keyring = crate::keyring::load(data_dir)?;
    let master_key_hex = crate::vault::ensure_unlocked_key()?;
    let keys = crate::keyring::unwrap_with_password(&keyring, &master_key_hex)
        .or_else(|_| crate::keyring::guarded_unwrap_with_password(&keyring, &master_key_hex))?;

    export_backup_with_bk(&keys.bk)
}

/// Export backup directly with a raw 32-byte backup key.
pub fn export_backup_with_bk(bk: &[u8; 32]) -> Result<Vec<u8>, CafError> {
    let conn = crate::db::open()?;
    let payload = dump_payload_from_conn(&conn)?;

    let mut salt = [0u8; 32];
    use aes_gcm::aead::rand_core::RngCore;
    OsRng.fill_bytes(&mut salt);

    let key = derive_key_from_bk(bk, &salt);
    let backup_bytes = pack_and_encrypt(&payload, &salt, BackupAuthMode::Keyring, &key)?;

    record_backup_success(payload.created_at)?;
    Ok(backup_bytes)
}

/// Import and atomically restore backup data (P07.3, P07.4).
pub fn import_backup(data: &[u8], passphrase_or_recovery: &str) -> Result<(), CafError> {
    let (header, header_len) = parse_header_meta(data)?;
    let salt_bytes = hex::decode(&header.salt)
        .map_err(|e| CafError::Vault(VaultError::Generic(format!("invalid salt hex: {e}"))))?;
    if salt_bytes.len() != 32 {
        return Err(CafError::Vault(VaultError::Generic(
            "invalid salt length".into(),
        )));
    }
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&salt_bytes);

    let key = match header.auth_mode {
        BackupAuthMode::Passphrase => derive_key_argon2(passphrase_or_recovery, &salt)?,
        BackupAuthMode::Keyring => {
            let data_dir = crate::paths::data_dir()?;
            let keyring = crate::keyring::load(&data_dir)?;
            let unwrapped = crate::keyring::unwrap_with_password(&keyring, passphrase_or_recovery)
                .or_else(|_| {
                    let ent = crate::keyring::mnemonic_to_entropy(passphrase_or_recovery)?;
                    crate::keyring::unwrap_with_recovery(&keyring, &ent)
                })
                .map_err(|_| {
                    CafError::Vault(VaultError::Generic(
                        "Password backup atau recovery phrase salah.".into(),
                    ))
                })?;
            derive_key_from_bk(&unwrapped.bk, &salt)
        }
    };

    let ciphertext_and_nonce = data
        .get(14 + header_len..)
        .ok_or_else(|| CafError::Vault(VaultError::Generic("truncated ciphertext".into())))?;

    if ciphertext_and_nonce.len() < 12 + 16 {
        return Err(CafError::Vault(VaultError::Generic(
            "Backup payload corrupted or truncated".into(),
        )));
    }

    let (nonce_bytes, ciphertext) = ciphertext_and_nonce.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key.as_ref())
        .map_err(|e| CafError::Vault(VaultError::Generic(format!("cipher init failed: {e}"))))?;

    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| {
            CafError::Vault(VaultError::Generic(
                "Backup decryption failed: authentication tag mismatch or corrupted data".into(),
            ))
        })?;

    let payload: BackupPayload = serde_json::from_slice(&plaintext)
        .map_err(|e| CafError::Vault(VaultError::Generic(format!("corrupt payload JSON: {e}"))))?;

    let mut conn = crate::db::open()?;
    restore_database_tables(&mut conn, &payload.tables, payload.schema_version)?;

    Ok(())
}

/// Discovers all dynamic tables and dumps their contents (P07.2).
pub fn dump_payload_from_conn(conn: &rusqlite::Connection) -> Result<BackupPayload, CafError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let schema_version: i32 = conn
        .query_row("PRAGMA user_version;", [], |r| r.get(0))
        .unwrap_or(0);

    let tables = dump_database_tables(conn)?;

    Ok(BackupPayload {
        app_id: APP_ID.to_string(),
        schema_version,
        created_at: now,
        tables,
    })
}

/// Dynamically discovers all tables in the SQLite schema (excluding virtual and system tables).
pub fn dump_database_tables(conn: &rusqlite::Connection) -> Result<Vec<TableDump>, CafError> {
    let mut stmt = conn
        .prepare(
            "SELECT name, sql FROM sqlite_master \
             WHERE type = 'table' \
               AND name NOT LIKE 'sqlite_%' \
               AND name NOT LIKE '%_fts%' \
               AND name NOT LIKE '%_idx%' \
               AND name NOT LIKE '%_docsize%' \
               AND name NOT LIKE '%_config%' \
               AND name NOT LIKE '%_content%' \
               AND name NOT LIKE '%_data%' \
             ORDER BY name;",
        )
        .map_err(|e| CafError::Db(DbError::Generic(format!("query sqlite_master: {e}"))))?;

    let table_meta = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let sql: Option<String> = row.get(1)?;
            Ok((name, sql))
        })
        .map_err(|e| CafError::Db(DbError::Generic(format!("{e}"))))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CafError::Db(DbError::Generic(format!("{e}"))))?;

    let mut dumps = Vec::new();
    for (name, sql) in table_meta {
        if let Some(s) = sql
            && s.trim().to_uppercase().starts_with("CREATE VIRTUAL TABLE")
        {
            continue;
        }

        // Columns
        let mut col_stmt = conn
            .prepare(&format!("PRAGMA table_info(\"{name}\");"))
            .map_err(|e| CafError::Db(DbError::Generic(format!("table_info: {e}"))))?;
        let columns: Vec<String> = col_stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|e| CafError::Db(DbError::Generic(format!("{e}"))))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CafError::Db(DbError::Generic(format!("{e}"))))?;

        if columns.is_empty() {
            continue;
        }

        // Rows
        let mut row_stmt = conn
            .prepare(&format!("SELECT * FROM \"{name}\";"))
            .map_err(|e| CafError::Db(DbError::Generic(format!("select rows: {e}"))))?;
        let col_count = columns.len();
        let mut rows = Vec::new();
        let mut cursor = row_stmt
            .query([])
            .map_err(|e| CafError::Db(DbError::Generic(format!("{e}"))))?;

        while let Some(sqlite_row) = cursor
            .next()
            .map_err(|e| CafError::Db(DbError::Generic(format!("{e}"))))?
        {
            let mut row_vals = Vec::with_capacity(col_count);
            for idx in 0..col_count {
                let v_ref = sqlite_row
                    .get_ref(idx)
                    .map_err(|e| CafError::Db(DbError::Generic(format!("{e}"))))?;
                let val = match v_ref {
                    rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                    rusqlite::types::ValueRef::Integer(i) => serde_json::json!(i),
                    rusqlite::types::ValueRef::Real(f) => serde_json::json!(f),
                    rusqlite::types::ValueRef::Text(t) => {
                        let s = std::str::from_utf8(t).unwrap_or("");
                        serde_json::json!(s)
                    }
                    rusqlite::types::ValueRef::Blob(b) => {
                        serde_json::json!({ "$b64": base64::engine::general_purpose::STANDARD.encode(b) })
                    }
                };
                row_vals.push(val);
            }
            rows.push(row_vals);
        }

        dumps.push(TableDump {
            name,
            columns,
            rows,
        });
    }

    Ok(dumps)
}

/// Restores all dumped tables into the database inside a single atomic SQLite transaction (P07.3).
pub fn restore_database_tables(
    conn: &mut rusqlite::Connection,
    tables: &[TableDump],
    schema_version: i32,
) -> Result<(), CafError> {
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .map_err(|e| CafError::Db(DbError::Generic(format!("disable foreign keys: {e}"))))?;

    let tx = conn
        .transaction()
        .map_err(|e| CafError::Db(DbError::Generic(format!("transaction begin: {e}"))))?;

    tx.execute_batch("PRAGMA defer_foreign_keys = ON;")
        .map_err(|e| CafError::Db(DbError::Generic(format!("defer foreign keys: {e}"))))?;

    for table in tables {
        tx.execute(&format!("DELETE FROM \"{}\";", table.name), [])
            .map_err(|e| {
                CafError::Db(DbError::Generic(format!("clear table {}: {e}", table.name)))
            })?;

        if table.columns.is_empty() || table.rows.is_empty() {
            continue;
        }

        let quoted_cols: Vec<String> = table.columns.iter().map(|c| format!("\"{}\"", c)).collect();
        let cols_joined = quoted_cols.join(", ");
        let placeholders: Vec<String> =
            (1..=table.columns.len()).map(|i| format!("?{i}")).collect();
        let ph_joined = placeholders.join(", ");
        let insert_sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({});",
            table.name, cols_joined, ph_joined
        );

        let mut insert_stmt = tx
            .prepare(&insert_sql)
            .map_err(|e| CafError::Db(DbError::Generic(format!("prepare insert: {e}"))))?;

        for row in &table.rows {
            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::with_capacity(row.len());
            for cell in row {
                match cell {
                    serde_json::Value::Null => params.push(Box::new(rusqlite::types::Null)),
                    serde_json::Value::Bool(b) => params.push(Box::new(*b)),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            params.push(Box::new(i));
                        } else if let Some(f) = n.as_f64() {
                            params.push(Box::new(f));
                        } else {
                            params.push(Box::new(rusqlite::types::Null));
                        }
                    }
                    serde_json::Value::String(s) => params.push(Box::new(s.clone())),
                    serde_json::Value::Object(map) => {
                        if let Some(serde_json::Value::String(b64)) = map.get("$b64")
                            && let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64)
                        {
                            params.push(Box::new(bytes));
                        } else {
                            params.push(Box::new(serde_json::to_string(cell).unwrap_or_default()));
                        }
                    }
                    serde_json::Value::Array(_) => {
                        params.push(Box::new(serde_json::to_string(cell).unwrap_or_default()));
                    }
                }
            }

            let params_slice: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            insert_stmt.execute(&params_slice[..]).map_err(|e| {
                CafError::Db(DbError::Generic(format!(
                    "insert row into {}: {e}",
                    table.name
                )))
            })?;
        }
    }

    // Rebuild FTS if ai_habits table exists
    let has_habits: bool = tx
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='ai_habits_fts'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if has_habits {
        let _ = tx.execute_batch("INSERT INTO ai_habits_fts(ai_habits_fts) VALUES('rebuild');");
    }

    tx.pragma_update(None, "user_version", schema_version)
        .map_err(|e| CafError::Db(DbError::Generic(format!("update user_version: {e}"))))?;

    tx.commit()
        .map_err(|e| CafError::Db(DbError::Generic(format!("commit restore: {e}"))))?;

    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| CafError::Db(DbError::Generic(format!("re-enable foreign keys: {e}"))))?;

    Ok(())
}

fn derive_key_argon2(passphrase: &str, salt: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>, CafError> {
    let mut derived_key = Zeroizing::new([0u8; 32]);
    let params = argon2::Params::new(64 * 1024, 3, 4, Some(32))
        .map_err(|e| CafError::Vault(VaultError::Generic(e.to_string())))?;
    let argon2 = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, derived_key.as_mut())
        .map_err(|e| CafError::Vault(VaultError::Generic(e.to_string())))?;
    Ok(derived_key)
}

fn derive_key_from_bk(bk: &[u8; 32], salt: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"caframework-backup-keyring-v2");
    hasher.update(salt);
    hasher.update(bk);
    let mut out = Zeroizing::new([0u8; 32]);
    out.copy_from_slice(&hasher.finalize());
    out
}

fn pack_and_encrypt(
    payload: &BackupPayload,
    salt: &[u8; 32],
    auth_mode: BackupAuthMode,
    key: &[u8; 32],
) -> Result<Vec<u8>, CafError> {
    let header = BackupHeader {
        app_id: payload.app_id.clone(),
        created_at: payload.created_at,
        schema_version: payload.schema_version,
        salt: hex::encode(salt),
        auth_mode,
    };

    let header_json = serde_json::to_string(&header)
        .map_err(|e| CafError::Vault(VaultError::Generic(e.to_string())))?;
    let header_bytes = header_json.as_bytes();
    let header_len = header_bytes.len() as u32;

    let payload_bytes = serde_json::to_vec(payload)
        .map_err(|e| CafError::Vault(VaultError::Generic(e.to_string())))?;

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CafError::Vault(VaultError::Generic(format!("cipher init failed: {e}"))))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, &payload_bytes[..])
        .map_err(|e| CafError::Vault(VaultError::Generic(format!("encryption failed: {e}"))))?;

    let mut out = Vec::with_capacity(8 + 2 + 4 + header_bytes.len() + 12 + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&FORMAT_VERSION.to_be_bytes());
    out.extend_from_slice(&header_len.to_be_bytes());
    out.extend_from_slice(header_bytes);
    out.extend_from_slice(nonce.as_slice());
    out.extend_from_slice(&ciphertext);

    Ok(out)
}

pub fn get_last_backup_time() -> Result<Option<u64>, CafError> {
    let conn = crate::db::open()?;
    let val: Result<String, _> = conn.query_row(
        "SELECT value FROM app_kv WHERE key = 'last_successful_backup_timestamp'",
        [],
        |r| r.get(0),
    );
    match val {
        Ok(s) => Ok(s.parse::<u64>().ok()),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(CafError::Db(DbError::Generic(format!("{e}")))),
    }
}

pub fn record_backup_success(timestamp: u64) -> Result<(), CafError> {
    let conn = crate::db::open()?;
    conn.execute(
        "INSERT INTO app_kv (key, value) VALUES ('last_successful_backup_timestamp', ?1) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
        [timestamp.to_string()],
    )
    .map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to record backup timestamp: {e}"
        )))
    })?;
    Ok(())
}

pub fn get_backup_prefs() -> Result<BackupPrefs, CafError> {
    let conn = crate::db::open()?;
    let last = get_last_backup_time()?;
    let folder: String = conn
        .query_row(
            "SELECT value FROM app_kv WHERE key = 'backup_folder_path'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();
    let retention: u32 = conn
        .query_row(
            "SELECT value FROM app_kv WHERE key = 'backup_retention_count'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(String::from("5"))
        .parse()
        .unwrap_or(5);

    Ok(BackupPrefs {
        folder_path: folder,
        retention_count: retention,
        last_successful_backup: last,
    })
}

pub fn set_backup_prefs(prefs: &BackupPrefs) -> Result<(), CafError> {
    let conn = crate::db::open()?;
    conn.execute(
        "INSERT INTO app_kv (key, value) VALUES ('backup_folder_path', ?1) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
        [&prefs.folder_path],
    )
    .map_err(|e| CafError::Db(DbError::Generic(format!("{e}"))))?;

    conn.execute(
        "INSERT INTO app_kv (key, value) VALUES ('backup_retention_count', ?1) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
        [prefs.retention_count.to_string()],
    )
    .map_err(|e| CafError::Db(DbError::Generic(format!("{e}"))))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

pub fn pack_and_encrypt_for_test(
    payload: &BackupPayload,
    salt: &[u8; 32],
    auth_mode: BackupAuthMode,
    key: &[u8; 32],
) -> Result<Vec<u8>, CafError> {
    pack_and_encrypt(payload, salt, auth_mode, key)
}

pub fn derive_key_from_bk_for_test(bk: &[u8; 32], salt: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    derive_key_from_bk(bk, salt)
}

pub fn test_decrypt_payload(data: &[u8], passphrase: &str) -> Result<BackupPayload, CafError> {
    let (header, header_len) = parse_header_meta(data)?;
    let salt_bytes = hex::decode(&header.salt)
        .map_err(|e| CafError::Vault(VaultError::Generic(format!("invalid salt: {e}"))))?;
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&salt_bytes);

    let key = derive_key_argon2(passphrase, &salt)?;
    let ciphertext_and_nonce = data
        .get(14 + header_len..)
        .ok_or_else(|| CafError::Vault(VaultError::Generic("data too short".into())))?;
    if ciphertext_and_nonce.len() < 12 + 16 {
        return Err(CafError::Vault(VaultError::Generic(
            "data too short".into(),
        )));
    }
    let (nonce_bytes, ciphertext) = ciphertext_and_nonce.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key.as_ref())
        .map_err(|e| CafError::Vault(VaultError::Generic(format!("cipher init: {e}"))))?;

    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| {
            CafError::Vault(VaultError::Generic(
                "Backup decryption failed: authentication tag mismatch or corrupted data".into(),
            ))
        })?;

    let payload: BackupPayload = serde_json::from_slice(&plaintext)
        .map_err(|e| CafError::Vault(VaultError::Generic(format!("json: {e}"))))?;
    Ok(payload)
}
