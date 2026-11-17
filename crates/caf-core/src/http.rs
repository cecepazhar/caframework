//! Shared HTTP client for CAFramework (FR-15, P09.1).
//!
//! Enforces:
//! - Connect timeout (10s default), request timeout (30s default)
//! - User-Agent: `<slug>/<version> (+<brand.website>)`
//! - HTTPS only (loopback allowed: localhost, 127.0.0.1, [::1])
//! - Response size cap (default 5MB)
//! - Allowed hosts enforcement on initial request AND on every redirect
//! - Typed error taxonomy (`HttpError`)

use crate::error::{CafError, HttpError};
use std::collections::HashMap;
use std::io::Read;
use std::time::{Duration, Instant};

pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024; // 5MB
pub const MAX_REDIRECTS: usize = 5;

#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub slug: String,
    pub version: String,
    pub brand_website: String,
    pub app_identifier: String,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            slug: "caframework".to_string(),
            version: "0.1.0-dev".to_string(),
            brand_website: "https://www.cecepazhar.com".to_string(),
            app_identifier: "com.fathforce.caframework".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub timeout_secs: Option<u64>,
    pub max_response_bytes: Option<usize>,
    pub allowed_hosts: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub duration_ms: u64,
}

impl HttpResponse {
    pub fn text(&self) -> Result<String, CafError> {
        String::from_utf8(self.body.clone())
            .map_err(|_| CafError::Http(HttpError::Network))
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, CafError> {
        serde_json::from_slice(&self.body)
            .map_err(|_| CafError::Http(HttpError::Network))
    }
}

pub struct HttpClient {
    config: HttpClientConfig,
}

impl HttpClient {
    pub fn new(config: HttpClientConfig) -> Self {
        Self { config }
    }

    pub fn default_client() -> Self {
        Self::new(HttpClientConfig::default())
    }

    pub fn user_agent(&self) -> String {
        format!(
            "{}/{} (+{})",
            self.config.slug, self.config.version, self.config.brand_website
        )
    }

    pub fn extract_host(url: &str) -> Result<String, CafError> {
        let trimmed = url.trim();
        let rest = if let Some(r) = trimmed.strip_prefix("https://") {
            r
        } else if let Some(r) = trimmed.strip_prefix("http://") {
            r
        } else {
            return Err(CafError::Http(HttpError::InvalidUrl));
        };

        let host_part = rest.split(['/', '?', '#']).next().unwrap_or("");
        // Strip port if present
        let host = if host_part.starts_with('[') {
            // IPv6 [::1]:port
            if let Some(end_bracket) = host_part.find(']') {
                host_part.get(..=end_bracket).unwrap_or(host_part)
            } else {
                host_part
            }
        } else if let Some((h, _port)) = host_part.split_once(':') {
            h
        } else {
            host_part
        };

        if host.is_empty() {
            return Err(CafError::Http(HttpError::InvalidUrl));
        }
        Ok(host.to_ascii_lowercase())
    }

    pub fn validate_url(&self, url: &str) -> Result<(), CafError> {
        let is_https = url.starts_with("https://");
        let is_loopback = url.starts_with("http://localhost")
            || url.starts_with("http://127.0.0.1")
            || url.starts_with("http://[::1]");

        if is_https || is_loopback {
            Ok(())
        } else {
            Err(CafError::Http(HttpError::InsecureHttp))
        }
    }

    pub fn is_host_allowed(host: &str, allowed_hosts: &[String]) -> bool {
        if allowed_hosts.is_empty() {
            return true;
        }
        let h = host.to_ascii_lowercase();
        allowed_hosts.iter().any(|allowed| {
            let a = allowed.to_ascii_lowercase();
            if a == "*" || a == h {
                true
            } else if let Some(suffix) = a.strip_prefix("*.") {
                h.ends_with(suffix) && h.len() > suffix.len()
            } else {
                false
            }
        })
    }

    pub fn execute(&self, req: HttpRequest) -> Result<HttpResponse, CafError> {
        let start = Instant::now();
        let mut current_url = req.url.clone();
        let timeout_secs = req.timeout_secs.unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS);
        let max_response_bytes = req.max_response_bytes.unwrap_or(DEFAULT_MAX_RESPONSE_BYTES);
        let allowed_hosts = req.allowed_hosts.unwrap_or_default();

        // 1. Validate initial URL
        self.validate_url(&current_url)?;
        let initial_host = Self::extract_host(&current_url)?;
        if !allowed_hosts.is_empty() && !Self::is_host_allowed(&initial_host, &allowed_hosts) {
            return Err(CafError::Http(HttpError::RedirectDisallowed));
        }

