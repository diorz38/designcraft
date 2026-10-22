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

use super::locale;

/// Parsed worksheet data from an XLSX file.
#[derive(Debug, Clone, Default)]
pub struct ExcelSheetData {
    pub name: String,
    /// 2D cell values grid [row][col] (0-indexed)
    pub grid: Vec<Vec<String>>,
    /// Merged cell ranges: ((start_col, start_row), (end_col, end_row))
    pub merges: Vec<((usize, usize), (usize, usize))>,
}

/// Cover values read from the `COVER` sheet (key/value rows, e.g. `title1 | PRODUK ...`).
#[derive(Debug, Clone, Default)]
pub struct ExcelCoverData {
    pub style: String,
    pub level: String,
    pub title1: String,
    pub title2: String,
    pub title_en: String,
    pub year: String,
    pub volume: String,
    pub catalogue: String,
    pub code: String,
    pub agency: String,
    pub image_hero: String,
    pub image_landscape: String,
    pub logo: String,
}

/// Generated CSV data from conversion.
#[derive(Debug, Clone, Default)]
pub struct ExcelConversionResult {
    pub catalog_csv_name: String,
    pub catalog_csv_content: String,
    /// (filename, csv_content)
    pub table_csvs: Vec<(String, String)>,
    /// Values for the Publication Cover card, taken from the `COVER` sheet if present.
    pub cover: Option<ExcelCoverData>,
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
    if trimmed.is_empty() { "sheet".to_string() } else { trimmed.to_string() }
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
/// Format table number into standard 4-digit dot-separated string "XX.XX" (e.g. "01.01").
pub fn format_table_number(num_val: &str, sheet_name: &str) -> String {
    let n = num_val.trim().trim_matches(|c| c == ':' || c == ';' || c == '.');
    let split_char = if n.contains('.') {
        Some('.')
    } else if n.contains('_') {
        Some('_')
    } else if n.contains('-') {
        Some('-')
    } else {
        None
    };

    if let Some(ch) = split_char {
        let parts: Vec<&str> = n.split(ch).collect();
        if parts.len() == 2
            && let Ok(p1) = parts[0].parse::<u32>()
            && let Ok(p2) = parts[1].parse::<u32>()
        {
            return format!("{p1:02}.{p2:02}");
        }
    }

    let f = sheet_name.trim().strip_suffix(".csv").unwrap_or(sheet_name.trim());
    if f.contains('_') {
        let parts: Vec<&str> = f.split('_').collect();
        if parts.len() == 2
            && let Ok(p1) = parts[0].parse::<u32>()
            && let Ok(p2) = parts[1].parse::<u32>()
        {
            return format!("{p1:02}.{p2:02}");
        }
    }

    if !n.is_empty() {
        n.to_string()
    } else {
        sheet_name.to_string()
    }
}

/// Check if a sheet is named `XX_XX` (e.g. 01_01, 02_03, etc.).
pub fn is_table_sheet_name(sheet_name: &str) -> bool {
    let s = sheet_name.trim().strip_suffix(".csv").unwrap_or(sheet_name.trim());
    let parts: Vec<&str> = s.split('_').collect();
    if parts.len() == 2 && parts[0].chars().all(|c| c.is_ascii_digit()) && parts[1].chars().all(|c| c.is_ascii_digit()) {
        !parts[0].is_empty() && !parts[1].is_empty()
    } else {
        false
    }
}

/// Split bilingual table title and harmonize trailing year (e.g. ", 2025") across both lines:
/// Line 1: Indonesian text, [year]
/// Line 2: English text, [year]
pub fn split_and_harmonize_table_title(title: &str, title_en_opt: Option<&str>) -> (String, Option<String>) {
    let (id_raw, en_raw_opt) = if let Some((id, en)) = locale::split_pair(title) {
        (id.to_string(), Some(en.to_string()))
    } else if let Some(en) = title_en_opt.filter(|s| !s.trim().is_empty()) {
        (title.trim().to_string(), Some(en.trim().to_string()))
    } else {
        (title.trim().to_string(), None)
    };

    let Some(en_raw) = en_raw_opt else {
        return (id_raw, None);
    };

    let id_trimmed = id_raw.trim();
    let en_trimmed = en_raw.trim();

    let id_year = extract_trailing_year(id_trimmed);
    let en_year = extract_trailing_year(en_trimmed);

    let (final_id, final_en) = match (id_year, en_year) {
        (None, Some(yr)) => {
            let id_clean = id_trimmed.trim_end_matches(',').trim_end();
            (format!("{id_clean}, {yr}"), en_trimmed.to_string())
        }
        (Some(yr), None) => {
            let en_clean = en_trimmed.trim_end_matches(',').trim_end();
            (id_trimmed.to_string(), format!("{en_clean}, {yr}"))
        }
        _ => (id_trimmed.to_string(), en_trimmed.to_string()),
    };

    (final_id, Some(final_en))
}

fn extract_trailing_year(s: &str) -> Option<&str> {
    let trimmed = s.trim();
    if trimmed.len() >= 4 {
        let last_4 = &trimmed[trimmed.len() - 4..];
        if last_4.chars().all(|c| c.is_ascii_digit()) {
            return Some(last_4);
        }
    }
    None
}

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
        let is_type = ["tabel", "table", "grafik", "graph", "bagan", "chart", "bab", "chapter", "section", "peta", "map"]
            .iter()
            .any(|k| first_lower.starts_with(k));

        if is_type {
            let t = words[0].to_string();
            let n = words[1].trim_matches(|c| c == ':' || c == ';').to_string();
            let title = if let Some(pos) = full.find(words[1]) {
                let after = &full[pos + words[1].len()..];
                after.trim().to_string()
            } else {
                words[2..].join(" ")
            };
            return (t, n, title, sheet_name.into());
        }

        // What if word 0 is already a number/code (e.g. "02.03 Judul...")
        if words[0].chars().any(|c| c.is_ascii_digit()) {
            let n = words[0].trim_matches(|c| c == ':' || c == ';' || c == '.').to_string();
            let title = if let Some(pos) = full.find(words[0]) {
                let after = &full[pos + words[0].len()..];
                after.trim().to_string()
            } else {
                words[1..].join(" ")
            };
            return ("Tabel".into(), n, title, sheet_name.into());
        }
    }

    if words.len() == 2 {
        return ("Tabel".into(), words[0].into(), words[1].into(), sheet_name.into());
    }

    ("Tabel".into(), sheet_name.into(), full, sheet_name.into())
}

/// Check if a sheet is an RTF review sheet (named RXX where XX is 01-99).
pub fn is_rtf_review_sheet(sheet_name: &str) -> bool {
    let s = sheet_name.trim();
    if s.len() >= 2 && (s.starts_with('R') || s.starts_with('r')) {
        let rest = &s[1..];
        rest.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

/// Extract first two-digit number from a string, e.g. "BAB 01 Pemerintahan" -> "01".
pub fn extract_first_two_digit_number(text: &str) -> Option<String> {
    let mut num_str = String::new();
    let mut in_num = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            num_str.push(ch);
            in_num = true;
        } else if in_num {
            break;
        }
    }
    if num_str.is_empty() {
        None
    } else if let Ok(val) = num_str.parse::<u32>() {
        Some(format!("{val:02}"))
    } else {
        Some(num_str)
    }
}

/// Check if a sheet carries Publication Cover values (named `COVER`).
pub fn is_cover_sheet(sheet_name: &str) -> bool {
    sheet_name.trim().eq_ignore_ascii_case("cover")
}

/// Sheets that are lookup/legend lists (e.g. dropdown sources), never publication pages.
pub fn is_ignored_sheet(sheet_name: &str) -> bool {
    sheet_name.trim().eq_ignore_ascii_case("note")
}

/// Parse the `COVER` sheet: one `key | value` row per field (see `ExcelCoverData`).
pub fn parse_cover_sheet(sheet: &ExcelSheetData) -> ExcelCoverData {
    let mut cover = ExcelCoverData::default();
    for row in &sheet.grid {
        let Some(key) = row.first().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()) else {
            continue;
        };
        // A value may span several columns (e.g. volume = 13, year = 2026).
        let value = row[1..].iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
        if value.is_empty() {
            continue;
        }
        let norm: String = key.chars().filter(|c| !matches!(c, '_' | ' ' | '-')).collect();
        let slot = match norm.as_str() {
            "style" => &mut cover.style,
            "level" => &mut cover.level,
            "titleid" | "title1" | "title" => &mut cover.title1,
            "title2" => &mut cover.title2,
            "titleen" => &mut cover.title_en,
            "year" => &mut cover.year,
            "volume" => &mut cover.volume,
            "nocatalogue" | "catalogue" | "catalog" | "katalog" => &mut cover.catalogue,
            "issn" | "isbn" | "code" => &mut cover.code,
            "officename" | "agency" | "lembaga" => &mut cover.agency,
            "image1" | "imagehero" | "image" => &mut cover.image_hero,
            "imagelanscape" | "imagelandscape" | "landscape" => &mut cover.image_landscape,
            "officelogo" | "logo" => &mut cover.logo,
            _ => continue,
        };
        *slot = value;
    }
    cover
}

