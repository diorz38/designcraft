//! CSV matrix parsing and Pubgen row predicates.

use super::*;

pub(crate) fn delimiter_name(delim: char) -> &'static str {
    match delim {
        ';' => "semicolon ';'",
        '\t' => "tab '\\t'",
        '|' => "pipe '|'",
        _ => "comma ','",
    }
}

/// Sniff candidate delimiter (',', ';', '\t', '|') using quotation-aware row sampling,
/// frequency analysis, and header consistency heuristics.
pub(crate) fn detect_delimiter(text: &str) -> char {
    let clean = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    if clean.trim().is_empty() {
        return ',';
    }

    const CANDIDATES: [char; 4] = [',', ';', '\t', '|'];
    const MAX_SAMPLED_ROWS: usize = 50;

    let mut rows_counts: Vec<[usize; 4]> = Vec::new();
    let mut current_counts = [0usize; 4];
    let mut in_quotes = false;
    let mut row_has_content = false;

    let mut chars = clean.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    chars.next(); // escaped quote
                } else {
                    in_quotes = !in_quotes;
                }
                row_has_content = true;
            }
            '\n' if !in_quotes => {
                if row_has_content {
                    rows_counts.push(current_counts);
                    current_counts = [0usize; 4];
                    row_has_content = false;
                    if rows_counts.len() >= MAX_SAMPLED_ROWS {
                        break;
                    }
                }
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                if row_has_content {
                    rows_counts.push(current_counts);
                    current_counts = [0usize; 4];
                    row_has_content = false;
                    if rows_counts.len() >= MAX_SAMPLED_ROWS {
                        break;
                    }
                }
            }
            c if !in_quotes => {
                if let Some(idx) = CANDIDATES.iter().position(|&cand| cand == c) {
                    if let Some(slot) = current_counts.get_mut(idx) {
                        *slot += 1;
                    }
                    row_has_content = true;
                } else if !c.is_whitespace() {
                    row_has_content = true;
                }
            }
            _ => {}
        }
    }

    if row_has_content && rows_counts.len() < MAX_SAMPLED_ROWS {
        rows_counts.push(current_counts);
    }

    if rows_counts.is_empty() {
        return ',';
    }

    let num_rows = rows_counts.len();
    let mut best_cand = ',';
    let mut best_score = 0.0f64;

    for (idx, &cand) in CANDIDATES.iter().enumerate() {
        let counts: Vec<usize> = rows_counts.iter().map(|r| r.get(idx).copied().unwrap_or(0)).collect();
        if counts.iter().all(|&c| c == 0) {
            continue;
        }

        // Count frequencies of each non-zero count
        let mut freq_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for &c in &counts {
            if c > 0 {
                *freq_map.entry(c).or_insert(0usize) += 1;
            }
        }

        let Some((&mode_val, &mode_freq)) = freq_map.iter().max_by_key(|(_val, freq)| *freq) else {
            continue;
        };

        let consistency = if num_rows > 0 { mode_freq as f64 / num_rows as f64 } else { 0.0 };
        let header_count = counts.first().copied().unwrap_or(0);
        let header_bonus = if header_count == mode_val {
            2.0
        } else if header_count > 0 {
            1.0
        } else {
            0.05
        };

        let non_zero_rows = counts.iter().filter(|&&c| c > 0).count();
        let non_zero_ratio = if num_rows > 0 { non_zero_rows as f64 / num_rows as f64 } else { 0.0 };
        let field_factor = (mode_val as f64).min(10.0);

        let score = consistency * consistency * header_bonus * non_zero_ratio * (field_factor + 1.0);

        if score > best_score + 1e-6 {
            best_score = score;
            best_cand = cand;
        }
    }

    best_cand
}

