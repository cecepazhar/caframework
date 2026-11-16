//! Scheduler runtime, runner isolation, panic handling, and history retention.

use crate::scheduler::parser::{compute_next_run, parse_schedule};
use chrono::{DateTime, Local, TimeZone};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub const MAX_RUN_HISTORY: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum CatchUpPolicy {
    Skip,
    RunOnce,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct JobRunRecord {
    pub id: String,
    pub job_key: String,
    pub started_at: i64,
    pub finished_at: i64,
    pub status: String, // "success" | "failed" | "panic" | "timed_out" | "skipped"
    pub message: String,
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Local>;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Local> {
        Local::now()
    }
}

pub struct MockClock {
    current: RwLock<DateTime<Local>>,
}

impl MockClock {
    pub fn new(initial: DateTime<Local>) -> Self {
        Self {
            current: RwLock::new(initial),
        }
    }

    pub fn set_now(&self, new_now: DateTime<Local>) {
        *self.current.write() = new_now;
    }

    pub fn advance(&self, duration: chrono::Duration) {
        let mut w = self.current.write();
        *w += duration;
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Local> {
        *self.current.read()
    }
}

pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type RunnerFn = Arc<dyn Fn() -> BoxedFuture<Result<String, String>> + Send + Sync>;

#[derive(Clone)]
pub struct RegisteredJob {
    pub key: String,
    pub title_en: String,
    pub title_id: String,
    pub default_schedule: String,
    pub permission: String,
    pub needs_vault: bool,
    pub catch_up: CatchUpPolicy,
    pub max_runtime_secs: u64,
    pub background_safe: bool,
    pub runner: RunnerFn,
}

#[derive(Debug, Clone, Default)]
pub struct JobOverride {
    pub schedule: Option<String>,
    pub enabled: Option<bool>,
}

pub struct SchedulerRuntime {
    pub clock: Arc<dyn Clock>,
    pub jobs: RwLock<HashMap<String, RegisteredJob>>,
    pub overrides: RwLock<HashMap<String, JobOverride>>,
    pub running_keys: RwLock<HashSet<String>>,
    pub history: RwLock<VecDeque<JobRunRecord>>,
    pub last_runs: RwLock<HashMap<String, i64>>,
    pub next_runs: RwLock<HashMap<String, i64>>,
}

impl SchedulerRuntime {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            jobs: RwLock::new(HashMap::new()),
            overrides: RwLock::new(HashMap::new()),
            running_keys: RwLock::new(HashSet::new()),
            history: RwLock::new(VecDeque::new()),
            last_runs: RwLock::new(HashMap::new()),
            next_runs: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_job(&self, job: RegisteredJob) {
        let key = job.key.clone();
        let default_sched = job.default_schedule.clone();
        self.jobs.write().insert(key.clone(), job);

        // Calculate initial next run if not set
        if !self.next_runs.read().contains_key(&key) {
            let now = self.clock.now();
            if let Ok(parsed) = parse_schedule(&default_sched)
                && let Some(next) = compute_next_run(&parsed, now)
            {
                self.next_runs.write().insert(key, next.timestamp());
            }
        }
    }

    pub fn get_effective_schedule(&self, key: &str) -> Option<String> {
        let jobs = self.jobs.read();
        let job = jobs.get(key)?;
        let overrides = self.overrides.read();
        let custom = overrides.get(key).and_then(|o| o.schedule.clone());
        Some(custom.unwrap_or_else(|| job.default_schedule.clone()))
    }

    pub fn is_job_enabled(&self, key: &str) -> bool {
        let overrides = self.overrides.read();
        if let Some(o) = overrides.get(key)
            && let Some(e) = o.enabled
        {
            return e;
        }
        true
    }

    pub fn set_job_enabled(&self, key: &str, enabled: bool) {
        let mut overrides = self.overrides.write();
        let entry = overrides.entry(key.to_string()).or_default();
        entry.enabled = Some(enabled);
    }

    pub fn update_schedule(&self, key: &str, new_schedule: &str) -> Result<(), String> {
        let _ = parse_schedule(new_schedule)?;
        let mut overrides = self.overrides.write();
        let entry = overrides.entry(key.to_string()).or_default();
        entry.schedule = Some(new_schedule.to_string());

        // Update next run
        let now = self.clock.now();
        if let Ok(parsed) = parse_schedule(new_schedule)
            && let Some(next) = compute_next_run(&parsed, now)
        {
            self.next_runs
                .write()
                .insert(key.to_string(), next.timestamp());
        }
        Ok(())
    }

    pub fn add_history_record(&self, record: JobRunRecord) {
        let mut h = self.history.write();
        h.push_front(record);
        if h.len() > MAX_RUN_HISTORY {
            h.truncate(MAX_RUN_HISTORY);
        }
    }

    pub fn get_history(&self, key: Option<&str>, limit: usize) -> Vec<JobRunRecord> {
        let h = self.history.read();
        let max_items = if limit == 0 {
            MAX_RUN_HISTORY
        } else {
            limit.min(MAX_RUN_HISTORY)
        };
        h.iter()
            .filter(|rec| key.is_none_or(|k| rec.job_key == k))
            .take(max_items)
            .cloned()
            .collect()
    }

    /// Check and handle missed runs on startup or clock jump.
    pub fn handle_catch_up(&self) -> Vec<String> {
        let now = self.clock.now();
        let now_ts = now.timestamp();
        let mut triggered = Vec::new();

        let jobs = self.jobs.read().clone();
        for (key, job) in jobs {
            if !self.is_job_enabled(&key) {
                continue;
            }

            let last_run = self.last_runs.read().get(&key).copied();
            let sched_str = self
                .get_effective_schedule(&key)
                .unwrap_or(job.default_schedule);
            let parsed = match parse_schedule(&sched_str) {
                Ok(p) => p,
                Err(_) => continue,
            };

            if let Some(last_ts) = last_run
                && let Some(last_dt) = Local.timestamp_opt(last_ts, 0).single()
                && let Some(next_due) = compute_next_run(&parsed, last_dt)
                && next_due.timestamp() <= now_ts
            {
                // Missed run occurred!
                match job.catch_up {
                    CatchUpPolicy::Skip => {
                        // Don't execute; recalculate next run from now
                        if let Some(next_fut) = compute_next_run(&parsed, now) {
                            self.next_runs.write().insert(key, next_fut.timestamp());
                        }
                    }
                    CatchUpPolicy::RunOnce => {
                        triggered.push(key);
                    }
                }
            } else if last_run.is_none() {
                // First initialization
                if let Some(next) = compute_next_run(&parsed, now) {
                    self.next_runs.write().insert(key, next.timestamp());
                }
            }
        }

        triggered
    }

    /// Execute a job safely with panic isolation, timeout enforcement, and overlap prevention.
    pub async fn execute_job(
        self: Arc<Self>,
        key: String,
        is_manual: bool,
    ) -> Result<String, String> {
        let job = {
            let jobs = self.jobs.read();
            jobs.get(&key)
                .cloned()
                .ok_or_else(|| format!("job '{key}' not registered"))?
        };

        // Overlap prevention
        {
            let mut running = self.running_keys.write();
            if running.contains(&key) {
                return Err(format!("job '{key}' is already running"));
            }
            running.insert(key.clone());
        }

        let started_at = self.clock.now().timestamp();
        let runner = job.runner.clone();
        let max_secs = if job.max_runtime_secs == 0 {
            300
        } else {
            job.max_runtime_secs
        };

        // Panic isolation via tokio::spawn
        let handle = tokio::spawn(async move {
            tokio::time::timeout(Duration::from_secs(max_secs), (runner)()).await
        });

        let (status, message, output) = match handle.await {
            Ok(timed_result) => match timed_result {
                Ok(inner_result) => match inner_result {
                    Ok(msg) => ("success".to_string(), msg.clone(), Ok(msg)),
                    Err(err) => ("failed".to_string(), err.clone(), Err(err)),
                },
                Err(_) => {
                    let msg = format!("job '{key}' exceeded max runtime of {max_secs}s");
                    ("timed_out".to_string(), msg.clone(), Err(msg))
                }
            },
            Err(panic_err) => {
                let msg = format!("job '{key}' panicked: {panic_err}");
                ("panic".to_string(), msg.clone(), Err(msg))
            }
        };

        // Remove from running set
        self.running_keys.write().remove(&key);

        let finished_at = self.clock.now().timestamp();
        self.last_runs.write().insert(key.clone(), finished_at);

        // Schedule next run if not manual
        if !is_manual {
            let sched_str = self
                .get_effective_schedule(&key)
                .unwrap_or(job.default_schedule);
            if let Ok(parsed) = parse_schedule(&sched_str)
                && let Some(next) = compute_next_run(&parsed, self.clock.now())
            {
                self.next_runs.write().insert(key.clone(), next.timestamp());
            }
        }

        self.add_history_record(JobRunRecord {
            id: uuid::Uuid::new_v4().to_string(),
            job_key: key,
            started_at,
            finished_at,
            status,
            message,
        });

        output
    }

    /// Periodic tick: checks due jobs and schedules them if vault requirements are satisfied.
    pub fn tick(self: &Arc<Self>) -> Vec<String> {
        let now = self.clock.now();
        let now_ts = now.timestamp();
        let mut to_run = Vec::new();

        let jobs = self.jobs.read().clone();
        let next_runs = self.next_runs.read().clone();

        for (key, job) in jobs {
            if !self.is_job_enabled(&key) {
                continue;
            }

            // Check if vault is needed and currently locked
            if job.needs_vault {
                let is_unlocked = crate::vault::is_unlocked().unwrap_or(false);
                if !is_unlocked {
                    // Vault locked: wait until unlocked, don't execute yet
                    continue;
                }
            }

            if let Some(&next_ts) = next_runs.get(&key)
                && next_ts <= now_ts
                && !self.running_keys.read().contains(&key)
            {
                to_run.push(key);
            }
        }

        to_run
    }
}
