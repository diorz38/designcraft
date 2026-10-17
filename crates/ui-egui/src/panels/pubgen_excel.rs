//! Excel (.xlsx) to CSV Data helper for Pubgen.
//!
//! Converts multi-sheet Excel workbooks into:
//! 1. One catalog CSV for `Page from CSV` (Card 1) with 4 columns: `type,number,title,file`
//!    extracted from the first row of each sheet.
//! 2. Individual table CSVs for each sheet (named from the sheet name) with auto-detected
//!    merged headers (using the `--` stacked header notation).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use zip::ZipArchive;

/// Parsed worksheet data from an XLSX file.
#[derive(Debug, Clone, Default)]
pub struct ExcelSheetData {
    pub name: String,
    /// 2D cell values grid [row][col] (0-indexed)
    pub grid: Vec<Vec<String>>,
    /// Merged cell ranges: ((start_col, start_row), (end_col, end_row))
    pub merges: Vec<((usize, usize), (usize, usize))>,
}

/// Generated CSV data from conversion.
#[derive(Debug, Clone, Default)]
pub struct ExcelConversionResult {
    pub catalog_csv_name: String,
    pub catalog_csv_content: String,
    /// (filename, csv_content)
    pub table_csvs: Vec<(String, String)>,
}

/// Parse Excel cell coordinate (e.g. "A1" -> (0, 0), "B2" -> (1, 1), "AA1" -> (26, 0)).
pub fn parse_cell_ref(r: &str) -> Option<(usize, usize)> {
    let mut col_str = String::new();
    let mut row_str = String::new();
    for ch in r.chars() {
        if ch.is_ascii_alphabetic() {
            col_str.push(ch.to_ascii_uppercase());
        } else if ch.is_ascii_digit() {
            row_str.push(ch);
        }
    }
    if col_str.is_empty() || row_str.is_empty() {
        return None;
    }
    let row: usize = row_str.parse::<usize>().ok()?.saturating_sub(1);
    let mut col: usize = 0;
    for b in col_str.bytes() {
        col = col.saturating_mul(26).saturating_add((b - b'A' + 1) as usize);
    }
    Some((col.saturating_sub(1), row))
}

/// Parse an Excel cell range (e.g. "B2:C2" -> ((1, 1), (2, 1))).
pub fn parse_range(ref_str: &str) -> Option<((usize, usize), (usize, usize))> {
    let (s, e) = ref_str.split_once(':')?;
    let start = parse_cell_ref(s)?;
    let end = parse_cell_ref(e)?;
    let min_col = start.0.min(end.0);
    let max_col = start.0.max(end.0);
    let min_row = start.1.min(end.1);
    let max_row = start.1.max(end.1);
    Some(((min_col, min_row), (max_col, max_row)))
}

/// Sanitize sheet name for safe filename on all platforms.
pub fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        "sheet".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Escape a string for RFC 4180 CSV output.
pub fn escape_csv_field(val: &str) -> String {
    if val.contains(',') || val.contains('"') || val.contains('\n') || val.contains('\r') || val.contains(';') {
        let escaped = val.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        val.to_string()
    }
}

/// Parse the first row of an Excel sheet into `(type, number, title, file)`.
///
/// Example:
/// "Tabel 02.03 Jumlah Pegawai Negeri Sipil Pemerintah..."
/// -> type: "Tabel", number: "02.03", title: "Jumlah Pegawai...", file: sheet_name
pub fn parse_catalog_entry(first_row_cells: &[String], sheet_name: &str) -> (String, String, String, String) {
    let non_empty: Vec<&str> = first_row_cells.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if non_empty.is_empty() {
        return ("Tabel".into(), sheet_name.into(), sheet_name.into(), sheet_name.into());
    }

    if non_empty.len() >= 3 {
        // First row cells are already split: [type, number, title...]
        let t = non_empty[0].to_string();
        let n = non_empty[1].trim_matches(|c| c == ':' || c == ';').to_string();
        let title = non_empty[2..].join(" ");
        return (t, n, title, sheet_name.into());
    }

    // Title is contained in a single cell (often merged A1..D1)
    let full = non_empty.join(" ");
    let words: Vec<&str> = full.split_whitespace().collect();
    if words.len() >= 3 {
        let first_lower = words[0].to_lowercase();
        let is_type = [
            "tabel", "table", "grafik", "graph", "bagan", "chart", "bab", "chapter", "section", "peta", "map",
        ]
        .iter()
        .any(|k| first_lower.starts_with(k));

        if is_type {
            let t = words[0].to_string();
            let n = words[1].trim_matches(|c| c == ':' || c == ';').to_string();
            let title = words[2..].join(" ");
            return (t, n, title, sheet_name.into());
        }

        // What if word 0 is already a number/code (e.g. "02.03 Judul...")
        if words[0].chars().any(|c| c.is_ascii_digit()) {
            let n = words[0].trim_matches(|c| c == ':' || c == ';' || c == '.').to_string();
            let title = words[1..].join(" ");
            return ("Tabel".into(), n, title, sheet_name.into());
        }
    }

    if words.len() == 2 {
        return ("Tabel".into(), words[0].into(), words[1].into(), sheet_name.into());
    }

    ("Tabel".into(), sheet_name.into(), full, sheet_name.into())
}

