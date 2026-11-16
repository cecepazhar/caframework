use crate::error::{CafError, VaultError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::RwLock;

pub const ATTEMPTS_FILE: &str = "auth_attempts.json";
pub const MAX_FREE_ATTEMPTS: u32 = 5;
pub const BASE_DELAY_SECS: u64 = 30;
pub const MAX_DELAY_SECS: u64 = 900; // 15 minutes
pub const MAX_PIN_FAILURES_LOCK_VAULT: u32 = 10;
pub const DEFAULT_PROFILE_LOCK_SECS: u64 = 300; // 5 minutes
pub const DEFAULT_VAULT_LOCK_SECS: u64 = 900; // 15 minutes

pub trait Clock: Send + Sync {
    fn now_secs(&self) -> u64;
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now_secs(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AuthAttemptState {
    pub consecutive_auth_failures: u32,
    pub lockout_until: u64,
    pub consecutive_pin_failures: u32,
}

pub struct BackoffManager {
    state: RwLock<AuthAttemptState>,
}

impl Default for BackoffManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BackoffManager {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(AuthAttemptState::default()),
        }
    }

    pub fn load(data_dir: &Path) -> Self {
        let file = data_dir.join(ATTEMPTS_FILE);
        if let Ok(content) = fs::read_to_string(&file)
            && let Ok(state) = serde_json::from_str::<AuthAttemptState>(&content)
        {
            return Self {
                state: RwLock::new(state),
            };
        }
        Self::new()
    }

    pub fn save(&self, data_dir: &Path) -> Result<(), CafError> {
        let state = match self.state.read() {
            Ok(s) => s.clone(),
            Err(_) => return Err(CafError::Vault(VaultError::Generic("lock poisoned".into()))),
        };
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| CafError::Vault(VaultError::Generic(e.to_string())))?;

        let tmp_file = data_dir.join(format!("{ATTEMPTS_FILE}.tmp"));
        let final_file = data_dir.join(ATTEMPTS_FILE);

        let _ = fs::create_dir_all(data_dir);
        fs::write(&tmp_file, json.as_bytes())
            .map_err(|e| CafError::Vault(VaultError::Generic(e.to_string())))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&tmp_file, fs::Permissions::from_mode(0o600));
        }

        fs::rename(&tmp_file, &final_file)
            .map_err(|e| CafError::Vault(VaultError::Generic(e.to_string())))?;
        Ok(())
    }

    pub fn check_lockout(&self, clock: &dyn Clock) -> Result<(), CafError> {
        let state = match self.state.read() {
            Ok(s) => s.clone(),
            Err(_) => return Err(CafError::Vault(VaultError::Generic("lock poisoned".into()))),
        };

        let now = clock.now_secs();
        if now < state.lockout_until {
            return Err(CafError::Vault(VaultError::LockoutActive));
        }
        Ok(())
    }

    pub fn record_auth_failure(
        &self,
        clock: &dyn Clock,
        data_dir: Option<&Path>,
    ) -> Result<u64, CafError> {
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return Err(CafError::Vault(VaultError::Generic("lock poisoned".into()))),
        };

        state.consecutive_auth_failures = state.consecutive_auth_failures.saturating_add(1);
        let failures = state.consecutive_auth_failures;

        let delay = if failures > MAX_FREE_ATTEMPTS {
            let exponent = failures - MAX_FREE_ATTEMPTS - 1;
            let factor = 1u64.checked_shl(exponent.min(10)).unwrap_or(1024);
            BASE_DELAY_SECS.saturating_mul(factor).min(MAX_DELAY_SECS)
        } else {
            0
        };

        let now = clock.now_secs();
        if delay > 0 {
            state.lockout_until = now.saturating_add(delay);
        } else {
            state.lockout_until = 0;
        }

        drop(state);
        if let Some(dir) = data_dir {
            let _ = self.save(dir);
        }

        Ok(delay)
    }

    pub fn record_auth_success(&self, data_dir: Option<&Path>) -> Result<(), CafError> {
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return Err(CafError::Vault(VaultError::Generic("lock poisoned".into()))),
        };

        state.consecutive_auth_failures = 0;
        state.lockout_until = 0;
        drop(state);

        if let Some(dir) = data_dir {
            let _ = self.save(dir);
        }
        Ok(())
    }

    /// Record a failed PIN attempt.
    /// Returns `true` if 10 consecutive failures occurred, which MUST lock the vault.
    pub fn record_pin_failure(
        &self,
        clock: &dyn Clock,
        data_dir: Option<&Path>,
    ) -> Result<bool, CafError> {
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return Err(CafError::Vault(VaultError::Generic("lock poisoned".into()))),
        };

        state.consecutive_pin_failures = state.consecutive_pin_failures.saturating_add(1);
        let pin_fails = state.consecutive_pin_failures;

        // Apply backoff delay on PIN as well
        if pin_fails > MAX_FREE_ATTEMPTS {
            let exponent = pin_fails - MAX_FREE_ATTEMPTS - 1;
            let factor = 1u64.checked_shl(exponent.min(10)).unwrap_or(1024);
            let delay = BASE_DELAY_SECS.saturating_mul(factor).min(MAX_DELAY_SECS);
            state.lockout_until = clock.now_secs().saturating_add(delay);
        }

        let should_lock_vault = pin_fails >= MAX_PIN_FAILURES_LOCK_VAULT;
        drop(state);

        if let Some(dir) = data_dir {
            let _ = self.save(dir);
        }

        Ok(should_lock_vault)
    }

    pub fn record_pin_success(&self, data_dir: Option<&Path>) -> Result<(), CafError> {
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return Err(CafError::Vault(VaultError::Generic("lock poisoned".into()))),
        };

        state.consecutive_pin_failures = 0;
        state.lockout_until = 0;
        drop(state);

        if let Some(dir) = data_dir {
            let _ = self.save(dir);
        }
        Ok(())
    }

    pub fn current_state(&self) -> AuthAttemptState {
        self.state.read().map(|s| s.clone()).unwrap_or_default()
    }
}
