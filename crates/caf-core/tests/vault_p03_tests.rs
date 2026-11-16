//! P03 Acceptance Tests: Vault, Keyring, and Authentication Back-off.

use caf_core::backoff::{BackoffManager, Clock, MAX_FREE_ATTEMPTS};
use caf_core::error::{CafError, VaultError};
use caf_core::vault;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

// Serialize tests touching process-wide CAF_DATA_DIR
static TEST_LOCK: Mutex<()> = Mutex::new(());

struct TestEnv {
    dir: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl TestEnv {
    fn new(label: &str) -> Self {
        let guard = TEST_LOCK.lock().expect("lock test guard");
        vault::lock();
        let dir = std::env::temp_dir().join(format!("caf_p03_{label}_{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::set_var("CAF_DATA_DIR", &dir);
        }
        Self { dir, _guard: guard }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        vault::lock();
        let _ = fs::remove_dir_all(&self.dir);
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::remove_var("CAF_DATA_DIR");
        }
    }
}

struct MockClock {
    time: Arc<AtomicU64>,
}

impl MockClock {
    fn new(start_secs: u64) -> Self {
        Self {
            time: Arc::new(AtomicU64::new(start_secs)),
        }
    }

    fn advance(&self, secs: u64) {
        self.time.fetch_add(secs, Ordering::SeqCst);
    }
}

impl Clock for MockClock {
    fn now_secs(&self) -> u64 {
        self.time.load(Ordering::SeqCst)
    }
}

// --------------------------------------------------------------------------
// P03.1: Remove the plaintext key path & detect legacy layout
// --------------------------------------------------------------------------

#[test]
fn test_p03_1_legacy_detection_and_no_plaintext_key() {
    let env = TestEnv::new("legacy_detection");

    // 1. Fake legacy: vault.key present
    let fake_vault_key = env.dir.join("vault.key");
    fs::write(
        &fake_vault_key,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    )
    .unwrap();
    assert_eq!(
        vault::is_vault_initialized().unwrap_err(),
        CafError::Vault(VaultError::LegacyFormat)
    );
    assert_eq!(
        vault::ensure_unlocked_key().unwrap_err(),
        CafError::Vault(VaultError::LegacyFormat)
    );
    fs::remove_file(&fake_vault_key).unwrap();

    // 2. Fake legacy: local.key present
    let fake_local_key = env.dir.join("local.key");
    fs::write(&fake_local_key, "some-legacy-key").unwrap();
    assert_eq!(
        vault::is_vault_initialized().unwrap_err(),
        CafError::Vault(VaultError::LegacyFormat)
    );
    fs::remove_file(&fake_local_key).unwrap();

    // 3. Fake legacy: vault_salt.bin and vault_canary.bin without keyring
    let fake_salt = env.dir.join("vault_salt.bin");
    let fake_canary = env.dir.join("vault_canary.bin");
    fs::write(&fake_salt, b"fake-salt-16bytes").unwrap();
    fs::write(&fake_canary, b"fake-canary").unwrap();
    assert_eq!(
        vault::is_vault_initialized().unwrap_err(),
        CafError::Vault(VaultError::LegacyFormat)
    );
    fs::remove_file(&fake_salt).unwrap();
    fs::remove_file(&fake_canary).unwrap();

    // 4. Normal setup + unlock + lock -> prove NO vault.key is EVER created
    let _setup = vault::setup_vault("correct-password-1234").expect("setup vault");
    assert!(vault::is_unlocked().unwrap());
    vault::lock();
    assert!(!vault::is_unlocked().unwrap());

    vault::unlock_with_password("correct-password-1234").expect("unlock");
    vault::lock();

    assert!(
        !env.dir.join("vault.key").exists(),
        "vault.key must NOT exist after setup + unlock + lock"
    );
    assert!(
        !env.dir.join("local.key").exists(),
        "local.key must NOT exist after setup + unlock + lock"
    );
}

// --------------------------------------------------------------------------
// P03.2: Keyring round trips
// --------------------------------------------------------------------------

#[test]
fn test_p03_2_keyring_round_trips() {
    let env = TestEnv::new("keyring_round_trips");
    let pass_a = "master-password-phase-one";
    let pass_b = "master-password-phase-two";

    // 1. Setup
    let setup = vault::setup_vault(pass_a).expect("setup failed");
    let mnemonic = setup.mnemonic.as_str().to_string();

    // Insert dummy record to DB to ensure table and data exist
    let conn = caf_core::db::open().expect("open db");
    conn.execute(
        "INSERT INTO app_kv (key, value) VALUES (?1, ?2)",
        ["test_entry", "p03_persisted_secret"],
    )
    .expect("insert entry");
    drop(conn);

    // 2. Lock & wrong password fails with WrongPassword
    vault::lock();
    let err = vault::unlock_with_password("wrong-password-9999").unwrap_err();
    assert_eq!(err, CafError::Vault(VaultError::WrongPassword));

    // 3. Unlock with password A succeeds
    vault::unlock_with_password(pass_a).expect("unlock with pass A");
    assert!(vault::is_unlocked().unwrap());

    // 4. Hash of page 1 before password change
    let db_path = env.dir.join("caframework.db");
    let db_bytes = fs::read(&db_path).expect("read db bytes");
    assert!(
        db_bytes.len() >= 4096,
        "database should be at least one page"
    );
    let hash_page_1_before = Sha256::digest(&db_bytes[..4096]);

    // 5. Change master password (re-wraps DEK without rekeying database)
    vault::change_master_password(pass_a, pass_b).expect("change password");

    // Verify hash of first page is UNCHANGED
    let db_bytes_after = fs::read(&db_path).expect("read db bytes after");
    let hash_page_1_after = Sha256::digest(&db_bytes_after[..4096]);
    assert_eq!(
        hash_page_1_before, hash_page_1_after,
        "Page 1 DB hash must remain unchanged when changing password (DEK re-wrapped, not rekeyed)"
    );

    // 6. Lock and verify old password fails, new password succeeds
    vault::lock();
    assert_eq!(
        vault::unlock_with_password(pass_a).unwrap_err(),
        CafError::Vault(VaultError::WrongPassword)
    );
    vault::unlock_with_password(pass_b).expect("unlock with pass B");

    // Verify data still readable
    let conn2 = caf_core::db::open().expect("open db after pass change");
    let val: String = conn2
        .query_row(
            "SELECT value FROM app_kv WHERE key = ?1",
            ["test_entry"],
            |r| r.get(0),
        )
        .expect("read back");
    assert_eq!(val, "p03_persisted_secret");
    drop(conn2);

    // 7. Recovery code round trip
    vault::lock();
    // Wrong recovery code fails
    let fake_recovery = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    assert_eq!(
        vault::unlock_with_recovery(fake_recovery).unwrap_err(),
        CafError::Vault(VaultError::WrongRecoveryCode)
    );

    // Valid recovery code unlocks
    vault::unlock_with_recovery(&mnemonic).expect("unlock with recovery");
    assert!(vault::is_unlocked().unwrap());

    // 8. Regenerate recovery code revokes the old one
    let (new_mnemonic, _) = vault::regenerate_recovery_code(pass_b).expect("regen recovery");
    vault::lock();

    // Old mnemonic is now revoked!
    assert_eq!(
        vault::unlock_with_recovery(&mnemonic).unwrap_err(),
        CafError::Vault(VaultError::WrongRecoveryCode)
    );

    // New mnemonic unlocks
    vault::unlock_with_recovery(&new_mnemonic).expect("unlock with new recovery");
    assert!(vault::is_unlocked().unwrap());
}

// --------------------------------------------------------------------------
// P03.3: Recovery code flow & zero secrets on disk
// --------------------------------------------------------------------------

#[test]
fn test_p03_3_recovery_words_never_written_to_disk() {
    let env = TestEnv::new("recovery_disk_scan");
    let password = "test-master-password-123";

    let setup = vault::setup_vault(password).expect("setup vault");
    let words: Vec<&str> = setup.mnemonic.as_str().split_whitespace().collect();
    assert_eq!(words.len(), 24, "Mnemonic must have exactly 24 words");

    // Confirmation test
    let positions = vec![2, 7, 19];
    let guesses = vec![words[2], words[7], words[19]];
    vault::confirm_recovery_words(&setup.entropy, &positions, &guesses).expect("confirm 3 words");

    let wrong_guesses = vec![words[2], "wrongword", words[19]];
    assert_eq!(
        vault::confirm_recovery_words(&setup.entropy, &positions, &wrong_guesses).unwrap_err(),
        CafError::Vault(VaultError::WrongRecoveryCode)
    );

    let dek_hex = hex::encode(&setup.keys.dek);

    // Scan all files in env.dir (recursively)
    for entry in fs::read_dir(&env.dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let bytes = fs::read(&path).unwrap();
            let content_str = String::from_utf8_lossy(&bytes);

            // DEK hex must NEVER appear in plaintext on disk
            assert!(
                !content_str.contains(&dek_hex),
                "File {:?} contains plaintext hex DEK!",
                path.file_name()
            );

            // None of the 24 recovery words may appear in keyring.v1.json or db as plain English word tokens
            if path.file_name().unwrap() == "keyring.v1.json" {
                for word in &words {
                    assert!(
                        !content_str.contains(&format!("\"{word}\"")),
                        "keyring.v1.json contains plaintext recovery word: {word}"
                    );
                }
            }
        }
    }
}

