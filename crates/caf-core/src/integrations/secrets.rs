//! Integration secret management in encrypted vault (FR-15, P09.3).
//!
//! Secrets are stored in `app_kv` inside the encrypted database.
//! Secrets are NEVER returned to the frontend (`has_secret` only).

use crate::error::{CafError, DbError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct IntegrationStatusDto {
    pub key: String,
    pub kind: String,
    pub base_url: String,
    pub auth_kind: String,
    pub has_secret: bool,
    pub user_configurable: bool,
}

fn secret_kv_key(integration_key: &str) -> String {
    format!("integration:secret:{integration_key}")
}

pub fn set_integration_secret(integration_key: &str, secret: &str) -> Result<(), CafError> {
    let conn = crate::db::open()?;

    let kv_key = secret_kv_key(integration_key);
    conn.execute(
        "INSERT INTO app_kv (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [&kv_key, secret],
    )
    .map_err(|e| CafError::Db(DbError::Generic(format!("failed to store secret: {e}"))))?;

    Ok(())
}

pub fn get_integration_secret(integration_key: &str) -> Result<Option<String>, CafError> {
    let conn = crate::db::open()?;

    let kv_key = secret_kv_key(integration_key);
    let mut stmt = conn
        .prepare("SELECT value FROM app_kv WHERE key = ?1")
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    let mut rows = stmt
        .query([&kv_key])
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    if let Some(row) = rows.next().map_err(|e| CafError::Db(DbError::Generic(e.to_string())))? {
        let val: String = row.get(0).map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;
        Ok(Some(val))
    } else {
        Ok(None)
    }
}

pub fn has_integration_secret(integration_key: &str) -> Result<bool, CafError> {
    Ok(get_integration_secret(integration_key)?.is_some())
}

pub fn delete_integration_secret(integration_key: &str) -> Result<(), CafError> {
    let conn = crate::db::open()?;

    let kv_key = secret_kv_key(integration_key);
    conn.execute("DELETE FROM app_kv WHERE key = ?1", [&kv_key])
        .map_err(|e| CafError::Db(DbError::Generic(format!("failed to delete secret: {e}"))))?;

    Ok(())
}
