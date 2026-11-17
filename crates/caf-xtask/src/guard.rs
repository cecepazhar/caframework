use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn get_workspace_root() -> PathBuf {
    if let Ok(dir) = env::var("CAF_GUARD_ROOT") {
        return PathBuf::from(dir);
    }
    match env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => {
            let p = PathBuf::from(dir);
            p.parent()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        }
        Err(_) => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

pub fn run_guard(only: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut failures = Vec::new();
    let root = get_workspace_root();

    // (a) Repository hygiene & tracked files (FR-14, A-07):
    // Checks for oversized files (> 1MB), forbidden extensions (*.key, *.db, *.log, binaries),
    // and forbidden internal directories (Notes/, target/, node_modules/, plans, ledgers, audits).
    if should_run("oversized_or_binary", only)
        || should_run("file_size_and_binary", only)
        || should_run("repository_hygiene", only)
    {
        check_tracked_files(&root, &mut failures)?;
    }

    // (b) No absolute paths from dev-machine outside Notes/
    if should_run("absolute_paths", only) {
        check_absolute_paths(&root, &mut failures)?;
    }

    // (c) Tauri command registries checking
    if should_run("invoke_registry", only) || should_run("unregistered_invokes", only) {
        check_invoke_registry(&root, &mut failures)?;
    }

    // (d) Hardcoded strings
    if should_run("hardcoded_strings", only) {
        crate::scanner::check_hardcoded_strings(&root, &mut failures)?;
    }

    // (e) Raw sync writes
    if should_run("raw_sync_writes", only) {
        crate::scanner::check_raw_sync_writes(&root, &mut failures)?;
    }

    // (f) Genericity
    if should_run("genericity", only) {
        check_genericity(&root, &mut failures)?;
    }

    // (g) Ledger fidelity & evidence guard (P01.8)
    if should_run("ledger", only) {
        check_ledger(&mut failures)?;
    }

    // (h) String(err) guard (P06.4)
    if should_run("string_err", only) {
        check_string_err(&root, &mut failures)?;
    }

    // (i) Gated commands check (P13.3)
    if should_run("gated_commands", only) {
        check_gated_commands(&root, &mut failures)?;
    }

    if !failures.is_empty() {
        for f in &failures {
            eprintln!("{}", f);
        }
        return Err(format!("Guard check failed with {} errors", failures.len()).into());
    }

    println!("All guards passed.");
    Ok(())
}

fn should_run(guard_name: &str, only: Option<&str>) -> bool {
    only.is_none_or(|o| o == guard_name)
}

fn get_tracked_files(root: &Path) -> Vec<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("ls-files")
        .current_dir(root)
        .output();
    if let Ok(out) = output
        && out.status.success()
    {
        let str_out = String::from_utf8_lossy(&out.stdout);
        return str_out
            .lines()
            .map(|l| root.join(l.trim()))
            .filter(|p| p.is_file())
            .collect();
    }
    // Fallback: walk directory excluding common ignored dirs
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter().flatten() {
        let p = entry.path();
        if p.is_file() && !is_excluded(p, root) {
            files.push(p.to_path_buf());
        }
    }
    files
}

fn is_excluded(path: &Path, root: &Path) -> bool {
    if let Ok(rel) = path.strip_prefix(root) {
        let rs = rel.to_string_lossy();
        if rs.starts_with(".git")
            || rs.starts_with("target")
            || rs.starts_with("frontend/node_modules")
            || rs.starts_with("frontend/.svelte-kit")
            || rs.starts_with("frontend/build")
            || rs.starts_with("crates/caf-app/gen/android")
            || rs.starts_with("dist")
            || rs.starts_with("Notes")
        {
            return true;
        }
    }
    false
}