/// Keys recognized for custom page template or divider selection in Excel sheets.
pub const TEMPLATE_KEYS: &[&str] =
    &["divider", "dividers", "diivider", "diividers", "diivder", "template", "templates", "tpl", "pembatas", "layout", "style"];

fn key_matches(k: &str, expected_keys: &[&str]) -> bool {
    let k_norm: String = k.chars().filter(|c| !matches!(c, '_' | ' ' | '-')).collect();
    expected_keys.iter().any(|key| {
        let key_norm: String = key.chars().filter(|c| !matches!(c, '_' | ' ' | '-')).collect();
        k_norm.eq_ignore_ascii_case(&key_norm)
    })
}

/// Extract the value for any of the given expected keys from a row of cells.
pub fn extract_field_for_keys(row: &[String], expected_keys: &[&str]) -> Option<String> {
    for cell in row {
        let t = cell.trim();
        for sep in [';', ':', '='] {
            if let Some((k, v)) = t.split_once(sep)
                && key_matches(k.trim(), expected_keys)
            {
                let val = v.trim().trim_matches('"').trim().to_string();
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
    }
    let non_empty: Vec<&str> = row.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if non_empty.len() >= 2
        && key_matches(non_empty[0], expected_keys)
    {
        let val = non_empty[1].trim_matches('"').trim().to_string();
        if !val.is_empty() {
            return Some(val);
        }
    }
    if non_empty.len() == 1 {
        let cell = non_empty[0];
        if let Some((k, v)) = cell.split_once(|c: char| c.is_whitespace())
            && key_matches(k.trim(), expected_keys)
        {
            let val = v.trim().trim_matches('"').trim().to_string();
            if !val.is_empty() {
                return Some(val);
            }
        }
    }
    None
}

/// Search for an optional template / divider specifier across any row in the sheet grid.
pub fn extract_template_from_sheet(sheet: &ExcelSheetData) -> Option<String> {
    for row in &sheet.grid {
        if let Some(tpl) = extract_field_for_keys(row, TEMPLATE_KEYS) {
            return Some(tpl);
        }
    }
    None
}

/// Lenient field extraction for title/file rows with fallback.
pub fn extract_field_lenient(row: &[String], expected_key: &str) -> String {
    for cell in row {
        let t = cell.trim();
        for sep in [';', ':', '='] {
            if let Some((k, v)) = t.split_once(sep)
                && k.trim().eq_ignore_ascii_case(expected_key)
            {
                return v.trim().trim_matches('"').trim().to_string();
            }
        }
    }
    let non_empty: Vec<&str> = row.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if non_empty.len() >= 2 && non_empty[0].eq_ignore_ascii_case(expected_key) {
        return non_empty[1].trim_matches('"').trim().to_string();
    }
    for &c in &non_empty {
        if !c.eq_ignore_ascii_case(expected_key) {
            return c.trim_matches('"').trim().to_string();
        }
    }
    String::new()
}

/// Parse an RXX RTF review sheet with 2 or more rows:
/// Parse an RXX RTF review sheet with support for bilingual English RTF (`file_en`):
/// Row 1: title;"BAB 01 Pemerintahan"
/// Row 2: file;[path to file RTF]
/// Optional: file_en;[path to English RTF]
/// Optional: divider;[template path or ID] (or diivider, template, etc.)
pub fn parse_rtf_review_sheet_bilingual(
    sheet: &ExcelSheetData,
) -> Option<(String, String, String, String, Option<String>, Option<String>, Option<String>)> {
    let mut title_val = String::new();
    let mut title_en = None;
    let mut file_val = String::new();
    let mut file_en = None;

    for row in &sheet.grid {
        if title_val.is_empty()
            && let Some(t) = extract_field_for_keys(row, &["title_id", "titleid", "title", "judul"])
        {
            title_val = t;
        }
        if title_en.is_none()
            && let Some(ten) = extract_field_for_keys(row, &["title_en", "titleen", "judulen", "judul_en"])
        {
            title_en = Some(ten);
        }
        if file_val.is_empty()
            && let Some(f) = extract_field_for_keys(row, &["file_id", "fileid", "file", "rtf", "path"])
        {
            file_val = f;
        }
        if file_en.is_none()
            && let Some(fen) = extract_field_for_keys(row, &["file_en", "fileen", "rtf_en", "rtfen", "path_en", "pathen"])
        {
            file_en = Some(fen);
        }
    }

    if title_val.is_empty()
        && let Some(row1) = sheet.grid.first()
    {
        title_val = extract_field_lenient(row1, "title");
    }

    if file_val.is_empty()
        && let Some(row2) = sheet.grid.get(1)
    {
        file_val = extract_field_lenient(row2, "file");
    }

    // Split combined bilingual title like "Kata Pengantar / Preface"
    if let Some((id_part, en_part)) = locale::split_pair(&title_val) {
        if title_en.is_none() {
            title_en = Some(en_part.to_string());
        }
        title_val = id_part.to_string();
    }

    let num_val = extract_first_two_digit_number(&title_val).unwrap_or_else(|| {
        let digits: String = sheet.name.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() == 1 {
            format!("0{digits}")
        } else if !digits.is_empty() {
            digits
        } else {
            "01".to_string()
        }
    });

    let template_val = extract_template_from_sheet(sheet);

    Some(("Ulasan".to_string(), num_val, title_val, file_val, template_val, title_en, file_en))
}

/// Backward-compatible wrapper for `parse_rtf_review_sheet_bilingual` returning 5-tuple.
pub fn parse_rtf_review_sheet_with_template(sheet: &ExcelSheetData) -> Option<(String, String, String, String, Option<String>)> {
    let (t, n, title, file, tpl, _, _) = parse_rtf_review_sheet_bilingual(sheet)?;
    Some((t, n, title, file, tpl))
}

/// The `image1` / `image2` picture paths of a front-matter (`BXX`) sheet, read leniently from any
/// row that names them (`image1;path`, `image1 | path`, …). Empty when the sheet has neither.
pub fn parse_sheet_images(sheet: &ExcelSheetData) -> (Option<String>, Option<String>) {
    // Strict key match, unlike `extract_field_lenient`: its fallback would turn any other cell
    // into a picture path.
    let find = |key: &str| -> Option<String> {
        for row in &sheet.grid {
            for (i, cell) in row.iter().enumerate() {
                let t = cell.trim();
                for sep in [';', ':', '='] {
                    if let Some((k, v)) = t.split_once(sep)
                        && k.trim().eq_ignore_ascii_case(key)
                        && !v.trim().is_empty()
                    {
                        return Some(v.trim().trim_matches('"').trim().to_string());
                    }
                }
                if t.eq_ignore_ascii_case(key)
                    && let Some(v) = row.get(i + 1).map(|s| s.trim().trim_matches('"').trim()).filter(|s| !s.is_empty())
                {
                    return Some(v.to_string());
                }
            }
        }
        None
    };
    (find("image1"), find("image2"))
}

/// Backward-compatible wrapper for `parse_rtf_review_sheet_with_template` returning 4-tuple.
pub fn parse_rtf_review_sheet(sheet: &ExcelSheetData) -> Option<(String, String, String, String)> {
    let (t, n, title, file, _) = parse_rtf_review_sheet_with_template(sheet)?;
    Some((t, n, title, file))
}

/// Check if a sheet is a chart / graphic sheet (named CXX_XX or CXX where XX is digits, e.g. C02_01).
pub fn is_chart_sheet(sheet_name: &str) -> bool {
    let s = sheet_name.trim();
    if s.len() >= 2 && (s.starts_with('C') || s.starts_with('c')) {
        let rest = &s[1..];
        rest.chars().all(|c| c.is_ascii_digit() || c == '_') && rest.chars().any(|c| c.is_ascii_digit())
    } else {
        false
    }
}

/// Check if a sheet is a publication-note sheet (named `A02`, `A01..A99`, `A_01..A_99`, `KETERANGAN`, etc.)
/// or a narrative RTF sheet starting with `G` (e.g. `G00`, `G01`, `G02`, etc.).
pub fn is_note_sheet(sheet_name: &str) -> bool {
    let s = sheet_name.trim();
    if s.is_empty() {
        return false;
    }
    let lower = s.to_lowercase();
    if lower == "keterangan"
        || lower == "keterangan publikasi"
        || lower == "keterangan_publikasi"
        || lower == "catatan"
        || lower == "catatan publikasi"
        || lower == "pubnote"
        || lower == "publication_note"
    {
        return true;
    }
    if let Some(rest) = lower.strip_prefix('a') {
        let rest_clean = rest.trim_start_matches('_').trim();
        if !rest_clean.is_empty() && rest_clean.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    if lower.starts_with('g') {
        return true;
    }
    false
}

/// Check if a sheet is a narrative RTF sheet starting with `G` (e.g. `G00`, `G01`, `G02`, etc.).
pub fn is_g_sheet(sheet_name: &str) -> bool {
    let s = sheet_name.trim();
    if s.is_empty() {
        return false;
    }
    s.starts_with('G') || s.starts_with('g')
}

/// Check if a sheet is a narrative RTF chapter sheet named `BXX` (e.g. `B01`, `B02`): front
/// matter laid out like an `RXX` review chapter. Digits only after the `B`, so `BAB…` never matches.
pub fn is_b_sheet(sheet_name: &str) -> bool {
    let s = sheet_name.trim();
    s.len() >= 2 && (s.starts_with('B') || s.starts_with('b')) && s[1..].chars().all(|c| c.is_ascii_digit())
}

/// Parse a publication note sheet (e.g. `A02`, `A01`, `Keterangan Publikasi`) or G sheet (e.g. `G00`, `G01`, `G02`):
/// Can contain title, file/rtf, template in key-value format (or cells) in any order,
/// or a single row with `.rtf` file path.
///
/// When the sheet only has 1 row (`file`), `title` is left empty so that Page from CSV does not add a Page Header.
///
/// Catalog entry: type `Note`, number from the sheet name (A02 -> 02, G00 -> 00), title, RTF path, template.
pub fn parse_note_sheet_with_template(sheet: &ExcelSheetData) -> Option<(String, String, String, String, Option<String>)> {
    let mut title_val = String::new();
    let mut file_val = String::new();

    for row in &sheet.grid {
        if title_val.is_empty()
            && let Some(t) = extract_field_for_keys(row, &["title", "judul", "keterangan"])
        {
            title_val = t;
        }
        if file_val.is_empty()
            && let Some(f) = extract_field_for_keys(row, &["file", "rtf", "path", "document", "dokumen"])
        {
            file_val = f;
        }
    }

    if file_val.is_empty() {
        for row in &sheet.grid {
            for cell in row {
                let c = cell.trim().trim_matches('"');
                if c.to_lowercase().ends_with(".rtf") {
                    file_val = c.to_string();
                    break;
                }
            }
            if !file_val.is_empty() {
                break;
            }
        }
    }

    // A note/narrative RTF sheet must have an associated RTF or document file path.
    if file_val.is_empty() {
        return None;
    }

    let non_empty_rows: Vec<&Vec<String>> = sheet.grid.iter().filter(|row| row.iter().any(|c| !c.trim().is_empty())).collect();

    // If the sheet has more than 1 non-empty row and title_val wasn't explicitly found by key,
    // look for a non-file row to serve as title.
    if title_val.is_empty() && non_empty_rows.len() > 1 {
        for row in &non_empty_rows {
            let non_empty: Vec<&str> = row.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if non_empty.is_empty() {
                continue;
            }
            let first = non_empty[0];
            if !first.eq_ignore_ascii_case("file")
                && !first.eq_ignore_ascii_case("template")
                && !first.eq_ignore_ascii_case("divider")
                && !first.to_lowercase().ends_with(".rtf")
            {
                if non_empty.len() >= 2 && first.eq_ignore_ascii_case("title") {
                    title_val = non_empty[1].trim_matches('"').trim().to_string();
                } else if non_empty.len() == 1 {
                    title_val = first.trim_matches('"').trim().to_string();
                } else {
                    title_val = non_empty.join(" ");
                }
                break;
            }
        }
    }

    // When the sheet only has 1 row: `file`, `title_val` remains empty so that Page from CSV
    // does not add a Page Header and simply places the RTF to the page.

    let digits: String = sheet.name.chars().filter(|c| c.is_ascii_digit()).collect();
    let num_val = match digits.len() {
        0 => {
            if sheet.name.trim().to_lowercase().starts_with('g') {
                "00".to_string()
            } else {
                "02".to_string()
            }
        }
        1 => format!("0{digits}"),
        _ => digits,
    };

    let template_val = extract_template_from_sheet(sheet);

    Some(("Note".to_string(), num_val, title_val, file_val, template_val))
}

/// Backward-compatible wrapper for `parse_note_sheet_with_template` returning 4-tuple.
pub fn parse_note_sheet(sheet: &ExcelSheetData) -> Option<(String, String, String, String)> {
    let (t, n, title, file, _) = parse_note_sheet_with_template(sheet)?;
    Some((t, n, title, file))
}

/// Extract the first dotted number from a string, e.g. "Grafik 02.01 Panjang..." -> "02.01".
pub fn extract_dotted_number(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| c == ':' || c == ';' || c == ',');
        if clean.contains('.') && clean.chars().any(|c| c.is_ascii_digit()) {
            let num = clean.trim_matches('.');
            if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return Some(num.to_string());
            }
        }
    }
    None
}

