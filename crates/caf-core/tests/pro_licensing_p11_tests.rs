//! Comprehensive test suite for P11: Pro Licensing Client, Token Verification, and Core Gating (FR-8).
//! Tests:
//! - P11.1: Licensing commands against gcc-mock.
//! - P11.2: Ed25519 token verification, tamper rejection, clock rollback detection, and offline grace limits.
//! - P11.3: Core policy gating (Policy::Pro / ProError::FeatureLocked) and dev unlock guardrails.
//! - P11.4: Licence refresh scheduler job.

use base64::Engine;
use caf_core::api::policy::guarded;
use caf_core::billing::store;
use caf_core::billing::token::verify_and_parse_token;
use caf_core::error::{CafError, ProError};
use caf_core::pro::*;
use caf_core::session::{Session, set_current_session};
use caf_core::vault;
use caf_xtask::gcc_mock::{MockState, create_mock_app, init_dev_keys};
use ed25519_dalek::{Signer, SigningKey};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct TestEnv {
    dir: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl TestEnv {
    fn new(label: &str) -> Self {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        vault::lock();
        caf_core::session::clear_current_session().ok();
        let dir = std::env::temp_dir().join(format!("caf_p11_{label}_{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::set_var("CAF_DATA_DIR", &dir);
        }
        vault::setup_vault("test-master-password-123").expect("setup vault");
        Self { dir, _guard: guard }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        vault::lock();
        caf_core::session::clear_current_session().ok();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn set_test_session(profile_id: &str, role: &str) {
    let mut sess = Session::new(profile_id, role);
    sess.permissions.insert("*".to_string());
    set_current_session(sess).expect("set session");
}

fn generate_test_token(
    key_bytes: &[u8; 32],
    kid: &str,
    product_code: &str,
    device_id: &str,
    tier: &str,
    status: &str,
    expires_at: Option<&str>,
) -> String {
    let signing_key = SigningKey::from_bytes(key_bytes);
    let header = serde_json::json!({
        "alg": "EdDSA",
        "typ": "JWT",
        "kid": kid
    });
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&header).expect("encode header"));

    let payload = serde_json::json!({
        "v": 1,
        "kid": kid,
        "account_id": "acc-123",
        "product_code": product_code,
        "device_id": device_id,
        "tier": tier,
        "billing_type": "yearly",
        "status": status,
        "expires_at": expires_at,
        "grace_until": None::<String>,
        "issued_at": "2026-11-17T00:00:00Z"
    });
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&payload).expect("encode payload"));

    let signing_input = format!("{header_b64}.{payload_b64}");
    let sig = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes());

    format!("{signing_input}.{sig_b64}")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_p11_1_licensing_commands_against_gcc_mock() {
    let env = TestEnv::new("p11_1_commands");
    set_test_session("profile-p11-user", "admin");

    let (signing_key, verifying_key, kid) = init_dev_keys();
    let pub_key_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.as_bytes());

    // Configure public key and mock URL in test data dir
    fs::write(env.dir.join("pro_pubkey.txt"), &pub_key_b64).expect("write pubkey");

    let state = MockState {
        signing_key: Arc::new(signing_key),
        verifying_key,
        kid,
        accounts: Arc::new(Mutex::new(HashMap::new())),
        devices: Arc::new(Mutex::new(HashMap::new())),
        checkouts: Arc::new(Mutex::new(HashMap::new())),
        license_keys: Arc::new(Mutex::new(HashMap::new())),
        scenario_bad_signature: Arc::new(Mutex::new(false)),
        scenario_error_500: Arc::new(Mutex::new(false)),
    };

    let app = create_mock_app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dynamic port");
    let port = listener.local_addr().expect("local addr").port();
    fs::write(
        env.dir.join("gcc_url.txt"),
        format!("http://127.0.0.1:{port}"),
    )
    .expect("write gcc url");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock app");
    });

    // 1. pro_server_available
    let available = pro_server_available().await.expect("pro_server_available");
    assert!(available, "mock GCC hub server should be reachable");

    // 2. pro_register
    let reg = pro_register(
        "pro_tester@example.com".into(),
        None,
        Some("Pro Tester".into()),
        Some("en".into()),
    )
    .await;
    assert!(reg.is_ok(), "pro_register should succeed");

    // 3. pro_resend_verification
    let resend = pro_resend_verification("pro_tester@example.com".into()).await;
    assert!(resend.is_ok(), "pro_resend_verification should succeed");

    // 4. pro_forgot_password
    let forgot = pro_forgot_password("pro_tester@example.com".into(), Some("en".into())).await;
    assert!(forgot.is_ok(), "pro_forgot_password should succeed");

    // 5. pro_login (verifies OTP and stores device info in vault)
    let login = pro_login("pro_tester@example.com".into(), Some("123456".into()))
        .await
        .expect("pro_login should succeed");
    assert_eq!(login.email, "pro_tester@example.com");

    // 6. pro_status (should report signed_in = true)
    let status = pro_status().expect("pro_status should succeed");
    assert!(status.signed_in, "should be signed in after login");
    assert!(status.key_configured, "key should be configured");

    // 7. pro_account (should retrieve account details)
    let acc_details = pro_account().await.expect("pro_account should succeed");
    assert!(
        acc_details.account.is_some(),
        "account details should be present"
    );

    // 8. pro_start_trial
    let trial = pro_start_trial()
        .await
        .expect("pro_start_trial should succeed");
    assert!(trial.status.signed_in);

    // 9. pro_revoke_device
    let revoke = pro_revoke_device("dev-sample".into()).await;
    assert!(revoke.is_ok(), "pro_revoke_device should succeed");

    // 10. pro_logout (should clear billing state from vault)
    let logout = pro_logout();
    assert!(logout.is_ok(), "pro_logout should succeed");

    let status_after_logout = pro_status().expect("pro_status after logout");
    assert!(
        !status_after_logout.signed_in,
        "signed_in should be false after logout"
    );
}

