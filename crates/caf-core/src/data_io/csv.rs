//! RFC 4180 compliant streaming CSV parser and writer with formula injection protection.

use crate::data_io::formula::neutralize_cell;
use crate::error::{CafError, DataIoError};
use std::io::{BufRead, BufReader, Read, Write};

/// Writes a single CSV row to the output stream.
/// Quotes values that contain commas, double quotes, or newlines, doubling any internal quotes.
/// Any formula injection characters are neutralized prior to writing when neutralize is true.
pub fn write_csv_row<W: Write>(
    writer: &mut W,
    row: &[&str],
    neutralize: bool,
) -> Result<(), CafError> {
    for (i, cell) in row.iter().enumerate() {
        if i > 0 {
            writer
                .write_all(b",")
                .map_err(|e| CafError::generic(e.to_string()))?;
        }
        let formatted = if neutralize {
            neutralize_cell(cell)
        } else {
            (*cell).to_string()
        };

        let needs_quotes = formatted.contains(',')
            || formatted.contains('"')
            || formatted.contains('\n')
            || formatted.contains('\r');

        if needs_quotes {
            writer
                .write_all(b"\"")
                .map_err(|e| CafError::generic(e.to_string()))?;
            for ch in formatted.chars() {
                if ch == '"' {
                    writer
                        .write_all(b"\"\"")
                        .map_err(|e| CafError::generic(e.to_string()))?;
                } else {
                    let mut b = [0u8; 4];
                    let encoded = ch.encode_utf8(&mut b);
                    writer
                        .write_all(encoded.as_bytes())
                        .map_err(|e| CafError::generic(e.to_string()))?;
                }
            }
            writer
                .write_all(b"\"")
                .map_err(|e| CafError::generic(e.to_string()))?;
        } else {
            writer
                .write_all(formatted.as_bytes())
                .map_err(|e| CafError::generic(e.to_string()))?;
        }
    }
    writer
        .write_all(b"\r\n")
        .map_err(|e| CafError::generic(e.to_string()))?;
    Ok(())
}

/// Streaming RFC 4180 CSV parser that parses rows one by one from a reader using BufRead.
pub struct CsvParser<R: Read> {
    reader: BufReader<R>,
    in_quotes: bool,
    current_cell: Vec<u8>,
    current_row: Vec<String>,
    row_count: usize,
    max_rows: usize,
    eof: bool,
}

impl<R: Read> CsvParser<R> {
    pub fn new(reader: R, max_rows: usize) -> Self {
        Self {
            reader: BufReader::new(reader),
            in_quotes: false,
            current_cell: Vec::with_capacity(64),
            current_row: Vec::new(),
            row_count: 0,
            max_rows,
            eof: false,
        }
    }

    /// Reads the next complete CSV record.
    pub fn next_row(&mut self) -> Result<Option<Vec<String>>, CafError> {
        if self.eof {
            return Ok(None);
        }

        loop {
            let (consumed, row_finished) = {
                let available = match self.reader.fill_buf() {
                    Ok(b) => b,
                    Err(_) => return Err(CafError::DataIo(DataIoError::ParseError)),
                };

                if available.is_empty() {
                    self.eof = true;
                    if !self.current_cell.is_empty() || !self.current_row.is_empty() {
                        let cell_str = String::from_utf8_lossy(&self.current_cell).to_string();
                        self.current_cell.clear();
                        self.current_row.push(cell_str);
                        self.row_count += 1;
                        if self.row_count > self.max_rows {
                            return Err(CafError::DataIo(DataIoError::RowLimitExceeded));
                        }
                        return Ok(Some(std::mem::take(&mut self.current_row)));
                    }
                    return Ok(None);
                }

                let mut idx = 0;
                let mut finished = false;

                while idx < available.len() {
                    let b = *available.get(idx).unwrap_or(&0);
                    if b == b'"' {
                        if self.in_quotes {
                            // Check if next byte is also '"'
                            if idx + 1 < available.len()
                                && *available.get(idx + 1).unwrap_or(&0) == b'"'
                            {
                                self.current_cell.push(b'"');
                                idx += 2;
                                continue;
                            } else {
                                self.in_quotes = false;
                            }
                        } else {
                            self.in_quotes = true;
                        }
                    } else if b == b',' {
                        if self.in_quotes {
                            self.current_cell.push(b',');
                        } else {
                            let cell_str = String::from_utf8_lossy(&self.current_cell).to_string();
                            self.current_cell.clear();
                            self.current_row.push(cell_str);
                        }
                    } else if b == b'\r' {
                        if self.in_quotes {
                            self.current_cell.push(b'\r');
                        }
                    } else if b == b'\n' {
                        if self.in_quotes {
                            self.current_cell.push(b'\n');
                        } else {
                            let cell_str = String::from_utf8_lossy(&self.current_cell).to_string();
                            self.current_cell.clear();
                            self.current_row.push(cell_str);
                            idx += 1;
                            finished = true;
                            break;
                        }
                    } else {
                        self.current_cell.push(b);
                    }
                    idx += 1;
                }
                (idx, finished)
            };

            self.reader.consume(consumed);

            if row_finished {
                self.row_count += 1;
                if self.row_count > self.max_rows {
                    return Err(CafError::DataIo(DataIoError::RowLimitExceeded));
                }
                return Ok(Some(std::mem::take(&mut self.current_row)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_writer_and_reader_roundtrip() {
        let mut out = Vec::new();
        let headers = ["Title", "Content", "Formula"];
        write_csv_row(&mut out, &headers, false).unwrap();

        let row1 = ["Note 1", "Hello, world!", "=1+1"];
        write_csv_row(&mut out, &row1, true).unwrap();

        let row2 = ["Note 2", "Multi\nline \"quote\"", "Normal"];
        write_csv_row(&mut out, &row2, true).unwrap();

        let mut parser = CsvParser::new(std::io::Cursor::new(out), 100);
        let h = parser.next_row().unwrap().unwrap();
        assert_eq!(h, vec!["Title", "Content", "Formula"]);

        let r1 = parser.next_row().unwrap().unwrap();
        assert_eq!(r1.first().map(String::as_str), Some("Note 1"));
        assert_eq!(r1.get(1).map(String::as_str), Some("Hello, world!"));
        assert_eq!(r1.get(2).map(String::as_str), Some("'=1+1"));

        let r2 = parser.next_row().unwrap().unwrap();
        assert_eq!(r2.first().map(String::as_str), Some("Note 2"));
        assert_eq!(r2.get(1).map(String::as_str), Some("Multi\nline \"quote\""));
        assert_eq!(r2.get(2).map(String::as_str), Some("Normal"));

        assert!(parser.next_row().unwrap().is_none());
    }

    #[test]
    fn test_csv_parser_row_limit() {
        let data = "col\na\nb\nc\nd\n";
        let mut parser = CsvParser::new(std::io::Cursor::new(data), 3);
        assert!(parser.next_row().is_ok());
        assert!(parser.next_row().is_ok());
        assert!(parser.next_row().is_ok());
        let err = parser.next_row();
        assert!(matches!(
            err,
            Err(CafError::DataIo(DataIoError::RowLimitExceeded))
        ));
    }
}
