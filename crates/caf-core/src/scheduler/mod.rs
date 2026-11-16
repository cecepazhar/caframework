//! In-app Scheduler module (FR-16, P08).
//!
//! Provides:
//! - Job registry with presets, intervals, and 5-field cron.
//! - Panic isolation and non-blocking background execution.
//! - Catch-up policy (`Skip` | `RunOnce`) with injectable clock.
//! - Overlap prevention: only one instance per job executes at any time.
//! - Run history with 500-run retention.
//! - Built-in framework jobs (`backup.scheduled`, `backup.overdue_reminder`, `audit.prune`, etc.).
//! - Desktop tray and start-at-login settings.

pub mod builtin;
pub mod parser;
pub mod runtime;

use crate::error::{CafError, DbError};
use once_cell::sync::Lazy;
use parser::{describe_schedule_en, describe_schedule_id};
pub use runtime::{
    CatchUpPolicy, Clock, JobRunRecord, MAX_RUN_HISTORY, RegisteredJob, SchedulerRuntime,
    SystemClock,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct JobStatusDto {
    pub key: String,
    pub title_en: String,
    pub title_id: String,
    pub schedule: String,
    pub description_en: String,
    pub description_id: String,
    pub permission: String,
    pub enabled: bool,
    pub needs_vault: bool,
    pub catch_up: CatchUpPolicy,
    pub max_runtime_secs: u64,
    pub background_safe: bool,
    pub last_run_at: Option<i64>,
    pub next_run_at: Option<i64>,
    pub last_status: Option<String>,
    pub is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS, Default)]
pub struct SchedulerDesktopSettings {
    pub close_to_tray: bool,
    pub start_at_login: bool,
}

static GLOBAL_RUNTIME: Lazy<Arc<SchedulerRuntime>> = Lazy::new(|| {
    let clock = Arc::new(SystemClock);
    let rt = Arc::new(SchedulerRuntime::new(clock));

    // Register built-in jobs
    for job in builtin::make_builtin_jobs() {
        rt.register_job(job);
    }

    rt
});

pub fn get_scheduler_runtime() -> Arc<SchedulerRuntime> {
    GLOBAL_RUNTIME.clone()
}

pub fn list_jobs() -> Vec<JobStatusDto> {
    let rt = get_scheduler_runtime();
    let jobs = rt.jobs.read().clone();
    let overrides = rt.overrides.read().clone();
    let running = rt.running_keys.read().clone();
    let last_runs = rt.last_runs.read().clone();
    let next_runs = rt.next_runs.read().clone();
    let history = rt.history.read().clone();

    let mut result = Vec::new();
    for (key, job) in jobs {
        let schedule = overrides
            .get(&key)
            .and_then(|o| o.schedule.clone())
            .unwrap_or_else(|| job.default_schedule.clone());

        let enabled = overrides.get(&key).and_then(|o| o.enabled).unwrap_or(true);

        let description_en = describe_schedule_en(&schedule);
        let description_id = describe_schedule_id(&schedule);

        let last_status = history
            .iter()
            .find(|rec| rec.job_key == key)
            .map(|rec| rec.status.clone());

        result.push(JobStatusDto {
            key: key.clone(),
            title_en: job.title_en,
            title_id: job.title_id,
            schedule,
            description_en,
            description_id,
            permission: job.permission,
            enabled,
            needs_vault: job.needs_vault,
            catch_up: job.catch_up,
            max_runtime_secs: job.max_runtime_secs,
            background_safe: job.background_safe,
            last_run_at: last_runs.get(&key).copied(),
            next_run_at: next_runs.get(&key).copied(),
            last_status,
            is_running: running.contains(&key),
        });
    }

    result.sort_by(|a, b| a.key.cmp(&b.key));
    result
}

pub fn set_job_enabled(key: &str, enabled: bool) -> Result<(), CafError> {
    let rt = get_scheduler_runtime();
    if !rt.jobs.read().contains_key(key) {
        return Err(CafError::Db(DbError::Generic(format!(
            "job '{key}' not found"
        ))));
    }
    rt.set_job_enabled(key, enabled);
    Ok(())
}

pub fn update_job_schedule(key: &str, schedule: &str) -> Result<(), CafError> {
    let rt = get_scheduler_runtime();
    if !rt.jobs.read().contains_key(key) {
        return Err(CafError::Db(DbError::Generic(format!(
            "job '{key}' not found"
        ))));
    }
    rt.update_schedule(key, schedule)
        .map_err(|e| CafError::Db(DbError::Generic(e)))?;
    Ok(())
}

pub async fn run_job_now(key: &str) -> Result<String, CafError> {
    let rt = get_scheduler_runtime();
    rt.execute_job(key.to_string(), true)
        .await
        .map_err(|e| CafError::Db(DbError::Generic(e)))
}

pub fn get_job_history(key: Option<&str>, limit: usize) -> Vec<JobRunRecord> {
    let rt = get_scheduler_runtime();
    rt.get_history(key, limit)
}

fn desktop_settings_path() -> std::path::PathBuf {
    crate::paths::data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("desktop_settings.json")
}

pub fn get_desktop_settings() -> SchedulerDesktopSettings {
    let path = desktop_settings_path();
    if let Ok(content) = fs::read_to_string(&path)
        && let Ok(settings) = serde_json::from_str::<SchedulerDesktopSettings>(&content)
    {
        return settings;
    }
    SchedulerDesktopSettings::default()
}

pub fn set_desktop_settings(settings: SchedulerDesktopSettings) -> Result<(), CafError> {
    let path = desktop_settings_path();
    let content = serde_json::to_string_pretty(&settings).map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to serialize desktop settings: {e}"
        )))
    })?;

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(&path, content).map_err(|e| {
        CafError::Db(DbError::Generic(format!(
            "failed to write desktop settings: {e}"
        )))
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }

    Ok(())
}
