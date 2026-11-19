//! Core session and authorization state management.

#[cfg(feature = "mod-profiles")]
use crate::error::DbError;
use crate::error::{AuthError, CafError, VaultError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub profile_id: String,
    pub display_name: String,
    pub role_key: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub profile_id: String,
    pub display_name: String,
    pub role: String,
    pub permissions: HashSet<String>,
    pub started_at: u64,
    pub last_activity_at: u64,
}

impl Session {
    pub fn new(profile_id: impl Into<String>, role: impl Into<String>) -> Self {
        let p_id = profile_id.into();
        let r_str = role.into();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let perms: HashSet<String> = crate::rbac::get_role_permissions(&r_str)
            .unwrap_or_default()
            .into_iter()
            .collect();
        Self {
            profile_id: p_id.clone(),
            display_name: p_id,
            role: r_str,
            permissions: perms,
            started_at: now,
            last_activity_at: now,
        }
    }

    pub fn with_details(
        profile_id: impl Into<String>,
        display_name: impl Into<String>,
        role: impl Into<String>,
        permissions: impl IntoIterator<Item = String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            profile_id: profile_id.into(),
            display_name: display_name.into(),
            role: role.into(),
            permissions: permissions.into_iter().collect(),
            started_at: now,
            last_activity_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_activity_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    pub fn has_permission(&self, required: &str) -> bool {
        self.permissions
            .iter()
            .any(|p| crate::rbac::matches_permission_str(p, required))
    }

    pub fn to_info(&self) -> SessionInfo {
        let mut perms: Vec<String> = self.permissions.iter().cloned().collect();
        perms.sort();
        SessionInfo {
            profile_id: self.profile_id.clone(),
            display_name: self.display_name.clone(),
            role_key: self.role.clone(),
            permissions: perms,
        }
    }
}

static CURRENT_SESSION: RwLock<Option<Session>> = RwLock::new(None);

pub fn get_current_session() -> Result<Session, CafError> {
    let lock = CURRENT_SESSION
        .read()
        .map_err(|_| CafError::Auth(AuthError::Unauthorized))?;
    lock.clone().ok_or(CafError::Auth(AuthError::Unauthorized))
}

pub fn set_current_session(session: Session) -> Result<(), CafError> {
    let mut lock = CURRENT_SESSION
        .write()
        .map_err(|_| CafError::Auth(AuthError::Unauthorized))?;
    *lock = Some(session);
    Ok(())
}

pub fn clear_current_session() -> Result<(), CafError> {
    let mut lock = CURRENT_SESSION
        .write()
        .map_err(|_| CafError::Auth(AuthError::Unauthorized))?;
    *lock = None;
    Ok(())
}

pub fn current_profile_id() -> Result<String, CafError> {
    get_current_session().map(|s| s.profile_id)
}

pub fn is_current_super_role() -> Result<bool, CafError> {
    get_current_session().map(|s| crate::rbac::is_super_role(&s.role))
}

pub fn has_current_permission(required: &str) -> Result<bool, CafError> {
    get_current_session().map(|s| s.has_permission(required))
}

/// Unlocks a profile and establishes a core-owned session.
pub fn profile_unlock(profile_id: &str, pin: &str) -> Result<SessionInfo, CafError> {
    if !crate::vault::is_unlocked().unwrap_or(false) {
        return Err(CafError::Vault(VaultError::Locked));
    }

    #[cfg(feature = "mod-profiles")]
    {
        let ok = crate::profiles::verify_pin(profile_id, pin)?;
        if !ok {
            return Err(CafError::Auth(AuthError::InvalidPin));
        }

        let profile = crate::profiles::get_profile(profile_id)?
            .ok_or(CafError::Db(DbError::RecordNotFound))?;

        let perms = crate::rbac::get_role_permissions(&profile.role)?;
        let session = Session::with_details(&profile.id, &profile.name, &profile.role, perms);
        let info = session.to_info();
        set_current_session(session)?;

        Ok(info)
    }
    #[cfg(not(feature = "mod-profiles"))]
    {
        let _ = (profile_id, pin);
        Err(CafError::generic(
            "Profiles module is not compiled into this binary",
        ))
    }
}

/// Ends the active profile session.
pub fn profile_lock() -> Result<(), CafError> {
    clear_current_session()
}

/// Returns info about the active session, or unauthorized if no session exists.
pub fn session_info() -> Result<SessionInfo, CafError> {
    get_current_session().map(|s| s.to_info())
}

/// Returns the list of permission keys granted to the active session.
/// If vault is locked or no session exists, returns an empty list.
pub fn session_permissions() -> Result<Vec<String>, CafError> {
    if !crate::vault::is_unlocked().unwrap_or(false) {
        return Ok(Vec::new());
    }
    match get_current_session() {
        Ok(s) => Ok(s.permissions.into_iter().collect()),
        Err(_) => Ok(Vec::new()),
    }
}

#[derive(Debug, Default)]
pub struct SessionManager {
    active_session: Option<Session>,
}

impl SessionManager {
    pub fn set_session(&mut self, session: Session) {
        self.active_session = Some(session);
    }

    pub fn clear(&mut self) {
        self.active_session = None;
    }

    pub fn get(&self) -> Result<&Session, CafError> {
        self.active_session
            .as_ref()
            .ok_or(CafError::Auth(AuthError::Unauthorized))
    }

    pub fn get_mut(&mut self) -> Result<&mut Session, CafError> {
        self.active_session
            .as_mut()
            .ok_or(CafError::Auth(AuthError::Unauthorized))
    }
}