#[test]
fn test_p11_2_token_verification_and_tamper_detection() {
    let env = TestEnv::new("p11_2_token");
    set_test_session("profile-p11-token", "admin");

    let (signing_key, verifying_key, kid) = init_dev_keys();
    let pub_key_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.as_bytes());
    let pubkeys = vec![(kid.clone(), pub_key_b64.clone())];

    fs::write(env.dir.join("pro_pubkey.txt"), &pub_key_b64).expect("write pubkey");

    let valid_token = generate_test_token(
        &signing_key.to_bytes(),
        &kid,
        "CAFRAMEWORK",
        "dev-123",
        "pro",
        "active",
        Some("2026-12-31T23:59:59Z"),
    );

    // 1. Valid token passes verification
    let parsed = verify_and_parse_token(&valid_token, &pubkeys, "CAFRAMEWORK", "dev-123");
    assert!(parsed.is_ok(), "valid token must verify successfully");
    let payload = parsed.expect("parsed payload");
    assert_eq!(payload.tier, "pro");
    assert_eq!(payload.status, "active");

    // 2. Tampered signature rejected
    let mut tampered = valid_token.clone();
    tampered.push_str("extra_corrupt_bytes");
    let verify_tampered = verify_and_parse_token(&tampered, &pubkeys, "CAFRAMEWORK", "dev-123");
    assert!(
        verify_tampered.is_err(),
        "tampered signature must be rejected"
    );

    // 3. Device ID or Product Code mismatch rejected
    let verify_wrong_dev =
        verify_and_parse_token(&valid_token, &pubkeys, "CAFRAMEWORK", "wrong-device");
    assert!(
        verify_wrong_dev.is_err(),
        "device mismatch must be rejected"
    );

    let verify_wrong_prod = verify_and_parse_token(&valid_token, &pubkeys, "OTHER_APP", "dev-123");
    assert!(
        verify_wrong_prod.is_err(),
        "product code mismatch must be rejected"
    );

    // 4. Clock rollback detection via get_pro_status_at
    store::save_device_info("dev-123", "tok-test").expect("save dev info");
    store::save_cached_token(&valid_token, 1_000_000).expect("save token");

    // Normal state at time 1_000_000
    let normal_status = get_pro_status_at(1_000_000).expect("normal status");
    assert!(normal_status.signed_in);

    // Rollback clock by 2 hours (< max_seen_time - 600s tolerance)
    let rollback_time = 1_000_000 - 7200;
    let rollback_status = get_pro_status_at(rollback_time).expect("rollback status");
    match rollback_status.entitlement {
        Entitlement::RevalidationRequired { reason } => {
            assert!(
                reason.contains("Clock rollback"),
                "expected clock rollback warning, got: {reason}"
            );
        }
        other => panic!("expected RevalidationRequired on clock rollback, got: {other:?}"),
    }

    // 5. Offline grace expiry (> 7 days without verification)
    let offline_expired_time = 1_000_000 + (8 * 86400); // 8 days later
    let expired_grace_status =
        get_pro_status_at(offline_expired_time).expect("expired grace status");
    match expired_grace_status.entitlement {
        Entitlement::RevalidationRequired { reason } => {
            assert!(
                reason.contains("Offline allowance exceeded"),
                "expected offline allowance warning, got: {reason}"
            );
        }
        other => panic!("expected RevalidationRequired on offline expiry, got: {other:?}"),
    }
}

