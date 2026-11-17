//! Comprehensive test suite for P10: Data Import and Export (FR-17).
//! Tests Entity Registry, CSV/XLSX/JSON streaming export, formula injection neutralization,
//! visibility/audit rules, 7-step transactional import, downloadable templates, limits,
//! and 10,000-case fuzzing.

use caf_core::data_io::xlsx::parse_xlsx;
use caf_core::data_io::*;
use caf_core::error::{CafError, DataIoError};
use caf_core::notes::{self, NoteInput};
use caf_core::rbac;
use caf_core::session::{self, Session};
use caf_core::vault;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct TestEnv {
    dir: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl TestEnv {
    fn new(label: &str) -> Self {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        vault::lock();
        session::clear_current_session().ok();
        let dir = std::env::temp_dir().join(format!("caf_p10_{label}_{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::set_var("CAF_DATA_DIR", &dir);
        }
        vault::setup_vault("test-master-password-123").expect("setup vault");
        let conn = caf_core::db::open().expect("open db");
        rbac::seed_roles(&conn).expect("seed roles");
        Self { dir, _guard: guard }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        vault::lock();
        session::clear_current_session().ok();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn set_test_session(profile_id: &str, role: &str) {
    let mut sess = Session::new(profile_id, role);
    if role == "super" {
        sess.permissions.insert("*".to_string());
    } else {
        sess.permissions.insert("notes:read".to_string());
        sess.permissions.insert("notes:write".to_string());
    }
    session::set_current_session(sess).expect("set session");
}

#[test]
fn test_p10_1_entity_registry() {
    let _env = TestEnv::new("p10_1_reg");
    let entities = list_entities();
    assert!(!entities.is_empty(), "entity registry must not be empty");

    let notes_def = get_entity("notes").expect("notes entity must be registered");
    assert_eq!(notes_def.key, "notes");
    assert_eq!(notes_def.table_name, "notes");
    assert_eq!(notes_def.formats, vec!["csv", "xlsx", "json"]);
    assert_eq!(notes_def.natural_keys, vec!["title"]);

    let field_names: Vec<&str> = notes_def.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"id"));
    assert!(field_names.contains(&"title"));
    assert!(field_names.contains(&"content"));
    assert!(field_names.contains(&"tags"));
    assert!(field_names.contains(&"visibility"));

    let title_field = notes_def
        .fields
        .iter()
        .find(|f| f.name == "title")
        .expect("title field");
    assert!(title_field.required);
    assert!(title_field.is_natural_key);
}

#[test]
fn test_p10_2_export_formats_and_formula_neutralization() {
    let _env = TestEnv::new("p10_2_export_formula");
    set_test_session("owner_1", "super");

    // Create notes with formula injection payloads
    notes::save_note(NoteInput {
        id: None,
        title: "=SUM(1,2)".into(),
        content: "@EVIL() payload".into(),
        tags: vec!["test".into()],
        visibility: Some("shared".into()),
    })
    .expect("save note 1");

    notes::save_note(NoteInput {
        id: None,
        title: "+cmd|' /C calc'!A0".into(),
        content: "-5+2 formula".into(),
        tags: vec!["calc".into()],
        visibility: Some("shared".into()),
    })
    .expect("save note 2");

    // 1. Export CSV
    let csv_res = export_entity("notes", "csv").expect("export csv");
    assert_eq!(csv_res.row_count, 2);
    let csv_content = fs::read_to_string(&csv_res.file_path).expect("read csv");
    assert!(
        csv_content.contains("'=SUM(1,2)"),
        "formula '=' must be neutralized with leading quote"
    );
    assert!(
        csv_content.contains("'@EVIL() payload"),
        "formula '@' must be neutralized with leading quote"
    );
    assert!(
        csv_content.contains("'+cmd|' /C calc'!A0"),
        "formula '+' must be neutralized with leading quote"
    );
    assert!(
        csv_content.contains("'-5+2 formula"),
        "formula '-' must be neutralized with leading quote"
    );

    // 2. Export XLSX
    let xlsx_res = export_entity("notes", "xlsx").expect("export xlsx");
    assert_eq!(xlsx_res.row_count, 2);
    let xlsx_file = fs::File::open(&xlsx_res.file_path).expect("open xlsx");
    let xlsx_rows = parse_xlsx(xlsx_file, 100).expect("parse xlsx");
    assert_eq!(xlsx_rows.len(), 3); // 1 header + 2 rows

    let all_cells: Vec<String> = xlsx_rows.into_iter().flatten().collect();
    assert!(
        all_cells.iter().any(|c| c == "'=SUM(1,2)"),
        "XLSX cell starting with '=' must be neutralized"
    );

    // 3. Export JSON
    let json_res = export_entity("notes", "json").expect("export json");
    assert_eq!(json_res.row_count, 2);
    let json_content = fs::read_to_string(&json_res.file_path).expect("read json");
    assert!(json_content.contains("=SUM(1,2)"));
}

#[test]
fn test_p10_2_export_visibility_and_super_audit() {
    let _env = TestEnv::new("p10_2_vis");

    // User A creates a private note
    set_test_session("user_a", "member");
    notes::save_note(NoteInput {
        id: None,
        title: "User A Private Note".into(),
        content: "Secret content".into(),
        tags: vec![],
        visibility: Some("private".into()),
    })
    .expect("save note user_a");

    // User B creates a private note and a shared note
    set_test_session("user_b", "member");
    notes::save_note(NoteInput {
        id: None,
        title: "User B Private Note".into(),
        content: "Top secret B".into(),
        tags: vec![],
        visibility: Some("private".into()),
    })
    .expect("save note user_b");

    notes::save_note(NoteInput {
        id: None,
        title: "User B Shared Note".into(),
        content: "Public info".into(),
        tags: vec![],
        visibility: Some("shared".into()),
    })
    .expect("save note shared user_b");

    // 1. User A exports notes -> must only see User A's private note + User B's shared note
    set_test_session("user_a", "viewer");
    let res_a = export_entity("notes", "json").expect("export user a");
    assert_eq!(
        res_a.row_count, 2,
        "viewer export must exclude other profile's private rows"
    );
    let content_a = fs::read_to_string(&res_a.file_path).expect("read json");
    assert!(content_a.contains("User A Private Note"));
    assert!(content_a.contains("User B Shared Note"));
    assert!(!content_a.contains("User B Private Note"));

    // 2. Super role exports notes -> sees all 3 notes and audits PRIVATE_READ
    set_test_session("admin_super", "super");
    let res_super = export_entity("notes", "json").expect("export super");
    assert_eq!(res_super.row_count, 3);

    let logs =
        caf_core::audit::get_logs(None, Some("PRIVATE_READ"), None).expect("query audit logs");

    assert!(
        !logs.is_empty(),
        "super-role reading others' private rows during export must log PRIVATE_READ audit event"
    );
    assert!(
        logs.iter()
            .any(|l| l.details.contains("export_entity=notes"))
    );
}

#[test]
fn test_p10_3_import_pipeline_7_steps() {
    let _env = TestEnv::new("p10_3_pipeline");
    set_test_session("importer_1", "member");

    // Step 1 & 2: Parse File & Auto Mapping
    let csv_data = b"Title,Description,Label,Visibility\n\
Note Alpha,Body of Alpha,work,shared\n\
Note Beta,Body of Beta,personal,private\n\
Note Gamma,,misc,shared\n";

    let file_info = parse_import_file("notes", "csv", csv_data).expect("parse import file");
    assert_eq!(file_info.total_rows, 3);
    assert!(file_info.columns.contains(&"Title".to_string()));

    let mut mapping = file_info.suggested_mapping;
    mapping.insert("Title".into(), "title".into());
    mapping.insert("Description".into(), "content".into());
    mapping.insert("Label".into(), "tags".into());
    mapping.insert("Visibility".into(), "visibility".into());

    // Step 3 & 5: Preview & Dry Run (InsertOnly mode)
    let preview = preview_import("notes", "csv", csv_data, &mapping, ImportMode::InsertOnly)
        .expect("preview import");
    assert_eq!(preview.total_rows, 3);
    assert_eq!(preview.to_insert, 3);
    assert_eq!(preview.to_update, 0);
    assert_eq!(preview.invalid_count, 0);

    // Step 6: Transactional execute (InsertOnly)
    let exec_report = execute_import(
        "notes",
        "csv",
        csv_data,
        &mapping,
        ImportMode::InsertOnly,
        ImportStrategy::AllOrNothing,
    )
    .expect("execute import");

    assert_eq!(exec_report.inserted, 3);
    assert_eq!(exec_report.updated, 0);
    assert_eq!(exec_report.failed.len(), 0);

    // Verify imported rows in DB: importer is owner and change_log recorded entries
    let conn = caf_core::db::open().expect("open db");
    let mut stmt = conn
        .prepare("SELECT title, owner_profile_id, rev FROM notes WHERE deleted_at IS NULL ORDER BY title ASC")
        .expect("prepare select");
    let saved_notes: Vec<(String, String, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("query notes")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect notes");

    assert_eq!(saved_notes.len(), 3);
    for (_, owner, rev) in &saved_notes {
        assert_eq!(owner, "importer_1", "importer must become owner");
        assert_eq!(*rev, 1);
    }

    let change_log_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM change_log WHERE entity = 'notes' AND op = 'insert'",
            [],
            |r| r.get(0),
        )
        .expect("count change_log");
    assert_eq!(
        change_log_count, 3,
        "sync helper must log insert for each row"
    );

    // Test Step 4 & conflict in InsertOnly mode
    let conflict_preview =
        preview_import("notes", "csv", csv_data, &mapping, ImportMode::InsertOnly)
            .expect("preview conflict");
    assert_eq!(
        conflict_preview.invalid_count, 3,
        "all 3 exist; must conflict in InsertOnly"
    );

    // Test Step 4: UpsertByNaturalKey
    let update_csv = b"Title,Description,Label,Visibility\n\
Note Alpha,Updated Alpha Content,work,shared\n\
Note Delta,New Delta,general,shared\n";

    let upsert_report = execute_import(
        "notes",
        "csv",
        update_csv,
        &mapping,
        ImportMode::UpsertByNaturalKey,
        ImportStrategy::AllOrNothing,
    )
    .expect("execute upsert");

    assert_eq!(upsert_report.inserted, 1, "Note Delta inserted");
    assert_eq!(upsert_report.updated, 1, "Note Alpha updated");

    // Verify revision bumped to 2 for Note Alpha
    let alpha_rev: i64 = conn
        .query_row(
            "SELECT rev FROM notes WHERE title = 'Note Alpha' AND deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .expect("get alpha rev");
    assert_eq!(alpha_rev, 2, "updated record revision must bump to 2");

    // Step 7: Failed rows downloadable report test with SkipInvalid
    let bad_csv = b"Title,Description\n\
,Missing Title Row\n\
Valid Title,Normal Body\n";

    let skip_report = execute_import(
        "notes",
        "csv",
        bad_csv,
        &mapping,
        ImportMode::InsertOnly,
        ImportStrategy::SkipInvalid,
    )
    .expect("execute skip invalid");

    assert_eq!(skip_report.inserted, 1);
    assert_eq!(skip_report.skipped, 1);
    assert_eq!(skip_report.failed.len(), 1);
    assert!(skip_report.failed_rows_csv.is_some());
    let failed_csv = skip_report.failed_rows_csv.expect("failed csv");
    assert!(failed_csv.contains("Field 'title' is required"));
}

#[test]
fn test_p10_4_templates_and_limits() {
    let _env = TestEnv::new("p10_4_limits");

    // 1. Downloadable templates
    for fmt in ["csv", "xlsx", "json"] {
        let template_bytes = generate_entity_template("notes", fmt).expect("generate template");
        assert!(
            !template_bytes.is_empty(),
            "template for {fmt} must not be empty"
        );
    }

    // 2. Reject file over size limit (> 20 MB)
    let huge_data = vec![b'a'; DEFAULT_MAX_FILE_BYTES + 10];
    let err_size = parse_import_file("notes", "csv", &huge_data);
    assert!(
        matches!(err_size, Err(CafError::DataIo(DataIoError::FileTooLarge))),
        "file over 20 MB must be rejected with FileTooLarge"
    );

    // 3. Reject file over row limit (> 50,000 rows)
    let mut big_csv = String::from("Title,Content\n");
    for i in 0..50_005 {
        big_csv.push_str(&format!("Note {i},Body {i}\n"));
    }
    let err_rows = parse_import_file("notes", "csv", big_csv.as_bytes());
    assert!(
        matches!(
            err_rows,
            Err(CafError::DataIo(DataIoError::RowLimitExceeded))
        ),
        "file over 50,000 rows must be rejected with RowLimitExceeded"
    );
}

#[test]
fn test_p10_3_fuzz_malformed_csv_json_xlsx_10000_cases() {
    let _env = TestEnv::new("p10_3_fuzz");
    use rand::RngCore;
    use rand::SeedableRng;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0x000C_AF10_F002);
    let mut mapping = HashMap::new();
    mapping.insert("Title".into(), "title".into());
    mapping.insert("Content".into(), "content".into());

    // Fuzz test ≥ 10,000 cases of arbitrary/malformed inputs
    for i in 0..10_000 {
        let len = (rng.next_u32() % 256) as usize;
        let mut malformed_bytes = vec![0u8; len];
        rng.fill_bytes(&mut malformed_bytes);

        // Mix in structural characters occasionally
        if i % 3 == 0 {
            for b in &mut malformed_bytes {
                if *b % 5 == 0 {
                    *b = b'"';
                } else if *b % 7 == 0 {
                    *b = b',';
                } else if *b % 11 == 0 {
                    *b = b'\n';
                }
            }
        }

        let fmt = match i % 3 {
            0 => "csv",
            1 => "json",
            _ => "xlsx",
        };

        // None of these should panic or crash
        let _ = parse_import_file("notes", fmt, &malformed_bytes);
        let _ = preview_import(
            "notes",
            fmt,
            &malformed_bytes,
            &mapping,
            ImportMode::InsertOnly,
        );
    }
}

#[test]
fn test_p10_2_streaming_performance_budget_100k_rows() {
    let _env = TestEnv::new("p10_2_perf");
    set_test_session("owner_perf", "super");

    // Insert 100,000 rows in SQLite
    let mut conn = caf_core::db::open().expect("open db");
    let tx = conn.transaction().expect("start tx");

    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO notes (id, title, content, tags, rev, created_at, updated_at, origin_device_id, owner_profile_id, visibility) \
                 VALUES (?1, ?2, ?3, '[]', 1, '2026-11-17T08:00:00Z', '2026-11-17T08:00:00Z', 'dev-1', 'owner_perf', 'shared')",
            )
            .expect("prepare insert");

        for i in 0..100_000 {
            let id = format!("note-perf-{i}");
            let title = format!("Performance Note {i}");
            let content = format!("Benchmarking row {i} streaming export without memory explosion");
            stmt.execute(rusqlite::params![id, title, content])
                .expect("execute insert");
        }
    }
    tx.commit().expect("commit 100k notes");

    // Measure streaming export time
    let start = Instant::now();
    let export_res = export_entity("notes", "csv").expect("streaming export 100k rows");
    let duration = start.elapsed();

    assert_eq!(export_res.row_count, 100_000);
    println!("Exported 100,000 rows to CSV in {:?}", duration);

    // PRD §6 budget: export of 100 000 rows ≤ 15 s
    assert!(
        duration.as_secs() < 15,
        "100,000-row export duration ({:?}) exceeded 15s PRD §6 budget",
        duration
    );
}
