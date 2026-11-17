//! Entity export orchestrator with permission, visibility, and audit enforcement.

use crate::data_io::csv::write_csv_row;
use crate::data_io::entity::get_entity_and_adapter;
use crate::data_io::json::JsonStreamWriter;
use crate::data_io::xlsx::generate_xlsx;
use crate::error::{CafError, DataIoError};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExportResult {
    pub entity: String,
    pub format: String,
    pub row_count: usize,
    pub file_path: String,
    pub file_size_bytes: u64,
}

/// Executes export of an entity into CSV, XLSX, or JSON, respecting visibility rules and auditing.
pub fn export_entity(entity_key: &str, format: &str) -> Result<ExportResult, CafError> {
    let session = crate::session::get_current_session()?;
    let is_super = session.has_permission("*") || crate::rbac::is_super_role(&session.role);
    let actor_profile_id = &session.profile_id;

    let (def, adapter) = get_entity_and_adapter(entity_key)?;
    let fmt = format.to_lowercase();
    if !def.formats.iter().any(|f| f.eq_ignore_ascii_case(&fmt)) {
        return Err(CafError::DataIo(DataIoError::UnsupportedFormat));
    }

    let conn = crate::db::open()?;
    let records = adapter.fetch_export_records(&conn, is_super, actor_profile_id)?;

    // Audit super-role private reads if any private record of another profile was fetched
    if is_super {
        let private_reads_count = records
            .iter()
            .filter(|r| {
                let owner = r.get("owner_profile_id").map(String::as_str).unwrap_or("");
                let vis = r.get("visibility").map(String::as_str).unwrap_or("shared");
                owner != actor_profile_id && vis != "shared"
            })
            .count();

        if private_reads_count > 0 {
            let _ = crate::audit::log_event(
                "PRIVATE_READ",
                Some(actor_profile_id),
                &format!("export_entity={entity_key}, count={private_reads_count}"),
            );
        }
    }

    // Determine target export file path
    let exports_dir = crate::paths::data_dir()?.join("exports");
    std::fs::create_dir_all(&exports_dir).map_err(|e| CafError::generic(e.to_string()))?;

    let filename = format!(
        "{}_{}_{}.{}",
        entity_key,
        chrono::Utc::now().format("%Y%m%d_%H%M%S"),
        uuid::Uuid::now_v7(),
        fmt
    );
    let file_path = exports_dir.join(&filename);

    let field_names: Vec<&str> = def.fields.iter().map(|f| f.name.as_str()).collect();

    match fmt.as_str() {
        "csv" => {
            let file = File::create(&file_path).map_err(|e| CafError::generic(e.to_string()))?;
            let mut writer = BufWriter::new(file);

            // Write header (no neutralization for field names)
            write_csv_row(&mut writer, &field_names, false)?;

            // Write rows (neutralize formula injection)
            for r in &records {
                let row_values: Vec<&str> = field_names
                    .iter()
                    .map(|f| r.get(*f).map(String::as_str).unwrap_or(""))
                    .collect();
                write_csv_row(&mut writer, &row_values, true)?;
            }
        }
        "xlsx" => {
            let mut xlsx_rows: Vec<Vec<String>> = Vec::with_capacity(records.len());
            for r in &records {
                let row: Vec<String> = field_names
                    .iter()
                    .map(|f| r.get(*f).cloned().unwrap_or_default())
                    .collect();
                xlsx_rows.push(row);
            }

            let file = File::create(&file_path).map_err(|e| CafError::generic(e.to_string()))?;
            generate_xlsx(file, &field_names, &xlsx_rows, true)?;
        }
        "json" => {
            let file = File::create(&file_path).map_err(|e| CafError::generic(e.to_string()))?;
            let mut writer = JsonStreamWriter::new(BufWriter::new(file))?;
            for r in &records {
                writer.write_record(r)?;
            }
            writer.finish()?;
        }
        _ => return Err(CafError::DataIo(DataIoError::UnsupportedFormat)),
    }

    let metadata = std::fs::metadata(&file_path).map_err(|e| CafError::generic(e.to_string()))?;

    Ok(ExportResult {
        entity: entity_key.to_string(),
        format: fmt,
        row_count: records.len(),
        file_path: file_path.to_string_lossy().to_string(),
        file_size_bytes: metadata.len(),
    })
}
