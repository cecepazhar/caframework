//! Downloadable template generation for registered entities.

use crate::data_io::csv::write_csv_row;
use crate::data_io::entity::{EntityFieldType, get_entity};
use crate::data_io::json::JsonStreamWriter;
use crate::data_io::xlsx::generate_xlsx;
use crate::error::{CafError, DataIoError};
use std::collections::HashMap;
use std::io::Cursor;

/// Generates a downloadable template for the given entity in CSV, XLSX, or JSON format.
pub fn generate_entity_template(entity_key: &str, format: &str) -> Result<Vec<u8>, CafError> {
    let def = get_entity(entity_key)?;
    let fmt = format.to_lowercase();
    if !def.formats.iter().any(|f| f.eq_ignore_ascii_case(&fmt)) {
        return Err(CafError::DataIo(DataIoError::UnsupportedFormat));
    }

    let field_names: Vec<&str> = def.fields.iter().map(|f| f.name.as_str()).collect();

    // Generate sample row 1
    let mut sample_row1: HashMap<String, String> = HashMap::new();
    let mut sample_row2: HashMap<String, String> = HashMap::new();

    for field in &def.fields {
        match field.name.as_str() {
            "id" => {
                sample_row1.insert("id".into(), "".into());
                sample_row2.insert("id".into(), "".into());
            }
            "title" => {
                sample_row1.insert("title".into(), "Sample Note 1".into());
                sample_row2.insert("title".into(), "Sample Note 2".into());
            }
            "content" => {
                sample_row1.insert(
                    "content".into(),
                    "This is an example note description.".into(),
                );
                sample_row2.insert(
                    "content".into(),
                    "Another sample note for demonstration.".into(),
                );
            }
            "tags" => {
                sample_row1.insert("tags".into(), "example, demo".into());
                sample_row2.insert("tags".into(), "general".into());
            }
            "visibility" => {
                sample_row1.insert("visibility".into(), "shared".into());
                sample_row2.insert("visibility".into(), "private".into());
            }
            _ => {
                let s1 = match field.field_type {
                    EntityFieldType::Integer => "100",
                    EntityFieldType::Float => "99.5",
                    EntityFieldType::Boolean => "true",
                    EntityFieldType::DateTime => "2026-11-17T08:00:00Z",
                    _ => "Sample Value",
                };
                sample_row1.insert(field.name.clone(), s1.into());
                sample_row2.insert(field.name.clone(), s1.into());
            }
        }
    }

    match fmt.as_str() {
        "csv" => {
            let mut out = Vec::new();
            write_csv_row(&mut out, &field_names, false)?;

            let r1_vals: Vec<&str> = field_names
                .iter()
                .map(|f| sample_row1.get(*f).map(String::as_str).unwrap_or(""))
                .collect();
            write_csv_row(&mut out, &r1_vals, false)?;

            let r2_vals: Vec<&str> = field_names
                .iter()
                .map(|f| sample_row2.get(*f).map(String::as_str).unwrap_or(""))
                .collect();
            write_csv_row(&mut out, &r2_vals, false)?;

            Ok(out)
        }
        "xlsx" => {
            let r1: Vec<String> = field_names
                .iter()
                .map(|f| sample_row1.get(*f).cloned().unwrap_or_default())
                .collect();
            let r2: Vec<String> = field_names
                .iter()
                .map(|f| sample_row2.get(*f).cloned().unwrap_or_default())
                .collect();

            let mut out = Cursor::new(Vec::new());
            generate_xlsx(&mut out, &field_names, &[r1, r2], false)?;
            Ok(out.into_inner())
        }
        "json" => {
            let mut out = Vec::new();
            let mut writer = JsonStreamWriter::new(&mut out)?;
            writer.write_record(&sample_row1)?;
            writer.write_record(&sample_row2)?;
            writer.finish()?;
            Ok(out)
        }
        _ => Err(CafError::DataIo(DataIoError::UnsupportedFormat)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_generation_all_formats() {
        let csv_bytes = generate_entity_template("notes", "csv").unwrap();
        assert!(!csv_bytes.is_empty());
        let csv_str = String::from_utf8_lossy(&csv_bytes);
        assert!(csv_str.contains("title"));

        let xlsx_bytes = generate_entity_template("notes", "xlsx").unwrap();
        assert!(!xlsx_bytes.is_empty());

        let json_bytes = generate_entity_template("notes", "json").unwrap();
        assert!(!json_bytes.is_empty());
        let json_str = String::from_utf8_lossy(&json_bytes);
        assert!(json_str.contains("Sample Note 1"));
    }
}