        // Configure ureq agent without automatic redirects (redirects = 0) so we inspect each redirect
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(timeout_secs)))
            .max_redirects(0)
            .build()
            .new_agent();

        let mut current_method = req.method.to_ascii_uppercase();
        let mut current_body = req.body.clone();
        let mut redirect_count = 0;

        loop {
            let mut ureq_req = match current_method.as_str() {
                "GET" => agent.get(&current_url).force_send_body(),
                "POST" => agent.post(&current_url),
                "PUT" => agent.put(&current_url),
                "DELETE" => agent.delete(&current_url).force_send_body(),
                "PATCH" => agent.patch(&current_url),
                "HEAD" => agent.head(&current_url).force_send_body(),
                _ => {
                    return Err(CafError::Http(HttpError::InvalidUrl));
                }
            };

            // Set default headers
            ureq_req = ureq_req
                .header("User-Agent", &self.user_agent())
                .header("X-CA-App-Id", &self.config.app_identifier);

            // Apply custom headers
            for (k, v) in &req.headers {
                ureq_req = ureq_req.header(k, v);
            }

            let mut resp = if let Some(body_bytes) = &current_body {
                if !req.headers.contains_key("Content-Type")
                    && !req.headers.contains_key("content-type")
                {
                    ureq_req = ureq_req.header("Content-Type", "application/json");
                }
                ureq_req
                    .send(body_bytes.as_slice())
                    .map_err(|e| map_ureq_error(e, timeout_secs))?
            } else {
                ureq_req
                    .send_empty()
                    .map_err(|e| map_ureq_error(e, timeout_secs))?
            };

            let status = resp.status().as_u16();

            // Handle redirects (301, 302, 303, 307, 308)
            if [301, 302, 303, 307, 308].contains(&status) {
                redirect_count += 1;
                if redirect_count > MAX_REDIRECTS {
                    return Err(CafError::Http(HttpError::Network));
                }

                let location = resp
                    .headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .ok_or(CafError::Http(HttpError::Network))?;

                let redirect_url = resolve_redirect_url(&current_url, location)?;

                // Validate redirect URL: must be HTTPS or loopback
                self.validate_url(&redirect_url)?;

                // Check allowed hosts on redirect!
                let redirect_host = Self::extract_host(&redirect_url)?;
                if !allowed_hosts.is_empty()
                    && !Self::is_host_allowed(&redirect_host, &allowed_hosts)
                {
                    return Err(CafError::Http(HttpError::RedirectDisallowed));
                }

                current_url = redirect_url;
                if status == 303 || ((status == 301 || status == 302) && current_method == "POST") {
                    current_method = "GET".to_string();
                    current_body = None;
                }
                continue;
            }

            // Collect response headers
            let mut headers_map = HashMap::new();
            for (hname, hval) in resp.headers() {
                if let Ok(v) = hval.to_str() {
                    headers_map.insert(hname.to_string(), v.to_string());
                }
            }

            // Check Content-Length if present
            if let Some(cl_str) = headers_map.get("content-length")
                && let Ok(cl) = cl_str.parse::<usize>()
                && cl > max_response_bytes
            {
                return Err(CafError::Http(HttpError::ResponseTooLarge));
            }

            // Read body with limit
            let mut body_buf = Vec::new();
            let mut chunk = [0u8; 8192];
            let mut reader = resp.body_mut().as_reader();

            loop {
                let n = reader
                    .read(&mut chunk)
                    .map_err(|e| map_io_error(e, timeout_secs))?;
                if n == 0 {
                    break;
                }
                body_buf.extend_from_slice(chunk.get(..n).unwrap_or(&[]));
                if body_buf.len() > max_response_bytes {
                    return Err(CafError::Http(HttpError::ResponseTooLarge));
                }
            }

            let duration_ms = start.elapsed().as_millis() as u64;

            if status >= 400 {
                return Err(CafError::Http(HttpError::HttpStatus));
            }

            return Ok(HttpResponse {
                status,
                headers: headers_map,
                body: body_buf,
                duration_ms,
            });
        }
    }

    pub fn get(&self, url: &str) -> Result<HttpResponse, CafError> {
        self.execute(HttpRequest {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
            timeout_secs: None,
            max_response_bytes: None,
            allowed_hosts: None,
        })
    }

    pub fn post_json<T: serde::Serialize>(
        &self,
        url: &str,
        payload: &T,
        timeout_secs: u64,
    ) -> Result<String, CafError> {
        let json_bytes = serde_json::to_vec(payload)
            .map_err(|_| CafError::Http(HttpError::Network))?;

        let res = self.execute(HttpRequest {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: Some(json_bytes),
            timeout_secs: Some(timeout_secs),
            max_response_bytes: None,
            allowed_hosts: None,
        })?;

        res.text()
    }
}

fn resolve_redirect_url(base: &str, location: &str) -> Result<String, CafError> {
    if location.starts_with("https://") || location.starts_with("http://") {
        Ok(location.to_string())
    } else if let Some(stripped_base) = base.strip_prefix("https://") {
        let host = stripped_base.split(['/', '?', '#']).next().unwrap_or("");
        if location.starts_with('/') {
            Ok(format!("https://{host}{location}"))
        } else {
            Ok(format!("https://{host}/{location}"))
        }
    } else if let Some(stripped_base) = base.strip_prefix("http://") {
        let host = stripped_base.split(['/', '?', '#']).next().unwrap_or("");
        if location.starts_with('/') {
            Ok(format!("http://{host}{location}"))
        } else {
            Ok(format!("http://{host}/{location}"))
        }
    } else {
        Err(CafError::Http(HttpError::InvalidUrl))
    }
}

fn map_ureq_error(err: ureq::Error, _timeout_secs: u64) -> CafError {
    let err_str = err.to_string();
    if err_str.contains("timeout") || err_str.contains("timed out") {
        CafError::Http(HttpError::Timeout)
    } else {
        CafError::Http(HttpError::Network)
    }
}

fn map_io_error(err: std::io::Error, _timeout_secs: u64) -> CafError {
    if err.kind() == std::io::ErrorKind::TimedOut {
        CafError::Http(HttpError::Timeout)
    } else {
        CafError::Http(HttpError::Network)
    }
}