fn check_tracked_files(
    root: &Path,
    failures: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let tracked = get_tracked_files(root);
    let ratchet_file = root.join("guards/ratchet/large_files_baseline.txt");
    let mut allowed_large_files = HashSet::new();
    if ratchet_file.exists()
        && let Ok(content) = fs::read_to_string(&ratchet_file)
    {
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(last) = parts.last() {
                    allowed_large_files.insert(last.to_string());
                }
            }
        }
    }

    for path in tracked {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        if let Ok(meta) = path.metadata()
            && meta.len() > 1_048_576
            && !allowed_large_files.contains(&rel_path)
        {
            failures.push(format!(
                "oversized_or_binary: {}:0: Tracked file is over 1MB ({} bytes)",
                rel_path,
                meta.len()
            ));
        }

        let fn_str = path.to_string_lossy();
        let rel_lower = rel_path.to_lowercase();

        // Check forbidden extensions: *.key, *.db, *.sqlite*, *.log, *.exe, *.dll, *.so
        if (rel_lower.ends_with(".db")
            || rel_lower.ends_with(".sqlite")
            || rel_lower.ends_with(".sqlite3")
            || rel_lower.ends_with(".key")
            || rel_lower.ends_with(".log")
            || rel_lower.ends_with(".exe")
            || rel_lower.ends_with(".dll")
            || rel_lower.ends_with(".so"))
            && !allowed_large_files.contains(&rel_path)
        {
            failures.push(format!(
                "repository_hygiene: {}:0: Tracked file has forbidden key/log/binary/DB extension",
                rel_path
            ));
        }

        // Check forbidden internal paths / documents inside repository (FR-14, A-07)
        if rel_path.starts_with("Notes/")
            || rel_path.starts_with("target/")
            || rel_path.starts_with("node_modules/")
            || rel_path.contains("/node_modules/")
            || rel_path == "task.md"
            || rel_path == "prompt.md"
            || rel_path == "prd.md"
            || rel_path == "prd-amendments.md"
            || rel_path.starts_with("ledger")
            || rel_path.contains("bukti/")
            || rel_path.contains("audit/")
        {
            failures.push(format!(
                "repository_hygiene: {}:0: Tracked file violates repository hygiene (internal notes/plans/ledgers/audits inside repo)",
                rel_path
            ));
        } else {
            let is_known_media = fn_str.ends_with(".png")
                || fn_str.ends_with(".jpg")
                || fn_str.ends_with(".jpeg")
                || fn_str.ends_with(".ico")
                || fn_str.ends_with(".icns")
                || fn_str.ends_with(".bmp")
                || fn_str.ends_with(".woff2")
                || fn_str.ends_with(".ttf");
            if !is_known_media {
                let mut buf = [0u8; 1024];
                if let Ok(mut f) = fs::File::open(&path) {
                    use std::io::Read;
                    if let Ok(n) = f.read(&mut buf)
                        && buf[..n].contains(&0)
                        && !allowed_large_files.contains(&rel_path)
                    {
                        failures.push(format!(
                            "oversized_or_binary: {}:0: Tracked file contains NUL bytes",
                            rel_path
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_absolute_paths(
    root: &Path,
    failures: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bad_paths = ["/home/cecepazhar", "C:\\Users\\", "/home/runner"];
    let tracked = get_tracked_files(root);
    let ratchet_file = root.join("guards/ratchet/absolute_paths.txt");
    let mut allowed_paths = HashSet::new();
    if ratchet_file.exists()
        && let Ok(content) = fs::read_to_string(&ratchet_file)
    {
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                let parts: Vec<&str> = line.split(':').collect();
                if let Some(first) = parts.first() {
                    allowed_paths.insert(first.trim().to_string());
                }
            }
        }
    }

    for path in tracked {
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        if rel_path.starts_with("Notes/")
            || rel_path.starts_with("guards/ratchet/")
            || allowed_paths.contains(&rel_path)
        {
            continue;
        }

        let is_text = path.extension().is_some_and(|ext| {
            let e = ext.to_string_lossy();
            matches!(
                e.as_ref(),
                "rs" | "toml" | "json" | "md" | "ts" | "svelte" | "yaml" | "yml" | "html" | "sh"
            )
        });

        if is_text && let Ok(content) = fs::read_to_string(&path) {
            for (i, line) in content.lines().enumerate() {
                for bad in &bad_paths {
                    if rel_path.contains("crates/caf-xtask/src/guard.rs") {
                        continue;
                    }
                    if line.contains(bad) {
                        failures.push(format!(
                            "absolute_paths: {}:{}: Contains dev-machine absolute path '{}'",
                            rel_path,
                            i + 1,
                            bad
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_invoke_registry(
    root: &Path,
    failures: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut defined_commands = HashSet::new();
    let mut invoked_commands = Vec::new();

    // 1. Scan for #[tauri::command] in crates/caf-app/src
    let app_src = root.join("crates/caf-app/src");
    if app_src.exists() {
        for entry in walkdir::WalkDir::new(&app_src).into_iter().flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|e| e == "rs")
                && let Ok(content) = fs::read_to_string(path)
            {
                let lines: Vec<&str> = content.lines().collect();
                for i in 0..lines.len() {
                    if lines[i].contains("#[tauri::command]") && i + 1 < lines.len() {
                        let next_line = lines[i + 1].trim();
                        if next_line.starts_with("pub fn ")
                            || next_line.starts_with("pub async fn ")
                            || next_line.starts_with("fn ")
                            || next_line.starts_with("async fn ")
                        {
                            let parts: Vec<&str> = next_line.split('(').collect();
                            if let Some(sig) = parts.first() {
                                let name = sig.split_whitespace().last().unwrap_or("");
                                if !name.is_empty() {
                                    defined_commands.insert(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Scan frontend/src for invoke(...)
    let fe_src = root.join("frontend/src");
    if fe_src.exists() {
        for entry in walkdir::WalkDir::new(&fe_src).into_iter().flatten() {
            let path = entry.path();
            if path.is_file() {
                let rel_path = path
                    .strip_prefix(root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                let fn_str = path.to_string_lossy();
                if (fn_str.ends_with(".ts")
                    || fn_str.ends_with(".svelte")
                    || fn_str.ends_with(".js"))
                    && let Ok(content) = fs::read_to_string(path)
                {
                    let bytes = content.as_bytes();
                    let mut i = 0;
                    let mut line_num = 1;
                    while i < bytes.len() {
                        if bytes[i] == b'\n' {
                            line_num += 1;
                        }
                        let is_prev_ident = i > 0
                            && (bytes[i - 1].is_ascii_alphanumeric()
                                || bytes[i - 1] == b'_'
                                || bytes[i - 1] == b'$');
                        let next_byte = bytes.get(i + 6);
                        let is_next_ident = next_byte
                            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'$');

                        if !is_prev_ident && !is_next_ident && bytes[i..].starts_with(b"invoke") {
                            let candidate_line = line_num;
                            i += 6;
                            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                                if bytes[i] == b'\n' {
                                    line_num += 1;
                                }
                                i += 1;
                            }
                            if i < bytes.len() && bytes[i] == b'<' {
                                while i < bytes.len() && bytes[i] != b'>' {
                                    if bytes[i] == b'\n' {
                                        line_num += 1;
                                    }
                                    i += 1;
                                }
                                i += 1;
                            }
                            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                                if bytes[i] == b'\n' {
                                    line_num += 1;
                                }
                                i += 1;
                            }
                            if !rel_path.contains("generated")
                                && !rel_path.contains("test")
                                && !rel_path.contains("tests")
                            {
                                failures.push(format!(
                                    "invoke_guard: {}:{}: invoke() is only allowed in generated bindings",
                                    rel_path, candidate_line
                                ));
                            }
                            if i < bytes.len() && bytes[i] == b'(' {
                                i += 1;
                                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                                    if bytes[i] == b'\n' {
                                        line_num += 1;
                                    }
                                    i += 1;
                                }
                                if i < bytes.len() && (bytes[i] == b'\'' || bytes[i] == b'"') {
                                    let quote = bytes[i];
                                    i += 1;
                                    let start = i;
                                    while i < bytes.len() && bytes[i] != quote && bytes[i] != b'\n'
                                    {
                                        i += 1;
                                    }
                                    let end = i;
                                    if let Ok(name) = std::str::from_utf8(&bytes[start..end]) {
                                        invoked_commands.push((
                                            name.to_string(),
                                            rel_path.clone(),
                                            candidate_line,
                                        ));
                                    }
                                } else if i < bytes.len() && bytes[i] != b')' {
                                    failures.push(format!(
                                        "invoke_registry: {}:{}: invoke() name is not a string literal",
                                        rel_path, candidate_line
                                    ));
                                }
                            }
                        } else {
                            i += 1;
                        }
                    }
                }
            }
        }
    }

    // Load ratchet for unregistered invokes
    let ratchet_file = root.join("guards/ratchet/unregistered_invokes_baseline.txt");
    let mut allowed_unregistered = HashSet::new();
    if ratchet_file.exists()
        && let Ok(content) = fs::read_to_string(&ratchet_file)
    {
        for line in content.lines() {
            let l = line.trim();
            if !l.is_empty() && !l.starts_with('#') {
                allowed_unregistered.insert(l.to_string());
            }
        }
    }

    // Check invoked commands against defined commands
    for (cmd, file, line) in invoked_commands {
        if cmd.starts_with("plugin:") || cmd.starts_with("__tauri") {
            continue;
        }
        if !defined_commands.contains(&cmd) && !allowed_unregistered.contains(&cmd) {
            failures.push(format!(
                "invoke_registry: {}:{}: Unregistered command invoked '{}'",
                file, line, cmd
            ));
        }
    }

    Ok(())
}

fn check_genericity(
    root: &Path,
    failures: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let forbidden_words = ["keluarga", "rumah tangga"];
    let tracked = get_tracked_files(root);
    let ratchet_file = root.join("guards/ratchet/genericity_baseline.txt");
    let mut allowed_genericity = HashSet::new();
    if ratchet_file.exists()
        && let Ok(content) = fs::read_to_string(&ratchet_file)
    {
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                let parts: Vec<&str> = line.split(':').collect();
                if let Some(first) = parts.first() {
                    allowed_genericity.insert(first.trim().to_string());
                }
            }
        }
    }

    for path in tracked {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        if rel.starts_with("Notes/")
            || rel.starts_with("crates/caf-xtask/src/guard.rs")
            || rel.starts_with("guards/")
            || allowed_genericity.contains(&rel)
        {
            continue;
        }

        let is_text = path.extension().is_some_and(|ext| {
            let e = ext.to_string_lossy();
            matches!(e.as_ref(), "rs" | "ts" | "svelte" | "toml" | "json")
        });

        if is_text && let Ok(content) = fs::read_to_string(&path) {
            for (i, line) in content.lines().enumerate() {
                for word in &forbidden_words {
                    if line.to_lowercase().contains(word) {
                        failures.push(format!(
                            "genericity: {}:{}: Contains product-specific domain term '{}'",
                            rel,
                            i + 1,
                            word
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_ledger(failures: &mut Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let plan_dir = if let Ok(dir) = env::var("CAF_PLAN_DIR") {
        PathBuf::from(dir)
    } else {
        let home = env::var("HOME").unwrap_or_else(|_| ".".into());
        let p_notes = PathBuf::from(&home).join("Notes/notes/4 - Product (Notes)/1 CAFramework");
        if p_notes.exists() {
            p_notes
        } else {
            PathBuf::from(home).join("Notes/notes/4 - Product/1 CAFramework")
        }
    };

    if !plan_dir.exists() {
        return Ok(());
    }

    let task_path = plan_dir.join("task.md");
    let ledger_path = plan_dir.join("ledger-v3.md");
    let bukti_dir = plan_dir.join("bukti/v3");

    if !task_path.exists() || !ledger_path.exists() {
        return Ok(());
    }

    let task_content = fs::read_to_string(&task_path)?;
    let ledger_content = fs::read_to_string(&ledger_path)?;

    // 1. Extract tasks from task.md
    let mut task_specs: Vec<(String, String)> = Vec::new();
    for line in task_content.lines() {
        let line = line.trim();
        if line.starts_with("**P") && line.contains("**") {
            let inner = line.trim_start_matches("**");
            if let Some(idx) = inner.find("**") {
                let bold_part = &inner[..idx];
                let mut parts = bold_part.splitn(2, ' ');
                if let (Some(id), Some(title)) = (parts.next(), parts.next())
                    && id.starts_with('P')
                    && id.contains('.')
                {
                    task_specs.push((id.trim().to_string(), title.trim().to_string()));
                }
            }
        }
    }

    // 2. Extract tasks from ledger-v3.md
    let mut ledger_tasks: std::collections::HashMap<String, (String, String, String)> =
        std::collections::HashMap::new();
    for line in ledger_content.lines() {
        let line = line.trim();
        if line.starts_with("| P") {
            let cols: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            if cols.len() >= 5 {
                let tid = cols[1].to_string();
                let title = cols[2].to_string();
                let status = cols[3].to_string();
                let anchor = cols[4].to_string();
                ledger_tasks.insert(tid, (title, status, anchor));
            }
        }
    }

    // 3. Compare tasks
    for (tid, expected_title) in &task_specs {
        match ledger_tasks.get(tid) {
            None => {
                failures.push(format!(
                    "ledger_guard: Task {} missing from ledger-v3.md",
                    tid
                ));
            }
            Some((actual_title, status, anchor)) => {
                if actual_title != expected_title {
                    failures.push(format!(
                        "ledger_guard: Task {} title mismatch. task.md: {:?} vs ledger: {:?}",
                        tid, expected_title, actual_title
                    ));
                }
                let valid_statuses = [
                    "TODO",
                    "IN_PROGRESS",
                    "DONE",
                    "DONE (sudah sesuai)",
                    "PARTIAL",
                    "UI-ONLY",
                    "NOT_VERIFIED",
                    "BLOCKED",
                ];
                if !valid_statuses.contains(&status.as_str()) {
                    failures.push(format!(
                        "ledger_guard: Task {} has invalid status {:?}",
                        tid, status
                    ));
                }
                if status == "DONE" || status == "DONE (sudah sesuai)" {
                    if anchor.is_empty() {
                        failures.push(format!(
                            "ledger_guard: Task {} marked {} without evidence anchor",
                            tid, status
                        ));
                    } else {
                        let file_part = anchor.split('#').next().unwrap_or("");
                        if !file_part.is_empty() {
                            let ev_file = bukti_dir.join(file_part);
                            if !ev_file.exists() {
                                failures.push(format!(
                                    "ledger_guard: Task {} evidence file {:?} does not exist in bukti/v3/",
                                    tid, file_part
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn check_string_err(
    root: &Path,
    failures: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let fe_src = root.join("frontend/src");
    if !fe_src.exists() {
        return Ok(());
    }
    for entry in walkdir::WalkDir::new(&fe_src).into_iter().flatten() {
        let path = entry.path();
        if path.is_file() {
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            if (rel_path.ends_with(".ts")
                || rel_path.ends_with(".svelte")
                || rel_path.ends_with(".js"))
                && !rel_path.ends_with("errors.ts")
                && !rel_path.contains("test")
                && let Ok(content) = fs::read_to_string(path)
            {
                for (idx, line) in content.lines().enumerate() {
                    if line.contains("String(err)") {
                        failures.push(format!(
                            "string_err_guard: {}:{}: String(err) is forbidden, use errorText(err) from $lib/errors",
                            rel_path,
                            idx + 1
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_gated_commands(
    root: &Path,
    failures: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_toml_path = env::var("CAF_APP_TOML")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("app.toml"));

    if !app_toml_path.exists() {
        return Ok(());
    }

    let toml_str = fs::read_to_string(&app_toml_path)?;
    let parsed: toml::Value = toml::from_str(&toml_str)?;

    let mut disabled_modules = HashSet::new();
    if let Some(modules_tbl) = parsed.get("modules").and_then(|m| m.as_table()) {
        for (k, v) in modules_tbl {
            if v.as_bool() == Some(false) {
                disabled_modules.insert(k.as_str());
            }
        }
    }

    if disabled_modules.is_empty() {
        return Ok(());
    }

    let module_commands: &[(&str, &[&str])] = &[
        (
            "profiles",
            &[
                "list_profiles",
                "save_profile",
                "delete_profile",
                "verify_pin",
                "profile_unlock",
                "profile_lock",
            ],
        ),
        (
            "ai",
            &[
                "get_ai_settings",
                "save_ai_settings",
                "set_ai_api_key",
                "clear_ai_api_key",
                "ai_preview_context",
                "ai_chat",
            ],
        ),
        (
            "ai_advanced",
            &[
                "ai_get_providers",
                "ai_save_provider",
                "ai_delete_provider",
                "ai_get_routing_matrix",
                "ai_save_routing_rule",
                "ai_get_personas",
                "ai_save_persona",
                "ai_delete_persona",
                "ai_get_habits",
                "ai_search_habits",
                "ai_toggle_habit_pin",
                "ai_delete_habit",
                "ai_get_skills",
                "ai_save_skill",
                "ai_delete_skill",
                "ai_dispatch_task",
                "ai_dispatch_task_with_skill",
            ],
        ),
        ("feedback", &["submit_feedback"]),
        (
            "crash",
            &["get_pending_crash_report", "dismiss_crash_report"],
        ),
        (
            "pro",
            &[
                "pro_status",
                "pro_server_available",
                "pro_register",
                "pro_resend_verification",
                "pro_forgot_password",
                "pro_login",
                "pro_start_trial",
                "pro_account",
                "pro_revoke_device",
                "pro_logout",
                "pro_gated_action",
            ],
        ),
        (
            "integrations",
            &[
                "list_integrations",
                "set_integration_secret",
                "delete_integration_secret",
                "test_integration_connection",
                "get_integration_logs",
                "get_outbox_items",
                "flush_outbox_now",
            ],
        ),
        (
            "scheduler",
            &[
                "scheduler_list_jobs",
                "scheduler_set_job_enabled",
                "scheduler_update_schedule",
                "scheduler_run_job_now",
                "scheduler_get_history",
                "scheduler_get_desktop_settings",
                "scheduler_set_desktop_settings",
            ],
        ),
    ];

    // Check capabilities/default.json
    let cap_path = root.join("crates/caf-app/capabilities/default.json");
    let cap_content = if cap_path.exists() {
        fs::read_to_string(&cap_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Check frontend/src/lib/api/generated_bindings.ts
    let bindings_path = root.join("frontend/src/lib/api/generated_bindings.ts");
    let bindings_content = if bindings_path.exists() {
        fs::read_to_string(&bindings_path).unwrap_or_default()
    } else {
        String::new()
    };

    for &(mod_name, cmds) in module_commands {
        if disabled_modules.contains(mod_name) {
            for &cmd in cmds {
                let allow_perm = format!("allow-{}", cmd.replace('_', "-"));
                if !cap_content.is_empty() && cap_content.contains(&allow_perm) {
                    failures.push(format!(
                        "gated_commands: module '{}' is disabled in app.toml, but command '{}' is leaked in capabilities/default.json",
                        mod_name, cmd
                    ));
                }
                let invoke_fn = format!("invoke('{}'", cmd);
                if !bindings_content.is_empty() && bindings_content.contains(&invoke_fn) {
                    failures.push(format!(
                        "gated_commands: module '{}' is disabled in app.toml, but command '{}' is leaked in generated_bindings.ts",
                        mod_name, cmd
                    ));
                }
            }
        }
    }

    Ok(())
}
