//! Command policy classification and registry.

use crate::error::{AuthError, CafError, VaultError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Policy {
    /// Works while vault is locked (setup, initial unlock, status, app info).
    Public,
    /// Vault must be unlocked, but no profile session required.
    Unlocked,
    /// Vault must be unlocked and an active profile session must exist.
    Session,
    /// Specific RBAC permission required on the active profile session.
    Requires(&'static str),
    /// Requires unlocked vault, active session, and Pro entitlement.
    Pro,
}

/// Metadata describing a registered command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPolicy {
    pub name: &'static str,
    pub policy: Policy,
    pub returns_list: bool,
    pub module: Option<&'static str>,
}

pub const COMMAND_POLICIES: &[CommandPolicy] = &[
    // Window management (Public)
    CommandPolicy {
        name: "window_minimize",
        policy: Policy::Public,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "window_maximize",
        policy: Policy::Public,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "window_close",
        policy: Policy::Public,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "window_start_dragging",
        policy: Policy::Public,
        returns_list: false,
        module: None,
    },
    // Vault commands
    CommandPolicy {
        name: "is_vault_initialized",
        policy: Policy::Public,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "validate_vault_password",
        policy: Policy::Public,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "lock_vault",
        policy: Policy::Public,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "change_master_password",
        policy: Policy::Requires("security:manage"),
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "reset_vault",
        policy: Policy::Requires("security:manage"),
        returns_list: false,
        module: None,
    },
    // Profiles
    #[cfg(feature = "mod-profiles")]
    CommandPolicy {
        name: "list_profiles",
        policy: Policy::Unlocked,
        returns_list: true,
        module: Some("profiles"),
    },
    #[cfg(feature = "mod-profiles")]
    CommandPolicy {
        name: "save_profile",
        policy: Policy::Session,
        returns_list: false,
        module: Some("profiles"),
    },
    #[cfg(feature = "mod-profiles")]
    CommandPolicy {
        name: "delete_profile",
        policy: Policy::Requires("users:manage"),
        returns_list: false,
        module: Some("profiles"),
    },
    #[cfg(feature = "mod-profiles")]
    CommandPolicy {
        name: "verify_pin",
        policy: Policy::Unlocked,
        returns_list: false,
        module: Some("profiles"),
    },
    #[cfg(feature = "mod-profiles")]
    CommandPolicy {
        name: "profile_unlock",
        policy: Policy::Unlocked,
        returns_list: false,
        module: Some("profiles"),
    },
    #[cfg(feature = "mod-profiles")]
    CommandPolicy {
        name: "profile_lock",
        policy: Policy::Session,
        returns_list: false,
        module: Some("profiles"),
    },
    CommandPolicy {
        name: "session_info",
        policy: Policy::Public,
        returns_list: false,
        module: None,
    },
    // RBAC
    CommandPolicy {
        name: "list_roles",
        policy: Policy::Session,
        returns_list: true,
        module: None,
    },
    CommandPolicy {
        name: "save_role",
        policy: Policy::Requires("roles:manage"),
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "delete_role",
        policy: Policy::Requires("roles:manage"),
        returns_list: false,
        module: None,
    },
    // Notes
    CommandPolicy {
        name: "list_notes",
        policy: Policy::Session,
        returns_list: true,
        module: None,
    },
    CommandPolicy {
        name: "save_note",
        policy: Policy::Session,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "delete_note",
        policy: Policy::Session,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "get_notes_summary",
        policy: Policy::Session,
        returns_list: true,
        module: None,
    },
    // AI
    #[cfg(feature = "mod-ai")]
    CommandPolicy {
        name: "get_ai_settings",
        policy: Policy::Session,
        returns_list: false,
        module: Some("ai"),
    },
    #[cfg(feature = "mod-ai")]
    CommandPolicy {
        name: "save_ai_settings",
        policy: Policy::Requires("settings:manage"),
        returns_list: false,
        module: Some("ai"),
    },
    #[cfg(feature = "mod-ai")]
    CommandPolicy {
        name: "set_ai_api_key",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
        module: Some("ai"),
    },
    #[cfg(feature = "mod-ai")]
    CommandPolicy {
        name: "clear_ai_api_key",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
        module: Some("ai"),
    },
    #[cfg(feature = "mod-ai")]
    CommandPolicy {
        name: "ai_preview_context",
        policy: Policy::Session,
        returns_list: false,
        module: Some("ai"),
    },
    #[cfg(feature = "mod-ai")]
    CommandPolicy {
        name: "ai_chat",
        policy: Policy::Session,
        returns_list: false,
        module: Some("ai"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_get_providers",
        policy: Policy::Session,
        returns_list: true,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_save_provider",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_delete_provider",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_get_routing_matrix",
        policy: Policy::Session,
        returns_list: true,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_save_routing_rule",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_get_personas",
        policy: Policy::Session,
        returns_list: true,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_save_persona",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_delete_persona",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_get_habits",
        policy: Policy::Session,
        returns_list: true,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_search_habits",
        policy: Policy::Session,
        returns_list: true,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_toggle_habit_pin",
        policy: Policy::Session,
        returns_list: false,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_delete_habit",
        policy: Policy::Session,
        returns_list: false,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_get_skills",
        policy: Policy::Session,
        returns_list: true,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_save_skill",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_delete_skill",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_dispatch_task",
        policy: Policy::Session,
        returns_list: false,
        module: Some("ai_advanced"),
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_dispatch_task_with_skill",
        policy: Policy::Session,
        returns_list: false,
        module: Some("ai_advanced"),
    },
    // Feedback & Crash
    #[cfg(feature = "mod-feedback")]
    CommandPolicy {
        name: "submit_feedback",
        policy: Policy::Public,
        returns_list: false,
        module: Some("feedback"),
    },
    #[cfg(feature = "mod-crash")]
    CommandPolicy {
        name: "get_pending_crash_report",
        policy: Policy::Public,
        returns_list: false,
        module: Some("crash"),
    },
    #[cfg(feature = "mod-crash")]
    CommandPolicy {
        name: "dismiss_crash_report",
        policy: Policy::Public,
        returns_list: false,
        module: Some("crash"),
    },
    // Backup
    CommandPolicy {
        name: "export_encrypted_backup",
        policy: Policy::Requires("backup:export"),
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "import_encrypted_backup",
        policy: Policy::Requires("backup:import"),
        returns_list: false,
        module: None,
    },
    // Preferences
    CommandPolicy {
        name: "get_performance_prefs",
        policy: Policy::Session,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "set_performance_prefs",
        policy: Policy::Session,
        returns_list: false,
        module: None,
    },
    // Scheduler (P08)
    #[cfg(feature = "mod-scheduler")]
    CommandPolicy {
        name: "scheduler_list_jobs",
        policy: Policy::Requires("scheduler:view"),
        returns_list: true,
        module: Some("scheduler"),
    },
    #[cfg(feature = "mod-scheduler")]
    CommandPolicy {
        name: "scheduler_set_job_enabled",
        policy: Policy::Requires("scheduler:manage"),
        returns_list: false,
        module: Some("scheduler"),
    },
    #[cfg(feature = "mod-scheduler")]
    CommandPolicy {
        name: "scheduler_update_schedule",
        policy: Policy::Requires("scheduler:manage"),
        returns_list: false,
        module: Some("scheduler"),
    },
    #[cfg(feature = "mod-scheduler")]
    CommandPolicy {
        name: "scheduler_run_job_now",
        policy: Policy::Requires("scheduler:manage"),
        returns_list: false,
        module: Some("scheduler"),
    },
    #[cfg(feature = "mod-scheduler")]
    CommandPolicy {
        name: "scheduler_get_history",
        policy: Policy::Requires("scheduler:view"),
        returns_list: true,
        module: Some("scheduler"),
    },
    #[cfg(feature = "mod-scheduler")]
    CommandPolicy {
        name: "scheduler_get_desktop_settings",
        policy: Policy::Session,
        returns_list: false,
        module: Some("scheduler"),
    },
    #[cfg(feature = "mod-scheduler")]
    CommandPolicy {
        name: "scheduler_set_desktop_settings",
        policy: Policy::Requires("settings:manage"),
        returns_list: false,
        module: Some("scheduler"),
    },
    // Integrations & Outbox (P09)
    #[cfg(feature = "mod-integrations")]
    CommandPolicy {
        name: "list_integrations",
        policy: Policy::Session,
        returns_list: true,
        module: Some("integrations"),
    },
    #[cfg(feature = "mod-integrations")]
    CommandPolicy {
        name: "set_integration_secret",
        policy: Policy::Requires("integrations:manage"),
        returns_list: false,
        module: Some("integrations"),
    },
    #[cfg(feature = "mod-integrations")]
    CommandPolicy {
        name: "delete_integration_secret",
        policy: Policy::Requires("integrations:manage"),
        returns_list: false,
        module: Some("integrations"),
    },
    #[cfg(feature = "mod-integrations")]
    CommandPolicy {
        name: "test_integration_connection",
        policy: Policy::Requires("integrations:manage"),
        returns_list: false,
        module: Some("integrations"),
    },
    #[cfg(feature = "mod-integrations")]
    CommandPolicy {
        name: "get_integration_logs",
        policy: Policy::Requires("integrations:manage"),
        returns_list: true,
        module: Some("integrations"),
    },
    #[cfg(feature = "mod-integrations")]
    CommandPolicy {
        name: "get_outbox_items",
        policy: Policy::Requires("integrations:manage"),
        returns_list: true,
        module: Some("integrations"),
    },
    #[cfg(feature = "mod-integrations")]
    CommandPolicy {
        name: "flush_outbox_now",
        policy: Policy::Requires("integrations:manage"),
        returns_list: false,
        module: Some("integrations"),
    },
    // Data Import / Export (P10)
    CommandPolicy {
        name: "list_entities",
        policy: Policy::Session,
        returns_list: true,
        module: None,
    },
    CommandPolicy {
        name: "get_entity_template",
        policy: Policy::Session,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "export_entity_data",
        policy: Policy::Session,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "parse_import_file",
        policy: Policy::Session,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "preview_import_data",
        policy: Policy::Session,
        returns_list: false,
        module: None,
    },
    CommandPolicy {
        name: "execute_import",
        policy: Policy::Session,
        returns_list: false,
        module: None,
    },
    // Pro licensing commands (FR-8, P11)
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_status",
        policy: Policy::Public,
        returns_list: false,
        module: Some("pro"),
    },
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_server_available",
        policy: Policy::Public,
        returns_list: false,
        module: Some("pro"),
    },
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_register",
        policy: Policy::Public,
        returns_list: false,
        module: Some("pro"),
    },
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_resend_verification",
        policy: Policy::Public,
        returns_list: false,
        module: Some("pro"),
    },
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_forgot_password",
        policy: Policy::Public,
        returns_list: false,
        module: Some("pro"),
    },
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_login",
        policy: Policy::Unlocked,
        returns_list: false,
        module: Some("pro"),
    },
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_start_trial",
        policy: Policy::Unlocked,
        returns_list: false,
        module: Some("pro"),
    },
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_account",
        policy: Policy::Unlocked,
        returns_list: false,
        module: Some("pro"),
    },
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_revoke_device",
        policy: Policy::Unlocked,
        returns_list: false,
        module: Some("pro"),
    },
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_logout",
        policy: Policy::Unlocked,
        returns_list: false,
        module: Some("pro"),
    },
    #[cfg(feature = "mod-pro")]
    CommandPolicy {
        name: "pro_gated_action",
        policy: Policy::Pro,
        returns_list: false,
        module: Some("pro"),
    },
];