/// Parse a CXX_XX chart/figure sheet with 2 or more rows:
/// Row 1: title;"Grafik 02.01 Panjang Jalan..." (or col 0: title, col 1: text)
/// Row 2: file;[path of image file] (or col 0: file, col 1: path)
/// Row 3 (optional): template;[template path or ID]
pub fn parse_chart_sheet_with_template(sheet: &ExcelSheetData) -> Option<(String, String, String, String, Option<String>)> {
    let row1 = sheet.grid.first()?;
    let row2 = sheet.grid.get(1);

    let title_val = extract_field_lenient(row1, "title");
    let file_val = row2.map(|r| extract_field_lenient(r, "file")).unwrap_or_default();

    let first_word = title_val.split_whitespace().next().unwrap_or("Grafik");
    let type_val = if ["grafik", "graph", "chart", "bagan", "diagram", "peta", "map"].iter().any(|k| first_word.eq_ignore_ascii_case(k)) {
        first_word.to_string()
    } else {
        "Grafik".to_string()
    };

    let num_val = extract_dotted_number(&title_val).unwrap_or_else(|| {
        let clean = sheet.name.trim();
        let stripped = clean.strip_prefix('C').or_else(|| clean.strip_prefix('c')).unwrap_or(clean);
        stripped.replace('_', ".")
    });

    let template_val = extract_template_from_sheet(sheet);

    Some((type_val, num_val, title_val, file_val, template_val))
}

/// Backward-compatible wrapper for `parse_chart_sheet_with_template` returning 4-tuple.
pub fn parse_chart_sheet(sheet: &ExcelSheetData) -> Option<(String, String, String, String)> {
    let (t, n, title, file, _) = parse_chart_sheet_with_template(sheet)?;
    Some((t, n, title, file))
}

/// Check if a sheet is a divider sheet: named `DIV`, `DIV1`, `DIV01`, `DIV_01`, `DIVIDER`, or `PEMBATAS` (case-insensitive).
pub fn is_divider_sheet(sheet_name: &str) -> bool {
    let s = sheet_name.trim();
    if s.is_empty() {
        return false;
    }
    let lower = s.to_lowercase();
    if lower == "div" || lower == "divider" || lower == "dividers" || lower == "pembatas" {
        return true;
    }
    if lower.starts_with("divider") || lower.starts_with("pembatas") {
        return true;
    }
    if let Some(rest) = lower.strip_prefix("div") {
        let rest_clean = rest.trim_start_matches(['_', '-', ' ', '.']);
        if rest_clean.is_empty() || rest_clean.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    false
}

/// Split chapter divider title into `(first_word, digit_number, rest_of_text)`.
/// E.g. "BAB 02 PEMERINTAHAN" -> ("BAB", "02", "PEMERINTAHAN")
///      "BAB 01 Geografi"      -> ("BAB", "01", "Geografi")
pub fn split_divider_title(title: &str) -> (String, String, String) {
    let words: Vec<&str> = title.split_whitespace().collect();
    if words.is_empty() {
        return (String::new(), String::new(), String::new());
    }
    let first_word = words[0].to_string();
    if words.len() == 1 {
        return (first_word, String::new(), String::new());
    }

    // Look for the first word among words[1..] that contains digits
    let mut num_idx = None;
    for (i, &w) in words.iter().enumerate().skip(1) {
        let digits: String = w.chars().filter(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            num_idx = Some((i, digits));
            break;
        }
    }

    if let Some((idx, digits)) = num_idx {
        let num = if digits.len() == 1 { format!("0{digits}") } else { digits };
        let rest_raw = words[idx + 1..].join(" ");
        let rest = rest_raw.trim_start_matches(|c: char| c == ':' || c == '-' || c == ';' || c == '.' || c.is_whitespace()).to_string();
        (first_word, num, rest)
    } else {
        let rest = words[1..].join(" ");
        (first_word, String::new(), rest)
    }
}

/// Resolve a divider template specification (e.g. "002" or "002_chapter_devider") to its template path.
pub fn resolve_divider_template(tpl_spec: &str) -> String {
    let raw = tpl_spec.trim().trim_matches('"').trim();
    if raw.is_empty() {
        return "assets/templates/dividers/002_chapter_devider.designcraft".to_string();
    }

    let is_numeric = raw.chars().all(|c| c.is_ascii_digit());
    if is_numeric {
        let n: u32 = raw.parse().unwrap_or(2);
        let prefix = format!("{n:03}");
        for base in ["assets/templates/dividers", "../../assets/templates/dividers", "../assets/templates/dividers"] {
            let dir = std::path::Path::new(base);
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if fname.starts_with(&prefix) && fname.ends_with(".designcraft") {
                        return format!("assets/templates/dividers/{fname}");
                    }
                }
            }
        }
        return format!("assets/templates/dividers/{prefix}_chapter_devider.designcraft");
    }

    if raw.contains('/') || raw.contains('\\') || raw.ends_with(".designcraft") {
        return raw.to_string();
    }
    format!("assets/templates/dividers/{raw}.designcraft")
}

