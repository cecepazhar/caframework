//! Connector layer, authentication kinds, rate/concurrency limits, and retry backoff (FR-15, P09.2).

use crate::error::{CafError, HttpError};
use crate::http::{HttpClient, HttpRequest, HttpResponse};
use crate::integrations::log::{
    IntegrationRequestLogRecord, add_request_log, redact_path_and_query,
};
use crate::integrations::secrets::get_integration_secret;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Semaphore;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthKind {
    None,
    ApiKeyHeader(String),
    ApiKeyQuery(String),
    Bearer,
    Basic,
    HmacSha256(String),
    OAuth2ClientCredentials,
}

impl AuthKind {
    pub fn parse(s: &str, header: Option<&str>) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "api_key_header" => AuthKind::ApiKeyHeader(header.unwrap_or("X-API-Key").to_string()),
            "api_key_query" => AuthKind::ApiKeyQuery(header.unwrap_or("api_key").to_string()),
            "bearer" => AuthKind::Bearer,
            "basic" => AuthKind::Basic,
            "hmac_sha256" => AuthKind::HmacSha256(header.unwrap_or("X-Signature").to_string()),
            "oauth2_client_credentials" => AuthKind::OAuth2ClientCredentials,
            _ => AuthKind::None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AuthKind::None => "none",
            AuthKind::ApiKeyHeader(_) => "api_key_header",
            AuthKind::ApiKeyQuery(_) => "api_key_query",
            AuthKind::Bearer => "bearer",
            AuthKind::Basic => "basic",
            AuthKind::HmacSha256(_) => "hmac_sha256",
            AuthKind::OAuth2ClientCredentials => "oauth2_client_credentials",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectorConfig {
    pub key: String,
    pub kind: String,
    pub base_url: String,
    pub user_configurable: bool,
    pub auth: AuthKind,
    pub timeout_ms: u64,
    pub retries: u32,
    pub rate_limit_per_minute: Option<u32>,
    pub concurrency: Option<u32>,
    pub allowed_hosts: Vec<String>,
}

pub struct Connector {
    pub config: ConnectorConfig,
    client: HttpClient,
    concurrency_semaphore: Option<Arc<Semaphore>>,
    rate_limit_minute_start: AtomicU64,
    rate_limit_count: AtomicU32,
}

impl Connector {
    pub fn new(config: ConnectorConfig) -> Self {
        let semaphore = config
            .concurrency
            .map(|c| Arc::new(Semaphore::new(c as usize)));
        let client = HttpClient::default_client();
        Self {
            config,
            client,
            concurrency_semaphore: semaphore,
            rate_limit_minute_start: AtomicU64::new(0),
            rate_limit_count: AtomicU32::new(0),
        }
    }

    fn check_rate_limit(&self) -> Result<(), CafError> {
        let limit = match self.config.rate_limit_per_minute {
            Some(l) if l > 0 => l,
            _ => return Ok(()),
        };

        let now_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let window = self.rate_limit_minute_start.load(Ordering::Acquire);
        if now_sec >= window + 60 {
            // New minute window
            self.rate_limit_minute_start
                .store(now_sec, Ordering::Release);
            self.rate_limit_count.store(1, Ordering::Release);
            Ok(())
        } else {
            let current = self.rate_limit_count.fetch_add(1, Ordering::SeqCst);
            if current >= limit {
                Err(CafError::Http(HttpError::HttpStatus))
            } else {
                Ok(())
            }
        }
    }

    pub async fn request(
        &self,
        method: &str,
        path: &str,
        custom_headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
        idempotency_key: Option<&str>,
    ) -> Result<HttpResponse, CafError> {
        self.send(
            method,
            path,
            body,
            custom_headers,
            idempotency_key.map(|s| s.to_string()),
        )
        .await
    }

