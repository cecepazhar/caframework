//! Tauri binding layer for CAFramework.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented
)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::todo,
        clippy::unimplemented
    )
)]

mod commands;
mod window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    run_with_start(std::time::Instant::now());
}

// Reason: Tauri entrypoint sets process-wide TLS environment variables, generates context macro, and initializes runtime loop
#[allow(clippy::expect_used, clippy::disallowed_methods)]
pub fn run_with_start(start: std::time::Instant) {
    caf_core::crash::install_panic_hook();

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            let _ = app.get_webview_window("main").map(|w| w.set_focus());
        }));
    }

    builder
        .setup(move |app| {
            #[cfg(target_os = "android")]
            {
                use tauri::Manager;
                if let Ok(app_data) = app.path().app_data_dir() {
                    let _ = caf_core::paths::set_custom_data_dir(app_data);
                }
            }

            window::create_main_window(app)?;

            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            #[cfg(target_os = "linux")]
            {
                // Fix for non-Debian distros (Fedora/Arch/RHEL): tauri-plugin-updater hardcodes
                // /etc/ssl/certs/ca-certificates.crt which doesn't exist on Fedora. Sanitize it
                // so child processes don't inherit a poisoned SSL_CERT_FILE.
                if let Ok(cert) = std::env::var("SSL_CERT_FILE")
                    && !std::path::Path::new(&cert).exists()
                {
                    let valid_ca = [
                        "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
                        "/etc/pki/tls/cert.pem",
                        "/etc/ssl/ca-bundle.pem",
                        "/etc/ssl/cert.pem",
                    ]
                    .into_iter()
                    .find(|p| std::path::Path::new(p).exists());

                    if let Some(valid) = valid_ca {
                        unsafe {
                            std::env::set_var("SSL_CERT_FILE", valid);
                        }
                    } else {
                        unsafe {
                            std::env::remove_var("SSL_CERT_FILE");
                        }
                    }
                }
            }

            #[cfg(target_os = "linux")]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    window
                        .with_webview(|webview| {
                            use webkit2gtk::{SettingsExt, WebViewExt};
                            let inner = webview.inner();
                            if let Some(settings) = inner.settings() {
                                let enable_gpu = caf_core::prefs::load_performance_prefs()
                                    .map(|p| p.gpu_acceleration)
                                    .unwrap_or(false);
                                settings.set_enable_webgl(enable_gpu);
                            }
                        })
                        .ok();
                }
            }

            println!("CAFRAMEWORK_COLD_START_MS={}", start.elapsed().as_millis());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::window_minimize,
            commands::window_maximize,
            commands::window_close,
            commands::window_start_dragging,
            commands::is_vault_initialized,
            commands::validate_vault_password,
            commands::lock_vault,
            commands::change_master_password,
            commands::reset_vault,
            #[cfg(feature = "mod-profiles")]
            commands::list_profiles,
            #[cfg(feature = "mod-profiles")]
            commands::save_profile,
            #[cfg(feature = "mod-profiles")]
            commands::delete_profile,
            #[cfg(feature = "mod-profiles")]
            commands::verify_pin,
            #[cfg(feature = "mod-profiles")]
            commands::profile_unlock,
            #[cfg(feature = "mod-profiles")]
            commands::profile_lock,
            commands::session_info,
            commands::list_roles,
            commands::save_role,
            commands::delete_role,
            commands::list_notes,
            commands::save_note,
            commands::delete_note,
            commands::get_notes_summary,
            #[cfg(feature = "mod-ai")]
            commands::get_ai_settings,
            #[cfg(feature = "mod-ai")]
            commands::save_ai_settings,
            #[cfg(feature = "mod-ai")]
            commands::set_ai_api_key,
            #[cfg(feature = "mod-ai")]
            commands::clear_ai_api_key,
            #[cfg(feature = "mod-ai")]
            commands::ai_preview_context,
            #[cfg(feature = "mod-ai")]
            commands::ai_chat,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_get_providers,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_save_provider,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_delete_provider,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_get_routing_matrix,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_save_routing_rule,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_get_personas,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_save_persona,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_delete_persona,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_get_habits,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_search_habits,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_toggle_habit_pin,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_delete_habit,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_get_skills,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_save_skill,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_delete_skill,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_dispatch_task,
            #[cfg(feature = "mod-ai-advanced")]
            commands::ai_dispatch_task_with_skill,
            #[cfg(feature = "mod-feedback")]
            commands::submit_feedback,
            #[cfg(feature = "mod-crash")]
            commands::get_pending_crash_report,
            #[cfg(feature = "mod-crash")]
            commands::dismiss_crash_report,
            commands::export_encrypted_backup,
            commands::import_encrypted_backup,
            commands::get_performance_prefs,
            commands::set_performance_prefs,
            #[cfg(feature = "mod-scheduler")]
            commands::scheduler_list_jobs,
            #[cfg(feature = "mod-scheduler")]
            commands::scheduler_set_job_enabled,
            #[cfg(feature = "mod-scheduler")]
            commands::scheduler_update_schedule,
            #[cfg(feature = "mod-scheduler")]
            commands::scheduler_run_job_now,
            #[cfg(feature = "mod-scheduler")]
            commands::scheduler_get_history,
            #[cfg(feature = "mod-scheduler")]
            commands::scheduler_get_desktop_settings,
            #[cfg(feature = "mod-scheduler")]
            commands::scheduler_set_desktop_settings,
            #[cfg(feature = "mod-integrations")]
            commands::list_integrations,
            #[cfg(feature = "mod-integrations")]
            commands::set_integration_secret,
            #[cfg(feature = "mod-integrations")]
            commands::delete_integration_secret,
            #[cfg(feature = "mod-integrations")]
            commands::test_integration_connection,
            #[cfg(feature = "mod-integrations")]
            commands::get_integration_logs,
            #[cfg(feature = "mod-integrations")]
            commands::get_outbox_items,
            #[cfg(feature = "mod-integrations")]
            commands::flush_outbox_now,
            // Data Import / Export (P10)
            commands::list_entities,
            commands::get_entity_template,
            commands::export_entity_data,
            commands::parse_import_file,
            commands::preview_import_data,
            commands::execute_import,
            // Pro licensing
            #[cfg(feature = "mod-pro")]
            commands::pro_status,
            #[cfg(feature = "mod-pro")]
            commands::pro_server_available,
            #[cfg(feature = "mod-pro")]
            commands::pro_register,
            #[cfg(feature = "mod-pro")]
            commands::pro_resend_verification,
            #[cfg(feature = "mod-pro")]
            commands::pro_forgot_password,
            #[cfg(feature = "mod-pro")]
            commands::pro_login,
            #[cfg(feature = "mod-pro")]
            commands::pro_start_trial,
            #[cfg(feature = "mod-pro")]
            commands::pro_account,
            #[cfg(feature = "mod-pro")]
            commands::pro_revoke_device,
            #[cfg(feature = "mod-pro")]
            commands::pro_logout,
            #[cfg(feature = "mod-pro")]
            commands::pro_gated_action,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
