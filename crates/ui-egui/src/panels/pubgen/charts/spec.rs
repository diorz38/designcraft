//! `ChartSpec` parser and data extractor for `CXX_XX` Excel sheets.
//!
//! # Entry points
//!
//! - [`parse_chart_spec`] — reads the sheet's key-value rows and returns a typed [`ChartSpec`]
//!   when a `chart_type` row is present; returns `None` for raster-image sheets.
//! - [`extract_chart_data`] — reads the numeric data matrix (categories + series columns) from the
//!   same sheet using the row-range indices in the spec.

use crate::dock::pubgen::excel::ExcelSheetData;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// One data series in a chart: its source column, Indonesian label, and optional English label.
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesSpec {
    /// Column letter in the sheet (e.g. `"B"`), normalised to uppercase.
    pub col: String,
    /// Indonesian series label (e.g. `"Laki-laki"`).
    pub label_id: String,
    /// English series label. Empty string means "same as Indonesian" / not provided.
    pub label_en: String,
}

impl SeriesSpec {
    /// Formatted display label: `"Indonesian / English"` if bilingual, otherwise `"Indonesian"`.
    pub fn display_label(&self, bilingual: bool) -> String {
        if bilingual && !self.label_en.is_empty() && self.label_en != self.label_id {
            format!("{} / {}", self.label_id, self.label_en)
        } else {
            self.label_id.clone()
        }
    }
}

/// Supported chart archetypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartType {
    /// Vertical grouped bars — one cluster of N bars per category.
    BarClustered,
    /// Vertical stacked bars — series stacked on top of each other.
    BarStacked,
    /// Horizontal bars ranked by value — axes swapped.
    HorizontalBar,
    /// Line chart with markers; one polyline per series.
    Line,
    /// Area chart (filled region under a line).
    Area,
    /// Donut / ring chart for part-to-whole breakdowns.
    Donut,
}

impl ChartType {
    /// Parse a `chart_type` cell value (case-insensitive).
    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "barclustered" | "bar" | "column" | "batang" => Some(Self::BarClustered),
            "barstacked" | "stacked" | "stackedbar" | "stackedcolumn" => Some(Self::BarStacked),
            "horizontalbar" | "hbar" | "bar_h" | "batanghorizontal" | "ranked" => Some(Self::HorizontalBar),
            "line" | "garis" | "trend" => Some(Self::Line),
            "area" | "filled" | "arealine" => Some(Self::Area),
            "donut" | "doughnut" | "pie" | "donat" | "lingkaran" => Some(Self::Donut),
            _ => None,
        }
    }
}

/// How numeric data values are displayed in labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValueFormat {
    /// Plain number, formatted with Indonesian thousand-separator (dot) and decimal-comma.
    #[default]
    Number,
    /// Percentage — appends `%`, one decimal place.
    Percent,
    /// Indonesian Rupiah — prefix `Rp`, dot-separated thousands.
    CurrencyIdr,
}

impl ValueFormat {
    fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "percent" | "percentage" | "persen" | "%" => Self::Percent,
            "idr" | "rupiah" | "rp" | "currency" | "currency_idr" | "mata_uang" => Self::CurrencyIdr,
            _ => Self::Number,
        }
    }

    /// Format `v` as a display string for axis ticks and bar labels.
    pub fn format(&self, v: f64, bilingual: bool) -> String {
        match self {
            Self::Percent => format!("{:.1}%", v),
            Self::CurrencyIdr => {
                // Indonesian: "Rp 1.234.567"   English: "IDR 1,234,567"
                let abs = v.abs();
                let formatted = format_with_separator(abs, if bilingual { ',' } else { '.' });
                let prefix = if bilingual { "IDR" } else { "Rp" };
                if v < 0.0 { format!("-{} {}", prefix, formatted) } else { format!("{} {}", prefix, formatted) }
            }
            Self::Number => format_indonesian_number(v),
        }
    }
}

/// Format a number with the given thousands separator (no decimal places for whole numbers,
/// one decimal place otherwise).
fn format_with_separator(v: f64, sep: char) -> String {
    let rounded = v.round() as i64;
    let s = rounded.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(sep);
        }
        out.push(ch);
    }
    if v < 0.0 {
        out.push('-');
    }
    out.chars().rev().collect()
}

/// Format a number in Indonesian style: dot-separated thousands, comma decimal.
pub fn format_indonesian_number(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let abs = v.abs();
    // Show one decimal if the value has a meaningful fractional part.
    let has_frac = (abs - abs.floor()).abs() > 0.049;
    if has_frac {
        let int_part = abs.floor() as i64;
        let frac = ((abs - abs.floor()) * 10.0).round() as u8;
        let int_s = format_with_separator(int_part as f64, '.');
        let sign = if v < 0.0 { "-" } else { "" };
        format!("{}{},{}", sign, int_s, frac)
    } else {
        let sign = if v < 0.0 { "-" } else { "" };
        format!("{}{}", sign, format_with_separator(abs, '.'))
    }
}

