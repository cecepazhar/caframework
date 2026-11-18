//! XLSX export and import engine with formula neutralization.

use crate::data_io::formula::neutralize_cell;
use crate::error::{CafError, DataIoError};
use std::io::{Read, Seek, Write};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

fn xml_escape(val: &str) -> String {
    let mut out = String::with_capacity(val.len());
    for c in val.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn col_name(mut col_idx: usize) -> String {
    let mut name = String::new();
    loop {
        let rem = (col_idx % 26) as u8;
        name.insert(0, (b'A' + rem) as char);
        if col_idx < 26 {
            break;
        }
        col_idx = (col_idx / 26) - 1;
    }
    name
}

/// Generates an XLSX file containing the given headers and records.
/// Formula injection is neutralized for every string cell when neutralize is true.
pub fn generate_xlsx<W: Write + Seek>(
    writer: W,
    headers: &[&str],
    rows: &[Vec<String>],
    neutralize: bool,
) -> Result<(), CafError> {
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default();

    // 1. [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)
        .map_err(|e| CafError::generic(e.to_string()))?;
    zip.write_all(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\
<Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\
</Types>",
    )
    .map_err(|e| CafError::generic(e.to_string()))?;

    // 2. _rels/.rels
    zip.start_file("_rels/.rels", options)
        .map_err(|e| CafError::generic(e.to_string()))?;
    zip.write_all(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/>\
</Relationships>",
    )
    .map_err(|e| CafError::generic(e.to_string()))?;

    // 3. xl/_rels/workbook.xml.rels
    zip.start_file("xl/_rels/workbook.xml.rels", options)
        .map_err(|e| CafError::generic(e.to_string()))?;
    zip.write_all(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/>\
</Relationships>",
    )
    .map_err(|e| CafError::generic(e.to_string()))?;

    // 4. xl/workbook.xml
    zip.start_file("xl/workbook.xml", options)
        .map_err(|e| CafError::generic(e.to_string()))?;
    zip.write_all(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
<sheets><sheet name=\"Sheet1\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>",
    )
    .map_err(|e| CafError::generic(e.to_string()))?;

    // 5. xl/worksheets/sheet1.xml
    zip.start_file("xl/worksheets/sheet1.xml", options)
        .map_err(|e| CafError::generic(e.to_string()))?;

    zip.write_all(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>",
    )
    .map_err(|e| CafError::generic(e.to_string()))?;

    // Header row
    zip.write_all(b"<row r=\"1\">")
        .map_err(|e| CafError::generic(e.to_string()))?;
    for (c_idx, header) in headers.iter().enumerate() {
        let cell_ref = format!("{}{}", col_name(c_idx), 1);
        let escaped = xml_escape(header);
        let cell_xml = format!("<c r=\"{cell_ref}\" t=\"inlineStr\"><is><t>{escaped}</t></is></c>");
        zip.write_all(cell_xml.as_bytes())
            .map_err(|e| CafError::generic(e.to_string()))?;
    }
    zip.write_all(b"</row>")
        .map_err(|e| CafError::generic(e.to_string()))?;

    // Data rows
    for (r_idx, row) in rows.iter().enumerate() {
        let row_num = r_idx + 2;
        let row_tag = format!("<row r=\"{row_num}\">");
        zip.write_all(row_tag.as_bytes())
            .map_err(|e| CafError::generic(e.to_string()))?;

        for (c_idx, cell) in row.iter().enumerate() {
            let cell_ref = format!("{}{}", col_name(c_idx), row_num);
            let formatted = if neutralize {
                neutralize_cell(cell)
            } else {
                std::borrow::Cow::Borrowed(cell.as_str())
            };
            let escaped = xml_escape(&formatted);
            let cell_xml =
                format!("<c r=\"{cell_ref}\" t=\"inlineStr\"><is><t>{escaped}</t></is></c>");
            zip.write_all(cell_xml.as_bytes())
                .map_err(|e| CafError::generic(e.to_string()))?;
        }
        zip.write_all(b"</row>")
            .map_err(|e| CafError::generic(e.to_string()))?;
    }

    zip.write_all(b"</sheetData></worksheet>")
        .map_err(|e| CafError::generic(e.to_string()))?;

    zip.finish().map_err(|e| CafError::generic(e.to_string()))?;
    Ok(())
}

/// Parses an XLSX file stream from a reader and returns rows of text strings.
pub fn parse_xlsx<R: Read + Seek>(
    reader: R,
    max_rows: usize,
) -> Result<Vec<Vec<String>>, CafError> {
    let mut archive =
        ZipArchive::new(reader).map_err(|_| CafError::DataIo(DataIoError::ParseError))?;

    // 1. Read shared strings if any
    let mut shared_strings: Vec<String> = Vec::new();
    if let Ok(mut ss_file) = archive.by_name("xl/sharedStrings.xml") {
        let mut content = String::new();
        ss_file
            .read_to_string(&mut content)
            .map_err(|_| CafError::DataIo(DataIoError::ParseError))?;
        // Simple extraction of <t>...</t> tags
        let mut cur = content.as_str();
        while let Some(start) = cur.find("<t>") {
            let after = cur.get(start + 3..).unwrap_or("");
            if let Some(end) = after.find("</t>") {
                let text = after.get(..end).unwrap_or("");
                shared_strings.push(unescape_xml(text));
                cur = after.get(end + 4..).unwrap_or("");
            } else {
                break;
            }
        }
    }

    // 2. Read sheet1.xml
    let mut sheet_file = archive
        .by_name("xl/worksheets/sheet1.xml")
        .map_err(|_| CafError::DataIo(DataIoError::ParseError))?;
    let mut sheet_xml = String::new();
    sheet_file
        .read_to_string(&mut sheet_xml)
        .map_err(|_| CafError::DataIo(DataIoError::ParseError))?;

    let mut rows: Vec<Vec<String>> = Vec::new();

    // Parse <row ...>...</row>
    let mut cur = sheet_xml.as_str();
    while let Some(r_start) = cur.find("<row ") {
        let after_row = cur.get(r_start..).unwrap_or("");
        let row_content_start = match after_row.find('>') {
            Some(i) => i + 1,
            None => break,
        };
        let row_content = after_row.get(row_content_start..).unwrap_or("");
        let row_end = match row_content.find("</row>") {
            Some(i) => i,
            None => break,
        };
        let inner_row = row_content.get(..row_end).unwrap_or("");
        cur = row_content.get(row_end + 6..).unwrap_or("");

        let mut row_cells: Vec<String> = Vec::new();
        let mut c_cur = inner_row;
        while let Some(c_start) = c_cur.find("<c ") {
            let c_after = c_cur.get(c_start..).unwrap_or("");
            let c_tag_end = match c_after.find('>') {
                Some(i) => i,
                None => break,
            };
            let c_tag = c_after.get(..c_tag_end).unwrap_or("");
            let is_inline = c_tag.contains("t=\"inlineStr\"");
            let is_shared = c_tag.contains("t=\"s\"");

            let c_content_start = c_tag_end + 1;
            let c_content = c_after.get(c_content_start..).unwrap_or("");
            let c_end = match c_content.find("</c>") {
                Some(i) => i,
                None => break,
            };
            let cell_body = c_content.get(..c_end).unwrap_or("");
            c_cur = c_content.get(c_end + 4..).unwrap_or("");

            let cell_value = if is_inline {
                extract_between(cell_body, "<t>", "</t>")
                    .map(|s| unescape_xml(&s))
                    .unwrap_or_default()
            } else if is_shared {
                let idx_str = extract_between(cell_body, "<v>", "</v>").unwrap_or_default();
                if let Ok(idx) = idx_str.trim().parse::<usize>() {
                    shared_strings.get(idx).cloned().unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                extract_between(cell_body, "<v>", "</v>")
                    .map(|s| unescape_xml(&s))
                    .unwrap_or_default()
            };

            row_cells.push(cell_value);
        }

        if !row_cells.is_empty() {
            rows.push(row_cells);
            if rows.len() > max_rows {
                return Err(CafError::DataIo(DataIoError::RowLimitExceeded));
            }
        }
    }

    Ok(rows)
}

fn extract_between(source: &str, start_tag: &str, end_tag: &str) -> Option<String> {
    let start = source.find(start_tag)?;
    let after = source.get(start + start_tag.len()..)?;
    let end = after.find(end_tag)?;
    Some(after.get(..end)?.to_string())
}

fn unescape_xml(val: &str) -> String {
    val.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_xlsx_generator_and_parser_roundtrip() {
        let headers = ["Title", "Formula", "Content"];
        let rows = vec![
            vec!["Note 1".into(), "=SUM(A1:B1)".into(), "Hello XLSX".into()],
            vec!["Note 2".into(), "Plain".into(), "Multi\nLine".into()],
        ];

        let mut buffer = Cursor::new(Vec::new());
        generate_xlsx(&mut buffer, &headers, &rows, true).unwrap();

        buffer.set_position(0);
        let parsed = parse_xlsx(buffer, 100).unwrap();
        assert_eq!(parsed.len(), 3); // 1 header + 2 data rows

        let h = parsed.first().unwrap();
        assert_eq!(h.first().map(String::as_str), Some("Title"));
        assert_eq!(h.get(1).map(String::as_str), Some("Formula"));
        assert_eq!(h.get(2).map(String::as_str), Some("Content"));

        let r1 = parsed.get(1).unwrap();
        assert_eq!(r1.first().map(String::as_str), Some("Note 1"));
        assert_eq!(r1.get(1).map(String::as_str), Some("'=SUM(A1:B1)")); // formula neutralized!
        assert_eq!(r1.get(2).map(String::as_str), Some("Hello XLSX"));

        let r2 = parsed.get(2).unwrap();
        assert_eq!(r2.first().map(String::as_str), Some("Note 2"));
        assert_eq!(r2.get(1).map(String::as_str), Some("Plain"));
        assert_eq!(r2.get(2).map(String::as_str), Some("Multi\nLine"));
    }
}
