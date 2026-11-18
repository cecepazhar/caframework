// Every #[tauri::command] registered in `invoke_handler!` (src/lib.rs) must be listed here too
// — Tauri v2 only auto-generates the `allow-<command>`/`deny-<command>` permissions that
// `capabilities/default.json` references for commands named in this list.
//
// A command missing from this list still compiles and still registers: it fails only at
// runtime, as `Command <name> not allowed by ACL`, and only on the code path that calls it.
// `caf-core/tests/arch.rs::tauri_command_registry_build_script_and_acl_agree` keeps this
// list, the invoke handler, and capabilities/default.json in lockstep so that can't recur.
const COMMANDS: &[&str] = &[
    // Window controls
    "window_minimize",
    "window_maximize",
    "window_close",
    "window_start_dragging",
    // Vault & Security
    "is_vault_initialized",
    "validate_vault_password",
    "lock_vault",
    "change_master_password",
    "reset_vault",
    // Multi-Profile & RBAC
    #[cfg(feature = "mod-profiles")]
    "list_profiles",
    #[cfg(feature = "mod-profiles")]
    "save_profile",
    #[cfg(feature = "mod-profiles")]
    "delete_profile",
    #[cfg(feature = "mod-profiles")]
    "verify_pin",
    #[cfg(feature = "mod-profiles")]
    "profile_unlock",
    #[cfg(feature = "mod-profiles")]
    "profile_lock",
    "session_info",
    "list_roles",
    "save_role",
    "delete_role",
    // Sample Vertical Slice (Notes)
    "list_notes",
    "save_note",
    "delete_note",
    "get_notes_summary",
    // Generic AI
    #[cfg(feature = "mod-ai")]
    "get_ai_settings",
    #[cfg(feature = "mod-ai")]
    "save_ai_settings",
    #[cfg(feature = "mod-ai")]
    "set_ai_api_key",
    #[cfg(feature = "mod-ai")]
    "clear_ai_api_key",
    #[cfg(feature = "mod-ai")]
    "ai_preview_context",
    #[cfg(feature = "mod-ai")]
    "ai_chat",
    // Unified Master AI Engine
    #[cfg(feature = "mod-ai-advanced")]
    "ai_get_providers",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_save_provider",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_delete_provider",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_get_routing_matrix",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_save_routing_rule",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_get_personas",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_save_persona",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_delete_persona",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_get_habits",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_search_habits",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_toggle_habit_pin",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_delete_habit",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_get_skills",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_save_skill",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_delete_skill",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_dispatch_task",
    #[cfg(feature = "mod-ai-advanced")]
    "ai_dispatch_task_with_skill",
    // Feedback & Crash Reporting
    #[cfg(feature = "mod-feedback")]
    "submit_feedback",
    #[cfg(feature = "mod-crash")]
    "get_pending_crash_report",
    #[cfg(feature = "mod-crash")]
    "dismiss_crash_report",
    // Backup & Restore
    "export_encrypted_backup",
    "import_encrypted_backup",
    // Preferences
    "get_performance_prefs",
    "set_performance_prefs",
    // Scheduler (P08)
    #[cfg(feature = "mod-scheduler")]
    "scheduler_list_jobs",
    #[cfg(feature = "mod-scheduler")]
    "scheduler_set_job_enabled",
    #[cfg(feature = "mod-scheduler")]
    "scheduler_update_schedule",
    #[cfg(feature = "mod-scheduler")]
    "scheduler_run_job_now",
    #[cfg(feature = "mod-scheduler")]
    "scheduler_get_history",
    #[cfg(feature = "mod-scheduler")]
    "scheduler_get_desktop_settings",
    #[cfg(feature = "mod-scheduler")]
    "scheduler_set_desktop_settings",
    // Integrations & Outbox (P09)
    #[cfg(feature = "mod-integrations")]
    "list_integrations",
    #[cfg(feature = "mod-integrations")]
    "set_integration_secret",
    #[cfg(feature = "mod-integrations")]
    "delete_integration_secret",
    #[cfg(feature = "mod-integrations")]
    "test_integration_connection",
    #[cfg(feature = "mod-integrations")]
    "get_integration_logs",
    #[cfg(feature = "mod-integrations")]
    "get_outbox_items",
    #[cfg(feature = "mod-integrations")]
    "flush_outbox_now",
    // Data Import / Export (P10)
    "list_entities",
    "get_entity_template",
    "export_entity_data",
    "parse_import_file",
    "preview_import_data",
    "execute_import",
    // Pro licensing
    #[cfg(feature = "mod-pro")]
    "pro_status",
    #[cfg(feature = "mod-pro")]
    "pro_server_available",
    #[cfg(feature = "mod-pro")]
    "pro_register",
    #[cfg(feature = "mod-pro")]
    "pro_resend_verification",
    #[cfg(feature = "mod-pro")]
    "pro_forgot_password",
    #[cfg(feature = "mod-pro")]
    "pro_login",
    #[cfg(feature = "mod-pro")]
    "pro_start_trial",
    #[cfg(feature = "mod-pro")]
    "pro_account",
    #[cfg(feature = "mod-pro")]
    "pro_revoke_device",
    #[cfg(feature = "mod-pro")]
    "pro_logout",
    #[cfg(feature = "mod-pro")]
    "pro_gated_action",
    // Runtime Feature Flags (A-03 / P14)
    "list_feature_flags",
    "get_feature_flag",
    "set_feature_flag",
];

fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("gagal menjalankan tauri-build");
}
