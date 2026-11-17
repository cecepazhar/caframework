//! GCC built-in adapter on the connector layer (FR-15, P09.6).
//!
//! Enforces:
//! - Standard headers carrying:
//!     - `X-CA-App-Id`
//!     - `X-CA-App-Version`
//!     - `X-CA-Device-Id`
//! - Built on `Connector` / `HttpClient`.
//! - Contract matching `docs/contracts/gcc-billing-v1.yaml` and `gcc_mock`.

use crate::error::{CafError, HttpError};
use crate::http::HttpClient;
use crate::integrations::connector::{AuthKind, Connector, ConnectorConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const GCC_APP_ID_HEADER: &str = "X-CA-App-Id";
pub const GCC_APP_VERSION_HEADER: &str = "X-CA-App-Version";
pub const GCC_DEVICE_ID_HEADER: &str = "X-CA-Device-Id";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GccAccount {
    pub id: String,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GccAuthResponse {
    pub device_token: String,
    pub account: GccAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GccPlanItem {
    pub plan_id: String,
    pub billing_type: String,
    pub price_minor: u64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GccCheckoutResponse {
    pub checkout_id: String,
    pub checkout_url: String,
    pub completion_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GccDeviceItem {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    pub is_current: bool,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GccEntitlementResponse {
    pub token: String,
}

pub struct GccAdapter {
    connector: Connector,
    app_id: String,
    app_version: String,
    device_id: String,
}

impl GccAdapter {
    pub fn new(
        base_url: &str,
        app_id: &str,
        app_version: &str,
        device_id: &str,
        allowed_hosts: Vec<String>,
    ) -> Result<Self, CafError> {
        let mut hosts = allowed_hosts;
        if let Ok(host) = HttpClient::extract_host(base_url)
            && !hosts.contains(&host)
        {
            hosts.push(host);
        }

        let config = ConnectorConfig {
            key: "gcc".to_string(),
            kind: "gcc_billing".to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            user_configurable: false,
            auth: AuthKind::None,
            timeout_ms: 10_000,
            retries: 2,
            rate_limit_per_minute: Some(60),
            concurrency: Some(4),
            allowed_hosts: hosts,
        };

        let connector = Connector::new(config);
        Ok(Self {
            connector,
            app_id: app_id.to_string(),
            app_version: app_version.to_string(),
            device_id: device_id.to_string(),
        })
    }

    fn standard_headers(&self, auth_token: Option<&str>) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("Content-Type".to_string(), "application/json".to_string());
        map.insert(GCC_APP_ID_HEADER.to_string(), self.app_id.clone());
        map.insert(GCC_APP_VERSION_HEADER.to_string(), self.app_version.clone());
        map.insert(GCC_DEVICE_ID_HEADER.to_string(), self.device_id.clone());
        if let Some(tok) = auth_token {
            map.insert("Authorization".to_string(), format!("Bearer {tok}"));
        }
        map
    }

    /// Step 1: Request OTP email
    pub async fn request_otp(&self, email: &str) -> Result<(), CafError> {
        let payload = serde_json::json!({
            "email": email,
            "product_code": self.app_id,
        })
        .to_string();

        let resp = self
            .connector
            .request(
                "POST",
                "/v1/auth/otp/request",
                self.standard_headers(None),
                Some(payload.into_bytes()),
                None,
            )
            .await?;

        if resp.status == 202 || resp.status == 200 {
            Ok(())
        } else {
            Err(CafError::Http(HttpError::HttpStatus))
        }
    }

    /// Step 2: Verify OTP and register device
    pub async fn verify_otp(
        &self,
        email: &str,
        otp_code: &str,
        device_name: &str,
    ) -> Result<GccAuthResponse, CafError> {
        let payload = serde_json::json!({
            "email": email,
            "otp_code": otp_code,
            "product_code": self.app_id,
            "device_id": self.device_id,
            "device_name": device_name,
        })
        .to_string();

        let resp = self
            .connector
            .request(
                "POST",
                "/v1/auth/otp/verify",
                self.standard_headers(None),
                Some(payload.into_bytes()),
                None,
            )
            .await?;

        if resp.status == 200 || resp.status == 201 {
            serde_json::from_slice(&resp.body).map_err(|_| CafError::Http(HttpError::Network))
        } else {
            Err(CafError::Http(HttpError::HttpStatus))
        }
    }

    /// Activate license key directly
    pub async fn activate_license(
        &self,
        license_key: &str,
        device_name: &str,
    ) -> Result<GccAuthResponse, CafError> {
        let payload = serde_json::json!({
            "license_key": license_key,
            "device_id": self.device_id,
            "device_name": device_name,
        })
        .to_string();

        let resp = self
            .connector
            .request(
                "POST",
                "/v1/licenses/activate",
                self.standard_headers(None),
                Some(payload.into_bytes()),
                None,
            )
            .await?;

        if resp.status == 200 || resp.status == 201 {
            serde_json::from_slice(&resp.body).map_err(|_| CafError::Http(HttpError::Network))
        } else {
            Err(CafError::Http(HttpError::HttpStatus))
        }
    }

    /// Fetch latest entitlement JWS token
    pub async fn get_entitlement(&self, device_token: &str) -> Result<String, CafError> {
        let resp = self
            .connector
            .request(
                "GET",
                &format!("/v1/entitlement?device_id={}", self.device_id),
                self.standard_headers(Some(device_token)),
                None,
                None,
            )
            .await?;

        if resp.status == 200 {
            let res: GccEntitlementResponse = serde_json::from_slice(&resp.body)
                .map_err(|_| CafError::Http(HttpError::Network))?;
            Ok(res.token)
        } else {
            Err(CafError::Http(HttpError::HttpStatus))
        }
    }

    /// List available subscription plans
    pub async fn get_plans(&self) -> Result<Vec<GccPlanItem>, CafError> {
        let resp = self
            .connector
            .request(
                "GET",
                &format!("/v1/plans?product_code={}", self.app_id),
                self.standard_headers(None),
                None,
                None,
            )
            .await?;

        if resp.status == 200 || resp.status == 201 {
            serde_json::from_slice(&resp.body).map_err(|_| CafError::Http(HttpError::Network))
        } else {
            Err(CafError::Http(HttpError::HttpStatus))
        }
    }

    /// Create checkout session
    pub async fn create_checkout(
        &self,
        device_token: &str,
        plan_id: &str,
        email: &str,
    ) -> Result<GccCheckoutResponse, CafError> {
        let payload = serde_json::json!({
            "plan_id": plan_id,
            "email": email,
            "device_id": self.device_id,
        })
        .to_string();

        let resp = self
            .connector
            .request(
                "POST",
                "/v1/checkout",
                self.standard_headers(Some(device_token)),
                Some(payload.into_bytes()),
                None,
            )
            .await?;

        if resp.status == 200 || resp.status == 201 {
            serde_json::from_slice(&resp.body).map_err(|_| CafError::Http(HttpError::Network))
        } else {
            Err(CafError::Http(HttpError::HttpStatus))
        }
    }

    /// List registered devices for current account
    pub async fn list_devices(&self, device_token: &str) -> Result<Vec<GccDeviceItem>, CafError> {
        let resp = self
            .connector
            .request(
                "GET",
                "/v1/devices",
                self.standard_headers(Some(device_token)),
                None,
                None,
            )
            .await?;

        if resp.status == 200 || resp.status == 201 {
            serde_json::from_slice(&resp.body).map_err(|_| CafError::Http(HttpError::Network))
        } else {
            Err(CafError::Http(HttpError::HttpStatus))
        }
    }

    /// Deactivate / revoke device
    pub async fn deactivate_device(
        &self,
        device_token: &str,
        target_device_id: &str,
    ) -> Result<(), CafError> {
        let resp = self
            .connector
            .request(
                "DELETE",
                &format!("/v1/devices/{target_device_id}"),
                self.standard_headers(Some(device_token)),
                None,
                None,
            )
            .await?;

        if resp.status == 200 || resp.status == 204 {
            Ok(())
        } else {
            Err(CafError::Http(HttpError::HttpStatus))
        }
    }
}
