//! Streaming JSON export and import for entity data.

use crate::error::{CafError, DataIoError};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{Read, Write};

/// Streaming JSON writer that emits an array of record objects.
pub struct JsonStreamWriter<W: Write> {
    writer: W,
    count: usize,
}

impl<W: Write> JsonStreamWriter<W> {
    pub fn new(mut writer: W) -> Result<Self, CafError> {
        writer
            .write_all(b"[\n")
            .map_err(|e| CafError::generic(e.to_string()))?;
        Ok(Self { writer, count: 0 })
    }

    pub fn write_record(&mut self, record: &HashMap<String, String>) -> Result<(), CafError> {
        if self.count > 0 {
            self.writer
                .write_all(b",\n")
                .map_err(|e| CafError::generic(e.to_string()))?;
        }
        let json_bytes =
            serde_json::to_vec(record).map_err(|e| CafError::generic(e.to_string()))?;
        self.writer
            .write_all(&json_bytes)
            .map_err(|e| CafError::generic(e.to_string()))?;
        self.count += 1;
        Ok(())
    }

    pub fn finish(mut self) -> Result<usize, CafError> {
        self.writer
            .write_all(b"\n]\n")
            .map_err(|e| CafError::generic(e.to_string()))?;
        self.writer
            .flush()
            .map_err(|e| CafError::generic(e.to_string()))?;
        Ok(self.count)
    }
}

/// Parses an input JSON stream as an array of row objects or maps.
pub fn parse_json_records<R: Read>(
    reader: R,
    max_rows: usize,
) -> Result<Vec<HashMap<String, String>>, CafError> {
    let value: Value =
        serde_json::from_reader(reader).map_err(|_| CafError::DataIo(DataIoError::ParseError))?;

    let arr = match value {
        Value::Array(items) => items,
        _ => return Err(CafError::DataIo(DataIoError::ParseError)),
    };

    if arr.len() > max_rows {
        return Err(CafError::DataIo(DataIoError::RowLimitExceeded));
    }

    let mut records = Vec::with_capacity(arr.len());
    for item in arr {
        match item {
            Value::Object(map) => {
                let mut record = HashMap::new();
                for (k, v) in map {
                    let str_val = match v {
                        Value::String(s) => s,
                        Value::Null => String::new(),
                        Value::Bool(b) => b.to_string(),
                        Value::Number(n) => n.to_string(),
                        Value::Array(a) => serde_json::to_string(&a).unwrap_or_default(),
                        Value::Object(o) => serde_json::to_string(&o).unwrap_or_default(),
                    };
                    record.insert(k, str_val);
                }
                records.push(record);
            }
            _ => return Err(CafError::DataIo(DataIoError::ParseError)),
        }
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_stream_writer_and_parser_roundtrip() {
        let mut buffer = Vec::new();
        let mut writer = JsonStreamWriter::new(&mut buffer).unwrap();

        let mut r1 = HashMap::new();
        r1.insert("title".to_string(), "First Note".to_string());
        r1.insert("content".to_string(), "Hello JSON".to_string());
        writer.write_record(&r1).unwrap();

        let mut r2 = HashMap::new();
        r2.insert("title".to_string(), "Second Note".to_string());
        r2.insert("content".to_string(), "World JSON".to_string());
        writer.write_record(&r2).unwrap();

        let count = writer.finish().unwrap();
        assert_eq!(count, 2);

        let parsed = parse_json_records(std::io::Cursor::new(buffer), 100).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(
            parsed
                .first()
                .and_then(|r| r.get("title"))
                .map(String::as_str),
            Some("First Note")
        );
        assert_eq!(
            parsed
                .get(1)
                .and_then(|r| r.get("title"))
                .map(String::as_str),
            Some("Second Note")
        );
    }
}