    pub async fn send(
        &self,
        method: &str,
        path: &str,
        body: Option<Vec<u8>>,
        custom_headers: HashMap<String, String>,
        idempotency_key: Option<String>,
    ) -> Result<HttpResponse, CafError> {
        self.check_rate_limit()?;

        // Concurrency limiter guard
        let _permit = match &self.concurrency_semaphore {
            Some(sem) => Some(
                sem.acquire()
                    .await
                    .map_err(|_| CafError::Http(HttpError::Network))?,
            ),
            None => None,
        };

        let mut url = if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            let base = self.config.base_url.trim_end_matches('/');
            let sub = path.trim_start_matches('/');
            format!("{base}/{sub}")
        };

        let mut headers = custom_headers;
        if let Some(idem) = &idempotency_key {
            headers.insert("X-Idempotency-Key".to_string(), idem.clone());
        }

        // Apply authentication
        let secret = get_integration_secret(&self.config.key).unwrap_or(None);
        if let Some(sec) = secret {
            match &self.config.auth {
                AuthKind::None => {}
                AuthKind::ApiKeyHeader(hname) => {
                    headers.insert(hname.clone(), sec);
                }
                AuthKind::ApiKeyQuery(qname) => {
                    let sep = if url.contains('?') { "&" } else { "?" };
                    url = format!("{url}{sep}{qname}={sec}");
                }
                AuthKind::Bearer => {
                    headers.insert("Authorization".to_string(), format!("Bearer {sec}"));
                }
                AuthKind::Basic => {
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(sec.as_bytes());
                    headers.insert("Authorization".to_string(), format!("Basic {b64}"));
                }
                AuthKind::HmacSha256(hname) => {
                    if let Ok(mut mac) = HmacSha256::new_from_slice(sec.as_bytes()) {
                        let payload = body.as_deref().unwrap_or(&[]);
                        mac.update(payload);
                        let sig = hex::encode(mac.finalize().into_bytes());
                        headers.insert(hname.clone(), sig);
                    }
                }
                AuthKind::OAuth2ClientCredentials => {
                    headers.insert("Authorization".to_string(), format!("Bearer {sec}"));
                }
            }
        }

        let is_idempotent = matches!(
            method.to_ascii_uppercase().as_str(),
            "GET" | "HEAD" | "PUT" | "DELETE"
        ) || idempotency_key.is_some();

        let max_attempts = if is_idempotent {
            self.config.retries.saturating_add(1).max(1)
        } else {
            1
        };

        let req_template = HttpRequest {
            method: method.to_string(),
            url: url.clone(),
            headers: headers.clone(),
            body: body.clone(),
            timeout_secs: Some(self.config.timeout_ms / 1000),
            max_response_bytes: None,
            allowed_hosts: Some(self.config.allowed_hosts.clone()),
        };

        let start_time = Instant::now();
        let mut last_err = CafError::Http(HttpError::Network);

        for attempt in 0..max_attempts {
            if attempt > 0 {
                let backoff_ms = 50 * (1 << attempt);
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }

            match self.client.execute(req_template.clone()) {
                Ok(resp) => {
                    // Log sanitized request
                    let host = HttpClient::extract_host(&url).unwrap_or_default();
                    let scrubbed_path = redact_path_and_query(&url);
                    add_request_log(IntegrationRequestLogRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        integration_key: self.config.key.clone(),
                        method: method.to_string(),
                        host,
                        path: scrubbed_path,
                        status: resp.status,
                        duration_ms: resp.duration_ms,
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                    });
                    return Ok(resp);
                }
                Err(err) => {
                    last_err = err;
                    if !is_idempotent {
                        break;
                    }
                }
            }
        }

        // Log failed request
        let host = HttpClient::extract_host(&url).unwrap_or_default();
        let scrubbed_path = redact_path_and_query(&url);
        add_request_log(IntegrationRequestLogRecord {
            id: uuid::Uuid::new_v4().to_string(),
            integration_key: self.config.key.clone(),
            method: method.to_string(),
            host,
            path: scrubbed_path,
            status: 0,
            duration_ms: start_time.elapsed().as_millis() as u64,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        });

        Err(last_err)
    }
}
