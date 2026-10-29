//! Locale words and bilingual helper utilities for Pubgen.
//!
//! Provides `Locale`, `LocaleWords`, and pure string parsing / splitting functions
//! for tables, charts, and bilingual metadata according to the sheet pattern rules.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Target publication locale for single-language generation mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    Id,
    En,
}

impl Locale {
    /// String identifier ("id" or "en").
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::En => "en",
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Locale {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Locale {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_ascii_lowercase().as_str() {
            "en" => Ok(Self::En),
            _ => Ok(Self::Id),
        }
    }
}

/// User-visible words and chrome vocabulary per locale.
pub struct LocaleWords {
    pub table: &'static str,        // "Tabel" / "Table"
    pub figure: &'static str,       // "Grafik" / "Chart"
    pub chapter: &'static str,      // "BAB" / "Chapter"
    pub review: &'static str,       // "Ulasan" / "Review"
    pub note: &'static str,         // "Keterangan" / "Note"
    pub divider: &'static str,      // "Pembatas" / "Divider"
    pub toc_contents: &'static str, // "Daftar Isi" / "Contents"
    pub toc_tables: &'static str,   // "Daftar Tabel" / "List of Tables"
    pub toc_figures: &'static str,  // "Daftar Grafik" / "List of Figures"
    pub page_column: &'static str,  // "halaman" / "page"
    pub source: &'static str,       // "Sumber" / "Source"
    pub total_words: &'static [&'static str],
    pub type_aliases: &'static [&'static str],
}

pub static ID_WORDS: LocaleWords = LocaleWords {
    table: "Tabel",
    figure: "Grafik",
    chapter: "BAB",
    review: "Ulasan",
    note: "Keterangan",
    divider: "Pembatas",
    toc_contents: "Daftar Isi",
    toc_tables: "Daftar Tabel",
    toc_figures: "Daftar Grafik",
    page_column: "halaman",
    source: "Sumber",
    total_words: &["jumlah", "tahun", "kabupaten", "kab", "kecamatan", "kec", "total", "sum", "subtotal", "year", "district", "subdistrict"],
    type_aliases: &["tabel", "table", "grafik", "chart", "bab", "chapter", "ulasan", "review", "keterangan", "note", "pembatas", "divider"],
};

pub static EN_WORDS: LocaleWords = LocaleWords {
    table: "Table",
    figure: "Chart",
    chapter: "Chapter",
    review: "Review",
    note: "Note",
    divider: "Divider",
    toc_contents: "Contents",
    toc_tables: "List of Tables",
    toc_figures: "List of Figures",
    page_column: "page",
    source: "Source",
    total_words: &["total", "sum", "subtotal", "year", "district", "subdistrict", "jumlah", "tahun", "kabupaten", "kab", "kecamatan", "kec"],
    type_aliases: &["table", "tabel", "chart", "grafik", "chapter", "bab", "review", "ulasan", "note", "keterangan", "divider", "pembatas"],
};

/// Get canonical vocabulary for the given locale.
pub fn words(locale: Locale) -> &'static LocaleWords {
    match locale {
        Locale::Id => &ID_WORDS,
        Locale::En => &EN_WORDS,
    }
}

/// Resolve chapter prefix from active profile nomenclature, or fall back to locale default.
pub fn chapter_prefix(profile: Option<&super::profiles::PubgenProfile>, locale: Locale) -> String {
    if let Some(p) = profile
        && !p.nomenclature.chapter_prefix.is_empty()
    {
        return p.nomenclature.chapter_prefix.clone();
    }
    words(locale).chapter.to_string()
}

/// Resolve table prefix from active profile nomenclature, or fall back to locale default.
pub fn table_prefix(profile: Option<&super::profiles::PubgenProfile>, locale: Locale) -> String {
    if let Some(p) = profile
        && !p.nomenclature.table_prefix.is_empty()
    {
        return p.nomenclature.table_prefix.clone();
    }
    words(locale).table.to_string()
}

/// Resolve chart/figure prefix from active profile nomenclature, or fall back to locale default.
pub fn chart_prefix(profile: Option<&super::profiles::PubgenProfile>, locale: Locale) -> String {
    if let Some(p) = profile
        && !p.nomenclature.chart_prefix.is_empty()
    {
        return p.nomenclature.chart_prefix.clone();
    }
    words(locale).figure.to_string()
}

/// Resolve TOC title from active profile nomenclature, or fall back to locale default.
pub fn toc_title(profile: Option<&super::profiles::PubgenProfile>, locale: Locale) -> String {
    if let Some(p) = profile
        && !p.nomenclature.toc_title.is_empty()
    {
        return p.nomenclature.toc_title.clone();
    }
    words(locale).toc_contents.to_string()
}