/// Parse a numeric cell value, handling both Indonesian format (`1.234,56`) and
/// English format (`1,234.56`), plus parenthesised negatives and trailing `%` / `Rp`.
pub fn parse_cell_number(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() || matches!(s, "-" | "—" | "–" | "- " | "--" | "−" | "..." | "…" | "N/A" | "n/a" | "NA" | "na") {
        return None;
    }
    // Strip leading currency symbols and spaces.
    let mut clean = s.trim_start_matches(|c: char| c == '$' || c == '€' || c == '£').trim();
    if clean.starts_with("Rp") || clean.starts_with("rp") || clean.starts_with("RP") {
        clean = clean[2..].trim();
    } else if clean.starts_with("IDR") || clean.starts_with("idr") {
        clean = clean[3..].trim();
    }
    // Strip trailing %.
    clean = clean.trim_end_matches('%').trim();
    if clean.is_empty() {
        return None;
    }

    // Handle parenthesised negatives: (1.234) -> -1234
    let (negative, clean) = if clean.starts_with('(') && clean.ends_with(')') && clean.len() > 2 {
        (true, clean[1..clean.len() - 1].trim())
    } else {
        (false, clean)
    };

    let clean = clean.replace('−', "-");

    let has_dot = clean.contains('.');
    let has_comma = clean.contains(',');

    let val = if has_dot && has_comma {
        let last_dot = clean.rfind('.').unwrap();
        let last_comma = clean.rfind(',').unwrap();
        if last_dot < last_comma {
            // Indonesian / European: 1.234,56
            let eu = clean.replace('.', "").replace(',', ".");
            eu.parse::<f64>().ok()
        } else {
            // US / English: 1,234.56
            let us = clean.replace(',', "");
            us.parse::<f64>().ok()
        }
    } else if has_comma {
        if clean.matches(',').count() > 1 {
            let us = clean.replace(',', "");
            us.parse::<f64>().ok()
        } else {
            let parts: Vec<&str> = clean.split(',').collect();
            if parts.len() == 2 && parts[1].len() == 3 && !parts[0].starts_with('0') && parts[0].len() <= 3 {
                let us = clean.replace(',', "");
                us.parse::<f64>().ok()
            } else {
                let eu = clean.replace(',', ".");
                eu.parse::<f64>().ok()
            }
        }
    } else if has_dot {
        if clean.matches('.').count() > 1 {
            let id = clean.replace('.', "");
            id.parse::<f64>().ok()
        } else {
            let parts: Vec<&str> = clean.split('.').collect();
            if parts.len() == 2 && parts[1].len() == 3 && !parts[0].starts_with('0') && parts[0].len() <= 3 {
                let id = clean.replace('.', "");
                id.parse::<f64>().ok()
            } else {
                clean.parse::<f64>().ok()
            }
        }
    } else {
        clean.parse::<f64>().ok()
    };

    val.map(|v| if negative { -v } else { v })
}

/// A fully-parsed chart specification, ready for coordinate mapping and vector emission.
#[derive(Debug, Clone)]
pub struct ChartSpec {
    pub chart_type: ChartType,
    /// Indonesian title (may contain bilingual " / " separator).
    pub title_id: String,
    /// Separate English title (empty = derive from `title_id` or leave monolingual).
    pub title_en: String,
    /// Column letter for category labels (X-axis for bar/line, segment labels for donut).
    pub category_col: String,
    /// Data series (1–3).
    pub series: Vec<SeriesSpec>,
    /// First data row index, 1-based (matches the Excel sheet row number visible to the user).
    pub data_start_row: usize,
    /// Last data row index, 1-based, inclusive.
    pub data_end_row: usize,
    pub value_format: ValueFormat,
    pub show_value_labels: bool,
    pub show_legend: bool,
    pub show_grid: bool,
    /// When `Some`, overrides the automatically-computed Y-axis ceiling.
    pub max_value_override: Option<f64>,
}

// ---------------------------------------------------------------------------
// Key constants
// ---------------------------------------------------------------------------