/// Parse a `DIV` divider sheet with support for `title_en`:
/// Row 1: title;"BAB 02 PEMERINTAHAN" (or title_id / judul)
/// Optional: title_en;"CHAPTER 02 GOVERNMENT" (or judulen)
/// Row 2: image;"E:\path\to\infografik.png" (or file / photo)
/// Row 3: template;"002" (or divider / template)
pub fn parse_divider_sheet_bilingual(sheet: &ExcelSheetData) -> Option<(String, String, String, String, Option<String>, Option<String>)> {
    let mut title_val = String::new();
    let mut title_en_val = None;
    let mut image_val = String::new();

    for row in &sheet.grid {
        if title_val.is_empty()
            && let Some(t) = extract_field_for_keys(row, &["title_id", "titleid", "title", "judul"])
        {
            title_val = t;
        }
        if title_en_val.is_none()
            && let Some(ten) = extract_field_for_keys(row, &["title_en", "titleen", "judulen", "judul_en"])
        {
            title_en_val = Some(ten);
        }
        if image_val.is_empty()
            && let Some(img) = extract_field_for_keys(row, &["image", "file", "photo", "hero", "gambar"])
        {
            image_val = img;
        }
    }

    if image_val.is_empty() {
        for row in &sheet.grid {
            for cell in row {
                let c = cell.trim().trim_matches('"');
                let lower = c.to_lowercase();
                if lower.ends_with(".png")
                    || lower.ends_with(".jpg")
                    || lower.ends_with(".jpeg")
                    || lower.ends_with(".webp")
                    || lower.ends_with(".svg")
                    || lower.ends_with(".pdf")
                {
                    image_val = c.to_string();
                    break;
                }
            }
            if !image_val.is_empty() {
                break;
            }
        }
    }

    if title_val.is_empty()
        && let Some(first_row) = sheet.grid.first()
    {
        let non_empty: Vec<&str> = first_row.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if non_empty.len() == 1 {
            title_val = non_empty[0].to_string();
        } else if non_empty.len() >= 2
            && !non_empty[0].eq_ignore_ascii_case("image")
            && !non_empty[0].eq_ignore_ascii_case("template")
            && !non_empty[0].eq_ignore_ascii_case("divider")
        {
            title_val = non_empty[1].to_string();
        }
    }

    let (_, split_num, _) = split_divider_title(&title_val);
    let num_val = if !split_num.is_empty() {
        split_num
    } else {
        extract_first_two_digit_number(&title_val).unwrap_or_else(|| {
            let digits: String = sheet.name.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 1 {
                format!("0{digits}")
            } else if !digits.is_empty() {
                digits
            } else {
                "01".to_string()
            }
        })
    };

    let raw_tpl = extract_template_from_sheet(sheet);
    let template_val = Some(resolve_divider_template(raw_tpl.as_deref().unwrap_or("002")));

    Some(("Divider".to_string(), num_val, title_val, image_val, template_val, title_en_val))
}

/// Backward-compatible wrapper for `parse_divider_sheet_bilingual` returning 5-tuple.
pub fn parse_divider_sheet_with_template(sheet: &ExcelSheetData) -> Option<(String, String, String, String, Option<String>)> {
    let (t, n, title, img, tpl, _) = parse_divider_sheet_bilingual(sheet)?;
    Some((t, n, title, img, tpl))
}

/// Backward-compatible wrapper for `parse_divider_sheet_with_template` returning 4-tuple.
pub fn parse_divider_sheet(sheet: &ExcelSheetData) -> Option<(String, String, String, String)> {
    let (t, n, title, file, _) = parse_divider_sheet_with_template(sheet)?;
    Some((t, n, title, file))
}

