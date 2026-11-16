//! Acceptance tests for P08 (Scheduler):
//! - P08.1 Parser property tests (>= 10,000 cases) and description golden tests in both languages.
//! - P08.2 Runtime: panic isolation, overlap prevention, vault wait, retention 500.
//! - P08.3 Catch-up: injectable clock with Skip and RunOnce policies.
//! - P08.4 Desktop tray and start-at-login settings.
//! - P08.5 Built-in jobs coverage.

use caf_core::scheduler::builtin::make_builtin_jobs;
use caf_core::scheduler::parser::{
    compute_next_run, describe_schedule_en, describe_schedule_id, parse_schedule,
};
use caf_core::scheduler::runtime::{
    CatchUpPolicy, Clock, JobRunRecord, MAX_RUN_HISTORY, MockClock, RegisteredJob, SchedulerRuntime,
};
use caf_core::scheduler::{SchedulerDesktopSettings, get_desktop_settings, set_desktop_settings};
use chrono::{Duration, Local, TimeZone};
use proptest::prelude::*;
use std::sync::Arc;

// --- P08.1 Parser Property Tests (>= 10 000 cases) ---

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    #[test]
    fn test_p08_1_parser_property_random_strings(s in "\\PC*") {
        // Must never panic on arbitrary input
        let res = parse_schedule(&s);
        if let Ok(parsed) = res {
            // If parsed successfully, next run must not panic
            let now = Local::now();
            let _ = compute_next_run(&parsed, now);
        }
    }

    #[test]
    fn test_p08_1_parser_property_valid_cron_permutations(
        min in 0u32..59,
        hour in 0u32..23,
        dom in 1u32..31,
        month in 1u32..12,
        dow in 0u32..7,
    ) {
        let cron_str = format!("{min} {hour} {dom} {month} {dow}");
        let res = parse_schedule(&cron_str);
        prop_assert!(res.is_ok());

        let now = Local::now();
        let next = compute_next_run(&res.unwrap(), now);
        prop_assert!(next.is_some());
    }
}

// --- P08.1 Description Golden Tests in EN and ID ---

#[test]
fn test_p08_1_description_golden_tests() {
    let cases = vec![
        (
            "hourly",
            "Every hour at minute 0",
            "Setiap jam pada menit 0",
        ),
        ("daily", "Every day at 00:00", "Setiap hari pukul 00.00"),
        (
            "weekly",
            "Every week on Monday at 02:00",
            "Setiap Senin pukul 02.00",
        ),
        (
            "monthly",
            "Every month on day 1 at 02:00",
            "Setiap tanggal 1 pukul 02.00",
        ),
        (
            "0 2 * * 1",
            "Every week on Monday at 02:00",
            "Setiap Senin pukul 02.00",
        ),
        ("0 0 * * *", "Every day at 00:00", "Setiap hari pukul 00.00"),
        (
            "0 * * * *",
            "Every hour at minute 0",
            "Setiap jam pada menit 0",
        ),
        (
            "30 14 * * *",
            "Every day at 14:30",
            "Setiap hari pukul 14.30",
        ),
        (
            "0 9 * * 1-5",
            "Monday to Friday at 09:00",
            "Senin hingga Jumat pukul 09.00",
        ),
        ("0 9 * * *", "Every day at 09:00", "Setiap hari pukul 09.00"),
        (
            "0 3 * * 0",
            "Every week on Sunday at 03:00",
            "Setiap Minggu pukul 03.00",
        ),
        ("every 15 minutes", "Every 15 minutes", "Setiap 15 menit"),
        ("every 5 minutes", "Every 5 minutes", "Setiap 5 menit"),
    ];

    for (expr, expected_en, expected_id) in cases {
        let desc_en = describe_schedule_en(expr);
        let desc_id = describe_schedule_id(expr);
        assert_eq!(desc_en, expected_en, "EN description mismatch for '{expr}'");
        assert_eq!(desc_id, expected_id, "ID description mismatch for '{expr}'");
    }
}

// --- P08.2 Runtime Tests: Panic Isolation, Overlap Prevention, Vault Wait, Retention ---