/// Detect locale from catalog row type words.
pub fn detect_from_records(type_col: usize, rows: &[Vec<String>]) -> Locale {
    let has_en = rows.iter().any(|r| {
        let t = r.get(type_col).map(String::as_str).unwrap_or("");
        t.eq_ignore_ascii_case("table") || t.eq_ignore_ascii_case("chart") || t.eq_ignore_ascii_case("chapter") || t.eq_ignore_ascii_case("review")
    });
    if has_en { Locale::En } else { Locale::Id }
}

/// "ID / EN" -> Some(("ID", "EN")). None when there is no `" / "`.
///
/// Delimiter is strictly `" / "` (space slash space). A bare `/` (e.g. "Tenaga Medis/Dokter")
/// is data and returns `None`.
pub fn split_pair(s: &str) -> Option<(&str, &str)> {
    let (id, en) = s.split_once(" / ")?;
    Some((id.trim(), en.trim()))
}

/// Two-line table title: ("ID", Some("EN")) or ("ID", None).
pub fn split_table_title(s: &str) -> (String, Option<String>) {
    if let Some((id, en)) = split_pair(s) { (id.to_string(), Some(en.to_string())) } else { (s.trim().to_string(), None) }
}

/// Chart title parts with exact character spans for italicization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChartTitleParts {
    pub id_kicker: String,
    pub en_kicker: Option<String>,
    pub number: String,
    pub id_subtitle: String,
    pub en_subtitle: Option<String>,
    pub suffix: String,
    /// Joined display string ready for rendering.
    pub display: String,
    /// Char offset ranges [start, end) into `display` that should receive italic style.
    pub italic_ranges: Vec<(usize, usize)>,
}

/// Parse chart title string into components and calculate exact italic character ranges.
///
/// Expected shape:
/// `"<ID kicker> / <EN kicker> <number> <ID subtitle> / <EN subtitle>, <year>"`
/// e.g. `"Grafik / Chart 02.01 Panjang Jalan / Road Length, 2026"`
pub fn split_chart_title_parts(s: &str) -> ChartTitleParts {
    let s_trimmed = s.trim();
    // Split into parts by " / "
    let parts: Vec<&str> = s_trimmed.split(" / ").collect();

    if parts.len() < 3 {
        // Fallback for single-language or non-standard title
        return ChartTitleParts {
            id_kicker: String::new(),
            en_kicker: None,
            number: String::new(),
            id_subtitle: s_trimmed.to_string(),
            en_subtitle: None,
            suffix: String::new(),
            display: s_trimmed.to_string(),
            italic_ranges: Vec::new(),
        };
    }

    let id_kicker = parts.first().copied().unwrap_or("").trim().to_string();

    // Middle part: "<EN kicker> <number> <ID subtitle>"
    let middle = parts.get(1).copied().unwrap_or("").trim();
    let middle_tokens: Vec<&str> = middle.split_whitespace().collect();
    let en_kicker = middle_tokens.first().copied().unwrap_or("").to_string();
    let number = middle_tokens.get(1).copied().unwrap_or("").to_string();
    let id_subtitle = if middle_tokens.len() > 2 { middle_tokens[2..].join(" ") } else { String::new() };

    // Third part: "<EN subtitle>, <year>" or everything after second " / "
    let third = parts[2..].join(" / ");
    let (en_subtitle, suffix) = if let Some(last_comma) = third.rfind(',') {
        let (sub, suf) = third.split_at(last_comma);
        (sub.trim().to_string(), suf.to_string())
    } else {
        (third.trim().to_string(), String::new())
    };

    // Reconstruct canonical display string:
    // "{id_kicker} / {en_kicker} {number} {id_subtitle} / {en_subtitle}{suffix}"
    let mut display = format!("{id_kicker} / {en_kicker}");
    let chart_start = id_kicker.chars().count() + 3; // after "{id_kicker} / "
    let chart_end = chart_start + en_kicker.chars().count();

    if !number.is_empty() {
        display.push(' ');
        display.push_str(&number);
    }
    if !id_subtitle.is_empty() {
        display.push(' ');
        display.push_str(&id_subtitle);
    }
    display.push_str(" / ");

    let sub_start = display.chars().count();
    display.push_str(&en_subtitle);
    let sub_end = display.chars().count();

    display.push_str(&suffix);

    let mut italic_ranges = Vec::new();
    if chart_start < chart_end {
        italic_ranges.push((chart_start, chart_end));
    }
    if sub_start < sub_end {
        italic_ranges.push((sub_start, sub_end));
    }

    ChartTitleParts { id_kicker, en_kicker: Some(en_kicker), number, id_subtitle, en_subtitle: Some(en_subtitle), suffix, display, italic_ranges }
}

