//! Thin Tauri command bindings for CAFramework.

use caf_core::api::policy::guarded;
use caf_core::{
    CafError, ai, backup, crash, feedback, notes, prefs, profiles, rbac, session, vault,
};

async fn run_blocking<F, R>(f: F) -> Result<R, CafError>
where
    F: FnOnce() -> Result<R, CafError> + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f).await.map_err(|e| {
        caf_core::CafError::Io(caf_core::error::IoError::Generic(format!(
            "Task join error: {e}"
        )))
    })?
}

async fn run_guarded<F, R>(command_name: &'static str, f: F) -> Result<R, CafError>
where
    F: FnOnce() -> Result<R, CafError> + Send + 'static,
    R: Send + 'static,
{
    run_blocking(move || guarded(command_name, f)).await
}

// Window management commands
#[tauri::command]
pub async fn window_minimize(#[allow(unused_variables)] window: tauri::Window) {
    let _ = guarded("window_minimize", || {
        #[cfg(not(target_os = "android"))]
        let _ = window.minimize();
        Ok(())
    });
}

#[tauri::command]
pub async fn window_maximize(#[allow(unused_variables)] window: tauri::Window) {
    let _ = guarded("window_maximize", || {
        #[cfg(not(target_os = "android"))]
        match window.is_maximized() {
            Ok(true) => {
                let _ = window.unmaximize();
            }
            _ => {
                let _ = window.maximize();
            }
        }
        Ok(())
    });
}

#[tauri::command]
pub async fn window_close(#[allow(unused_variables)] window: tauri::Window) {
    let _ = guarded("window_close", || {
        #[cfg(not(target_os = "android"))]
        let _ = window.close();
        Ok(())
    });
}

#[tauri::command]
pub async fn window_start_dragging(#[allow(unused_variables)] window: tauri::Window) {
    let _ = guarded("window_start_dragging", || {
        #[cfg(not(target_os = "android"))]
        let _ = window.start_dragging();
        Ok(())
    });
}

// Vault commands
#[tauri::command]
pub async fn is_vault_initialized() -> Result<bool, CafError> {
    run_guarded("is_vault_initialized", vault::is_vault_initialized).await
}

#[tauri::command]
pub async fn validate_vault_password(password: String) -> Result<bool, CafError> {
    run_guarded("validate_vault_password", move || {
        vault::validate_password(&password)
    })
    .await
}

#[tauri::command]
pub async fn lock_vault() -> Result<(), CafError> {
    run_guarded("lock_vault", move || {
        vault::lock();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn change_master_password(
    old_password: String,
    new_password: String,
) -> Result<(), CafError> {
    run_guarded("change_master_password", move || {
        vault::change_master_password(&old_password, &new_password)
    })
    .await
}

#[tauri::command]
pub async fn reset_vault() -> Result<(), CafError> {
    run_guarded("reset_vault", vault::reset_vault).await
}

// Profile commands
#[tauri::command]
pub async fn list_profiles() -> Result<Vec<profiles::ProfileRecord>, CafError> {
    run_guarded("list_profiles", profiles::list_profiles).await
}

#[tauri::command]
pub async fn save_profile(
    input: profiles::ProfileInput,
) -> Result<profiles::ProfileRecord, CafError> {
    run_guarded("save_profile", move || profiles::save_profile(input)).await
}

#[tauri::command]
pub async fn delete_profile(id: String) -> Result<(), CafError> {
    run_guarded("delete_profile", move || profiles::delete_profile(&id)).await
}

#[tauri::command]
pub async fn verify_pin(profile_id: String, pin: String) -> Result<bool, CafError> {
    run_guarded("verify_pin", move || {
        profiles::verify_pin(&profile_id, &pin)
    })
    .await
}

#[tauri::command]
pub async fn profile_unlock(
    profile_id: String,
    pin: String,
) -> Result<session::SessionInfo, CafError> {
    run_guarded("profile_unlock", move || {
        session::profile_unlock(&profile_id, &pin)
    })
    .await
}

#[tauri::command]
pub async fn profile_lock() -> Result<(), CafError> {
    run_guarded("profile_lock", session::profile_lock).await
}

#[tauri::command]
pub async fn session_info() -> Result<session::SessionInfo, CafError> {
    run_guarded("session_info", session::session_info).await
}

// RBAC commands
#[tauri::command]
pub async fn list_roles() -> Result<Vec<rbac::RoleRecord>, CafError> {
    run_guarded("list_roles", rbac::list_roles).await
}

#[tauri::command]
pub async fn save_role(input: rbac::RoleInput) -> Result<rbac::RoleRecord, CafError> {
    run_guarded("save_role", move || {
        let caller_id = session::current_profile_id()?;
        let key = input.key.clone();
        if let Some(_existing) = rbac::get_role(&key)? {
            rbac::update_role(&key, input, &caller_id)
        } else {
            rbac::create_role(input, &caller_id)
        }
    })
    .await
}

#[tauri::command]
pub async fn delete_role(key: String) -> Result<(), CafError> {
    run_guarded("delete_role", move || {
        let caller_id = session::current_profile_id()?;
        rbac::delete_role(&key, &caller_id)
    })
    .await
}

// Notes sample commands (Vertical slice)
#[tauri::command]
pub async fn list_notes() -> Result<Vec<notes::NoteRecord>, CafError> {
    run_guarded("list_notes", notes::list_notes).await
}

#[tauri::command]
pub async fn save_note(input: notes::NoteInput) -> Result<notes::NoteRecord, CafError> {
    run_guarded("save_note", move || notes::save_note(input)).await
}

#[tauri::command]
pub async fn delete_note(id: String) -> Result<(), CafError> {
    run_guarded("delete_note", move || notes::delete_note(&id)).await
}

#[tauri::command]
pub async fn get_notes_summary() -> Result<Vec<caf_core::visibility::OwnerSummary>, CafError> {
    run_guarded("get_notes_summary", notes::get_notes_summary).await
}

// AI commands
#[tauri::command]
pub async fn get_ai_settings() -> Result<ai::AiSettings, CafError> {
    run_guarded("get_ai_settings", ai::get_settings).await
}

#[tauri::command]
pub async fn save_ai_settings(settings: ai::AiSettings) -> Result<(), CafError> {
    run_guarded("save_ai_settings", move || ai::save_settings(&settings)).await
}

#[tauri::command]
pub async fn set_ai_api_key(api_key: String) -> Result<(), CafError> {
    run_guarded("set_ai_api_key", move || ai::set_ai_api_key(&api_key)).await
}

#[tauri::command]
pub async fn clear_ai_api_key() -> Result<(), CafError> {
    run_guarded("clear_ai_api_key", ai::clear_ai_api_key).await
}

#[tauri::command]
pub async fn ai_preview_context(level: ai::PrivacyLevel) -> Result<ai::ContextPayload, CafError> {
    run_guarded("ai_preview_context", move || {
        ai::preview_context_for_current_session(level)
    })
    .await
}

#[tauri::command]
pub async fn ai_chat(
    prompt: String,
    privacy_level: Option<ai::PrivacyLevel>,
    consent_given: Option<bool>,
) -> Result<ai::AiChatResponse, CafError> {
    run_guarded("ai_chat", move || {
        let session = caf_core::session::get_current_session()?;
        ai::chat(
            &session,
            &prompt,
            privacy_level,
            consent_given.unwrap_or(false),
        )
    })
    .await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_get_providers() -> Result<Vec<ai::AiProviderSummary>, CafError> {
    run_guarded("ai_get_providers", ai::get_all_providers).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_save_provider(provider: ai::AiProviderConfig) -> Result<(), CafError> {
    run_guarded("ai_save_provider", move || ai::save_ai_provider(provider)).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_delete_provider(id: String) -> Result<(), CafError> {
    run_guarded("ai_delete_provider", move || ai::delete_ai_provider(&id)).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_get_routing_matrix() -> Result<Vec<ai::TaskRouteRule>, CafError> {
    run_guarded("ai_get_routing_matrix", ai::get_task_routing_matrix).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_save_routing_rule(rule: ai::TaskRouteRule) -> Result<(), CafError> {
    run_guarded("ai_save_routing_rule", move || {
        ai::save_task_routing_rule(rule)
    })
    .await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_get_personas() -> Result<Vec<ai::SystemPersona>, CafError> {
    run_guarded("ai_get_personas", ai::get_user_personas).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_save_persona(persona: ai::SystemPersona) -> Result<(), CafError> {
    run_guarded("ai_save_persona", move || ai::save_user_persona(persona)).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_delete_persona(id: String) -> Result<(), CafError> {
    run_guarded("ai_delete_persona", move || ai::delete_user_persona(&id)).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_get_habits() -> Result<Vec<ai::HabitFact>, CafError> {
    run_guarded("ai_get_habits", ai::get_habit_memories).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_search_habits(
    query: String,
    limit: Option<usize>,
) -> Result<Vec<ai::HabitFact>, CafError> {
    run_guarded("ai_search_habits", move || {
        ai::search_habit_memories(&query, limit.unwrap_or(20))
    })
    .await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_toggle_habit_pin(id: String) -> Result<bool, CafError> {
    run_guarded("ai_toggle_habit_pin", move || {
        ai::toggle_habit_pin_status(&id)
    })
    .await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_delete_habit(id: String) -> Result<(), CafError> {
    run_guarded("ai_delete_habit", move || ai::delete_habit_memory(&id)).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_get_skills() -> Result<Vec<ai::CustomSkill>, CafError> {
    run_guarded("ai_get_skills", ai::get_all_skills).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_save_skill(skill: ai::CustomSkill) -> Result<(), CafError> {
    run_guarded("ai_save_skill", move || ai::save_custom_skill(skill)).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_delete_skill(id: String) -> Result<(), CafError> {
    run_guarded("ai_delete_skill", move || ai::delete_custom_skill(&id)).await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_dispatch_task(
    task_type: String,
    query: String,
    terminal_ctx: Option<ai::TerminalContext>,
) -> Result<ai::DispatchResult, CafError> {
    run_guarded("ai_dispatch_task", move || {
        let tt = ai::TaskType::from_str_opt(&task_type).unwrap_or(ai::TaskType::Chat);
        ai::dispatch_task(tt, &query, terminal_ctx)
    })
    .await
}

#[cfg(feature = "mod-ai-advanced")]
#[tauri::command]
pub async fn ai_dispatch_task_with_skill(
    task_type: String,
    query: String,
    skill_name: Option<String>,
    terminal_ctx: Option<ai::TerminalContext>,
) -> Result<ai::DispatchResult, CafError> {
    run_guarded("ai_dispatch_task_with_skill", move || {
        let tt = ai::TaskType::from_str_opt(&task_type).unwrap_or(ai::TaskType::Chat);
        ai::dispatch_task_with_skill(tt, &query, skill_name, terminal_ctx)
    })
    .await
}

// Feedback & Crash commands
#[tauri::command]
pub async fn submit_feedback(
    rating: i32,
    content: String,
    name: Option<String>,
    profession: Option<String>,
) -> Result<(), CafError> {
    run_guarded("submit_feedback", move || {
        feedback::submit_feedback(rating, &content, name.as_deref(), profession.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn get_pending_crash_report() -> Result<Option<crash::ScrubbedCrashReport>, CafError> {
    run_guarded("get_pending_crash_report", crash::pending_crash_report).await
}

#[tauri::command]
pub async fn dismiss_crash_report(id: String, never_again: Option<bool>) -> Result<(), CafError> {
    run_guarded("dismiss_crash_report", move || {
        crash::dismiss_crash_report(&id, never_again.unwrap_or(false))
    })
    .await
}

// Backup commands
#[tauri::command]
pub async fn export_encrypted_backup(password: String) -> Result<Vec<u8>, CafError> {
    run_guarded("export_encrypted_backup", move || {
        backup::export_backup(&password)
    })
    .await
}

#[tauri::command]
pub async fn import_encrypted_backup(data: Vec<u8>, password: String) -> Result<(), CafError> {
    run_guarded("import_encrypted_backup", move || {
        backup::import_backup(&data, &password)
    })
    .await
}

// Performance & Preferences
#[tauri::command]
pub async fn get_performance_prefs() -> Result<prefs::PerformancePrefs, CafError> {
    run_guarded("get_performance_prefs", prefs::load_performance_prefs).await
}

#[tauri::command]
pub async fn set_performance_prefs(prefs: prefs::PerformancePrefs) -> Result<(), CafError> {
    run_guarded("set_performance_prefs", move || {
        prefs::save_performance_prefs(&prefs)
    })
    .await
}

// Scheduler commands (P08)
#[tauri::command]
pub async fn scheduler_list_jobs() -> Result<Vec<caf_core::scheduler::JobStatusDto>, CafError> {
    run_guarded("scheduler_list_jobs", || {
        Ok(caf_core::scheduler::list_jobs())
    })
    .await
}

#[tauri::command]
pub async fn scheduler_set_job_enabled(key: String, enabled: bool) -> Result<(), CafError> {
    run_guarded("scheduler_set_job_enabled", move || {
        caf_core::scheduler::set_job_enabled(&key, enabled)
    })
    .await
}

#[tauri::command]
pub async fn scheduler_update_schedule(key: String, schedule: String) -> Result<(), CafError> {
    run_guarded("scheduler_update_schedule", move || {
        caf_core::scheduler::update_job_schedule(&key, &schedule)
    })
    .await
}

#[tauri::command]
pub async fn scheduler_run_job_now(key: String) -> Result<String, CafError> {
    guarded("scheduler_run_job_now", || Ok(()))?;
    caf_core::scheduler::run_job_now(&key).await
}

#[tauri::command]
pub async fn scheduler_get_history(
    key: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<caf_core::scheduler::JobRunRecord>, CafError> {
    run_guarded("scheduler_get_history", move || {
        Ok(caf_core::scheduler::get_job_history(
            key.as_deref(),
            limit.unwrap_or(50),
        ))
    })
    .await
}

#[tauri::command]
pub async fn scheduler_get_desktop_settings()
-> Result<caf_core::scheduler::SchedulerDesktopSettings, CafError> {
    run_guarded("scheduler_get_desktop_settings", || {
        Ok(caf_core::scheduler::get_desktop_settings())
    })
    .await
}

#[tauri::command]
pub async fn scheduler_set_desktop_settings(
    settings: caf_core::scheduler::SchedulerDesktopSettings,
) -> Result<(), CafError> {
    run_guarded("scheduler_set_desktop_settings", move || {
        caf_core::scheduler::set_desktop_settings(settings)
    })
    .await
}

// Integrations & Outbox (P09)
#[tauri::command]
pub async fn list_integrations()
-> Result<Vec<caf_core::integrations::IntegrationStatusDto>, CafError> {
    run_guarded("list_integrations", || {
        caf_core::integrations::list_integrations()
    })
    .await
}

#[tauri::command]
pub async fn set_integration_secret(key: String, secret: String) -> Result<(), CafError> {
    run_guarded("set_integration_secret", move || {
        caf_core::integrations::secrets::set_integration_secret(&key, &secret)
    })
    .await
}

#[tauri::command]
pub async fn delete_integration_secret(key: String) -> Result<(), CafError> {
    run_guarded("delete_integration_secret", move || {
        caf_core::integrations::secrets::delete_integration_secret(&key)
    })
    .await
}

#[tauri::command]
pub async fn test_integration_connection(
    key: String,
) -> Result<caf_core::integrations::TestConnectionResult, CafError> {
    guarded("test_integration_connection", || Ok(()))?;
    caf_core::integrations::test_integration_connection(&key).await
}

#[tauri::command]
pub async fn get_integration_logs(
    key: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<caf_core::integrations::IntegrationRequestLogRecord>, CafError> {
    run_guarded("get_integration_logs", move || {
        Ok(caf_core::integrations::get_request_logs(
            key.as_deref(),
            limit.unwrap_or(100),
        ))
    })
    .await
}

#[tauri::command]
pub async fn get_outbox_items(
    status: Option<caf_core::integrations::OutboxStatus>,
    limit: Option<usize>,
) -> Result<Vec<caf_core::integrations::OutboxItem>, CafError> {
    run_guarded("get_outbox_items", move || {
        Ok(caf_core::integrations::get_outbox_items(
            status,
            limit.unwrap_or(100),
        ))
    })
    .await
}

#[tauri::command]
pub async fn flush_outbox_now() -> Result<caf_core::integrations::outbox::FlushSummary, CafError> {
    guarded("flush_outbox_now", || Ok(()))?;
    caf_core::integrations::flush_outbox().await
}

// Data Import / Export commands (P10)
#[tauri::command]
pub async fn list_entities() -> Result<Vec<caf_core::data_io::EntityDefinition>, CafError> {
    run_guarded("list_entities", || Ok(caf_core::data_io::list_entities())).await
}

#[tauri::command]
pub async fn get_entity_template(entity_key: String, format: String) -> Result<Vec<u8>, CafError> {
    run_guarded("get_entity_template", move || {
        caf_core::data_io::generate_entity_template(&entity_key, &format)
    })
    .await
}

#[tauri::command]
pub async fn export_entity_data(
    entity_key: String,
    format: String,
) -> Result<caf_core::data_io::ExportResult, CafError> {
    run_guarded("export_entity_data", move || {
        caf_core::data_io::export_entity(&entity_key, &format)
    })
    .await
}

#[tauri::command]
pub async fn parse_import_file(
    entity_key: String,
    format: String,
    data: Vec<u8>,
) -> Result<caf_core::data_io::ImportFileInfo, CafError> {
    run_guarded("parse_import_file", move || {
        caf_core::data_io::parse_import_file(&entity_key, &format, &data)
    })
    .await
}

#[tauri::command]
pub async fn preview_import_data(
    entity_key: String,
    format: String,
    data: Vec<u8>,
    mapping: std::collections::HashMap<String, String>,
    mode: caf_core::data_io::ImportMode,
) -> Result<caf_core::data_io::ImportPreview, CafError> {
    run_guarded("preview_import_data", move || {
        caf_core::data_io::preview_import(&entity_key, &format, &data, &mapping, mode)
    })
    .await
}

#[tauri::command]
pub async fn execute_import(
    entity_key: String,
    format: String,
    data: Vec<u8>,
    mapping: std::collections::HashMap<String, String>,
    mode: caf_core::data_io::ImportMode,
    strategy: caf_core::data_io::ImportStrategy,
) -> Result<caf_core::data_io::ImportExecutionReport, CafError> {
    run_guarded("execute_import", move || {
        caf_core::data_io::execute_import(&entity_key, &format, &data, &mapping, mode, strategy)
    })
    .await
}

// Pro licensing commands (FR-8, P11)
#[tauri::command]
pub async fn pro_status() -> Result<caf_core::pro::ProStatus, CafError> {
    run_guarded("pro_status", caf_core::pro::get_pro_status).await
}

#[tauri::command]
pub async fn pro_server_available() -> Result<bool, CafError> {
    guarded("pro_server_available", || Ok(()))?;
    caf_core::pro::pro_server_available().await
}

#[tauri::command]
pub async fn pro_register(
    email: String,
    password: Option<String>,
    name: Option<String>,
    locale: Option<String>,
) -> Result<(), CafError> {
    guarded("pro_register", || Ok(()))?;
    caf_core::pro::pro_register(email, password, name, locale).await
}

#[tauri::command]
pub async fn pro_resend_verification(email: String) -> Result<(), CafError> {
    guarded("pro_resend_verification", || Ok(()))?;
    caf_core::pro::pro_resend_verification(email).await
}

#[tauri::command]
pub async fn pro_forgot_password(email: String, locale: Option<String>) -> Result<(), CafError> {
    guarded("pro_forgot_password", || Ok(()))?;
    caf_core::pro::pro_forgot_password(email, locale).await
}

#[tauri::command]
pub async fn pro_login(
    email: String,
    password: Option<String>,
) -> Result<caf_core::pro::ProAccount, CafError> {
    guarded("pro_login", || Ok(()))?;
    caf_core::pro::pro_login(email, password).await
}

#[tauri::command]
pub async fn pro_start_trial() -> Result<caf_core::pro::SyncOutcome, CafError> {
    guarded("pro_start_trial", || Ok(()))?;
    caf_core::pro::pro_start_trial().await
}

#[tauri::command]
pub async fn pro_account() -> Result<caf_core::pro::AccountDetails, CafError> {
    guarded("pro_account", || Ok(()))?;
    caf_core::pro::pro_account().await
}

#[tauri::command]
pub async fn pro_revoke_device(device_id: String) -> Result<(), CafError> {
    guarded("pro_revoke_device", || Ok(()))?;
    caf_core::pro::pro_revoke_device(device_id).await
}

#[tauri::command]
pub async fn pro_logout() -> Result<(), CafError> {
    run_guarded("pro_logout", caf_core::pro::pro_logout).await
}

#[tauri::command]
pub async fn pro_gated_action() -> Result<String, CafError> {
    run_guarded("pro_gated_action", caf_core::pro::pro_gated_action).await
}
