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
}

pub const COMMAND_POLICIES: &[CommandPolicy] = &[
    // Window management (Public)
    CommandPolicy {
        name: "window_minimize",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "window_maximize",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "window_close",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "window_start_dragging",
        policy: Policy::Public,
        returns_list: false,
    },
    // Vault commands
    CommandPolicy {
        name: "is_vault_initialized",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "validate_vault_password",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "lock_vault",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "change_master_password",
        policy: Policy::Requires("security:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "reset_vault",
        policy: Policy::Requires("security:manage"),
        returns_list: false,
    },
    // Profiles
    CommandPolicy {
        name: "list_profiles",
        policy: Policy::Unlocked,
        returns_list: true,
    },
    CommandPolicy {
        name: "save_profile",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "delete_profile",
        policy: Policy::Requires("users:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "verify_pin",
        policy: Policy::Unlocked,
        returns_list: false,
    },
    CommandPolicy {
        name: "profile_unlock",
        policy: Policy::Unlocked,
        returns_list: false,
    },
    CommandPolicy {
        name: "profile_lock",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "session_info",
        policy: Policy::Public,
        returns_list: false,
    },
    // RBAC
    CommandPolicy {
        name: "list_roles",
        policy: Policy::Session,
        returns_list: true,
    },
    CommandPolicy {
        name: "save_role",
        policy: Policy::Requires("roles:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "delete_role",
        policy: Policy::Requires("roles:manage"),
        returns_list: false,
    },
    // Notes
    CommandPolicy {
        name: "list_notes",
        policy: Policy::Session,
        returns_list: true,
    },
    CommandPolicy {
        name: "save_note",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "delete_note",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "get_notes_summary",
        policy: Policy::Session,
        returns_list: true,
    },
    // AI
    CommandPolicy {
        name: "get_ai_settings",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "save_ai_settings",
        policy: Policy::Requires("settings:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "set_ai_api_key",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "clear_ai_api_key",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "ai_preview_context",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "ai_chat",
        policy: Policy::Session,
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_get_providers",
        policy: Policy::Session,
        returns_list: true,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_save_provider",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_delete_provider",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_get_routing_matrix",
        policy: Policy::Session,
        returns_list: true,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_save_routing_rule",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_get_personas",
        policy: Policy::Session,
        returns_list: true,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_save_persona",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_delete_persona",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_get_habits",
        policy: Policy::Session,
        returns_list: true,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_search_habits",
        policy: Policy::Session,
        returns_list: true,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_toggle_habit_pin",
        policy: Policy::Session,
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_delete_habit",
        policy: Policy::Session,
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_get_skills",
        policy: Policy::Session,
        returns_list: true,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_save_skill",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_delete_skill",
        policy: Policy::Requires("ai:manage"),
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_dispatch_task",
        policy: Policy::Session,
        returns_list: false,
    },
    #[cfg(feature = "mod-ai-advanced")]
    CommandPolicy {
        name: "ai_dispatch_task_with_skill",
        policy: Policy::Session,
        returns_list: false,
    },
    // Feedback & Crash
    CommandPolicy {
        name: "submit_feedback",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "get_pending_crash_report",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "dismiss_crash_report",
        policy: Policy::Public,
        returns_list: false,
    },
    // Backup
    CommandPolicy {
        name: "export_encrypted_backup",
        policy: Policy::Requires("backup:export"),
        returns_list: false,
    },
    CommandPolicy {
        name: "import_encrypted_backup",
        policy: Policy::Requires("backup:import"),
        returns_list: false,
    },
    // Preferences
    CommandPolicy {
        name: "get_performance_prefs",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "set_performance_prefs",
        policy: Policy::Session,
        returns_list: false,
    },
    // Scheduler (P08)
    CommandPolicy {
        name: "scheduler_list_jobs",
        policy: Policy::Requires("scheduler:view"),
        returns_list: true,
    },
    CommandPolicy {
        name: "scheduler_set_job_enabled",
        policy: Policy::Requires("scheduler:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "scheduler_update_schedule",
        policy: Policy::Requires("scheduler:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "scheduler_run_job_now",
        policy: Policy::Requires("scheduler:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "scheduler_get_history",
        policy: Policy::Requires("scheduler:view"),
        returns_list: true,
    },
    CommandPolicy {
        name: "scheduler_get_desktop_settings",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "scheduler_set_desktop_settings",
        policy: Policy::Requires("settings:manage"),
        returns_list: false,
    },
    // Integrations & Outbox (P09)
    CommandPolicy {
        name: "list_integrations",
        policy: Policy::Session,
        returns_list: true,
    },
    CommandPolicy {
        name: "set_integration_secret",
        policy: Policy::Requires("integrations:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "delete_integration_secret",
        policy: Policy::Requires("integrations:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "test_integration_connection",
        policy: Policy::Requires("integrations:manage"),
        returns_list: false,
    },
    CommandPolicy {
        name: "get_integration_logs",
        policy: Policy::Requires("integrations:manage"),
        returns_list: true,
    },
    CommandPolicy {
        name: "get_outbox_items",
        policy: Policy::Requires("integrations:manage"),
        returns_list: true,
    },
    CommandPolicy {
        name: "flush_outbox_now",
        policy: Policy::Requires("integrations:manage"),
        returns_list: false,
    },
    // Data Import / Export (P10)
    CommandPolicy {
        name: "list_entities",
        policy: Policy::Session,
        returns_list: true,
    },
    CommandPolicy {
        name: "get_entity_template",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "export_entity_data",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "parse_import_file",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "preview_import_data",
        policy: Policy::Session,
        returns_list: false,
    },
    CommandPolicy {
        name: "execute_import",
        policy: Policy::Session,
        returns_list: false,
    },
    // Pro licensing commands (FR-8, P11)
    CommandPolicy {
        name: "pro_status",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "pro_server_available",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "pro_register",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "pro_resend_verification",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "pro_forgot_password",
        policy: Policy::Public,
        returns_list: false,
    },
    CommandPolicy {
        name: "pro_login",
        policy: Policy::Unlocked,
        returns_list: false,
    },
    CommandPolicy {
        name: "pro_start_trial",
        policy: Policy::Unlocked,
        returns_list: false,
    },
    CommandPolicy {
        name: "pro_account",
        policy: Policy::Unlocked,
        returns_list: false,
    },
    CommandPolicy {
        name: "pro_revoke_device",
        policy: Policy::Unlocked,
        returns_list: false,
    },
    CommandPolicy {
        name: "pro_logout",
        policy: Policy::Unlocked,
        returns_list: false,
    },
    CommandPolicy {
        name: "pro_gated_action",
        policy: Policy::Pro,
        returns_list: false,
    },
];

pub fn get_command_policy(command_name: &str) -> Option<&'static CommandPolicy> {
    COMMAND_POLICIES.iter().find(|cp| cp.name == command_name)
}

pub fn check_policy(command_name: &'static str) -> Result<(), CafError> {
    let policy = match get_command_policy(command_name) {
        Some(cp) => &cp.policy,
        None => return Err(CafError::Auth(AuthError::Unauthorized)),
    };

    match policy {
        Policy::Public => Ok(()),
        Policy::Unlocked => {
            if !crate::vault::is_unlocked().unwrap_or(false) {
                return Err(CafError::Vault(VaultError::Locked));
            }
            Ok(())
        }
        Policy::Session => {
            if !crate::vault::is_unlocked().unwrap_or(false) {
                return Err(CafError::Vault(VaultError::Locked));
            }
            let _sess = crate::session::get_current_session()?;
            Ok(())
        }
        Policy::Requires(perm) => {
            if !crate::vault::is_unlocked().unwrap_or(false) {
                return Err(CafError::Vault(VaultError::Locked));
            }
            let sess = crate::session::get_current_session()?;
            if sess.has_permission(perm)
                || sess.has_permission("*")
                || crate::rbac::is_super_role(&sess.role)
            {
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
            crate::pro::enforce_pro_gating()?;
            Ok(())
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
