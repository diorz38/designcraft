//! Table color themes and the DIV-swatch resolver.

use super::*;
/// Table color theme specifying background colors for table header/footer and row number row.

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TableColorTheme {
    pub header_footer: String,
    pub row_number: String,
    #[serde(default)]
    pub header_text_color: Option<String>,
    #[serde(default)]
    pub data_odd_fill: Option<String>,
    #[serde(default)]
    pub data_even_fill: Option<String>,
    #[serde(default)]
    pub font_size_delta: Option<i32>,
}

impl TableColorTheme {
    pub fn new(header_footer: impl Into<String>, row_number: impl Into<String>) -> Self {
        Self {
            header_footer: header_footer.into(),
            row_number: row_number.into(),
            header_text_color: None,
            data_odd_fill: None,
            data_even_fill: None,
            font_size_delta: None,
        }
    }

    pub fn brs6() -> Self {
        Self {
            header_footer: "#D07C38".into(),
            row_number: "#E7CFB5".into(),
            header_text_color: Some("[Paper]".into()),
            data_odd_fill: Some("#FEF0D2".into()),
            data_even_fill: Some("#FEF7E9".into()),
            font_size_delta: Some(-1),
        }
    }

    pub fn is_none(&self) -> bool {
        let hf = self.header_footer.trim();
        hf.is_empty() || hf.eq_ignore_ascii_case("none")
    }
}

pub const TABLE_COLOR_THEMES: &[(&str, &str, &str)] = &[
    ("#FFCB56 - #FFE9A9", "#FFCB56", "#FFE9A9"),
    ("#F3D2AF - #F8EADC", "#F3D2AF", "#F8EADC"),
    ("#9E7EB5 - #C4A4DA", "#9E7EB5", "#C4A4DA"),
    ("None", "", ""),
    ("BRS6 (#D07C38 - #E7CFB5)", "#D07C38", "#E7CFB5"),
];

/// Parse a table color theme from string (preset index "0", "1", "2", hex names, or custom pair).
pub(crate) fn parse_table_color_theme(val: &str) -> Option<TableColorTheme> {
    let s = val.trim();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_lowercase();
    if lower == "none" || lower == "plain" || lower == "white" || lower == "transparent" {
        return Some(TableColorTheme::new("", ""));
    }
    if s == "4" || lower == "brs6" || lower == "brs 6" || lower.contains("d07c3") || lower.contains("d07c38") {
        return Some(TableColorTheme::brs6());
    }
    if s == "0" || lower.contains("ffcb56") || lower == "gold" || lower == "amber" {
        return Some(TableColorTheme::new("#FFCB56", "#FFE9A9"));
    }
    if s == "1" || lower.contains("f3d2af") || lower.contains("f8eadc") || lower == "peach" || lower == "sand" {
        return Some(TableColorTheme::new("#F3D2AF", "#F8EADC"));
    }
    if s == "2" || lower.contains("9e7eb5") || lower.contains("c4a4da") || lower == "lilac" || lower == "lavender" {
        return Some(TableColorTheme::new("#9E7EB5", "#C4A4DA"));
    }
    for sep in ['-', ',', '/', ';'] {
        if let Some((h, r)) = s.split_once(sep) {
            let h = h.trim();
            let r = r.trim();
            if !h.is_empty() && !r.is_empty() {
                let h_hex = if h.starts_with('#') { h.to_string() } else { format!("#{h}") };
                let r_hex = if r.starts_with('#') { r.to_string() } else { format!("#{r}") };
                return Some(TableColorTheme::new(h_hex, r_hex));
            }
        }
    }
    None
}

/// Ensure a hex color swatch exists in the document swatches and return its canonical swatch name.
pub(crate) fn ensure_swatch(doc: &mut designcraft_doc::Document, hex: &str) -> String {
    let clean = hex.trim();
    if clean.is_empty() || clean.eq_ignore_ascii_case("none") {
        return String::new();
    }
    if clean == "[Paper]" || clean.eq_ignore_ascii_case("white") {
        return "[Paper]".to_string();
    }
    let formatted = if clean.starts_with('#') { clean.to_string() } else { format!("#{clean}") };
    let normalized = if formatted.len() == 6 && formatted.starts_with('#') {
        format!("{formatted}8")
    } else {
        formatted
    };
    let upper = normalized.to_uppercase();
    if !doc.swatches.iter().any(|s| s.name.eq_ignore_ascii_case(&upper))
        && let Some(col) = designcraft_color::Color::from_hex(&upper)
    {
        doc.swatches.push(designcraft_color::swatch::Swatch::color(&upper, col));
    }
    upper
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_parse_table_color_theme() {
        assert_eq!(parse_table_color_theme("0"), Some(TableColorTheme::new("#FFCB56", "#FFE9A9")));
        assert_eq!(parse_table_color_theme("gold"), Some(TableColorTheme::new("#FFCB56", "#FFE9A9")));
        assert_eq!(parse_table_color_theme("1"), Some(TableColorTheme::new("#F3D2AF", "#F8EADC")));
        assert_eq!(parse_table_color_theme("peach"), Some(TableColorTheme::new("#F3D2AF", "#F8EADC")));
        assert_eq!(parse_table_color_theme("2"), Some(TableColorTheme::new("#9E7EB5", "#C4A4DA")));
        assert_eq!(parse_table_color_theme("lilac"), Some(TableColorTheme::new("#9E7EB5", "#C4A4DA")));
        assert_eq!(parse_table_color_theme("none"), Some(TableColorTheme::new("", "")));
        assert_eq!(parse_table_color_theme("plain"), Some(TableColorTheme::new("", "")));
        assert_eq!(parse_table_color_theme("#112233 - #445566"), Some(TableColorTheme::new("#112233", "#445566")));
        assert_eq!(parse_table_color_theme("BRS6"), Some(TableColorTheme::brs6()));
        assert_eq!(parse_table_color_theme("brs6"), Some(TableColorTheme::brs6()));
        assert_eq!(parse_table_color_theme("#d07c3"), Some(TableColorTheme::brs6()));
        assert_eq!(parse_table_color_theme("#d07c38"), Some(TableColorTheme::brs6()));
    }
}