/// Automatically reconstruct table column headers by propagating horizontal & vertical
/// cell merges across header rows and joining ancestor levels with `--`.
#[allow(clippy::needless_range_loop)]
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
            let is_vert_merge_continuation = merges.iter().any(|&((sc, sr), (ec, er))| sc <= c && c <= ec && sr < abs_row && er >= abs_row);

            if !val.is_empty() && !is_vert_merge_continuation && parts.last().map(|s| s.as_str()) != Some(val) {
                parts.push(val.to_string());
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
pub fn detect_header_end_row(grid: &[Vec<String>], merges: &[((usize, usize), (usize, usize))], header_start_row: usize, num_cols: usize) -> usize {
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
        let has_horizontal_merges_in_parent = merges.iter().any(|&((sc, sr), (ec, _))| sr == header_start_row && sc < ec);
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

struct CatalogEntryItem {
    entry_type: String,
    number: String,
    title: String,
    file: String,
    template: Option<String>,
    /// Front-matter sheets only: the two picture files of the page.
    image1: Option<String>,
    image2: Option<String>,
    title_en: Option<String>,
    file_en: Option<String>,
}

/// Convert multiple Excel sheets into 1 catalog CSV and individual table CSVs.
/// If any sheet defines a template/divider (e.g. in Row 3), the catalog emits a 5th `template` column.
/// If no sheet defines a template, the catalog preserves the standard 4-column format `type,number,title,file`.
/// When bilingual fields (`title_en` on DIV or `file_en` on RXX/BXX) are present, `title_en,file_en` columns are appended.
pub fn convert_excel_to_csv_data(sheets: &[ExcelSheetData], base_stem: &str) -> ExcelConversionResult {
    let mut entries = Vec::new();
    let mut table_csvs = Vec::new();
    let mut cover = None;

    for sheet in sheets {
        let sheet_name = sheet.name.trim();
        let safe_name = sanitize_filename(sheet_name);

        // The COVER sheet supplies Publication Cover values, not a catalog page or table.
        if is_cover_sheet(sheet_name) {
            cover = Some(parse_cover_sheet(sheet));
            continue;
        }
        if is_ignored_sheet(sheet_name) {
            continue;
        }

        // Check if sheet is a DIV divider sheet
        if is_divider_sheet(sheet_name)
            && let Some((t_val, n_val, title_val, f_val, tpl_val, ten_val)) = parse_divider_sheet_bilingual(sheet)
        {
            entries.push(CatalogEntryItem {
                entry_type: t_val,
                number: n_val,
                title: title_val,
                file: f_val,
                template: tpl_val,
                image1: None,
                image2: None,
                title_en: ten_val,
                file_en: None,
            });
            // DIV sheets are divider spreads, not tabular data
            continue;
        }

        // Check if sheet is an RXX review chapter or a BXX front-matter chapter: both are
        // narrative RTF sheets laid out the same way
        if (is_rtf_review_sheet(sheet_name) || is_b_sheet(sheet_name))
            && let Some((t_val, n_val, title_val, f_val, tpl_val, ten_val, fen_val)) = parse_rtf_review_sheet_bilingual(sheet)
        {
            let (image1, image2) = if is_b_sheet(sheet_name) { parse_sheet_images(sheet) } else { (None, None) };
            entries.push(CatalogEntryItem {
                entry_type: t_val,
                number: n_val,
                title: title_val,
                file: f_val,
                template: tpl_val,
                image1,
                image2,
                title_en: ten_val,
                file_en: fen_val,
            });
            // Narrative chapters (RTF), not tabular data
            continue;
        }

        // Check if sheet is a CXX_XX chart / figure sheet
        if is_chart_sheet(sheet_name)
            && let Some((t_val, n_val, title_val, f_val, tpl_val)) = parse_chart_sheet_with_template(sheet)
        {
            entries.push(CatalogEntryItem {
                entry_type: t_val,
                number: n_val,
                title: title_val,
                file: f_val,
                template: tpl_val,
                image1: None,
                image2: None,
                title_en: None,
                file_en: None,
            });
            // CXX sheets are graphic chart figures, not tabular data
            continue;
        }

        // Check if sheet is the A02 publication-note sheet (title + RTF path)
        if is_note_sheet(sheet_name)
            && let Some((t_val, n_val, title_val, f_val, tpl_val)) = parse_note_sheet_with_template(sheet)
        {
            entries.push(CatalogEntryItem {
                entry_type: t_val,
                number: n_val,
                title: title_val,
                file: f_val,
                template: tpl_val,
                image1: None,
                image2: None,
                title_en: None,
                file_en: None,
            });
            // The A02 sheet carries a narrative RTF note, not tabular data
            continue;
        }

        // 1. Process first row for catalog
        let first_row = sheet.grid.first().cloned().unwrap_or_default();
        let (t_val, n_val, title_val, f_val) = parse_catalog_entry(&first_row, &safe_name);
        let tpl_val = extract_template_from_sheet(sheet);
        entries.push(CatalogEntryItem {
            entry_type: t_val,
            number: n_val,
            title: title_val,
            file: f_val,
            template: tpl_val,
            image1: None,
            image2: None,
            title_en: None,
            file_en: None,
        });

        // 2. Process table data for individual CSV
        if let Some(table_csv) = worksheet_to_csv(sheet) {
            table_csvs.push((format!("{safe_name}.csv"), table_csv));
        }
    }

    let has_any_template = entries.iter().any(|e| e.template.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false));
    let has_any_image = entries.iter().any(|e| e.image1.is_some() || e.image2.is_some());
    let has_any_bilingual = entries.iter().any(|e| {
        e.title_en.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false) || e.file_en.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
    });

    if has_any_image {
        let mut rows = vec![match (has_any_template, has_any_bilingual) {
            (true, true) => "type,number,title,file,template,image1,image2,title_en,file_en".to_string(),
            (true, false) => "type,number,title,file,template,image1,image2".to_string(),
            (false, true) => "type,number,title,file,image1,image2,title_en,file_en".to_string(),
            (false, false) => "type,number,title,file,image1,image2".to_string(),
        }];
        for e in &entries {
            let base = format!(
                "{},{},{},{}",
                escape_csv_field(&e.entry_type),
                escape_csv_field(&e.number),
                escape_csv_field(&e.title),
                escape_csv_field(&e.file),
            );
            let base = if has_any_template { format!("{base},{}", escape_csv_field(e.template.as_deref().unwrap_or(""))) } else { base };
            let with_img =
                format!("{base},{},{}", escape_csv_field(e.image1.as_deref().unwrap_or("")), escape_csv_field(e.image2.as_deref().unwrap_or("")),);
            if has_any_bilingual {
                rows.push(format!(
                    "{with_img},{},{}",
                    escape_csv_field(e.title_en.as_deref().unwrap_or("")),
                    escape_csv_field(e.file_en.as_deref().unwrap_or("")),
                ));
            } else {
                rows.push(with_img);
            }
        }
        return ExcelConversionResult {
            catalog_csv_name: format!("{base_stem}_catalog.csv"),
            catalog_csv_content: rows.join("\n"),
            table_csvs,
            cover,
        };
    }

    let mut catalog_rows = Vec::new();
    match (has_any_template, has_any_bilingual) {
        (true, true) => catalog_rows.push("type,number,title,file,template,title_en,file_en".to_string()),
        (true, false) => catalog_rows.push("type,number,title,file,template".to_string()),
        (false, true) => catalog_rows.push("type,number,title,file,title_en,file_en".to_string()),
        (false, false) => catalog_rows.push("type,number,title,file".to_string()),
    }
    for e in &entries {
        let base = format!(
            "{},{},{},{}",
            escape_csv_field(&e.entry_type),
            escape_csv_field(&e.number),
            escape_csv_field(&e.title),
            escape_csv_field(&e.file),
        );
        let base = if has_any_template { format!("{base},{}", escape_csv_field(e.template.as_deref().unwrap_or(""))) } else { base };
        if has_any_bilingual {
            catalog_rows.push(format!(
                "{base},{},{}",
                escape_csv_field(e.title_en.as_deref().unwrap_or("")),
                escape_csv_field(e.file_en.as_deref().unwrap_or("")),
            ));
        } else {
            catalog_rows.push(base);
        }
    }

    ExcelConversionResult { catalog_csv_name: format!("{base_stem}_catalog.csv"), catalog_csv_content: catalog_rows.join("\n"), table_csvs, cover }
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
                        let target = rels.get(&r_id).cloned().unwrap_or_else(|| format!("xl/worksheets/sheet{}.xml", sheets_to_load.len() + 1));
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
                    if inside_t && let Ok(txt) = t.decode() {
                        current_si.push_str(&txt);
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
                Ok(quick_xml::events::Event::Start(ref e)) => match e.name().local_name().as_ref() {
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
                },
                Ok(quick_xml::events::Event::Text(ref t)) => {
                    if (inside_v || inside_inline_t)
                        && let Ok(txt) = t.decode()
                    {
                        cur_v.push_str(&txt);
                    }
                }
                Ok(quick_xml::events::Event::End(ref e)) => match e.name().local_name().as_ref() {
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
                },
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

        result_sheets.push(ExcelSheetData { name: sheet_name, grid, merges });
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
    pub cover: Option<ExcelCoverData>,
}

/// Create a ZIP archive from a slice of (filename, file_content) entries.
pub fn create_csv_zip(files: &[(&str, &str)]) -> Result<Vec<u8>, String> {
    if files.is_empty() {
        return Err("No CSV files to include in zip archive".into());
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (filename, content) in files {
            z.start_file(*filename, options).map_err(|e| format!("Failed to add '{filename}' to zip archive: {e}"))?;
            z.write_all(content.as_bytes()).map_err(|e| format!("Failed to write '{filename}' content to zip archive: {e}"))?;
        }
        z.finish().map_err(|e| format!("Failed to finalize zip archive: {e}"))?;
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
        std::fs::write(&tbl_path, tbl_content).map_err(|e| format!("Failed to write table CSV '{}': {e}", tbl_path.display()))?;
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
    std::fs::write(&zip_path, &zip_bytes).map_err(|e| format!("Failed to write ZIP archive '{}': {e}", zip_path.display()))?;

    Ok(ConvertedExcelPackage {
        parent_dir: parent_dir.to_string_lossy().to_string(),
        table_count,
        catalog_path: catalog_path.to_string_lossy().to_string(),
        table_paths,
        zip_path: zip_path.to_string_lossy().to_string(),
        zip_name,
        zip_bytes,
        csv_files,
        cover: converted.cover,
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
            "Tabel 02.03 Jumlah Pegawai Negeri Sipil Pemerintah Kecamatan Karawang Timur Menurut Tingkat Pendidikan dan Jenis Kelamin, 2025"
                .to_string(),
        ];
        let (t, n, title, file) = parse_catalog_entry(&row, "02_03");
        assert_eq!(t, "Tabel");
        assert_eq!(n, "02.03");
        assert_eq!(title, "Jumlah Pegawai Negeri Sipil Pemerintah Kecamatan Karawang Timur Menurut Tingkat Pendidikan dan Jenis Kelamin, 2025");
        assert_eq!(file, "02_03");

        // Cell split case
        let row_split = vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into()];
        let (t2, n2, title2, file2) = parse_catalog_entry(&row_split, "01_01");
        assert_eq!(t2, "Tabel");
        assert_eq!(n2, "01.01.");
        assert_eq!(title2, "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006");
        assert_eq!(file2, "01_01");
    }

    #[test]
    fn test_cover_sheet_feeds_cover_values_and_is_not_a_page() {
        let cover = ExcelSheetData {
            name: "COVER".into(),
            grid: vec![
                vec!["style".into(), "CHEVRON MEDALLION".into()],
                vec!["title1".into(), "PRODUK DOMESTIK REGIONAL BRUTO".into()],
                vec!["title2".into(), "KABUPATEN KARAWANG".into()],
                vec!["year".into(), "".into(), "2026".into()],
                vec!["volume".into(), "13".into(), "2026".into()],
                vec!["no_catalogue".into(), "1102001.3215113".into()],
                vec!["ISSN".into(), "2828-7436".into()],
                vec!["office_name".into(), "BADAN PUSAT STATISTIK".into()],
                vec!["image_1".into(), "F:\\samples\\images\\city_skyline.jpg".into()],
                vec!["image_lanscape".into(), "F:\\samples\\images\\city_landscape.jpg".into()],
                vec!["office_logo".into(), "F:\\samples\\images\\bps_logo.png".into()],
            ],
            merges: Vec::new(),
        };
        let table = ExcelSheetData {
            name: "02_01".into(),
            grid: vec![vec!["Tabel 02.01 Luas Wilayah".into()], vec!["Kecamatan".into(), "Luas".into()], vec!["Karawang".into(), "100".into()]],
            merges: Vec::new(),
        };

        assert!(is_cover_sheet("cover"));
        assert!(is_cover_sheet(" COVER "));
        assert!(!is_cover_sheet("Covers"));
        assert!(is_ignored_sheet("note"));
        assert!(is_ignored_sheet(" NOTE "));
        // The publication-note page sheet is A02, not `note`.
        assert!(!is_ignored_sheet("A02"));

        let res = convert_excel_to_csv_data(&[cover, table], "BAB");
        let c = res.cover.as_ref().expect("COVER sheet should yield cover values");
        assert_eq!(c.style, "CHEVRON MEDALLION");
        assert_eq!(c.title1, "PRODUK DOMESTIK REGIONAL BRUTO");
        assert_eq!(c.title2, "KABUPATEN KARAWANG");
        assert_eq!(c.year, "2026");
        assert_eq!(c.volume, "13 2026");
        assert_eq!(c.catalogue, "1102001.3215113");
        assert_eq!(c.code, "2828-7436");
        assert_eq!(c.agency, "BADAN PUSAT STATISTIK");
        assert_eq!(c.image_hero, "F:\\samples\\images\\city_skyline.jpg");
        assert_eq!(c.image_landscape, "F:\\samples\\images\\city_landscape.jpg");
        assert_eq!(c.logo, "F:\\samples\\images\\bps_logo.png");

        // COVER must not become a catalog entry or a table CSV.
        assert_eq!(res.catalog_csv_content.lines().count(), 2);
        assert!(res.catalog_csv_content.contains("Tabel,02.01,"));
        assert!(!res.catalog_csv_content.contains("COVER"));
        assert_eq!(res.table_csvs.len(), 1);
        assert_eq!(res.table_csvs[0].0, "02_01.csv");
    }

    /// `BXX` front-matter sheets become `Ulasan` chapters, laid out like `RXX`, and never a table.
    #[test]
    fn test_bxx_sheet_is_ulasan_like_rxx() {
        assert!(is_b_sheet("B01"));
        assert!(is_b_sheet("b02"));
        assert!(!is_b_sheet("BAB"));
        assert!(!is_b_sheet("B"));
        assert!(!is_b_sheet("R01"));
        assert!(!is_b_sheet("01_01"));

        let sheet = ExcelSheetData {
            name: "B01".into(),
            grid: vec![
                vec!["title".into(), "Kata Pengantar".into()],
                vec!["file".into(), "plan/pubgen/samples/B01.rtf".into()],
                vec!["image1".into(), "plan/pubgen/samples/B01_pict1.jpg".into()],
                vec!["image2".into(), "plan/pubgen/samples/B01_pict2.png".into()],
            ],
            merges: Vec::new(),
        };
        let res = convert_excel_to_csv_data(std::slice::from_ref(&sheet), "BAB_01_2025");
        assert!(res.catalog_csv_content.contains("Ulasan,01,Kata Pengantar"), "catalog: {}", res.catalog_csv_content);
        assert!(!res.catalog_csv_content.contains("Tabel,title"), "sheets are not tables: {}", res.catalog_csv_content);
        assert!(res.table_csvs.is_empty(), "a BXX sheet emits no table CSV: {:?}", res.table_csvs);
        // The page layout reads the picture paths from the catalog's `image1` / `image2` columns.
        let header = res.catalog_csv_content.lines().next().unwrap_or_default().to_string();
        assert_eq!(header, "type,number,title,file,image1,image2", "catalog: {}", res.catalog_csv_content);
        assert!(
            res.catalog_csv_content
                .contains("Ulasan,01,Kata Pengantar,plan/pubgen/samples/B01.rtf,plan/pubgen/samples/B01_pict1.jpg,plan/pubgen/samples/B01_pict2.png"),
            "catalog: {}",
            res.catalog_csv_content
        );
    }

    #[test]
    fn test_is_rtf_review_sheet_and_parse_rxx() {
        assert!(is_rtf_review_sheet("R01"));
        assert!(is_rtf_review_sheet("R02"));
        assert!(is_rtf_review_sheet("R1"));
        assert!(is_rtf_review_sheet("R99"));
        assert!(is_rtf_review_sheet("r05"));
        assert!(!is_rtf_review_sheet("02_01"));
        assert!(!is_rtf_review_sheet("Report"));
        assert!(!is_rtf_review_sheet("Table1"));

        assert_eq!(extract_first_two_digit_number("BAB 01 Pemerintahan"), Some("01".into()));
        assert_eq!(extract_first_two_digit_number("BAB 2 Keadaan Alam"), Some("02".into()));
        assert_eq!(extract_first_two_digit_number("BAB 12"), Some("12".into()));

        // Test with semicolon syntax: title;BAB 01 Pemerintahan and file;path/to/file.rtf
        let sheet1 = ExcelSheetData {
            name: "R01".into(),
            grid: vec![vec!["title;BAB 01 Pemerintahan".into()], vec!["file;plan/pubgen/samples/ulasan_bab_1.rtf".into()]],
            merges: Vec::new(),
        };
        let (t1, n1, title1, f1) = parse_rtf_review_sheet(&sheet1).unwrap();
        assert_eq!(t1, "Ulasan");
        assert_eq!(n1, "01");
        assert_eq!(title1, "BAB 01 Pemerintahan");
        assert_eq!(f1, "plan/pubgen/samples/ulasan_bab_1.rtf");

        // Test with multi-column syntax: col 0 = "title", col 1 = "BAB 02 Keadaan Alam"
        let sheet2 = ExcelSheetData {
            name: "R02".into(),
            grid: vec![vec!["title".into(), "BAB 02 Keadaan Alam".into()], vec!["file".into(), "ulasan_bab_2.rtf".into()]],
            merges: Vec::new(),
        };
        let (t2, n2, title2, f2) = parse_rtf_review_sheet(&sheet2).unwrap();
        assert_eq!(t2, "Ulasan");
        assert_eq!(n2, "02");
        assert_eq!(title2, "BAB 02 Keadaan Alam");
        assert_eq!(f2, "ulasan_bab_2.rtf");
    }

    #[test]
    fn test_is_chart_sheet_and_parse_cxx() {
        assert!(is_chart_sheet("C02_01"));
        assert!(is_chart_sheet("C01_02"));
        assert!(is_chart_sheet("c02_01"));
        assert!(is_chart_sheet("C02"));
        assert!(!is_chart_sheet("R02"));
        assert!(!is_chart_sheet("02_01"));
        assert!(!is_chart_sheet("Chart"));

        assert_eq!(extract_dotted_number("Grafik 02.01 Panjang Jalan"), Some("02.01".into()));
        assert_eq!(extract_dotted_number("Panjang Jalan 02.01"), Some("02.01".into()));
        assert_eq!(extract_dotted_number("Grafik 1.2"), Some("1.2".into()));
        assert_eq!(extract_dotted_number("No digits here"), None);

        // Test with multi-column syntax matching user's sample C02_01:
        // Row 1: col 0 = "title", col 1 = "Grafik 02.01 Panjang Jalan Kecamatan Karawang Timur 2026"
        // Row 2: col 0 = "file", col 1 = "plan/pubgen/samples/C02_01.png"
        let sheet1 = ExcelSheetData {
            name: "C02_01".into(),
            grid: vec![
                vec!["title".into(), "Grafik 02.01 Panjang Jalan Kecamatan Karawang Timur 2026".into()],
                vec!["file".into(), "plan/pubgen/samples/C02_01.png".into()],
            ],
            merges: Vec::new(),
        };
        let (t1, n1, title1, f1) = parse_chart_sheet(&sheet1).unwrap();
        assert_eq!(t1, "Grafik");
        assert_eq!(n1, "02.01");
        assert_eq!(title1, "Grafik 02.01 Panjang Jalan Kecamatan Karawang Timur 2026");
        assert_eq!(f1, "plan/pubgen/samples/C02_01.png");

        // Test with semicolon syntax:
        let sheet2 = ExcelSheetData {
            name: "C02_02".into(),
            grid: vec![vec!["title;Grafik 02.02 Produksi Padi".into()], vec!["file;C02_02.jpg".into()]],
            merges: Vec::new(),
        };
        let (t2, n2, title2, f2) = parse_chart_sheet(&sheet2).unwrap();
        assert_eq!(t2, "Grafik");
        assert_eq!(n2, "02.02");
        assert_eq!(title2, "Grafik 02.02 Produksi Padi");
        assert_eq!(f2, "C02_02.jpg");
    }

    #[test]
    fn test_is_note_sheet_and_parse_a02() {
        assert!(is_note_sheet("A02"));
        assert!(is_note_sheet("a02"));
        assert!(is_note_sheet(" A02 "));
        assert!(is_note_sheet("A01"));
        assert!(is_note_sheet("A03"));
        assert!(is_note_sheet("A2"));
        assert!(is_note_sheet("A_02"));
        assert!(is_note_sheet("keterangan"));
        assert!(is_note_sheet("Keterangan Publikasi"));
        assert!(is_note_sheet("G00"));
        assert!(is_note_sheet("G01"));
        assert!(is_note_sheet("G02"));
        assert!(is_note_sheet("g05"));
        assert!(is_note_sheet("G"));
        assert!(is_g_sheet("G00"));
        assert!(is_g_sheet("G01"));
        assert!(is_g_sheet("g_02"));
        assert!(!is_g_sheet("A02"));
        assert!(!is_g_sheet("R01"));
        assert!(!is_note_sheet("A"));
        assert!(!is_note_sheet("AB02"));
        assert!(!is_note_sheet("R02"));
        assert!(!is_note_sheet("02_01"));

        // Row 1: title | Keterangan Publikasi, Row 2: file | RTF path
        let sheet = ExcelSheetData {
            name: "A02".into(),
            grid: vec![vec!["title".into(), "Keterangan Publikasi".into()], vec!["file".into(), "plan/pubgen/samples/notes/A02.rtf".into()]],
            merges: Vec::new(),
        };
        let (t, n, title, file) = parse_note_sheet(&sheet).unwrap();
        assert_eq!(t, "Note");
        assert_eq!(n, "02");
        assert_eq!(title, "Keterangan Publikasi");
        assert_eq!(file, "plan/pubgen/samples/notes/A02.rtf");

        // Semicolon syntax inside one cell still parses; number comes from the sheet name.
        let sheet =
            ExcelSheetData { name: "A02".into(), grid: vec![vec!["title;Catatan Penting".into()], vec!["file;A02.rtf".into()]], merges: Vec::new() };
        let (t, n, title, file) = parse_note_sheet(&sheet).unwrap();
        assert_eq!((t.as_str(), n.as_str()), ("Note", "02"));
        assert_eq!((title.as_str(), file.as_str()), ("Catatan Penting", "A02.rtf"));

        // Single row with file path only: title is empty so no Page Header is added
        let sheet_single =
            ExcelSheetData { name: "A02".into(), grid: vec![vec!["file".into(), "E:\\plan\\pubgen\\samples\\A02.rtf".into()]], merges: Vec::new() };
        let (t, n, title, file) = parse_note_sheet(&sheet_single).unwrap();
        assert_eq!(t, "Note");
        assert_eq!(n, "02");
        assert_eq!(title, "");
        assert_eq!(file, "E:\\plan\\pubgen\\samples\\A02.rtf");

        // G-sheet single row with file path: title is empty, number is 00
        let sheet_g00 =
            ExcelSheetData { name: "G00".into(), grid: vec![vec!["file".into(), "plan/pubgen/samples/G00.rtf".into()]], merges: Vec::new() };
        let (tg, ng, title_g, file_g) = parse_note_sheet(&sheet_g00).unwrap();
        assert_eq!(tg, "Note");
        assert_eq!(ng, "00");
        assert_eq!(title_g, "");
        assert_eq!(file_g, "plan/pubgen/samples/G00.rtf");

        // G-sheet with title and file: title is preserved
        let sheet_g01 = ExcelSheetData {
            name: "G01".into(),
            grid: vec![vec!["title".into(), "Glosarium".into()], vec!["file".into(), "plan/pubgen/samples/G01.rtf".into()]],
            merges: Vec::new(),
        };
        let (tg1, ng1, title_g1, file_g1) = parse_note_sheet(&sheet_g01).unwrap();
        assert_eq!(tg1, "Note");
        assert_eq!(ng1, "01");
        assert_eq!(title_g1, "Glosarium");
        assert_eq!(file_g1, "plan/pubgen/samples/G01.rtf");
    }

    #[test]
    fn test_build_merged_table_headers_with_double_dash() {
        // Reconstruct user's exact spreadsheet from attached image:
        // Row 1: Tingkat Pendidikan | Jenis Kelamin (merged B2:C2) | Jumlah
        // Row 2: (merged A2:A3)     | Laki-Laki    | Perempuan     | (merged D2:D3)
        let grid = vec![
            vec!["Tabel 02.03 Title".into()],                                                      // Row 0 (Title)
            vec!["Tingkat Pendidikan".into(), "Jenis Kelamin".into(), "".into(), "Jumlah".into()], // Row 1
            vec!["".into(), "Laki-Laki".into(), "Perempuan".into(), "".into()],                    // Row 2
            vec!["Sekolah Dasar (SD)".into(), "-".into(), "-".into(), "-".into()],                 // Row 3
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
                vec![
                    "Tabel 02.03 Jumlah Pegawai Negeri Sipil Pemerintah Kecamatan Karawang Timur Menurut Tingkat Pendidikan dan Jenis Kelamin, 2025"
                        .into(),
                ],
                vec!["Tingkat Pendidikan".into(), "Jenis Kelamin".into(), "".into(), "Jumlah".into()],
                vec!["".into(), "Laki-Laki".into(), "Perempuan".into(), "".into()],
                vec!["Sekolah Dasar (SD)".into(), "-".into(), "-".into(), "-".into()],
                vec!["SMP/Sederajat".into(), "1".into(), "-".into(), "1".into()],
                vec!["Jumlah".into(), "22".into(), "5".into(), "27".into()],
                vec!["Sumber: BPS Kabupaten Karawang / Dinas terkait".into(), "".into(), "".into(), "".into()],
            ],
            merges: vec![((0, 1), (0, 2)), ((1, 1), (2, 1)), ((3, 1), (3, 2))],
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

        let sheet3 = ExcelSheetData {
            name: "A02".into(),
            grid: vec![vec!["title".into(), "Keterangan Publikasi".into()], vec!["file".into(), "notes/A02.rtf".into()]],
            merges: Vec::new(),
        };

        let res = convert_excel_to_csv_data(&[sheet1, sheet2, sheet3], "KBDA_2026");
        assert_eq!(res.catalog_csv_name, "KBDA_2026_catalog.csv");

        // Verify catalog content (1 catalog row per sheet)
        let cat_lines: Vec<&str> = res.catalog_csv_content.lines().collect();
        assert_eq!(cat_lines.len(), 4); // Header + 3 sheets
        assert_eq!(cat_lines[0], "type,number,title,file");
        assert_eq!(
            cat_lines[1],
            "Tabel,02.03,\"Jumlah Pegawai Negeri Sipil Pemerintah Kecamatan Karawang Timur Menurut Tingkat Pendidikan dan Jenis Kelamin, 2025\",02_03"
        );
        assert_eq!(cat_lines[2], "Tabel,02.04,\"Nama dan Luas Desa Per Kecamatan di Kabupaten Karawang, 2026\",02_04");
        assert_eq!(cat_lines[3], "Note,02,Keterangan Publikasi,notes/A02.rtf");

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
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#,
            )
            .unwrap();

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
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="8" uniqueCount="8">
  <si><t>Tabel 02.03 Jumlah Pegawai Negeri Sipil Pemerintah Kecamatan Karawang Timur Menurut Tingkat Pendidikan dan Jenis Kelamin, 2025</t></si>
  <si><t>Tingkat Pendidikan</t></si>
  <si><t>Jenis Kelamin</t></si>
  <si><t>Jumlah</t></si>
  <si><t>Laki-Laki</t></si>
  <si><t>Perempuan</t></si>
  <si><t>Sekolah Dasar (SD)</t></si>
  <si><t>Sumber: BPS Kabupaten Karawang / Dinas terkait</t></si>
</sst>"#,
            )
            .unwrap();

            // xl/worksheets/sheet1.xml
            zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
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
</worksheet>"#,
            )
            .unwrap();

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
        assert_eq!(s_list.len(), 7, "BAB_02_2025.xlsx has 7 sheets: DIV, R02, 02_01, 02_02, 02_03, C02_01, note");

        let converted = convert_excel_to_csv_data(&s_list, "BAB_02_2025");
        assert!(converted.cover.is_none(), "this workbook has no COVER sheet");
        // DIV (Divider), R02 (Ulasan) and C02_01 (Grafik) are not tables, and note is not a page,
        // so only the 3 tabular sheets produce table CSVs
        assert_eq!(converted.table_csvs.len(), 3);
        assert_eq!(converted.table_csvs[0].0, "02_01.csv");
        assert_eq!(converted.table_csvs[1].0, "02_02.csv");
        assert_eq!(converted.table_csvs[2].0, "02_03.csv");
        for (tbl_name, tbl_csv) in &converted.table_csvs {
            assert!(tbl_name.ends_with(".csv"));
            assert!(!tbl_csv.is_empty());
        }

        // Verify catalog content includes the Divider, Ulasan and Grafik rows
        assert!(converted.catalog_csv_content.contains("Divider,02,BAB 02 PEMERINTAHAN,"));
        assert!(converted.catalog_csv_content.contains("Ulasan,02,Bab 02  Pemerintahan,"));
        assert!(converted.catalog_csv_content.contains("Grafik,02.01,Grafik 02.01 Panjang Jalan Kecamatan Karawang Timur 2026,"));
        assert!(converted.catalog_csv_content.contains("Tabel,02.01,"));
    }

    #[test]
    fn test_parse_rtf_with_row3_divider_and_typos() {
        // Standard syntax: divider;dividers/special_infographic_divider
        let sheet1 = ExcelSheetData {
            name: "R01".into(),
            grid: vec![
                vec!["title;BAB 01 Pemerintahan".into()],
                vec!["file;ulasan_bab_1.rtf".into()],
                vec!["divider;dividers/special_infographic_divider".into()],
            ],
            merges: Vec::new(),
        };
        let (t1, n1, title1, f1, tpl1) = parse_rtf_review_sheet_with_template(&sheet1).unwrap();
        assert_eq!(t1, "Ulasan");
        assert_eq!(n1, "01");
        assert_eq!(title1, "BAB 01 Pemerintahan");
        assert_eq!(f1, "ulasan_bab_1.rtf");
        assert_eq!(tpl1, Some("dividers/special_infographic_divider".into()));

        // Typo syntax with two i's (diivider) and multi-column layout
        let sheet2 = ExcelSheetData {
            name: "R02".into(),
            grid: vec![
                vec!["title".into(), "BAB 02 Sosial".into()],
                vec!["file".into(), "E:\\path\\to\\ulasan_2.rtf".into()],
                vec!["diivider".into(), "dividers/001_ulasan_chapter".into()],
            ],
            merges: Vec::new(),
        };
        let (t2, n2, title2, f2, tpl2) = parse_rtf_review_sheet_with_template(&sheet2).unwrap();
        assert_eq!(t2, "Ulasan");
        assert_eq!(n2, "02");
        assert_eq!(title2, "BAB 02 Sosial");
        assert_eq!(f2, "E:\\path\\to\\ulasan_2.rtf");
        assert_eq!(tpl2, Some("dividers/001_ulasan_chapter".into()));

        // Check backward compatibility wrapper parse_rtf_review_sheet
        let (bt, bn, btitle, bf) = parse_rtf_review_sheet(&sheet2).unwrap();
        assert_eq!(bt, "Ulasan");
        assert_eq!(bn, "02");
        assert_eq!(btitle, "BAB 02 Sosial");
        assert_eq!(bf, "E:\\path\\to\\ulasan_2.rtf");
    }

    #[test]
    fn test_convert_excel_to_csv_data_with_row3_template() {
        let sheet_r01 = ExcelSheetData {
            name: "R01".into(),
            grid: vec![
                vec!["title".into(), "BAB 01 Pemerintahan".into()],
                vec!["file".into(), "ulasan_bab_1.rtf".into()],
                vec!["diivider".into(), "dividers/special_infographic_divider".into()],
            ],
            merges: Vec::new(),
        };

        let sheet_tbl = ExcelSheetData {
            name: "01_01".into(),
            grid: vec![
                vec!["Tabel 01.01 Luas Wilayah Menurut Kecamatan, 2026".into()],
                vec!["Kecamatan".into(), "Luas".into()],
                vec!["Karawang".into(), "100".into()],
            ],
            merges: Vec::new(),
        };

        let res = convert_excel_to_csv_data(&[sheet_r01, sheet_tbl], "TEST_CONVERT");
        let lines: Vec<&str> = res.catalog_csv_content.lines().collect();

        // Must output 5-column header because sheet_r01 specified a template!
        assert_eq!(lines[0], "type,number,title,file,template");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], "Ulasan,01,BAB 01 Pemerintahan,ulasan_bab_1.rtf,dividers/special_infographic_divider");
        assert_eq!(lines[2], "Tabel,01.01,\"Luas Wilayah Menurut Kecamatan, 2026\",01_01,");
    }

    #[test]
    fn test_is_divider_sheet_and_split_title() {
        assert!(is_divider_sheet("DIV"));
        assert!(is_divider_sheet("div"));
        assert!(is_divider_sheet("DIV1"));
        assert!(is_divider_sheet("DIV01"));
        assert!(is_divider_sheet("DIV 1"));
        assert!(is_divider_sheet("DIV 01"));
        assert!(is_divider_sheet("DIV-1"));
        assert!(is_divider_sheet("DIV-01"));
        assert!(is_divider_sheet("DIV02"));
        assert!(is_divider_sheet("DIV_02"));
        assert!(is_divider_sheet("DIVIDER"));
        assert!(is_divider_sheet("pembatas"));
        assert!(!is_divider_sheet("COVER"));
        assert!(!is_divider_sheet("A02"));
        assert!(!is_divider_sheet("R02"));
        assert!(!is_divider_sheet("02_01"));

        let (first, num, rest) = split_divider_title("BAB 02 PEMERINTAHAN");
        assert_eq!(first, "BAB");
        assert_eq!(num, "02");
        assert_eq!(rest, "PEMERINTAHAN");

        let (first1, num1, rest1) = split_divider_title("BAB 01 Geografi");
        assert_eq!(first1, "BAB");
        assert_eq!(num1, "01");
        assert_eq!(rest1, "Geografi");

        let (first2, num2, rest2) = split_divider_title("BAB 2: PEMERINTAHAN DAERAH");
        assert_eq!(first2, "BAB");
        assert_eq!(num2, "02");
        assert_eq!(rest2, "PEMERINTAHAN DAERAH");
    }

    #[test]
    fn test_convert_excel_to_csv_data_with_div_sheet() {
        // As seen in user image #2:
        // Sheet DIV:
        // title | BAB 02 PEMERINTAHAN
        // image | E:\Github\designcraft\plan\pubgen\samples\infografik_01.png
        // template | 002
        let sheet_div = ExcelSheetData {
            name: "DIV".into(),
            grid: vec![
                vec!["title".into(), "BAB 02 PEMERINTAHAN".into()],
                vec!["image".into(), "E:\\Github\\designcraft\\plan\\pubgen\\samples\\infografik_01.png".into()],
                vec!["template".into(), "002".into()],
            ],
            merges: Vec::new(),
        };

        let sheet_tbl = ExcelSheetData {
            name: "02_01".into(),
            grid: vec![
                vec!["Tabel 02.01 Jumlah Penduduk".into()],
                vec!["Kecamatan".into(), "Jumlah".into()],
                vec!["Karawang".into(), "50000".into()],
            ],
            merges: Vec::new(),
        };

        let res = convert_excel_to_csv_data(&[sheet_div, sheet_tbl], "BAB_02");
        let lines: Vec<&str> = res.catalog_csv_content.lines().collect();

        assert_eq!(lines[0], "type,number,title,file,template");
        assert_eq!(lines.len(), 3);
        assert_eq!(
            lines[1],
            "Divider,02,BAB 02 PEMERINTAHAN,E:\\Github\\designcraft\\plan\\pubgen\\samples\\infografik_01.png,assets/templates/dividers/002_chapter_devider.designcraft"
        );
        assert_eq!(lines[2], "Tabel,02.01,Jumlah Penduduk,02_01,");

        // The DIV sheet must NOT produce a table CSV, only 02_01 does!
        assert_eq!(res.table_csvs.len(), 1);
        assert_eq!(res.table_csvs[0].0, "02_01.csv");
    }

    #[test]
    fn test_bilingual_excel_cover_and_catalog() {
        // Test COVER sheet with title_id, title2, and title_en
        let cover_sheet = ExcelSheetData {
            name: "COVER".into(),
            grid: vec![
                vec!["title_id".into(), "KECAMATAN KARAWANG TIMUR".into()],
                vec!["title2".into(), "DALAM ANGKA 2026".into()],
                vec!["title_en".into(), "KARAWANG TIMUR SUBDISTRICT IN FIGURES 2026".into()],
                vec!["author".into(), "BPS Kabupaten Karawang".into()],
            ],
            merges: Vec::new(),
        };
        let cover = parse_cover_sheet(&cover_sheet);
        assert_eq!(cover.title1, "KECAMATAN KARAWANG TIMUR");
        assert_eq!(cover.title2, "DALAM ANGKA 2026");
        assert_eq!(cover.title_en, "KARAWANG TIMUR SUBDISTRICT IN FIGURES 2026");

        // Test DIV with title_en and R01 with file_en
        let div_sheet = ExcelSheetData {
            name: "DIV".into(),
            grid: vec![
                vec!["title".into(), "BAB 02 PEMERINTAHAN".into()],
                vec!["title_en".into(), "CHAPTER 02 GOVERNMENT".into()],
                vec!["image".into(), "infografik_02.png".into()],
                vec!["template".into(), "002".into()],
            ],
            merges: Vec::new(),
        };
        let r01_sheet = ExcelSheetData {
            name: "R02".into(),
            grid: vec![
                vec!["title".into(), "BAB 02 PEMERINTAHAN".into()],
                vec!["file".into(), "ulasan_02_id.rtf".into()],
                vec!["file_en".into(), "ulasan_02_en.rtf".into()],
            ],
            merges: Vec::new(),
        };
        let tbl_sheet = ExcelSheetData {
            name: "02_01".into(),
            grid: vec![vec!["Tabel 02.01 Luas Wilayah / Area".into()], vec!["Kecamatan".into(), "Luas".into()]],
            merges: Vec::new(),
        };

        let res = convert_excel_to_csv_data(&[cover_sheet, div_sheet, r01_sheet, tbl_sheet], "BILINGUAL_RUN");
        assert!(res.cover.is_some());
        let lines: Vec<&str> = res.catalog_csv_content.lines().collect();
        // Since bilingual fields are present and DIV has a template, header has 7 columns:
        // type,number,title,file,template,title_en,file_en
        assert_eq!(lines[0], "type,number,title,file,template,title_en,file_en");
        assert_eq!(lines.len(), 4);
        assert!(lines[1].contains(
            "Divider,02,BAB 02 PEMERINTAHAN,infografik_02.png,assets/templates/dividers/002_chapter_devider.designcraft,CHAPTER 02 GOVERNMENT,"
        ));
        assert!(lines[2].contains("Ulasan,02,BAB 02 PEMERINTAHAN,ulasan_02_id.rtf,,,ulasan_02_en.rtf"));
        assert_eq!(lines[3], "Tabel,02.01,Luas Wilayah / Area,02_01,,,");
    }
}