// --------------------------------------------------------------------------
// P03.4: Integrity and plain-header guard
// --------------------------------------------------------------------------

#[test]
fn test_p03_4_integrity_and_plain_header_guard() {
    let env = TestEnv::new("plain_header_guard");
    let password = "valid-master-password-000";

    // 1. Setup creates an encrypted DB which does NOT start with "SQLite format 3"
    let _ = vault::setup_vault(password).expect("setup");
    let db_path = env.dir.join("caframework.db");
    let header = fs::read(&db_path).expect("read db");
    assert_ne!(
        &header[..16],
        b"SQLite format 3\0",
        "Encrypted database must NEVER start with plaintext SQLite header"
    );

    vault::lock();

    // 2. Plaintext header test: replace DB with unencrypted SQLite file
    let plain_header = b"SQLite format 3\0\x04\x00\x01\x01\x00@  \0\0\0\x01";
    fs::write(&db_path, plain_header).expect("overwrite with plain sqlite header");

    let err = vault::unlock_with_password(password).unwrap_err();
    assert_eq!(
        err,
        CafError::Vault(VaultError::CorruptedHeader),
        "Plaintext SQLite header must be rejected with CorruptedHeader"
    );

    // 3. Corrupted DB test: replace DB with garbage bytes
    let garbage = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33];
    fs::write(&db_path, garbage).expect("overwrite with garbage");

    let err_corrupt = vault::unlock_with_password(password).unwrap_err();
    assert_eq!(
        err_corrupt,
        CafError::Vault(VaultError::CorruptedHeader),
        "Corrupted DB fixture must yield CorruptedHeader"
    );
}