#[test]
fn test_p11_3_gating_in_core_and_dev_unlock() {
    let env = TestEnv::new("p11_3_gating");
    set_test_session("profile-p11-gate", "admin");

    // 1. Without Pro entitlement, enforce_pro_gating fails with FeatureLocked
    let gate_res = enforce_pro_gating();
    assert!(
        matches!(gate_res, Err(CafError::Pro(ProError::FeatureLocked))),
        "expected ProError::FeatureLocked, got: {gate_res:?}"
    );

    // 2. Command guarded with Policy::Pro fails
    let cmd_res = guarded("pro_gated_action", pro_gated_action);
    assert!(
        matches!(cmd_res, Err(CafError::Pro(ProError::FeatureLocked))),
        "pro_gated_action must be blocked without Pro license"
    );

    // 3. Developer unlock test (only in debug assertions)
    #[cfg(debug_assertions)]
    {
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::set_var("CAF_DEV_PRO_UNLOCK", "1");
        }

        let dev_unlocked = enforce_pro_gating();
        assert!(
            dev_unlocked.is_ok(),
            "CAF_DEV_PRO_UNLOCK=1 must unlock in debug builds"
        );

        let dev_cmd_res = guarded("pro_gated_action", pro_gated_action);
        assert!(
            dev_cmd_res.is_ok(),
            "pro_gated_action must succeed when dev unlocked"
        );

        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::remove_var("CAF_DEV_PRO_UNLOCK");
        }
    }

    // 4. Valid Pro token grants access
    let (signing_key, verifying_key, kid) = init_dev_keys();
    let pub_key_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.as_bytes());
    fs::write(env.dir.join("pro_pubkey.txt"), &pub_key_b64).expect("write pubkey");

    let token = generate_test_token(
        &signing_key.to_bytes(),
        &kid,
        "CAFRAMEWORK",
        "dev-active",
        "pro",
        "active",
        Some("2030-01-01T00:00:00Z"),
    );

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    store::save_device_info("dev-active", "tok-active").expect("save dev info");
    store::save_cached_token(&token, now).expect("save token");

    let status = get_pro_status().expect("status with token");
    assert!(
        status.license.as_ref().is_some_and(|l| l.entitled),
        "license should be entitled"
    );

    let gated_ok = pro_gated_action();
    assert!(
        gated_ok.is_ok(),
        "pro_gated_action must succeed with valid pro token"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_p11_4_scheduler_job_licence_refresh() {
    let env = TestEnv::new("p11_4_scheduler");
    set_test_session("profile-p11-sched", "admin");

    let (signing_key, verifying_key, kid) = init_dev_keys();
    let pub_key_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.as_bytes());
    fs::write(env.dir.join("pro_pubkey.txt"), &pub_key_b64).expect("write pubkey");

    let state = MockState {
        signing_key: Arc::new(signing_key),
        verifying_key,
        kid,
        accounts: Arc::new(Mutex::new(HashMap::new())),
        devices: Arc::new(Mutex::new(HashMap::new())),
        checkouts: Arc::new(Mutex::new(HashMap::new())),
        license_keys: Arc::new(Mutex::new(HashMap::new())),
        scenario_bad_signature: Arc::new(Mutex::new(false)),
        scenario_error_500: Arc::new(Mutex::new(false)),
    };

    let app = create_mock_app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dynamic port");
    let port = listener.local_addr().expect("local addr").port();
    fs::write(
        env.dir.join("gcc_url.txt"),
        format!("http://127.0.0.1:{port}"),
    )
    .expect("write gcc url");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock app");
    });

    // Login user to obtain device token
    let login = pro_login("sched_user@example.com".into(), Some("123456".into()))
        .await
        .expect("pro_login should succeed");
    assert_eq!(login.email, "sched_user@example.com");

    // Execute refresh_licence_now (which scheduler builtin job pro.licence_refresh invokes)
    let refreshed = refresh_licence_now()
        .await
        .expect("refresh_licence_now should succeed");
    assert!(refreshed.signed_in, "should remain signed in after refresh");

    // Verify scheduler job is registered
    let jobs = caf_core::scheduler::builtin::make_builtin_jobs();
    let pro_job = jobs.iter().find(|j| j.key == "pro.licence_refresh");
    assert!(
        pro_job.is_some(),
        "pro.licence_refresh job must be registered in scheduler"
    );
}