const CHART_TYPE_KEYS: &[&str] = &["chart_type", "charttype", "chart", "type_chart", "jenis_grafik"];
const TITLE_KEYS: &[&str] = &["title", "judul", "title_id", "titleid"];
const TITLE_EN_KEYS: &[&str] = &["title_en", "titleen", "judul_en", "judulen", "english_title"];
const CATEGORY_COL_KEYS: &[&str] = &["category_col", "categorycol", "cat_col", "label_col", "kolom_kategori", "x_col"];
const DATA_START_KEYS: &[&str] = &["data_start_row", "datastartrow", "start_row", "row_start", "baris_awal"];
const DATA_END_KEYS: &[&str] = &["data_end_row", "dataendrow", "end_row", "row_end", "baris_akhir"];
const VALUE_FORMAT_KEYS: &[&str] = &["value_format", "valueformat", "format", "number_format"];
const SHOW_VALUE_LABELS_KEYS: &[&str] = &["show_value_labels", "showvaluelabels", "value_labels", "labels"];
const SHOW_LEGEND_KEYS: &[&str] = &["show_legend", "showlegend", "legend"];
const SHOW_GRID_KEYS: &[&str] = &["show_grid", "showgrid", "grid", "gridlines"];
const MAX_VALUE_KEYS: &[&str] = &["max_value", "maxvalue", "y_max", "ymax", "ceiling"];