/// Automatically reconstruct table column headers by propagating horizontal & vertical
/// cell merges across header rows and joining ancestor levels with `--`.
pub fn build_merged_table_headers(
    grid: &[Vec<String>],
    merges: &[((usize, usize), (usize, usize))],
    header_start_row: usize,
    header_end_row: usize,
    num_cols: usize,
) -> Vec<String> {
    let header_rows_count = (header_end_row.saturating_sub(header_start_row) + 1).max(1);
    let mut eff_grid = vec![vec![String::new(); num_cols]; header_rows_count];

    // 1. Populate direct cell values
    for (r_offset, r) in (header_start_row..=header_end_row).enumerate() {
        if let Some(row_cells) = grid.get(r) {
            for c in 0..num_cols {
                if let Some(val) = row_cells.get(c) {
                    eff_grid[r_offset][c] = val.trim().to_string();
                }
            }
        }
    }

    // 2. Propagate horizontal merges (e.g. B2:C2 has "Jenis Kelamin" in B2 -> copy to C2)
    for &((sc, sr), (ec, er)) in merges {
        if sr <= header_end_row && er >= header_start_row {
            let val = grid.get(sr).and_then(|row| row.get(sc)).map(|s| s.trim()).unwrap_or("");
            if !val.is_empty() && sc < ec {
                for r in sr.max(header_start_row)..=er.min(header_end_row) {
                    let r_off = r.saturating_sub(header_start_row);
                    for c in sc..=ec.min(num_cols.saturating_sub(1)) {
                        if eff_grid[r_off][c].is_empty() {
                            eff_grid[r_off][c] = val.to_string();
                        }
                    }
                }
            }
        }
    }

    // 3. Chain ancestor header levels down the rows into "Parent--Child"
    let mut col_headers = Vec::with_capacity(num_cols);
    for c in 0..num_cols {
        let mut parts: Vec<String> = Vec::new();
        for r_off in 0..header_rows_count {
            let val = eff_grid[r_off][c].trim();
            let abs_row = r_off + header_start_row;

            // Check if this cell is a continuation of a vertical merge
            let is_vert_merge_continuation = merges.iter().any(|&((sc, sr), (ec, er))| {
                sc <= c && c <= ec && sr < abs_row && er >= abs_row
            });

            if !val.is_empty() && !is_vert_merge_continuation {
                if parts.last().map(|s| s.as_str()) != Some(val) {
                    parts.push(val.to_string());
                }
            }
        }

        if parts.is_empty() {
            col_headers.push(format!("Column {}", c + 1));
        } else {
            col_headers.push(parts.join("--"));
        }
    }

    col_headers
}

/// Determine where the multi-row table header ends (typically row 1 to row 2 or 3).
pub fn detect_header_end_row(
    grid: &[Vec<String>],
    merges: &[((usize, usize), (usize, usize))],
    header_start_row: usize,
    num_cols: usize,
) -> usize {
    if grid.len() <= header_start_row + 1 {
        return header_start_row;
    }

    let mut end_row = header_start_row;

    // Check merges starting at or overlapping header_start_row
    for &((_, sr), (_, er)) in merges {
        if sr == header_start_row && er > header_start_row {
            end_row = end_row.max(er);
        }
    }

    // Check if the next row only has values under horizontally merged columns
    let candidate = header_start_row + 1;
    if candidate < grid.len() && candidate <= end_row + 1 && candidate < header_start_row + 3 {
        let has_horizontal_merges_in_parent = merges.iter().any(|&((sc, sr), (ec, _))| {
            sr == header_start_row && sc < ec
        });
        if has_horizontal_merges_in_parent {
            let row_cells = &grid[candidate];
            let non_empty_count = row_cells.iter().take(num_cols).filter(|s| !s.trim().is_empty()).count();
            if non_empty_count > 0 && non_empty_count < num_cols {
                end_row = end_row.max(candidate);
            }
        }
    }

    // Cap header depth to at most 3 rows to protect against malformed data
    end_row.min(header_start_row + 2).min(grid.len().saturating_sub(1))
}