/// Convert char offsets [start, end) into UTF-8 byte offsets in `text`.
pub fn char_to_byte_range(text: &str, char_start: usize, char_end: usize) -> (usize, usize) {
    let mut byte_start = text.len();
    let mut byte_end = text.len();
    let mut count = 0;

    for (b_idx, _) in text.char_indices() {
        if count == char_start {
            byte_start = b_idx;
        }
        if count == char_end {
            byte_end = b_idx;
            break;
        }
        count += 1;
    }

    if count < char_start {
        byte_start = text.len();
    }
    if count < char_end {
        byte_end = text.len();
    }

    (byte_start, byte_end)
}

/// Apply italic styling to the specified character ranges of a story (defaults to "Bold Italic").
pub fn italicize_ranges(app: &mut crate::DesignApp, story: u64, text: &str, ranges: &[(usize, usize)]) {
    italicize_ranges_style(app, story, text, ranges, "Bold Italic");
}

/// Apply character font style (e.g. "Italic", "Regular", "Bold Italic") to the specified character ranges of a story.
pub fn italicize_ranges_style(app: &mut crate::DesignApp, story: u64, text: &str, ranges: &[(usize, usize)], font_style: &str) {
    for &(char_start, char_end) in ranges {
        let (byte_start, byte_end) = char_to_byte_range(text, char_start, char_end);
        if byte_start >= byte_end {
            continue;
        }
        let _ = app.run(
            "text.select",
            serde_json::json!({
                "story": story,
                "anchor": byte_start,
                "focus": byte_end,
            }),
        );
        let _ = app.run(
            "type.char",
            serde_json::json!({
                "attrs": {
                    "fontStyle": font_style
                }
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_pair() {
        assert_eq!(split_pair("A / B"), Some(("A", "B")));
        assert_eq!(split_pair("Tenaga Medis/Dokter"), None);
        assert_eq!(split_pair("A/B"), None);
        assert_eq!(split_pair("Single Title"), None);
    }

    #[test]
    fn test_split_table_title() {
        assert_eq!(split_table_title("Banyaknya Desa / Number of Villages"), ("Banyaknya Desa".to_string(), Some("Number of Villages".to_string())));
        assert_eq!(split_table_title("Puskesmas/Posyandu"), ("Puskesmas/Posyandu".to_string(), None));
    }

    #[test]
    fn test_split_chart_title_parts() {
        let input = "Grafik / Chart 02.01 Panjang Jalan Kecamatan Karawang Timur / Road Length in East Karawang District, 2026";
        let parts = split_chart_title_parts(input);

        assert_eq!(parts.id_kicker, "Grafik");
        assert_eq!(parts.en_kicker, Some("Chart".to_string()));
        assert_eq!(parts.number, "02.01");
        assert_eq!(parts.id_subtitle, "Panjang Jalan Kecamatan Karawang Timur");
        assert_eq!(parts.en_subtitle, Some("Road Length in East Karawang District".to_string()));
        assert_eq!(parts.suffix, ", 2026");

        // Verify italic character spans
        assert_eq!(parts.italic_ranges.len(), 2);
        let (c_start, c_end) = parts.italic_ranges[0];
        let chart_slice: String = parts.display.chars().skip(c_start).take(c_end - c_start).collect();
        assert_eq!(chart_slice, "Chart");

        let (s_start, s_end) = parts.italic_ranges[1];
        let sub_slice: String = parts.display.chars().skip(s_start).take(s_end - s_start).collect();
        assert_eq!(sub_slice, "Road Length in East Karawang District");
    }

    #[test]
    fn test_char_to_byte_range_multibyte() {
        let text = "Grafik\u{2028}Chart"; // U+2028 is a 3-byte character
        assert_eq!(text.chars().count(), 12);
        assert_eq!(text.len(), 14);

        let (b_start, b_end) = char_to_byte_range(text, 7, 12);
        assert_eq!(&text[b_start..b_end], "Chart");
    }

    #[test]
    fn test_locale_serde() {
        assert_eq!(serde_json::to_string(&Locale::Id).unwrap(), "\"id\"");
        assert_eq!(serde_json::to_string(&Locale::En).unwrap(), "\"en\"");
        assert_eq!(serde_json::from_str::<Locale>("\"id\"").unwrap(), Locale::Id);
        assert_eq!(serde_json::from_str::<Locale>("\"en\"").unwrap(), Locale::En);
        assert_eq!(serde_json::from_str::<Locale>("\"unknown\"").unwrap(), Locale::Id);
    }
}
