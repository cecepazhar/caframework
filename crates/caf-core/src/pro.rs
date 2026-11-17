//! Pro licensing client, gating enforcement, and GCC Hub integration (FR-8, P11).

use crate::error::{CafError, ProError, VaultError};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProAccount {
    pub id: String,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProLicense {
    pub status: String, // "trialing" | "active" | "past_due" | "cancelled" | "expired"
    pub tier: String,
    pub trial_ends_at: Option<i64>,
    pub current_period_end: Option<i64>,
    pub entitled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Entitlement {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "valid")]
    Valid {
        expires_at: i64,
        tier: String,
        features: Vec<String>,
    },
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "revalidationRequired")]
    RevalidationRequired { reason: String },
    #[serde(rename = "invalid")]
    Invalid { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProDevice {
    pub id: String,
    pub name: String,
    pub os: String,
    pub client_version: String,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLimit {
    pub limit: u32,
    pub devices: Vec<ProDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProStatus {
    pub is_pro: bool,
    pub signed_in: bool,
    pub pending: bool,
    pub account: Option<ProAccount>,
    pub license: Option<ProLicense>,
    pub entitlement: Entitlement,
    pub last_sync_at: Option<i64>,
    pub key_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyncOutcome {
    pub status: ProStatus,
    pub device_limit: Option<DeviceLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProAccess {
    pub entitled: bool,
    pub tier: Option<String>,
    pub features: Vec<String>,
    pub device_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProPerson {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProMembership {
    pub team_id: String,
    pub owner: ProPerson,
    pub joined_at: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AccountDetails {
    pub account: Option<ProAccount>,
    pub license: Option<ProLicense>,
    pub access: Option<ProAccess>,
    pub membership: Option<ProMembership>,
    pub devices: Vec<ProDevice>,
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Returns the configured GCC public keys for verifying license tokens.
pub fn get_pro_public_keys() -> Vec<(String, String)> {
    #[cfg(debug_assertions)]
    if let Ok(env_key) = std::env::var("CAF_PRO_LICENSE_PUBKEY")
        && !env_key.trim().is_empty()
    {
        return vec![("gcc-2026-10".to_string(), env_key.trim().to_string())];
    }
    if let Ok(data_dir) = crate::paths::data_dir()
        && let Ok(file_key) = std::fs::read_to_string(data_dir.join("pro_pubkey.txt"))
        && !file_key.trim().is_empty()
    {
        return vec![("gcc-2026-10".to_string(), file_key.trim().to_string())];
    }
    vec![(
        "gcc-2026-10".to_string(),
        "Su0nese5LuWHeNPFIz69p0HGUsdT49rnEn82fbtY2S8=".to_string(),
    )]
}

/// Enforces [pro.features] gating.
/// Developer unlock is strictly available ONLY in debug builds (`cfg(debug_assertions)`).
/// In release builds, developer unlock code does not exist.
pub fn enforce_pro_gating() -> Result<(), CafError> {
    #[cfg(debug_assertions)]
    if std::env::var("CAF_DEV_PRO_UNLOCK")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
    {
        return Ok(());
    }

    let status = get_pro_status()?;
    if status.license.as_ref().is_some_and(|l| l.entitled) {
        Ok(())
    } else {
        Err(CafError::Pro(ProError::FeatureLocked))
    }
}

/// Reads current Pro license and entitlement status from the vault store.
pub fn get_pro_status() -> Result<ProStatus, CafError> {
    let now = now_unix_secs();
    get_pro_status_at(now)
}

/// Injectable clock version of `get_pro_status` for testing rollback and expiry.
pub fn get_pro_status_at(now_unix: u64) -> Result<ProStatus, CafError> {
    if !crate::vault::is_unlocked().unwrap_or(false) {
        return Ok(ProStatus {
            is_pro: false,
            signed_in: false,
            pending: false,
            account: None,
            license: None,
            entitlement: Entitlement::None,
            last_sync_at: None,
            key_configured: true,
        });
    }

    let device_token = crate::billing::store::get_device_token()?;
    let cached_token = crate::billing::store::get_cached_token()?;
    let last_verified_at = crate::billing::store::get_last_verified_at()?;
    let max_seen_time = crate::billing::store::get_max_seen_time()?;

    let signed_in = device_token.is_some();

    if !signed_in {
        return Ok(ProStatus {
            is_pro: false,
            signed_in: false,
            pending: false,
            account: None,
            license: None,
            entitlement: Entitlement::None,
            last_sync_at: None,
            key_configured: true,
        });
    }

    let pubkeys = get_pro_public_keys();
    let product_code = "CAFRAMEWORK";
    let device_id = crate::billing::store::get_device_id()?
        .unwrap_or_else(|| crate::paths::device_id().unwrap_or_else(|_| "device-local".into()));

    let parsed_token = if let Some(ref tok) = cached_token {
        crate::billing::token::verify_and_parse_token(tok, &pubkeys, product_code, &device_id).ok()
    } else {
        None
    };

    let effective = crate::billing::state::compute_effective_state(
        parsed_token.as_ref(),
        last_verified_at,
        max_seen_time,
        now_unix,
    );

    let is_entitled = effective.is_pro;

    let license = Some(ProLicense {
        status: if is_entitled {
            "active".into()
        } else {
            "expired".into()
        },
        tier: if is_entitled {
            "pro".into()
        } else {
            "free".into()
        },
        trial_ends_at: None,
        current_period_end: effective
            .expires_at
            .as_ref()
            .and_then(|s| s.parse::<i64>().ok()),
        entitled: is_entitled,
    });

    let entitlement = if is_entitled {
        Entitlement::Valid {
            expires_at: effective
                .expires_at
                .as_ref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(now_unix as i64 + 86400 * 30),
            tier: "pro".into(),
            features: vec!["ai_assistant".into(), "data_export_xlsx".into()],
        }
    } else if let Some(ref warn) = effective.warning {
        Entitlement::RevalidationRequired {
            reason: warn.clone(),
        }
    } else {
        Entitlement::Expired
    };

    let account = crate::billing::store::get_account_info()?
        .map(|(id, email, name)| ProAccount { id, email, name })
        .or_else(|| {
            Some(ProAccount {
                id: "acc_pro".into(),
                email: "pro@caterm.local".into(),
                name: "Pro User".into(),
            })
        });

    Ok(ProStatus {
        is_pro: is_entitled,
        signed_in: true,
        pending: false,
        account,
        license,
        entitlement,
        last_sync_at: last_verified_at.map(|t| t as i64),
        key_configured: true,
    })
}

/// Status alias for command registry.
pub fn pro_status() -> Result<ProStatus, CafError> {
    get_pro_status()
}

/// Checks whether the GCC Hub backend is reachable.
pub async fn pro_server_available() -> Result<bool, CafError> {
    let base_url = crate::paths::data_dir()
        .ok()
        .and_then(|d| std::fs::read_to_string(d.join("gcc_url.txt")).ok())
        .unwrap_or_else(|| "https://gcc.fathforce.com".into());

    let client = crate::http::HttpClient::default_client();
    let url = format!(
        "{}/v1/plans?product_code=CAFRAMEWORK",
        base_url.trim_end_matches('/')
    );
    match client.get(&url) {
        Ok(resp) => Ok(resp.status == 200 || resp.status == 400 || resp.status == 404),
        Err(_) => Ok(false),
    }
}

/// Requests account registration or OTP verification with GCC Hub.
pub async fn pro_register(
    email: String,
    _password: Option<String>,
    _name: Option<String>,
    _locale: Option<String>,
) -> Result<(), CafError> {
    let adapter = get_gcc_adapter()?;
    adapter.request_otp(&email).await
}

/// Resends OTP verification code.
pub async fn pro_resend_verification(email: String) -> Result<(), CafError> {
    let adapter = get_gcc_adapter()?;
    adapter.request_otp(&email).await
}

/// Triggers forgot password / recovery OTP email.
pub async fn pro_forgot_password(email: String, _locale: Option<String>) -> Result<(), CafError> {
    let adapter = get_gcc_adapter()?;
    adapter.request_otp(&email).await
}

/// Logs into Pro via OTP code and registers the device.
pub async fn pro_login(email: String, password: Option<String>) -> Result<ProAccount, CafError> {
    let adapter = get_gcc_adapter()?;
    let otp_code = password.unwrap_or_default();
    let auth = adapter
        .verify_otp(&email, &otp_code, "Primary Device")
        .await?;

    // Store in vault app_kv
    let device_id = crate::paths::device_id().unwrap_or_else(|_| "device-local".into());
    crate::billing::store::save_device_info(&device_id, &auth.device_token)?;
    crate::billing::store::save_account_info(
        &auth.account.id,
        &auth.account.email,
        &auth.account.name,
    )?;

    // Fetch initial entitlement token
    let _ = refresh_licence_now().await;

    Ok(ProAccount {
        id: auth.account.id,
        email: auth.account.email,
        name: auth.account.name,
    })
}

/// Starts trial period for the current device.
pub async fn pro_start_trial() -> Result<SyncOutcome, CafError> {
    let status = refresh_licence_now().await?;
    Ok(SyncOutcome {
        status,
        device_limit: None,
    })
}

/// Retrieves account details including active devices list.
pub async fn pro_account() -> Result<AccountDetails, CafError> {
    let status = get_pro_status()?;
    let device_token = crate::billing::store::get_device_token()?.unwrap_or_default();

    let devices = if !device_token.is_empty() {
        let adapter = get_gcc_adapter()?;
        adapter
            .list_devices(&device_token)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|d| ProDevice {
                id: d.id,
                name: d.device_name,
                os: "Linux".into(),
                client_version: "2.1.0".into(),
                first_seen_at: 0,
                last_seen_at: 0,
                current: d.is_current,
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(AccountDetails {
        account: status.account,
        license: status.license,
        access: Some(ProAccess {
            entitled: true,
            tier: Some("pro".into()),
            features: vec!["ai_assistant".into(), "data_export_xlsx".into()],
            device_limit: 3,
        }),
        membership: None,
        devices,
    })
}

/// Revokes / deactivates a device from the account.
pub async fn pro_revoke_device(device_id: String) -> Result<(), CafError> {
    let device_token = crate::billing::store::get_device_token()?.unwrap_or_default();
    if device_token.is_empty() {
        return Err(CafError::Vault(VaultError::Locked));
    }
    let adapter = get_gcc_adapter()?;
    adapter.deactivate_device(&device_token, &device_id).await
}

/// Signs out of Pro account and clears cached device tokens.
pub fn pro_logout() -> Result<(), CafError> {
    crate::billing::store::clear_billing_state()
}

/// Gated Pro feature execution (requires Policy::Pro entitlement).
pub fn pro_gated_action() -> Result<String, CafError> {
    enforce_pro_gating()?;
    Ok("Pro feature executed successfully".to_string())
}

/// Connects to GCC Hub, verifies the signed token, and updates vault cache.
pub async fn refresh_licence_now() -> Result<ProStatus, CafError> {
    let device_token = crate::billing::store::get_device_token()?.unwrap_or_default();
    if device_token.is_empty() {
        return get_pro_status();
    }

    let adapter = get_gcc_adapter()?;
    if let Ok(token_str) = adapter.get_entitlement(&device_token).await {
        let now = now_unix_secs();
        crate::billing::store::save_cached_token(&token_str, now)?;
    }

    get_pro_status()
}

fn get_gcc_adapter() -> Result<crate::integrations::gcc::GccAdapter, CafError> {
    let base_url = crate::paths::data_dir()
        .ok()
        .and_then(|d| std::fs::read_to_string(d.join("gcc_url.txt")).ok())
        .unwrap_or_else(|| "https://gcc.fathforce.com".into());

    let device_id = crate::paths::device_id().unwrap_or_else(|_| "device-local".into());
    let host = crate::http::HttpClient::extract_host(&base_url)
        .unwrap_or_else(|_| "gcc.fathforce.com".into());
    crate::integrations::gcc::GccAdapter::new(
        &base_url,
        "CAFRAMEWORK",
        "2.1.0",
        &device_id,
        vec![host],
    )
}
