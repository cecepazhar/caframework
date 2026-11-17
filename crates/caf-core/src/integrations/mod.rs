//! Integration and Connector layer (FR-15, P09).
//!
//! Submodules:
//! - `connector`: HTTP connector with auth types, rate limiting, and retries.
//! - `gcc`: GCC billing and entitlement adapter.
//! - `log`: Request log with PII redaction and secret scrubbing (no headers/bodies).
//! - `outbox`: Offline write queue with idempotency, TTL, and dead-letter queue.
//! - `secrets`: Secure vault-backed secret storage (secrets never returned to frontend).

pub mod connector;
pub mod gcc;
pub mod log;
pub mod outbox;
pub mod secrets;

use crate::error::{CafError, HttpError};
use connector::{AuthKind, Connector, ConnectorConfig};
use outbox::FlushSummary;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

pub use gcc::GccAdapter;
pub use log::{IntegrationRequestLogRecord, get_request_logs};
pub use outbox::{OutboxItem, OutboxStatus, get_outbox_items};
pub use secrets::IntegrationStatusDto;

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
    pub duration_ms: u64,
}

static CONNECTOR_REGISTRY: LazyLock<RwLock<HashMap<String, Arc<Connector>>>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        // Register default GCC connector
        let gcc_config = ConnectorConfig {
            key: "gcc".to_string(),
            kind: "gcc_billing".to_string(),
            base_url: "https://127.0.0.1:3007/v1".to_string(),
            user_configurable: false,
            auth: AuthKind::None,
            timeout_ms: 10_000,
            retries: 2,
            rate_limit_per_minute: Some(60),
            concurrency: Some(4),
            allowed_hosts: vec!["127.0.0.1".to_string(), "localhost".to_string()],
        };
        let c = Connector::new(gcc_config);
        map.insert("gcc".to_string(), Arc::new(c));
        RwLock::new(map)
    });

pub fn register_connector(connector: Connector) {
    let mut reg = CONNECTOR_REGISTRY.write();
    reg.insert(connector.config.key.clone(), Arc::new(connector));
}

pub fn get_connector(key: &str) -> Option<Arc<Connector>> {
    CONNECTOR_REGISTRY.read().get(key).cloned()
}

pub fn list_integrations() -> Result<Vec<IntegrationStatusDto>, CafError> {
    let reg = CONNECTOR_REGISTRY.read();
    let mut list = Vec::new();

    for c in reg.values() {
        let has_secret = secrets::has_integration_secret(&c.config.key).unwrap_or(false);
        list.push(IntegrationStatusDto {
            key: c.config.key.clone(),
            kind: c.config.kind.clone(),
            base_url: c.config.base_url.clone(),
            auth_kind: c.config.auth.as_str().to_string(),
            has_secret,
            user_configurable: c.config.user_configurable,
        });
    }

    list.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(list)
}

pub async fn test_integration_connection(key: &str) -> Result<TestConnectionResult, CafError> {
    let connector = get_connector(key)
        .ok_or(CafError::Http(HttpError::Network))?;

    let start = std::time::Instant::now();
    let res = connector.request("GET", "/", HashMap::new(), None, None).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    match res {
        Ok(resp) => Ok(TestConnectionResult {
            success: resp.status < 400,
            message: format!("HTTP {}", resp.status),
            duration_ms,
        }),
        Err(e) => Ok(TestConnectionResult {
            success: false,
            message: format!("Connection failed: {e}"),
            duration_ms,
        }),
    }
}

pub async fn flush_outbox() -> Result<FlushSummary, CafError> {
    outbox::flush_outbox_with_runner(|item| async move {
        let connector = match get_connector(&item.integration_key) {
            Some(c) => c,
            None => return Err(format!("unknown integration: {}", item.integration_key)),
        };

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Idempotency-Key".to_string(), item.idempotency_key.clone());

        match connector
            .request(
                &item.method,
                &item.endpoint,
                headers,
                Some(item.payload.into_bytes()),
                Some(&item.idempotency_key),
            )
            .await
        {
            Ok(resp) if resp.status < 400 => Ok(()),
            Ok(resp) => Err(format!("HTTP error {}", resp.status)),
            Err(e) => Err(format!("network error: {e}")),
        }
    })
    .await
}
