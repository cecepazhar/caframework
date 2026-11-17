//! The 7-step import pipeline per FR-17 with validation, dry-run, and transactional execution.

use crate::data_io::csv::{CsvParser, write_csv_row};
use crate::data_io::entity::{EntityDefinition, EntityFieldType, get_entity_and_adapter};
use crate::data_io::formula::unneutralize_cell;
use crate::data_io::json::parse_json_records;
use crate::data_io::xlsx::parse_xlsx;
use crate::error::{CafError, DataIoError, DbError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use ts_rs::TS;

pub const DEFAULT_MAX_FILE_BYTES: usize = 20 * 1024 * 1024; // 20 MB
pub const DEFAULT_MAX_ROWS: usize = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    InsertOnly,
    UpsertById,
    UpsertByNaturalKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImportStrategy {
    AllOrNothing,
    SkipInvalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ImportFileInfo {
    pub entity: String,
    pub format: String,
    pub columns: Vec<String>,
    pub total_rows: usize,
    pub suggested_mapping: HashMap<String, String>, // file_column -> entity_field
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct RowPreview {
    pub row_number: usize,
    pub values: HashMap<String, String>,
    pub action: String, // "insert" | "update" | "invalid"
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ImportPreview {
    pub total_rows: usize,
    pub to_insert: usize,
    pub to_update: usize,
    pub invalid_count: usize,
    pub preview_rows: Vec<RowPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FailedRow {
    pub row_number: usize,
    pub reason: String,
    pub raw_data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ImportExecutionReport {
    pub entity: String,
    pub total_rows: usize,
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: Vec<FailedRow>,
    pub failed_rows_csv: Option<String>,
}

/// Step 1 & 2: Parses the uploaded file, validates size/row limits, and generates column mapping.
pub fn parse_import_file(
    entity_key: &str,
    format: &str,
    data: &[u8],
) -> Result<ImportFileInfo, CafError> {
    if data.len() > DEFAULT_MAX_FILE_BYTES {
        return Err(CafError::DataIo(DataIoError::FileTooLarge));
    }

    let (def, _) = get_entity_and_adapter(entity_key)?;
    let fmt = format.to_lowercase();
    let records = parse_raw_records(&fmt, data, DEFAULT_MAX_ROWS)?;

    if records.len() > DEFAULT_MAX_ROWS {
        return Err(CafError::DataIo(DataIoError::RowLimitExceeded));
    }

    let mut columns = Vec::new();
    if let Some(first) = records.first() {
        for k in first.keys() {
            columns.push(k.clone());
        }
        columns.sort();
    }

    let suggested_mapping = auto_map_columns(&columns, &def);

    Ok(ImportFileInfo {
        entity: entity_key.to_string(),
        format: fmt,
        columns,
        total_rows: records.len(),
        suggested_mapping,
    })
}

/// Step 3 & 5: Performs dry run validation without committing changes.
pub fn preview_import(
    entity_key: &str,
    format: &str,
    data: &[u8],
    mapping: &HashMap<String, String>,
    mode: ImportMode,
) -> Result<ImportPreview, CafError> {
    if data.len() > DEFAULT_MAX_FILE_BYTES {
        return Err(CafError::DataIo(DataIoError::FileTooLarge));
    }

    let (def, adapter) = get_entity_and_adapter(entity_key)?;
    let fmt = format.to_lowercase();
    let records = parse_raw_records(&fmt, data, DEFAULT_MAX_ROWS)?;

    let mut conn = crate::db::open()?;
    let tx = conn
        .transaction()
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    let mut to_insert = 0;
    let mut to_update = 0;
    let mut invalid_count = 0;
    let mut preview_rows = Vec::new();

    for (idx, raw) in records.iter().enumerate() {
        let row_number = idx + 1;
        let mapped_fields = apply_mapping(raw, mapping);

        let validation_res = validate_row(&mapped_fields, &def);
        let (action, error) = match validation_res {
            Err(err_msg) => {
                invalid_count += 1;
                ("invalid".to_string(), Some(err_msg))
            }
            Ok(()) => {
                let id_opt = mapped_fields.get("id").map(String::as_str);
                let mut nat_keys = HashMap::new();
                for nk in &def.natural_keys {
                    if let Some(val) = mapped_fields.get(nk) {
                        nat_keys.insert(nk.clone(), val.clone());
                    }
                }

                let existing_id = match mode {
                    ImportMode::InsertOnly => {
                        let found = adapter.resolve_existing_id(&tx, id_opt, &nat_keys)?;
                        if found.is_some() {
                            Some("conflict".to_string())
                        } else {
                            None
                        }
                    }
                    ImportMode::UpsertById => {
                        adapter.resolve_existing_id(&tx, id_opt, &HashMap::new())?
                    }
                    ImportMode::UpsertByNaturalKey => {
                        adapter.resolve_existing_id(&tx, None, &nat_keys)?
                    }
                };

                if mode == ImportMode::InsertOnly && existing_id.is_some() {
                    invalid_count += 1;
                    (
                        "invalid".to_string(),
                        Some("Record already exists (insert-only conflict)".to_string()),
                    )
                } else if existing_id.is_some() {
                    to_update += 1;
                    ("update".to_string(), None)
                } else {
                    to_insert += 1;
                    ("insert".to_string(), None)
                }
            }
        };

        if row_number <= 10 {
            preview_rows.push(RowPreview {
                row_number,
                values: mapped_fields,
                action,
                error,
            });
        }
    }

    Ok(ImportPreview {
        total_rows: records.len(),
        to_insert,
        to_update,
        invalid_count,
        preview_rows,
    })
}

/// Step 6 & 7: Executes import inside a single transaction with specified strategy.
pub fn execute_import(
    entity_key: &str,
    format: &str,
    data: &[u8],
    mapping: &HashMap<String, String>,
    mode: ImportMode,
    strategy: ImportStrategy,
) -> Result<ImportExecutionReport, CafError> {
    if data.len() > DEFAULT_MAX_FILE_BYTES {
        return Err(CafError::DataIo(DataIoError::FileTooLarge));
    }

    let session = crate::session::get_current_session()?;
    let actor_profile_id = session.profile_id.clone();
    let device_id = crate::paths::device_id().unwrap_or_else(|_| "device-local".into());

    let (def, adapter) = get_entity_and_adapter(entity_key)?;
    let fmt = format.to_lowercase();
    let records = parse_raw_records(&fmt, data, DEFAULT_MAX_ROWS)?;

    let mut conn = crate::db::open()?;
    let tx = conn
        .transaction()
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    let mut inserted = 0;
    let mut updated = 0;
    let mut skipped = 0;
    let mut failed = Vec::new();

    for (idx, raw) in records.iter().enumerate() {
        let row_number = idx + 1;
        let mapped_fields = apply_mapping(raw, mapping);

        if let Err(err_msg) = validate_row(&mapped_fields, &def) {
            failed.push(FailedRow {
                row_number,
                reason: err_msg,
                raw_data: raw.clone(),
            });
            if strategy == ImportStrategy::AllOrNothing {
                return Ok(build_failed_report(
                    entity_key,
                    records.len(),
                    failed,
                    strategy,
                ));
            } else {
                skipped += 1;
                continue;
            }
        }

        let id_opt = mapped_fields.get("id").map(String::as_str);
        let mut nat_keys = HashMap::new();
        for nk in &def.natural_keys {
            if let Some(val) = mapped_fields.get(nk) {
                nat_keys.insert(nk.clone(), val.clone());
            }
        }

        let existing_id_res = match mode {
            ImportMode::InsertOnly => {
                let found = adapter.resolve_existing_id(&tx, id_opt, &nat_keys)?;
                if found.is_some() {
                    Err(CafError::DataIo(DataIoError::NaturalKeyConflict))
                } else {
                    Ok(None)
                }
            }
            ImportMode::UpsertById => adapter.resolve_existing_id(&tx, id_opt, &HashMap::new()),
            ImportMode::UpsertByNaturalKey => adapter.resolve_existing_id(&tx, None, &nat_keys),
        };

        let existing_id = match existing_id_res {
            Ok(opt) => opt,
            Err(e) => {
                failed.push(FailedRow {
                    row_number,
                    reason: e.to_string(),
                    raw_data: raw.clone(),
                });
                if strategy == ImportStrategy::AllOrNothing {
                    return Ok(build_failed_report(
                        entity_key,
                        records.len(),
                        failed,
                        strategy,
                    ));
                } else {
                    skipped += 1;
                    continue;
                }
            }
        };

        if let Some(target_id) = existing_id {
            if let Err(e) = adapter.update_record(
                &tx,
                &target_id,
                &mapped_fields,
                &actor_profile_id,
                &device_id,
            ) {
                failed.push(FailedRow {
                    row_number,
                    reason: e.to_string(),
                    raw_data: raw.clone(),
                });
                if strategy == ImportStrategy::AllOrNothing {
                    return Ok(build_failed_report(
                        entity_key,
                        records.len(),
                        failed,
                        strategy,
                    ));
                } else {
                    skipped += 1;
                    continue;
                }
            }
            updated += 1;
        } else {
            if let Err(e) =
                adapter.insert_record(&tx, &mapped_fields, &actor_profile_id, &device_id)
            {
                failed.push(FailedRow {
                    row_number,
                    reason: e.to_string(),
                    raw_data: raw.clone(),
                });
                if strategy == ImportStrategy::AllOrNothing {
                    return Ok(build_failed_report(
                        entity_key,
                        records.len(),
                        failed,
                        strategy,
                    ));
                } else {
                    skipped += 1;
                    continue;
                }
            }
            inserted += 1;
        }
    }

    // Commit transaction
    tx.commit()
        .map_err(|e| CafError::Db(DbError::Generic(e.to_string())))?;

    let failed_csv = if failed.is_empty() {
        None
    } else {
        Some(generate_failed_csv(&failed))
    };

    Ok(ImportExecutionReport {
        entity: entity_key.to_string(),
        total_rows: records.len(),
        inserted,
        updated,
        skipped,
        failed,
        failed_rows_csv: failed_csv,
    })
}

fn build_failed_report(
    entity: &str,
    total_rows: usize,
    failed: Vec<FailedRow>,
    _strategy: ImportStrategy,
) -> ImportExecutionReport {
    let failed_csv = Some(generate_failed_csv(&failed));
    ImportExecutionReport {
        entity: entity.to_string(),
        total_rows,
        inserted: 0,
        updated: 0,
        skipped: total_rows,
        failed,
        failed_rows_csv: failed_csv,
    }
}

pub fn generate_failed_csv(failed: &[FailedRow]) -> String {
    let mut out = Vec::new();
    let headers = ["row_number", "error_reason", "raw_data"];
    let _ = write_csv_row(&mut out, &headers, false);

    for f in failed {
        let row_num = f.row_number.to_string();
        let raw_json = serde_json::to_string(&f.raw_data).unwrap_or_default();
        let row = [row_num.as_str(), f.reason.as_str(), raw_json.as_str()];
        let _ = write_csv_row(&mut out, &row, false);
    }
    String::from_utf8_lossy(&out).to_string()
}

fn validate_row(mapped: &HashMap<String, String>, def: &EntityDefinition) -> Result<(), String> {
    for field in &def.fields {
        let val = mapped
            .get(&field.name)
            .map(String::as_str)
            .unwrap_or("")
            .trim();
        if field.required && val.is_empty() {
            return Err(format!("Field '{}' is required", field.name));
        }

        if !val.is_empty() {
            match field.field_type {
                EntityFieldType::Integer => {
                    if val.parse::<i64>().is_err() {
                        return Err(format!("Field '{}' must be an integer", field.name));
                    }
                }
                EntityFieldType::Float => {
                    if val.parse::<f64>().is_err() {
                        return Err(format!("Field '{}' must be a number", field.name));
                    }
                }
                EntityFieldType::Boolean => {
                    let v = val.to_lowercase();
                    if v != "true"
                        && v != "false"
                        && v != "1"
                        && v != "0"
                        && v != "yes"
                        && v != "no"
                    {
                        return Err(format!("Field '{}' must be a boolean", field.name));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn apply_mapping(
    raw: &HashMap<String, String>,
    mapping: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut mapped = HashMap::new();
    for (file_col, entity_field) in mapping {
        if let Some(val) = raw.get(file_col) {
            let clean_val = unneutralize_cell(val.trim());
            mapped.insert(entity_field.clone(), clean_val.to_string());
        }
    }
    mapped
}

fn auto_map_columns(columns: &[String], def: &EntityDefinition) -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    for col in columns {
        let clean = col.trim().to_lowercase().replace([' ', '-'], "_");
        for field in &def.fields {
            let field_clean = field.name.to_lowercase();
            if clean == field_clean {
                mapping.insert(col.clone(), field.name.clone());
                break;
            }
        }
    }
    mapping
}

fn parse_raw_records(
    format: &str,
    data: &[u8],
    max_rows: usize,
) -> Result<Vec<HashMap<String, String>>, CafError> {
    match format {
        "csv" => {
            let mut parser = CsvParser::new(data, max_rows + 1);
            let header_row = parser
                .next_row()?
                .ok_or(CafError::DataIo(DataIoError::ParseError))?;

            let mut records = Vec::new();
            while let Some(row) = parser.next_row()? {
                let mut map = HashMap::new();
                for (i, cell) in row.into_iter().enumerate() {
                    if let Some(col_name) = header_row.get(i) {
                        map.insert(col_name.clone(), cell);
                    }
                }
                records.push(map);
            }
            Ok(records)
        }
        "xlsx" => {
            let cursor = Cursor::new(data);
            let rows = parse_xlsx(cursor, max_rows + 1)?;
            if rows.is_empty() {
                return Err(CafError::DataIo(DataIoError::ParseError));
            }

            let header_row = rows.first().cloned().unwrap_or_default();
            let mut records = Vec::new();

            for row in rows.into_iter().skip(1) {
                let mut map = HashMap::new();
                for (i, cell) in row.into_iter().enumerate() {
                    if let Some(col_name) = header_row.get(i) {
                        map.insert(col_name.clone(), cell);
                    }
                }
                records.push(map);
            }
            Ok(records)
        }
        "json" => parse_json_records(data, max_rows),
        _ => Err(CafError::DataIo(DataIoError::UnsupportedFormat)),
    }
}
