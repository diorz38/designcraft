//! Declarative publication profiles for Pubgen (Pillar 7).
//!
//! Provides the [`PubgenProfile`] schema, built-in bundled presets, and profile manager
//! that parameterize layout geometry, sheet routing, nomenclature, and table styling.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PubgenProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub sector: ProfileSector,
    pub geometry: ProfileGeometry,
    pub structure: ProfileStructure,
    pub nomenclature: ProfileNomenclature,
    pub sheet_routing: ProfileSheetRouting,
    pub table_styling: ProfileTableStyling,
    #[serde(default)]
    pub typography: ProfileTypography,
}

impl Default for PubgenProfile {
    fn default() -> Self {
        ProfileManager::default_profile()
    }
}

impl PubgenProfile {
    /// Sanitize all bounds and fields to prevent engine crashes or invalid geometry.
    pub fn sanitize(&mut self) {
        self.geometry.margins_mm.sanitize();
        if self.id.trim().is_empty() {
            self.id = "custom".to_string();
        }
        if self.name.trim().is_empty() {
            self.name = "Custom Profile".to_string();
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ProfileSector {
    #[default]
    BpsOfficial,
    Government,
    Corporate,
    Academic,
    Custom,
}

impl ProfileSector {
    pub fn label(self) -> &'static str {
        match self {
            Self::BpsOfficial => "BPS Official",
            Self::Government => "Pemerintahan / LPPD",
            Self::Corporate => "Korporat & Bisnis",
            Self::Academic => "Akademik & Riset",
            Self::Custom => "Kustom",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProfileGeometry {
    pub page_size: String,  // "A4", "B5", "A5", "Letter"
    pub facing_pages: bool, // true = facing spreads with spine/gutter; false = single page
    pub margins_mm: MarginsMm,
}

impl Default for ProfileGeometry {
    fn default() -> Self {
        Self { page_size: "A4".to_string(), facing_pages: true, margins_mm: MarginsMm::default() }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct MarginsMm {
    pub top: f64,
    pub bottom: f64,
    pub inside: f64,  // Or Left for single pages
    pub outside: f64, // Or Right for single pages
}

impl Default for MarginsMm {
    fn default() -> Self {
        Self { top: 20.2, bottom: 20.0, inside: 19.0, outside: 15.0 }
    }
}

impl MarginsMm {
    pub fn sanitize(&mut self) {
        self.top = self.top.clamp(5.0, 100.0);
        self.bottom = self.bottom.clamp(5.0, 100.0);
        self.inside = self.inside.clamp(5.0, 100.0);
        self.outside = self.outside.clamp(5.0, 100.0);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProfileStructure {
    pub has_front_matter: bool,
    pub front_matter_numbering: String, // "roman_lower", "arabic", "none"
    pub restart_body_numbering: bool,
    pub require_chapter_dividers: bool,
    pub include_inside_cover: bool,
    pub include_back_cover: bool,
    #[serde(default = "default_true")]
    pub bilingual: bool,
    #[serde(default)]
    pub header_footer_preset: Option<String>,
}

impl Default for ProfileStructure {
    fn default() -> Self {
        Self {
            has_front_matter: true,
            front_matter_numbering: "roman_lower".to_string(),
            restart_body_numbering: true,
            require_chapter_dividers: true,
            include_inside_cover: true,
            include_back_cover: true,
            bilingual: true,
            header_footer_preset: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProfileNomenclature {
    pub chapter_prefix: String,     // "BAB", "Bagian", "Section", or ""
    pub table_prefix: String,       // "Tabel", "Table"
    pub chart_prefix: String,       // "Grafik", "Chart", "Figure"
    pub toc_title: String,          // "Daftar Isi", "Table of Contents"
    pub list_tables_title: String,  // "Daftar Tabel", "List of Tables"
    pub list_charts_title: String,  // "Daftar Grafik", "List of Figures"
    pub source_labels: Vec<String>, // ["Sumber:", "Source:", "Data:"]
    pub note_labels: Vec<String>,   // ["Catatan:", "Note:", "Notes:"]
}

impl Default for ProfileNomenclature {
    fn default() -> Self {
        Self {
            chapter_prefix: "BAB".to_string(),
            table_prefix: "Tabel".to_string(),
            chart_prefix: "Grafik".to_string(),
            toc_title: "Daftar Isi".to_string(),
            list_tables_title: "Daftar Tabel".to_string(),
            list_charts_title: "Daftar Grafik".to_string(),
            source_labels: vec!["Sumber:".to_string(), "Source:".to_string(), "Sumber / Source:".to_string()],
            note_labels: vec!["Catatan:".to_string(), "Note:".to_string(), "Catatan / Note:".to_string(), "Keterangan:".to_string()],
        }
    }
}

impl ProfileNomenclature {
    pub fn matches_source(&self, text: &str) -> bool {
        let t = text.trim();
        self.source_labels.iter().any(|l| {
            let label_trim = l.trim();
            let label_no_colon = label_trim.trim_end_matches(':');
            t.starts_with(label_trim) || t.starts_with(label_no_colon)
        })
    }

    pub fn matches_note(&self, text: &str) -> bool {
        let t = text.trim();
        self.note_labels.iter().any(|l| {
            let label_trim = l.trim();
            let label_no_colon = label_trim.trim_end_matches(':');
            t.starts_with(label_trim) || t.starts_with(label_no_colon)
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProfileSheetRouting {
    pub cover_patterns: Vec<String>,
    pub divider_patterns: Vec<String>,
    pub narrative_patterns: Vec<String>,
    pub table_patterns: Vec<String>,
    pub chart_patterns: Vec<String>,
    #[serde(default)]
    pub graphic_patterns: Vec<String>,
}

impl Default for ProfileSheetRouting {
    fn default() -> Self {
        Self {
            cover_patterns: vec!["(?i)^cover$".to_string(), "(?i)^sampul$".to_string()],
            divider_patterns: vec!["(?i)^div.*".to_string(), "(?i)^pembatas.*".to_string()],
            narrative_patterns: vec!["(?i)^r\\d+.*".to_string(), "(?i)^ulasan.*".to_string()],
            table_patterns: vec!["(?i)^\\d+[_.]\\d+.*".to_string(), "(?i)^tabel.*".to_string(), "(?i)^table.*".to_string()],
            chart_patterns: vec!["(?i)^c\\d+.*".to_string(), "(?i)^chart.*".to_string(), "(?i)^grafik.*".to_string()],
            graphic_patterns: vec!["(?i)^g\\d+.*".to_string(), "(?i)^gambar.*".to_string(), "(?i)^image.*".to_string()],
        }
    }
}

fn matches_any_pattern(patterns: &[String], s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    for p in patterns {
        if let Ok(re) = regex::Regex::new(p) {
            if re.is_match(trimmed) {
                return true;
            }
        } else if trimmed.eq_ignore_ascii_case(p) {
            return true;
        }
    }
    false
}

impl ProfileSheetRouting {
    pub fn matches_cover(&self, sheet_name: &str) -> bool {
        matches_any_pattern(&self.cover_patterns, sheet_name)
    }

    pub fn matches_divider(&self, sheet_name: &str) -> bool {
        matches_any_pattern(&self.divider_patterns, sheet_name)
    }

    pub fn matches_narrative(&self, sheet_name: &str) -> bool {
        matches_any_pattern(&self.narrative_patterns, sheet_name)
    }

    pub fn matches_table(&self, sheet_name: &str) -> bool {
        matches_any_pattern(&self.table_patterns, sheet_name)
    }

    pub fn matches_chart(&self, sheet_name: &str) -> bool {
        matches_any_pattern(&self.chart_patterns, sheet_name)
    }

    pub fn matches_graphic(&self, sheet_name: &str) -> bool {
        matches_any_pattern(&self.graphic_patterns, sheet_name)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProfileTypography {
    #[serde(default = "default_body_font")]
    pub body_font: String,
    #[serde(default = "default_heading_font")]
    pub heading_font: String,
    #[serde(default = "default_title_font")]
    pub title_font: String,
}

fn default_body_font() -> String {
    "Arial".to_string()
}

fn default_heading_font() -> String {
    "Arial".to_string()
}

fn default_title_font() -> String {
    "Arial Black".to_string()
}

impl Default for ProfileTypography {
    fn default() -> Self {
        Self { body_font: default_body_font(), heading_font: default_heading_font(), title_font: default_title_font() }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProfileTableStyling {
    pub include_column_number_row: bool, // (1) (2) (3) row
    #[serde(default = "default_true")]
    pub include_table_footer: bool, // Footer row (Jumlah / Total)
    pub alternating_row_fills: bool,     // Zebra striping
    pub horizontal_rules_only: bool,     // Minimalist / APA style (no vertical rules)
    pub default_theme_palette: String,   // "bps_teal", "corporate_navy", "monochrome"
}

impl Default for ProfileTableStyling {
    fn default() -> Self {
        Self {
            include_column_number_row: true,
            include_table_footer: true,
            alternating_row_fills: true,
            horizontal_rules_only: false,
            default_theme_palette: "bps_teal".to_string(),
        }
    }
}

// Bundled profile JSON strings
pub static BUNDLED_BPS_DDA: &str = include_str!("../../../../../assets/pubgen/profiles/bps_dda.json");
pub static BUNDLED_BPS_BRS: &str = include_str!("../../../../../assets/pubgen/profiles/bps_brs.json");
pub static BUNDLED_CORPORATE_ANNUAL: &str = include_str!("../../../../../assets/pubgen/profiles/corporate_annual.json");
pub static BUNDLED_ACADEMIC_MONOGRAPH: &str = include_str!("../../../../../assets/pubgen/profiles/academic_monograph.json");

/// Manages built-in and user-custom publication profiles.
pub struct ProfileManager;

impl ProfileManager {
    /// Returns the default profile (BPS DDA).
    pub fn default_profile() -> PubgenProfile {
        Self::parse_or_fallback(BUNDLED_BPS_DDA, "bps_dda")
    }

    /// Returns all 4 built-in publication profiles bundled with DesignCraft.
    pub fn builtin_profiles() -> Vec<PubgenProfile> {
        vec![
            Self::parse_or_fallback(BUNDLED_BPS_DDA, "bps_dda"),
            Self::parse_or_fallback(BUNDLED_BPS_BRS, "bps_brs"),
            Self::parse_or_fallback(BUNDLED_CORPORATE_ANNUAL, "corporate_annual"),
            Self::parse_or_fallback(BUNDLED_ACADEMIC_MONOGRAPH, "academic_monograph"),
        ]
    }

    /// Gets a profile by its ID, searching built-ins first. Falls back safely to BPS DDA.
    pub fn get_profile(id: &str) -> PubgenProfile {
        let id_clean = id.trim();
        for p in Self::builtin_profiles() {
            if p.id.eq_ignore_ascii_case(id_clean) {
                return p;
            }
        }
        // Fallback safely to default DDA
        Self::default_profile()
    }

    /// Parses a JSON string into a profile, validating fields and applying sanitization.
    pub fn from_json(json_str: &str) -> Result<PubgenProfile, String> {
        let mut profile: PubgenProfile = serde_json::from_str(json_str).map_err(|e| format!("Invalid profile JSON: {e}"))?;
        profile.sanitize();
        Ok(profile)
    }

    /// Safely parses bundled profile JSON, falling back to a minimal valid struct if corrupted.
    fn parse_or_fallback(json_str: &str, fallback_id: &str) -> PubgenProfile {
        match Self::from_json(json_str) {
            Ok(p) => p,
            Err(_) => PubgenProfile {
                id: fallback_id.to_string(),
                name: fallback_id.to_string(),
                description: "Fallback publication profile".to_string(),
                sector: ProfileSector::BpsOfficial,
                geometry: ProfileGeometry::default(),
                structure: ProfileStructure::default(),
                nomenclature: ProfileNomenclature::default(),
                sheet_routing: ProfileSheetRouting::default(),
                table_styling: ProfileTableStyling::default(),
                typography: ProfileTypography::default(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_profiles_load_and_validate() {
        let profiles = ProfileManager::builtin_profiles();
        assert_eq!(profiles.len(), 4);

        let dda = &profiles[0];
        assert_eq!(dda.id, "bps_dda");
        assert!(dda.table_styling.include_column_number_row);
        assert_eq!(dda.geometry.margins_mm.top, 20.2);

        let brs = &profiles[1];
        assert_eq!(brs.id, "bps_brs");
        assert!(brs.table_styling.include_column_number_row);
        assert!(!brs.table_styling.include_table_footer);
        assert!(!brs.structure.bilingual);
        assert!(dda.structure.bilingual);
        assert!(brs.geometry.facing_pages, "BPS/BRS print recto-verso");
        assert_eq!(brs.geometry.margins_mm.top, 18.0);

        let corp = &profiles[2];
        assert_eq!(corp.id, "corporate_annual");
        assert!(corp.table_styling.horizontal_rules_only);
        assert_eq!(corp.nomenclature.chapter_prefix, "Section");

        let acad = &profiles[3];
        assert_eq!(acad.id, "academic_monograph");
        assert_eq!(acad.nomenclature.chart_prefix, "Gambar");
    }

    #[test]
    fn test_sheet_routing_matching() {
        let dda = ProfileManager::get_profile("bps_dda");
        assert!(dda.sheet_routing.matches_table("01_01"));
        assert!(dda.sheet_routing.matches_table("Tabel 1"));
        assert!(dda.sheet_routing.matches_divider("DIV_01"));
        assert!(dda.sheet_routing.matches_divider("PEMBATAS"));
        assert!(dda.sheet_routing.matches_chart("C01_01"));
        assert!(dda.sheet_routing.matches_narrative("R01"));

        let corp = ProfileManager::get_profile("corporate_annual");
        assert!(corp.sheet_routing.matches_table("Financial_Summary"));
        assert!(corp.sheet_routing.matches_table("BalanceSheet"));
        assert!(corp.sheet_routing.matches_divider("Section_1"));
        assert!(corp.sheet_routing.matches_narrative("Management_Report"));
    }

    #[test]
    fn test_nomenclature_labels_matching() {
        let dda = ProfileManager::get_profile("bps_dda");
        assert!(dda.nomenclature.matches_source("Sumber: BPS Karawang"));
        assert!(dda.nomenclature.matches_source("Source: BPS"));
        assert!(dda.nomenclature.matches_note("Catatan: Angka sementara"));

        let corp = ProfileManager::get_profile("corporate_annual");
        assert!(corp.nomenclature.matches_source("Source: Internal Audit"));
        assert!(corp.nomenclature.matches_note("Note: In billions of IDR"));
    }

    #[test]
    fn test_sanitize_margins_clamping() {
        let mut margins = MarginsMm {
            top: 2.0,      // below 5.0
            bottom: 200.0, // above 100.0
            inside: 15.0,
            outside: -5.0,
        };
        margins.sanitize();
        assert_eq!(margins.top, 5.0);
        assert_eq!(margins.bottom, 100.0);
        assert_eq!(margins.inside, 15.0);
        assert_eq!(margins.outside, 5.0);
    }

    #[test]
    fn test_profile_application_and_state_sync() {
        let mut st = super::super::state::PubgenState::default();
        let corp = ProfileManager::get_profile("corporate_annual");
        st.apply_profile(corp, true);

        assert_eq!(st.active_profile_id, "corporate_annual");
        assert_eq!(st.card2_col_numbers, false, "Column numbers row should be false for corporate annual");
        assert_eq!(st.card2_style, 3, "Style should be 3 (horizontal rules only) for corporate annual");
        assert!(st.profile_user_locked);
    }

    #[test]
    fn test_auto_detect_profile_logic() {
        use super::super::excel::detect_profile_from_sheets;
        assert_eq!(detect_profile_from_sheets(&["COVER", "DIV", "01_01", "C01_01"]), "bps_dda");
        assert_eq!(detect_profile_from_sheets(&["BRS_UTAMA", "Ringkasan"]), "bps_brs");
        assert_eq!(detect_profile_from_sheets(&["BalanceSheet", "Financial_Summary"]), "corporate_annual");
        assert_eq!(detect_profile_from_sheets(&["Pendahuluan", "Metodologi", "Abstrak"]), "academic_monograph");
    }
}