#[tokio::test]
async fn test_p08_2_runtime_panic_isolation() {
    let clock = Arc::new(caf_core::scheduler::runtime::SystemClock);
    let rt = Arc::new(SchedulerRuntime::new(clock));

    // Register a panicking job
    rt.register_job(RegisteredJob {
        key: "test.panicking".to_string(),
        title_en: "Panicking Job".to_string(),
        title_id: "Job Panik".to_string(),
        default_schedule: "hourly".to_string(),
        permission: "test:run".to_string(),
        needs_vault: false,
        catch_up: CatchUpPolicy::Skip,
        max_runtime_secs: 10,
        background_safe: true,
        runner: Arc::new(|| {
            Box::pin(async {
                panic!("intentional panic inside job runner!");
            })
        }),
    });

    // Execute job: must NOT panic the process, must return Err and record panic status in history
    let res = rt
        .clone()
        .execute_job("test.panicking".to_string(), true)
        .await;
    assert!(res.is_err(), "Panicking job must return Err");

    let history = rt.get_history(Some("test.panicking"), 10);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, "panic");
    assert!(history[0].message.contains("panicked"));
}

#[tokio::test]
async fn test_p08_2_runtime_overlap_prevention() {
    let clock = Arc::new(caf_core::scheduler::runtime::SystemClock);
    let rt = Arc::new(SchedulerRuntime::new(clock));

    rt.register_job(RegisteredJob {
        key: "test.slow".to_string(),
        title_en: "Slow Job".to_string(),
        title_id: "Job Lambat".to_string(),
        default_schedule: "hourly".to_string(),
        permission: "test:run".to_string(),
        needs_vault: false,
        catch_up: CatchUpPolicy::Skip,
        max_runtime_secs: 10,
        background_safe: true,
        runner: Arc::new(|| {
            Box::pin(async {
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                Ok("done slow".to_string())
            })
        }),
    });

    let rt1 = rt.clone();
    let handle1 = tokio::spawn(async move { rt1.execute_job("test.slow".to_string(), true).await });

    // Short sleep to ensure first job has acquired the running flag
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    // Second execution must fail immediately with overlap error
    let res2 = rt.clone().execute_job("test.slow".to_string(), true).await;
    assert!(res2.is_err(), "Second overlapping execution must fail");
    assert!(res2.unwrap_err().contains("already running"));

    let res1 = handle1.await.expect("join handle");
    assert!(res1.is_ok(), "First execution must succeed");
}

#[test]
fn test_p08_2_runtime_vault_wait() {
    let clock = Arc::new(caf_core::scheduler::runtime::SystemClock);
    let rt = Arc::new(SchedulerRuntime::new(clock));

    rt.register_job(RegisteredJob {
        key: "test.vault_needed".to_string(),
        title_en: "Vault Job".to_string(),
        title_id: "Job Vault".to_string(),
        default_schedule: "hourly".to_string(),
        permission: "test:run".to_string(),
        needs_vault: true,
        catch_up: CatchUpPolicy::Skip,
        max_runtime_secs: 10,
        background_safe: false,
        runner: Arc::new(|| Box::pin(async { Ok("ok".to_string()) })),
    });

    // Set next run in the past so it is due
    rt.next_runs
        .write()
        .insert("test.vault_needed".to_string(), 100);

    // Vault is currently locked in uninitialized test environment:
    // tick() must NOT trigger the job needing vault
    let due = rt.tick();
    assert!(
        !due.contains(&"test.vault_needed".to_string()),
        "Job needing vault must not trigger while vault is locked"
    );
}

#[test]
fn test_p08_2_runtime_history_retention_500() {
    let clock = Arc::new(caf_core::scheduler::runtime::SystemClock);
    let rt = SchedulerRuntime::new(clock);

    for i in 0..550 {
        rt.add_history_record(JobRunRecord {
            id: format!("rec-{i}"),
            job_key: "test.key".to_string(),
            started_at: i,
            finished_at: i + 1,
            status: "success".to_string(),
            message: format!("run {i}"),
        });
    }

    let history = rt.get_history(None, 0);
    assert_eq!(
        history.len(),
        MAX_RUN_HISTORY,
        "History retention must be bounded at exactly 500 records"
    );
    // Most recent is rec-549
    assert_eq!(history[0].id, "rec-549");
    assert_eq!(history[MAX_RUN_HISTORY - 1].id, "rec-50");
}

