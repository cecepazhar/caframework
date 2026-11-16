//! Zero-Knowledge Argon2id Keyring Vault Manager (ADR-0001).

use crate::backoff::{Clock, SystemClock};
use crate::error::{CafError, VaultError};
use parking_lot::RwLock;
use std::path::Path;
use std::sync::LazyLock;
use zeroize::{Zeroize, Zeroizing};

pub const MIN_PASSWORD_LEN: usize = 12; // D-7

static ACTIVE_VAULT_KEY: LazyLock<RwLock<Option<[u8; 32]>>> = LazyLock::new(|| RwLock::new(None));

fn vault_err(msg: impl Into<String>) -> CafError {
    CafError::Vault(VaultError::Generic(msg.into()))
}

/// Detects legacy plaintext keys or pre-keyring formats.
/// Returns `VaultError::LegacyFormat` if any legacy files exist.
pub fn detect_legacy_vault(data_dir: &Path) -> Result<(), CafError> {
    let has_vault_key = data_dir.join("vault.key").exists();
    let has_local_key = data_dir.join("local.key").exists();
    let has_canary = data_dir.join("vault_canary.bin").exists();
    let has_salt = data_dir.join("vault_salt.bin").exists();
    let has_keyring = data_dir.join(crate::keyring::KEYRING_FILE).exists();

    if has_vault_key || has_local_key || (!has_keyring && (has_canary || has_salt)) {
        return Err(CafError::Vault(VaultError::LegacyFormat));
    }
    Ok(())
}

/// Returns true if the vault is currently unlocked in memory.
pub fn is_unlocked() -> Result<bool, CafError> {
    let guard = ACTIVE_VAULT_KEY.read();
    Ok(guard.is_some())
}

/// Returns the current active DEK hex string if unlocked.
/// Returns `VaultError::LegacyFormat` if legacy files are detected.
/// Returns `VaultError::Locked` if the vault is locked.
pub fn ensure_unlocked_key() -> Result<String, CafError> {
    let data_dir = crate::paths::resolve_data_dir()?.path;
    detect_legacy_vault(&data_dir)?;

    let guard = ACTIVE_VAULT_KEY.read();
    if let Some(ref key) = *guard {
        Ok(hex::encode(key))
    } else {
        Err(CafError::Vault(VaultError::Locked))
    }
}

/// Returns true if `keyring.v1.json` exists and no legacy files exist.
pub fn is_vault_initialized() -> Result<bool, CafError> {
    let data_dir = crate::paths::resolve_data_dir()?.path;
    detect_legacy_vault(&data_dir)?;
    Ok(data_dir.join(crate::keyring::KEYRING_FILE).exists())
}

/// Locks the vault and securely zeroizes the in-memory DEK.
pub fn lock() {
    let mut guard = ACTIVE_VAULT_KEY.write();
    if let Some(mut key) = guard.take() {
        key.zeroize();
    }
}

/// Resets the vault files. Legacy files are reported if found.
pub fn reset_vault() -> Result<(), CafError> {
    let data_dir = crate::paths::resolve_data_dir()?.path;

    lock();

    if data_dir.join("vault.key").exists() || data_dir.join("local.key").exists() {
        let _ = crate::audit::log_event(
            "VAULT_RESET",
            None,
            "Legacy vault format detected during reset",
        );
    }

    let files = [
        crate::keyring::KEYRING_FILE,
        crate::backoff::ATTEMPTS_FILE,
        "caframework.db",
        "caframework.db-wal",
        "caframework.db-shm",
        "caframework.db-journal",
    ];
    for f in &files {
        let p = data_dir.join(f);
        if p.exists() {
            let _ = std::fs::remove_file(&p);
        }
    }

    Ok(())
}

/// Setup a new vault with master password. Generates 24-word recovery code (P03.3).
pub fn setup_vault(password: &str) -> Result<crate::keyring::SetupResult, CafError> {
    let data_dir = crate::paths::resolve_data_dir()?.path;
    detect_legacy_vault(&data_dir)?;

    if data_dir.join(crate::keyring::KEYRING_FILE).exists() {
        return Err(vault_err("Vault already initialized"));
    }

    crate::paths::ensure_secure_dir(&data_dir)?;

    let setup_res = crate::keyring::initialize_vault_keyring(password)?;
    crate::keyring::save(&data_dir, &setup_res.keyring)?;

    {
        let mut guard = ACTIVE_VAULT_KEY.write();
        let mut dek_arr = [0u8; 32];
        dek_arr.copy_from_slice(setup_res.keys.dek.as_slice());
        *guard = Some(dek_arr);
    }

    let _ = crate::db::open()?;
    let _ = crate::audit::log_event(
        "VAULT_INIT",
        None,
        "Vault initialized with ADR-0001 keyring",
    );

    Ok(setup_res)
}

/// Confirm 3 random recovery words during setup.
pub fn confirm_recovery_words(
    entropy: &[u8],
    positions: &[usize],
    guesses: &[&str],
) -> Result<(), CafError> {
    crate::keyring::confirm_recovery_words(entropy, positions, guesses)
}

/// Unlock vault with master password, checking back-off and auto-lock rules.
pub fn unlock_with_password(password: &str) -> Result<bool, CafError> {
    unlock_with_password_and_clock(password, &SystemClock)
}