pub fn get_command_policy(command_name: &str) -> Option<&'static CommandPolicy> {
    COMMAND_POLICIES.iter().find(|cp| cp.name == command_name)
}

/// Helper method on CommandPolicy to check if the command is enabled for a given list of active modules.
impl CommandPolicy {
    pub fn is_enabled_for_modules(&self, enabled_modules: &[&str]) -> bool {
        match self.module {
            None => true,
            Some(m) => enabled_modules.contains(&m),
        }
    }
}

pub fn check_policy(command_name: &str) -> Result<(), CafError> {
    let policy = match get_command_policy(command_name) {
        Some(cp) => &cp.policy,
        None => return Err(CafError::Auth(AuthError::Generic("unknown command".into()))),
    };

    match policy {
        Policy::Public => Ok(()),
        Policy::Unlocked => {
            if crate::vault::is_unlocked().unwrap_or(false) {
                Ok(())
            } else {
                Err(CafError::Vault(VaultError::Locked))
            }
        }
        Policy::Session => {
            if !crate::vault::is_unlocked().unwrap_or(false) {
                return Err(CafError::Vault(VaultError::Locked));
            }
            crate::session::get_current_session()?;
            Ok(())
        }
        Policy::Requires(permission) => {
            if !crate::vault::is_unlocked().unwrap_or(false) {
                return Err(CafError::Vault(VaultError::Locked));
            }
            let session = crate::session::get_current_session()?;
            if session.has_permission(permission) {
                Ok(())
            } else {
                Err(CafError::Auth(AuthError::PermissionDenied))
            }
        }
        Policy::Pro => {
            if !crate::vault::is_unlocked().unwrap_or(false) {
                return Err(CafError::Vault(VaultError::Locked));
            }
            let _sess = crate::session::get_current_session()?;
            #[cfg(feature = "mod-pro")]
            {
                crate::pro::enforce_pro_gating()?;
                Ok(())
            }
            #[cfg(not(feature = "mod-pro"))]
            {
                Err(CafError::generic(
                    "Pro module is not compiled into this binary",
                ))
            }
        }
    }
}