/// Convert a single worksheet to CSV string with merged headers.
pub fn worksheet_to_csv(sheet: &ExcelSheetData) -> Option<String> {
    if sheet.grid.is_empty() {
        return None;
    }

    // Determine the active column count of the table (ignoring trailing empty columns)
    let mut num_cols = 0usize;
    for r in 1..sheet.grid.len() {
        if let Some(row) = sheet.grid.get(r) {
            for (c, cell) in row.iter().enumerate() {
                if !cell.trim().is_empty() {
                    num_cols = num_cols.max(c + 1);
                }
            }
        }
    }
    // Also consider merge cells
    for &((sc, sr), (ec, _)) in &sheet.merges {
        if sr >= 1 {
            num_cols = num_cols.max(sc + 1).max(ec + 1);
        }
    }

    if num_cols == 0 {
        return None;
    }

    let header_start_row = 1;
    let header_end_row = detect_header_end_row(&sheet.grid, &sheet.merges, header_start_row, num_cols);
    let headers = build_merged_table_headers(&sheet.grid, &sheet.merges, header_start_row, header_end_row, num_cols);

    let mut csv_lines = Vec::new();
    csv_lines.push(headers.iter().map(|h| escape_csv_field(h)).collect::<Vec<_>>().join(","));

    // Emit data rows
    let data_start = header_end_row + 1;
    for r in data_start..sheet.grid.len() {
        let row = &sheet.grid[r];
        let has_any = (0..num_cols).any(|c| row.get(c).map(|s| !s.trim().is_empty()).unwrap_or(false));
        if !has_any {
            continue;
        }

        let line = (0..num_cols)
            .map(|c| {
                let cell_val = row.get(c).map(String::as_str).unwrap_or("").trim();
                escape_csv_field(cell_val)
            })
            .collect::<Vec<_>>()
            .join(",");
        csv_lines.push(line);
    }

    Some(csv_lines.join("\n"))
}

/// Convert multiple Excel sheets into 1 catalog CSV and individual table CSVs.
pub fn convert_excel_to_csv_data(sheets: &[ExcelSheetData], base_stem: &str) -> ExcelConversionResult {
    let mut catalog_rows = Vec::new();
    catalog_rows.push("type,number,title,file".to_string());

    let mut table_csvs = Vec::new();

    for sheet in sheets {
        let sheet_name = sheet.name.trim();
        let safe_name = sanitize_filename(sheet_name);

        // 1. Process first row for catalog
        let first_row = sheet.grid.first().cloned().unwrap_or_default();
        let (t_val, n_val, title_val, f_val) = parse_catalog_entry(&first_row, &safe_name);
        catalog_rows.push(format!(
            "{},{},{},{}",
            escape_csv_field(&t_val),
            escape_csv_field(&n_val),
            escape_csv_field(&title_val),
            escape_csv_field(&f_val)
        ));

        // 2. Process table data for individual CSV
        if let Some(table_csv) = worksheet_to_csv(sheet) {
            table_csvs.push((format!("{safe_name}.csv"), table_csv));
        }
    }

    ExcelConversionResult {
        catalog_csv_name: format!("{base_stem}_catalog.csv"),
        catalog_csv_content: catalog_rows.join("\n"),
        table_csvs,
    }
}

