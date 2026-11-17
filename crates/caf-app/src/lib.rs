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
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::verify_pin,
            commands::profile_unlock,
            commands::profile_lock,
            commands::session_info,
            commands::list_roles,
            commands::save_role,
            commands::delete_role,
            commands::list_notes,
            commands::save_note,
            commands::delete_note,
            commands::get_notes_summary,
            commands::get_ai_settings,
            commands::save_ai_settings,
            commands::set_ai_api_key,
            commands::clear_ai_api_key,
            commands::ai_preview_context,
            commands::ai_chat,
            commands::ai_get_providers,
            commands::ai_save_provider,
            commands::ai_delete_provider,
            commands::ai_get_routing_matrix,
            commands::ai_save_routing_rule,
            commands::ai_get_personas,
            commands::ai_save_persona,
            commands::ai_delete_persona,
            commands::ai_get_habits,
            commands::ai_search_habits,
            commands::ai_toggle_habit_pin,
            commands::ai_delete_habit,
            commands::ai_get_skills,
            commands::ai_save_skill,
            commands::ai_delete_skill,
            commands::ai_dispatch_task,
            commands::ai_dispatch_task_with_skill,
            commands::submit_feedback,
            commands::get_pending_crash_report,
            commands::dismiss_crash_report,
            commands::export_encrypted_backup,
            commands::import_encrypted_backup,
            commands::get_performance_prefs,
            commands::set_performance_prefs,
            commands::scheduler_list_jobs,
            commands::scheduler_set_job_enabled,
            commands::scheduler_update_schedule,
            commands::scheduler_run_job_now,
            commands::scheduler_get_history,
            commands::scheduler_get_desktop_settings,
            commands::scheduler_set_desktop_settings,
            commands::list_integrations,
            commands::set_integration_secret,
            commands::delete_integration_secret,
            commands::test_integration_connection,
            commands::get_integration_logs,
            commands::get_outbox_items,
            commands::flush_outbox_now,
            // Data Import / Export (P10)
            commands::list_entities,
            commands::get_entity_template,
            commands::export_entity_data,
            commands::parse_import_file,
            commands::preview_import_data,
            commands::execute_import,
            // Pro licensing
            commands::pro_status,
            commands::pro_server_available,
            commands::pro_register,
            commands::pro_resend_verification,
            commands::pro_forgot_password,
            commands::pro_login,
            commands::pro_start_trial,
            commands::pro_account,
            commands::pro_revoke_device,
            commands::pro_logout,
            commands::pro_gated_action,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