pub fn unlock_with_password_and_clock(password: &str, clock: &dyn Clock) -> Result<bool, CafError> {
    let data_dir = crate::paths::resolve_data_dir()?.path;
    detect_legacy_vault(&data_dir)?;

    let backoff = crate::backoff::BackoffManager::load(&data_dir);
    backoff.check_lockout(clock)?;

    let keyring = crate::keyring::load(&data_dir)?;
    match crate::keyring::unwrap_with_password(&keyring, password) {
        Ok(keys) => {
            let _ = backoff.record_auth_success(Some(&data_dir));
            {
                let mut guard = ACTIVE_VAULT_KEY.write();
                let mut dek_arr = [0u8; 32];
                dek_arr.copy_from_slice(keys.dek.as_slice());
                *guard = Some(dek_arr);
            }
            let _ = crate::db::open()?;
            let _ = crate::audit::log_event("VAULT_UNLOCK", None, "Vault unlocked with password");
            Ok(true)
        }
        Err(e) => {
            let _ = backoff.record_auth_failure(clock, Some(&data_dir));
            Err(e)
        }
    }
}

/// Unlock vault with 24-word recovery mnemonic phrase.
pub fn unlock_with_recovery(mnemonic_phrase: &str) -> Result<bool, CafError> {
    unlock_with_recovery_and_clock(mnemonic_phrase, &SystemClock)
}

pub fn unlock_with_recovery_and_clock(
    mnemonic_phrase: &str,
    clock: &dyn Clock,
) -> Result<bool, CafError> {
    let data_dir = crate::paths::resolve_data_dir()?.path;
    detect_legacy_vault(&data_dir)?;

    let backoff = crate::backoff::BackoffManager::load(&data_dir);
    backoff.check_lockout(clock)?;

    let entropy = crate::keyring::mnemonic_to_entropy(mnemonic_phrase)?;
    let keyring = crate::keyring::load(&data_dir)?;
    match crate::keyring::unwrap_with_recovery(&keyring, &entropy) {
        Ok(keys) => {
            let _ = backoff.record_auth_success(Some(&data_dir));
            {
                let mut guard = ACTIVE_VAULT_KEY.write();
                let mut dek_arr = [0u8; 32];
                dek_arr.copy_from_slice(keys.dek.as_slice());
                *guard = Some(dek_arr);
            }
            let _ = crate::db::open()?;
            let _ =
                crate::audit::log_event("VAULT_UNLOCK", None, "Vault unlocked with recovery code");
            Ok(true)
        }
        Err(e) => {
            let _ = backoff.record_auth_failure(clock, Some(&data_dir));
            Err(e)
        }
    }
}

/// Legacy/unified entrypoint: initializes if uninitialized, or unlocks if initialized.
pub fn validate_password(password: &str) -> Result<bool, CafError> {
    let data_dir = crate::paths::resolve_data_dir()?.path;
    detect_legacy_vault(&data_dir)?;

    if !data_dir.join(crate::keyring::KEYRING_FILE).exists() {
        let _ = setup_vault(password)?;
        return Ok(true);
    }

    unlock_with_password(password)
}

/// Change master password: re-wraps the DEK without rekeying the database file (P03.2).
pub fn change_master_password(old_pass: &str, new_pass: &str) -> Result<(), CafError> {
    if old_pass == new_pass {
        return Err(vault_err("Master password baru harus berbeda."));
    }
    let data_dir = crate::paths::resolve_data_dir()?.path;
    detect_legacy_vault(&data_dir)?;

    let mut keyring = crate::keyring::load(&data_dir)?;
    crate::keyring::change_master_password(&mut keyring, old_pass, new_pass)?;
    crate::keyring::save(&data_dir, &keyring)?;

    let _ = crate::audit::log_event(
        "VAULT_REKEY",
        None,
        "Master password changed (DEK re-wrapped)",
    );
    Ok(())
}

/// Regenerate recovery code: revokes old recovery code and replaces it in the keyring (P03.2).
pub fn regenerate_recovery_code(
    password: &str,
) -> Result<(Zeroizing<String>, Zeroizing<Vec<u8>>), CafError> {
    let data_dir = crate::paths::resolve_data_dir()?.path;
    detect_legacy_vault(&data_dir)?;

    let mut keyring = crate::keyring::load(&data_dir)?;
    let (new_mnemonic, new_entropy) = crate::keyring::generate_mnemonic();
    crate::keyring::replace_recovery_slot(&mut keyring, password, &new_entropy)?;
    crate::keyring::save(&data_dir, &keyring)?;

    let _ = crate::audit::log_event(
        "VAULT_RECOVERY_REGEN",
        None,
        "Recovery code regenerated and old code revoked",
    );
    Ok((new_mnemonic, new_entropy))
}

#[doc(hidden)]
/// Internal test helper to initialize an active vault for unit test suites.
pub fn init_test_vault(data_dir: &Path) {
    let _ = crate::paths::ensure_secure_dir(data_dir);
    let dek = [0x42u8; 32];
    {
        let mut guard = ACTIVE_VAULT_KEY.write();
        *guard = Some(dek);
    }
    let setup = crate::keyring::initialize_vault_keyring("super-secret-password-123");
    if let Ok(mut res) = setup {
        res.keys.dek = Zeroizing::new(dek);
        let _ = crate::keyring::save(data_dir, &res.keyring);
    }
    let _ = crate::db::open();
}