/// Parse all worksheets from an `.xlsx` file using `zip` and `quick-xml`.
pub fn parse_xlsx_sheets(path: &str) -> Result<Vec<ExcelSheetData>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open file '{path}': {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid XLSX archive '{path}': {e}"))?;

    // 1. Read xl/_rels/workbook.xml.rels to resolve relationship IDs
    let mut rels: HashMap<String, String> = HashMap::new();
    if let Ok(mut r_file) = archive.by_name("xl/_rels/workbook.xml.rels") {
        let mut xml = String::new();
        let _ = r_file.read_to_string(&mut xml);
        let mut reader = quick_xml::Reader::from_str(&xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Empty(ref e)) | Ok(quick_xml::events::Event::Start(ref e)) => {
                    if e.name().local_name().as_ref() == b"Relationship" {
                        let mut id = String::new();
                        let mut target = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"Id" {
                                id = String::from_utf8_lossy(&attr.value).to_string();
                            } else if attr.key.local_name().as_ref() == b"Target" {
                                target = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        if !id.is_empty() && !target.is_empty() {
                            let normalized = if target.starts_with('/') {
                                target.trim_start_matches('/').to_string()
                            } else if target.starts_with("xl/") {
                                target
                            } else {
                                format!("xl/{target}")
                            };
                            rels.insert(id, normalized);
                        }
                    }
                }
                Ok(quick_xml::events::Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    // 2. Read xl/workbook.xml to extract sheet list and targets
    let mut sheets_to_load: Vec<(String, String)> = Vec::new();
    if let Ok(mut wb_file) = archive.by_name("xl/workbook.xml") {
        let mut xml = String::new();
        let _ = wb_file.read_to_string(&mut xml);
        let mut reader = quick_xml::Reader::from_str(&xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Empty(ref e)) | Ok(quick_xml::events::Event::Start(ref e)) => {
                    if e.name().local_name().as_ref() == b"sheet" {
                        let mut name = String::new();
                        let mut r_id = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"name" {
                                name = String::from_utf8_lossy(&attr.value).to_string();
                            } else if attr.key.local_name().as_ref() == b"id" {
                                r_id = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        let target = rels.get(&r_id).cloned().unwrap_or_else(|| {
                            format!("xl/worksheets/sheet{}.xml", sheets_to_load.len() + 1)
                        });
                        if !name.is_empty() {
                            sheets_to_load.push((name, target));
                        }
                    }
                }
                Ok(quick_xml::events::Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    if sheets_to_load.is_empty() {
        return Err("No sheets found in workbook".into());
    }

    // 3. Read xl/sharedStrings.xml (shared string table)
    let mut shared_strings: Vec<String> = Vec::new();
    if let Ok(mut ss_file) = archive.by_name("xl/sharedStrings.xml") {
        let mut xml = String::new();
        let _ = ss_file.read_to_string(&mut xml);
        let mut reader = quick_xml::Reader::from_str(&xml);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let mut current_si = String::new();
        let mut inside_t = false;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    if e.name().local_name().as_ref() == b"si" {
                        current_si.clear();
                    } else if e.name().local_name().as_ref() == b"t" {
                        inside_t = true;
                    }
                }
                Ok(quick_xml::events::Event::Text(ref t)) => {
                    if inside_t {
                        if let Ok(txt) = t.decode() {
                            current_si.push_str(&txt);
                        }
                    }
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    if e.name().local_name().as_ref() == b"t" {
                        inside_t = false;
                    } else if e.name().local_name().as_ref() == b"si" {
                        shared_strings.push(std::mem::take(&mut current_si));
                    }
                }
                Ok(quick_xml::events::Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    // 4. Read each worksheet
    let mut result_sheets = Vec::new();
    for (sheet_name, target_path) in sheets_to_load {
        let Ok(mut ws_file) = archive.by_name(&target_path) else {
            continue;
        };
        let mut xml = String::new();
        let _ = ws_file.read_to_string(&mut xml);

        let mut merges = Vec::new();
        let mut cell_map: HashMap<(usize, usize), String> = HashMap::new();
        let mut max_row = 0usize;
        let mut max_col = 0usize;

        let mut reader = quick_xml::Reader::from_str(&xml);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();

        let mut cur_coord: Option<(usize, usize)> = None;
        let mut cur_type = String::new();
        let mut inside_v = false;
        let mut inside_inline_t = false;
        let mut cur_v = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Empty(ref e)) => {
                    if e.name().local_name().as_ref() == b"mergeCell" {
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"ref" {
                                let ref_str = String::from_utf8_lossy(&attr.value);
                                if let Some(range) = parse_range(&ref_str) {
                                    merges.push(range);
                                }
                            }
                        }
                    }
                }
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    match e.name().local_name().as_ref() {
                        b"mergeCell" => {
                            for attr in e.attributes().flatten() {
                                if attr.key.local_name().as_ref() == b"ref" {
                                    let ref_str = String::from_utf8_lossy(&attr.value);
                                    if let Some(range) = parse_range(&ref_str) {
                                        merges.push(range);
                                    }
                                }
                            }
                        }
                        b"c" => {
                            cur_v.clear();
                            cur_type.clear();
                            cur_coord = None;
                            for attr in e.attributes().flatten() {
                                if attr.key.local_name().as_ref() == b"r" {
                                    let r_val = String::from_utf8_lossy(&attr.value);
                                    cur_coord = parse_cell_ref(&r_val);
                                } else if attr.key.local_name().as_ref() == b"t" {
                                    cur_type = String::from_utf8_lossy(&attr.value).to_string();
                                }
                            }
                        }
                        b"v" => {
                            inside_v = true;
                            cur_v.clear();
                        }
                        b"t" if cur_coord.is_some() => {
                            inside_inline_t = true;
                        }
                        _ => {}
                    }
                }
                Ok(quick_xml::events::Event::Text(ref t)) => {
                    if inside_v || inside_inline_t {
                        if let Ok(txt) = t.decode() {
                            cur_v.push_str(&txt);
                        }
                    }
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    match e.name().local_name().as_ref() {
                        b"v" => {
                            inside_v = false;
                        }
                        b"t" => {
                            inside_inline_t = false;
                        }
                        b"c" => {
                            if let Some((c, r)) = cur_coord {
                                let val_str = if cur_type == "s" {
                                    let idx = cur_v.trim().parse::<usize>().unwrap_or(usize::MAX);
                                    shared_strings.get(idx).cloned().unwrap_or_default()
                                } else {
                                    cur_v.trim().to_string()
                                };
                                if !val_str.is_empty() {
                                    max_col = max_col.max(c);
                                    max_row = max_row.max(r);
                                    cell_map.insert((c, r), val_str);
                                }
                            }
                            cur_coord = None;
                            cur_v.clear();
                        }
                        _ => {}
                    }
                }
                Ok(quick_xml::events::Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        // Build 2D grid
        let num_rows = if cell_map.is_empty() { 0 } else { max_row + 1 };
        let num_cols = if cell_map.is_empty() { 0 } else { max_col + 1 };
        let mut grid = vec![vec![String::new(); num_cols]; num_rows];
        for ((c, r), val) in cell_map {
            if r < num_rows && c < num_cols {
                grid[r][c] = val;
            }
        }

        result_sheets.push(ExcelSheetData {
            name: sheet_name,
            grid,
            merges,
        });
    }

    Ok(result_sheets)
}

/// Result package from converting an XLSX file.
#[derive(Debug, Clone, Default)]
pub struct ConvertedExcelPackage {
    pub parent_dir: String,
    pub table_count: usize,
    pub catalog_path: String,
    pub table_paths: Vec<String>,
    pub zip_path: String,
    pub zip_name: String,
    pub zip_bytes: Vec<u8>,
    pub csv_files: Vec<(String, String)>,
}

/// Create a ZIP archive from a slice of (filename, file_content) entries.
pub fn create_csv_zip(files: &[(&str, &str)]) -> Result<Vec<u8>, String> {
    if files.is_empty() {
        return Err("No CSV files to include in zip archive".into());
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (filename, content) in files {
            z.start_file(*filename, options)
                .map_err(|e| format!("Failed to add '{filename}' to zip archive: {e}"))?;
            z.write_all(content.as_bytes())
                .map_err(|e| format!("Failed to write '{filename}' content to zip archive: {e}"))?;
        }
        z.finish()
            .map_err(|e| format!("Failed to finalize zip archive: {e}"))?;
    }
    Ok(buf.into_inner())
}

/// Execute conversion of an XLSX file into catalog CSV, table CSVs, and a combined ZIP archive saved in the same directory.
pub fn execute_convert_excel(path: &str) -> Result<ConvertedExcelPackage, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("File does not exist: {path}"));
    }

    let parent_dir = p.parent().unwrap_or(Path::new("."));
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("excel_data");

    let sheets = parse_xlsx_sheets(path)?;
    if sheets.is_empty() {
        return Err("No worksheets could be extracted from file".into());
    }

    let converted = convert_excel_to_csv_data(&sheets, stem);

    // 1. Write catalog CSV
    let catalog_path = parent_dir.join(&converted.catalog_csv_name);
    std::fs::write(&catalog_path, &converted.catalog_csv_content)
        .map_err(|e| format!("Failed to write catalog CSV '{}': {e}", catalog_path.display()))?;

    // 2. Write individual table CSVs
    let table_count = converted.table_csvs.len();
    let mut table_paths = Vec::with_capacity(table_count);
    for (tbl_name, tbl_content) in &converted.table_csvs {
        let tbl_path = parent_dir.join(tbl_name);
        std::fs::write(&tbl_path, tbl_content)
            .map_err(|e| format!("Failed to write table CSV '{}': {e}", tbl_path.display()))?;
        table_paths.push(tbl_path.to_string_lossy().to_string());
    }

    // 3. Assemble all generated CSVs and build ZIP archive
    let mut csv_files = Vec::with_capacity(1 + table_count);
    csv_files.push((converted.catalog_csv_name.clone(), converted.catalog_csv_content.clone()));
    for (tbl_name, tbl_content) in &converted.table_csvs {
        csv_files.push((tbl_name.clone(), tbl_content.clone()));
    }

    let files_to_zip: Vec<(&str, &str)> = csv_files.iter().map(|(n, c)| (n.as_str(), c.as_str())).collect();
    let zip_bytes = create_csv_zip(&files_to_zip)?;

    let zip_name = format!("{stem}_csvs.zip");
    let zip_path = parent_dir.join(&zip_name);
    std::fs::write(&zip_path, &zip_bytes)
        .map_err(|e| format!("Failed to write ZIP archive '{}': {e}", zip_path.display()))?;

    Ok(ConvertedExcelPackage {
        parent_dir: parent_dir.to_string_lossy().to_string(),
        table_count,
        catalog_path: catalog_path.to_string_lossy().to_string(),
        table_paths,
        zip_path: zip_path.to_string_lossy().to_string(),
        zip_name,
        zip_bytes,
        csv_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cell_ref_and_range() {
        assert_eq!(parse_cell_ref("A1"), Some((0, 0)));
        assert_eq!(parse_cell_ref("B2"), Some((1, 1)));
        assert_eq!(parse_cell_ref("C2"), Some((2, 1)));
        assert_eq!(parse_cell_ref("D2"), Some((3, 1)));
        assert_eq!(parse_cell_ref("AA1"), Some((26, 0)));
        assert_eq!(parse_cell_ref("AB5"), Some((27, 4)));

        assert_eq!(parse_range("B2:C2"), Some(((1, 1), (2, 1))));
        assert_eq!(parse_range("A2:A3"), Some(((0, 1), (0, 2))));
        assert_eq!(parse_range("D2:D3"), Some(((3, 1), (3, 2))));
    }

    #[test]
    fn test_parse_catalog_entry_from_first_row() {
        // User example from prompt:
        let row = vec![
            "Tabel 02.03 Jumlah Pegawai Negeri Sipil Pemerintah Kecamatan Karawang Timur Menurut Tingkat Pendidikan dan Jenis Kelamin, 2025".to_string()
        ];
        let (t, n, title, file) = parse_catalog_entry(&row, "02_03");
        assert_eq!(t, "Tabel");
        assert_eq!(n, "02.03");
        assert_eq!(
            title,
            "Jumlah Pegawai Negeri Sipil Pemerintah Kecamatan Karawang Timur Menurut Tingkat Pendidikan dan Jenis Kelamin, 2025"
        );
        assert_eq!(file, "02_03");

        // Cell split case
        let row_split = vec![
            "Tabel".into(),
            "01.01.".into(),
            "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into(),
        ];
        let (t2, n2, title2, file2) = parse_catalog_entry(&row_split, "01_01");
        assert_eq!(t2, "Tabel");
        assert_eq!(n2, "01.01.");
        assert_eq!(title2, "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006");
        assert_eq!(file2, "01_01");
    }

    #[test]
    fn test_build_merged_table_headers_with_double_dash() {
        // Reconstruct user's exact spreadsheet from attached image:
        // Row 1: Tingkat Pendidikan | Jenis Kelamin (merged B2:C2) | Jumlah
        // Row 2: (merged A2:A3)     | Laki-Laki    | Perempuan     | (merged D2:D3)
        let grid = vec![
            vec!["Tabel 02.03 Title".into()], // Row 0 (Title)
            vec![
                "Tingkat Pendidikan".into(),
                "Jenis Kelamin".into(),
                "".into(),
                "Jumlah".into(),
            ], // Row 1
            vec!["".into(), "Laki-Laki".into(), "Perempuan".into(), "".into()], // Row 2
            vec!["Sekolah Dasar (SD)".into(), "-".into(), "-".into(), "-".into()], // Row 3
        ];
        let merges = vec![
            ((0, 1), (0, 2)), // A2:A3
            ((1, 1), (2, 1)), // B2:C2
            ((3, 1), (3, 2)), // D2:D3
        ];

        let headers = build_merged_table_headers(&grid, &merges, 1, 2, 4);
        assert_eq!(headers.len(), 4);
        assert_eq!(headers[0], "Tingkat Pendidikan");
        assert_eq!(headers[1], "Jenis Kelamin--Laki-Laki");
        assert_eq!(headers[2], "Jenis Kelamin--Perempuan");
        assert_eq!(headers[3], "Jumlah");
    }

    #[test]
    fn test_convert_excel_to_csv_data_full_workflow() {
        let sheet1 = ExcelSheetData {
            name: "02_03".into(),
            grid: vec![
                vec!["Tabel 02.03 Jumlah Pegawai Negeri Sipil Pemerintah Kecamatan Karawang Timur Menurut Tingkat Pendidikan dan Jenis Kelamin, 2025".into()],
                vec!["Tingkat Pendidikan".into(), "Jenis Kelamin".into(), "".into(), "Jumlah".into()],
                vec!["".into(), "Laki-Laki".into(), "Perempuan".into(), "".into()],
                vec!["Sekolah Dasar (SD)".into(), "-".into(), "-".into(), "-".into()],
                vec!["SMP/Sederajat".into(), "1".into(), "-".into(), "1".into()],
                vec!["Jumlah".into(), "22".into(), "5".into(), "27".into()],
                vec!["Sumber: BPS Kabupaten Karawang / Dinas terkait".into(), "".into(), "".into(), "".into()],
            ],
            merges: vec![
                ((0, 1), (0, 2)),
                ((1, 1), (2, 1)),
                ((3, 1), (3, 2)),
            ],
        };

        let sheet2 = ExcelSheetData {
            name: "02_04".into(),
            grid: vec![
                vec!["Tabel 02.04 Nama dan Luas Desa Per Kecamatan di Kabupaten Karawang, 2026".into()],
                vec!["Kecamatan".into(), "Desa".into(), "Luas (km2)".into()],
                vec!["Karawang Timur".into(), "Adiarsa Timur".into(), "3.45".into()],
            ],
            merges: Vec::new(),
        };

        let res = convert_excel_to_csv_data(&[sheet1, sheet2], "KBDA_2026");
        assert_eq!(res.catalog_csv_name, "KBDA_2026_catalog.csv");

        // Verify catalog content (1 catalog row per sheet)
        let cat_lines: Vec<&str> = res.catalog_csv_content.lines().collect();
        assert_eq!(cat_lines.len(), 3); // Header + 2 sheets
        assert_eq!(cat_lines[0], "type,number,title,file");
        assert_eq!(
            cat_lines[1],
            "Tabel,02.03,\"Jumlah Pegawai Negeri Sipil Pemerintah Kecamatan Karawang Timur Menurut Tingkat Pendidikan dan Jenis Kelamin, 2025\",02_03"
        );
        assert_eq!(
            cat_lines[2],
            "Tabel,02.04,\"Nama dan Luas Desa Per Kecamatan di Kabupaten Karawang, 2026\",02_04"
        );

        // Verify 2 table CSVs generated
        assert_eq!(res.table_csvs.len(), 2);
        assert_eq!(res.table_csvs[0].0, "02_03.csv");
        let tbl1_lines: Vec<&str> = res.table_csvs[0].1.lines().collect();
        // Header with -- delimiter
        assert_eq!(tbl1_lines[0], "Tingkat Pendidikan,Jenis Kelamin--Laki-Laki,Jenis Kelamin--Perempuan,Jumlah");
        assert_eq!(tbl1_lines[1], "Sekolah Dasar (SD),-,-,-");
        assert_eq!(tbl1_lines[2], "SMP/Sederajat,1,-,1");
        assert_eq!(tbl1_lines[3], "Jumlah,22,5,27");
        assert_eq!(tbl1_lines[4], "Sumber: BPS Kabupaten Karawang / Dinas terkait,,,");

        assert_eq!(res.table_csvs[1].0, "02_04.csv");
        let tbl2_lines: Vec<&str> = res.table_csvs[1].1.lines().collect();
        assert_eq!(tbl2_lines[0], "Kecamatan,Desa,Luas (km2)");
        assert_eq!(tbl2_lines[1], "Karawang Timur,Adiarsa Timur,3.45");
    }

    #[test]
    fn test_parse_and_convert_synthetic_xlsx_file() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let xlsx_path = temp_dir.join("test_synth.xlsx");

        // Build a valid synthetic XLSX zip archive in memory and write to temp file
        {
            let file = std::fs::File::create(&xlsx_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            // xl/_rels/workbook.xml.rels
            zip.start_file("xl/_rels/workbook.xml.rels", options).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#).unwrap();

            // xl/workbook.xml
            zip.start_file("xl/workbook.xml", options).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="02_03" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#).unwrap();

            // xl/sharedStrings.xml
            zip.start_file("xl/sharedStrings.xml", options).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="8" uniqueCount="8">
  <si><t>Tabel 02.03 Jumlah Pegawai Negeri Sipil Pemerintah Kecamatan Karawang Timur Menurut Tingkat Pendidikan dan Jenis Kelamin, 2025</t></si>
  <si><t>Tingkat Pendidikan</t></si>
  <si><t>Jenis Kelamin</t></si>
  <si><t>Jumlah</t></si>
  <si><t>Laki-Laki</t></si>
  <si><t>Perempuan</t></si>
  <si><t>Sekolah Dasar (SD)</t></si>
  <si><t>Sumber: BPS Kabupaten Karawang / Dinas terkait</t></si>
</sst>"#).unwrap();

            // xl/worksheets/sheet1.xml
            zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="s"><v>0</v></c>
    </row>
    <row r="2">
      <c r="A2" t="s"><v>1</v></c>
      <c r="B2" t="s"><v>2</v></c>
      <c r="D2" t="s"><v>3</v></c>
    </row>
    <row r="3">
      <c r="B3" t="s"><v>4</v></c>
      <c r="C3" t="s"><v>5</v></c>
    </row>
    <row r="4">
      <c r="A4" t="s"><v>6</v></c>
      <c r="B4"><v>-</v></c>
      <c r="C4"><v>-</v></c>
      <c r="D4"><v>-</v></c>
    </row>
    <row r="5">
      <c r="A5" t="s"><v>7</v></c>
    </row>
  </sheetData>
  <mergeCells count="3">
    <mergeCell ref="A2:A3"/>
    <mergeCell ref="B2:C2"/>
    <mergeCell ref="D2:D3"/>
  </mergeCells>
</worksheet>"#).unwrap();

            zip.finish().unwrap();
        }

        // Test execution
        let res = execute_convert_excel(xlsx_path.to_str().unwrap());
        assert!(res.is_ok(), "Conversion should succeed: {res:?}");
        let pkg = res.unwrap();
        assert_eq!(pkg.table_count, 1);
        assert_eq!(pkg.table_paths.len(), 1);
        assert!(std::path::Path::new(&pkg.zip_path).exists(), "ZIP archive should be written to disk");
        assert!(!pkg.zip_bytes.is_empty(), "ZIP bytes should be non-empty");
        assert_eq!(pkg.csv_files.len(), 2, "Catalog + 1 table CSV");

        // Verify catalog file
        let catalog_content = std::fs::read_to_string(&pkg.catalog_path).unwrap();
        assert!(catalog_content.contains("type,number,title,file"));
        assert!(catalog_content.contains("Tabel,02.03,"));
        assert!(catalog_content.contains(",02_03"));

        // Verify table CSV
        let table_path = std::path::Path::new(&pkg.parent_dir).join("02_03.csv");
        assert!(table_path.exists());
        assert_eq!(pkg.table_paths[0], table_path.to_string_lossy().to_string());
        let table_content = std::fs::read_to_string(&table_path).unwrap();
        assert!(table_content.contains("Tingkat Pendidikan,Jenis Kelamin--Laki-Laki,Jenis Kelamin--Perempuan,Jumlah"));
        assert!(table_content.contains("Sekolah Dasar (SD),-,-,-"));
        assert!(table_content.contains("Sumber: BPS Kabupaten Karawang / Dinas terkait,,,"));

        // Verify ZIP contents
        let zip_file = std::fs::File::open(&pkg.zip_path).unwrap();
        let mut zip_reader = zip::ZipArchive::new(zip_file).unwrap();
        assert_eq!(zip_reader.len(), 2);
        assert!(zip_reader.by_name("test_synth_catalog.csv").is_ok());
        assert!(zip_reader.by_name("02_03.csv").is_ok());

        // Cleanup
        let _ = std::fs::remove_file(&xlsx_path);
        let _ = std::fs::remove_file(&pkg.catalog_path);
        let _ = std::fs::remove_file(&table_path);
        let _ = std::fs::remove_file(&pkg.zip_path);
    }

    #[test]
    fn test_parse_real_sample_bab_02_2025_if_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("plan").join("pubgen").join("samples").join("BAB_02_2025.xlsx"))
            .unwrap_or_default();
        if !path.exists() {
            return;
        }

        let sheets = parse_xlsx_sheets(path.to_str().unwrap());
        assert!(sheets.is_ok(), "Should parse sheets from BAB_02_2025.xlsx: {:?}", sheets);
        let s_list = sheets.unwrap();
        assert_eq!(s_list.len(), 3);

        let converted = convert_excel_to_csv_data(&s_list, "BAB_02_2025");
        assert_eq!(converted.table_csvs.len(), 3);
        assert_eq!(converted.table_csvs[0].0, "02_01.csv");
        assert_eq!(converted.table_csvs[1].0, "02_02.csv");
        assert_eq!(converted.table_csvs[2].0, "02_03.csv");
        for (tbl_name, tbl_csv) in &converted.table_csvs {
            assert!(tbl_name.ends_with(".csv"));
            assert!(!tbl_csv.is_empty());
        }
    }
}