pub fn guarded<F, R>(command_name: &'static str, f: F) -> Result<R, CafError>
where
    F: FnOnce() -> Result<R, CafError>,
{
    check_policy(command_name)?;
    f()
}

pub fn enforce_policy(
    command_name: &str,
    session: Option<&crate::session::Session>,
) -> Result<(), CafError> {
    let cp = get_command_policy(command_name)
        .ok_or_else(|| CafError::Auth(AuthError::Generic("unknown command".into())))?;
    match &cp.policy {
        Policy::Public => Ok(()),
        Policy::Unlocked => {
            if crate::vault::is_unlocked().unwrap_or(false) {
                Ok(())
            } else {
                Err(CafError::Vault(VaultError::Locked))
            }
        }
        Policy::Session => {
            if !crate::vault::is_unlocked().unwrap_or(false) {
                return Err(CafError::Vault(VaultError::Locked));
            }
            if session.is_some() {
                Ok(())
            } else {
                Err(CafError::Auth(AuthError::Unauthorized))
            }
        }
        Policy::Requires(perm) => {
            if !crate::vault::is_unlocked().unwrap_or(false) {
                return Err(CafError::Vault(VaultError::Locked));
            }
            let s = session.ok_or(CafError::Auth(AuthError::Unauthorized))?;
            if s.has_permission(perm) {
                Ok(())
            } else {
                Err(CafError::Auth(AuthError::PermissionDenied))
            }
        }
        Policy::Pro => {
            if !crate::vault::is_unlocked().unwrap_or(false) {
                return Err(CafError::Vault(VaultError::Locked));
            }
            if session.is_none() {
                return Err(CafError::Auth(AuthError::Unauthorized));
            }
            #[cfg(feature = "mod-pro")]
            {
                crate::pro::enforce_pro_gating()?;
                Ok(())
            }
            #[cfg(not(feature = "mod-pro"))]
            {
                Err(CafError::generic(
                    "Pro module is not compiled into this binary",
                ))
            }
        }
    }
}