// --------------------------------------------------------------------------
// P03.5: Back-off and auto-lock (D-23)
// --------------------------------------------------------------------------

#[test]
fn test_p03_5_backoff_and_lockout() {
    let env = TestEnv::new("backoff_test");
    let clock = MockClock::new(1_000_000);
    let manager = BackoffManager::new();

    // 1. 5 free attempts: no lockout
    for _ in 1..=MAX_FREE_ATTEMPTS {
        manager.check_lockout(&clock).expect("check lockout");
        let delay = manager.record_auth_failure(&clock, Some(&env.dir)).unwrap();
        assert_eq!(delay, 0, "5 free attempts must have 0 delay");
    }

    // 2. Attempt 6: 30s delay
    let delay_6 = manager.record_auth_failure(&clock, Some(&env.dir)).unwrap();
    assert_eq!(delay_6, 30, "6th attempt must trigger 30s delay");
    assert_eq!(
        manager.check_lockout(&clock).unwrap_err(),
        CafError::Vault(VaultError::LockoutActive)
    );

    // Advance clock by 31 seconds: lockout expired
    clock.advance(31);
    assert!(manager.check_lockout(&clock).is_ok());

    // 3. Attempt 7: doubling delay to 60s
    let delay_7 = manager.record_auth_failure(&clock, Some(&env.dir)).unwrap();
    assert_eq!(delay_7, 60, "7th attempt must double to 60s delay");

    // 4. Auth success resets counters
    manager
        .record_auth_success(Some(&env.dir))
        .expect("record success");
    assert!(manager.check_lockout(&clock).is_ok());
    assert_eq!(manager.current_state().consecutive_auth_failures, 0);

    // 5. PIN failures: 10 failures lock vault
    for i in 1..=9 {
        let lock = manager.record_pin_failure(&clock, Some(&env.dir)).unwrap();
        assert!(!lock, "PIN attempt {i} should not lock vault");
    }
    let lock_10 = manager.record_pin_failure(&clock, Some(&env.dir)).unwrap();
    assert!(lock_10, "10th consecutive PIN failure MUST lock vault");

    // 6. Persistence across restart
    let reloaded = BackoffManager::load(&env.dir);
    assert_eq!(reloaded.current_state().consecutive_pin_failures, 10);
}

// --------------------------------------------------------------------------
// P03.6: Secret handling & Unix permissions
// --------------------------------------------------------------------------

#[test]
fn test_p03_6_secret_handling_and_unix_permissions() {
    let env = TestEnv::new("secret_handling");
    let password = "master-password-perm-test";

    let _ = vault::setup_vault(password).expect("setup");
    assert!(vault::is_unlocked().unwrap());

    // Test Unix permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let dir_perm = fs::metadata(&env.dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            dir_perm, 0o700,
            "Data directory mode must be 0700 on Unix, found {dir_perm:o}"
        );

        let keyring_path = env.dir.join("keyring.v1.json");
        let keyring_perm = fs::metadata(&keyring_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            keyring_perm, 0o600,
            "Keyring file mode must be 0600 on Unix, found {keyring_perm:o}"
        );
    }

    // Zeroized on lock
    vault::lock();
    assert!(!vault::is_unlocked().unwrap());
    assert_eq!(
        vault::ensure_unlocked_key().unwrap_err(),
        CafError::Vault(VaultError::Locked)
    );
}
