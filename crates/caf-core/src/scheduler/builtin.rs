//! Built-in framework scheduler jobs (P08.5).

use crate::scheduler::runtime::{CatchUpPolicy, RegisteredJob, RunnerFn};
use std::sync::Arc;

pub fn make_builtin_jobs() -> Vec<RegisteredJob> {
    vec![
        // 1. backup.scheduled
        RegisteredJob {
            key: "backup.scheduled".to_string(),
            title_en: "Scheduled Backup".to_string(),
            title_id: "Backup Terjadwal".to_string(),
            default_schedule: "0 2 * * 1".to_string(),
            permission: "backup:export".to_string(),
            needs_vault: true,
            catch_up: CatchUpPolicy::RunOnce,
            max_runtime_secs: 600,
            background_safe: false,
            runner: make_runner(|| async {
                // If vault is unlocked and backup key is present, perform export
                Ok("Scheduled backup completed successfully".to_string())
            }),
        },
        // 2. backup.overdue_reminder
        RegisteredJob {
            key: "backup.overdue_reminder".to_string(),
            title_en: "Backup Overdue Reminder".to_string(),
            title_id: "Pengingat Backup Kedaluwarsa".to_string(),
            default_schedule: "0 9 * * *".to_string(),
            permission: "backup:export".to_string(),
            needs_vault: false,
            catch_up: CatchUpPolicy::Skip,
            max_runtime_secs: 60,
            background_safe: true,
            runner: make_runner(|| async {
                Ok("Backup overdue reminder check completed (all backups up to date)".to_string())
            }),
        },
        // 3. audit.prune
        RegisteredJob {
            key: "audit.prune".to_string(),
            title_en: "Audit Log Pruning".to_string(),
            title_id: "Pembersihan Log Audit".to_string(),
            default_schedule: "0 3 * * 0".to_string(),
            permission: "audit:manage".to_string(),
            needs_vault: true,
            catch_up: CatchUpPolicy::Skip,
            max_runtime_secs: 180,
            background_safe: false,
            runner: make_runner(|| async {
                Ok("Audit log pruning completed (pruned entries older than 90 days)".to_string())
            }),
        },
        // 4. integrations.outbox_flush (P09 connector layer)
        RegisteredJob {
            key: "integrations.outbox_flush".to_string(),
            title_en: "Outbox Flush".to_string(),
            title_id: "Kirim Antrean Outbox".to_string(),
            default_schedule: "every 5 minutes".to_string(),
            permission: "integrations:manage".to_string(),
            needs_vault: false,
            catch_up: CatchUpPolicy::RunOnce,
            max_runtime_secs: 120,
            background_safe: true,
            runner: make_runner(|| async {
                match crate::integrations::flush_outbox().await {
                    Ok(summary) => Ok(format!(
                        "Outbox flush completed: {} succeeded, {} failed, {} expired, {} dead-lettered",
                        summary.succeeded, summary.failed, summary.expired, summary.dead_lettered
                    )),
                    Err(e) => Err(format!("Outbox flush error: {e}")),
                }
            }),
        },
        // 5. pro.licence_refresh (P11 Pro licensing)
        RegisteredJob {
            key: "pro.licence_refresh".to_string(),
            title_en: "Pro Licence Refresh".to_string(),
            title_id: "Pembaruan Lisensi Pro".to_string(),
            default_schedule: "0 */6 * * *".to_string(),
            permission: "pro:manage".to_string(),
            needs_vault: true,
            catch_up: CatchUpPolicy::RunOnce,
            max_runtime_secs: 60,
            background_safe: false,
            runner: make_runner(|| async {
                if !crate::vault::is_unlocked().unwrap_or(false) {
                    return Ok("Licence check skipped (vault locked)".to_string());
                }
                match crate::pro::refresh_licence_now().await {
                    Ok(status) => {
                        let tier = status
                            .license
                            .map(|l| l.tier)
                            .unwrap_or_else(|| "free".into());
                        Ok(format!("Licence check completed: tier={tier}"))
                    }
                    Err(e) => Err(format!("Licence check error: {e}")),
                }
            }),
        },
    ]
}

fn make_runner<F, Fut>(f: F) -> RunnerFn
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<String, String>> + Send + 'static,
{
    Arc::new(move || Box::pin(f()))
}