/// Series column/label keys for series index `n` (0-based): returns (col_keys, label_id_keys, label_en_keys).
fn series_keys(n: usize) -> (&'static [&'static str], &'static [&'static str], &'static [&'static str]) {
    match n {
        0 => (
            &["series_1_col", "series1col", "s1_col", "col_1", "data_col", "value_col"],
            &["series_1_label", "series1label", "s1_label", "label_1", "series_1", "series1"],
            &["series_1_label_en", "series1labelen", "s1_label_en", "label_1_en", "label1en"],
        ),
        1 => (
            &["series_2_col", "series2col", "s2_col", "col_2"],
            &["series_2_label", "series2label", "s2_label", "label_2", "series_2", "series2"],
            &["series_2_label_en", "series2labelen", "s2_label_en", "label_2_en", "label2en"],
        ),
        2 => (
            &["series_3_col", "series3col", "s3_col", "col_3"],
            &["series_3_label", "series3label", "s3_label", "label_3", "series_3", "series3"],
            &["series_3_label_en", "series3labelen", "s3_label_en", "label_3_en", "label3en"],
        ),
        _ => (&[], &[], &[]),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a `CXX_XX` sheet into a [`ChartSpec`] when the sheet carries a `chart_type` row.
///
/// Returns `None` (not an error) when:
/// - No `chart_type` row is found → raster-image fallback.
/// - `chart_type` is present but required fields are missing → logs a warning, raster fallback.
///
/// This function never panics.
pub fn parse_chart_spec(sheet: &ExcelSheetData) -> Option<ChartSpec> {
    // Build a flat map of all key-value pairs in the sheet (scan every row).
    let kv = collect_kv(sheet);

    // `chart_type` must be present for vector rendering.
    let chart_type_raw = lookup(&kv, CHART_TYPE_KEYS)?;
    let chart_type = match ChartType::parse(&chart_type_raw) {
        Some(ct) => ct,
        None => {
            log::warn!("charts: unrecognised chart_type value '{}' in sheet '{}'; falling back to raster image", chart_type_raw, sheet.name);
            return None;
        }
    };

    let title_id = lookup(&kv, TITLE_KEYS).unwrap_or_default();
    let title_en = lookup(&kv, TITLE_EN_KEYS).unwrap_or_default();

    // category_col is required.
    let category_col = match lookup(&kv, CATEGORY_COL_KEYS) {
        Some(c) => c.trim().to_uppercase(),
        None => {
            log::warn!("charts: missing category_col in sheet '{}'; falling back to raster image", sheet.name);
            return None;
        }
    };

    // data_start_row / data_end_row are required.
    let data_start_row = match lookup(&kv, DATA_START_KEYS).and_then(|s| s.trim().parse::<usize>().ok()) {
        Some(r) if r >= 1 => r,
        _ => {
            log::warn!("charts: missing or invalid data_start_row in sheet '{}'; falling back to raster image", sheet.name);
            return None;
        }
    };
    let data_end_row = match lookup(&kv, DATA_END_KEYS).and_then(|s| s.trim().parse::<usize>().ok()) {
        Some(r) if r >= data_start_row => r,
        _ => {
            log::warn!("charts: missing or invalid data_end_row in sheet '{}'; falling back to raster image", sheet.name);
            return None;
        }
    };

    // Parse series (at least one required).
    let mut series = Vec::new();
    for n in 0..3 {
        let (col_keys, label_id_keys, label_en_keys) = series_keys(n);
        let Some(col) = lookup(&kv, col_keys) else { break };
        let col = col.trim().to_uppercase();
        if col.is_empty() {
            break;
        }
        let label_id = lookup(&kv, label_id_keys).unwrap_or_else(|| format!("Series {}", n + 1));
        let label_en = lookup(&kv, label_en_keys).unwrap_or_default();
        series.push(SeriesSpec { col, label_id, label_en });
    }
    if series.is_empty() {
        log::warn!("charts: no series_1_col found in sheet '{}'; falling back to raster image", sheet.name);
        return None;
    }

    // Optional fields with defaults.
    let value_format = lookup(&kv, VALUE_FORMAT_KEYS).map(|s| ValueFormat::parse(&s)).unwrap_or_default();
    let show_value_labels = parse_bool(lookup(&kv, SHOW_VALUE_LABELS_KEYS).as_deref(), true);
    let show_legend = parse_bool(lookup(&kv, SHOW_LEGEND_KEYS).as_deref(), true);
    let show_grid = parse_bool(lookup(&kv, SHOW_GRID_KEYS).as_deref(), true);
    let max_value_override = lookup(&kv, MAX_VALUE_KEYS).and_then(|s| s.trim().parse::<f64>().ok()).filter(|&v| v > 0.0);

    Some(ChartSpec {
        chart_type,
        title_id,
        title_en,
        category_col,
        series,
        data_start_row,
        data_end_row,
        value_format,
        show_value_labels,
        show_legend,
        show_grid,
        max_value_override,
    })
}

/// Extract numeric data from `sheet` using the row/column indices in `spec`.
///
/// Returns `(categories, series_values)` where both `Vec`s have the same length.
/// - `categories[i]` — the label string from the category column for row `i`.
/// - `series_values[s][i]` — the numeric value for series `s`, row `i` (0.0 when unparseable).
///
/// Returns `Err(String)` when no data rows could be parsed (the caller surfaces this as a
/// generation warning and falls back to raster).
pub fn extract_chart_data(
    sheet: &ExcelSheetData,
    spec: &ChartSpec,
) -> Result<(Vec<String>, Vec<Vec<f64>>), String> {
    // Convert column letters to 0-based indices.
    let cat_col_idx = col_letter_to_index(&spec.category_col)
        .ok_or_else(|| format!("invalid category_col '{}'", spec.category_col))?;

    let series_col_indices: Vec<usize> = spec
        .series
        .iter()
        .map(|s| col_letter_to_index(&s.col).ok_or_else(|| format!("invalid series col '{}'", s.col)))
        .collect::<Result<Vec<_>, _>>()?;

    // Convert 1-based spec row numbers to 0-based grid indices.
    let start_idx = spec.data_start_row.saturating_sub(1);
    let end_idx = spec.data_end_row.saturating_sub(1);

    if start_idx >= sheet.grid.len() {
        return Err(format!(
            "data_start_row {} exceeds sheet row count {}",
            spec.data_start_row,
            sheet.grid.len()
        ));
    }

    let mut categories: Vec<String> = Vec::new();
    let mut series_values: Vec<Vec<f64>> = vec![Vec::new(); spec.series.len()];

    for row_idx in start_idx..=end_idx.min(sheet.grid.len().saturating_sub(1)) {
        let row = &sheet.grid[row_idx];

        // Get category label; skip rows whose category cell is empty or a column header.
        let cat = row.get(cat_col_idx).map(|s| s.trim().trim_matches('"').trim().to_string()).unwrap_or_default();
        if cat.is_empty() {
            continue;
        }
        // Skip if the category cell looks like a column-header number (plain integer like "1", "(1)").
        if is_column_header(&cat) {
            continue;
        }

        // Parse values for each series; use 0.0 for unparseable cells.
        let mut row_has_value = false;
        for (s_idx, &col_idx) in series_col_indices.iter().enumerate() {
            let raw = row.get(col_idx).map(String::as_str).unwrap_or("");
            let v = parse_cell_number(raw).unwrap_or(0.0);
            if v != 0.0 {
                row_has_value = true;
            }
            series_values[s_idx].push(v);
        }

        // Skip rows where every series value is zero AND category looks numeric (total row).
        if !row_has_value && parse_cell_number(&cat).is_some() {
            // Remove the zero values we just pushed.
            for sv in series_values.iter_mut() {
                sv.pop();
            }
            continue;
        }

        categories.push(cat);
    }

    if categories.is_empty() {
        return Err(format!(
            "no data rows found in sheet '{}' between rows {} and {}",
            sheet.name, spec.data_start_row, spec.data_end_row
        ));
    }

    Ok((categories, series_values))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a flat `Vec<(key, value)>` from all key-value rows in the sheet grid.
/// Supports `key;value`, `key:value`, `key=value`, and two-cell `[key, value]` formats.
fn collect_kv(sheet: &ExcelSheetData) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for row in &sheet.grid {
        // Two-cell row: [key, value].
        let non_empty: Vec<&str> = row.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if non_empty.len() >= 2 {
            let k = non_empty[0].trim_end_matches([':', ';', '=']).trim().to_lowercase();
            let v = non_empty[1].trim_matches('"').trim().to_string();
            if !k.is_empty() && !v.is_empty() {
                out.push((k, v));
            }
        }
        // Inline `key;value` / `key:value` / `key=value` within one cell.
        for cell in row {
            let t = cell.trim();
            for sep in [';', ':', '='] {
                if let Some((k, v)) = t.split_once(sep) {
                    let k = k.trim().to_lowercase();
                    let v = v.trim().trim_matches('"').trim().to_string();
                    if !k.is_empty() && !v.is_empty() {
                        out.push((k, v));
                        break;
                    }
                }
            }
        }
    }
    out
}

/// Look up the first value matching any of `keys` (case-insensitive, ignoring `_`, `-`, ` `).
fn lookup(kv: &[(String, String)], keys: &[&str]) -> Option<String> {
    let normalised_keys: Vec<String> = keys
        .iter()
        .map(|k| k.chars().filter(|c| !matches!(c, '_' | '-' | ' ')).collect::<String>().to_lowercase())
        .collect();

    kv.iter().find_map(|(k, v)| {
        let kn: String = k.chars().filter(|c| !matches!(c, '_' | '-' | ' ')).collect();
        if normalised_keys.iter().any(|nk| nk == &kn) { Some(v.clone()) } else { None }
    })
}

/// Convert an Excel column letter to a 0-based index (`"A"` → 0, `"B"` → 1, `"AA"` → 26).
pub fn col_letter_to_index(col: &str) -> Option<usize> {
    let col = col.trim().to_uppercase();
    if col.is_empty() || !col.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let mut idx: usize = 0;
    for b in col.bytes() {
        idx = idx.saturating_mul(26).saturating_add((b - b'A' + 1) as usize);
    }
    Some(idx.saturating_sub(1))
}

/// Parse a boolean string (`"true"`, `"yes"`, `"1"`, `"false"`, `"no"`, `"0"`).
/// Returns `default` when `s` is `None` or unrecognised.
fn parse_bool(s: Option<&str>, default: bool) -> bool {
    match s.map(|s| s.trim().to_lowercase()).as_deref() {
        Some("true" | "yes" | "1" | "on" | "ya" | "aktif") => true,
        Some("false" | "no" | "0" | "off" | "tidak" | "nonaktif") => false,
        _ => default,
    }
}

/// Returns `true` when `s` looks like a column-header marker such as `"(1)"`, `"(2)"`, `"1"`.
fn is_column_header(s: &str) -> bool {
    let s = s.trim();
    // Parenthesised single digit: (1), (2) …
    if s.starts_with('(') && s.ends_with(')') {
        let inner = s[1..s.len() - 1].trim();
        if inner.len() <= 2 && inner.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    // Plain single digit: 1, 2, 3 (but not "2.34" or "1.000")
    if s.len() <= 2 && s.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: build an ExcelSheetData from a list of rows (each row is a list of &str).
    fn sheet(name: &str, rows: &[&[&str]]) -> ExcelSheetData {
        ExcelSheetData {
            name: name.to_string(),
            grid: rows.iter().map(|row| row.iter().map(|s| s.to_string()).collect()).collect(),
            merges: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // ChartType::parse
    // -----------------------------------------------------------------------

    #[test]
    fn chart_type_parses_all_aliases() {
        assert_eq!(ChartType::parse("bar_clustered"), Some(ChartType::BarClustered));
        assert_eq!(ChartType::parse("BAR"), Some(ChartType::BarClustered));
        assert_eq!(ChartType::parse("column"), Some(ChartType::BarClustered));
        assert_eq!(ChartType::parse("bar_stacked"), Some(ChartType::BarStacked));
        assert_eq!(ChartType::parse("stacked"), Some(ChartType::BarStacked));
        assert_eq!(ChartType::parse("horizontal_bar"), Some(ChartType::HorizontalBar));
        assert_eq!(ChartType::parse("hbar"), Some(ChartType::HorizontalBar));
        assert_eq!(ChartType::parse("line"), Some(ChartType::Line));
        assert_eq!(ChartType::parse("garis"), Some(ChartType::Line));
        assert_eq!(ChartType::parse("area"), Some(ChartType::Area));
        assert_eq!(ChartType::parse("donut"), Some(ChartType::Donut));
        assert_eq!(ChartType::parse("pie"), Some(ChartType::Donut));
        assert_eq!(ChartType::parse("donat"), Some(ChartType::Donut));
        assert_eq!(ChartType::parse("unknown_type"), None);
        assert_eq!(ChartType::parse(""), None);
    }

    // -----------------------------------------------------------------------
    // parse_cell_number
    // -----------------------------------------------------------------------

    #[test]
    fn parse_cell_number_handles_indonesian_format() {
        assert_eq!(parse_cell_number("1.234,56"), Some(1234.56));
        assert_eq!(parse_cell_number("12.345"), Some(12345.0)); // dot = thousands
        assert_eq!(parse_cell_number("0"), Some(0.0));
    }

    #[test]
    fn parse_cell_number_handles_english_format() {
        assert_eq!(parse_cell_number("1,234.56"), Some(1234.56));
        assert_eq!(parse_cell_number("1234"), Some(1234.0));
        assert_eq!(parse_cell_number("3.14"), Some(3.14));
    }

    #[test]
    fn parse_cell_number_handles_parenthesised_negatives() {
        assert_eq!(parse_cell_number("(1.234)"), Some(-1234.0));
        assert_eq!(parse_cell_number("(500)"), Some(-500.0));
    }

    #[test]
    fn parse_cell_number_handles_percent_and_currency() {
        assert_eq!(parse_cell_number("45%"), Some(45.0));
        assert_eq!(parse_cell_number("Rp 1.500"), Some(1500.0));
        assert_eq!(parse_cell_number("IDR 2,000"), Some(2000.0));
    }

    #[test]
    fn parse_cell_number_returns_none_for_dash_and_empty() {
        assert_eq!(parse_cell_number(""), None);
        assert_eq!(parse_cell_number("-"), None);
        assert_eq!(parse_cell_number("—"), None);
        assert_eq!(parse_cell_number("N/A"), None);
        assert_eq!(parse_cell_number("..."), None);
    }

    // -----------------------------------------------------------------------
    // col_letter_to_index
    // -----------------------------------------------------------------------

    #[test]
    fn col_letter_to_index_handles_single_and_multi_letter() {
        assert_eq!(col_letter_to_index("A"), Some(0));
        assert_eq!(col_letter_to_index("B"), Some(1));
        assert_eq!(col_letter_to_index("Z"), Some(25));
        assert_eq!(col_letter_to_index("AA"), Some(26));
        assert_eq!(col_letter_to_index("a"), Some(0)); // lowercase
        assert_eq!(col_letter_to_index(""), None);
        assert_eq!(col_letter_to_index("1"), None);
    }

    // -----------------------------------------------------------------------
    // parse_chart_spec — success paths
    // -----------------------------------------------------------------------

    fn minimal_bar_sheet() -> ExcelSheetData {
        sheet("C02_01", &[
            &["chart_type", "bar_clustered"],
            &["title", "Grafik 02.01 Penduduk Menurut Desa, 2026"],
            &["category_col", "A"],
            &["series_1_col", "B"],
            &["series_1_label", "Laki-laki"],
            &["data_start_row", "7"],
            &["data_end_row", "20"],
        ])
    }

    #[test]
    fn bar_clustered_spec_parses_correctly() {
        let s = minimal_bar_sheet();
        let spec = parse_chart_spec(&s).expect("should parse");
        assert_eq!(spec.chart_type, ChartType::BarClustered);
        assert!(spec.title_id.contains("Penduduk"));
        assert_eq!(spec.category_col, "A");
        assert_eq!(spec.series.len(), 1);
        assert_eq!(spec.series[0].col, "B");
        assert_eq!(spec.series[0].label_id, "Laki-laki");
        assert_eq!(spec.data_start_row, 7);
        assert_eq!(spec.data_end_row, 20);
        // Default optional fields.
        assert!(spec.show_value_labels);
        assert!(spec.show_legend);
        assert!(spec.show_grid);
        assert!(spec.max_value_override.is_none());
    }

    #[test]
    fn two_series_spec_parses_both() {
        let s = sheet("C03_01", &[
            &["chart_type", "bar_clustered"],
            &["title", "Grafik 03.01 Penduduk"],
            &["category_col", "A"],
            &["series_1_col", "B"],
            &["series_1_label", "Laki-laki"],
            &["series_1_label_en", "Male"],
            &["series_2_col", "C"],
            &["series_2_label", "Perempuan"],
            &["series_2_label_en", "Female"],
            &["data_start_row", "5"],
            &["data_end_row", "15"],
        ]);
        let spec = parse_chart_spec(&s).expect("should parse");
        assert_eq!(spec.series.len(), 2);
        assert_eq!(spec.series[0].label_en, "Male");
        assert_eq!(spec.series[1].col, "C");
        assert_eq!(spec.series[1].label_en, "Female");
    }

    #[test]
    fn optional_fields_are_parsed() {
        let s = sheet("C01_01", &[
            &["chart_type", "donut"],
            &["title", "Grafik 01.01 Luas Wilayah"],
            &["title_en", "Chart 01.01 Area"],
            &["category_col", "A"],
            &["series_1_col", "B"],
            &["series_1_label", "Ha"],
            &["data_start_row", "3"],
            &["data_end_row", "10"],
            &["value_format", "percent"],
            &["show_value_labels", "false"],
            &["show_legend", "true"],
            &["show_grid", "no"],
            &["max_value", "100.0"],
        ]);
        let spec = parse_chart_spec(&s).expect("should parse");
        assert_eq!(spec.chart_type, ChartType::Donut);
        assert_eq!(spec.title_en, "Chart 01.01 Area");
        assert_eq!(spec.value_format, ValueFormat::Percent);
        assert!(!spec.show_value_labels);
        assert!(spec.show_legend);
        assert!(!spec.show_grid);
        assert_eq!(spec.max_value_override, Some(100.0));
    }

    #[test]
    fn inline_semicolon_syntax_is_parsed() {
        // Sheets may use "key;value" in a single cell instead of two cells.
        let s = sheet("C04_01", &[
            &["chart_type;line"],
            &["title;Grafik 04.01 Trend"],
            &["category_col;A"],
            &["series_1_col;B"],
            &["series_1_label;Nilai"],
            &["data_start_row;2"],
            &["data_end_row;8"],
        ]);
        let spec = parse_chart_spec(&s).expect("should parse inline syntax");
        assert_eq!(spec.chart_type, ChartType::Line);
    }

    // -----------------------------------------------------------------------
    // parse_chart_spec — failure / fallback paths
    // -----------------------------------------------------------------------

    #[test]
    fn missing_chart_type_returns_none() {
        // A plain raster-image CXX sheet — no chart_type row.
        let s = sheet("C05_01", &[
            &["title", "Grafik 05.01 Panjang Jalan"],
            &["file", "grafik_jalan.png"],
        ]);
        assert!(parse_chart_spec(&s).is_none(), "no chart_type → None (raster fallback)");
    }

    #[test]
    fn unrecognised_chart_type_returns_none() {
        let s = sheet("C06_01", &[
            &["chart_type", "radar_chart"],       // not supported
            &["title", "Grafik 06.01 Radar"],
            &["category_col", "A"],
            &["series_1_col", "B"],
            &["series_1_label", "Nilai"],
            &["data_start_row", "2"],
            &["data_end_row", "10"],
        ]);
        assert!(parse_chart_spec(&s).is_none());
    }

    #[test]
    fn missing_category_col_returns_none() {
        let s = sheet("C07_01", &[
            &["chart_type", "bar"],
            &["title", "Grafik 07.01"],
            // category_col missing
            &["series_1_col", "B"],
            &["series_1_label", "Nilai"],
            &["data_start_row", "2"],
            &["data_end_row", "5"],
        ]);
        assert!(parse_chart_spec(&s).is_none(), "missing category_col → None");
    }

    #[test]
    fn missing_series_col_returns_none() {
        let s = sheet("C08_01", &[
            &["chart_type", "bar"],
            &["title", "Grafik 08.01"],
            &["category_col", "A"],
            // series_1_col missing
            &["series_1_label", "Nilai"],
            &["data_start_row", "2"],
            &["data_end_row", "5"],
        ]);
        assert!(parse_chart_spec(&s).is_none(), "missing series_1_col → None");
    }

    #[test]
    fn missing_data_start_row_returns_none() {
        let s = sheet("C09_01", &[
            &["chart_type", "bar"],
            &["title", "Grafik 09.01"],
            &["category_col", "A"],
            &["series_1_col", "B"],
            &["series_1_label", "Nilai"],
            // data_start_row missing
            &["data_end_row", "10"],
        ]);
        assert!(parse_chart_spec(&s).is_none(), "missing data_start_row → None");
    }

    #[test]
    fn data_end_before_start_returns_none() {
        let s = sheet("C10_01", &[
            &["chart_type", "bar"],
            &["title", "Grafik 10.01"],
            &["category_col", "A"],
            &["series_1_col", "B"],
            &["series_1_label", "Nilai"],
            &["data_start_row", "20"],
            &["data_end_row", "5"],   // end < start
        ]);
        assert!(parse_chart_spec(&s).is_none(), "data_end < data_start → None");
    }

    // -----------------------------------------------------------------------
    // extract_chart_data
    // -----------------------------------------------------------------------

    fn two_series_data_sheet() -> ExcelSheetData {
        // Rows 1–5 are metadata; rows 6–9 are data (1-based).
        let mut rows: Vec<Vec<String>> = vec![
            vec!["chart_type".into(), "bar_clustered".into()],
            vec!["title".into(), "Grafik 02.01 Penduduk".into()],
            vec!["category_col".into(), "A".into()],
            vec!["series_1_col".into(), "B".into()],
            vec!["series_2_col".into(), "C".into()],
            // Row 6 (index 5): data header — should be skipped by column-header check.
            vec!["Desa".into(), "(1)".into(), "(2)".into()],
            // Rows 7–10 (indices 6–9): data rows.
            vec!["Desa Karang Mukti".into(), "1.234".into(), "1.190".into()],
            vec!["Desa Cikampek".into(), "987".into(), "1.034".into()],
            vec!["Desa Telukjambe".into(), "2.100".into(), "2.050".into()],
            vec!["".into(), "".into(), "".into()],  // empty row — should be skipped
        ];
        // Add label / row spec rows at end (they'll be ignored by extract).
        rows.push(vec!["series_1_label".into(), "Laki-laki".into()]);
        rows.push(vec!["series_2_label".into(), "Perempuan".into()]);
        rows.push(vec!["data_start_row".into(), "7".into()]);
        rows.push(vec!["data_end_row".into(), "10".into()]);
        ExcelSheetData { name: "C02_01".into(), grid: rows, merges: vec![] }
    }

    #[test]
    fn data_extraction_returns_correct_categories_and_values() {
        let s = two_series_data_sheet();
        let spec = parse_chart_spec(&s).expect("should parse");
        let (cats, vals) = extract_chart_data(&s, &spec).expect("should extract data");

        // 3 non-empty rows with valid data (row 4 is empty → skipped).
        assert_eq!(cats.len(), 3);
        assert_eq!(cats[0], "Desa Karang Mukti");
        assert_eq!(cats[1], "Desa Cikampek");

        assert_eq!(vals.len(), 2); // 2 series
        assert_eq!(vals[0].len(), 3);
        assert!((vals[0][0] - 1234.0).abs() < 0.01, "Indonesian number format: 1.234 = 1234");
        assert!((vals[0][1] - 987.0).abs() < 0.01);
        assert!((vals[1][0] - 1190.0).abs() < 0.01);
    }

    #[test]
    fn data_extraction_skips_empty_rows() {
        let s = two_series_data_sheet();
        let spec = parse_chart_spec(&s).expect("should parse");
        let (cats, _) = extract_chart_data(&s, &spec).expect("should extract data");
        // The empty row should not appear in categories.
        assert!(!cats.contains(&String::new()));
    }

    #[test]
    fn data_extraction_handles_english_number_format() {
        let s = sheet("C11_01", &[
            &["chart_type", "line"],
            &["title", "Grafik 11.01"],
            &["category_col", "A"],
            &["series_1_col", "B"],
            &["series_1_label", "Value"],
            &["data_start_row", "8"],
            &["data_end_row", "10"],
            &["2021", "1,500"],   // English comma-thousands
            &["2022", "2,300.5"],
            &["2023", "3,100"],
        ]);
        let spec = parse_chart_spec(&s).expect("should parse");
        let (cats, vals) = extract_chart_data(&s, &spec).expect("should extract");
        assert_eq!(cats, vec!["2021", "2022", "2023"]);
        assert!((vals[0][0] - 1500.0).abs() < 0.01);
        assert!((vals[0][1] - 2300.5).abs() < 0.01);
    }

    #[test]
    fn data_extraction_returns_err_when_no_valid_rows() {
        let s = sheet("C12_01", &[
            &["chart_type", "bar"],
            &["title", "Grafik 12.01"],
            &["category_col", "A"],
            &["series_1_col", "B"],
            &["series_1_label", "Nilai"],
            &["data_start_row", "100"],  // beyond sheet length
            &["data_end_row", "110"],
        ]);
        let spec = parse_chart_spec(&s).expect("should parse");
        let result = extract_chart_data(&s, &spec);
        assert!(result.is_err(), "data_start beyond sheet → Err");
    }

    // -----------------------------------------------------------------------
    // ValueFormat::format
    // -----------------------------------------------------------------------

    #[test]
    fn value_format_number_uses_indonesian_separator() {
        let f = ValueFormat::Number;
        assert_eq!(f.format(1234.0, false), "1.234");
        assert_eq!(f.format(0.0, false), "0");
        assert_eq!(f.format(1234567.0, false), "1.234.567");
    }

    #[test]
    fn value_format_percent_appends_symbol() {
        let f = ValueFormat::Percent;
        assert_eq!(f.format(45.0, false), "45.0%");
        assert_eq!(f.format(33.3, false), "33.3%");
    }

    #[test]
    fn value_format_idr_uses_locale_appropriate_separator() {
        let f = ValueFormat::CurrencyIdr;
        assert_eq!(f.format(1500000.0, false), "Rp 1.500.000");
        assert_eq!(f.format(1500000.0, true), "IDR 1,500,000");
    }
}

