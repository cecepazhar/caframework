//! Integration request logging with redaction (FR-15, P09.4).
//!
//! Enforces:
//! - Method, host, path, status, duration_ms, timestamp recorded.
//! - NO headers and NO bodies stored.
//! - PII scrubber and query secret scrubber applied to URL path/query.
//! - Rolling in-memory history (max 500 records).

use crate::pii::PiiScrubber;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const MAX_REQUEST_LOG_RETENTION: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct IntegrationRequestLogRecord {
    pub id: String,
    pub integration_key: String,
    pub method: String,
    pub host: String,
    pub path: String, // sanitized and scrubbed
    pub status: u16,
    pub duration_ms: u64,
    pub timestamp: i64,
}

use std::sync::LazyLock;

static QUERY_SECRET_RE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)([?&](?:api_key|token|secret|key|password|access_token|code|client_secret)=)([^&]+)",
    )
    .ok()
});

pub fn redact_path_and_query(path_or_url: &str) -> String {
    // 1. Redact known query string secret patterns
    let scrubbed_query = if let Some(re) = QUERY_SECRET_RE.as_ref() {
        re.replace_all(path_or_url, "${1}[REDACTED]").to_string()
    } else {
        path_or_url.to_string()
    };

    // 2. Apply general PII scrubber (emails, IPs, phones, API keys)
    let scrubber = PiiScrubber::new();
    let (final_path, _) = scrubber.scrub_text(&scrubbed_query);
    final_path
}

static REQUEST_LOGS: Lazy<RwLock<VecDeque<IntegrationRequestLogRecord>>> =
    Lazy::new(|| RwLock::new(VecDeque::new()));

pub fn add_request_log(record: IntegrationRequestLogRecord) {
    let mut logs = REQUEST_LOGS.write();
    logs.push_front(record);
    if logs.len() > MAX_REQUEST_LOG_RETENTION {
        logs.truncate(MAX_REQUEST_LOG_RETENTION);
    }
}

pub fn get_request_logs(
    integration_key: Option<&str>,
    limit: usize,
) -> Vec<IntegrationRequestLogRecord> {
    let logs = REQUEST_LOGS.read();
    let take_count = if limit == 0 {
        MAX_REQUEST_LOG_RETENTION
    } else {
        limit.min(MAX_REQUEST_LOG_RETENTION)
    };
    logs.iter()
        .filter(|r| integration_key.is_none_or(|k| r.integration_key == k))
        .take(take_count)
        .cloned()
        .collect()
}

pub fn clear_request_logs() {
    REQUEST_LOGS.write().clear();
}
