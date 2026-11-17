//! Offline integration write outbox (FR-15, P09.5).
//!
//! Enforces:
//! - Offline write queue with ordered delivery (`created_at` ASC).
//! - Idempotency key per item (`X-Idempotency-Key`).
//! - Automatic expiration / TTL.
//! - Dead-letter queue transition after `max_attempts` (default 5).
//! - Flushed by scheduler job `integrations.outbox_flush`.
//! - Storage isolated in `outbox.json` (0600 permissions), accessible even if vault is locked.

use crate::error::{CafError, IoError};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_MAX_OUTBOX_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum OutboxStatus {
    Pending,
    Flushed,
    DeadLetter,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct OutboxItem {
    pub id: String,
    pub integration_key: String,
    pub endpoint: String,
    pub method: String,
    pub payload: String,
    pub idempotency_key: String,
    pub created_at: i64,
    pub attempts: u32,
    pub max_attempts: u32,
    pub status: OutboxStatus,
    pub last_error: Option<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
pub struct FlushSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub expired: usize,
    pub dead_lettered: usize,
}

fn outbox_file_path() -> PathBuf {
    crate::paths::data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("outbox.json")
}

static OUTBOX_LOCK: RwLock<()> = RwLock::new(());

pub fn load_outbox() -> Vec<OutboxItem> {
    let _guard = OUTBOX_LOCK.read();
    let path = outbox_file_path();
    if let Ok(content) = fs::read_to_string(&path)
        && let Ok(items) = serde_json::from_str::<Vec<OutboxItem>>(&content)
    {
        return items;
    }
    Vec::new()
}

pub fn save_outbox(items: &[OutboxItem]) -> Result<(), CafError> {
    let _guard = OUTBOX_LOCK.write();
    let path = outbox_file_path();
    let content = serde_json::to_string_pretty(items)
        .map_err(|e| CafError::Io(IoError::Generic(format!("serialization error: {e}"))))?;

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(&path, content)
        .map_err(|e| CafError::Io(IoError::Generic(format!("failed to write outbox: {e}"))))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn enqueue_outbox(
    integration_key: &str,
    endpoint: &str,
    method: &str,
    payload: &str,
    idempotency_key: Option<&str>,
    ttl_secs: Option<u64>,
) -> Result<String, CafError> {
    let mut items = load_outbox();
    let now = now_secs();
    let id = uuid::Uuid::new_v4().to_string();
    let idem_key = idempotency_key
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let expires_at = ttl_secs.map(|ttl| now + ttl as i64);

    let item = OutboxItem {
        id: id.clone(),
        integration_key: integration_key.to_string(),
        endpoint: endpoint.to_string(),
        method: method.to_ascii_uppercase(),
        payload: payload.to_string(),
        idempotency_key: idem_key,
        created_at: now,
        attempts: 0,
        max_attempts: DEFAULT_MAX_OUTBOX_ATTEMPTS,
        status: OutboxStatus::Pending,
        last_error: None,
        expires_at,
    };

    items.push(item);
    save_outbox(&items)?;
    Ok(id)
}

pub async fn flush_outbox_with_runner<F, Fut>(runner: F) -> Result<FlushSummary, CafError>
where
    F: Fn(OutboxItem) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let mut items = load_outbox();
    let now = now_secs();
    let mut summary = FlushSummary::default();

    // Sort pending items by creation timestamp (FIFO delivery order)
    items.sort_by_key(|i| i.created_at);

    for item in &mut items {
        if item.status != OutboxStatus::Pending {
            continue;
        }

        summary.total += 1;

        // 1. Check TTL Expiry
        if let Some(exp) = item.expires_at
            && now >= exp
        {
            item.status = OutboxStatus::Expired;
            item.last_error = Some("item expired before delivery".to_string());
            summary.expired += 1;
            continue;
        }

        // 2. Attempt delivery
        item.attempts += 1;
        match runner(item.clone()).await {
            Ok(()) => {
                item.status = OutboxStatus::Flushed;
                item.last_error = None;
                summary.succeeded += 1;
            }
            Err(err_msg) => {
                item.last_error = Some(err_msg);
                if item.attempts >= item.max_attempts {
                    item.status = OutboxStatus::DeadLetter;
                    summary.dead_lettered += 1;
                } else {
                    summary.failed += 1;
                }
            }
        }
    }

    save_outbox(&items)?;
    Ok(summary)
}

pub fn get_outbox_items(status: Option<OutboxStatus>, limit: usize) -> Vec<OutboxItem> {
    let items = load_outbox();
    let mut filtered: Vec<OutboxItem> = items
        .into_iter()
        .filter(|i| status.is_none_or(|s| i.status == s))
        .collect();
    filtered.sort_by_key(|i| std::cmp::Reverse(i.created_at));
    if limit > 0 {
        filtered.truncate(limit);
    }
    filtered
}

pub fn clear_outbox() -> Result<(), CafError> {
    save_outbox(&[])
}