// --- P08.3 Catch-Up Tests with Injectable Clock ---

#[test]
fn test_p08_3_catch_up_skip_and_run_once() {
    let base_dt = Local
        .with_ymd_and_hms(2026, 11, 1, 10, 0, 0)
        .single()
        .unwrap();
    let clock = Arc::new(MockClock::new(base_dt));
    let rt = SchedulerRuntime::new(clock.clone());

    // Job A: CatchUpPolicy::Skip (daily at 00:00)
    rt.register_job(RegisteredJob {
        key: "job.skip".to_string(),
        title_en: "Skip Job".to_string(),
        title_id: "Skip Job".to_string(),
        default_schedule: "daily".to_string(),
        permission: "test:run".to_string(),
        needs_vault: false,
        catch_up: CatchUpPolicy::Skip,
        max_runtime_secs: 10,
        background_safe: true,
        runner: Arc::new(|| Box::pin(async { Ok("ok".to_string()) })),
    });

    // Job B: CatchUpPolicy::RunOnce (daily at 00:00)
    rt.register_job(RegisteredJob {
        key: "job.run_once".to_string(),
        title_en: "RunOnce Job".to_string(),
        title_id: "RunOnce Job".to_string(),
        default_schedule: "daily".to_string(),
        permission: "test:run".to_string(),
        needs_vault: false,
        catch_up: CatchUpPolicy::RunOnce,
        max_runtime_secs: 10,
        background_safe: true,
        runner: Arc::new(|| Box::pin(async { Ok("ok".to_string()) })),
    });

    // Set last_run to base_dt for both jobs
    rt.last_runs
        .write()
        .insert("job.skip".to_string(), base_dt.timestamp());
    rt.last_runs
        .write()
        .insert("job.run_once".to_string(), base_dt.timestamp());

    // Advance clock by 3 days (missing 3 scheduled runs)
    clock.advance(Duration::days(3));

    // Handle catch up
    let triggered = rt.handle_catch_up();

    // job.run_once MUST be triggered once
    assert!(
        triggered.contains(&"job.run_once".to_string()),
        "Job with RunOnce policy must trigger on catch-up"
    );

    // job.skip MUST NOT be triggered
    assert!(
        !triggered.contains(&"job.skip".to_string()),
        "Job with Skip policy must not trigger on catch-up"
    );

    // job.skip's next run must now be in the future (strictly after current clock now)
    let next_skip = rt.next_runs.read().get("job.skip").copied().unwrap();
    assert!(
        next_skip > clock.now().timestamp(),
        "Next run for Skip job must be updated to future"
    );
}

// --- P08.4 Desktop Tray and Start-at-Login Settings ---

#[test]
fn test_p08_4_desktop_tray_and_login_settings() {
    let initial = get_desktop_settings();
    // Default is off
    assert!(!initial.close_to_tray);
    assert!(!initial.start_at_login);

    let updated = SchedulerDesktopSettings {
        close_to_tray: true,
        start_at_login: true,
    };
    set_desktop_settings(updated).expect("set desktop settings");

    let loaded = get_desktop_settings();
    assert!(loaded.close_to_tray);
    assert!(loaded.start_at_login);

    // Revert back
    set_desktop_settings(initial).expect("revert desktop settings");
}

// --- P08.5 Built-In Jobs Coverage ---

#[tokio::test]
async fn test_p08_5_builtin_jobs_coverage() {
    let builtin_jobs = make_builtin_jobs();
    let expected_keys = vec![
        "backup.scheduled",
        "backup.overdue_reminder",
        "audit.prune",
        "integrations.outbox_flush",
        "pro.licence_refresh",
    ];

    for key in expected_keys {
        let job = builtin_jobs.iter().find(|j| j.key == key);
        assert!(job.is_some(), "Built-in job '{key}' must be registered");

        let job = job.unwrap();
        // Test runner execution
        let fut = (job.runner)();
        let res = fut.await;
        assert!(
            res.is_ok(),
            "Runner for '{key}' must execute successfully: {res:?}"
        );
    }
}