/// Parse delimited tabular data conforming to RFC 4180 with the specified delimiter.
pub(crate) fn parse_csv_content_with_delim(text: &str, delim: char) -> Vec<Vec<String>> {
    let clean = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = clean.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            c if c == delim && !in_quotes => {
                row.push(std::mem::take(&mut field));
            }
            '\n' if !in_quotes => {
                row.push(std::mem::take(&mut field));
                if row.iter().any(|c| !c.trim().is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                if row.iter().any(|c| !c.trim().is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            other => {
                field.push(other);
            }
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        if row.iter().any(|c| !c.trim().is_empty()) {
            rows.push(row);
        }
    }
    rows
}

pub(crate) fn parse_csv_content(text: &str) -> Vec<Vec<String>> {
    let delim = detect_delimiter(text);
    parse_csv_content_with_delim(text, delim)
}

/// Find a CSV column by exact header name (case-insensitive, all columns); `fallback` position
/// when no header matches — keeps legacy fixed-position layouts working.
/// Resolve a catalog-relative image: `resolve_catalog_related_file` for each of `exts` in
/// turn (direct paths are already covered by that helper itself).
pub(crate) fn resolve_image_file(base: &str, file_str: &str, exts: &[&str]) -> Option<std::path::PathBuf> {
    exts.iter().find_map(|ext| resolve_catalog_related_file(base, file_str, ext))
}

pub(crate) fn find_named_col(headers: &[String], patterns: &[&str], fallback: usize) -> usize {
    for (i, h) in headers.iter().enumerate() {
        let lower = h.trim().to_lowercase();
        if patterns.iter().any(|p| lower == *p) {
            return i;
        }
    }
    fallback
}

/// Header names the Page-from-CSV machinery knows. Used to detect when a legacy
/// fallback position is occupied by a *different* column (e.g. `image2` sitting where
/// `file_en` would be in the 7-column layout).
const KNOWN_CSV_HEADERS: &[&str] = &[
    "type", "category", "kicker", "kind", "number", "no", "num", "id", "#", "title", "name", "headline", "chapter", "file", "filename", "template",
    "title_en", "titleen", "file_en", "fileen", "image1", "image2",
];

/// The effective column for a named Page-from-CSV column: the stored index when its header
/// still matches, else re-resolved by name (records set without `load_card1_csv`, e.g. by the
/// Excel conversion path), else the legacy fixed position.
pub(crate) fn effective_col(stored: usize, headers: &[String], patterns: &[&str], fallback: usize) -> usize {
    if let Some(h) = headers.get(stored)
        && patterns.contains(&h.trim().to_lowercase().as_str())
    {
        return stored;
    }
    let found = find_named_col(headers, patterns, usize::MAX);
    if found != usize::MAX {
        return found;
    }
    match headers.get(fallback) {
        None => usize::MAX,
        Some(h) => {
            let l = h.trim().to_lowercase();
            let known_other = KNOWN_CSV_HEADERS.iter().any(|k| !patterns.contains(k) && l == *k);
            if known_other { usize::MAX } else { fallback }
        }
    }
}

pub(crate) fn load_card1_csv(st: &mut PubgenState, path: &str) {
    st.card1_path = path.to_string();
    let Ok(content) = std::fs::read_to_string(path) else {
        st.card1_status = Some((format!("Cannot read file: {path}"), false));
        return;
    };
    let delim = detect_delimiter(&content);
    st.card1_delim = Some(delim);
    let mut rows = parse_csv_content_with_delim(&content, delim);
    if rows.is_empty() {
        st.card1_status = Some(("File is empty".into(), false));
        return;
    }
    let headers = rows.remove(0);
    st.card1_headers = headers.clone();
    st.card1_records = rows;

    // Detect column indexes case-insensitively within the first 3 columns
    let find_col = |patterns: &[&str], fallback: usize| -> usize {
        for (i, h) in headers.iter().enumerate().take(3) {
            let lower = h.trim().to_lowercase();
            if patterns.iter().any(|p| lower.contains(p)) {
                return i;
            }
        }
        fallback.min(headers.len().saturating_sub(1)).min(2)
    };

    st.card1_type_col = find_col(&["type", "category", "kicker", "kind"], 0);
    st.card1_num_col = find_col(&["number", "no", "num", "id", "#"], 1);
    st.card1_title_col = find_col(&["title", "name", "headline", "chapter"], 2);

    // The remaining columns match by exact header name across all columns (type/number/title
    // above stay within the first 3 for legacy layouts). A missing header keeps the legacy
    // fixed position, so old CSVs keep working.
    st.card1_file_col = find_named_col(&headers, &["file", "filename"], 3);
    st.card1_template_col = find_named_col(&headers, &["template"], 4);
    st.card1_title_en_col = find_named_col(&headers, &["title_en", "titleen"], 5);
    st.card1_file_en_col = find_named_col(&headers, &["file_en", "fileen"], 6);

    st.card1_range_to = st.card1_records.len();
    let delim_name = delimiter_name(delim);
    let file_info = if headers.len() >= 4 { " · 'file' col detected" } else { "" };
    st.card1_status = Some((format!("Loaded {} records (delimiter: {delim_name}){file_info}", st.card1_records.len()), true));
}

pub(crate) fn load_card2_csv(st: &mut PubgenState, path: &str) {
    st.card2_path = path.to_string();
    let Ok(content) = std::fs::read_to_string(path) else {
        st.card2_status = Some((format!("Cannot read file: {path}"), false));
        return;
    };
    let delim = detect_delimiter(&content);
    st.card2_delim = Some(delim);
    let rows = parse_csv_content_with_delim(&content, delim);
    if rows.is_empty() {
        st.card2_status = Some(("File is empty".into(), false));
        return;
    }
    st.card2_matrix = rows;

    for row in &st.card2_matrix {
        let first = row.first().map(|s| s.trim()).unwrap_or("");
        let first_lower = first.to_lowercase();
        if excel::THEME_KEYS.iter().any(|k| first_lower.starts_with(k)) {
            let val = if let Some(pos) = first.find(':') {
                first[pos + 1..].trim().to_string()
            } else {
                row.get(1).map(|s| s.trim().to_string()).unwrap_or_default()
            };
            if !val.is_empty()
                && let Some(th) = parse_table_color_theme(&val)
            {
                if th == TableColorTheme::new("#FFCB56", "#FFE9A9") {
                    st.card2_color_theme = 0;
                } else if th == TableColorTheme::new("#F3D2AF", "#F8EADC") {
                    st.card2_color_theme = 1;
                } else if th == TableColorTheme::new("#9E7EB5", "#C4A4DA") {
                    st.card2_color_theme = 2;
                } else if th.is_none() {
                    st.card2_color_theme = 3;
                }
                st.card2_theme_override = Some(val);
            }
            break;
        }
    }

    let r = st.card2_matrix.len();
    let c = st.card2_matrix.first().map(Vec::len).unwrap_or(0);
    let delim_name = delimiter_name(delim);
    st.card2_status = Some((format!("Loaded {r} rows × {c} columns (delimiter: {delim_name})"), true));
}

/// Detect if the last row of the tabular CSV only contains data in the first column
/// (the remaining columns are empty), functioning as a footnote or source note.
pub(crate) fn detect_footnote_row(matrix: &[Vec<String>]) -> Option<String> {
    if matrix.len() < 2 {
        return None;
    }
    let cols_count = matrix.iter().map(Vec::len).max().unwrap_or(0);
    if cols_count <= 1 {
        return None;
    }
    let last = matrix.last()?;
    let first = last.first().map(|s| s.trim()).unwrap_or("");
    let rest_empty = last.iter().skip(1).all(|c| c.trim().is_empty());
    if !first.is_empty() && rest_empty { Some(first.to_string()) } else { None }
}

/// Check if a cell starts with one of the summary/break keywords:
/// Jumlah, Tahun, Kab, Kabupaten, Kec, Kecamatan (or Total).
pub(crate) fn matches_pubgen_row_condition(text: &str) -> bool {
    let trimmed = text.trim().trim_start_matches(['*', '-', '•', '–', '—']).trim();
    let lower = trimmed.to_lowercase();
    if lower.is_empty() {
        return false;
    }

    let match_word = |prefix: &str| -> bool {
        if lower == prefix {
            return true;
        }
        if let Some(rest) = lower.strip_prefix(prefix) {
            if let Some(first_ch) = rest.chars().next() {
                first_ch.is_whitespace() || matches!(first_ch, '.' | '/' | '-' | ':' | '(' | '[' | '–' | '—')
            } else {
                true
            }
        } else {
            false
        }
    };

    match_word("jumlah") || match_word("tahun") || match_word("total") || match_word("sum") || match_word("subtotal") || match_word("year")
}

pub(crate) fn is_index_or_empty(text: &str) -> bool {
    let t = text.trim();
    t.is_empty() || t.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | ')' | '(' | '-' | '#'))
}

/// Check if a cell's string representation represents numeric data, a dash, or a missing data symbol.
pub(crate) fn is_numeric_or_dash(text: &str) -> bool {
    let t = text.replace('\u{00A0}', " ");
    let t = t.trim().trim_matches('"').trim_matches('\'').trim();
    if t.is_empty() {
        return false;
    }
    if matches!(t, "-" | "—" | "–" | "- " | "--" | "−" | "..." | "…" | "N/A" | "n/a" | "NA" | "na") {
        return true;
    }
    let mut clean = t.trim_matches(|c: char| c == '%' || c == '$' || c == '€' || c == '£').trim();
    if clean.is_empty() {
        return false;
    }
    if clean.starts_with('(') && clean.ends_with(')') && clean.len() > 2 {
        clean = clean[1..clean.len() - 1].trim();
    }
    let clean = clean.replace('−', "-");
    if clean.parse::<f64>().is_ok() {
        return true;
    }
    let without_commas = clean.replace(',', "");
    if without_commas.parse::<f64>().is_ok() {
        return true;
    }
    let eu_converted = clean.replace('.', "").replace(',', ".");
    if eu_converted.parse::<f64>().is_ok() {
        return true;
    }
    false
}
