//! Pubgen panel: automated CSV document publishing and table insertion.
//!
//! Provides two card tools matching the InDesign-style Properties layout:
//! 1. "Page from CSV": Ingests CSV (columns `type`, `number`, `title`) and synthesizes
//!    multi-page layouts using geometric templates.
//! 2. "Table from CSV": Ingests tabular CSV matrix data and constructs native styled tables.

use std::path::Path;

use egui::{Color32, RichText, Sense, Ui, pos2, vec2};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::DesignApp;
use crate::theme::Tokens;
use crate::widgets::{divider, full_width, outline_button, rule, section, selection_band};

#[path = "pubgen_excel.rs"]
pub mod excel;

#[path = "pubgen_locale.rs"]
pub mod locale;
pub use locale::Locale;

const PANEL_ID: &str = "pubgen_panel_state";

/// Index of the "Modern Horizon" cover style — the one traced from the reference annual-report
/// design, whose palette is navy + orange rather than the amber of the other presets.
const HORIZON_STYLE: usize = 7;

/// Index of the "Business Report" cover style — the navy/orange diagonal cover derived from the
/// business-report reference image, sharing the Horizon palette.
const BUSINESS_STYLE: usize = 8;

/// Index of the "Urban Angle" cover style — the diagonal-wedge cityscape cover derived from
/// `design-6.designcraft`, sharing the Report palette.
const URBAN_STYLE: usize = 9;

/// Theme preset index and colours of the reference annual-report cover (navy + orange).
const REPORT_THEME: usize = 6;

/// Select the palette the reference design uses, so a freshly picked Modern Horizon cover already
/// looks like the reference instead of the amber default.
fn apply_report_palette(st: &mut PubgenState) {
    st.cover_theme_preset = REPORT_THEME;
    st.cover_color_primary = "#082641".into();
    st.cover_color_accent = "#f5871f".into();
    st.cover_color_dark = "#082641".into();
}

/// The traced design-6 palette: deep petrol navy and warm marigold.
fn apply_urban_palette(st: &mut PubgenState) {
    st.cover_theme_preset = REPORT_THEME;
    st.cover_color_primary = "#034460".into();
    st.cover_color_accent = "#fcb91e".into();
    st.cover_color_dark = "#034460".into();
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_one_u32() -> u32 {
    1
}

fn default_toc_title() -> String {
    "Contents".into()
}

fn default_toc_style() -> String {
    "TOC Entry".into()
}

/// The Quick Actions "TOC style" choices: what the table of contents lists.
pub const TOC_KINDS: [&str; 4] = ["TOC Entry", "TOC Table", "TOC Chart", "TOC Paragraph"];

/// The paragraph style of a narrative page's title: the pages the `GXX` and `RXX` sheets produce.
/// The paragraph table of contents lists these pages and nothing else, so a table, chart, divider
/// or publication-note page never shows up in it.
pub const TOC_NARRATIVE_STYLE: &str = "TOC Chapter";

/// The English twin title of a Dual Lang narrative page ("Chapter 01 Geography"): tagged with
/// this style at generation, so the paragraph TOC pairs it with the Indonesian chapter row
/// ("BAB 01 Geografi / Geography") instead of listing it as a page of its own. Only tagged when
/// the pairing checkbox is on with the Dual Lang toggle.
pub const TOC_CHAPTER_EN_STYLE: &str = "TOC Chapter EN";

/// The words the first text box on a page must start with for each TOC style; none lists every
/// paragraph of the tagged style (the original behaviour). A paragraph list takes chapter pages
/// only: `RXX` and `BXX` pages open with their "BAB 01" line, table and chart pages don't.
pub fn toc_kind_words(kind: usize) -> &'static [&'static str] {
    match kind {
        1 => &["Tabel", "Table"],
        2 => &["Grafik", "Chart"],
        3 => &["BAB"],
        _ => &[],
    }
}

/// The title a TOC style fills in when it is picked; `None` leaves the typed title alone.
pub fn toc_kind_title(kind: usize, bilingual: bool) -> Option<&'static str> {
    match (kind, bilingual) {
        (1, false) => Some("Daftar Tabel"),
        (1, true) => Some("Daftar Tabel / List of Tables"),
        (2, false) => Some("Daftar Grafik"),
        (2, true) => Some("Daftar Grafik / List of Charts"),
        (3, false) => Some("Daftar Isi"),
        (3, true) => Some("Daftar Isi / Table of Contents"),
        _ => None,
    }
}

/// The TOC style a stored table of contents was generated with: its entry's `pageTitle` filter.
pub fn toc_kind_of_words(words: &[String]) -> usize {
    let has = |w: &str| words.iter().any(|x| x.eq_ignore_ascii_case(w));
    if has("bab") {
        3
    } else if has("grafik") || has("chart") {
        2
    } else if has("tabel") || has("table") {
        1
    } else {
        0
    }
}

fn default_cover_title() -> String {
    "KARAWANG TIMUR".into()
}

/// The second headline line of the cover: the templates give it its own frame and style.
fn default_cover_title2() -> String {
    "Title 2".into()
}

fn default_cover_year() -> String {
    "2026".into()
}

fn default_cover_volume() -> String {
    "Volume 23 2026".into()
}

fn default_cover_catalogue() -> String {
    "1102001.3215113".into()
}

fn default_cover_code_num() -> String {
    "2828-7436".into()
}

fn default_cover_primary() -> String {
    "#0072bc".into()
}

fn default_cover_accent() -> String {
    "#f5ab19".into()
}

fn default_cover_dark() -> String {
    "#122647".into()
}

fn default_cover_skyline() -> String {
    "outputs/city_skyline.jpg".into()
}

fn default_cover_landscape() -> String {
    "outputs/city_landscape.jpg".into()
}

fn default_cover_logo() -> String {
    "outputs/bps_logo.png".into()
}

fn default_cover_agency_text() -> String {
    "BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG".into()
}

fn default_hf_style() -> String {
    "Modern Minimal".into()
}

fn default_hf_offset() -> f64 {
    24.0
}

fn default_hf_rule_weight() -> f64 {
    0.5
}

fn default_hf_accent_bar_height() -> f64 {
    4.0
}

fn default_hf_page_token() -> String {
    "{page}".into()
}

fn default_hf_color_primary() -> String {
    "#111827".into()
}

fn default_hf_color_accent() -> String {
    "#3b82f6".into()
}

fn default_hf_color_dark() -> String {
    "#1f2937".into()
}

fn default_hf_color_rule() -> String {
    "#e5e7eb".into()
}

fn default_hf_color_text() -> String {
    "#6b7280".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PubgenState {
    // Card 1: Page from CSV
    pub card1_path: String,
    pub card1_records: Vec<Vec<String>>,
    pub card1_headers: Vec<String>,
    pub card1_type_col: usize,
    pub card1_num_col: usize,
    pub card1_title_col: usize,
    pub card1_preset: usize,
    pub card1_range_all: bool,
    pub card1_range_from: usize,
    pub card1_range_to: usize,
    pub card1_append: bool,
    /// Prefix the tagged (TOC) paragraph with `type number` — the page title line then reads
    /// "Tabel 01.01. Jarak …" and so does its table-of-contents entry.
    #[serde(default = "default_true")]
    pub card1_toc_prefix: bool,
    /// Auto-insert table for each generated page from the 4th column (`file`) if available.
    #[serde(default = "default_true")]
    pub card1_auto_insert_tables: bool,
    /// Indent wrapped lines of TOC entries (hanging indent) so multi-line entries wrap nicely.
    #[serde(default = "default_true")]
    pub toc_hanging_indent: bool,
    pub card1_status: Option<(String, bool)>,
    #[serde(default)]
    pub card1_delim: Option<char>,
    /// Optional template path for Card 1 (used when preset == 6 "Modular Template", or leave blank for canonical auto-dispatch)
    #[serde(default)]
    pub card1_template_path: String,

    // Card 2: Table from CSV
    pub card2_path: String,
    pub card2_matrix: Vec<Vec<String>>,
    /// List of available table CSV paths (e.g. from multi-sheet Excel conversion)
    #[serde(default)]
    pub card2_sheet_tables: Vec<String>,
    #[serde(default)]
    pub card2_sheet_idx: usize,
    pub card2_first_row_header: bool,
    pub card2_col_numbers: bool,
    pub card2_table_footer: bool,
    #[serde(default = "default_true")]
    pub card2_row_conditions: bool,
    pub card2_style: usize,
    pub card2_placement: usize,
    pub card2_status: Option<(String, bool)>,
    #[serde(default)]
    pub card2_delim: Option<char>,

    // Card 3: Quick Actions
    #[serde(default)]
    pub card3_status: Option<(String, bool)>,
    /// Path to RTF file for Page from RTF in Quick Actions
    #[serde(default)]
    pub rtf_path: String,
    /// Path to image file for Page from Image in Quick Actions
    #[serde(default)]
    pub image_path: String,
    /// Page from Image mode: false = image object (fit inside page margins), true = page
    /// background (full page bounds on the selected page, sent to back)
    #[serde(default)]
    pub image_as_background: bool,

    // Card 1 "TOC Line" preset and Card 3 "Add TOC": the paragraph style that marks TOC entries.
    #[serde(default = "default_toc_style")]
    pub toc_style: String,
    #[serde(default = "default_toc_title")]
    pub toc_title: String,
    #[serde(default = "default_false")]
    pub toc_include_self: bool,
    /// Pair bilingual headings (ID / EN) of the same style on a Dual Lang page into one TOC row.
    #[serde(default)]
    pub toc_pair_bilingual: bool,
    #[serde(default)]
    pub toc_layout_style: usize,
    /// Quick Actions "TOC style": 0 TOC Entry (every tagged paragraph, the original behaviour),
    /// 1 TOC Table (only pages whose first text box starts with "Tabel"/"Table"), 2 TOC Chart
    /// (only "Grafik"/"Chart").
    #[serde(default)]
    pub toc_kind: usize,
    #[serde(default = "default_one_u32")]
    pub card3_renumber_start: u32,
    #[serde(default)]
    pub card3_renumber_style: usize,

    /// Last generated CSV files (filename, content) for ZIP export
    #[serde(skip)]
    pub generated_csv_files: Vec<(String, String)>,
    /// Suggested ZIP filename (e.g. "BAB_02_2025_csvs.zip")
    #[serde(default)]
    pub generated_zip_name: String,
    /// Cached ZIP archive bytes
    #[serde(skip)]
    pub generated_zip_bytes: Vec<u8>,

    /// Automatically run "Page from CSV" and insert tables after Excel conversion
    #[serde(default = "default_true")]
    pub excel_auto_generate: bool,
    /// Whether the last converted workbook had a `COVER` sheet
    #[serde(skip)]
    pub excel_had_cover: bool,

    /// Status message for header actions (e.g. Set Page Margin)
    #[serde(default)]
    pub header_status: Option<(String, bool)>,

    /// Target locale for single-language generation mode.
    #[serde(default)]
    pub locale: Locale,
    /// Bilingual / Dual-language generation mode: false = Single Lang (default), true = Dual Lang.
    #[serde(default)]
    pub bilingual_output: bool,
    /// Document-level language setting: "" = leave run language untouched (default), else language name e.g. "Indonesian".
    #[serde(default)]
    pub doc_language: String,

    // Card: Publication Cover (before Quick Actions)
    #[serde(default)]
    pub cover_style: usize,
    #[serde(default = "default_cover_title")]
    pub cover_title: String,
    /// The second headline line. The cover templates carry it in their own frame (`title#2`),
    /// styled smaller and in another colour than the headline above it.
    #[serde(default = "default_cover_title2")]
    pub cover_title2: String,
    #[serde(default)]
    pub cover_title_en: String,
    #[serde(default = "default_cover_year")]
    pub cover_year: String,
    #[serde(default = "default_cover_volume")]
    pub cover_volume: String,
    #[serde(default = "default_cover_catalogue")]
    pub cover_catalogue: String,
    #[serde(default)]
    pub cover_code_type: usize,
    #[serde(default = "default_cover_code_num")]
    pub cover_code_num: String,
    #[serde(default)]
    pub cover_theme_preset: usize,
    #[serde(default = "default_cover_primary")]
    pub cover_color_primary: String,
    #[serde(default = "default_cover_accent")]
    pub cover_color_accent: String,
    #[serde(default = "default_cover_dark")]
    pub cover_color_dark: String,
    #[serde(default = "default_cover_skyline")]
    pub cover_image_skyline: String,
    #[serde(default = "default_cover_landscape")]
    pub cover_image_landscape: String,
    #[serde(default = "default_cover_logo")]
    pub cover_image_logo: String,
    #[serde(default = "default_cover_agency_text")]
    pub cover_agency_text: String,
    #[serde(default)]
    pub cover_status: Option<(String, bool)>,

    // Card: Header & Footer Template (between Cover and Quick Actions)
    #[serde(default = "default_hf_style")]
    pub hf_style: String,
    #[serde(default)]
    pub hf_custom_name: String,
    #[serde(default)]
    pub hf_sided: usize,
    /// 0 = all pages, 1 = one section, 2 = a page range.
    #[serde(default)]
    pub hf_scope: usize,
    /// 0-based index into the document's sections (engine expects the index, UI shows 1-based).
    #[serde(default)]
    pub hf_scope_section: usize,
    #[serde(default)]
    pub hf_scope_start: usize,
    #[serde(default)]
    pub hf_scope_end: usize,
    #[serde(default = "default_hf_offset")]
    pub hf_header_offset: f64,
    #[serde(default = "default_hf_offset")]
    pub hf_footer_offset: f64,
    #[serde(default)]
    pub hf_header_single: String,
    #[serde(default)]
    pub hf_header_left: String,
    #[serde(default)]
    pub hf_header_right: String,
    #[serde(default = "default_hf_page_token")]
    pub hf_footer_single: String,
    #[serde(default = "default_hf_page_token")]
    pub hf_footer_left: String,
    #[serde(default = "default_hf_page_token")]
    pub hf_footer_right: String,
    #[serde(default)]
    pub hf_header_rule: bool,
    #[serde(default = "default_hf_rule_weight")]
    pub hf_header_rule_weight: f64,
    #[serde(default = "default_true")]
    pub hf_footer_rule: bool,
    #[serde(default = "default_hf_rule_weight")]
    pub hf_footer_rule_weight: f64,
    #[serde(default)]
    pub hf_top_accent_bar: bool,
    #[serde(default = "default_hf_accent_bar_height")]
    pub hf_top_accent_bar_height: f64,
    #[serde(default = "default_true")]
    pub hf_top_accent_bar_bleed: bool,
    #[serde(default = "default_hf_color_primary")]
    pub hf_color_primary: String,
    #[serde(default = "default_hf_color_accent")]
    pub hf_color_accent: String,
    #[serde(default = "default_hf_color_dark")]
    pub hf_color_dark: String,
    #[serde(default = "default_hf_color_rule")]
    pub hf_color_rule: String,
    #[serde(default = "default_hf_color_text")]
    pub hf_color_text: String,
    #[serde(default)]
    pub hf_status: Option<(String, bool)>,
    #[serde(skip)]
    pub hf_initialized: bool,
    #[serde(skip)]
    pub hf_show_save_row: bool,
}

impl Default for PubgenState {
    fn default() -> Self {
        Self {
            card1_path: String::new(),
            card1_records: Vec::new(),
            card1_headers: Vec::new(),
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            card1_range_all: true,
            card1_range_from: 1,
            card1_range_to: 10,
            card1_append: true,
            card1_toc_prefix: true,
            card1_auto_insert_tables: true,
            toc_hanging_indent: true,
            card1_status: None,
            card1_delim: None,
            card1_template_path: String::new(),
            card2_path: String::new(),
            card2_matrix: Vec::new(),
            card2_sheet_tables: Vec::new(),
            card2_sheet_idx: 0,
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_row_conditions: true,
            card2_style: 3,     // Horizontal Rules
            card2_placement: 2, // At page
            card2_status: None,
            card2_delim: None,
            card3_status: None,
            rtf_path: String::new(),
            image_path: String::new(),
            image_as_background: false,
            toc_style: default_toc_style(),
            toc_title: default_toc_title(),
            toc_include_self: false,
            toc_pair_bilingual: false,
            toc_layout_style: 0,
            toc_kind: 0,
            card3_renumber_start: 1,
            card3_renumber_style: 0,
            generated_csv_files: Vec::new(),
            generated_zip_name: String::new(),
            generated_zip_bytes: Vec::new(),
            cover_style: 2,
            cover_title: default_cover_title(),
            cover_title2: default_cover_title2(),
            cover_title_en: String::new(),
            cover_year: default_cover_year(),
            cover_volume: default_cover_volume(),
            cover_catalogue: default_cover_catalogue(),
            cover_code_type: 0,
            cover_code_num: default_cover_code_num(),
            cover_theme_preset: 0,
            cover_color_primary: default_cover_primary(),
            cover_color_accent: default_cover_accent(),
            cover_color_dark: default_cover_dark(),
            cover_image_skyline: default_cover_skyline(),
            cover_image_landscape: default_cover_landscape(),
            cover_image_logo: default_cover_logo(),
            cover_agency_text: default_cover_agency_text(),
            cover_status: None,
            excel_auto_generate: true,
            excel_had_cover: false,
            header_status: None,
            locale: Locale::Id,
            bilingual_output: false,
            doc_language: String::new(),
            hf_style: default_hf_style(),
            hf_custom_name: String::new(),
            hf_sided: 0,
            hf_scope: 0,
            hf_scope_section: 0,
            hf_scope_start: 1,
            hf_scope_end: 1,
            hf_header_offset: default_hf_offset(),
            hf_footer_offset: default_hf_offset(),
            hf_header_single: String::new(),
            hf_header_left: String::new(),
            hf_header_right: String::new(),
            hf_footer_single: default_hf_page_token(),
            hf_footer_left: default_hf_page_token(),
            hf_footer_right: default_hf_page_token(),
            hf_header_rule: false,
            hf_header_rule_weight: default_hf_rule_weight(),
            hf_footer_rule: true,
            hf_footer_rule_weight: default_hf_rule_weight(),
            hf_top_accent_bar: false,
            hf_top_accent_bar_height: default_hf_accent_bar_height(),
            hf_top_accent_bar_bleed: true,
            hf_color_primary: default_hf_color_primary(),
            hf_color_accent: default_hf_color_accent(),
            hf_color_dark: default_hf_color_dark(),
            hf_color_rule: default_hf_color_rule(),
            hf_color_text: default_hf_color_text(),
            hf_status: None,
            hf_initialized: false,
            hf_show_save_row: false,
        }
    }
}

/// Returns the custom document margins:
/// 1. From existing document pages with local or custom (non-36pt, non-0) margins
/// 2. From parent pages with custom (non-36pt, non-0) margins
/// 3. From any spread page with non-zero margins
/// 4. From any parent page with non-zero margins
/// 5. Fallback: Standard BPS custom margins (Top 20.20 mm, Bottom 20 mm, Inside 19 mm, Outside 15 mm)
pub fn detect_document_margins(doc: &designcraft_doc::Document) -> designcraft_doc::Margins {
    const PT_PER_MM: f64 = 72.0 / 25.4;
    let bps_margins =
        designcraft_doc::Margins { top: 20.20 * PT_PER_MM, bottom: 20.0 * PT_PER_MM, inside: 19.0 * PT_PER_MM, outside: 15.0 * PT_PER_MM };

    // 1. Look for existing document pages with local margins or non-default margins (excluding COVER/DIV with 0 margins)
    if let Some(m) = doc
        .spreads
        .iter()
        .flat_map(|sp| &sp.pages)
        .find(|pg| pg.margins.top > 0.0 && pg.margins.bottom > 0.0 && (pg.local_margins || pg.margins != designcraft_doc::Margins::uniform(36.0)))
        .map(|pg| pg.margins)
    {
        return m;
    }

    // 2. Look for parents with non-default margins
    if let Some(m) = doc
        .parents
        .iter()
        .flat_map(|sp| &sp.pages)
        .find(|pg| pg.margins.top > 0.0 && pg.margins.bottom > 0.0 && pg.margins != designcraft_doc::Margins::uniform(36.0))
        .map(|pg| pg.margins)
    {
        return m;
    }

    // 3. Fallback: Standard BPS custom margins (Top 20.20 mm, Bottom 20 mm, Inside 19 mm, Outside 15 mm)
    bps_margins
}

/// Sets document margins across all spreads and parents:
/// - Top: 20.20 mm
/// - Bottom: 20.00 mm
/// - Inside: 19.00 mm
/// - Outside: 15.00 mm
pub fn execute_set_page_margins(app: &mut DesignApp) -> Result<String, String> {
    const PT_PER_MM: f64 = 72.0 / 25.4;
    let top_pt = 20.20 * PT_PER_MM;
    let bottom_pt = 20.0 * PT_PER_MM;
    let inside_pt = 19.0 * PT_PER_MM;
    let outside_pt = 15.0 * PT_PER_MM;

    // Record which pages are COVER or DIV (already 0 margins) before layout.documentSetup
    let zero_margin_pages: Vec<usize> = app
        .session
        .active()
        .map(|s| {
            (0..s.doc.page_count())
                .filter(|&p| {
                    if let Some((si, pi)) = s.doc.page_loc(p)
                        && let Some(pg) = s.doc.spreads.get(si).and_then(|sp| sp.pages.get(pi))
                    {
                        return pg.margins.top == 0.0 && pg.margins.bottom == 0.0;
                    }
                    false
                })
                .collect()
        })
        .unwrap_or_default();

    app.run(
        "layout.documentSetup",
        json!({
            "margins": {
                "top": top_pt,
                "bottom": bottom_pt,
                "inside": inside_pt,
                "outside": outside_pt
            },
            "adjustLayout": true
        }),
    )?;

    // Keep COVER and DIV pages adjusted to Page Size (margins = 0)
    let _ = app.session.edit(|doc, _| {
        let mut abs_p = 0usize;
        for sp in &mut doc.spreads {
            let sp = std::sync::Arc::make_mut(sp);
            for pg in &mut sp.pages {
                let pb = pg.bounds();
                let is_cover_or_div = zero_margin_pages.contains(&abs_p)
                    || sp.items.iter().any(|it| {
                        let ib = it.bounds();
                        ib.x0 < pb.x1
                            && ib.x1 > pb.x0
                            && (it.label.contains("cover")
                                || it.label.contains("divider")
                                || it.label.starts_with("#slot:text:level")
                                || it.label.starts_with("#slot:text:chapter_prefix")
                                || it.label.starts_with("#slot:chapter_prefix")
                                || it.label.starts_with("#slot:text:first_word"))
                    });
                if is_cover_or_div {
                    pg.margins = designcraft_doc::Margins::uniform(0.0);
                    pg.local_margins = true;
                }
                abs_p += 1;
            }
            sp.relayout();
        }
        Ok(serde_json::Value::Null)
    });

    // If a header/footer is already applied to the document, re-apply it so its geometry,
    // frames, and hairline rules immediately follow the new page margins.
    if app.session.active().and_then(|s| s.doc.header_footer.as_ref()).is_some() {
        let _ = app.run(
            "layout.headerFooter.apply",
            json!({
                "margins": {
                    "top": top_pt,
                    "bottom": bottom_pt,
                    "inside": inside_pt,
                    "outside": outside_pt
                }
            }),
        );
    }

    Ok("Page margins set: Top 20.20 mm, Bottom 20 mm, Inside 19 mm, Outside 15 mm".to_string())
}

fn pubgen_header_band(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    ui.horizontal(|ui| {
        ui.set_min_height(33.0);
        let g = crate::rtl::plain(ui.ctx(), "Pubgen", crate::theme::semibold(11.5), t.text_strong);
        let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 10.0, 33.0), Sense::hover());
        ui.painter().galley(pos2(r.min.x, r.center().y - g.size().y / 2.0), g, t.text_strong);

        let lang = app.ui.language.clone();
        let tip = "Set document margins:\nTop: 20.20 mm\nBottom: 20 mm\nInside: 19 mm\nOutside: 15 mm";
        let resp = outline_button(ui, crate::i18n::tr(&lang, "Set Page Margin"), 120.0).on_hover_text(tip);
        if resp.clicked() {
            match execute_set_page_margins(app) {
                Ok(msg) => st.header_status = Some((msg, true)),
                Err(err) => st.header_status = Some((err, false)),
            }
        }

        let mode_raw = if st.bilingual_output { "Dual Lang" } else { "Single Lang" };
        let mode_label = crate::i18n::tr(&lang, mode_raw);
        let mode_tip = "Single Lang: generate the active output language only.\n\
                        Dual Lang (Bilingual): generate both languages, per the sheet pattern rules (see docs/pubgen-guide.md).";
        let mode_resp = outline_button(ui, mode_label, 90.0).on_hover_text(mode_tip);
        if mode_resp.clicked() {
            let was_bilingual = st.bilingual_output;
            st.bilingual_output = !st.bilingual_output;
            let mode_str = if st.bilingual_output { "Dual Lang (Bilingual)" } else { "Single Lang" };
            st.header_status = Some((format!("Mode: {mode_str}"), true));
            if let Some(old_title) = toc_kind_title(st.toc_kind, was_bilingual)
                && (st.toc_title.trim().is_empty() || st.toc_title.trim() == old_title)
                && let Some(new_title) = toc_kind_title(st.toc_kind, st.bilingual_output)
            {
                st.toc_title = new_title.to_string();
            }
        }
    });
    rule(ui);

    if let Some((msg, ok)) = &st.header_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(10.5).color(color)).wrap());
    }
}

pub fn show(app: &mut DesignApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    if app.session.active().is_none() {
        selection_band(ui, "Pubgen");
        crate::rtl::label(ui, RichText::new(crate::i18n::tr(&app.ui.language, "No document open")).color(t.text_dim));
        return;
    }

    let max_w = ui.available_width();
    ui.set_max_width(max_w);
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    ui.spacing_mut().item_spacing = vec2(6.0, 5.0);
    let state_id = egui::Id::new(PANEL_ID);
    let mut st: PubgenState = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();

    pubgen_header_band(app, ui, &mut st, &t);

    // Card 1: Page from CSV
    card1_ui(app, ui, &mut st, &t);

    divider(ui);
    rule(ui);

    // Card 2: Table from CSV
    card2_ui(app, ui, &mut st, &t);

    divider(ui);
    rule(ui);

    // Card: Publication Cover
    card_cover_ui(app, ui, &mut st, &t);

    divider(ui);
    rule(ui);

    // Card: Header & Footer Template
    card_header_footer_ui(app, ui, &mut st, &t);

    divider(ui);
    rule(ui);

    // Card 3: Quick Actions
    card3_quick_actions_ui(app, ui, &mut st, &t);

    ui.data_mut(|d| d.insert_temp(state_id, st));
}

fn card1_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Page from CSV"), true);

    // Active catalog indicator (populated from Excel conversion)
    let (file_label, color) = if st.card1_path.is_empty() {
        (crate::i18n::tr(&app.ui.language, "No file selected").to_string(), t.text_dim)
    } else {
        let name = std::path::Path::new(&st.card1_path).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| st.card1_path.clone());
        (format!("{name} ({} records)", st.card1_records.len()), t.text)
    };
    ui.add(egui::Label::new(RichText::new(file_label).size(11.0).color(color)).truncate());

    // Column detection badges
    if !st.card1_headers.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Columns:").size(10.5).color(t.text_dim));
            if let Some(d) = st.card1_delim {
                badge(ui, t, "delim", delimiter_name(d));
            }
            badge(ui, t, "type", st.card1_headers.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("col 0"));
            badge(ui, t, "number", st.card1_headers.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("col 1"));
            badge(ui, t, "title", st.card1_headers.get(st.card1_title_col.min(2)).map(String::as_str).unwrap_or("col 2"));
            if let Some(f_hdr) = st.card1_headers.get(3) {
                badge(ui, t, "file", f_hdr);
            }
            if let Some(tpl_hdr) = st.card1_headers.get(4) {
                badge(ui, t, "template", tpl_hdr);
            }
            if let Some(idx) = st.card1_headers.iter().position(|h| h.trim().eq_ignore_ascii_case("title_en"))
                && let Some(h) = st.card1_headers.get(idx)
            {
                badge(ui, t, "title_en", h);
            }
            if let Some(idx) = st.card1_headers.iter().position(|h| h.trim().eq_ignore_ascii_case("file_en"))
                && let Some(h) = st.card1_headers.get(idx)
            {
                badge(ui, t, "file_en", h);
            }
        });
    }

    if st.card1_headers.len() >= 4 || st.card1_records.iter().any(|r| r.len() >= 4) {
        ui.checkbox(&mut st.card1_auto_insert_tables, "Auto-insert table from 'file' col (e.g. 02_01.csv)");
    }

    // Layout Preset Selector
    ui.horizontal(|ui| {
        ui.label(RichText::new("Layout:").size(11.0).color(t.text));
        let presets = [
            "1 Row · 3 Columns",
            "1 Row · 3 Equal Cols",
            "Editorial Chapter (Stacked)",
            "Technical Spec Card",
            "Minimal Zine",
            "TOC Line (1 paragraph/record)",
            "Modular Template",
        ];
        let cur = presets.get(st.card1_preset).copied().unwrap_or("1 Row · 3 Columns");
        let combo_w = (ui.available_width() - 4.0).clamp(80.0, 180.0);
        egui::ComboBox::from_id_salt("pubgen_card1_preset").selected_text(cur).width(combo_w).show_ui(ui, |ui| {
            for (i, p) in presets.iter().enumerate() {
                ui.selectable_value(&mut st.card1_preset, i, *p);
            }
        });
    });

    if st.card1_preset == 6 {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Template:").size(11.0).color(t.text));
            let w = (ui.available_width() - 4.0).clamp(80.0, 240.0);
            ui.add_sized([w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.card1_template_path))
                .on_hover_text("Path to .designcraft template file (leave blank for canonical auto-dispatch)");
        });
    }

    // Every preset tags its title paragraph with this paragraph style; a TOC entry's text is the
    // paragraph's text, so "prefix" makes the entry read "Tabel 01.01. Jarak …".
    ui.horizontal(|ui| {
        ui.label(RichText::new("TOC tag style:").size(11.0).color(t.text));
        let w = (ui.available_width() - 4.0).clamp(80.0, 180.0);
        ui.add_sized([w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.toc_style))
            .on_hover_text("Paragraph style that tags each generated page's title, so the table of contents lists that page");
    });
    if st.card1_preset != 5 {
        ui.checkbox(&mut st.card1_toc_prefix, "TOC entry: prefix with type + number");
    }

    // Record Scope
    ui.horizontal(|ui| {
        ui.label(RichText::new("Scope:").size(11.0).color(t.text));
        ui.radio_value(&mut st.card1_range_all, true, "All");
        ui.radio_value(&mut st.card1_range_all, false, "Range");
        if !st.card1_range_all {
            ui.add(egui::DragValue::new(&mut st.card1_range_from).range(1..=st.card1_records.len().max(1)).prefix("From: "));
            ui.add(egui::DragValue::new(&mut st.card1_range_to).range(1..=st.card1_records.len().max(1)).prefix("To: "));
        }
    });

    ui.checkbox(&mut st.card1_append, "Append to document");

    // Action button
    let can_gen = !st.card1_records.is_empty();
    if ui.add_enabled(can_gen, egui::Button::new("Generate Pages")).clicked() {
        let res = execute_generate_pages(app, st);
        st.card1_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(err) => (err, false),
        });
    }

    if let Some((msg, ok)) = &st.card1_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

fn card2_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Table from CSV"), true);

    // Sheet table selector (if populated by Excel conversion)
    if !st.card2_sheet_tables.is_empty() {
        let mut new_selection = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Sheet table:").size(11.0).color(t.text));
            let cur_name = st
                .card2_sheet_tables
                .get(st.card2_sheet_idx)
                .and_then(|p| std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "Select sheet".into());
            let combo_w = (ui.available_width() - 4.0).clamp(100.0, 240.0);
            egui::ComboBox::from_id_salt("pubgen_card2_sheet_selector").selected_text(cur_name).width(combo_w).show_ui(ui, |ui| {
                for (idx, p) in st.card2_sheet_tables.iter().enumerate() {
                    let name = std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.clone());
                    let is_sel = idx == st.card2_sheet_idx;
                    if ui.selectable_label(is_sel, name).clicked() {
                        new_selection = Some(idx);
                    }
                }
            });
        });
        if let Some(idx) = new_selection
            && let Some(p) = st.card2_sheet_tables.get(idx).cloned()
        {
            st.card2_sheet_idx = idx;
            st.card2_path = p.clone();
            load_card2_csv(st, &p);
        }
    }

    // Active table indicator (populated from Excel conversion)
    let (file_label, color) = if st.card2_path.is_empty() {
        (crate::i18n::tr(&app.ui.language, "No file selected").to_string(), t.text_dim)
    } else {
        let name = std::path::Path::new(&st.card2_path).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| st.card2_path.clone());
        let rows = st.card2_matrix.len();
        let cols = st.card2_matrix.first().map(Vec::len).unwrap_or(0);
        (format!("{name} ({}×{} grid)", rows, cols), t.text)
    };
    ui.add(egui::Label::new(RichText::new(file_label).size(11.0).color(color)).truncate());

    ui.checkbox(&mut st.card2_first_row_header, "First row is header");
    ui.checkbox(&mut st.card2_col_numbers, "Add column numbers (1), (2)…");
    ui.checkbox(&mut st.card2_table_footer, "Table footer (last row)");
    ui.checkbox(&mut st.card2_row_conditions, "Condition rules (Jumlah, Tahun, Kab, Kec…)");

    if !st.card2_matrix.is_empty()
        && let Some(d) = st.card2_delim
    {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Delimiter:").size(10.5).color(t.text_dim));
            badge(ui, t, "delim", delimiter_name(d));
        });
    }

    if st.card2_first_row_header {
        let merge_count = st.card2_matrix.first().map(|r| r.iter().filter(|c| c.contains("--")).count()).unwrap_or(0);
        if merge_count > 1 {
            let num_stacks = st.card2_matrix.first().map(|r| r.iter().map(|c| c.splitn(3, "--").count()).max().unwrap_or(1).clamp(1, 3)).unwrap_or(1);
            let label = if num_stacks > 1 {
                format!("✓ Auto-detected {num_stacks}-stack header columns (\"--\")")
            } else {
                "✓ Auto-detected merged header columns (\"--\")".to_string()
            };
            ui.add(egui::Label::new(RichText::new(label).size(10.5).color(t.accent)).wrap());
        }
    }

    if let Some(fn_text) = detect_footnote_row(&st.card2_matrix) {
        let display_fn = if fn_text.len() > 38 { format!("{}…", &fn_text[..38]) } else { fn_text };
        ui.add(egui::Label::new(RichText::new(format!("✓ Auto-detected table note: \"{display_fn}\"")).size(10.5).color(t.accent)).wrap());
    }

    if st.card2_row_conditions {
        let cond_rows = detect_condition_rows(&st.card2_matrix, st.card2_first_row_header);
        if !cond_rows.is_empty() {
            let label = if cond_rows.len() == 1 {
                "✓ Auto-detected 1 summary/break row with rule formatting (\"Jumlah\", etc.)".to_string()
            } else {
                format!("✓ Auto-detected {} summary/break rows with rule formatting (\"Jumlah\", etc.)", cond_rows.len())
            };
            ui.add(egui::Label::new(RichText::new(label).size(10.5).color(t.accent)).wrap());
        }
    }

    // Table Style Selector
    ui.horizontal(|ui| {
        ui.label(RichText::new("Style:").size(11.0).color(t.text));
        let styles = ["Basic Grid", "Corporate Navy", "Minimalist", "Horizontal Rules"];
        let cur = styles.get(st.card2_style).copied().unwrap_or("Basic Grid");
        let combo_w = (ui.available_width() - 4.0).clamp(80.0, 180.0);
        egui::ComboBox::from_id_salt("pubgen_card2_style").selected_text(cur).width(combo_w).show_ui(ui, |ui| {
            for (i, s) in styles.iter().enumerate() {
                ui.selectable_value(&mut st.card2_style, i, *s);
            }
        });
    });

    // Placement
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Place:").size(11.0).color(t.text));
        ui.radio_value(&mut st.card2_placement, 0, "At cursor");
        ui.radio_value(&mut st.card2_placement, 1, "New frame");
        ui.radio_value(&mut st.card2_placement, 2, "At page");
    });
    if st.card2_placement == 2 {
        let cur_page = target_page_index(app);
        let label = app.session.page_label(cur_page);
        ui.add(egui::Label::new(RichText::new(format!("Target: Page {label} (below title)")).size(10.5).color(t.accent)).wrap());
    }

    // Action button
    let can_insert = !st.card2_matrix.is_empty();
    if ui.add_enabled(can_insert, egui::Button::new("Insert Table")).clicked() {
        if std::path::Path::new(&st.card2_path).exists() {
            load_card2_csv(st, &st.card2_path.clone());
        }
        let res = execute_insert_table(app, st);
        st.card2_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(err) => (err, false),
        });
    }

    if let Some((msg, ok)) = &st.card2_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

fn adjust_hex(hex: &str, factor: f32) -> String {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() != 6 {
        return hex.to_string();
    }
    let r = u8::from_str_radix(clean.get(0..2).unwrap_or("00"), 16).unwrap_or(128);
    let g = u8::from_str_radix(clean.get(2..4).unwrap_or("00"), 16).unwrap_or(128);
    let b = u8::from_str_radix(clean.get(4..6).unwrap_or("00"), 16).unwrap_or(128);
    let clamp = |c: f32| c.clamp(0.0, 255.0) as u8;
    let r2 = clamp(r as f32 * factor);
    let g2 = clamp(g as f32 * factor);
    let b2 = clamp(b as f32 * factor);
    format!("#{r2:02x}{g2:02x}{b2:02x}")
}

fn style_thumbnail_texture(ctx: &egui::Context, style_idx: usize) -> Option<egui::TextureHandle> {
    let key = egui::Id::new(("pubgen_cover_style_thumbnail_tex", style_idx));
    if let Some(tex) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return Some(tex);
    }
    let (primary_name, alt_name) = match style_idx {
        0 => ("wave_curves.png", "karawang_timur_cover_2026.png"),
        1 => ("geometric_medallion.png", "karawang_timur_geometric_cover.png"),
        2 => ("folded_origami.png", "karawang_timur_origami_cover.png"),
        3 => ("chevron_medallion.png", "karawang_timur_chevron_cover.png"),
        4 => ("karawang_timur_hexagon_cover.png", "honeycomb_hexagon.png"),
        5 => ("corporate_headline.png", "karawang_timur_corporate_cover.png"),
        6 => ("diagonal_arrow.png", "karawang_timur_diagonal_cover.png"),
        7 => ("modern_horizon.png", "karawang_timur_modern_horizon_cover.png"),
        8 => ("business_report.png", "karawang_timur_business_cover.png"),
        9 => ("urban_angle.png", "urban_angle.png"),
        _ => return None,
    };
    let candidates = [
        format!("plan/pubgen/samples/thumb/{primary_name}"),
        format!("samples/thumb/{primary_name}"),
        format!("outputs/{primary_name}"),
        format!("outputs/{alt_name}"),
        primary_name.to_string(),
        alt_name.to_string(),
        format!("../outputs/{primary_name}"),
        format!("../outputs/{alt_name}"),
    ];
    let mut bytes = None;
    for cand in &candidates {
        if let Ok(b) = std::fs::read(cand) {
            bytes = Some(b);
            break;
        }
    }
    let bytes = bytes?;
    let pm = designcraft_render::decode_pixmap_page(&bytes, 0)?;
    let ci = egui::ColorImage::from_rgba_premultiplied([pm.width() as usize, pm.height() as usize], pm.data_as_u8_slice());
    let fitted = crate::widgets::fit_texture(ci, 300);
    let tex = ctx.load_texture(format!("pubgen_cover_thumb_{style_idx}"), fitted, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(key, tex.clone()));
    Some(tex)
}

fn paint_fallback_cover_thumb(ui: &mut Ui, style_idx: usize, w: f32, h: f32, t: &Tokens) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 4.0, Color32::from_rgb(250, 250, 252));
    p.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Outside);

    match style_idx {
        0 => {
            let bottom_rect = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.45), rect.max);
            p.rect_filled(bottom_rect, egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, Color32::from_rgb(18, 38, 71));
            let center = rect.center() + vec2(0.0, -h * 0.02);
            p.circle_filled(center, w * 0.23, Color32::from_rgb(0, 114, 188));
            p.circle_stroke(center, w * 0.25, egui::Stroke::new(2.0, Color32::from_rgb(245, 171, 25)));
        }
        1 => {
            let band = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.28), rect.min + vec2(w, h * 0.72));
            p.rect_filled(band, 0.0, Color32::from_rgb(0, 114, 188));
            let c = rect.center() + vec2(w * 0.1, h * 0.08);
            p.circle_filled(c, w * 0.22, Color32::WHITE);
            p.circle_stroke(c, w * 0.22, egui::Stroke::new(2.0, Color32::from_rgb(245, 171, 25)));
        }
        2 => {
            let top_right = egui::Rect::from_min_max(rect.min + vec2(w * 0.4, 0.0), rect.min + vec2(w, h * 0.38));
            p.rect_filled(top_right, egui::CornerRadius { nw: 0, ne: 4, sw: 0, se: 0 }, Color32::from_rgb(100, 150, 200));
            let gold_facet = egui::Rect::from_min_max(rect.min + vec2(0.0, 0.0), rect.min + vec2(w * 0.4, h * 0.2));
            p.rect_filled(gold_facet, egui::CornerRadius { nw: 4, ne: 0, sw: 0, se: 0 }, Color32::from_rgb(245, 171, 25));
            let dark_facet = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.2), rect.min + vec2(w * 0.5, h * 0.45));
            p.rect_filled(dark_facet, 0.0, Color32::from_rgb(18, 38, 71));
        }
        3 => {
            // Style 3 (Chevron & Medallion): Downward chevron + horseshoe ring + footer + center year
            let chevron_pts = vec![
                rect.min,
                rect.min + vec2(w, 0.0),
                rect.min + vec2(w, h * 0.18),
                rect.min + vec2(w * 0.5, h * 0.53),
                rect.min + vec2(0.0, h * 0.18),
            ];

            let c = rect.min + vec2(w * 0.5, h * 0.58);
            p.circle_filled(c, w * 0.23, Color32::from_rgb(30, 41, 59));
            p.add(egui::Shape::convex_polygon(chevron_pts, Color32::from_rgb(0, 114, 188), egui::Stroke::NONE));
            p.circle_stroke(c, w * 0.28, egui::Stroke::new(3.5, Color32::from_rgb(245, 171, 25)));

            let footer = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.90), rect.max);
            p.rect_filled(footer, egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, Color32::from_rgb(18, 38, 71));
        }
        4 => {
            // Style 4 (Honeycomb Hexagon): Dark background + vertical stripes + chevron banner + tri-hexagon cluster
            p.rect_filled(rect, 4.0, Color32::from_rgb(17, 24, 39));

            // Stripes
            let s1 = egui::Rect::from_min_max(rect.min + vec2(w * 0.60, 0.0), rect.min + vec2(w * 0.68, h));
            let s2 = egui::Rect::from_min_max(rect.min + vec2(w * 0.70, 0.0), rect.min + vec2(w * 0.74, h));
            let s3 = egui::Rect::from_min_max(rect.min + vec2(w * 0.76, 0.0), rect.min + vec2(w * 0.78, h));
            p.rect_filled(s1, 0.0, Color32::from_rgba_unmultiplied(245, 171, 25, 200));
            p.rect_filled(s2, 0.0, Color32::from_rgba_unmultiplied(245, 171, 25, 140));
            p.rect_filled(s3, 0.0, Color32::from_rgba_unmultiplied(245, 171, 25, 90));

            // Left banner
            let banner = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.26), rect.min + vec2(w * 0.45, h * 0.50));
            p.rect_filled(banner, 0.0, Color32::from_rgb(0, 114, 188));
            p.line_segment(
                [rect.min + vec2(0.0, h * 0.50), rect.min + vec2(w * 0.45, h * 0.50)],
                egui::Stroke::new(2.0, Color32::from_rgb(245, 171, 25)),
            );

            // Tri-hexagon cluster representation
            let c = rect.min + vec2(w * 0.65, h * 0.48);
            p.circle_filled(c, w * 0.24, Color32::WHITE);
            p.circle_filled(c, w * 0.21, Color32::from_rgb(30, 41, 59));
            p.circle_stroke(c, w * 0.22, egui::Stroke::new(1.5, Color32::from_rgb(245, 171, 25)));
        }
        5 => {
            // Style 5 (Corporate Headline): White base + top bar + top-right metadata + photo vertical frame + rounded headline block + bottom-right block + bottom-left logo (position lock)
            p.rect_filled(rect, 4.0, Color32::WHITE);
            // Top bar
            let top_bar = egui::Rect::from_min_max(rect.min, rect.min + vec2(w * 0.90, h * 0.025));
            p.rect_filled(top_bar, 0.0, Color32::from_rgb(108, 92, 231));
            // Top-right metadata lines (position lock)
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.62, h * 0.035), rect.min + vec2(w * 0.94, h * 0.048)),
                1.0,
                Color32::from_rgb(100, 116, 139),
            );
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.70, h * 0.055), rect.min + vec2(w * 0.94, h * 0.068)),
                1.0,
                Color32::from_rgb(100, 116, 139),
            );
            // Bottom-right block
            let br_block = egui::Rect::from_min_max(rect.min + vec2(w * 0.72, h * 0.92), rect.max);
            p.rect_filled(br_block, egui::CornerRadius { nw: 0, ne: 0, sw: 0, se: 4 }, Color32::from_rgb(108, 92, 231));
            // Center-right vertical photo frame
            let photo = egui::Rect::from_min_max(rect.min + vec2(w * 0.38, h * 0.20), rect.min + vec2(w * 0.95, h * 0.92));
            p.rect_filled(photo, 0.0, Color32::from_rgb(140, 170, 200));
            // Top-left L bracket
            let l_h = egui::Rect::from_min_max(rect.min + vec2(w * 0.36, h * 0.19), rect.min + vec2(w * 0.55, h * 0.205));
            let l_v = egui::Rect::from_min_max(rect.min + vec2(w * 0.36, h * 0.19), rect.min + vec2(w * 0.38, h * 0.32));
            p.rect_filled(l_h, 0.0, Color32::from_rgb(108, 92, 231));
            p.rect_filled(l_v, 0.0, Color32::from_rgb(108, 92, 231));
            // Top-right corporate accent card
            let corp_box = egui::Rect::from_min_max(rect.min + vec2(w * 0.58, h * 0.13), rect.min + vec2(w, h * 0.28));
            p.rect_filled(corp_box, 0.0, Color32::from_rgb(108, 92, 231));
            // Middle-left year & volume line
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.31), rect.min + vec2(w * 0.34, h * 0.39)),
                2.0,
                Color32::from_rgb(30, 41, 59),
            );
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.41), rect.min + vec2(w * 0.32, h * 0.43)),
                1.0,
                Color32::from_rgb(108, 92, 231),
            );
            // Headline block with rounded top-right (semi-transparent)
            let hl_block = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.54), rect.min + vec2(w * 0.58, h * 0.81));
            p.rect_filled(hl_block, egui::CornerRadius { nw: 0, ne: 8, sw: 0, se: 0 }, Color32::from_rgba_unmultiplied(108, 92, 231, 218));
            // Bottom-left logo & agency text (position lock)
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.87), rect.min + vec2(w * 0.14, h * 0.94)),
                2.0,
                Color32::from_rgb(108, 92, 231),
            );
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.16, h * 0.88), rect.min + vec2(w * 0.52, h * 0.90)),
                1.0,
                Color32::from_rgb(30, 41, 59),
            );
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.16, h * 0.91), rect.min + vec2(w * 0.46, h * 0.93)),
                1.0,
                Color32::from_rgb(30, 41, 59),
            );
        }
        6 => {
            // Style 6 (Diagonal Arrow): white base + right photo panel + navy top banner +
            // amber corner wedge + navy forward wedge carrying the year + mid-left headline
            p.rect_filled(rect, 4.0, Color32::WHITE);

            // Right-hand photo panel
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.48, 0.0), rect.max), 0.0, Color32::from_rgb(146, 172, 198));

            let navy = Color32::from_rgb(18, 38, 71);

            // Navy diagonal banner across the top
            let band = vec![
                rect.min + vec2(w * 0.19, 0.0),
                rect.min + vec2(w * 0.672, 0.0),
                rect.min + vec2(w * 0.481, h * 0.304),
                rect.min + vec2(0.0, h * 0.304),
            ];
            p.add(egui::Shape::convex_polygon(band, navy, egui::Stroke::NONE));

            // Amber corner wedge (top-left)
            let wedge = vec![rect.min, rect.min + vec2(w * 0.146, 0.0), rect.min + vec2(0.0, h * 0.233)];
            p.add(egui::Shape::convex_polygon(wedge, Color32::from_rgb(245, 171, 25), egui::Stroke::NONE));

            // Top-right metadata lines (position lock, over the photo)
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.56, h * 0.020), rect.min + vec2(w * 0.94, h * 0.032)),
                1.0,
                Color32::from_rgb(226, 232, 240),
            );
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.60, h * 0.040), rect.min + vec2(w * 0.94, h * 0.052)),
                1.0,
                Color32::from_rgb(226, 232, 240),
            );

            // Navy forward wedge carrying the year and volume
            let wedge2 = vec![rect.min + vec2(0.0, h * 0.617), rect.min + vec2(w * 0.591, h * 0.702), rect.min + vec2(0.0, h * 0.788)];
            p.add(egui::Shape::convex_polygon(wedge2, navy, egui::Stroke::NONE));
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.03, h * 0.672), rect.min + vec2(w * 0.27, h * 0.726)), 2.0, Color32::WHITE);
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.03, h * 0.735), rect.min + vec2(w * 0.26, h * 0.751)),
                1.0,
                Color32::from_rgb(226, 232, 240),
            );

            // Middle-left headline: level (navy) / title (amber) / DALAM ANGKA (navy)
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.425), rect.min + vec2(w * 0.42, h * 0.465)), 2.0, navy);
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.468), rect.min + vec2(w * 0.62, h * 0.508)),
                2.0,
                Color32::from_rgb(245, 171, 25),
            );
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.512), rect.min + vec2(w * 0.45, h * 0.552)), 2.0, navy);

            // Bottom-left logo & agency text (position lock)
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.885), rect.min + vec2(w * 0.13, h * 0.935)),
                2.0,
                Color32::from_rgb(0, 160, 220),
            );
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.15, h * 0.892), rect.min + vec2(w * 0.47, h * 0.905)), 1.0, navy);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.15, h * 0.915), rect.min + vec2(w * 0.42, h * 0.928)), 1.0, navy);
        }
        7 => {
            // Style 7 (Modern Horizon): full-bleed photo under a sweeping arc + orange swoosh +
            // navy bottom block with the rounded top-left corner carrying the headline
            p.rect_filled(rect, 4.0, Color32::WHITE);
            let navy = Color32::from_rgb(8, 38, 65);
            let orange = Color32::from_rgb(245, 135, 31);

            // Photograph fills the top arc: an approximation of the traced sweep.
            let arc = vec![rect.min, rect.min + vec2(w, 0.0), rect.min + vec2(w, h * 0.557), rect.min + vec2(0.0, h * 0.204)];
            p.add(egui::Shape::convex_polygon(arc, Color32::from_rgb(150, 172, 196), egui::Stroke::NONE));

            // Navy block: rounded top-left corner, flat lower half.
            let navy_pts = vec![
                rect.min + vec2(0.0, h * 0.627),
                rect.min + vec2(w * 0.326, h * 0.499),
                rect.min + vec2(w * 0.625, h * 0.549),
                rect.min + vec2(w, h * 0.549),
                rect.max,
                rect.min + vec2(0.0, h),
            ];
            p.add(egui::Shape::convex_polygon(navy_pts, navy, egui::Stroke::NONE));

            // Orange wedge following the arc (left) and the accent block (right).
            let orange_pts = vec![
                rect.min + vec2(0.0, h * 0.208),
                rect.min + vec2(w, h * 0.561),
                rect.min + vec2(w, h * 0.855),
                rect.min + vec2(w * 0.625, h * 0.855),
                rect.min + vec2(w * 0.625, h * 0.544),
                rect.min + vec2(0.0, h * 0.622),
            ];
            p.add(egui::Shape::convex_polygon(orange_pts, orange, egui::Stroke::NONE));

            // Top-right metadata (position lock)
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.56, h * 0.020), rect.min + vec2(w * 0.94, h * 0.032)), 1.0, navy);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.60, h * 0.040), rect.min + vec2(w * 0.94, h * 0.052)), 1.0, navy);

            // Headline: level (white) / title (orange) / DALAM ANGKA (white) inside the navy block
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.662), rect.min + vec2(w * 0.50, h * 0.706)), 2.0, Color32::WHITE);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.716), rect.min + vec2(w * 0.72, h * 0.766)), 2.0, orange);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.776), rect.min + vec2(w * 0.50, h * 0.820)), 2.0, Color32::WHITE);

            // Year & volume centred in the accent block
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.68, h * 0.645), rect.min + vec2(w * 0.92, h * 0.712)), 2.0, Color32::WHITE);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.72, h * 0.734), rect.min + vec2(w * 0.88, h * 0.748)), 1.0, Color32::WHITE);

            // Bottom-left logo & agency text (position lock)
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.885), rect.min + vec2(w * 0.14, h * 0.945)),
                2.0,
                Color32::from_rgb(0, 160, 220),
            );
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.16, h * 0.892), rect.min + vec2(w * 0.52, h * 0.905)), 1.0, Color32::WHITE);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.16, h * 0.915), rect.min + vec2(w * 0.46, h * 0.928)), 1.0, Color32::WHITE);
        }
        9 => {
            // Style 9 (Urban Angle): orange top-left wedge, navy diagonal band, cityscape photo on
            // the right, white negative-X, navy bottom wedge.
            let accent = Color32::from_rgb(252, 185, 30);
            let navy = Color32::from_rgb(3, 68, 96);
            // Photo region on the right.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min + vec2(w * 0.39, h * 0.43), rect.min + vec2(w, 0.0), rect.min + vec2(w, h * 0.99)],
                Color32::from_rgb(120, 150, 170),
                egui::Stroke::NONE,
            ));
            // Orange top-left wedge.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min, rect.min + vec2(w * 0.86, 0.0), rect.min + vec2(0.0, h * 0.43)],
                accent,
                egui::Stroke::NONE,
            ));
            // Navy diagonal band.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min + vec2(w * 0.50, 0.0), rect.min + vec2(w * 0.90, 0.0), rect.min + vec2(w * 0.44, h * 0.45)],
                navy,
                egui::Stroke::NONE,
            ));
            // Navy bottom wedge.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min + vec2(w * 0.39, h * 0.78), rect.min + vec2(w * 0.82, h), rect.min + vec2(w * 0.35, h)],
                navy,
                egui::Stroke::NONE,
            ));
            // Headline stack on the white left panel.
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.68), rect.min + vec2(w * 0.50, h * 0.72)), 2.0, navy);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.75), rect.min + vec2(w * 0.60, h * 0.80)), 2.0, accent);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.83), rect.min + vec2(w * 0.45, h * 0.87)), 2.0, navy);
            // Top-left company mark.
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.05, h * 0.02), rect.min + vec2(w * 0.12, h * 0.06)), 1.0, navy);
            // Rotated year in the band.
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.55, h * 0.10), rect.min + vec2(w * 0.78, h * 0.20)), 2.0, Color32::WHITE);
        }
        _ => {
            // Style 8 (Business Report, traced from design-10): white base, orange top band and
            // bottom-right corner, arrow-shaped photograph with its chevron sliver across a thin
            // white gap, navy plane crossing from the left with a white year circle, headline
            // top-left, agency block bottom-left.
            p.rect_filled(rect, 4.0, Color32::WHITE);
            let navy = Color32::from_rgb(30, 58, 95);
            let orange = Color32::from_rgb(240, 130, 35);

            // Orange bottom-right corner, then the navy plane crossing from the left.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min + vec2(w * 0.712, h * 0.877), rect.max, rect.min + vec2(w * 0.903, h)],
                orange,
                egui::Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![
                    rect.min + vec2(0.0, h * 0.351),
                    rect.min + vec2(w * 0.364, h * 0.497),
                    rect.min + vec2(w * 0.364, h * 0.738),
                    rect.min + vec2(w, h * 0.993),
                    rect.min,
                ],
                navy,
                egui::Stroke::NONE,
            ));

            // Orange band along the top edge.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min + vec2(w * 0.416, 0.0), rect.min + vec2(w, 0.0), rect.min + vec2(w, h * 0.096), rect.min + vec2(w * 0.824, h * 0.17)],
                orange,
                egui::Stroke::NONE,
            ));

            // Arrow-shaped photograph, the white gap across it, then the chevron sliver.
            let arrow = [
                rect.min + vec2(w * 0.34, h * 0.376),
                rect.min + vec2(w, h * 0.108),
                rect.min + vec2(w, h * 0.745),
                rect.min + vec2(w * 0.34, h * 0.48),
            ];
            p.add(egui::Shape::convex_polygon(arrow.to_vec(), Color32::from_rgb(150, 172, 196), egui::Stroke::NONE));
            p.add(egui::Shape::convex_polygon(
                vec![
                    rect.min + vec2(w * 0.34, h * 0.48),
                    rect.min + vec2(w, h * 0.745),
                    rect.min + vec2(w * 0.99, h * 0.751),
                    rect.min + vec2(w * 0.371, h * 0.503),
                ],
                Color32::WHITE,
                egui::Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![
                    rect.min + vec2(w * 0.371, h * 0.503),
                    rect.min + vec2(w * 0.99, h * 0.751),
                    rect.min + vec2(w * 0.711, h * 0.866),
                    rect.min + vec2(w * 0.371, h * 0.73),
                ],
                Color32::from_rgb(108, 124, 140),
                egui::Stroke::NONE,
            ));

            // White year circle on the navy plane.
            p.circle_filled(rect.min + vec2(w * 0.252, h * 0.631), w * 0.16, Color32::WHITE);

            // Headline stack: level (navy), title (orange), DALAM ANGKA (navy).
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.114), rect.min + vec2(w * 0.6, h * 0.15)), 2.0, navy);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.152), rect.min + vec2(w * 0.62, h * 0.207)), 2.0, orange);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.209), rect.min + vec2(w * 0.6, h * 0.245)), 2.0, navy);

            // Two-tone year and the volume line inside the circle.
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.135, h * 0.608), rect.min + vec2(w * 0.25, h * 0.663)), 2.0, navy);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.255, h * 0.608), rect.min + vec2(w * 0.365, h * 0.663)), 2.0, orange);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.135, h * 0.687), rect.min + vec2(w * 0.405, h * 0.717)), 2.0, navy);

            // Agency block bottom-left: logo mark + white agency lines on the navy plane.
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.077, h * 0.917), rect.min + vec2(w * 0.145, h * 0.965)), 1.0, orange);
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.158, h * 0.919), rect.min + vec2(w * 0.521, h * 0.969)),
                1.0,
                Color32::from_gray(235),
            );
        }
    }
}

fn show_style_thumbnail_tooltip(ui: &mut Ui, style_idx: usize, name: &str, t: &Tokens) {
    ui.vertical(|ui| {
        ui.label(RichText::new(name).size(12.0).strong().color(t.text_strong));
        ui.add_space(4.0);

        let ctx = ui.ctx().clone();
        if let Some(tex) = style_thumbnail_texture(&ctx, style_idx) {
            let size = tex.size_vec2();
            let thumb_w = 150.0;
            let thumb_h = (thumb_w * size.y / size.x.max(1.0)).min(220.0);
            let (rect, _) = ui.allocate_exact_size(vec2(thumb_w, thumb_h), Sense::hover());
            ui.painter().rect_filled(rect, 4.0, Color32::from_gray(30));
            ui.painter().image(tex.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Outside);
        } else {
            paint_fallback_cover_thumb(ui, style_idx, 150.0, 212.0, t);
        }

        let desc = match style_idx {
            0 => "Wave curves, concentric cradle ring, and circular medallion photo.",
            1 => "Modern diagonal polygon landscape background with circular cutout.",
            2 => "Geometric origami folded facet ribbons with hexagonal photo header.",
            3 => "Executive downward chevron crest with golden horseshoe medallion cradle.",
            4 => "Architectural tri-hexagon honeycomb cluster with corporate chevron banner and racing stripes.",
            5 => "Modern corporate architecture layout with top framing, headline focal block, and position-locked institutional metadata.",
            6 => "Navy diagonal banner and amber corner wedge over a half-bleed cityscape, with a forward navy wedge carrying the year and volume, and a position-locked agency footer.",
            7 => "Full-bleed cityscape under a sweeping arc with an accent wedge that follows it, a dark institutional block carrying the headline, and a position-locked agency footer.",
            8 => "Arrow-shaped skyline photograph with a chevron sliver across a white gap, an orange top band and corner, a navy plane carrying the white year circle, the headline top-left and the agency block bottom-left.",
            9 => "Orange corner wedge and a navy diagonal band carrying the rotated year over a right-hand cityscape, with a white negative-space headline panel and a navy bottom wedge.",
            _ => "",
        };
        ui.add_space(3.0);
        ui.label(RichText::new(desc).size(10.0).color(t.text_dim));
    });
}

fn card_cover_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Publication Cover"), true);

    // 1. Template Style with Hover Thumbnail Preview
    ui.horizontal(|ui| {
        ui.label(RichText::new("Style:").size(11.0).color(t.text));
        let styles = [
            "Wave Curves",
            "Geometric & Medallion",
            "Folded Origami",
            "Chevron & Medallion",
            "Honeycomb Hexagon",
            "Corporate Headline",
            "Diagonal Arrow",
            "Modern Horizon",
            "Business Report",
            "Urban Angle",
        ];
        let cur_style = styles.get(st.cover_style).copied().unwrap_or("Wave Curves");
        // The dock can be as narrow as 220 pt: the dropdown takes what the row has left.
        let combo_w = (ui.available_width() - 80.0).clamp(110.0, 200.0);
        let prev_style = st.cover_style;
        let combo = egui::ComboBox::from_id_salt("pubgen_cover_style").selected_text(cur_style).width(combo_w).show_ui(ui, |ui| {
            for (idx, s) in styles.iter().enumerate() {
                let resp = ui.selectable_value(&mut st.cover_style, idx, *s);
                resp.on_hover_ui(|ui| {
                    show_style_thumbnail_tooltip(ui, idx, s, t);
                });
            }
        });
        if st.cover_style != prev_style {
            match st.cover_style {
                URBAN_STYLE => apply_urban_palette(st),
                HORIZON_STYLE | BUSINESS_STYLE => apply_report_palette(st),
                _ => {}
            }
        }
        combo.response.on_hover_ui(|ui| {
            show_style_thumbnail_tooltip(ui, st.cover_style, cur_style, t);
        });

        // Small eye preview icon button that also triggers hover preview
        let (thumb_icon_rect, thumb_resp) = ui.allocate_exact_size(vec2(22.0, 20.0), Sense::click());
        let icon_hovered = thumb_resp.hovered();
        ui.painter().rect_filled(thumb_icon_rect, 3.0, if icon_hovered { t.accent } else { t.panel });
        ui.painter().rect_stroke(thumb_icon_rect, 3.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Outside);
        ui.painter().text(
            thumb_icon_rect.center(),
            egui::Align2::CENTER_CENTER,
            "👁",
            egui::FontId::proportional(11.0),
            if icon_hovered { Color32::WHITE } else { t.text_dim },
        );
        thumb_resp.on_hover_ui(|ui| {
            show_style_thumbnail_tooltip(ui, st.cover_style, cur_style, t);
        });
    });

    // 2. Color Theme Preset
    ui.horizontal(|ui| {
        ui.label(RichText::new("Theme:").size(11.0).color(t.text));
        let themes = [
            "Classic BPS (Blue/Gold)",
            "Corporate Navy & Amber",
            "Emerald & Gold",
            "Crimson & Slate",
            "Modern Teal & Cyan",
            "Modern Violet & Slate",
            "Report Navy & Orange",
            "Custom",
        ];
        let cur_theme = themes.get(st.cover_theme_preset).copied().unwrap_or("Classic BPS (Blue/Gold)");
        let prev_theme = st.cover_theme_preset;
        let combo_w = (ui.available_width() - 56.0).clamp(110.0, 220.0);
        egui::ComboBox::from_id_salt("pubgen_cover_theme").selected_text(cur_theme).width(combo_w).show_ui(ui, |ui| {
            for (idx, th) in themes.iter().enumerate() {
                ui.selectable_value(&mut st.cover_theme_preset, idx, *th);
            }
        });
        if st.cover_theme_preset != prev_theme {
            match st.cover_theme_preset {
                0 => {
                    st.cover_color_primary = "#0072bc".into();
                    st.cover_color_accent = "#f5ab19".into();
                    st.cover_color_dark = "#122647".into();
                }
                1 => {
                    st.cover_color_primary = "#1e3a8a".into();
                    st.cover_color_accent = "#d97706".into();
                    st.cover_color_dark = "#0f172a".into();
                }
                2 => {
                    st.cover_color_primary = "#047857".into();
                    st.cover_color_accent = "#eab308".into();
                    st.cover_color_dark = "#064e3b".into();
                }
                3 => {
                    st.cover_color_primary = "#be123c".into();
                    st.cover_color_accent = "#ea580c".into();
                    st.cover_color_dark = "#1e293b".into();
                }
                4 => {
                    st.cover_color_primary = "#0891b2".into();
                    st.cover_color_accent = "#f59e0b".into();
                    st.cover_color_dark = "#1e293b".into();
                }
                5 => {
                    st.cover_color_primary = "#6c5ce7".into();
                    st.cover_color_accent = "#a29bfe".into();
                    st.cover_color_dark = "#1e293b".into();
                }
                6 => apply_report_palette(st),
                _ => {}
            }
        }
    });

    // Custom color hex inputs
    ui.horizontal(|ui| {
        ui.label(RichText::new("Colors:").size(10.5).color(t.text_dim));
        let field_w = ((ui.available_width() - 56.0) / 3.0 - 6.0).clamp(44.0, 72.0);
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_color_primary))
            .on_hover_text("Primary Color (e.g. #0072bc)");
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_color_accent))
            .on_hover_text("Accent Color (e.g. #f5ab19)");
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_color_dark))
            .on_hover_text("Dark / Secondary Color (e.g. #122647)");
    });

    // 3. Form fields
    // #1 Title: one headline, typed over several rows (the cover keeps it as written).
    ui.horizontal(|ui| {
        ui.label(RichText::new("Title:").size(11.0).color(t.text));
        let avail = (ui.available_width() - 4.0).max(100.0);
        ui.add_sized([avail, crate::widgets::FIELD_H * 1.8], egui::TextEdit::multiline(&mut st.cover_title).hint_text("KARAWANG TIMUR\nDALAM ANGKA"))
            .on_hover_text("The cover headline, as written (multi-line supported)");
    });

    // #2 Title 2: the second headline line, in the cover's own frame and style.
    ui.horizontal(|ui| {
        ui.label(RichText::new("Title 2:").size(11.0).color(t.text));
        let avail = (ui.available_width() - 4.0).max(100.0);
        ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_title2))
            .on_hover_text("The second headline line, set smaller and in another colour");
    });

    // Title EN: English translation of title (optional, used in bilingual publications)
    ui.horizontal(|ui| {
        ui.label(RichText::new("Title EN:").size(11.0).color(t.text));
        let avail = (ui.available_width() - 4.0).max(100.0);
        ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_title_en))
            .on_hover_text("English title for dual-language cover");
    });

    // #3 Year & #4 Volume
    ui.horizontal(|ui| {
        ui.label(RichText::new("Year:").size(11.0).color(t.text));
        ui.add_sized([65.0, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_year));

        ui.label(RichText::new("Volume:").size(11.0).color(t.text));
        let avail = (ui.available_width() - 4.0).max(80.0);
        ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_volume));
    });

    // #5 Catalogue Number
    ui.horizontal(|ui| {
        ui.label(RichText::new("Catalogue:").size(11.0).color(t.text));
        let avail = (ui.available_width() - 4.0).max(100.0);
        ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_catalogue));
    });

    // #6 Code: ISSN / ISBN + number
    ui.horizontal(|ui| {
        let code_types = ["ISSN", "ISBN"];
        let cur_ct = code_types.get(st.cover_code_type).copied().unwrap_or("ISSN");
        egui::ComboBox::from_id_salt("pubgen_cover_code_type").selected_text(cur_ct).width(70.0).show_ui(ui, |ui| {
            for (idx, ct) in code_types.iter().enumerate() {
                ui.selectable_value(&mut st.cover_code_type, idx, *ct);
            }
        });
        let avail = (ui.available_width() - 4.0).max(80.0);
        ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_code_num));
    });

    // 4. Dynamic Cover Images based on Selected Style
    divider(ui);
    ui.label(RichText::new("Cover Images:").size(11.0).strong().color(t.text_strong));

    match st.cover_style {
        0 => {
            // Style 0 (Wave Curves): 1 image (Circular Photo)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Photo:").size(10.5).color(t.text));
                let w = (ui.available_width() - 65.0).max(80.0);
                ui.add_sized(
                    [w, crate::widgets::FIELD_H],
                    egui::TextEdit::singleline(&mut st.cover_image_skyline).hint_text("outputs/city_skyline.jpg"),
                )
                .on_hover_text("Circular center photo");
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
                    && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
                {
                    st.cover_image_skyline = path;
                }
            });
        }
        1 => {
            // Style 1 (Geometric & Medallion): 2 images (Background Photo & Medallion Cutout)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Background:").size(10.5).color(t.text));
                let w = (ui.available_width() - 65.0).max(80.0);
                ui.add_sized(
                    [w, crate::widgets::FIELD_H],
                    egui::TextEdit::singleline(&mut st.cover_image_landscape).hint_text("outputs/city_landscape.jpg"),
                )
                .on_hover_text("Diagonal polygon background landscape photo");
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
                    && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
                {
                    st.cover_image_landscape = path;
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Medallion:").size(10.5).color(t.text));
                let w = (ui.available_width() - 65.0).max(80.0);
                ui.add_sized(
                    [w, crate::widgets::FIELD_H],
                    egui::TextEdit::singleline(&mut st.cover_image_skyline).hint_text("outputs/city_skyline.jpg"),
                )
                .on_hover_text("Circular cutout foreground photo");
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
                    && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
                {
                    st.cover_image_skyline = path;
                }
            });
        }
        2 => {
            // Style 2 (Folded Origami): 1 image (Header / Hexagonal Cutout Photo)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Header Photo:").size(10.5).color(t.text));
                let w = (ui.available_width() - 65.0).max(80.0);
                ui.add_sized(
                    [w, crate::widgets::FIELD_H],
                    egui::TextEdit::singleline(&mut st.cover_image_skyline).hint_text("outputs/city_skyline.jpg"),
                )
                .on_hover_text("Top-right hexagonal photo cutout");
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
                    && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
                {
                    st.cover_image_skyline = path;
                }
            });
        }
        3 => {
            // Style 3 (Chevron & Medallion): 1 image (Medallion Photo)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Medallion:").size(10.5).color(t.text));
                let w = (ui.available_width() - 65.0).max(80.0);
                ui.add_sized(
                    [w, crate::widgets::FIELD_H],
                    egui::TextEdit::singleline(&mut st.cover_image_skyline).hint_text("outputs/city_skyline.jpg"),
                )
                .on_hover_text("Circular center photo nestled inside the horseshoe medallion");
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
                    && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
                {
                    st.cover_image_skyline = path;
                }
            });
        }
        4 => {
            // Style 4 (Honeycomb Hexagon): 1 image (Tri-hexagon honeycomb cluster photo)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Honeycomb:").size(10.5).color(t.text));
                let w = (ui.available_width() - 65.0).max(80.0);
                ui.add_sized(
                    [w, crate::widgets::FIELD_H],
                    egui::TextEdit::singleline(&mut st.cover_image_landscape).hint_text("outputs/city_landscape.jpg"),
                )
                .on_hover_text("Tri-hexagon honeycomb cluster photo");
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
                    && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
                {
                    st.cover_image_landscape = path;
                }
            });
        }
        5 => {
            // Style 5 (Corporate Headline): 1 image (Vertical architectural photo)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Photo:").size(10.5).color(t.text));
                let w = (ui.available_width() - 65.0).max(80.0);
                ui.add_sized(
                    [w, crate::widgets::FIELD_H],
                    egui::TextEdit::singleline(&mut st.cover_image_skyline).hint_text("outputs/city_skyline.jpg"),
                )
                .on_hover_text("Vertical architecture or city skyline photo");
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
                    && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
                {
                    st.cover_image_skyline = path;
                }
            });
        }
        6 => {
            // Style 6 (Diagonal Arrow): 1 image (Full-height cityscape panel on the right)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Photo:").size(10.5).color(t.text));
                let w = (ui.available_width() - 65.0).max(80.0);
                ui.add_sized(
                    [w, crate::widgets::FIELD_H],
                    egui::TextEdit::singleline(&mut st.cover_image_skyline).hint_text("outputs/city_skyline.jpg"),
                )
                .on_hover_text("Full-height cityscape panel behind the diagonal band and arrow");
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
                    && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
                {
                    st.cover_image_skyline = path;
                }
            });
        }
        _ => {
            // Style 7 (Modern Horizon) & Style 8 (Business Report): 1 image (Hero photograph)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Photo:").size(10.5).color(t.text));
                let w = (ui.available_width() - 65.0).max(80.0);
                let hover = match st.cover_style {
                    URBAN_STYLE => "Cityscape photograph filling the right-hand diagonal panels",
                    BUSINESS_STYLE => "Skyscraper photograph filling the crossed diagonal bands",
                    _ => "Hero photograph clipped by the sweeping arc across the upper page",
                };
                ui.add_sized(
                    [w, crate::widgets::FIELD_H],
                    egui::TextEdit::singleline(&mut st.cover_image_skyline).hint_text("outputs/city_skyline.jpg"),
                )
                .on_hover_text(hover);
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
                    && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
                {
                    st.cover_image_skyline = path;
                }
            });
        }
    }

    // 5. Logo & Agency Branding
    divider(ui);
    ui.label(RichText::new("Logo & Branding:").size(11.0).strong().color(t.text_strong));

    ui.horizontal(|ui| {
        ui.label(RichText::new("Logo:").size(10.5).color(t.text));
        let w = (ui.available_width() - 65.0).max(80.0);
        ui.add_sized([w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_image_logo).hint_text("outputs/bps_logo.png"))
            .on_hover_text("Path to logo image file (PNG / JPG / SVG)");
        if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
            && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
        {
            st.cover_image_logo = path;
        }
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Logo Text:").size(10.5).color(t.text));
        let avail = (ui.available_width() - 4.0).max(80.0);
        ui.add_sized(
            [avail, crate::widgets::FIELD_H * 1.8],
            egui::TextEdit::multiline(&mut st.cover_agency_text).hint_text("BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG"),
        )
        .on_hover_text("Text displayed next to the logo (multi-line supported)");
    });

    // Action button
    let fw = full_width(ui);
    let gen_btn =
        outline_button(ui, "Generate Cover Page", fw).on_hover_text("Automatically inserts a new cover page before Page 1 and generates layout");
    if gen_btn.clicked() {
        let res = execute_generate_cover(app, st);
        st.cover_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(err) => (err, false),
        });
    }

    if let Some((msg, ok)) = &st.cover_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

fn load_hf_template_to_state(st: &mut PubgenState, tpl: &Value) {
    if let Some(name) = tpl.get("name").and_then(Value::as_str) {
        st.hf_style = name.to_string();
    }
    if let Some(sided) = tpl.get("sided").and_then(Value::as_str) {
        st.hf_sided = if sided.eq_ignore_ascii_case("single") { 1 } else { 0 };
    }
    if let Some(ho) = tpl.get("headerOffset").and_then(Value::as_f64) {
        st.hf_header_offset = ho;
    }
    if let Some(fo) = tpl.get("footerOffset").and_then(Value::as_f64) {
        st.hf_footer_offset = fo;
    }
    if let Some(h) = tpl.get("header") {
        st.hf_header_single = h.get("single").and_then(Value::as_str).unwrap_or("").to_string();
        st.hf_header_left = h.get("left").and_then(Value::as_str).unwrap_or("").to_string();
        st.hf_header_right = h.get("right").and_then(Value::as_str).unwrap_or("").to_string();
    }
    if let Some(f) = tpl.get("footer") {
        st.hf_footer_single = f.get("single").and_then(Value::as_str).unwrap_or("").to_string();
        st.hf_footer_left = f.get("left").and_then(Value::as_str).unwrap_or("").to_string();
        st.hf_footer_right = f.get("right").and_then(Value::as_str).unwrap_or("").to_string();
    }
    if let Some(obj) = tpl.get("objects") {
        st.hf_header_rule = obj.get("headerRule").and_then(Value::as_bool).unwrap_or(false);
        st.hf_header_rule_weight = obj.get("headerRuleWeight").and_then(Value::as_f64).unwrap_or(0.5);
        st.hf_footer_rule = obj.get("footerRule").and_then(Value::as_bool).unwrap_or(false);
        st.hf_footer_rule_weight = obj.get("footerRuleWeight").and_then(Value::as_f64).unwrap_or(0.5);
        st.hf_top_accent_bar = obj.get("topAccentBar").and_then(Value::as_bool).unwrap_or(false);
        st.hf_top_accent_bar_height = obj.get("topAccentBarHeight").and_then(Value::as_f64).unwrap_or(4.0);
        st.hf_top_accent_bar_bleed = obj.get("topAccentBarBleed").and_then(Value::as_bool).unwrap_or(true);
    }
    if let Some(cols) = tpl.get("colors") {
        st.hf_color_primary = cols.get("primary").and_then(Value::as_str).unwrap_or("#0072bc").to_string();
        st.hf_color_accent = cols.get("accent").and_then(Value::as_str).unwrap_or("#f5ab19").to_string();
        st.hf_color_dark = cols.get("dark").and_then(Value::as_str).unwrap_or("#122647").to_string();
        st.hf_color_rule = cols.get("ruleColor").and_then(Value::as_str).unwrap_or("#cbd5e1").to_string();
        st.hf_color_text = cols.get("textColor").and_then(Value::as_str).unwrap_or("#334155").to_string();
    }
}

fn show_hf_preview_tooltip(ui: &mut Ui, st: &PubgenState, name: &str, t: &Tokens) {
    ui.vertical(|ui| {
        ui.label(RichText::new(name).size(12.0).strong().color(t.text_strong));
        ui.add_space(4.0);

        let (rect, _) = ui.allocate_exact_size(vec2(160.0, 110.0), Sense::hover());
        // Page background
        ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(250, 250, 250));
        ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, Color32::from_rgb(200, 200, 200)), egui::StrokeKind::Outside);

        let parse_color = |s: &str, def: Color32| {
            if let Some(c) = designcraft_color::Color::from_hex(s) {
                let [r, g, b, _] = c.to_rgba8(1.0);
                Color32::from_rgb(r, g, b)
            } else {
                def
            }
        };

        let primary_color = parse_color(&st.hf_color_primary, Color32::from_rgb(0, 114, 188));
        let rule_color = parse_color(&st.hf_color_rule, Color32::from_rgb(200, 200, 200));

        // 1. Top accent bar
        if st.hf_top_accent_bar {
            let bar_rect = egui::Rect::from_min_size(rect.min, vec2(rect.width(), 4.0));
            ui.painter().rect_filled(bar_rect, 0.0, primary_color);
        }

        // 2. Header text & rule
        let header_y = rect.min.y + 14.0;
        let left_x = rect.min.x + 8.0;
        let right_x = rect.max.x - 8.0;

        if !st.hf_header_left.is_empty() || !st.hf_header_single.is_empty() {
            ui.painter().text(pos2(left_x, header_y), egui::Align2::LEFT_CENTER, "Headline", egui::FontId::proportional(8.0), t.text_dim);
        }
        if !st.hf_header_right.is_empty() {
            ui.painter().text(pos2(right_x, header_y), egui::Align2::RIGHT_CENTER, "Section", egui::FontId::proportional(8.0), t.text_dim);
        }
        if st.hf_header_rule {
            ui.painter().line_segment([pos2(left_x, header_y + 6.0), pos2(right_x, header_y + 6.0)], egui::Stroke::new(1.0, rule_color));
        }

        // Faint body lines
        let body_y0 = header_y + 14.0;
        for i in 0..4 {
            let by = body_y0 + (i as f32) * 8.0;
            let bw = if i == 3 { rect.width() * 0.45 } else { rect.width() - 24.0 };
            ui.painter().line_segment([pos2(left_x + 4.0, by), pos2(left_x + 4.0 + bw, by)], egui::Stroke::new(1.0, Color32::from_gray(230)));
        }

        // 3. Footer text & rule
        let footer_y = rect.max.y - 12.0;
        if st.hf_footer_rule {
            ui.painter().line_segment([pos2(left_x, footer_y - 6.0), pos2(right_x, footer_y - 6.0)], egui::Stroke::new(1.0, rule_color));
        }
        ui.painter().text(pos2(left_x, footer_y), egui::Align2::LEFT_CENTER, "12", egui::FontId::proportional(8.0), t.text_dim);
        ui.painter().text(pos2(right_x, footer_y), egui::Align2::RIGHT_CENTER, "Document", egui::FontId::proportional(8.0), t.text_dim);

        ui.add_space(3.0);
        let desc = match name {
            "Modern Minimal" => "Clean, understated page number with subtle footer divider.",
            "Editorial Book" => "Refined running headers with hairline rule, ideal for magazines and books.",
            "Corporate Report" => "Full-width top accent bar, running headers, and confidentiality footer.",
            _ => "Custom user header & footer template.",
        };
        ui.label(RichText::new(desc).size(10.0).color(t.text_dim));
    });
}

fn card_header_footer_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Header & Footer Template"), true);

    // Fetch all templates (built-in + user custom) from engine
    let templates = designcraft_engine::cmd::header_footer::all_templates(&app.session);

    // Lazy initialize from default or current doc if not yet initialized
    if !st.hf_initialized {
        st.hf_initialized = true;
        if let Some(doc_hf) = app.session.active().and_then(|s| s.doc.header_footer.as_ref()) {
            st.hf_style = doc_hf.preset.clone();
            st.hf_sided = if doc_hf.sided == designcraft_doc::Sided::Single { 1 } else { 0 };
            st.hf_header_offset = doc_hf.header_offset;
            st.hf_footer_offset = doc_hf.footer_offset;
            st.hf_header_single = doc_hf.header.single.clone();
            st.hf_header_left = doc_hf.header.left.clone();
            st.hf_header_right = doc_hf.header.right.clone();
            st.hf_footer_single = doc_hf.footer.single.clone();
            st.hf_footer_left = doc_hf.footer.left.clone();
            st.hf_footer_right = doc_hf.footer.right.clone();
            st.hf_header_rule = doc_hf.objects.header_rule;
            st.hf_header_rule_weight = doc_hf.objects.header_rule_weight;
            st.hf_footer_rule = doc_hf.objects.footer_rule;
            st.hf_footer_rule_weight = doc_hf.objects.footer_rule_weight;
            st.hf_top_accent_bar = doc_hf.objects.top_accent_bar;
            st.hf_top_accent_bar_height = doc_hf.objects.top_accent_bar_height;
            st.hf_top_accent_bar_bleed = doc_hf.objects.top_accent_bar_bleed;
            st.hf_color_primary = doc_hf.colors.primary.clone();
            st.hf_color_accent = doc_hf.colors.accent.clone();
            st.hf_color_dark = doc_hf.colors.dark.clone();
            st.hf_color_rule = doc_hf.colors.rule_color.clone();
            st.hf_color_text = doc_hf.colors.text_color.clone();
        } else if let Some(first_tpl) = templates.first() {
            load_hf_template_to_state(st, first_tpl);
        }
    }

    // 1. Template Style Dropdown with Hover Preview Tooltip & Eye Icon
    ui.horizontal(|ui| {
        ui.label(RichText::new("Style:").size(11.0).color(t.text));

        let cur_style = st.hf_style.clone();
        let combo_w = (ui.available_width() - 80.0).clamp(110.0, 200.0);
        let mut picked_template: Option<Value> = None;

        let combo = egui::ComboBox::from_id_salt("pubgen_hf_style").selected_text(&cur_style).width(combo_w).show_ui(ui, |ui| {
            for tpl in &templates {
                let name = tpl.get("name").and_then(Value::as_str).unwrap_or("Template");
                let is_builtin = tpl.get("builtIn").and_then(Value::as_bool).unwrap_or(false);
                let label = if is_builtin { name.to_string() } else { format!("{name} ★") };
                let is_selected = cur_style == name;
                let resp = ui.selectable_label(is_selected, label);
                if resp.clicked() && !is_selected {
                    picked_template = Some(tpl.clone());
                }
                resp.on_hover_ui(|ui| {
                    show_hf_preview_tooltip(ui, st, name, t);
                });
            }
        });

        if let Some(tpl) = picked_template {
            load_hf_template_to_state(st, &tpl);
        }

        combo.response.on_hover_ui(|ui| {
            show_hf_preview_tooltip(ui, st, &cur_style, t);
        });

        // Eye preview button
        let (thumb_icon_rect, thumb_resp) = ui.allocate_exact_size(vec2(22.0, 20.0), Sense::click());
        let icon_hovered = thumb_resp.hovered();
        ui.painter().rect_filled(thumb_icon_rect, 3.0, if icon_hovered { t.accent } else { t.panel });
        ui.painter().rect_stroke(thumb_icon_rect, 3.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Outside);
        ui.painter().text(
            thumb_icon_rect.center(),
            egui::Align2::CENTER_CENTER,
            "👁",
            egui::FontId::proportional(11.0),
            if icon_hovered { Color32::WHITE } else { t.text_dim },
        );
        thumb_resp.on_hover_ui(|ui| {
            show_hf_preview_tooltip(ui, st, &cur_style, t);
        });

        // If current template is custom (not built-in), show a small delete button
        let cur_is_builtin = templates
            .iter()
            .any(|tpl| tpl.get("name").and_then(Value::as_str) == Some(&cur_style) && tpl.get("builtIn").and_then(Value::as_bool) == Some(true));
        if !cur_is_builtin && !cur_style.is_empty() {
            let del_btn = ui.button("🗑").on_hover_text("Delete this custom template");
            if del_btn.clicked() {
                match execute_delete_hf_template(app, st, &cur_style) {
                    Ok(msg) => st.hf_status = Some((msg, true)),
                    Err(err) => st.hf_status = Some((err, false)),
                }
            }
        }
    });

    // 2. Sidedness & Offsets
    ui.horizontal(|ui| {
        ui.label(RichText::new("Sided:").size(11.0).color(t.text));
        let sided_options = ["Double-sided", "Single-sided"];
        let cur_s = sided_options.get(st.hf_sided).copied().unwrap_or("Double-sided");
        egui::ComboBox::from_id_salt("pubgen_hf_sided").selected_text(cur_s).width(95.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut st.hf_sided, 0, "Double-sided");
            ui.selectable_value(&mut st.hf_sided, 1, "Single-sided");
        });

        ui.label(RichText::new("H.Off:").size(10.5).color(t.text_dim));
        ui.add(egui::DragValue::new(&mut st.hf_header_offset).range(0.0..=120.0).speed(0.5).suffix(" pt"))
            .on_hover_text("Header offset from top of page");

        ui.label(RichText::new("F.Off:").size(10.5).color(t.text_dim));
        ui.add(egui::DragValue::new(&mut st.hf_footer_offset).range(0.0..=120.0).speed(0.5).suffix(" pt"))
            .on_hover_text("Footer offset from bottom of page");
    });

    // 2b. Scope: which pages this template applies to (all, one section, or a page range).
    ui.horizontal(|ui| {
        ui.label(RichText::new("Scope:").size(11.0).color(t.text));
        let (page_count, sections) = app
            .session
            .active()
            .map(|a| (a.doc.page_count(), a.doc.sections.iter().map(|s| (s.prefix.clone(), s.start)).collect::<Vec<_>>()))
            .unwrap_or((1, Vec::new()));
        let scope_options = ["All Pages", "Section", "Pages"];
        let cur_scope = scope_options.get(st.hf_scope).copied().unwrap_or("All Pages");
        egui::ComboBox::from_id_salt("pubgen_hf_scope").selected_text(cur_scope).width(85.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut st.hf_scope, 0, "All Pages");
            ui.selectable_value(&mut st.hf_scope, 1, "Section");
            ui.selectable_value(&mut st.hf_scope, 2, "Pages");
        });
        match st.hf_scope {
            1 => {
                if st.hf_scope_section >= sections.len() {
                    st.hf_scope_section = 0;
                }
                let sec_label = |i: usize, s: &(String, usize)| {
                    if s.0.is_empty() { format!("Section {} (page {})", i + 1, s.1 + 1) } else { format!("{} (page {})", s.0, s.1 + 1) }
                };
                let cur_sec = sections
                    .get(st.hf_scope_section)
                    .map_or_else(|| format!("Section {}", st.hf_scope_section + 1), |s| sec_label(st.hf_scope_section, s));
                egui::ComboBox::from_id_salt("pubgen_hf_scope_section").selected_text(cur_sec).width(130.0).show_ui(ui, |ui| {
                    for (i, s) in sections.iter().enumerate() {
                        if ui.selectable_label(st.hf_scope_section == i, sec_label(i, s)).clicked() {
                            st.hf_scope_section = i;
                        }
                    }
                });
            }
            2 => {
                let start = st.hf_scope_start.clamp(1, page_count.max(1));
                let end = st.hf_scope_end.clamp(start, page_count.max(1));
                (st.hf_scope_start, st.hf_scope_end) = (start, end);
                ui.add(egui::DragValue::new(&mut st.hf_scope_start).range(1..=page_count.max(1)).prefix("From: "));
                ui.add(egui::DragValue::new(&mut st.hf_scope_end).range(st.hf_scope_start..=page_count.max(1)).prefix("To: "));
            }
            _ => {}
        }
        if ui.button(RichText::new("Sections…").size(10.5).color(t.text_dim)).clicked() {
            crate::menus::activate(app, "layout.section", &Value::Null);
        }
    });

    // 3. Header Text Fields
    if st.hf_sided == 0 {
        // Double-sided headers
        ui.horizontal(|ui| {
            ui.label(RichText::new("H.Left:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_header_left).hint_text("{page} | Title"))
                .on_hover_text("Left (verso) header text. Tokens: {page}, {section}, {filename}, {runningHeader}");
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("H.Right:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_header_right).hint_text("Title | {page}"))
                .on_hover_text("Right (recto) header text. Tokens: {page}, {section}, {filename}, {runningHeader}");
        });
    } else {
        // Single-sided header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Header:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_header_single).hint_text("{page} | Document"))
                .on_hover_text("Single-sided header text. Tokens: {page}, {section}, {filename}, {runningHeader}");
        });
    }

    // 4. Footer Text Fields
    if st.hf_sided == 0 {
        // Double-sided footers
        ui.horizontal(|ui| {
            ui.label(RichText::new("F.Left:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_footer_left).hint_text("{page}"))
                .on_hover_text("Left (verso) footer text");
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("F.Right:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_footer_right).hint_text("Confidential"))
                .on_hover_text("Right (recto) footer text");
        });
    } else {
        // Single-sided footer
        ui.horizontal(|ui| {
            ui.label(RichText::new("Footer:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_footer_single).hint_text("{page}"))
                .on_hover_text("Single-sided footer text");
        });
    }

    // 5. Decorative Objects (Rules & Accent Bar)
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.checkbox(&mut st.hf_header_rule, RichText::new("H.Rule").size(10.5).color(t.text)).on_hover_text("Thin hairline rule below header text");
        if st.hf_header_rule {
            ui.add(egui::DragValue::new(&mut st.hf_header_rule_weight).range(0.25..=6.0).speed(0.25).suffix("pt"));
        }

        ui.checkbox(&mut st.hf_footer_rule, RichText::new("F.Rule").size(10.5).color(t.text)).on_hover_text("Thin hairline rule above footer text");
        if st.hf_footer_rule {
            ui.add(egui::DragValue::new(&mut st.hf_footer_rule_weight).range(0.25..=6.0).speed(0.25).suffix("pt"));
        }
    });

    ui.horizontal(|ui| {
        ui.checkbox(&mut st.hf_top_accent_bar, RichText::new("Top Accent Bar").size(10.5).color(t.text))
            .on_hover_text("Colored geometric accent bar across the top page edge");
        if st.hf_top_accent_bar {
            ui.add(egui::DragValue::new(&mut st.hf_top_accent_bar_height).range(1.0..=36.0).speed(0.5).suffix("pt"))
                .on_hover_text("Height of top accent bar");
            ui.checkbox(&mut st.hf_top_accent_bar_bleed, RichText::new("Bleed").size(10.0).color(t.text_dim))
                .on_hover_text("Extend accent bar into bleed area");
        }
    });

    // 6. Template Colors
    ui.horizontal(|ui| {
        ui.label(RichText::new("Colors:").size(10.5).color(t.text_dim));
        let field_w = ((ui.available_width() - 40.0) / 4.0 - 4.0).clamp(38.0, 60.0);
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_color_primary))
            .on_hover_text("Primary / Accent Bar color hex (e.g. #0072bc)");
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_color_accent))
            .on_hover_text("Accent color hex (e.g. #f5ab19)");
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_color_rule))
            .on_hover_text("Hairline rule color hex (e.g. #cbd5e1)");
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_color_text))
            .on_hover_text("Text color hex (e.g. #334155)");
    });

    // 7. Save As New Template (Collapsible / Toggleable)
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        let toggle_text = if st.hf_show_save_row { "▾ Save As New Template…" } else { "▸ Save As New Template…" };
        if ui.button(RichText::new(toggle_text).size(10.5).color(t.text_dim)).clicked() {
            st.hf_show_save_row = !st.hf_show_save_row;
        }
    });

    if st.hf_show_save_row {
        ui.horizontal(|ui| {
            let avail = (ui.available_width() - 84.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_custom_name).hint_text("Template Name…"));
            let save_btn = ui.button(RichText::new("Save").size(11.0).color(t.text_strong));
            if save_btn.clicked() {
                match execute_save_hf_template(app, st) {
                    Ok(msg) => st.hf_status = Some((msg, true)),
                    Err(err) => st.hf_status = Some((err, false)),
                }
            }
        });
    }

    // 8. Action Buttons (Apply & Remove)
    ui.add_space(4.0);
    let fw = full_width(ui);
    let apply_btn =
        outline_button(ui, "Apply Header & Footer", fw).on_hover_text("Applies the current header & footer template configuration to the document");
    if apply_btn.clicked() {
        let res = execute_apply_header_footer(app, st);
        st.hf_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(err) => (err, false),
        });
    }

    ui.horizontal(|ui| {
        if ui.button(RichText::new("Remove from Document").size(10.5).color(t.text_dim)).clicked() {
            match execute_remove_header_footer(app) {
                Ok(msg) => st.hf_status = Some((msg, true)),
                Err(err) => st.hf_status = Some((err, false)),
            }
        }
    });

    // Status message
    if let Some((msg, ok)) = &st.hf_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

pub fn execute_apply_header_footer(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    // Same shapes the Header/Footer… dialog sends: "all", {"section": 0-based index}, {"range": [first, last]}.
    let scope = match st.hf_scope {
        1 => json!({ "section": st.hf_scope_section }),
        2 => {
            let start = st.hf_scope_start.max(1) as u64;
            let end = (st.hf_scope_end as u64).max(start);
            json!({ "range": [start, end] })
        }
        _ => json!("all"),
    };
    let mut params = json!({
        "preset": st.hf_style,
        "sided": if st.hf_sided == 1 { "single" } else { "double" },
        "scope": scope,
        "headerOffset": st.hf_header_offset,
        "footerOffset": st.hf_footer_offset,
        "header": {
            "single": st.hf_header_single,
            "left": st.hf_header_left,
            "right": st.hf_header_right,
        },
        "footer": {
            "single": st.hf_footer_single,
            "left": st.hf_footer_left,
            "right": st.hf_footer_right,
        },
        "objects": {
            "headerRule": st.hf_header_rule,
            "headerRuleWeight": st.hf_header_rule_weight,
            "footerRule": st.hf_footer_rule,
            "footerRuleWeight": st.hf_footer_rule_weight,
            "topAccentBar": st.hf_top_accent_bar,
            "topAccentBarHeight": st.hf_top_accent_bar_height,
            "topAccentBarBleed": st.hf_top_accent_bar_bleed,
        },
        "colors": {
            "primary": st.hf_color_primary,
            "accent": st.hf_color_accent,
            "dark": st.hf_color_dark,
            "ruleColor": st.hf_color_rule,
            "textColor": st.hf_color_text,
        }
    });

    let doc_margins = app.session.active().map(|s| detect_document_margins(&s.doc));
    if let Some(m) = doc_margins
        && let Some(obj) = params.as_object_mut()
    {
        obj.insert(
            "margins".to_string(),
            json!({
                "top": m.top,
                "bottom": m.bottom,
                "inside": m.inside,
                "outside": m.outside,
            }),
        );
    }

    app.run("layout.headerFooter.apply", params)?;
    Ok(format!("Header & footer template '{}' applied successfully", st.hf_style))
}

pub fn execute_remove_header_footer(app: &mut DesignApp) -> Result<String, String> {
    app.run("layout.headerFooter.remove", json!({}))?;
    Ok("Header and footer removed from document".to_string())
}

pub fn execute_save_hf_template(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let name = st.hf_custom_name.trim().to_string();
    if name.is_empty() {
        return Err("Please enter a template name".to_string());
    }
    let template = json!({
        "name": name,
        "description": "User custom template",
        "sided": if st.hf_sided == 1 { "single" } else { "double" },
        "headerOffset": st.hf_header_offset,
        "footerOffset": st.hf_footer_offset,
        "header": {
            "single": st.hf_header_single,
            "left": st.hf_header_left,
            "right": st.hf_header_right,
        },
        "footer": {
            "single": st.hf_footer_single,
            "left": st.hf_footer_left,
            "right": st.hf_footer_right,
        },
        "objects": {
            "headerRule": st.hf_header_rule,
            "headerRuleWeight": st.hf_header_rule_weight,
            "footerRule": st.hf_footer_rule,
            "footerRuleWeight": st.hf_footer_rule_weight,
            "topAccentBar": st.hf_top_accent_bar,
            "topAccentBarHeight": st.hf_top_accent_bar_height,
            "topAccentBarBleed": st.hf_top_accent_bar_bleed,
        },
        "colors": {
            "primary": st.hf_color_primary,
            "accent": st.hf_color_accent,
            "dark": st.hf_color_dark,
            "ruleColor": st.hf_color_rule,
            "textColor": st.hf_color_text,
        }
    });

    app.run("layout.headerFooter.saveTemplate", json!({ "template": template }))?;
    st.hf_style = name.to_string();
    st.hf_custom_name.clear();
    Ok(format!("Template '{name}' saved successfully"))
}

pub fn execute_delete_hf_template(app: &mut DesignApp, st: &mut PubgenState, name: &str) -> Result<String, String> {
    app.run("layout.headerFooter.deleteTemplate", json!({ "name": name }))?;
    st.hf_style = "Modern Minimal".to_string();
    Ok(format!("Template '{name}' deleted"))
}

fn add_cover_text_frame(
    app: &mut DesignApp,
    spread: usize,
    rect: [f64; 4],
    text: &str,
    color: &str,
    font_family: &str,
    font_style: &str,
    size: f64,
    leading: Option<f64>,
    tracking: Option<f64>,
    align: &str,
) -> Result<u64, String> {
    let res = app
        .run(
            "frame.create",
            json!({
                "spread": spread,
                "rect": rect,
                "content": "text",
                "text": text,
                "caret": false,
            }),
        )
        .map_err(|e| format!("frame.create failed for '{text}': {e}"))?;

    let id = res.get("id").and_then(Value::as_u64).ok_or("Missing frame id")?;
    let story = res.get("story").and_then(Value::as_u64);

    let mut char_attrs = json!({
        "fontFamily": font_family,
        "fontStyle": font_style,
        "size": size,
    });
    if let Some(lead) = leading {
        char_attrs["leading"] = json!({"kind": "points", "value": lead});
    }
    if let Some(track) = tracking {
        char_attrs["tracking"] = json!(track);
    }

    if let Some(sid) = story {
        let _ = app.run("text.select", json!({"story": sid, "anchor": 0, "focus": text.len()}));
        let _ = app.run("object.color", json!({"color": color, "text": true}));
        let _ = app.run("type.char", json!({"attrs": char_attrs.clone()}));
        let _ = app.run("type.para", json!({"attrs": {"align": align}}));
    }

    let _ = app.run("selection.set", json!({"ids": [id]}));
    let _ = app.run("object.color", json!({"color": color, "text": true}));
    let _ = app.run("type.char", json!({"attrs": char_attrs}));
    let _ = app.run("type.para", json!({"attrs": {"align": align}}));

    Ok(id)
}

/// The world bounds of one item, read back through `document.inspect`.
fn item_bounds(app: &mut DesignApp, id: u64) -> Option<[f64; 4]> {
    fn scan(v: &Value, id: u64) -> Option<[f64; 4]> {
        let b = v.get("bounds").and_then(Value::as_array)?;
        if v.get("id").and_then(Value::as_u64) != Some(id) || b.len() != 4 {
            return None;
        }
        Some([b[0].as_f64()?, b[1].as_f64()?, b[2].as_f64()?, b[3].as_f64()?])
    }

    fn walk(v: &Value, id: u64) -> Option<[f64; 4]> {
        if let Some(b) = scan(v, id) {
            return Some(b);
        }
        match v {
            Value::Array(items) => items.iter().find_map(|it| walk(it, id)),
            Value::Object(obj) => obj.values().find_map(|it| walk(it, id)),
            _ => None,
        }
    }

    walk(&app.run("document.inspect", json!({})).ok()?, id)
}

/// The Pubgen sample-image folder, found from the working directory or the crate location
/// (tests run with the crate dir as the working directory, the app from the repo root).
fn sample_images_dir() -> Option<std::path::PathBuf> {
    let rel = std::path::Path::new("plan").join("pubgen").join("samples").join("images");
    if rel.is_dir() {
        return Some(rel);
    }
    let from_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(&rel);
    if from_manifest.is_dir() { Some(from_manifest) } else { None }
}

/// Resolve a cover image path: as given, or the same file name in the Pubgen sample images
/// (so the default `outputs/...` paths also resolve when the app runs from another directory).
fn resolve_cover_image(path: &str) -> String {
    if std::path::Path::new(path).is_file() {
        return path.to_string();
    }
    if let Some(name) = std::path::Path::new(path).file_name()
        && let Some(dir) = sample_images_dir()
    {
        let sample = dir.join(name);
        if sample.is_file() {
            return sample.to_string_lossy().to_string();
        }
    }
    path.to_string()
}

fn resolve_template_path(path: &str) -> Option<std::path::PathBuf> {
    let p = std::path::Path::new(path);
    if p.is_file() {
        return Some(p.to_path_buf());
    }
    let from_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(path);
    if from_manifest.is_file() {
        return Some(from_manifest);
    }
    None
}

fn execute_generate_cover(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    let title_text = if st.cover_title.trim().is_empty() { "KARAWANG TIMUR" } else { st.cover_title.trim() };
    // The second headline line has its own frame in the cover templates (`title#2`), styled
    // smaller and in another colour than the headline above it. An empty field leaves that
    // frame blank.
    let title2_text = st.cover_title2.trim();
    let year_text = if st.cover_year.trim().is_empty() { "2026" } else { st.cover_year.trim() };
    let volume_text = if st.cover_volume.trim().is_empty() { "Volume 23 2026" } else { st.cover_volume.trim() };

    let cat_raw = if st.cover_catalogue.trim().is_empty() { "1102001.3215113" } else { st.cover_catalogue.trim() };
    let cat_text = if cat_raw.to_lowercase().starts_with("katalog") { cat_raw.to_string() } else { format!("Katalog BPS: {cat_raw}") };

    let code_type = if st.cover_code_type == 0 { "ISSN" } else { "ISBN" };
    let code_num = if st.cover_code_num.trim().is_empty() { "2828-7436" } else { st.cover_code_num.trim() };
    let code_text = format!("{code_type} {code_num}");

    let primary = if st.cover_color_primary.trim().is_empty() { "#0072bc" } else { st.cover_color_primary.trim() };
    let accent = if st.cover_color_accent.trim().is_empty() { "#f5ab19" } else { st.cover_color_accent.trim() };
    let dark = if st.cover_color_dark.trim().is_empty() { "#122647" } else { st.cover_color_dark.trim() };

    let skyline =
        resolve_cover_image(if st.cover_image_skyline.trim().is_empty() { "outputs/city_skyline.jpg" } else { st.cover_image_skyline.trim() });
    let landscape =
        resolve_cover_image(if st.cover_image_landscape.trim().is_empty() { "outputs/city_landscape.jpg" } else { st.cover_image_landscape.trim() });
    let logo = resolve_cover_image(if st.cover_image_logo.trim().is_empty() { "outputs/bps_logo.png" } else { st.cover_image_logo.trim() });

    let agency_text = if st.cover_agency_text.trim().is_empty() { "BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG" } else { st.cover_agency_text.trim() };

    // The hand-designed covers in `covers/`: A4 artboards with the artwork's transforms intact
    // and every text/photo frame carrying a `#slot:` tag (a headline split over two frames uses
    // `title` and `title#2`). `covers_original/` keeps the untouched sources.
    let canonical_templates = [
        "assets/templates/covers/000_wave_curves.designcraft",
        "assets/templates/covers/001_geometric_medallion.designcraft",
        "assets/templates/covers/002_folded_origami.designcraft",
        "assets/templates/covers/003_chevron_medallion.designcraft",
        "assets/templates/covers/004_honeycomb_hexagon.designcraft",
        "assets/templates/covers/005_corporate_headline.designcraft",
        "assets/templates/covers/006_diagonal_arrow.designcraft",
        "assets/templates/covers/007_modern_horizon.designcraft",
        "assets/templates/covers/008_business_report.designcraft",
        "assets/templates/covers/009_urban_angle.designcraft",
    ];

    if let Some(&tpl_path) = canonical_templates.get(st.cover_style)
        && let Some(tpl_file) = resolve_template_path(tpl_path)
    {
        let tpl_path_str = tpl_file.to_string_lossy().to_string();
        let mut text_slots = serde_json::Map::new();
        text_slots.insert("title".into(), json!(title_text));
        text_slots.insert("title#2".into(), json!(title2_text));
        if st.bilingual_output && !st.cover_title_en.trim().is_empty() {
            text_slots.insert("title_en".into(), json!(st.cover_title_en.trim()));
        }
        text_slots.insert("year".into(), json!(year_text));
        text_slots.insert("volume".into(), json!(volume_text));
        text_slots.insert("catalogue".into(), json!(cat_text));
        text_slots.insert("code".into(), json!(code_text));
        text_slots.insert("agency".into(), json!(agency_text));

        let mut apply_params = json!({
            "path": tpl_path_str,
            "before": 0,
            "slots": {
                "text": text_slots,
                "image": {
                    "hero": skyline,
                    "landscape": landscape,
                    "logo": logo,
                }
            },
            "palette": {
                "Primary": primary,
                "Accent": accent,
                "Dark": dark,
            }
        });
        if !st.doc_language.trim().is_empty() {
            apply_params["lang"] = json!(st.doc_language.trim());
        }

        let res = app.run("template.page.apply", apply_params);
        if res.is_ok() {
            crate::canvas::go_to_page(app, 0);
            let _ = app.session.edit(|doc, _| {
                if let Some(sp) = doc.spreads.first_mut() {
                    let sp = std::sync::Arc::make_mut(sp);
                    if let Some(pg) = sp.pages.first_mut() {
                        pg.margins = designcraft_doc::Margins::uniform(0.0);
                        pg.local_margins = true;
                    }
                    sp.relayout();
                }
                Ok(serde_json::Value::Null)
            });
            let style_names = [
                "Wave Curves",
                "Geometric Medallion",
                "Folded Origami",
                "Chevron & Medallion",
                "Honeycomb Hexagon",
                "Corporate Headline",
                "Diagonal Arrow",
                "Modern Horizon",
                "Business Report",
                "Urban Angle",
            ];
            let sname = style_names.get(st.cover_style).copied().unwrap_or("Template Cover");
            return Ok(format!("Cover page generated successfully ({sname})"));
        }
    }

    // 1. Automatically insert a new page before the first page (Page 1)
    app.run("layout.pages.insert", json!({"before": 0, "count": 1})).map_err(|e| format!("Failed to insert cover page: {e}"))?;
    crate::canvas::go_to_page(app, 0);
    let _ = app.session.edit(|doc, _| {
        if let Some(sp) = doc.spreads.first_mut() {
            let sp = std::sync::Arc::make_mut(sp);
            if let Some(pg) = sp.pages.first_mut() {
                pg.margins = designcraft_doc::Margins::uniform(0.0);
                pg.local_margins = true;
            }
            sp.relayout();
        }
        Ok(serde_json::Value::Null)
    });

    let (pw, ph) = {
        let active = app.session.active().ok_or("No active document")?;
        let spread = active.doc.spreads.first().ok_or("Spread missing")?;
        let page = spread.pages.first().ok_or("Page missing")?;
        (page.width, page.height)
    };

    let style_name = match st.cover_style {
        0 => {
            // Style 0: Wave Curves
            // 1. Dark charcoal/theme full background
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": dark, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 2. Concentric Bottom Cradle
            app.run("frame.create", json!({"spread": 0, "rect": [123.64, 246.0, 471.64, 594.0], "content": "unassigned", "shape": "ellipse"}))?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.3}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            app.run("frame.create", json!({"spread": 0, "rect": [130.64, 253.0, 464.64, 587.0], "content": "unassigned", "shape": "ellipse"}))?;
            let _ = app.run("object.color", json!({"color": adjust_hex(accent, 1.15), "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 3. Blue Wave Swoosh
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [pw, 280.0]}, {"p": [pw, 430.0], "in": [pw, 380.0], "out": [510.0, 490.0]},
                    {"p": [380.0, 565.0], "in": [450.0, 545.0], "out": [320.0, 580.0]},
                    {"p": [240.0, 560.0], "in": [270.0, 568.0], "out": [240.0, 560.0]},
                    {"p": [330.0, 545.0], "in": [260.0, 550.0], "out": [430.0, 500.0]},
                    {"p": [pw, 280.0], "in": [490.0, 390.0], "out": [pw, 280.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 8, "size": 16, "opacity": 0.32}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 4. White Top Wave
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 0.0]}, {"p": [pw, 0.0]}, {"p": [pw, 280.0], "out": [490.0, 390.0]},
                    {"p": [330.0, 545.0], "in": [430.0, 500.0], "out": [260.0, 550.0]},
                    {"p": [160.0, 542.0], "in": [200.0, 545.0], "out": [80.0, 530.0]},
                    {"p": [0.0, 490.0], "in": [50.0, 505.0]}, {"p": [0.0, 0.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 8, "size": 16, "opacity": 0.22}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 5. Circular Medallion Rings
            app.run("frame.create", json!({"spread": 0, "rect": [145.64, 268.0, 449.64, 572.0], "content": "unassigned", "shape": "ellipse"}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.25}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            app.run("frame.create", json!({"spread": 0, "rect": [151.64, 274.0, 443.64, 566.0], "content": "unassigned", "shape": "ellipse"}))?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            app.run("frame.create", json!({"spread": 0, "rect": [158.64, 281.0, 436.64, 559.0], "content": "unassigned", "shape": "ellipse"}))?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // Photo frame
            app.run("frame.create", json!({"spread": 0, "rect": [161.64, 284.0, 433.64, 556.0], "content": "graphic", "shape": "ellipse"}))?;
            let _ = app.run("file.place", json!({"path": skyline}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 6. Top-Right Metadata (#5 and #6)
            add_cover_text_frame(app, 0, [320.0, 24.0, 555.0, 42.0], &cat_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;
            add_cover_text_frame(app, 0, [320.0, 42.0, 555.0, 60.0], &code_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;

            // 7. Headline Block
            add_cover_text_frame(
                app,
                0,
                [20.0, 126.0, 575.0, 186.0],
                title_text,
                primary,
                "Arial Black",
                "Regular",
                33.0,
                Some(39.0),
                Some(30.0),
                "center",
            )?;
            // The second headline line: smaller, and in the accent colour.
            add_cover_text_frame(
                app,
                0,
                [20.0, 186.0, 575.0, 225.0],
                title2_text,
                accent,
                "Arial Black",
                "Regular",
                24.0,
                Some(28.0),
                Some(30.0),
                "center",
            )?;

            // 8. Bottom-Left BPS Logo
            app.run("frame.create", json!({"spread": 0, "rect": [40.0, 742.0, 84.0, 788.0], "content": "graphic", "shape": "rectangle"}))?;
            let _ = app.run("file.place", json!({"path": logo}));
            let _ = app.run("object.fit", json!({"mode": "fitProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            add_cover_text_frame(
                app,
                0,
                [92.0, 746.0, 290.0, 788.0],
                agency_text,
                "#ffffff",
                "Arial",
                "Bold Italic",
                10.0,
                Some(13.5),
                Some(20.0),
                "left",
            )?;

            // 9. Bottom-Right Year & Volume
            add_cover_text_frame(app, 0, [320.0, 712.0, 555.0, 768.0], year_text, accent, "Arial Black", "Regular", 48.0, None, Some(30.0), "right")?;
            add_cover_text_frame(app, 0, [320.0, 768.0, 555.0, 795.0], volume_text, "#cbd5e1", "Arial", "Bold", 12.0, None, Some(20.0), "right")?;

            "Wave Curves"
        }
        1 => {
            // Style 1: Geometric & Medallion
            // 1. Clean White Base
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 2. Top-Right Metadata (#5 and #6)
            add_cover_text_frame(app, 0, [320.0, 24.0, 555.0, 42.0], &cat_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;
            add_cover_text_frame(app, 0, [320.0, 42.0, 555.0, 60.0], &code_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;

            // 3. Diagonal Polygon Graphic Frame (Landscape)
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 390.0]}, {"p": [170.0, 230.0]}, {"p": [410.0, 230.0]},
                    {"p": [pw, 75.0]}, {"p": [pw, 550.0]}, {"p": [340.0, 550.0]},
                    {"p": [160.0, 580.0]}, {"p": [0.0, 680.0]}
                ]}),
            )?;
            let _ = app.run("object.content", json!({"type": "graphic"}));
            let _ = app.run("file.place", json!({"path": landscape}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // Multiply color overlay
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 390.0]}, {"p": [170.0, 230.0]}, {"p": [410.0, 230.0]},
                    {"p": [pw, 75.0]}, {"p": [pw, 550.0]}, {"p": [340.0, 550.0]},
                    {"p": [160.0, 580.0]}, {"p": [0.0, 680.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.opacity", json!({"opacity": 0.82, "blend": "multiply"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // Accent stripes
            app.run("path.create", json!({"spread": 0, "closed": false, "anchors": [{"p": [10.0, 365.0]}, {"p": [155.0, 222.0]}]}))?;
            let _ = app.run("object.stroke", json!({"weight": 7.0}));
            let _ = app.run("object.color", json!({"color": accent, "target": "stroke"}));

            app.run("path.create", json!({"spread": 0, "closed": false, "anchors": [{"p": [435.0, 220.0]}, {"p": [585.0, 95.0]}]}))?;
            let _ = app.run("object.stroke", json!({"weight": 7.0}));
            let _ = app.run("object.color", json!({"color": accent, "target": "stroke"}));

            // Bottom-left accent ribbon
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 680.0]}, {"p": [155.0, 575.0]}, {"p": [170.0, 590.0]}, {"p": [0.0, 715.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // Ribbon fold
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [140.0, 588.0]}, {"p": [170.0, 590.0]}, {"p": [162.0, 565.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": adjust_hex(accent, 0.85), "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // Circular photo cutout
            app.run("frame.create", json!({"spread": 0, "rect": [217.0, 292.0, 533.0, 608.0], "content": "unassigned", "shape": "ellipse"}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 8, "size": 16, "opacity": 0.28}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            app.run("frame.create", json!({"spread": 0, "rect": [223.0, 298.0, 527.0, 602.0], "content": "graphic", "shape": "ellipse"}))?;
            let _ = app.run("file.place", json!({"path": skyline}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // Accent arc around circle
            app.run(
                "path.create",
                json!({"spread": 0, "closed": false, "anchors": [
                    {"p": [340.0, 615.0], "out": [390.0, 625.0]},
                    {"p": [480.0, 595.0], "in": [435.0, 620.0]}
                ]}),
            )?;
            let _ = app.run("object.stroke", json!({"weight": 5.0}));
            let _ = app.run("object.color", json!({"color": accent, "target": "stroke"}));

            // Headline Block
            add_cover_text_frame(app, 0, [35.0, 90.0, 440.0, 142.0], title_text, accent, "Arial Black", "Regular", 34.0, None, Some(30.0), "left")?;
            add_cover_text_frame(app, 0, [35.0, 142.0, 440.0, 195.0], title2_text, dark, "Arial Black", "Regular", 24.0, None, Some(30.0), "left")?;

            // Bottom-Left Logo
            app.run("frame.create", json!({"spread": 0, "rect": [35.0, 742.0, 82.0, 788.0], "content": "graphic", "shape": "rectangle"}))?;
            let _ = app.run("file.place", json!({"path": logo}));
            let _ = app.run("object.fit", json!({"mode": "fitProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            add_cover_text_frame(
                app,
                0,
                [90.0, 746.0, 280.0, 788.0],
                agency_text,
                dark,
                "Arial",
                "Bold Italic",
                10.0,
                Some(13.5),
                Some(20.0),
                "left",
            )?;

            // Bottom-Right Year & Volume
            add_cover_text_frame(app, 0, [320.0, 712.0, 560.0, 768.0], year_text, accent, "Arial Black", "Regular", 48.0, None, Some(30.0), "right")?;
            add_cover_text_frame(app, 0, [320.0, 768.0, 560.0, 795.0], volume_text, dark, "Arial", "Bold", 12.0, None, Some(20.0), "right")?;

            "Geometric & Medallion"
        }
        2 => {
            // Style 2: Folded Origami
            // 1. Clean White Base
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 2. Photo Polygon Frame (Top-Right Hexagonal Cutout)
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [235.0, 0.0]}, {"p": [pw, 0.0]}, {"p": [pw, 290.0]},
                    {"p": [325.0, 405.0]}, {"p": [105.0, 235.0]}, {"p": [105.0, 75.0]}
                ]}),
            )?;
            let _ = app.run("object.content", json!({"type": "graphic"}));
            let _ = app.run("file.place", json!({"path": skyline}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 3. Ribbon Border along Photo Lower Edge
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [105.0, 235.0]}, {"p": [325.0, 405.0]}, {"p": [pw, 290.0]},
                    {"p": [pw, 305.0]}, {"p": [325.0, 420.0]}, {"p": [105.0, 248.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 4. Top-Left Accent Corner
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 0.0]}, {"p": [235.0, 0.0]}, {"p": [105.0, 75.0]},
                    {"p": [105.0, 160.0]}, {"p": [0.0, 95.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 5. Dark Navy / Theme Dark Polygon
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 95.0]}, {"p": [105.0, 160.0]}, {"p": [105.0, 248.0]},
                    {"p": [265.0, 360.0]}, {"p": [145.0, 435.0]}, {"p": [0.0, 350.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": dark, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 6. Amber / Shaded Facet on Left
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 350.0]}, {"p": [145.0, 435.0]}, {"p": [0.0, 500.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": adjust_hex(accent, 0.90), "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 7. Folded Origami Ribbon Facets
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [145.0, 435.0]}, {"p": [265.0, 360.0]}, {"p": [325.0, 420.0]}, {"p": [325.0, 540.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": adjust_hex(accent, 0.98), "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [325.0, 420.0]}, {"p": [pw, 305.0]}, {"p": [pw, 350.0]},
                    {"p": [460.0, 460.0]}, {"p": [325.0, 540.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": adjust_hex(accent, 1.08), "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [460.0, 460.0]}, {"p": [pw, 350.0]}, {"p": [pw, 540.0]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": adjust_hex(accent, 0.92), "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 8. Top-Right Metadata (#5 and #6)
            add_cover_text_frame(app, 0, [320.0, 24.0, 555.0, 42.0], &cat_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;
            add_cover_text_frame(app, 0, [320.0, 42.0, 555.0, 60.0], &code_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;

            // 9. Headline Block
            add_cover_text_frame(app, 0, [40.0, 597.0, 550.0, 647.0], title_text, primary, "Arial Black", "Regular", 38.0, None, Some(20.0), "left")?;
            add_cover_text_frame(app, 0, [40.0, 647.0, 550.0, 697.0], title2_text, accent, "Arial Black", "Regular", 28.0, None, Some(20.0), "left")?;

            // 10. Bottom-Left Logo
            app.run("frame.create", json!({"spread": 0, "rect": [40.0, 742.0, 84.0, 788.0], "content": "graphic", "shape": "rectangle"}))?;
            let _ = app.run("file.place", json!({"path": logo}));
            let _ = app.run("object.fit", json!({"mode": "fitProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            add_cover_text_frame(
                app,
                0,
                [92.0, 746.0, 290.0, 788.0],
                agency_text,
                dark,
                "Arial",
                "Bold Italic",
                10.0,
                Some(13.5),
                Some(20.0),
                "left",
            )?;

            // 11. Bottom-Right Year & Volume
            add_cover_text_frame(app, 0, [320.0, 712.0, 555.0, 768.0], year_text, accent, "Arial Black", "Regular", 48.0, None, Some(30.0), "right")?;
            add_cover_text_frame(app, 0, [320.0, 768.0, 555.0, 795.0], volume_text, dark, "Arial", "Bold", 12.0, None, Some(20.0), "right")?;

            "Folded Origami"
        }
        3 => {
            // Style 3: Chevron & Medallion (from design-1.designcraft)
            let sx = pw / 595.275591;
            let sy = ph / 841.889764;
            let s_avg = (sx + sy) / 2.0;

            // 1. Crisp white base (Item 11)
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 2. Mid-body subtle tint (Item 12)
            app.run("frame.create", json!({"spread": 0, "rect": [0, 117.84 * sy, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": "#f8fafc", "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 3. Oval Photo Medallion (Item 17) - Resized & placed behind chevron
            app.run(
                "frame.create",
                json!({
                    "spread": 0,
                    "rect": [162.68 * sx, 339.71 * sy, 437.95 * sx, 629.56 * sy],
                    "content": "graphic",
                    "shape": "ellipse"
                }),
            )?;
            let _ = app.run("file.place", json!({"path": skyline}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 4. Downward Chevron (Item 13)
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.71 * sx, 0.0], "out": [151.68 * sx, 0.0]},
                    {"p": [pw, 0.0], "in": [377.97 * sx, 0.0], "out": [pw, 42.97 * sy]},
                    {"p": [pw, 147.47 * sy], "in": [pw, 94.38 * sy], "out": [449.39 * sx, 292.88 * sy]},
                    {"p": [294.74 * sx, 447.03 * sy]},
                    {"p": [0.71 * sx, 153.94 * sy], "in": [126.35 * sx, 279.18 * sy], "out": [0.71 * sx, 107.50 * sy]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.22}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 5. Accent bottom stroke along Chevron (Item 14)
            app.run(
                "path.create",
                json!({"spread": 0, "closed": false, "anchors": [
                    {"p": [0.71 * sx, 153.94 * sy], "out": [126.35 * sx, 279.18 * sy]},
                    {"p": [294.74 * sx, 447.03 * sy]},
                    {"p": [pw, 147.47 * sy], "in": [449.39 * sx, 292.88 * sy]}
                ]}),
            )?;
            let _ = app.run("object.stroke", json!({"weight": 4.25 * s_avg}));
            let _ = app.run("object.color", json!({"color": accent, "target": "stroke"}));

            // 6. Horseshoe Medallion Ring (Item 15)
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [116.22 * sx, 484.53 * sy], "in": [116.22 * sx, 585.77 * sy], "out": [116.22 * sx, 428.43 * sy]},
                    {"p": [181.35 * sx, 344.59 * sy], "in": [141.51 * sx, 378.21 * sy], "out": [188.35 * sx, 351.56 * sy]},
                    {"p": [202.65 * sx, 365.80 * sy], "in": [195.50 * sx, 358.69 * sy], "out": [168.58 * sx, 393.82 * sy]},
                    {"p": [146.87 * sx, 483.69 * sy], "in": [146.87 * sx, 436.22 * sy], "out": [146.87 * sx, 568.05 * sy]},
                    {"p": [300.12 * sx, 636.43 * sy], "in": [215.48 * sx, 636.43 * sy], "out": [384.74 * sx, 636.43 * sy]},
                    {"p": [453.36 * sx, 483.69 * sy], "in": [453.36 * sx, 568.05 * sy], "out": [453.36 * sx, 433.45 * sy]},
                    {"p": [391.47 * sx, 361.04 * sy], "in": [429.02 * sx, 388.87 * sy], "out": [400.42 * sx, 352.11 * sy]},
                    {"p": [412.85 * sx, 339.71 * sy], "in": [407.95 * sx, 344.59 * sy], "out": [456.14 * sx, 373.24 * sy]},
                    {"p": [484.01 * sx, 484.53 * sy], "in": [484.01 * sx, 425.64 * sy], "out": [484.01 * sx, 585.77 * sy]},
                    {"p": [300.12 * sx, 667.83 * sy], "in": [401.67 * sx, 667.83 * sy], "out": [198.55 * sx, 667.83 * sy]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 6, "size": 16, "opacity": 0.28}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 7. Bottom Footer Bar (Item 19)
            app.run("frame.create", json!({"spread": 0, "rect": [0.0, 757.00 * sy, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": dark, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 8. Footer Top Accent Stroke (Item 20)
            app.run(
                "path.create",
                json!({"spread": 0, "closed": false, "anchors": [
                    {"p": [0.0, 757.00 * sy]},
                    {"p": [pw, 757.00 * sy]}
                ]}),
            )?;
            let _ = app.run("object.stroke", json!({"weight": 3.5 * sy}));
            let _ = app.run("object.color", json!({"color": accent, "target": "stroke"}));

            // 9. Top-Right Metadata (Items 21 & 23)
            add_cover_text_frame(
                app,
                0,
                [311.49 * sx, 22.63 * sy, 569.74 * sx, 42.43 * sy],
                &cat_text,
                "#ffffff",
                "Arial",
                "Bold",
                8.0 * s_avg,
                None,
                None,
                "right",
            )?;
            add_cover_text_frame(
                app,
                0,
                [311.49 * sx, 42.43 * sy, 569.74 * sx, 62.23 * sy],
                &code_text,
                "#ffffff",
                "Arial",
                "Bold",
                8.0 * s_avg,
                None,
                None,
                "right",
            )?;

            // 10. Headline Block Inside Chevron (Items 25 & 27)
            add_cover_text_frame(
                app,
                0,
                [21.28 * sx, 201.46 * sy, 573.99 * sx, 275.00 * sy],
                title_text,
                "#ffffff",
                "Arial Black",
                "Regular",
                26.0 * s_avg,
                Some(31.0 * s_avg),
                Some(20.0),
                "center",
            )?;
            // The second headline line: smaller, and in the accent colour.
            add_cover_text_frame(
                app,
                0,
                [21.28 * sx, 275.00 * sy, 573.99 * sx, 330.00 * sy],
                title2_text,
                accent,
                "Arial Black",
                "Regular",
                19.0 * s_avg,
                Some(23.0 * s_avg),
                Some(20.0),
                "center",
            )?;

            // 11. Center Medallion Year (Item 33) - 65pt Arial Black in Accent Gold!
            add_cover_text_frame(
                app,
                0,
                [160.0 * sx, 440.0 * sy, 440.0 * sx, 530.0 * sy],
                year_text,
                accent,
                "Arial Black",
                "Regular",
                65.0 * s_avg,
                None,
                Some(20.0),
                "center",
            )?;

            // 12. Bottom-Left Logo & Agency Text (Items 29 & 31)
            app.run(
                "frame.create",
                json!({
                    "spread": 0,
                    "rect": [28.38 * sx, 770.79 * sy, 82.30 * sx, 827.36 * sy],
                    "content": "graphic",
                    "shape": "rectangle"
                }),
            )?;
            let _ = app.run("file.place", json!({"path": logo}));
            let _ = app.run("object.fit", json!({"mode": "fitProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            add_cover_text_frame(
                app,
                0,
                [93.65 * sx, 773.61 * sy, 368.92 * sx, 830.19 * sy],
                agency_text,
                "#ffffff",
                "Arial",
                "Bold Italic",
                9.5 * s_avg,
                Some(12.5 * s_avg),
                Some(15.0),
                "left",
            )?;

            // 13. Bottom-Right Volume (Item 35)
            add_cover_text_frame(
                app,
                0,
                [368.92 * sx, 766.80 * sy, 569.74 * sx, 810.0 * sy],
                volume_text,
                "#cbd5e1",
                "Arial",
                "Bold",
                9.0 * s_avg,
                None,
                Some(15.0),
                "right",
            )?;

            "Chevron & Medallion"
        }
        4 => {
            // Style 4: Honeycomb Hexagon (from design-2.designcraft)
            let sx = pw / 595.275591;
            let sy = ph / 841.889764;
            let s_avg = (sx + sy) / 2.0;

            // 1. Deep Midnight Executive Background (Item 24)
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": dark, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 2. Three Vertical Accent Stripes (Items 26, 27, 28)
            app.run(
                "frame.create",
                json!({"spread": 0, "rect": [355.03 * sx, 0.0, 407.97 * sx, ph], "content": "unassigned", "shape": "rectangle"}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.opacity", json!({"opacity": 0.85}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            app.run(
                "frame.create",
                json!({"spread": 0, "rect": [415.03 * sx, 0.0, 443.94 * sx, ph], "content": "unassigned", "shape": "rectangle"}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.opacity", json!({"opacity": 0.60}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            app.run(
                "frame.create",
                json!({"spread": 0, "rect": [451.03 * sx, 0.0, 462.92 * sx, ph], "content": "unassigned", "shape": "rectangle"}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.opacity", json!({"opacity": 0.35}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 3. Left Horizontal Chevron Banner (Item 13) in Corporate Sapphire
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 199.74 * sy]},
                    {"p": [176.59 * sx, 199.74 * sy]},
                    {"p": [300.92 * sx, 324.07 * sy]},
                    {"p": [180.99 * sx, 444.00 * sy]},
                    {"p": [0.0, 444.00 * sy]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.30}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // Gold bottom accent rule along chevron
            app.run(
                "path.create",
                json!({"spread": 0, "closed": false, "anchors": [
                    {"p": [0.0, 444.00 * sy]},
                    {"p": [180.99 * sx, 444.00 * sy]},
                    {"p": [300.92 * sx, 324.07 * sy]}
                ]}),
            )?;
            let _ = app.run("object.stroke", json!({"weight": 4.0 * s_avg}));
            let _ = app.run("object.color", json!({"color": accent, "target": "stroke"}));

            // 4. Right Horizontal Banner Extension (Item 17)
            app.run(
                "frame.create",
                json!({
                    "spread": 0,
                    "rect": [514.35 * sx, 304.60 * sy, pw, 534.00 * sy],
                    "content": "unassigned",
                    "shape": "rectangle"
                }),
            )?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            app.run(
                "path.create",
                json!({"spread": 0, "closed": false, "anchors": [
                    {"p": [514.35 * sx, 534.00 * sy]},
                    {"p": [pw, 534.00 * sy]}
                ]}),
            )?;
            let _ = app.run("object.stroke", json!({"weight": 4.0 * s_avg}));
            let _ = app.run("object.color", json!({"color": accent, "target": "stroke"}));

            // 5. Tri-Hexagon Honeycomb Outer White Matting & Shadow (Item 19)
            let tri_hex_anchors = json!([
                {"p": [180.81 * sx, 424.99 * sy]},
                {"p": [243.85 * sx, 324.44 * sy]},
                {"p": [346.12 * sx, 325.68 * sy]},
                {"p": [408.51 * sx, 223.50 * sy]},
                {"p": [526.58 * sx, 224.97 * sy]},
                {"p": [579.76 * sx, 332.72 * sy]},
                {"p": [522.00 * sx, 428.00 * sy]},
                {"p": [576.26 * sx, 527.77 * sy]},
                {"p": [512.15 * sx, 626.18 * sy]},
                {"p": [395.52 * sx, 624.82 * sy]},
                {"p": [344.69 * sx, 528.28 * sy]},
                {"p": [232.47 * sx, 526.92 * sy]}
            ]);

            app.run("path.create", json!({"spread": 0, "closed": true, "anchors": tri_hex_anchors}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 6, "size": 18, "opacity": 0.35}));
            let _ = app.run("object.stroke", json!({"weight": 8.0 * s_avg}));
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "stroke"}));

            // 6. Tri-Hexagon Photo Frame
            let hex_res = app.run("path.create", json!({"spread": 0, "closed": true, "anchors": tri_hex_anchors}))?;
            let hex_id = hex_res.get("id").and_then(serde_json::Value::as_u64);
            let _ = app.run("object.content", json!({"type": "graphic"}));
            let _ = app.run("file.place", json!({"path": landscape, "frame": hex_id}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally", "ids": [hex_id]}));
            let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [hex_id]}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]", "ids": [hex_id]}));

            // Inner gold piping around Tri-Hexagon
            app.run("path.create", json!({"spread": 0, "closed": true, "anchors": tri_hex_anchors}))?;
            let _ = app.run("object.stroke", json!({"weight": 2.5 * s_avg}));
            let _ = app.run("object.color", json!({"color": accent, "target": "stroke"}));

            // 7. Top-Left Logo & Agency Text
            app.run(
                "frame.create",
                json!({
                    "spread": 0,
                    "rect": [36.0 * sx, 32.0 * sy, 74.0 * sx, 72.0 * sy],
                    "content": "graphic",
                    "shape": "rectangle"
                }),
            )?;
            let _ = app.run("file.place", json!({"path": logo}));
            let _ = app.run("object.fit", json!({"mode": "fitProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            add_cover_text_frame(
                app,
                0,
                [82.0 * sx, 34.0 * sy, 320.0 * sx, 72.0 * sy],
                agency_text,
                "#ffffff",
                "Arial",
                "Bold Italic",
                9.0 * s_avg,
                Some(12.0 * s_avg),
                Some(15.0),
                "left",
            )?;

            // 8. Top-Right Metadata
            add_cover_text_frame(
                app,
                0,
                [pw - 250.0 * sx, 34.0 * sy, pw - 36.0 * sx, 50.0 * sy],
                &cat_text,
                "#ffffff",
                "Arial",
                "Bold",
                8.0 * s_avg,
                None,
                None,
                "right",
            )?;
            add_cover_text_frame(
                app,
                0,
                [pw - 250.0 * sx, 50.0 * sy, pw - 36.0 * sx, 66.0 * sy],
                &code_text,
                "#ffffff",
                "Arial",
                "Bold",
                8.0 * s_avg,
                None,
                None,
                "right",
            )?;

            // 9. Headline Block (Inside Left Chevron Banner)
            add_cover_text_frame(
                app,
                0,
                [36.0 * sx, 265.0 * sy, 270.0 * sx, 340.0 * sy],
                title_text,
                "#ffffff",
                "Arial Black",
                "Regular",
                25.0 * s_avg,
                Some(30.0 * s_avg),
                Some(15.0),
                "left",
            )?;
            // The second headline line: smaller, and in the accent colour.
            add_cover_text_frame(
                app,
                0,
                [36.0 * sx, 340.0 * sy, 270.0 * sx, 395.0 * sy],
                title2_text,
                accent,
                "Arial Black",
                "Regular",
                18.0 * s_avg,
                Some(22.0 * s_avg),
                Some(15.0),
                "left",
            )?;

            // 10. Lower-Left Focal Year Block
            add_cover_text_frame(
                app,
                0,
                [36.0 * sx, 640.0 * sy, 260.0 * sx, 715.0 * sy],
                year_text,
                "#ffffff",
                "Arial Black",
                "Regular",
                60.0 * s_avg,
                None,
                Some(15.0),
                "left",
            )?;

            app.run(
                "path.create",
                json!({"spread": 0, "closed": false, "anchors": [
                    {"p": [36.0 * sx, 722.0 * sy]},
                    {"p": [200.0 * sx, 722.0 * sy]}
                ]}),
            )?;
            let _ = app.run("object.stroke", json!({"weight": 4.0 * s_avg}));
            let _ = app.run("object.color", json!({"color": accent, "target": "stroke"}));

            add_cover_text_frame(
                app,
                0,
                [36.0 * sx, 730.0 * sy, 260.0 * sx, 755.0 * sy],
                volume_text,
                "#cbd5e1",
                "Arial",
                "Bold",
                11.0 * s_avg,
                None,
                Some(20.0),
                "left",
            )?;

            // 11. Bottom Subtle Rule
            app.run(
                "path.create",
                json!({"spread": 0, "closed": false, "anchors": [
                    {"p": [36.0 * sx, 795.0 * sy]},
                    {"p": [pw - 36.0 * sx, 795.0 * sy]}
                ]}),
            )?;
            let _ = app.run("object.stroke", json!({"weight": 1.0 * s_avg}));
            let _ = app.run("object.color", json!({"color": "#334155", "target": "stroke"}));

            add_cover_text_frame(
                app,
                0,
                [36.0 * sx, 804.0 * sy, 300.0 * sx, 824.0 * sy],
                "PUBLIKASI BPS KABUPATEN KARAWANG",
                "#94a3b8",
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "left",
            )?;
            add_cover_text_frame(
                app,
                0,
                [pw - 200.0 * sx, 804.0 * sy, pw - 36.0 * sx, 824.0 * sy],
                "bps.go.id",
                accent,
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "right",
            )?;

            "Honeycomb Hexagon"
        }
        5 => {
            // Style 5: Corporate Headline (from design-3.designcraft)
            let sx = pw / 595.275591;
            let sy = ph / 841.889764;
            let s_avg = (sx + sy) / 2.0;

            // 1. Base white background
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 2. Top accent bar
            app.run("frame.create", json!({"spread": 0, "rect": [0.0, 0.0, 548.0 * sx, 16.0 * sy], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 3. Top-Right Metadata (Position Lock: Line #1 Catalogue, Line #2 ISSN/ISBN)
            add_cover_text_frame(
                app,
                0,
                [pw - 260.0 * sx, 24.0 * sy, pw - 36.0 * sx, 42.0 * sy],
                &cat_text,
                dark,
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "right",
            )?;
            add_cover_text_frame(
                app,
                0,
                [pw - 260.0 * sx, 42.0 * sy, pw - 36.0 * sx, 60.0 * sy],
                &code_text,
                dark,
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "right",
            )?;

            // 4. Bottom-right purple block
            app.run("frame.create", json!({"spread": 0, "rect": [438.0 * sx, 780.0 * sy, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 5. Mid-right accent tab
            app.run(
                "frame.create",
                json!({"spread": 0, "rect": [565.0 * sx, 680.0 * sy, pw, 720.0 * sy], "content": "unassigned", "shape": "rectangle"}),
            )?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 6. Center-right vertical photo frame
            let photo_res = app.run(
                "frame.create",
                json!({
                    "spread": 0,
                    "rect": [221.0 * sx, 168.0 * sy, 565.0 * sx, 780.0 * sy],
                    "content": "graphic",
                    "shape": "rectangle"
                }),
            )?;
            let photo_id = photo_res.get("id").and_then(Value::as_u64);
            let _ = app.run("file.place", json!({"path": skyline, "frame": photo_id}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally", "ids": [photo_id]}));
            let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [photo_id]}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]", "ids": [photo_id]}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 8, "size": 18, "opacity": 0.22, "ids": [photo_id]}));

            // 7. Top-left L-bracket framing the photo
            app.run(
                "frame.create",
                json!({"spread": 0, "rect": [213.0 * sx, 161.0 * sy, 325.0 * sx, 170.0 * sy], "content": "unassigned", "shape": "rectangle"}),
            )?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            app.run(
                "frame.create",
                json!({"spread": 0, "rect": [213.0 * sx, 161.0 * sy, 221.0 * sx, 265.0 * sy], "content": "unassigned", "shape": "rectangle"}),
            )?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 8. Top-right purple accent card (without placeholder "Corporate Design" text)
            app.run(
                "frame.create",
                json!({"spread": 0, "rect": [342.0 * sx, 108.0 * sy, pw, 234.0 * sy], "content": "unassigned", "shape": "rectangle"}),
            )?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.22}));

            // 9. Middle-left Big Year & Volume (replacing Business Report)
            let (y1, y2) = if year_text.len() >= 4 { (&year_text[..2], &year_text[2..]) } else { (year_text, "") };
            add_cover_text_frame(
                app,
                0,
                [36.0 * sx, 260.0 * sy, 118.0 * sx, 335.0 * sy],
                y1,
                dark,
                "Arial Black",
                "Regular",
                62.0 * s_avg,
                None,
                Some(10.0),
                "left",
            )?;
            if !y2.is_empty() {
                add_cover_text_frame(
                    app,
                    0,
                    [116.0 * sx, 260.0 * sy, 215.0 * sx, 335.0 * sy],
                    y2,
                    primary,
                    "Arial Black",
                    "Regular",
                    62.0 * s_avg,
                    None,
                    Some(10.0),
                    "left",
                )?;
            }

            // Volume text directly under Year numerals
            add_cover_text_frame(
                app,
                0,
                [36.0 * sx, 340.0 * sy, 220.0 * sx, 360.0 * sy],
                volume_text,
                dark,
                "Arial",
                "Bold",
                11.0 * s_avg,
                None,
                Some(25.0),
                "left",
            )?;

            // Accent underline rule
            app.run(
                "path.create",
                json!({"spread": 0, "closed": false, "anchors": [
                    {"p": [36.0 * sx, 364.0 * sy]},
                    {"p": [205.0 * sx, 364.0 * sy]}
                ]}),
            )?;
            let _ = app.run("object.stroke", json!({"weight": 2.5 * s_avg}));
            let _ = app.run("object.color", json!({"color": primary, "target": "stroke"}));

            // 10. Headline Purple Block (Bottom-Left)
            let r_corner = 46.0 * s_avg;
            app.run(
                "path.create",
                json!({
                    "spread": 0,
                    "closed": true,
                    "anchors": [
                        {"p": [0.0, 458.0 * sy]},
                        {"p": [340.0 * sx - r_corner, 458.0 * sy], "out": [340.0 * sx - r_corner + r_corner * 0.552, 458.0 * sy]},
                        {"p": [340.0 * sx, 458.0 * sy + r_corner], "in": [340.0 * sx, 458.0 * sy + r_corner - r_corner * 0.552]},
                        {"p": [340.0 * sx, 680.0 * sy]},
                        {"p": [0.0, 680.0 * sy]}
                    ]
                }),
            )?;
            let _ = app.run("object.color", json!({"color": primary, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));
            let _ = app.run("object.opacity", json!({"opacity": 0.85}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 8, "size": 16, "opacity": 0.28}));

            let formatted_title = if title_text.contains(' ') { title_text.replace(' ', "\n") } else { title_text.to_string() };
            add_cover_text_frame(
                app,
                0,
                [26.0 * sx, 508.0 * sy, 330.0 * sx, 616.0 * sy],
                &formatted_title,
                "#ffffff",
                "Arial Black",
                "Regular",
                34.0 * s_avg,
                Some(39.0 * s_avg),
                Some(10.0),
                "left",
            )?;
            add_cover_text_frame(
                app,
                0,
                [28.0 * sx, 624.0 * sy, 320.0 * sx, 654.0 * sy],
                title2_text,
                "#ffffff",
                "Arial",
                "Bold",
                15.0 * s_avg,
                None,
                Some(50.0),
                "left",
            )?;

            // 11. Bottom-Left Logo & Agency Text (Position Lock)
            app.run(
                "frame.create",
                json!({
                    "spread": 0,
                    "rect": [36.0 * sx, 738.0 * sy, 78.0 * sx, 784.0 * sy],
                    "content": "graphic",
                    "shape": "rectangle"
                }),
            )?;
            let _ = app.run("file.place", json!({"path": logo}));
            let _ = app.run("object.fit", json!({"mode": "fitProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            add_cover_text_frame(
                app,
                0,
                [86.0 * sx, 742.0 * sy, 360.0 * sx, 786.0 * sy],
                agency_text,
                dark,
                "Arial",
                "Bold Italic",
                10.0 * s_avg,
                Some(13.5 * s_avg),
                None,
                "left",
            )?;

            "Corporate Headline"
        }
        6 => {
            // Style 6: Diagonal Arrow (from design-4.designcraft)
            let sx = pw / 595.275591;
            let sy = ph / 841.889764;
            let s_avg = (sx + sy) / 2.0;

            // 1. Base white background
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 2. Full-height cityscape panel on the right (half-bleed)
            let photo_res = app.run(
                "frame.create",
                json!({
                    "spread": 0,
                    "rect": [286.0 * sx, 0.0, pw, ph],
                    "content": "graphic",
                    "shape": "rectangle"
                }),
            )?;
            let photo_id = photo_res.get("id").and_then(Value::as_u64);
            let _ = app.run("file.place", json!({"path": skyline, "frame": photo_id}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally", "ids": [photo_id]}));
            let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [photo_id]}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]", "ids": [photo_id]}));

            // 3. Dark diagonal banner across the top, leaning over the photo
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [113.20 * sx, 0.0]},
                    {"p": [399.74 * sx, 0.0]},
                    {"p": [286.54 * sx, 256.0 * sy]},
                    {"p": [0.0, 256.0 * sy]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": dark, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 4. Accent corner wedge in the top-left, off the banner's leading edge
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 0.0]},
                    {"p": [86.87 * sx, 0.0]},
                    {"p": [0.0, 196.46 * sy]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 5. Top-right metadata over the photograph (Position Lock: Catalogue, then ISSN/ISBN)
            add_cover_text_frame(
                app,
                0,
                [330.0 * sx, 17.0 * sy, 560.0 * sx, 35.0 * sy],
                &cat_text,
                "#e2e8f0",
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "right",
            )?;
            add_cover_text_frame(
                app,
                0,
                [330.0 * sx, 35.0 * sy, 560.0 * sx, 53.0 * sy],
                &code_text,
                "#e2e8f0",
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "right",
            )?;

            // 6. Forward wedge crossing from the white field into the photo
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 519.0 * sy]},
                    {"p": [352.0 * sx, 591.0 * sy]},
                    {"p": [0.0, 663.0 * sy]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": dark, "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.18}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 7. Year and Volume, centred inside the wedge
            add_cover_text_frame(
                app,
                0,
                [6.15 * sx, 569.0 * sy, 196.15 * sx, 639.0 * sy],
                year_text,
                "#ffffff",
                "Arial Black",
                "Regular",
                44.0 * s_avg,
                None,
                Some(20.0),
                "center",
            )?;
            add_cover_text_frame(
                app,
                0,
                [0.0, 610.0 * sy, 172.30 * sx, 632.0 * sy],
                volume_text,
                "#e2e8f0",
                "Arial",
                "Bold",
                11.0 * s_avg,
                None,
                Some(40.0),
                "center",
            )?;

            // 8. Middle-left headline: the title (accent), then the standard publication suffix
            // One full-width line: the frame is as wide as the page, so a title never hyphenates here.
            add_cover_text_frame(
                app,
                0,
                [36.0 * sx, 394.0 * sy, 559.28 * sx, 428.0 * sy],
                title_text,
                accent,
                "Arial Black",
                "Regular",
                30.0 * s_avg,
                Some(34.0 * s_avg),
                Some(10.0),
                "left",
            )?;
            add_cover_text_frame(
                app,
                0,
                [36.0 * sx, 431.0 * sy, 384.0 * sx, 465.0 * sy],
                title2_text,
                dark,
                "Arial Black",
                "Regular",
                28.0 * s_avg,
                None,
                Some(20.0),
                "left",
            )?;

            // 9. Bottom-Left Logo & Agency Text (Position Lock)
            app.run(
                "frame.create",
                json!({
                    "spread": 0,
                    "rect": [36.0 * sx, 742.0 * sy, 76.0 * sx, 786.0 * sy],
                    "content": "graphic",
                    "shape": "rectangle"
                }),
            )?;
            let _ = app.run("file.place", json!({"path": logo}));
            let _ = app.run("object.fit", json!({"mode": "fitProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            add_cover_text_frame(
                app,
                0,
                [84.0 * sx, 744.0 * sy, 340.0 * sx, 786.0 * sy],
                agency_text,
                dark,
                "Arial",
                "Bold Italic",
                10.0 * s_avg,
                Some(13.5 * s_avg),
                None,
                "left",
            )?;

            "Diagonal Arrow"
        }
        7 => {
            // Style 7: Modern Horizon (from design-5.designcraft)
            // A full-bleed photograph under a sweeping arc, an accent wedge that follows the arc and
            // turns into the accent block on the right, and a dark block across the lower page whose
            // rounded top-left corner carries the headline.
            let sx = pw / 595.275591;
            let sy = ph / 841.889764;
            let s_avg = (sx + sy) / 2.0;
            let x = |v: f64| v * sx;
            let y = |v: f64| v * sy;

            // 1. Base white background
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 2. Hero photograph, clipped by the arc that sweeps across the upper page (the traced
            //    geometry of design-5.designcraft, which runs from the left edge into the right edge).
            let photo_res = app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, 0.0]},
                    {"p": [x(595.776), 0.0]},
                    {"p": [x(595.776), y(469.637)], "out": [x(284.631), y(445.387)]},
                    {"p": [0.0, y(172.068)], "in": [x(40.517), y(324.164)]}
                ]}),
            )?;
            let photo_id = photo_res.get("id").and_then(Value::as_u64);
            let _ = app.run("file.place", json!({"path": skyline, "frame": photo_id}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally", "ids": [photo_id]}));
            let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [photo_id]}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]", "ids": [photo_id]}));

            // 3. Dark block across the lower page, with the traced rounded top-left corner.
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, y(528.159)], "in": [0.0, y(567.917)], "out": [x(29.018), y(465.043)]},
                    {"p": [x(194.405), y(420.0)], "in": [x(105.107), y(419.999)], "out": [x(215.156), y(419.999)]},
                    {"p": [x(254.093), y(426.959)], "in": [x(235.193), y(422.433)], "out": [x(300.0), y(440.0)]},
                    {"p": [x(372.305), y(462.0)], "in": [x(330.0), y(450.0)]},
                    {"p": [x(595.276), y(462.0)]},
                    {"p": [x(595.276), y(841.890)]},
                    {"p": [0.0, y(841.890)]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": dark, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 4. Accent wedge following the arc, opening into the accent block on the right. One path:
            //    its lower boundary tracks the dark block's top edge, leaving a hairline of white.
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, y(175.068)], "in": [0.0, y(300.0)], "out": [x(40.517), y(327.164)]},
                    {"p": [x(595.776), y(472.637)], "in": [x(284.631), y(448.387)]},
                    {"p": [x(595.776), y(720.0)]},
                    {"p": [x(372.305), y(720.0)]},
                    {"p": [x(372.305), y(458.0)], "out": [x(330.0), y(446.0)]},
                    {"p": [x(254.093), y(422.959)], "in": [x(300.0), y(436.0)], "out": [x(235.193), y(418.433)]},
                    {"p": [x(194.405), y(415.999)], "in": [x(215.156), y(415.999)], "out": [x(105.107), y(415.999)]},
                    {"p": [0.0, y(524.159)], "in": [x(29.018), y(461.043)], "out": [0.0, y(300.0)]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 5. Top-right metadata over the photograph (Position Lock: catalogue, then ISSN/ISBN)
            add_cover_text_frame(app, 0, [x(330.0), y(17.0), x(560.0), y(35.0)], &cat_text, dark, "Arial", "Bold", 8.5 * s_avg, None, None, "right")?;
            add_cover_text_frame(
                app,
                0,
                [x(330.0), y(35.0), x(560.0), y(53.0)],
                &code_text,
                dark,
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "right",
            )?;

            // 6. Headline inside the dark block: the title (accent), then the standard suffix.
            //    The frames stop short of the accent block, whose left edge cuts into the dark block,
            //    so an accent-coloured title never runs out over accent-coloured paper. One word per
            //    line in the title, as on the reference cover, so a long title never hyphenates.
            let formatted_title = if title_text.contains(' ') { title_text.replace(' ', "\n") } else { title_text.to_string() };
            add_cover_text_frame(
                app,
                0,
                [x(36.0), y(578.0), x(368.0), y(686.0)],
                &formatted_title,
                accent,
                "Arial Black",
                "Regular",
                34.0 * s_avg,
                Some(39.0 * s_avg),
                Some(10.0),
                "left",
            )?;
            add_cover_text_frame(
                app,
                0,
                [x(36.0), y(690.0), x(380.0), y(730.0)],
                title2_text,
                "#ffffff",
                "Arial Black",
                "Regular",
                26.0 * s_avg,
                None,
                Some(20.0),
                "left",
            )?;

            // 7. Year with its volume directly underneath, centred in the accent block
            add_cover_text_frame(
                app,
                0,
                [x(390.0), y(540.0), x(580.0), y(620.0)],
                year_text,
                "#ffffff",
                "Arial Black",
                "Regular",
                56.0 * s_avg,
                None,
                Some(20.0),
                "center",
            )?;
            add_cover_text_frame(
                app,
                0,
                [x(390.0), y(624.0), x(580.0), y(648.0)],
                volume_text,
                "#ffffff",
                "Arial",
                "Bold",
                11.0 * s_avg,
                None,
                Some(40.0),
                "center",
            )?;

            // 8. Bottom-Left Logo & Agency Text (Position Lock)
            app.run(
                "frame.create",
                json!({
                    "spread": 0,
                    "rect": [x(36.0), y(752.0), x(84.0), y(804.0)],
                    "content": "graphic",
                    "shape": "rectangle"
                }),
            )?;
            let _ = app.run("file.place", json!({"path": logo}));
            let _ = app.run("object.fit", json!({"mode": "fitProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            add_cover_text_frame(
                app,
                0,
                [x(92.0), y(754.0), x(380.0), y(806.0)],
                agency_text,
                "#ffffff",
                "Arial",
                "Bold Italic",
                10.0 * s_avg,
                Some(13.5 * s_avg),
                None,
                "left",
            )?;

            "Modern Horizon"
        }
        8 => {
            // Style 8: Business Report — traced from design-10.designcraft (the navy/orange
            // business-report cover). White base; an orange corner tucked behind the navy plane in
            // the bottom-right; an orange band along the top; the metadata, the headline stack and
            // the agency block on the navy plane; then the arrow-shaped skyline photograph with
            // its chevron sliver across a thin white gap, and the white circle carrying the
            // two-tone year on top. Every text frame maps to a Publication Cover option.
            let sx = pw / 595.275591;
            let sy = ph / 841.889764;
            let s_avg = (sx + sy) / 2.0;
            let x = |v: f64| v * sx;
            let y = |v: f64| v * sy;

            // 1. Base white background
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 2. Orange corner tucked into the bottom-right, behind the navy plane (traced item 15).
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [x(423.8), y(738.4)], "in": [x(479.5), y(770.8)], "out": [x(448.2), y(724.1)]},
                    {"p": [pw, y(637.6)], "in": [x(563.4), y(656.3)], "out": [pw, y(702.0)]},
                    {"p": [pw, y(838.1)], "in": [pw, y(772.9)], "out": [x(537.9), y(804.7)]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 3. Navy plane crossing in front of the photograph from the left (traced item 16).
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [0.0, y(295.6)], "in": [0.0, y(455.1)], "out": [x(71.4), y(336.0)]},
                    {"p": [x(216.5), y(418.3)], "in": [x(144.6), y(377.5)], "out": [x(216.5), y(519.3)]},
                    {"p": [x(216.5), y(621.2)]},
                    {"p": [pw, y(835.8)], "in": [x(409.3), y(730.4)], "out": [pw, y(839.8)]},
                    {"p": [pw, ph]},
                    {"p": [0.0, ph]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": dark, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 4. Orange band along the top edge (traced item 17), behind the metadata corner.
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [x(247.4), 0.0], "in": [x(304.8), y(33.9)], "out": [x(344.1), 0.0]},
                    {"p": [pw, 0.0]},
                    {"p": [pw, y(81.1)], "in": [pw, y(55.6)], "out": [x(539.9), y(114.1)]},
                    {"p": [x(490.3), y(143.4)]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 5. Top-right publication metadata over the orange band (Position Lock: catalogue,
            //    then ISSN/ISBN).
            add_cover_text_frame(
                app,
                0,
                [x(320.0), y(14.0), x(580.0), y(32.0)],
                &cat_text,
                "#ffffff",
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "right",
            )?;
            add_cover_text_frame(
                app,
                0,
                [x(320.0), y(32.0), x(580.0), y(50.0)],
                &code_text,
                "#ffffff",
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "right",
            )?;

            // 6. Headline stack top-left: the title (orange, one line), then DALAM ANGKA (navy) —
            //    the navy / orange / navy rhythm of the reference.
            add_cover_text_frame(
                app,
                0,
                [x(36.0), y(128.0), x(370.0), y(174.0)],
                title_text,
                accent,
                "Arial Black",
                "Regular",
                26.0 * s_avg,
                Some(30.0 * s_avg),
                Some(10.0),
                "left",
            )?;
            add_cover_text_frame(
                app,
                0,
                [x(36.0), y(176.0), x(360.0), y(206.0)],
                title2_text,
                dark,
                "Arial Black",
                "Regular",
                20.0 * s_avg,
                None,
                Some(26.0),
                "left",
            )?;

            // 7. Agency logo & name on the navy plane, bottom-left (Position Lock).
            app.run(
                "frame.create",
                json!({"spread": 0, "rect": [x(46.0), y(772.0), x(86.0), y(812.0)], "content": "graphic", "shape": "rectangle"}),
            )?;
            let _ = app.run("file.place", json!({"path": logo}));
            let _ = app.run("object.fit", json!({"mode": "fitProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            add_cover_text_frame(
                app,
                0,
                [x(94.0), y(774.0), x(310.0), y(816.0)],
                agency_text,
                "#ffffff",
                "Arial",
                "Bold Italic",
                10.0 * s_avg,
                Some(13.5 * s_avg),
                None,
                "left",
            )?;

            // 8. The photograph fills both traced subpaths of design-10's photo frame: the arrow
            //    itself, and — across a thin white gap — the chevron sliver below it. One placed
            //    image goes into both frames at the same world position, so the buildings continue
            //    across the gap.
            let arrow_id = app
                .run(
                    "path.create",
                    json!({"spread": 0, "closed": true, "anchors": [
                        {"p": [x(202.3), y(316.7)], "in": [x(202.3), y(345.5)], "out": [x(324.4), y(246.6)]},
                        {"p": [x(594.8), y(91.2)], "in": [x(469.1), y(163.4)], "out": [x(594.8), y(189.2)]},
                        {"p": [x(594.8), y(627.1)], "in": [x(594.8), y(491.9)], "out": [x(483.5), y(564.0)]},
                        {"p": [x(202.3), y(404.3)], "in": [x(317.5), y(469.7)], "out": [x(202.3), y(375.1)]}
                    ]}),
                )?
                .get("id")
                .and_then(Value::as_u64);
            let chev_id = app
                .run(
                    "path.create",
                    json!({"spread": 0, "closed": true, "anchors": [
                        {"p": [x(220.8), y(423.2)], "in": [x(220.8), y(487.8)], "out": [x(343.5), y(492.7)]},
                        {"p": [x(590.0), y(632.4)], "in": [x(477.8), y(568.9)], "out": [x(535.4), y(664.1)]},
                        {"p": [x(423.3), y(729.1)], "in": [x(479.3), y(696.6)], "out": [x(355.0), y(690.4)]},
                        {"p": [x(220.8), y(614.3)], "in": [x(286.2), y(651.3)], "out": [x(220.8), y(553.3)]}
                    ]}),
                )?
                .get("id")
                .and_then(Value::as_u64);
            if let (Some(arrow_id), Some(chev_id)) = (arrow_id, chev_id) {
                if let Some(img_id) = app.run("file.place", json!({"path": skyline, "x": x(130.6), "y": y(91.2)}))?.get("id").and_then(Value::as_u64)
                {
                    // Stretch the placed image over the cover box both shapes sit on, then paste
                    // copies into each frame — the world position carries over, so the two read
                    // as one photograph.
                    if let Some(from) = item_bounds(app, img_id) {
                        let _ = app.run(
                            "transform.resize",
                            json!({"from": from, "to": [x(130.6), y(91.2), x(809.7), y(742.4)], "content": true, "ids": [img_id]}),
                        );
                    }
                    let _ = app.run("edit.copy", json!({}));
                    let _ = app.run("edit.pasteInto", json!({"id": arrow_id}));
                    let _ = app.run("edit.pasteInto", json!({"id": chev_id}));
                    let _ = app.run("edit.clear", json!({"ids": [img_id]}));
                }
                let _ = app.run("object.dropShadow", json!({"on": true, "distance": 7.0, "size": 5.0, "opacity": 0.3, "ids": [arrow_id, chev_id]}));
                let _ = app.run("object.stroke", json!({"swatch": "[None]", "ids": [arrow_id, chev_id]}));
            }

            // 9. White circle on the navy plane carrying the two-tone year — the medallion of the
            //    reference.
            app.run(
                "frame.create",
                json!({"spread": 0, "rect": [x(55.0), y(436.0), x(245.0), y(626.0)], "content": "unassigned", "shape": "ellipse"}),
            )?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.dropShadow", json!({"on": true, "distance": 6.0, "size": 12.0, "opacity": 0.25}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            let year_chars: Vec<char> = year_text.chars().collect();
            let (year_first, year_second) = if year_chars.len() >= 2 {
                let mid = year_chars.len().div_ceil(2);
                (year_chars[..mid].iter().collect::<String>(), year_chars[mid..].iter().collect::<String>())
            } else {
                (year_text.to_string(), String::new())
            };
            add_cover_text_frame(
                app,
                0,
                [x(78.0), y(512.0), x(160.0), y(558.0)],
                &year_first,
                dark,
                "Arial Black",
                "Regular",
                54.0 * s_avg,
                None,
                Some(10.0),
                "left",
            )?;
            add_cover_text_frame(
                app,
                0,
                [x(140.0), y(512.0), x(222.0), y(558.0)],
                &year_second,
                accent,
                "Arial Black",
                "Regular",
                54.0 * s_avg,
                None,
                Some(10.0),
                "left",
            )?;
            add_cover_text_frame(
                app,
                0,
                [x(78.0), y(578.0), x(242.0), y(604.0)],
                volume_text,
                dark,
                "Arial",
                "Bold",
                10.0 * s_avg,
                None,
                Some(30.0),
                "left",
            )?;

            "Business Report"
        }
        9 => {
            // Style 9: Urban Angle (from design-6.designcraft). Percentage-based geometry so the
            // traced shapes scale with the page: orange top-left wedge, navy diagonal band behind a
            // right-hand cityscape panel, white negative-X, navy bottom wedge.
            let sx = pw / 595.275591;
            let sy = ph / 841.889764;
            let s_avg = (sx + sy) / 2.0;
            let x = |v: f64| v * sx;
            let y = |v: f64| v * sy;

            // 1. Base white background.
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
            let _ = app.run("object.color", json!({"color": "#ffffff", "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 2. Navy bottom wedge.
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [x(521.13), y(564.5)]},
                    {"p": [x(595.28), y(631.26)], "in": [x(552.24), y(592.51)], "out": [x(595.28), y(701.88)]},
                    {"p": [x(595.28), y(841.89)], "in": [x(595.28), y(780.19)], "out": [x(482.55), y(841.89)]},
                    {"p": [x(271.37), y(841.89)], "in": [x(364.38), y(841.89)], "out": [x(333.69), y(772.67)]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": dark, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 3. Navy diagonal band behind the cityscape.
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [x(71.61), y(75.34)]},
                    {"p": [x(162.59), y(-1.0)], "in": [x(108.52), y(44.36)], "out": [x(292.29), y(-1.0)]},
                    {"p": [x(595.28), y(-1.0)], "in": [x(444.42), y(-1.0)], "out": [x(595.28), y(152.97)]},
                    {"p": [x(595.28), y(699.43)], "in": [x(595.28), y(525.07)], "out": [x(503.88), y(590.51)]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": dark, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 4. Cityscape photograph across the right-hand panels (two traced subpaths).
            let photo_res = app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [x(231.97), y(363.57)]},
                    {"p": [x(537.04), y(0.0)], "in": [x(382.64), y(184.0)], "out": [x(556.2), y(0.0)]},
                    {"p": [x(594.78), y(0.0)], "in": [x(575.46), y(0.0)], "out": [x(594.78), y(57.76)]},
                    {"p": [x(594.78), y(340.84)], "in": [x(594.78), y(192.02)], "out": [x(487.89), y(468.21)]},
                    {"p": [x(403.84), y(568.39)]}
                ]}),
            )?;
            let photo_id = photo_res.get("id").and_then(Value::as_u64);
            let _ = app.run("object.content", json!({"type": "graphic", "ids": [photo_id]}));
            let _ = app.run("file.place", json!({"path": skyline, "frame": photo_id}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally", "ids": [photo_id]}));
            let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [photo_id]}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]", "ids": [photo_id]}));
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [x(344.83), y(652.62)]},
                    {"p": [x(594.78), y(373.1)], "in": [x(456.73), y(527.48)], "out": [x(594.78), y(544.64)]},
                    {"p": [x(594.78), y(835.54)], "in": [x(594.78), y(729.3)], "out": [x(567.52), y(866.02)]},
                    {"p": [x(551.59), y(883.83)]}
                ]}),
            )?;
            let _ = app.run("object.content", json!({"type": "graphic"}));
            let _ = app.run("file.place", json!({"path": skyline}));
            let _ = app.run("object.fit", json!({"mode": "fillProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 5. Orange top-left wedge, drawn last so it cuts cleanly over the band.
            app.run(
                "path.create",
                json!({"spread": 0, "closed": true, "anchors": [
                    {"p": [x(299.84), y(-0.5)], "in": [x(203.05), y(-0.5)], "out": [x(208.94), y(107.83)]},
                    {"p": [x(0.0), y(356.84)], "in": [x(64.17), y(280.36)], "out": [x(0.0), y(243.84)]},
                    {"p": [x(0.0), y(-0.5)], "in": [x(0.0), y(96.13)], "out": [x(99.02), y(-0.5)]}
                ]}),
            )?;
            let _ = app.run("object.color", json!({"color": accent, "target": "fill"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            // 6. Rotated year in the navy band, split into two tones like the reference.
            let (y1, y2) = if year_text.len() >= 4 { (&year_text[..2], &year_text[2..]) } else { (year_text, "") };
            let year1 = add_cover_text_frame(
                app,
                0,
                [x(150.0), y(73.0), x(232.0), y(153.0)],
                y1,
                "#ffffff",
                "Arial Black",
                "Regular",
                44.0 * s_avg,
                None,
                Some(10.0),
                "center",
            )?;
            let year2 = add_cover_text_frame(
                app,
                0,
                [x(205.0), y(61.0), x(290.0), y(141.0)],
                y2,
                accent,
                "Arial Black",
                "Regular",
                44.0 * s_avg,
                None,
                Some(10.0),
                "center",
            )?;
            let _ = app.run("transform.rotate", json!({"ids": [year1, year2], "angle": -50.0}));

            // 7. Volume caption along the band, under the year.
            let vol_id = add_cover_text_frame(
                app,
                0,
                [x(252.0), y(151.0), x(356.0), y(185.0)],
                volume_text,
                "#ffffff",
                "Arial",
                "Bold",
                8.0 * s_avg,
                None,
                Some(20.0),
                "left",
            )?;
            let _ = app.run("transform.rotate", json!({"ids": [vol_id], "angle": -50.0}));

            // 8. Headline stack on the white left panel (title in accent, then DALAM ANGKA).
            add_cover_text_frame(
                app,
                0,
                [x(36.0), y(650.0), x(340.0), y(724.0)],
                &title_text.replace(' ', "\n"),
                accent,
                "Arial Black",
                "Regular",
                30.0 * s_avg,
                Some(34.0 * s_avg),
                Some(8.0),
                "left",
            )?;
            add_cover_text_frame(
                app,
                0,
                [x(36.0), y(728.0), x(330.0), y(776.0)],
                title2_text,
                dark,
                "Arial Black",
                "Regular",
                22.0 * s_avg,
                None,
                Some(15.0),
                "left",
            )?;

            // 9. Top-left agency logo & text over the orange wedge (Position Lock).
            app.run("frame.create", json!({"spread": 0, "rect": [x(36.0), y(24.0), x(74.0), y(64.0)], "content": "graphic", "shape": "rectangle"}))?;
            let _ = app.run("file.place", json!({"path": logo}));
            let _ = app.run("object.fit", json!({"mode": "fitProportionally"}));
            let _ = app.run("object.fit", json!({"mode": "centerContent"}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]"}));

            add_cover_text_frame(
                app,
                0,
                [x(82.0), y(26.0), x(300.0), y(68.0)],
                agency_text,
                dark,
                "Arial",
                "Bold Italic",
                10.0 * s_avg,
                Some(13.5 * s_avg),
                None,
                "left",
            )?;

            // 10. Top-right publication metadata over the photograph (Position Lock).
            add_cover_text_frame(
                app,
                0,
                [x(330.0), y(17.0), x(560.0), y(35.0)],
                &cat_text,
                "#ffffff",
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "right",
            )?;
            add_cover_text_frame(
                app,
                0,
                [x(330.0), y(35.0), x(560.0), y(53.0)],
                &code_text,
                "#ffffff",
                "Arial",
                "Bold",
                8.5 * s_avg,
                None,
                None,
                "right",
            )?;

            "Urban Angle"
        }
        _ => return Err(format!("Unsupported cover style: {}", st.cover_style)),
    };

    Ok(format!("Cover page generated before Page 1 (Style: {style_name})"))
}

fn card3_quick_actions_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Quick Actions"), true);

    let facing = app.session.active().map(|s| s.doc.settings.facing_pages).unwrap_or(true);
    let cur_page =
        app.session.active().and_then(|s| s.selection.page).or_else(|| crate::canvas::current_page(app)).unwrap_or_else(|| target_page_index(app));
    let cur_label = app.session.page_label(cur_page);

    // Button 1: switch Facing Pages
    // When currently Facing (Double-sided), button offers switching to Single-sided.
    // When currently Single-sided, button offers switching to Double-sided.
    let facing_text = if facing { crate::i18n::tr(&app.ui.language, "Single-sided") } else { crate::i18n::tr(&app.ui.language, "Double-sided") };
    let add_page_text = crate::i18n::tr(&app.ui.language, "Add Blank Page");

    let fw = full_width(ui);
    let hw = (fw - 10.0) / 2.0;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;

        let tip_facing = if facing {
            "Current: Double-sided (facing pages). Click to switch to Single-sided layout"
        } else {
            "Current: Single-sided layout. Click to switch to Double-sided (facing pages)"
        };
        let resp_facing = outline_button(ui, facing_text, hw).on_hover_text(tip_facing);
        if resp_facing.clicked() {
            let next_facing = !facing;
            let _ = app.run("layout.documentSetup", json!({"facingPages": next_facing}));
            st.card3_status =
                Some((if next_facing { "Switched to Double-sided (Facing Pages)".into() } else { "Switched to Single-sided layout".into() }, true));
        }

        let tip_page = format!("Insert a blank page after Page {cur_label}");
        let resp_page = outline_button(ui, add_page_text, hw).on_hover_text(tip_page);
        if resp_page.clicked() {
            let _ = app.run("layout.pages.insert", json!({"after": cur_page, "count": 1}));
            crate::canvas::go_to_page(app, cur_page + 1);
            let next_label = app.session.page_label(cur_page + 1);
            st.card3_status = Some((format!("Added blank page after Page {cur_label} (new Page {next_label})"), true));
        }
    });

    let hf_text = crate::i18n::tr(&app.ui.language, "Header/Footer…");
    let tbl_text = crate::i18n::tr(&app.ui.language, "Table Options…");

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;

        let tip_hf = "Configure page running headers, footers, page numbering, and section titles";
        let resp_hf = outline_button(ui, hf_text, hw).on_hover_text(tip_hf);
        if resp_hf.clicked() {
            let _ = crate::header_footer::open(app);
        }

        let tip_tbl = "Configure table header/footer rows and repeating continuation rows";
        let resp_tbl = outline_button(ui, tbl_text, hw).on_hover_text(tip_tbl);
        if resp_tbl.clicked() {
            let _ = crate::dialogs::open_table_options(app);
        }
    });

    // TOC: build a table of contents on the selected page from the TOC paragraph style.
    ui.horizontal(|ui| {
        ui.label(RichText::new("TOC title:").size(11.0).color(t.text));
        let w = (ui.available_width() - 4.0).clamp(80.0, 180.0);
        ui.add_sized([w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.toc_title));
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("TOC style:").size(11.0).color(t.text));
        let cur = TOC_KINDS.get(st.toc_kind).copied().unwrap_or(TOC_KINDS[0]);
        let tip = "TOC Entry: every page whose title carries the TOC tag style, plus the narrative pages\nTOC Table: only pages whose first text box starts with \"Tabel\" or \"Table\"\nTOC Chart: only pages whose first text box starts with \"Grafik\" or \"Chart\"\nTOC Paragraph: the narrative pages (the GXX and RXX sheets) with their Heading 1 and Heading 2 paragraphs nested under them";
        let before = st.toc_kind;
        egui::ComboBox::from_id_salt("pubgen_toc_kind")
            .selected_text(cur)
            .width(110.0)
            .show_ui(ui, |ui| {
                for (idx, k) in TOC_KINDS.iter().enumerate() {
                    ui.selectable_value(&mut st.toc_kind, idx, *k);
                }
            })
            .response
            .on_hover_text(tip);
        // Picking a table/chart/paragraph list fills in its title per single or dual lang mode.
        if st.toc_kind != before
            && let Some(title) = toc_kind_title(st.toc_kind, st.bilingual_output)
        {
            st.toc_title = title.to_string();
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("TOC layout:").size(11.0).color(t.text));
        let layouts = ["Standard", "TablePage"];
        let cur_layout = layouts.get(st.toc_layout_style).copied().unwrap_or("Standard");
        let tip_layout = "Standard: 'Tabel 4.2.1 Title ......... 36'\nTablePage: BPS statistical catalog layout with 'Tabel / halaman' header, standalone number column, and hanging indent";
        egui::ComboBox::from_id_salt("pubgen_toc_layout_style")
            .selected_text(cur_layout)
            .width(110.0)
            .show_ui(ui, |ui| {
                for (idx, l) in layouts.iter().enumerate() {
                    ui.selectable_value(&mut st.toc_layout_style, idx, *l);
                }
            })
            .response
            .on_hover_text(tip_layout);
    });
    ui.checkbox(&mut st.toc_hanging_indent, "Indent wrapped TOC entries (hanging indent)");
    ui.checkbox(&mut st.toc_include_self, "Include TOC page in TOC entries");
    ui
        .checkbox(&mut st.toc_pair_bilingual, "Pair bilingual headings (ID / EN)")
        .on_hover_text("Merge the same style's left-column and right-column headings on a Dual Lang page into one row (\"Indonesian / English\"). Only active with the Dual Lang toggle on and needs the two-column Dual Lang layout; full-width headings never pair.");

    let hw_toc = (ui.available_width() - 4.0) * 0.5;
    ui.horizontal(|ui| {
        let add_toc_text = crate::i18n::tr(&app.ui.language, "Add TOC");
        let tip_toc = format!("Insert a {} on Page {cur_label}", TOC_KINDS.get(st.toc_kind).copied().unwrap_or(TOC_KINDS[0]));
        if outline_button(ui, add_toc_text, hw_toc).on_hover_text(tip_toc).clicked() {
            let res = execute_add_toc(app, st);
            st.card3_status = Some(match res {
                Ok(msg) => (msg, true),
                Err(e) => (e, false),
            });
        }

        let rebuild_toc_text = crate::i18n::tr(&app.ui.language, "Rebuild TOC");
        let tip_rebuild = "Rebuild the table of contents on this page (its own TOC style, title and page numbers)";
        if outline_button(ui, rebuild_toc_text, hw_toc).on_hover_text(tip_rebuild).clicked() {
            let res = execute_rebuild_toc(app, st);
            st.card3_status = Some(match res {
                Ok(msg) => (msg, true),
                Err(e) => (e, false),
            });
        }
    });

    divider(ui);

    // Renumber Page / Section
    ui.horizontal(|ui| {
        ui.label(RichText::new("Renumber:").size(11.0).color(t.text));
        ui.add(egui::DragValue::new(&mut st.card3_renumber_start).range(1..=9999).prefix("Start: "));
        let styles = ["1, 2, 3…", "i, ii, iii…", "I, II, III…"];
        let cur_style = styles.get(st.card3_renumber_style).copied().unwrap_or("1, 2, 3…");
        egui::ComboBox::from_id_salt("pubgen_renumber_style").selected_text(cur_style).width(80.0).show_ui(ui, |ui| {
            for (idx, s) in styles.iter().enumerate() {
                ui.selectable_value(&mut st.card3_renumber_style, idx, *s);
            }
        });
    });
    let hw_renum = (ui.available_width() - 4.0) * 0.5;
    ui.horizontal(|ui| {
        let tip_renum = format!("Restart page numbering at {} on Page {cur_label}", st.card3_renumber_start);
        if outline_button(ui, "Restart page #", hw_renum).on_hover_text(tip_renum).clicked() {
            let res = execute_renumber_page(app, st);
            st.card3_status = Some(match res {
                Ok(msg) => (msg, true),
                Err(e) => (e, false),
            });
        }
        let tip_cont = format!("Continue numbering on Page {cur_label} (remove section restart)");
        if outline_button(ui, "Continue #", hw_renum).on_hover_text(tip_cont).clicked() {
            let res = execute_continue_numbering(app, st);
            st.card3_status = Some(match res {
                Ok(msg) => (msg, true),
                Err(e) => (e, false),
            });
        }
    });

    // Helper: Page from RTF
    divider(ui);
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 65.0).max(80.0);
        ui.add_sized([w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.rtf_path).hint_text("Path to .rtf file…"));
        if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
            && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
        {
            st.rtf_path = path;
        }
    });
    let tip_rtf = format!("Import RTF document to Page {cur_label} (auto-flows to new pages if needed)");
    if outline_button(ui, "Page from RTF", full_width(ui)).on_hover_text(tip_rtf).clicked() {
        if st.rtf_path.trim().is_empty()
            && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
        {
            st.rtf_path = path;
        }
        let path = st.rtf_path.trim().to_string();
        let res = execute_page_from_rtf(app, st, &path);
        st.card3_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(e) => (e, false),
        });
    }

    // Helper: Page from Image
    divider(ui);
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 65.0).max(80.0);
        ui.add_sized([w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.image_path).hint_text("Path to image file…"));
        if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
            && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
        {
            st.image_path = path;
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("As:").size(11.0).color(t.text));
        ui.radio_value(&mut st.image_as_background, false, "Image object")
            .on_hover_text("Create an image object fitted inside the page margins of the selected page");
        ui.radio_value(&mut st.image_as_background, true, "Page background")
            .on_hover_text("Fit image proportionally to full page, keeping aspect ratio (sent to the back, behind all items)");
    });
    let tip_img = if st.image_as_background {
        format!("Apply image as page background to Page {cur_label} (proportional, sent to back)")
    } else {
        format!("Place image fitted inside the page margins of Page {cur_label}")
    };
    if outline_button(ui, "Page from Image", full_width(ui)).on_hover_text(tip_img).clicked() {
        if st.image_path.trim().is_empty()
            && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
        {
            st.image_path = path;
        }
        let path = st.image_path.trim().to_string();
        let res = execute_page_from_image(app, st, &path);
        st.card3_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(e) => (e, false),
        });
    }

    // Helper: Excel (.xlsx) to CSV Data
    divider(ui);
    let tip_excel = "Convert a multi-sheet Excel file (.xlsx) into 1 catalog CSV for Page from CSV + individual table CSVs (with auto-merged headers). \
        A sheet named COVER supplies the Publication Cover values (style, title1, title2, year, volume, no_catalogue, ISSN, office_name, image_1, image_lanscape, office_logo)";
    ui.checkbox(&mut st.excel_auto_generate, "Auto-generate pages, tables & cover from sheets")
        .on_hover_text("When checked, picking an Excel file automatically runs 'Page from CSV', inserts each sheet's table with default options, and generates the cover page before Page 1 from the COVER sheet");
    let resp_excel = outline_button(ui, "Excel to CSV Data…", full_width(ui)).on_hover_text(tip_excel);
    if resp_excel.clicked()
        && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("dataMerge"))
    {
        let res = execute_excel_and_generate(app, st, &path);
        st.card3_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(e) => (e, false),
        });
    }

    let tip_zip = "Download all generated catalog and table CSVs packaged in a single .zip archive";
    let zip_label =
        if !st.generated_zip_name.is_empty() { format!("Download {}…", st.generated_zip_name) } else { "Download CSVs as ZIP…".to_string() };
    if outline_button(ui, &zip_label, full_width(ui)).on_hover_text(tip_zip).clicked() {
        let res = execute_download_csv_zip(app, st);
        st.card3_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(e) => (e, false),
        });
    }

    let mode_str = if facing { "Double-sided" } else { "Single-sided" };
    ui.add(egui::Label::new(RichText::new(format!("Mode: {mode_str} · Selected: Page {cur_label}")).size(10.5).color(t.text_dim)).truncate());

    if let Some((msg, ok)) = &st.card3_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

fn badge(ui: &mut Ui, t: &Tokens, role: &str, col_name: &str) {
    let text = format!("{role}: {col_name}");
    let galley = ui.painter().layout_no_wrap(text, egui::FontId::proportional(10.0), t.text_strong);
    let (rect, _) = ui.allocate_exact_size(vec2(galley.size().x + 8.0, 16.0), Sense::hover());
    ui.painter().rect_filled(rect, 3.0, t.panel_darker);
    ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Middle);
    ui.painter().galley(rect.min + vec2(4.0, 2.0), galley, Color32::WHITE);
}

// ---------------------------------------------------------------------------
// CSV File Ingestion (RFC 4180 parsing with quotes and delimiter sniffing)
// ---------------------------------------------------------------------------

pub fn delimiter_name(delim: char) -> &'static str {
    match delim {
        ';' => "semicolon ';'",
        '\t' => "tab '\\t'",
        '|' => "pipe '|'",
        _ => "comma ','",
    }
}

/// Sniff candidate delimiter (',', ';', '\t', '|') using quotation-aware row sampling,
/// frequency analysis, and header consistency heuristics.
pub fn detect_delimiter(text: &str) -> char {
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
pub fn parse_csv_content_with_delim(text: &str, delim: char) -> Vec<Vec<String>> {
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

pub fn parse_csv_content(text: &str) -> Vec<Vec<String>> {
    let delim = detect_delimiter(text);
    parse_csv_content_with_delim(text, delim)
}

fn load_card1_csv(st: &mut PubgenState, path: &str) {
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

    st.card1_range_to = st.card1_records.len();
    let delim_name = delimiter_name(delim);
    let file_info = if headers.len() >= 4 { " · 'file' col detected" } else { "" };
    st.card1_status = Some((format!("Loaded {} records (delimiter: {delim_name}){file_info}", st.card1_records.len()), true));
}

fn load_card2_csv(st: &mut PubgenState, path: &str) {
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
    let r = st.card2_matrix.len();
    let c = st.card2_matrix.first().map(Vec::len).unwrap_or(0);
    let delim_name = delimiter_name(delim);
    st.card2_status = Some((format!("Loaded {r} rows × {c} columns (delimiter: {delim_name})"), true));
}

/// Detect if the last row of the tabular CSV only contains data in the first column
/// (the remaining columns are empty), functioning as a footnote or source note.
pub fn detect_footnote_row(matrix: &[Vec<String>]) -> Option<String> {
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
pub fn matches_pubgen_row_condition(text: &str) -> bool {
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

    match_word("jumlah")
        || match_word("tahun")
        || match_word("kabupaten")
        || match_word("kab")
        || match_word("kecamatan")
        || match_word("kec")
        || match_word("total")
        || match_word("sum")
        || match_word("subtotal")
        || match_word("district")
        || match_word("subdistrict")
        || match_word("year")
}

pub fn is_index_or_empty(text: &str) -> bool {
    let t = text.trim();
    t.is_empty() || t.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | ')' | '(' | '-' | '#'))
}

/// Detect row indices (0-based relative to matrix) matching the summary/break keyword condition.
pub fn detect_condition_rows(matrix: &[Vec<String>], first_row_header: bool) -> Vec<usize> {
    let start_idx = if first_row_header { 1 } else { 0 };
    let mut matching = Vec::new();
    for (i, row) in matrix.iter().enumerate().skip(start_idx) {
        let col0 = row.first().map(String::as_str).unwrap_or("");
        let col1 = row.get(1).map(String::as_str).unwrap_or("");
        if matches_pubgen_row_condition(col0) || (is_index_or_empty(col0) && matches_pubgen_row_condition(col1)) {
            matching.push(i);
        }
    }
    matching
}

/// Estimate raw advance width of a string in points (based on standard 12pt proportional font).
fn estimate_text_advance(text: &str) -> f64 {
    let mut w: f64 = 0.0;
    for c in text.chars() {
        w += match c {
            ' ' => 5.0,
            '.' | ',' | ':' | ';' | '!' | '\'' | '|' | '/' | '\\' => 3.2,
            'i' | 'l' | 'j' => 4.0,
            'I' | 'J' => 4.8,
            'f' | 't' | 'r' => 4.8,
            '-' | '–' | '—' => 5.5,
            '0'..='9' => 7.5,
            'a'..='z' => match c {
                'm' | 'w' => 10.0,
                _ => 6.8,
            },
            'A'..='Z' => match c {
                'M' | 'W' => 12.0,
                'C' | 'D' | 'G' | 'H' | 'K' | 'N' | 'O' | 'Q' | 'U' | 'V' | 'X' | 'Y' => 9.0,
                'E' | 'F' | 'L' | 'P' | 'S' | 'T' | 'Z' => 7.5,
                _ => 8.2,
            },
            _ => 7.5,
        };
    }
    w
}

/// Estimate single-line text advance width in points (based on standard 12pt proportional font).
fn estimate_text_width(text: &str, is_uppercase_kicker: bool) -> f64 {
    let w = estimate_text_advance(text);
    // Headroom buffer so Knuth-Plass / greedy line breaker never breaks single-line text
    let buffer = if is_uppercase_kicker { 12.0 } else { 8.0 };
    (w + buffer).max(20.0)
}

/// Resolve a template path or template ID from a catalog specification.
///
/// Supports:
/// - Direct file path (as-is or with `.designcraft`)
/// - Path relative to the catalog file's directory
/// - Path relative to `assets/templates/` or any of its subfolders (`dividers/`, `covers/`, etc.)
/// - Template ID match from scanned template catalog (e.g. `001_ulasan_chapter` or fuzzy stem)
pub fn resolve_template_path_spec(catalog_path: &str, spec: &str) -> Option<String> {
    let clean = spec.trim().trim_matches('"').trim();
    if clean.is_empty() {
        return None;
    }

    // 1. Direct path check as-is or with .designcraft
    let direct_p = std::path::Path::new(clean);
    if direct_p.is_file() {
        return Some(clean.to_string());
    }
    let direct_ext = format!("{clean}.designcraft");
    if std::path::Path::new(&direct_ext).is_file() {
        return Some(direct_ext);
    }

    // 2. Relative to catalog file directory
    if !catalog_path.is_empty() {
        let cat_p = std::path::Path::new(catalog_path);
        let base_dir = if cat_p.is_file() { cat_p.parent() } else { Some(cat_p) };
        if let Some(dir) = base_dir {
            let rel = dir.join(clean);
            if rel.is_file() {
                return Some(rel.to_string_lossy().to_string());
            }
            let rel_ext = dir.join(&direct_ext);
            if rel_ext.is_file() {
                return Some(rel_ext.to_string_lossy().to_string());
            }
            let rel_tpl = dir.join("templates").join(clean);
            if rel_tpl.is_file() {
                return Some(rel_tpl.to_string_lossy().to_string());
            }
            let rel_tpl_ext = dir.join("templates").join(&direct_ext);
            if rel_tpl_ext.is_file() {
                return Some(rel_tpl_ext.to_string_lossy().to_string());
            }
        }
    }

    // 3. Search in standard assets/templates root
    let root_candidates = [
        std::path::PathBuf::from("assets/templates"),
        std::path::PathBuf::from("../../assets/templates"),
        std::path::PathBuf::from("../assets/templates"),
    ];
    if let Some(root) = root_candidates.into_iter().find(|p| p.is_dir()) {
        // Direct under assets/templates
        let p1 = root.join(clean);
        if p1.is_file() {
            return Some(p1.to_string_lossy().to_string());
        }
        let p1_ext = root.join(&direct_ext);
        if p1_ext.is_file() {
            return Some(p1_ext.to_string_lossy().to_string());
        }

        // Try under subcategories (dividers, covers, front_matter, data_tables, infographics, articles)
        let subcats = ["dividers", "data_tables", "front_matter", "covers", "infographics", "articles"];
        for sub in subcats {
            let sub_p = root.join(sub).join(clean);
            if sub_p.is_file() {
                return Some(sub_p.to_string_lossy().to_string());
            }
            let sub_p_ext = root.join(sub).join(&direct_ext);
            if sub_p_ext.is_file() {
                return Some(sub_p_ext.to_string_lossy().to_string());
            }
        }

        // Match against template catalogue IDs (e.g. "001_ulasan_chapter" or fuzzy matching stem)
        let scanned = designcraft_engine::cmd::template::scan_template_catalog(&root);
        let clean_lower = clean.to_lowercase();
        // Exact ID match
        if let Some(found) = scanned.iter().find(|t| t.id.to_lowercase() == clean_lower) {
            return Some(found.path.to_string_lossy().to_string());
        }
        // If numeric 1-3 digits (e.g. "002" or "2"), check for divider templates with matching prefix
        if clean.len() <= 3 && clean.chars().all(|c| c.is_ascii_digit()) {
            let padded = format!("{:03}_", clean.parse::<u32>().unwrap_or(0));
            if let Some(found) = scanned.iter().find(|t| t.category == "dividers" && t.id.starts_with(&padded)) {
                return Some(found.path.to_string_lossy().to_string());
            }
            if let Some(found) = scanned.iter().find(|t| t.id.starts_with(&padded)) {
                return Some(found.path.to_string_lossy().to_string());
            }
        }
        // Substring / suffix / prefix match on ID
        if let Some(found) = scanned.iter().find(|t| {
            let id_l = t.id.to_lowercase();
            id_l.contains(&clean_lower) || clean_lower.contains(&id_l)
        }) {
            return Some(found.path.to_string_lossy().to_string());
        }
    }

    None
}

fn post_process_template_applied_page(
    app: &mut DesignApp,
    st: &PubgenState,
    applied_info: &Value,
    is_cover_or_div: bool,
    doc_margins: designcraft_doc::Margins,
    is_narrative_row: bool,
    title_val: &str,
    type_val: &str,
) -> Option<u64> {
    let mut table_placeholder_id = None;
    let mut story_ids_to_tag = Vec::new();
    let mut body_story_ids = Vec::new();
    let mut image_slot_ids = Vec::new();
    // Dual Lang Mode: the divider's English title boxes (styled after the loop below).
    let mut title_en_stories: Vec<designcraft_doc::StoryId> = Vec::new();
    if let Some(items) = applied_info.get("items").and_then(Value::as_array) {
        if let Some(active) = app.session.active() {
            for it_id_val in items {
                if let Some(it_id) = it_id_val.as_u64() {
                    let item_id = designcraft_doc::ItemId(it_id);
                    if let Some(loc) = active.doc.find(item_id)
                        && let designcraft_doc::SpreadRef::Doc(si) = loc.spread
                        && let Some(spread) = active.doc.spreads.get(si)
                        && let Some(item) = spread.items.iter().find(|i| i.id == item_id)
                    {
                        if st.bilingual_output
                            && type_val.eq_ignore_ascii_case("Divider")
                            && item.label.starts_with("#slot:text:title_en")
                            && let Some(tf) = item.text_frame()
                            && !title_en_stories.contains(&tf.story)
                        {
                            title_en_stories.push(tf.story);
                        }
                        if item.label.starts_with("#slot:table")
                            || (item.label.contains("table") && !matches!(item.content, designcraft_doc::Content::Text(_)))
                        {
                            table_placeholder_id = Some(it_id);
                        } else if (item.label.contains("title") || item.label.contains("chapter_title"))
                            && let Some(tf) = item.text_frame()
                        {
                            story_ids_to_tag.push(tf.story);
                        } else if item.label.starts_with("#slot:story:")
                            && let Some(tf) = item.text_frame()
                        {
                            body_story_ids.push(tf.story);
                        } else if item.label.contains("image") || item.label.contains("photo") || item.label.contains("hero") {
                            image_slot_ids.push(it_id);
                        }
                    }
                }
            }
        }

        // Dual Lang Mode: the divider's title_en textbox is 22 pt italic with a black
        // stroke behind the fill.
        for sid in title_en_stories {
            let len = app.session.active().and_then(|a| a.doc.story(sid)).map(|story| story.text.len()).unwrap_or(0);
            if len > 0 {
                let _ = app.run("text.select", json!({"story": sid, "anchor": 0, "focus": len}));
                let _ = app.run("type.char", json!({"size": 22.0, "fontStyle": "Italic", "stroke": "[Black]", "strokeBehindFill": true}));
            }
        }

        for img_id in image_slot_ids {
            let fit_mode = if is_cover_or_div { "fitContentToFrame" } else { "fitProportionally" };
            let _ = app.run("object.fit", json!({"mode": fit_mode, "ids": [img_id]}));
            let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [img_id]}));
            let _ = app.run("object.fittingOptions", json!({"ids": [img_id], "fitting": fit_mode}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]", "ids": [img_id]}));
        }

        if !story_ids_to_tag.is_empty()
            && let Some(active) = app.session.active_mut()
        {
            // A narrative page (the G and Ulasan sheets) carries the narrative style, so the
            // paragraph table of contents lists it and nothing else.
            let title_style = if is_narrative_row { TOC_NARRATIVE_STYLE.to_string() } else { st.toc_style.clone() };
            let doc = std::sync::Arc::make_mut(&mut active.doc);
            for sid in story_ids_to_tag {
                if let Some(story) = doc.story_mut(sid) {
                    for p in &mut story.paras {
                        p.style = title_style.clone();
                    }
                }
            }
        }

        // A sheet with no title line puts the RTF at the top of the page, so the page's title is
        // the RTF's own heading: tag that paragraph for the paragraph table of contents.
        if title_val.trim().is_empty() && (is_narrative_row || type_val.eq_ignore_ascii_case("Note")) {
            for sid in body_story_ids {
                tag_page_title(app, sid, TOC_NARRATIVE_STYLE);
            }
        }
    }

    // 1. Update page margins for modular template:
    // DIV/COVER: margins = 0.0 (adjust to Page Size)
    // Content page: margins = doc_margins (adjust to Page Margin)
    let page_margins_info = app.session.edit(|doc, _| {
        let cur_p = doc.page_count().saturating_sub(1);
        if let Some((si, pi)) = doc.page_loc(cur_p)
            && let Some(sp) = doc.spreads.get_mut(si)
        {
            let sp = std::sync::Arc::make_mut(sp);
            if let Some(pg) = sp.pages.get_mut(pi) {
                pg.margins = if is_cover_or_div { designcraft_doc::Margins::uniform(0.0) } else { doc_margins };
                pg.local_margins = true;
            }
            sp.relayout();
            if let Some(pg) = sp.pages.get(pi) {
                return Ok(serde_json::json!({
                    "x0": pg.margin_rect().x0,
                    "x1": pg.margin_rect().x1,
                    "page_x0": pg.bounds().x0,
                    "page_w": pg.bounds().width(),
                }));
            }
        }
        Ok(serde_json::Value::Null)
    });

    // For content pages (not COVER or DIV), adjust template items to follow page margins
    if !is_cover_or_div
        && let Ok(info_val) = page_margins_info
        && let (Some(m_x0), Some(m_x1), Some(pg_x0), Some(pg_w)) = (
            info_val.get("x0").and_then(Value::as_f64),
            info_val.get("x1").and_then(Value::as_f64),
            info_val.get("page_x0").and_then(Value::as_f64),
            info_val.get("page_w").and_then(Value::as_f64),
        )
        && let Some(items) = applied_info.get("items").and_then(Value::as_array)
    {
        let item_ids: Vec<designcraft_doc::ItemId> = items.iter().filter_map(Value::as_u64).map(designcraft_doc::ItemId).collect();

        let local_m_x0 = m_x0 - pg_x0;
        let local_m_x1 = m_x1 - pg_x0;
        let _ = app.session.edit(|doc, _| {
            for &it_id in &item_ids {
                if let Some(it) = doc.item_mut(it_id)
                    && let Some(pb) = it.path.bounds()
                {
                    // Template items originally placed at template's default margin (36pt)
                    if (pb.x0 - 36.0).abs() < 4.0 {
                        let is_full_width = (pb.x1 - (pg_w - 36.0)).abs() < 4.0;
                        let new_x1 = if is_full_width { local_m_x1 } else { (local_m_x0 + pb.width()).min(local_m_x1) };
                        it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(local_m_x0, pb.y0, new_x1, pb.y1));
                    }
                }
            }
            Ok(serde_json::Value::Null)
        });
    }

    table_placeholder_id
}

fn execute_generate_pages(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    if st.card1_records.is_empty() {
        return Err("No records to generate".into());
    }

    let total = st.card1_records.len();
    let (start_idx, end_idx) = if st.card1_range_all {
        (0, total)
    } else {
        let from = st.card1_range_from.clamp(1, total).saturating_sub(1);
        let to = st.card1_range_to.clamp(from + 1, total);
        (from, to)
    };

    let slice = st.card1_records.get(start_idx..end_idx).ok_or("Invalid range")?;
    let mut generated_count = 0usize;
    let mut inserted_tables_count = 0usize;
    let mut inserted_rtfs_count = 0usize;
    let mut inserted_images_count = 0usize;

    // Every preset tags its title (or the combined TOC line) with the TOC paragraph style, so the
    // Quick Actions "Add TOC" lists one entry per generated page. The narrative pages (the G and
    // Ulasan sheets) carry the narrative style instead, so the paragraph list names them alone.
    ensure_para_style(app, &st.toc_style);
    ensure_para_style(app, TOC_NARRATIVE_STYLE);

    let doc_margins = {
        let active = app.session.active().ok_or("No active document")?;
        detect_document_margins(&active.doc)
    };

    let template_col = st.card1_headers.iter().position(|h| h.trim().eq_ignore_ascii_case("template"));
    let title_en_col = st.card1_headers.iter().position(|h| h.trim().eq_ignore_ascii_case("title_en"));
    let file_en_col = st.card1_headers.iter().position(|h| h.trim().eq_ignore_ascii_case("file_en"));

    for row in slice {
        let type_val = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let num_val = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let title_val = row.get(st.card1_title_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let title_en_val = title_en_col.and_then(|c| row.get(c)).map(String::as_str).unwrap_or("").trim();
        let file_en_val = file_en_col.and_then(|c| row.get(c)).map(String::as_str).unwrap_or("").trim();
        let file_val = row.get(3).map(String::as_str).unwrap_or("").trim();
        let (clean_id_title, clean_en_title) = if let Some((id_t, en_t)) = locale::split_pair(title_val) {
            (id_t, if title_en_val.is_empty() { en_t } else { title_en_val })
        } else {
            (title_val, title_en_val)
        };
        let has_chapter_prefix = clean_id_title.split_whitespace().next().is_some_and(|w| w.eq_ignore_ascii_case("bab"));
        // The preface stays single-language: it is not processed in Dual Lang Mode.
        let is_preface = title_val.eq_ignore_ascii_case("preface") || clean_id_title.eq_ignore_ascii_case("preface");
        let is_div = is_divider_type(type_val);
        let is_cover_or_div = is_div || type_val.eq_ignore_ascii_case("COVER");
        // The narrative sheets: `GXX` (the front matter) and `RXX` (the chapter reviews). Their
        // pages are the ones the paragraph table of contents lists.
        let is_narrative_row = type_val.eq_ignore_ascii_case("Ulasan") || type_val.eq_ignore_ascii_case("G");

        let row_tpl_spec = template_col.and_then(|c| row.get(c)).map(String::as_str).unwrap_or("").trim();
        let explicit_tpl_path = if !row_tpl_spec.is_empty() { resolve_template_path_spec(&st.card1_path, row_tpl_spec) } else { None };

        if explicit_tpl_path.is_some() || st.card1_preset == 6 || is_div {
            // Modular Template Mode: delegate to template engine
            let raw_tpl = if let Some(ref ep) = explicit_tpl_path {
                ep.clone()
            } else if !st.card1_template_path.trim().is_empty() {
                st.card1_template_path.trim().to_string()
            } else if is_divider_type(type_val) {
                "assets/templates/dividers/002_chapter_devider.designcraft".to_string()
            } else if is_ulasan_type(type_val) {
                "assets/templates/dividers/001_ulasan_chapter.designcraft".to_string()
            } else if type_val.eq_ignore_ascii_case("Note") || type_val.eq_ignore_ascii_case("G") {
                "assets/templates/front_matter/001_publication_note.designcraft".to_string()
            } else {
                "assets/templates/data_tables/001_single_table_standard.designcraft".to_string()
            };

            let tpl_path = resolve_template_path_spec(&st.card1_path, &raw_tpl).unwrap_or(raw_tpl);

            if std::path::Path::new(&tpl_path).exists() {
                let mut text_slots = serde_json::Map::new();
                let mut story_slots = serde_json::Map::new();
                let mut image_slots = serde_json::Map::new();

                if is_divider_type(type_val) {
                    let (first_word, digit_num, rest_text) = excel::split_divider_title(title_val);
                    let prefix = if first_word.is_empty() || first_word.chars().all(|c| c.is_ascii_digit()) {
                        "BAB".to_string()
                    } else if !rest_text.is_empty() {
                        first_word.clone()
                    } else {
                        "BAB".to_string()
                    };
                    text_slots.insert("first_word".into(), json!(prefix));
                    text_slots.insert("prefix".into(), json!(prefix));
                    text_slots.insert("chapter_prefix".into(), json!(prefix));
                    text_slots.insert("title_prefix".into(), json!(prefix));

                    // 2. digit number (replacing text "01")
                    let final_digit = if !digit_num.is_empty() { digit_num } else { num_val.to_string() };
                    text_slots.insert("chapter_num".into(), json!(final_digit));
                    text_slots.insert("number".into(), json!(final_digit));
                    text_slots.insert("digit".into(), json!(final_digit));
                    text_slots.insert("num".into(), json!(final_digit));

                    // 3. rest of text (replacing text "Geografi")
                    let mut final_title = if !rest_text.is_empty() {
                        rest_text
                    } else if !first_word.eq_ignore_ascii_case("bab") && !first_word.chars().all(|c| c.is_ascii_digit()) {
                        first_word
                    } else {
                        title_val.to_string()
                    };

                    let title_en = if st.bilingual_output {
                        if !title_en_val.is_empty() {
                            title_en_val.to_string()
                        } else if let Some((id_t, en_t)) = locale::split_pair(&final_title) {
                            let (id_s, en_s) = (id_t.to_string(), en_t.to_string());
                            final_title = id_s;
                            en_s
                        } else {
                            String::new()
                        }
                    } else {
                        if let Some((id_t, en_t)) = locale::split_pair(&final_title) {
                            let (id_s, en_s) = (id_t.to_string(), en_t.to_string());
                            final_title = match st.locale {
                                locale::Locale::Id => id_s,
                                locale::Locale::En => en_s,
                            };
                        }
                        String::new()
                    };

                    text_slots.insert("chapter_title".into(), json!(final_title));
                    text_slots.insert("title".into(), json!(final_title));
                    text_slots.insert("rest".into(), json!(final_title));
                    text_slots.insert("title_rest".into(), json!(final_title));
                    if !is_preface {
                        text_slots.insert("title_en".into(), json!(title_en));
                        text_slots.insert("chapter_title_en".into(), json!(title_en));
                    }

                    // Shape with color #e5007e is placeholder for image in col image
                    if let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty())
                        && let Some(img_file) = resolve_catalog_related_file(&st.card1_path, file_str, "png")
                            .or_else(|| resolve_catalog_related_file(&st.card1_path, file_str, "jpg"))
                            .or_else(|| resolve_catalog_related_file(&st.card1_path, file_str, "jpeg"))
                            .or_else(|| resolve_catalog_related_file(&st.card1_path, file_str, "pdf"))
                            .or_else(|| {
                                let p = std::path::Path::new(file_str);
                                if p.is_file() { Some(p.to_path_buf()) } else { None }
                            })
                    {
                        image_slots.insert("image".into(), json!(img_file.to_string_lossy()));
                        image_slots.insert("photo".into(), json!(img_file.to_string_lossy()));
                        image_slots.insert("hero".into(), json!(img_file.to_string_lossy()));
                        image_slots.insert("divider_image".into(), json!(img_file.to_string_lossy()));
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }
                } else if is_ulasan_type(type_val) {
                    let (ch_line, title_line) = split_chapter_title(clean_id_title, num_val);
                    text_slots.insert("chapter_num".into(), json!(ch_line));
                    text_slots.insert("chapter_title".into(), json!(title_line));
                    text_slots.insert("title".into(), json!(title_line));
                    text_slots.insert("number".into(), json!(ch_line));
                    if let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty())
                        && let Some(rtf_file) = resolve_catalog_related_file(&st.card1_path, file_str, "rtf")
                    {
                        story_slots.insert("narrative_body".into(), json!(rtf_file.to_string_lossy()));
                        inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                    }
                } else if type_val.eq_ignore_ascii_case("Note") || type_val.eq_ignore_ascii_case("G") {
                    let note_title = title_val.trim();
                    text_slots.insert("title".into(), json!(note_title));
                    text_slots.insert("note_title".into(), json!(note_title));
                    if let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty())
                        && let Some(rtf_file) = resolve_catalog_related_file(&st.card1_path, file_str, "rtf")
                    {
                        story_slots.insert("note_body".into(), json!(rtf_file.to_string_lossy()));
                        inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                    }
                } else if is_chart_type(type_val) {
                    let (ch_line, title_line) = split_chart_title(title_val, num_val, type_val);
                    text_slots.insert("chart_num".into(), json!(ch_line));
                    text_slots.insert("chart_title".into(), json!(title_line));
                    text_slots.insert("title".into(), json!(title_line));
                    text_slots.insert("number".into(), json!(ch_line));
                    if st.bilingual_output && !title_en_val.is_empty() {
                        text_slots.insert("chart_title_en".into(), json!(title_en_val));
                        text_slots.insert("title_en".into(), json!(title_en_val));
                    }
                    if let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty())
                        && let Some(img_file) = resolve_catalog_related_file(&st.card1_path, file_str, "png")
                            .or_else(|| resolve_catalog_related_file(&st.card1_path, file_str, "jpg"))
                            .or_else(|| resolve_catalog_related_file(&st.card1_path, file_str, "jpeg"))
                    {
                        image_slots.insert("chart_image".into(), json!(img_file.to_string_lossy()));
                        image_slots.insert("image".into(), json!(img_file.to_string_lossy()));
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }
                } else {
                    let (id_title, en_title_opt) =
                        excel::split_and_harmonize_table_title(title_val, if title_en_val.is_empty() { None } else { Some(title_en_val) });
                    let formatted_num = excel::format_table_number(num_val, file_val);
                    if st.bilingual_output || en_title_opt.is_some() {
                        let kind_en = format!("{} {}", locale::words(locale::Locale::En).table, formatted_num);
                        text_slots.insert("table_num".into(), json!(format!("{} {}", locale::words(locale::Locale::Id).table, formatted_num)));
                        text_slots.insert("table_kind_en".into(), json!(kind_en));
                        text_slots.insert("table_title".into(), json!(id_title));
                        text_slots.insert("table_title_en".into(), json!(en_title_opt.unwrap_or_default()));
                        text_slots.insert("title".into(), json!(id_title));
                        text_slots.insert("number".into(), json!(formatted_num));
                    } else {
                        let table_title_val = match st.locale {
                            locale::Locale::Id => id_title,
                            locale::Locale::En => en_title_opt.unwrap_or(id_title),
                        };
                        text_slots.insert("table_num".into(), json!(formatted_num));
                        text_slots.insert("table_kind_en".into(), json!(""));
                        text_slots.insert("table_title".into(), json!(table_title_val));
                        text_slots.insert("table_title_en".into(), json!(""));
                        text_slots.insert("title".into(), json!(table_title_val));
                        text_slots.insert("number".into(), json!(formatted_num));
                    }
                }

                let mut apply_params = json!({
                    "path": tpl_path,
                    "slots": {
                        "text": text_slots,
                        "story": story_slots,
                        "image": image_slots,
                    },
                    "image_fitting": if is_div { "fitContentToFrame" } else { "fitProportionally" },
                    "palette": {
                        "Primary": st.cover_color_primary.trim(),
                        "Accent": st.cover_color_accent.trim(),
                        "Dark": st.cover_color_dark.trim(),
                    }
                });
                if !st.doc_language.trim().is_empty() {
                    apply_params["lang"] = json!(st.doc_language.trim());
                }
                let apply_res = app.run("template.page.apply", apply_params);

                if let Ok(applied_info) = apply_res {
                    let table_placeholder_id = post_process_template_applied_page(
                        app,
                        st,
                        &applied_info,
                        is_cover_or_div,
                        doc_margins,
                        is_narrative_row,
                        title_val,
                        type_val,
                    );
                    generated_count = generated_count.saturating_add(1);

                    // Auto-insert table from CSV if enabled and present
                    if st.card1_auto_insert_tables
                        && let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty())
                        && let Some(tbl_file) = resolve_catalog_related_file(&st.card1_path, file_str, "csv")
                        && let Ok(content) = std::fs::read_to_string(&tbl_file)
                    {
                        if let Some(tbl_slot_id) = table_placeholder_id {
                            let _ = app.run("edit.clear", json!({"ids": [tbl_slot_id]}));
                        }
                        let delim = detect_delimiter(&content);
                        let tbl_matrix = parse_csv_content_with_delim(&content, delim);
                        if !tbl_matrix.is_empty() {
                            let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                            if execute_insert_table_matrix(app, st, Some(&tbl_matrix), Some(cur_page_idx)).is_ok() {
                                inserted_tables_count = inserted_tables_count.saturating_add(1);
                            }
                        }
                    }

                    // Sequential EN spread for Ulasan — the preface is not processed in Dual
                    // Lang Mode.
                    if st.bilingual_output
                        && !file_en_val.is_empty()
                        && !is_preface
                        && is_ulasan_type(type_val)
                        && let Some(en_rtf_file) = resolve_catalog_related_file(&st.card1_path, file_en_val, "rtf")
                    {
                        let (en_ch_line, en_title_line) = if has_chapter_prefix {
                            if !clean_en_title.is_empty() {
                                (format!("CHAPTER {num_val}"), clean_en_title.to_string())
                            } else if let Ok(en_bytes) = std::fs::read(&en_rtf_file)
                                && let Ok(imp) = designcraft_textimport::import("review_en.rtf", &en_bytes)
                                && let Some(first_para) = imp.story.text.lines().map(str::trim).find(|l| !l.is_empty())
                            {
                                split_chapter_title_locale(first_para, num_val, true)
                            } else {
                                (format!("CHAPTER {num_val}"), clean_id_title.to_string())
                            }
                        } else {
                            (String::new(), if !clean_en_title.is_empty() { clean_en_title.to_string() } else { clean_id_title.to_string() })
                        };

                        let mut en_text_slots = serde_json::Map::new();
                        en_text_slots.insert("chapter_num".into(), json!(en_ch_line));
                        en_text_slots.insert("chapter_title".into(), json!(en_title_line));
                        en_text_slots.insert("title".into(), json!(en_title_line));
                        en_text_slots.insert("number".into(), json!(en_ch_line));

                        let mut en_story_slots = serde_json::Map::new();
                        en_story_slots.insert("narrative_body".into(), json!(en_rtf_file.to_string_lossy()));

                        let mut en_apply_params = json!({
                            "path": tpl_path,
                            "slots": {
                                "text": en_text_slots,
                                "story": en_story_slots,
                                "image": serde_json::Map::new(),
                            },
                            "image_fitting": "fitProportionally",
                            "palette": {
                                "Primary": st.cover_color_primary.trim(),
                                "Accent": st.cover_color_accent.trim(),
                                "Dark": st.cover_color_dark.trim(),
                            }
                        });
                        if !st.doc_language.trim().is_empty() {
                            en_apply_params["lang"] = json!(st.doc_language.trim());
                        }

                        if let Ok(en_applied) = app.run("template.page.apply", en_apply_params) {
                            post_process_template_applied_page(app, st, &en_applied, false, doc_margins, is_narrative_row, title_val, type_val);
                            let en_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                            italicize_page_stories(app, en_page_idx, en_page_idx);
                            inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                            generated_count = generated_count.saturating_add(1);
                        }
                    }

                    continue;
                }
            }
        }

        // 1. Insert Page
        app.run("layout.pages.insert", json!({"count": 1}))?;

        // Adjust page margins:
        // - DIV/COVER sheet: adjusts to Page Size (margins = 0)
        // - Content pages: adjust with Page Margin (margins = doc_margins)
        let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
        let _ = app.session.edit(|doc, _| {
            if let Some((si, pi)) = doc.page_loc(cur_page_idx)
                && let Some(sp) = doc.spreads.get_mut(si)
            {
                let sp = std::sync::Arc::make_mut(sp);
                if let Some(pg) = sp.pages.get_mut(pi) {
                    pg.margins = if is_cover_or_div { designcraft_doc::Margins::uniform(0.0) } else { doc_margins };
                    pg.local_margins = true;
                }
                sp.relayout();
            }
            Ok(serde_json::Value::Null)
        });

        // 2. Query Page Bounds & Margins
        let (margin, page_bounds, spread_idx) = {
            let active = app.session.active().ok_or("No active document")?;
            let doc = &active.doc;
            let (si, pi) = doc.page_loc(cur_page_idx).ok_or("Page loc missing")?;
            let spread = doc.spreads.get(si).ok_or("Spread missing")?;
            let page = spread.pages.get(pi).ok_or("Page missing")?;
            (page.margin_rect(), page.bounds(), si)
        };

        if st.card1_preset == 5 {
            // TOC Line: one combined paragraph per record (type + number + title), so each record
            // becomes a single table-of-contents entry.
            let parts: Vec<&str> = [type_val, num_val, title_val].into_iter().filter(|s| !s.is_empty()).collect();
            let line = parts.join(" ");
            if !line.is_empty() {
                frame_with_style(app, spread_idx, [margin.x0, margin.y0, margin.x1, margin.y0 + 40.0], &line, &st.toc_style);
            }
        } else if type_val.eq_ignore_ascii_case("Ulasan") {
            // Ulasan / Editorial Chapter layout:
            // Split title "BAB 01 Pemerintahan" into "BAB 01" and "Pemerintahan"
            let (ch_line, title_line) = split_chapter_title(clean_id_title, num_val);
            let resolved_en_rtf = if st.bilingual_output && !file_en_val.is_empty() && !is_preface {
                resolve_catalog_related_file(&st.card1_path, file_en_val, "rtf")
            } else {
                None
            };

            if st.bilingual_output
                && has_chapter_prefix
                && let Some(en_rtf_file) = resolved_en_rtf
            {
                // Bilingual Review Chapter (RXX):
                // 3-Line Header on Page 1:
                // Line 1: "BAB 01" (Arial Black, 24.0)
                // Line 2: "Geografi" (Arial Black, 20.0, tagged with TOC style)
                // Line 3: "Chapter 01 Geography" (Arial Bold Italic, 15.0)
                if !ch_line.is_empty() {
                    frame_with_font_full(
                        app,
                        spread_idx,
                        [margin.x0, margin.y0, margin.x1, margin.y0 + 36.0],
                        &ch_line,
                        "Arial Black",
                        None,
                        24.0,
                        None,
                    );
                }
                // 2nd Textbox: "Pemerintahan" (font Arial Black, size 20.0) -> second row
                // Tag with the narrative style so the paragraph table of contents lists this
                // chapter (and only the narrative pages).
                if !title_line.is_empty() {
                    frame_with_font_full(
                        app,
                        spread_idx,
                        [margin.x0, margin.y0 + 40.0, margin.x1, margin.y0 + 72.0],
                        &title_line,
                        "Arial Black",
                        None,
                        20.0,
                        Some(TOC_NARRATIVE_STYLE),
                    );
                }
                let en_header_text = if !clean_en_title.is_empty() {
                    let t = clean_en_title.trim();
                    if t.to_lowercase().starts_with("chapter") {
                        t.to_string()
                    } else if !num_val.is_empty() {
                        format!("Chapter {num_val} {t}")
                    } else {
                        t.to_string()
                    }
                } else if !title_en_val.is_empty() {
                    title_en_val.to_string()
                } else {
                    format!("Chapter {num_val} {title_line}")
                };
                if !en_header_text.is_empty() {
                    frame_with_font_full(
                        app,
                        spread_idx,
                        [margin.x0, margin.y0 + 76.0, margin.x1, margin.y0 + 104.0],
                        &en_header_text,
                        "Arial",
                        Some("Bold Italic"),
                        15.0,
                        None,
                    );
                }

                let header_h = 114.0;
                let gutter = 14.17; // 5 mm
                let total_w = (margin.x1 - margin.x0).max(40.0);
                let col_w = ((total_w - gutter) / 2.0).max(10.0);
                let left_rect = [margin.x0, margin.y0 + header_h, margin.x0 + col_w, margin.y1];
                let right_rect = [margin.x0 + col_w + gutter, margin.y0 + header_h, margin.x1, margin.y1];

                let left_fid = app
                    .run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": left_rect,
                            "content": "text",
                            "caret": false
                        }),
                    )
                    .ok()
                    .and_then(|res| res.get("id").and_then(Value::as_u64));

                let right_fid = app
                    .run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": right_rect,
                            "content": "text",
                            "caret": false
                        }),
                    )
                    .ok()
                    .and_then(|res| res.get("id").and_then(Value::as_u64));

                let mut cur_left_fid = left_fid;
                let mut cur_right_fid = right_fid;
                let mut id_over = false;
                let mut en_over = false;
                let mut en_story_id = None;

                if let Some(fid) = left_fid
                    && let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty())
                    && let Some(rtf_file) = resolve_catalog_related_file(&st.card1_path, file_str, "rtf")
                {
                    let rtf_path_str = rtf_file.to_string_lossy().to_string();
                    let res = app.run(
                        "file.place",
                        json!({
                            "path": rtf_path_str,
                            "frame": fid,
                            "autoflow": false,
                        }),
                    );
                    if let Ok(val) = res {
                        let sid = val.get("story").and_then(Value::as_u64).map(designcraft_doc::StoryId);
                        id_over = val.get("overset").and_then(Value::as_bool).unwrap_or(false) || sid.is_some_and(|s| story_is_overset(app, s));
                        inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                    }
                }

                if let Some(fid) = right_fid {
                    let en_path_str = en_rtf_file.to_string_lossy().to_string();
                    let res = app.run(
                        "file.place",
                        json!({
                            "path": en_path_str,
                            "frame": fid,
                            "autoflow": false,
                        }),
                    );
                    if let Ok(val) = res {
                        en_story_id = val.get("story").and_then(Value::as_u64);
                        let sid = en_story_id.map(designcraft_doc::StoryId);
                        en_over = val.get("overset").and_then(Value::as_bool).unwrap_or(false) || sid.is_some_and(|s| story_is_overset(app, s));
                        inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                    }
                }

                if en_story_id.is_none()
                    && let Some(fid) = right_fid
                    && let Some(active) = app.session.active()
                {
                    en_story_id = active.doc.item(designcraft_doc::ItemId(fid)).and_then(|it| it.text_frame()).map(|tf| tf.story.0);
                }

                let mut page_loop_count = 0usize;
                while (id_over || en_over) && page_loop_count < 50 {
                    page_loop_count = page_loop_count.saturating_add(1);
                    let _ = app.run("layout.pages.insert", json!({"count": 1}));
                    let new_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);

                    let _ = app.session.edit(|doc, _| {
                        if let Some((si, pi)) = doc.page_loc(new_page_idx)
                            && let Some(sp) = doc.spreads.get_mut(si)
                        {
                            let sp = std::sync::Arc::make_mut(sp);
                            if let Some(pg) = sp.pages.get_mut(pi) {
                                pg.margins = doc_margins;
                                pg.local_margins = true;
                            }
                            sp.relayout();
                        }
                        Ok(serde_json::Value::Null)
                    });

                    let (new_margin, new_spread_idx) = {
                        if let Some(active) = app.session.active()
                            && let Some((si, pi)) = active.doc.page_loc(new_page_idx)
                            && let Some(spread) = active.doc.spreads.get(si)
                            && let Some(page) = spread.pages.get(pi)
                        {
                            (page.margin_rect(), si)
                        } else {
                            (margin, spread_idx)
                        }
                    };

                    let new_total_w = (new_margin.x1 - new_margin.x0).max(40.0);
                    let new_col_w = ((new_total_w - gutter) / 2.0).max(10.0);
                    let new_left_rect = [new_margin.x0, new_margin.y0, new_margin.x0 + new_col_w, new_margin.y1];
                    let new_right_rect = [new_margin.x0 + new_col_w + gutter, new_margin.y0, new_margin.x1, new_margin.y1];

                    if id_over && let Some(prev_fid) = cur_left_fid {
                        let res = app.run(
                            "text.thread",
                            json!({
                                "from": prev_fid,
                                "spread": new_spread_idx,
                                "rect": new_left_rect,
                                "autoflow": false,
                            }),
                        );
                        if let Ok(val) = res {
                            cur_left_fid = val.get("to").and_then(Value::as_u64);
                            let sid = val.get("story").and_then(Value::as_u64).map(designcraft_doc::StoryId);
                            id_over = val.get("overset").and_then(Value::as_bool).unwrap_or(false) || sid.is_some_and(|s| story_is_overset(app, s));
                        } else {
                            id_over = false;
                        }
                    } else {
                        id_over = false;
                    }

                    if en_over && let Some(prev_fid) = cur_right_fid {
                        let res = app.run(
                            "text.thread",
                            json!({
                                "from": prev_fid,
                                "spread": new_spread_idx,
                                "rect": new_right_rect,
                                "autoflow": false,
                            }),
                        );
                        if let Ok(val) = res {
                            cur_right_fid = val.get("to").and_then(Value::as_u64);
                            let sid = val.get("story").and_then(Value::as_u64).map(designcraft_doc::StoryId);
                            en_over = val.get("overset").and_then(Value::as_bool).unwrap_or(false) || sid.is_some_and(|s| story_is_overset(app, s));
                        } else {
                            en_over = false;
                        }
                    } else {
                        en_over = false;
                    }

                    generated_count = generated_count.saturating_add(1);
                }

                if let Some(sid) = en_story_id {
                    italicize_story(app, sid);
                }
            } else {
                let header_h = if has_chapter_prefix { 90.0 } else { 44.0 };

                if has_chapter_prefix {
                    // 1st Textbox: "BAB 01" (font Arial Black, size 24.0) -> first row
                    if !ch_line.is_empty() {
                        frame_with_font_and_style(
                            app,
                            spread_idx,
                            [margin.x0, margin.y0, margin.x1, margin.y0 + 36.0],
                            &ch_line,
                            "Arial Black",
                            24.0,
                            None,
                        );
                    }
                    // 2nd Textbox: "Pemerintahan" (font Arial Black, size 20.0) -> second row
                    // Tag with the narrative style so the paragraph table of contents lists this
                    // chapter (and only the narrative pages).
                    if !title_line.is_empty() {
                        frame_with_font_and_style(
                            app,
                            spread_idx,
                            [margin.x0, margin.y0 + 40.0, margin.x1, margin.y0 + 80.0],
                            &title_line,
                            "Arial Black",
                            20.0,
                            Some(TOC_NARRATIVE_STYLE),
                        );
                    }
                } else if !clean_id_title.is_empty() {
                    // Front matter: the title alone, centered across the page margins.
                    if let Some(sid) = frame_with_font_and_style(
                        app,
                        spread_idx,
                        [margin.x0, margin.y0, margin.x1, margin.y0 + 40.0],
                        &clean_id_title.to_uppercase(),
                        "Arial",
                        12.0,
                        Some(TOC_NARRATIVE_STYLE),
                    ) {
                        let _ = app.run("text.select", json!({"story": sid, "anchor": 0, "focus": clean_id_title.chars().count() as u64}));
                        let _ = app.run("type.alignCenter", json!({}));
                    }
                }

                let (mw, mh) = (margin.x1 - margin.x0, margin.y1 - margin.y0);
                // Images of a front-matter sheet: `image1` (official photo) wraps tight around the text (Word "Tight")
                // placed before RTF placement; `image2` (signature) is anchored dynamically to the signatory block.
                if !has_chapter_prefix {
                    let img1_rect = [margin.x0, margin.y0 + mh * 0.254, margin.x0 + mw * 0.335, margin.y0 + mh * 0.528];
                    if place_catalog_image(app, st, row, "image1", spread_idx, img1_rect, "boundingBox").is_some() {
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }
                }

                // Place RTF narrative body into a frame below the headers
                let mut placed_rtf_fid: Option<u64> = None;
                if let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    let resolved_rtf = resolve_catalog_related_file(&st.card1_path, file_str, "rtf");

                    if let Some(rtf_file) = resolved_rtf {
                        let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                        crate::canvas::go_to_page(app, cur_page_idx);
                        let rtf_y0 = margin.y0 + header_h;
                        let rtf_rect = [margin.x0, rtf_y0, margin.x1, margin.y1];
                        let created_fid = app
                            .run(
                                "frame.create",
                                json!({
                                    "spread": spread_idx,
                                    "rect": rtf_rect,
                                    "content": "text",
                                    "caret": false
                                }),
                            )
                            .ok()
                            .and_then(|res| res.get("id").and_then(Value::as_u64));

                        if let Some(fid) = created_fid {
                            let _ = app.run("selection.set", json!({"ids": [fid]}));
                            let mut st_copy = st.clone();
                            let rtf_path_str = rtf_file.to_string_lossy().to_string();
                            if execute_page_from_rtf(app, &mut st_copy, &rtf_path_str).is_ok() {
                                inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                                placed_rtf_fid = Some(fid);
                            }
                        }
                    }
                }

                // Dynamically anchor image2 (signature) around the signatory name line in the RTF
                if !has_chapter_prefix {
                    let default_img2_rect = [margin.x1 - mw * 0.244, margin.y0 + mh * 0.354, margin.x1 - mw * 0.044, margin.y0 + mh * 0.464];
                    let (target_spread, img2_rect) = placed_rtf_fid
                        .map(|fid| compute_signature_placement(app, fid, default_img2_rect, spread_idx, margin))
                        .unwrap_or((spread_idx, default_img2_rect));
                    if place_catalog_image(app, st, row, "image2", target_spread, img2_rect, "none").is_some() {
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }
                }

                // Sequential EN spread for front-matter Ulasan in procedural mode
                if st.bilingual_output
                    && !file_en_val.is_empty()
                    && !has_chapter_prefix
                    && let Some(en_rtf_file) = resolve_catalog_related_file(&st.card1_path, file_en_val, "rtf")
                {
                    let en_title_line = if !clean_en_title.is_empty() { clean_en_title.to_string() } else { clean_id_title.to_string() };

                    let _ = app.run("layout.pages.insert", json!({"count": 1}));
                    let en_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                    let _ = app.session.edit(|doc, _| {
                        if let Some((si, pi)) = doc.page_loc(en_page_idx)
                            && let Some(sp) = doc.spreads.get_mut(si)
                        {
                            let sp = std::sync::Arc::make_mut(sp);
                            if let Some(pg) = sp.pages.get_mut(pi) {
                                pg.margins = doc_margins;
                                pg.local_margins = true;
                            }
                            sp.relayout();
                        }
                        Ok(serde_json::Value::Null)
                    });

                    let (en_margin, en_spread_idx) = {
                        if let Some(active) = app.session.active()
                            && let Some((si, pi)) = active.doc.page_loc(en_page_idx)
                            && let Some(spread) = active.doc.spreads.get(si)
                            && let Some(page) = spread.pages.get(pi)
                        {
                            (page.margin_rect(), si)
                        } else {
                            (margin, spread_idx)
                        }
                    };

                    let en_header_h = 44.0;
                    if !en_title_line.is_empty()
                        && let Some(sid) = frame_with_font_and_style(
                            app,
                            en_spread_idx,
                            [en_margin.x0, en_margin.y0, en_margin.x1, en_margin.y0 + 40.0],
                            &en_title_line.to_uppercase(),
                            "Arial",
                            12.0,
                            Some(&st.toc_style),
                        )
                    {
                        let _ = app.run("text.select", json!({"story": sid, "anchor": 0, "focus": en_title_line.chars().count() as u64}));
                        let _ = app.run("type.alignCenter", json!({}));
                    }

                    let (mw, mh) = (en_margin.x1 - en_margin.x0, en_margin.y1 - en_margin.y0);
                    // Place image1 before English RTF text so textWrap: "boundingBox" wraps body text around it.
                    let img1_rect = [en_margin.x0, en_margin.y0 + mh * 0.254, en_margin.x0 + mw * 0.335, en_margin.y0 + mh * 0.528];
                    if place_catalog_image(app, st, row, "image1", en_spread_idx, img1_rect, "boundingBox").is_some() {
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }

                    crate::canvas::go_to_page(app, en_page_idx);
                    let rtf_y0 = en_margin.y0 + en_header_h;
                    let rtf_rect = [en_margin.x0, rtf_y0, en_margin.x1, en_margin.y1];
                    let created_fid = app
                        .run(
                            "frame.create",
                            json!({
                                "spread": en_spread_idx,
                                "rect": rtf_rect,
                                "content": "text",
                                "caret": false
                            }),
                        )
                        .ok()
                        .and_then(|res| res.get("id").and_then(Value::as_u64));

                    let mut placed_en_rtf_fid: Option<u64> = None;
                    if let Some(fid) = created_fid {
                        let _ = app.run("selection.set", json!({"ids": [fid]}));
                        let mut st_copy = st.clone();
                        let rtf_path_str = en_rtf_file.to_string_lossy().to_string();
                        if execute_page_from_rtf(app, &mut st_copy, &rtf_path_str).is_ok() {
                            inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                            placed_en_rtf_fid = Some(fid);
                        }
                    }
                    let end_page = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(en_page_idx);
                    italicize_page_stories(app, en_page_idx, end_page);

                    // Dynamically anchor image2 (signature) around the signatory name line in the English Preface
                    let default_img2_rect =
                        [en_margin.x1 - mw * 0.244, en_margin.y0 + mh * 0.354, en_margin.x1 - mw * 0.044, en_margin.y0 + mh * 0.464];
                    let (target_spread, img2_rect) = placed_en_rtf_fid
                        .map(|fid| compute_signature_placement(app, fid, default_img2_rect, en_spread_idx, en_margin))
                        .unwrap_or((en_spread_idx, default_img2_rect));
                    if place_catalog_image(app, st, row, "image2", target_spread, img2_rect, "none").is_some() {
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }

                    generated_count = generated_count.saturating_add(1);
                }
            }
        } else if is_div {
            let (first_word, digit_num, rest_text) = excel::split_divider_title(title_val);
            let final_num = if !digit_num.is_empty() { digit_num } else { num_val.to_string() };
            let prefix = if first_word.is_empty() || first_word.chars().all(|c| c.is_ascii_digit()) {
                "BAB".to_string()
            } else if !rest_text.is_empty() {
                first_word.clone()
            } else {
                "BAB".to_string()
            };
            let mut chapter_title = if !rest_text.is_empty() {
                rest_text
            } else if !first_word.eq_ignore_ascii_case("bab") && !first_word.chars().all(|c| c.is_ascii_digit()) {
                first_word
            } else {
                title_val.to_string()
            };

            let title_en = if st.bilingual_output {
                if !title_en_val.is_empty() {
                    title_en_val.to_string()
                } else if let Some((id_t, en_t)) = locale::split_pair(&chapter_title) {
                    let (id_s, en_s) = (id_t.to_string(), en_t.to_string());
                    chapter_title = id_s;
                    en_s
                } else {
                    String::new()
                }
            } else {
                if let Some((id_t, en_t)) = locale::split_pair(&chapter_title) {
                    let (id_s, en_s) = (id_t.to_string(), en_t.to_string());
                    chapter_title = match st.locale {
                        locale::Locale::Id => id_s,
                        locale::Locale::En => en_s,
                    };
                }
                String::new()
            };

            // 1. DIV background / hero image adjusts to full Page Size with fitContentToFrame
            if let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                let resolved_img = resolve_catalog_related_file(&st.card1_path, file_str, "png")
                    .or_else(|| resolve_catalog_related_file(&st.card1_path, file_str, "jpg"))
                    .or_else(|| resolve_catalog_related_file(&st.card1_path, file_str, "jpeg"));

                if let Some(img_file) = resolved_img {
                    let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                    crate::canvas::go_to_page(app, cur_page_idx);
                    let img_rect = [page_bounds.x0, page_bounds.y0, page_bounds.x1, page_bounds.y1];
                    let created_fid = app
                        .run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": img_rect,
                                "content": "graphic",
                                "caret": false
                            }),
                        )
                        .ok()
                        .and_then(|res| res.get("id").and_then(Value::as_u64));

                    if let Some(fid) = created_fid {
                        let img_path_str = img_file.to_string_lossy().to_string();
                        let _ = app.run("file.place", json!({"path": img_path_str, "frame": fid}));
                        let _ = app.run("object.fit", json!({"mode": "fitContentToFrame", "ids": [fid]}));
                        let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [fid]}));
                        let _ = app.run("object.fittingOptions", json!({"ids": [fid], "fitting": "fitContentToFrame"}));
                        let _ = app.run("selection.set", json!({"ids": [fid]}));
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }
                }
            }

            // 2. Textboxes drawn on top of divider page
            if !prefix.is_empty() {
                frame_with_font_and_style(
                    app,
                    spread_idx,
                    [page_bounds.x0 + 36.0, page_bounds.y0 + 36.0, page_bounds.x1 - 100.0, page_bounds.y0 + 72.0],
                    &prefix,
                    "Arial Black",
                    24.0,
                    None,
                );
            }

            if !final_num.is_empty() {
                frame_with_font_and_style(
                    app,
                    spread_idx,
                    [page_bounds.x1 - 90.0, page_bounds.y0 + 36.0, page_bounds.x1 - 36.0, page_bounds.y0 + 96.0],
                    &final_num,
                    "Arial Black",
                    40.0,
                    None,
                );
            }

            if !chapter_title.is_empty() {
                frame_with_font_and_style(
                    app,
                    spread_idx,
                    [page_bounds.x0 + 36.0, page_bounds.y0 + 78.0, page_bounds.x1 - 100.0, page_bounds.y0 + 118.0],
                    &chapter_title,
                    "Arial Black",
                    20.0,
                    Some(&st.toc_style),
                );
            }

            if st.bilingual_output && !title_en.is_empty() {
                frame_with_font_and_style(
                    app,
                    spread_idx,
                    [page_bounds.x0 + 36.0, page_bounds.y0 + 120.0, page_bounds.x1 - 100.0, page_bounds.y0 + 150.0],
                    &title_en,
                    "Arial",
                    16.0,
                    None,
                );
            }
        } else if is_chart_type(type_val) {
            // Chart / Grafik layout:
            let en_title_text = if !title_en_val.is_empty() {
                title_en_val
            } else if clean_en_title != clean_id_title {
                clean_en_title
            } else {
                ""
            };
            let has_dual_title_en = st.bilingual_output && !en_title_text.is_empty() && (!title_val.contains(" / ") || !title_en_val.is_empty());
            let title_h = if has_dual_title_en { 60.0 } else { 48.0 };

            if has_dual_title_en {
                let (ch_line, title_line) = split_chart_title(clean_id_title, num_val, type_val);
                let full_text = if title_line.is_empty() {
                    format!("{ch_line}{}{en_title_text}", designcraft_doc::story::FORCED_LINE_BREAK)
                } else {
                    format!(
                        "{ch_line}{}{title_line}{}{en_title_text}",
                        designcraft_doc::story::FORCED_LINE_BREAK,
                        designcraft_doc::story::FORCED_LINE_BREAK
                    )
                };

                let title_rect = [margin.x0, margin.y0, margin.x1, margin.y0 + title_h];
                if let Some(story) = frame_with_centered_bold_title(app, spread_idx, title_rect, &full_text, Some(&st.toc_style)) {
                    let b_start = full_text.len().saturating_sub(en_title_text.len());
                    let b_end = full_text.len();
                    let _ = app.run(
                        "text.select",
                        json!({
                            "story": story,
                            "anchor": b_start,
                            "focus": b_end,
                        }),
                    );
                    let _ = app.run(
                        "type.char",
                        json!({
                            "fontFamily": "Arial",
                            "fontStyle": "Italic",
                            "size": 9.0,
                        }),
                    );
                }
            } else if st.bilingual_output && title_val.contains(" / ") {
                let parts = locale::split_chart_title_parts(title_val);
                let line1 = format!(
                    "{} / {}{}",
                    parts.id_kicker,
                    parts.en_kicker.as_deref().unwrap_or(""),
                    if parts.number.is_empty() { String::new() } else { format!(" {}", parts.number) }
                );
                let line2 = format!("{} / {}{}", parts.id_subtitle, parts.en_subtitle.as_deref().unwrap_or(""), parts.suffix);
                let full_text = format!("{line1}{}{line2}", designcraft_doc::story::FORCED_LINE_BREAK);
                let title_rect = [margin.x0, margin.y0, margin.x1, margin.y0 + title_h];
                if let Some(story) = frame_with_centered_bold_title(app, spread_idx, title_rect, &full_text, Some(&st.toc_style)) {
                    let chart_start = parts.id_kicker.chars().count() + 3;
                    let chart_end = chart_start + parts.en_kicker.as_deref().unwrap_or("").chars().count();

                    let line2_start = line1.chars().count() + 1;
                    let sub_start = line2_start + parts.id_subtitle.chars().count() + 3;
                    let sub_end = sub_start + parts.en_subtitle.as_deref().unwrap_or("").chars().count();

                    let mut ranges = Vec::new();
                    if chart_start < chart_end {
                        ranges.push((chart_start, chart_end));
                    }
                    if sub_start < sub_end {
                        ranges.push((sub_start, sub_end));
                    }
                    locale::italicize_ranges(app, story, &full_text, &ranges);
                }
            } else {
                let (ch_line, title_line) = split_chart_title(title_val, num_val, type_val);
                let full_text =
                    if title_line.is_empty() { ch_line } else { format!("{ch_line}{}{title_line}", designcraft_doc::story::FORCED_LINE_BREAK) };

                let title_rect = [margin.x0, margin.y0, margin.x1, margin.y0 + title_h];
                frame_with_centered_bold_title(app, spread_idx, title_rect, &full_text, Some(&st.toc_style));
            }

            // 2. Place image from 'file' column inside margins below the title
            if let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                let resolved_img = resolve_catalog_related_file(&st.card1_path, file_str, "png")
                    .or_else(|| resolve_catalog_related_file(&st.card1_path, file_str, "jpg"))
                    .or_else(|| resolve_catalog_related_file(&st.card1_path, file_str, "jpeg"));

                if let Some(img_file) = resolved_img {
                    let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                    crate::canvas::go_to_page(app, cur_page_idx);
                    let img_y0 = margin.y0 + title_h + 8.0;
                    let img_rect = [margin.x0, img_y0, margin.x1, margin.y1];
                    let created_fid = app
                        .run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": img_rect,
                                "content": "graphic",
                                "caret": false
                            }),
                        )
                        .ok()
                        .and_then(|res| res.get("id").and_then(Value::as_u64));

                    if let Some(fid) = created_fid {
                        let img_path_str = img_file.to_string_lossy().to_string();
                        let _ = app.run("file.place", json!({"path": img_path_str, "frame": fid}));
                        let _ = app.run("object.fit", json!({"mode": "fitProportionally", "ids": [fid]}));
                        let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [fid]}));
                        let _ = app.run("object.fit", json!({"mode": "fitFrameToContent", "ids": [fid]}));
                        let _ = app.run("selection.set", json!({"ids": [fid]}));
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }
                }
            }
        } else if type_val.eq_ignore_ascii_case("Note") || type_val.eq_ignore_ascii_case("G") {
            // Publication note (A02) or narrative RTF sheet (G):
            // When title is provided, add centred Arial title box header and place RTF below it.
            // When sheet only had 1 row (file) with no title, don't add Page Header, just add RTF to page.
            let has_header = !title_val.trim().is_empty();
            if has_header {
                let title_rect = [margin.x0, margin.y0, margin.x1, margin.y0 + 36.0];
                // Tag the title with the narrative style so the paragraph table of contents names
                // the page, as the template flow does for its title slot.
                frame_with_centered_arial(app, spread_idx, title_rect, title_val.trim(), Some(TOC_NARRATIVE_STYLE));
            }

            if let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty())
                && let Some(rtf_file) = resolve_catalog_related_file(&st.card1_path, file_str, "rtf")
            {
                let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                crate::canvas::go_to_page(app, cur_page_idx);
                let rtf_rect =
                    if has_header { [margin.x0, margin.y0 + 44.0, margin.x1, margin.y1] } else { [margin.x0, margin.y0, margin.x1, margin.y1] };
                let created_fid = app
                    .run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": rtf_rect,
                            "content": "text",
                            "caret": false
                        }),
                    )
                    .ok()
                    .and_then(|res| res.get("id").and_then(Value::as_u64));

                if let Some(fid) = created_fid {
                    let _ = app.run("selection.set", json!({"ids": [fid]}));
                    let mut st_copy = st.clone();
                    let rtf_path_str = rtf_file.to_string_lossy().to_string();
                    if execute_page_from_rtf(app, &mut st_copy, &rtf_path_str).is_ok() {
                        inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                        // No title line on the page, so the page's title is the RTF's own
                        // heading: tag that paragraph for the paragraph table of contents.
                        if !has_header && let Some(sid) = story_of_frame(app, fid) {
                            tag_page_title(app, sid, TOC_NARRATIVE_STYLE);
                        }
                    }
                }
            }
        } else {
            // 3. Compute Frame Geometry per Preset
            match st.card1_preset {
                1 => {
                    // 1 Row · 3 Equal Columns (1/3 width each)
                    let gutter = 12.0;
                    let total_w = (margin.x1 - margin.x0).max(120.0);
                    let avail = (total_w - 2.0 * gutter).max(60.0);
                    let col_w = avail / 3.0;

                    let c1_x0 = margin.x0;
                    let c1_x1 = c1_x0 + col_w;

                    let c2_x0 = c1_x1 + gutter;
                    let c2_x1 = c2_x0 + col_w;

                    let c3_x0 = c2_x1 + gutter;
                    let c3_x1 = margin.x1;

                    let row_y0 = margin.y0;
                    let row_y1 = margin.y0 + 60.0;

                    let is_table = type_val.eq_ignore_ascii_case("tabel")
                        || type_val.eq_ignore_ascii_case("table")
                        || excel::is_table_sheet_name(file_val)
                        || title_val.contains(" / ");

                    let is_bilingual_table = is_table && (st.bilingual_output || title_val.contains(" / ") || !title_en_val.is_empty());

                    if !type_val.is_empty() || is_table {
                        let (formatted_type, italic_second) = if is_bilingual_table {
                            (format!("Tabel{}Table", designcraft_doc::story::FORCED_LINE_BREAK), true)
                        } else if is_table {
                            (if st.locale == locale::Locale::En { "Table".to_string() } else { "Tabel".to_string() }, false)
                        } else {
                            (type_val.to_string(), false)
                        };

                        if let Ok(res) = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [c1_x0, row_y0, c1_x1, row_y1],
                                "content": "text",
                                "text": formatted_type,
                                "caret": false
                            }),
                        ) && italic_second
                            && let Some(story) = res.get("story").and_then(Value::as_u64)
                        {
                            let (b_start_1, b_end_1) = locale::char_to_byte_range(&formatted_type, 0, 5);
                            let _ = app.run("text.select", json!({"story": story, "anchor": b_start_1, "focus": b_end_1}));
                            let _ = app.run("type.char", json!({"attrs": {"fontStyle": "Regular"}}));

                            locale::italicize_ranges_style(app, story, &formatted_type, &[(6, 11)], "Italic");
                        }
                    }

                    let formatted_num = if is_table { excel::format_table_number(num_val, file_val) } else { num_val.to_string() };

                    if !formatted_num.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [c2_x0, row_y0, c2_x1, row_y1],
                                "content": "text",
                                "text": formatted_num,
                                "caret": false
                            }),
                        );
                    }

                    if !title_val.is_empty() {
                        let (id_t, en_t_opt) =
                            excel::split_and_harmonize_table_title(title_val, if title_en_val.is_empty() { None } else { Some(title_en_val) });

                        if let Some(en_t) = en_t_opt {
                            let full_title = format!("{id_t}{}{en_t}", designcraft_doc::story::FORCED_LINE_BREAK);
                            if let Some(story) = frame_with_style(app, spread_idx, [c3_x0, row_y0, c3_x1, row_y1], &full_title, &st.toc_style) {
                                let (b_start_1, b_end_1) = locale::char_to_byte_range(&full_title, 0, id_t.chars().count());
                                let _ = app.run("text.select", json!({"story": story, "anchor": b_start_1, "focus": b_end_1}));
                                let _ = app.run("type.char", json!({"attrs": {"fontStyle": "Regular"}}));

                                let en_start = id_t.chars().count() + 1;
                                let en_end = en_start + en_t.chars().count();
                                locale::italicize_ranges_style(app, story, &full_title, &[(en_start, en_end)], "Italic");
                            }
                        } else {
                            frame_with_style(app, spread_idx, [c3_x0, row_y0, c3_x1, row_y1], &id_t, &st.toc_style);
                        }
                    }
                }
                2 => {
                    // Editorial Chapter (Stacked)
                    if !type_val.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [margin.x0, margin.y0, margin.x1, margin.y0 + 22.0],
                                "content": "text",
                                "text": type_val.to_uppercase(),
                                "caret": false
                            }),
                        );
                    }
                    if !num_val.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [margin.x0, margin.y0 + 26.0, margin.x1, margin.y0 + 86.0],
                                "content": "text",
                                "text": num_val,
                                "caret": false
                            }),
                        );
                    }
                    if !title_val.is_empty() {
                        frame_with_style(app, spread_idx, [margin.x0, margin.y0 + 92.0, margin.x1, margin.y0 + 200.0], title_val, &st.toc_style);
                    }
                }
                3 => {
                    // Technical Spec Card
                    // Accent banner (graphic rectangle)
                    let _ = app.run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": [margin.x0, margin.y0, margin.x1, margin.y0 + 6.0],
                            "content": "unassigned"
                        }),
                    );
                    // Numeral box at top right
                    if !num_val.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [margin.x1 - 80.0, margin.y0 + 16.0, margin.x1, margin.y0 + 56.0],
                                "content": "text",
                                "text": num_val,
                                "caret": false
                            }),
                        );
                    }
                    // Kicker
                    if !type_val.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [margin.x0, margin.y0 + 16.0, margin.x1 - 90.0, margin.y0 + 36.0],
                                "content": "text",
                                "text": type_val.to_uppercase(),
                                "caret": false
                            }),
                        );
                    }
                    // Title
                    if !title_val.is_empty() {
                        frame_with_style(
                            app,
                            spread_idx,
                            [margin.x0, margin.y0 + 40.0, margin.x1 - 90.0, margin.y0 + 110.0],
                            title_val,
                            &st.toc_style,
                        );
                    }
                }
                4 => {
                    // Minimal Zine
                    if !num_val.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [margin.x0, margin.y0 + 60.0, margin.x1, margin.y0 + 110.0],
                                "content": "text",
                                "text": num_val,
                                "caret": false
                            }),
                        );
                    }
                    if !title_val.is_empty() {
                        frame_with_style(app, spread_idx, [margin.x0, margin.y0 + 120.0, margin.x1, margin.y0 + 200.0], title_val, &st.toc_style);
                    }
                }
                5 => {
                    // TOC Line: one combined paragraph per record (type + number + title), so each record
                    // becomes a single table-of-contents entry.
                    let parts: Vec<&str> = [type_val, num_val, title_val].into_iter().filter(|s| !s.is_empty()).collect();
                    let line = parts.join(" ");
                    if !line.is_empty() {
                        frame_with_style(app, spread_idx, [margin.x0, margin.y0, margin.x1, margin.y0 + 40.0], &line, &st.toc_style);
                    }
                }
                _ => {
                    // 0 or Default: 1 Row · 3 Columns (Col 1 & 2 fit tightly to content, Col 3 takes rest of width with space at end)
                    let gutter = 8.0;
                    let end_space = 20.0;
                    let row_y0 = margin.y0;
                    let row_y1 = margin.y0 + 60.0;
                    let mut cur_x = margin.x0;

                    let is_table = type_val.eq_ignore_ascii_case("tabel")
                        || type_val.eq_ignore_ascii_case("table")
                        || excel::is_table_sheet_name(file_val)
                        || title_val.contains(" / ");

                    let is_bilingual_table = is_table && (st.bilingual_output || title_val.contains(" / ") || !title_en_val.is_empty());

                    // Col 1: Type (Fit tightly to content, starting flush with margin.x0)
                    if !type_val.is_empty() || is_table {
                        let (formatted_type, italic_second) = if is_bilingual_table {
                            (format!("Tabel{}Table", designcraft_doc::story::FORCED_LINE_BREAK), true)
                        } else if is_table {
                            (if st.locale == locale::Locale::En { "Table".to_string() } else { "Tabel".to_string() }, false)
                        } else {
                            (type_val.to_string(), false)
                        };
                        let c1_w = estimate_text_width(if italic_second { "Table" } else { &formatted_type }, false);
                        let c1_x0 = cur_x;
                        let c1_x1 = (c1_x0 + c1_w).min(margin.x1 - 100.0);
                        cur_x = c1_x1 + gutter;

                        if let Ok(res) = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [c1_x0, row_y0, c1_x1, row_y1],
                                "content": "text",
                                "text": formatted_type,
                                "caret": false
                            }),
                        ) {
                            if let Some(id) = res.get("id").and_then(Value::as_u64) {
                                let _ = app.run("selection.set", json!({"ids": [id]}));
                            }
                            let _ = app.run("type.alignRight", json!({}));
                            if italic_second && let Some(story) = res.get("story").and_then(Value::as_u64) {
                                let (b_start_1, b_end_1) = locale::char_to_byte_range(&formatted_type, 0, 5);
                                let _ = app.run("text.select", json!({"story": story, "anchor": b_start_1, "focus": b_end_1}));
                                let _ = app.run("type.char", json!({"attrs": {"fontStyle": "Regular"}}));

                                locale::italicize_ranges_style(app, story, &formatted_type, &[(6, 11)], "Italic");
                            }
                        }
                    }

                    // Col 2: Number (Fit tightly to content)
                    let formatted_num = if is_table { excel::format_table_number(num_val, file_val) } else { num_val.to_string() };

                    if !formatted_num.is_empty() {
                        let c2_w = estimate_text_width(&formatted_num, false);
                        let c2_x0 = cur_x;
                        let c2_x1 = (c2_x0 + c2_w).min(margin.x1 - 60.0);
                        cur_x = c2_x1 + gutter;

                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [c2_x0, row_y0, c2_x1, row_y1],
                                "content": "text",
                                "text": formatted_num,
                                "caret": false
                            }),
                        );
                    }

                    // Col 3: Title (Takes rest of width with space in the end)
                    if !title_val.is_empty() {
                        let c3_x0 = cur_x;
                        let max_x1 = (margin.x1 - end_space).max(c3_x0 + 40.0);
                        let c3_x1 = max_x1.min(margin.x1);

                        let (id_t, en_t_opt) =
                            excel::split_and_harmonize_table_title(title_val, if title_en_val.is_empty() { None } else { Some(title_en_val) });

                        if let Some(en_t) = en_t_opt {
                            let full_title = format!("{id_t}{}{en_t}", designcraft_doc::story::FORCED_LINE_BREAK);
                            if let Some(story) = frame_with_style(app, spread_idx, [c3_x0, row_y0, c3_x1, row_y1], &full_title, &st.toc_style) {
                                let (b_start_1, b_end_1) = locale::char_to_byte_range(&full_title, 0, id_t.chars().count());
                                let _ = app.run("text.select", json!({"story": story, "anchor": b_start_1, "focus": b_end_1}));
                                let _ = app.run("type.char", json!({"attrs": {"fontStyle": "Regular"}}));

                                let en_start = id_t.chars().count() + 1;
                                let en_end = en_start + en_t.chars().count();
                                locale::italicize_ranges_style(app, story, &full_title, &[(en_start, en_end)], "Italic");
                            }
                        } else {
                            frame_with_style(app, spread_idx, [c3_x0, row_y0, c3_x1, row_y1], &id_t, &st.toc_style);
                        }
                    }
                }
            }
            // Auto-insert table from 'file' column if enabled
            if st.card1_auto_insert_tables
                && let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty())
            {
                let catalog_p = std::path::Path::new(&st.card1_path);
                let dir = catalog_p.parent().unwrap_or(std::path::Path::new("."));
                let candidate1 = dir.join(file_str);
                let candidate2 = dir.join(format!("{file_str}.csv"));
                let candidate3 = dir.join(format!("{}.csv", excel::sanitize_filename(file_str)));
                let resolved = if candidate1.exists() && candidate1.is_file() {
                    Some(candidate1)
                } else if candidate2.exists() && candidate2.is_file() {
                    Some(candidate2)
                } else if candidate3.exists() && candidate3.is_file() {
                    Some(candidate3)
                } else {
                    None
                };

                if let Some(tbl_file) = resolved
                    && let Ok(content) = std::fs::read_to_string(&tbl_file)
                {
                    let delim = detect_delimiter(&content);
                    let tbl_matrix = parse_csv_content_with_delim(&content, delim);
                    if !tbl_matrix.is_empty() {
                        let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                        if execute_insert_table_matrix(app, st, Some(&tbl_matrix), Some(cur_page_idx)).is_ok() {
                            inserted_tables_count = inserted_tables_count.saturating_add(1);
                        }
                    }
                }
            }
        }
        generated_count = generated_count.saturating_add(1);
    }

    let mut details = Vec::new();
    if inserted_rtfs_count > 0 {
        details.push(format!("{inserted_rtfs_count} RTF chapter(s) placed"));
    }
    if inserted_images_count > 0 {
        details.push(format!("{inserted_images_count} chart image(s) placed"));
    }
    if inserted_tables_count > 0 {
        details.push(format!("{inserted_tables_count} tables auto-inserted"));
    }
    let tbl_msg = if !details.is_empty() { format!(" (with {})", details.join(", ")) } else { String::new() };
    Ok(format!("Successfully generated {generated_count} pages{tbl_msg}!"))
}

/// Target page index (0-based absolute index):
/// Priority: Canvas current view page (e.g. when selected in Pages tab) -> Selected item spread -> Last document page.
pub fn target_page_index(app: &DesignApp) -> usize {
    if let Some(cur) = crate::canvas::current_page(app) {
        return cur;
    }
    if let Some(st) = app.session.active() {
        let sel_id = st.selection.items.first().copied().or_else(|| st.selection.text.and_then(|t| t.frame));
        if let Some(id) = sel_id {
            if let Some(p) = st.doc.page_of_item(id) {
                return p;
            }
            if let Some(loc) = st.doc.find(id)
                && let designcraft_doc::SpreadRef::Doc(si) = loc.spread
            {
                return st.doc.first_page_of_spread(si);
            }
        }
        st.doc.page_count().saturating_sub(1)
    } else {
        0
    }
}

/// Create the paragraph style `name` if the document does not already have it, so `toc.generate`
/// (which requires an existing style) can collect the "TOC Line" paragraphs.
fn ensure_para_style(app: &mut DesignApp, name: &str) {
    let exists = app
        .session
        .execute("style.list", &json!({}))
        .ok()
        .and_then(|v| v.get("paragraph").and_then(Value::as_array).map(|a| a.iter().any(|s| s.as_str() == Some(name))))
        .unwrap_or(false);
    if !exists {
        let _ = app.run("style.paragraph.create", json!({"name": name}));
    }
}

/// How many of a page's opening paragraphs the title search weighs.
const TITLE_SCAN: usize = 12;

/// The paragraph that reads as the page's title: the largest centred line among the page's opening
/// paragraphs (the first one on a tie).
///
/// A `Note`/`G` sheet gives no title, so the page opens with the RTF's own heading. Word writes
/// that heading as a `Heading n`, as the `Pendahuluan` style or as direct formatting (bold,
/// capitals, coloured, centred) from sheet to sheet, so the search goes by how it looks rather
/// than by its name. On the compilers' page that leaves out the ISSN line above it (right-aligned)
/// and the publication's title below it (set smaller), and takes "TIM PENYUSUN/COMPILERS".
fn page_title_index(doc: &designcraft_doc::Document, sid: designcraft_doc::StoryId) -> Option<usize> {
    let story = doc.story(sid)?;
    let mut seen = 0;
    let mut sized: Vec<(usize, f64, bool)> = Vec::new(); // paragraph, text size, centred
    for (i, r) in story.para_ranges().iter().enumerate() {
        if !story.text.get(r.clone()).is_some_and(|t| !t.trim().is_empty()) {
            continue;
        }
        seen += 1;
        if seen > TITLE_SCAN {
            break;
        }
        let mut size: Option<f64> = None;
        for (run, format) in story.runs() {
            if run.start >= r.end {
                break;
            }
            if run.end <= r.start {
                continue;
            }
            if let Some(s) = format.over.size {
                size = Some(size.map_or(s, |m: f64| m.max(s)));
            }
        }
        let size = size.or_else(|| story.paras.get(i).and_then(|p| doc.styles.para(&p.style)).and_then(|s| s.chars.size)).unwrap_or(0.0);
        let centred = story.paras.get(i).is_some_and(|p| p.para.align == Some(designcraft_doc::Align::Center));
        sized.push((i, size, centred));
    }
    let largest = |centred_only: bool| {
        sized
            .iter()
            .filter(|(_, _, centred)| !centred_only || *centred)
            .fold(None::<(usize, f64)>, |best, &(i, size, _)| match best {
                Some((_, b)) if b >= size => best,
                _ => Some((i, size)),
            })
            .map(|(i, _)| i)
    };
    largest(true).or_else(|| largest(false))
}

/// Tag the paragraph that reads as a title-less `Note`/`G` page's title with `style`, so the
/// paragraph table of contents names the page. Only the style changes: the RTF's own centring,
/// boldness and size stay on the paragraph.
fn tag_page_title(app: &mut DesignApp, sid: designcraft_doc::StoryId, style: &str) {
    ensure_para_style(app, style);
    let _ = app.session.edit(|doc, _| {
        if let Some(i) = page_title_index(doc, sid)
            && let Some(story) = doc.story_mut(sid)
            && let Some(r) = story.para_ranges().get(i).cloned()
        {
            story.format_paras(r.start..r.start, |f| f.style = style.to_string());
        }
        Ok(serde_json::Value::Null)
    });
}

/// The story of the text frame `fid`, as the document has it now.
fn story_of_frame(app: &DesignApp, fid: u64) -> Option<designcraft_doc::StoryId> {
    let active = app.session.active()?;
    let id = designcraft_doc::ItemId(fid);
    let item = active.doc.spreads.iter().flat_map(|s| s.items.iter()).find(|i| i.id == id)?;
    item.text_frame().map(|tf| tf.story)
}

/// Create a text frame and tag its paragraph with the TOC paragraph style, so `toc.generate`
/// lists the frame's text as an entry (one entry per generated page).
fn frame_with_style(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], text: &str, style: &str) -> Option<u64> {
    let res = app.run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "text", "text": text, "caret": false})).ok()?;
    let story = res.get("story").and_then(Value::as_u64)?;
    let end = text.len().max(1) as u64;
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    let _ = app.run("style.paragraph.apply", json!({"name": style}));
    Some(story)
}

/// Create a text frame with specific font family, optional font style, font size, and optional paragraph style.
fn frame_with_font_full(
    app: &mut DesignApp,
    spread_idx: usize,
    rect: [f64; 4],
    text: &str,
    font_family: &str,
    font_style: Option<&str>,
    font_size: f64,
    style_opt: Option<&str>,
) -> Option<u64> {
    let res = app.run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "text", "text": text, "caret": false})).ok()?;
    let story = res.get("story").and_then(Value::as_u64)?;
    let end = text.len().max(1) as u64;
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    if let Some(style) = style_opt {
        let _ = app.run("style.paragraph.apply", json!({"name": style}));
    }
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    let mut type_char_params = json!({"fontFamily": font_family, "size": font_size});
    if let Some(fs) = font_style {
        type_char_params["fontStyle"] = json!(fs);
    }
    let _ = app.run("type.char", type_char_params);
    Some(story)
}

/// Create a text frame with specific font family, font size, and optional paragraph style.
fn frame_with_font_and_style(
    app: &mut DesignApp,
    spread_idx: usize,
    rect: [f64; 4],
    text: &str,
    font_family: &str,
    font_size: f64,
    style_opt: Option<&str>,
) -> Option<u64> {
    frame_with_font_full(app, spread_idx, rect, text, font_family, None, font_size, style_opt)
}

/// Check if a story is overset.
fn story_is_overset(app: &DesignApp, sid: designcraft_doc::StoryId) -> bool {
    if let Some(active) = app.session.active() {
        designcraft_compose::compose_story(&active.doc, sid, &Default::default()).overset_at.is_some()
    } else {
        false
    }
}

/// Set all character runs in the given story to Italic (or Bold Italic if bold).
fn italicize_story(app: &mut DesignApp, story_id: u64) {
    let _ = app.session.edit(|doc, _| {
        if let Some(story) = doc.story_mut(designcraft_doc::StoryId(story_id)) {
            for run in &mut story.chars {
                let cur = run.format.over.font_style.as_deref().unwrap_or("Regular");
                let next = if cur.eq_ignore_ascii_case("bold") || cur.eq_ignore_ascii_case("bold italic") { "Bold Italic" } else { "Italic" };
                run.format.over.font_style = Some(next.to_string());
            }
        }
        Ok(serde_json::Value::Null)
    });
}

/// Set all character runs in all stories belonging to the given page range to Italic (or Bold Italic if bold).
fn italicize_page_stories(app: &mut DesignApp, start_page: usize, end_page: usize) {
    let _ = app.session.edit(|doc, _| {
        for pi in start_page..=end_page {
            if let Some((si, _)) = doc.page_loc(pi)
                && let Some(spread) = doc.spreads.get(si)
            {
                let story_ids: Vec<_> = spread
                    .items
                    .iter()
                    .filter(|it| doc.page_of_item(it.id) == Some(pi))
                    .filter_map(|it| it.text_frame())
                    .map(|tf| tf.story)
                    .collect();
                for sid in story_ids {
                    if let Some(story) = doc.story_mut(sid) {
                        for run in &mut story.chars {
                            let cur = run.format.over.font_style.as_deref().unwrap_or("Regular");
                            let next =
                                if cur.eq_ignore_ascii_case("bold") || cur.eq_ignore_ascii_case("bold italic") { "Bold Italic" } else { "Italic" };
                            run.format.over.font_style = Some(next.to_string());
                        }
                    }
                }
            }
        }
        Ok(serde_json::Value::Null)
    });
}

/// Helper to place a catalog image (e.g. "image1", "image2") on `spread_idx` with `rect` and `wrap`.
fn place_catalog_image(
    app: &mut DesignApp,
    st: &PubgenState,
    row: &[String],
    key: &str,
    spread_idx: usize,
    rect: [f64; 4],
    wrap: &str,
) -> Option<u64> {
    let col = |key: &str| -> Option<String> {
        let idx = st.card1_headers.iter().position(|h| h.trim().eq_ignore_ascii_case(key))?;
        let v = row.get(idx).map(|s| s.trim()).filter(|s| !s.is_empty())?;
        Some(v.to_string())
    };
    let file_str = col(key)?;
    let img = resolve_catalog_related_file(&st.card1_path, &file_str, "png")
        .or_else(|| resolve_catalog_related_file(&st.card1_path, file_str.as_str(), "jpg"))
        .or_else(|| resolve_catalog_related_file(&st.card1_path, &file_str, "jpeg"))?;
    let res = app.run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "graphic"})).ok()?;
    let fid = res.get("id").and_then(Value::as_u64)?;
    let _ = app.run("selection.set", json!({"ids": [fid]}));
    let _ = app.run("file.place", json!({"path": img.to_string_lossy(), "frame": fid}));
    let _ = app.run("object.fit", json!({"mode": "fitProportionally", "ids": [fid]}));
    if wrap != "none" {
        let _ = app.run("object.textWrap", json!({"mode": wrap, "offset": 6.0, "ids": [fid]}));
    }
    Some(fid)
}

/// Computes the dynamic placement rect for the signature (`image2`) anchored to the signatory
/// block at the end of the narrative text (e.g. Kata Pengantar / Preface).
/// Returns `(target_spread_idx, [x0, y0, x1, y1])`.
fn compute_signature_placement(
    app: &DesignApp,
    fid: u64,
    default_rect: [f64; 4],
    default_spread: usize,
    margin: designcraft_geom::Rect,
) -> (usize, [f64; 4]) {
    let Some(sid) = story_of_frame(app, fid) else {
        return (default_spread, default_rect);
    };
    let Some(active) = app.session.active() else {
        return (default_spread, default_rect);
    };
    let cs = designcraft_compose::compose_story(&active.doc, sid, &Default::default());

    // Find the last frame containing non-whitespace text lines.
    let mut found = None;
    for frame in cs.frames.iter().rev() {
        let non_empty: Vec<&designcraft_compose::Line> =
            frame.lines.iter().filter(|l| l.glyphs.iter().any(|g| g.visible && !g.rendered_char.is_whitespace())).collect();
        if !non_empty.is_empty() {
            let last_line = non_empty.last().copied();
            let prev_line = if non_empty.len() >= 2 { non_empty.get(non_empty.len().saturating_sub(2)).copied() } else { None };
            if let Some(ll) = last_line {
                found = Some((frame.frame, ll, prev_line));
                break;
            }
        }
    }

    let Some((target_frame_id, last_line, prev_line)) = found else {
        return (default_spread, default_rect);
    };

    let Some(frame_item) = active.doc.item(target_frame_id) else {
        return (default_spread, default_rect);
    };

    let target_spread_idx = active.doc.spreads.iter().position(|s| s.items.iter().any(|it| it.id == target_frame_id)).unwrap_or(default_spread);

    let last_visible: Vec<_> = last_line.glyphs.iter().filter(|g| g.visible && !g.rendered_char.is_whitespace()).collect();
    if last_visible.is_empty() {
        return (default_spread, default_rect);
    }

    let line_min_x = last_visible.iter().map(|g| g.x).fold(f64::INFINITY, f64::min);
    let line_max_x = last_visible.iter().map(|g| g.x + g.adv).fold(f64::NEG_INFINITY, f64::max);
    let center_x = (line_min_x + line_max_x) / 2.0;

    let spread_center_pt = frame_item.xf * designcraft_geom::Point::new(center_x, last_line.baseline);
    let spread_center_x = spread_center_pt.x;
    let spread_last_baseline_y = spread_center_pt.y;

    let prev_baseline_y = prev_line.map(|pl| (frame_item.xf * designcraft_geom::Point::new(0.0, pl.baseline)).y);

    let mw = margin.x1 - margin.x0;
    let img_w = (mw * 0.22).clamp(80.0, 110.0);

    let (y0, y1) = if let Some(pby) = prev_baseline_y {
        let gap = spread_last_baseline_y - pby;
        if gap > 35.0 {
            let h = (gap * 0.95).clamp(50.0, 75.0);
            let bottom = spread_last_baseline_y - 6.0;
            (bottom - h, bottom)
        } else {
            let h = 65.0;
            let bottom = spread_last_baseline_y - 6.0;
            (bottom - h, bottom)
        }
    } else {
        let h = 65.0;
        let bottom = spread_last_baseline_y - 6.0;
        (bottom - h, bottom)
    };

    let mut x0 = spread_center_x - img_w / 2.0;
    let mut x1 = spread_center_x + img_w / 2.0;

    if x1 > margin.x1 {
        let shift = x1 - margin.x1;
        x0 -= shift;
        x1 -= shift;
    }
    if x0 < margin.x0 {
        let shift = margin.x0 - x0;
        x0 += shift;
        x1 += shift;
    }

    let mut final_y0 = y0;
    let mut final_y1 = y1;
    if final_y1 > margin.y1 {
        let shift = final_y1 - margin.y1;
        final_y0 -= shift;
        final_y1 -= shift;
    }
    if final_y0 < margin.y0 {
        let shift = margin.y0 - final_y0;
        final_y0 += shift;
        final_y1 += shift;
    }

    (target_spread_idx, [x0, final_y0, x1, final_y1])
}

/// Split title like "BAB 01 Pemerintahan" into ("BAB 01", "Pemerintahan")
pub fn split_chapter_title(title: &str, num_val: &str) -> (String, String) {
    split_chapter_title_locale(title, num_val, false)
}

pub fn split_chapter_title_locale(title: &str, num_val: &str, is_en: bool) -> (String, String) {
    let t = title.trim();
    let words: Vec<&str> = t.split_whitespace().collect();
    if words.len() >= 3 && (words[0].eq_ignore_ascii_case("bab") || words[0].eq_ignore_ascii_case("chapter")) {
        let ch_num = words[1].trim_matches(|c: char| c == '.' || c == ':' || c == ';');
        let chapter_line = format!("{} {}", words[0].to_uppercase(), ch_num);
        let title_line = words[2..].join(" ");
        return (chapter_line, title_line);
    }
    if words.len() >= 2 && (words[0].eq_ignore_ascii_case("bab") || words[0].eq_ignore_ascii_case("chapter")) {
        let ch_num = words[1].trim_matches(|c: char| c == '.' || c == ':' || c == ';');
        return (format!("{} {}", words[0].to_uppercase(), ch_num), String::new());
    }

    if !num_val.is_empty() {
        let prefix = if is_en { "CHAPTER" } else { "BAB" };
        let chapter_line = format!("{prefix} {num_val}");
        let clean = t
            .strip_prefix("CHAPTER")
            .or_else(|| t.strip_prefix("chapter"))
            .or_else(|| t.strip_prefix("BAB"))
            .or_else(|| t.strip_prefix("bab"))
            .unwrap_or(t)
            .trim();
        let clean2 = clean.strip_prefix(num_val).unwrap_or(clean).trim();
        let clean3 = clean2.trim_matches(|c: char| c == '.' || c == ':' || c == ';').trim();
        let final_title = if clean3.is_empty() { t.to_string() } else { clean3.to_string() };
        return (chapter_line, final_title);
    }

    (t.to_string(), String::new())
}

/// Create a centered text frame with Arial Bold and optional paragraph style.
fn frame_with_centered_bold_title(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], text: &str, style_opt: Option<&str>) -> Option<u64> {
    let res = app.run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "text", "text": text, "caret": false})).ok()?;
    let fid = res.get("id").and_then(Value::as_u64)?;
    let story = res.get("story").and_then(Value::as_u64)?;
    let end = text.len().max(1) as u64;
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    if let Some(style) = style_opt {
        let _ = app.run("style.paragraph.apply", json!({"name": style}));
    }
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    let _ = app.run("type.char", json!({"fontFamily": "Arial", "fontStyle": "Bold", "size": 11.0}));
    let _ = app.run("selection.set", json!({"ids": [fid]}));
    let _ = app.run("type.alignCenter", json!({}));
    Some(story)
}

/// Centred Arial title in a 100 % transparent text box (no fill, no stroke).
fn frame_with_centered_arial(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], text: &str, style: Option<&str>) -> Option<u64> {
    let res = app.run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "text", "text": text, "caret": false})).ok()?;
    let fid = res.get("id").and_then(Value::as_u64)?;
    let story = res.get("story").and_then(Value::as_u64)?;
    let end = text.len().max(1) as u64;
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    if let Some(style) = style {
        let _ = app.run("style.paragraph.apply", json!({"name": style}));
    }
    let _ = app.run("type.char", json!({"fontFamily": "Arial", "size": 11.0}));
    let _ = app.run("selection.set", json!({"ids": [fid]}));
    let _ = app.run("type.alignCenter", json!({}));
    let _ = app.run("object.fill", json!({"ids": [fid], "swatch": "[None]"}));
    let _ = app.run("object.stroke", json!({"ids": [fid], "swatch": "[None]"}));
    Some(fid)
}

/// Resolve a catalog `file` value beside the catalog CSV: as written, relative to the CSV's
/// folder, the same name with `extension` appended, or just the file name beside the catalog
/// (so paths copied from another machine still resolve).
fn resolve_catalog_related_file(catalog_path: &str, file_str: &str, extension: &str) -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(catalog_path).parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut candidates = vec![dir.join(file_str), std::path::PathBuf::from(file_str), dir.join(format!("{file_str}.{extension}"))];
    if let Some(name) = std::path::Path::new(file_str).file_name() {
        candidates.push(dir.join(name));
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Split chart title like "Grafik 02.01 Panjang Jalan..." into ("Grafik 02.01", "Panjang Jalan...")
pub fn split_chart_title(title: &str, num_val: &str, type_val: &str) -> (String, String) {
    let t = title.trim();
    let words: Vec<&str> = t.split_whitespace().collect();
    let eff_type = if type_val.trim().is_empty() { "Grafik" } else { type_val.trim() };

    if words.len() >= 3 && is_chart_type(words[0]) {
        let ch_num = words[1].trim_matches(|c: char| c == ':' || c == ';');
        let line1 = format!("{} {}", words[0], ch_num);
        let line2 = words[2..].join(" ");
        return (line1, line2);
    }

    if words.len() >= 2 && is_chart_type(words[0]) {
        let ch_num = words[1].trim_matches(|c: char| c == ':' || c == ';');
        return (format!("{} {}", words[0], ch_num), String::new());
    }

    if words.len() >= 2 && words[0].contains('.') && words[0].chars().any(|c| c.is_ascii_digit()) {
        let ch_num = words[0].trim_matches(|c: char| c == ':' || c == ';');
        let line1 = format!("{eff_type} {ch_num}");
        let line2 = words[1..].join(" ");
        return (line1, line2);
    }

    if !num_val.is_empty() {
        let line1 = format!("{eff_type} {num_val}");
        let clean = words
            .iter()
            .copied()
            .filter(|w| !is_chart_type(w) && !w.trim_matches(|c: char| c == ':' || c == ';' || c == '.').eq_ignore_ascii_case(num_val))
            .collect::<Vec<_>>()
            .join(" ");
        let final_title = if clean.is_empty() { t.to_string() } else { clean };
        return (line1, final_title);
    }

    (t.to_string(), String::new())
}

pub fn is_ulasan_type(type_val: &str) -> bool {
    let s = type_val.trim();
    if s.is_empty() {
        return false;
    }
    let lower = s.to_lowercase();
    if lower == "ulasan" || lower == "review" || lower == "editorial" || lower == "chapter" {
        return true;
    }
    if (lower.starts_with('r') || lower.starts_with('b')) && lower[1..].chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    false
}

pub fn is_divider_type(type_val: &str) -> bool {
    let s = type_val.trim();
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

pub fn is_chart_type(type_val: &str) -> bool {
    ["grafik", "graph", "chart", "bagan", "diagram"].iter().any(|k| type_val.eq_ignore_ascii_case(k))
}

/// The number of paragraphs the TOC would collect for `style` right now.
#[cfg(test)]
fn toc_entry_count(app: &mut DesignApp, style: &str) -> usize {
    app.session.execute("toc.entries", &json!({"entries": [{"style": style, "level": 1}]})).ok().and_then(|v| v.as_array().map(Vec::len)).unwrap_or(0)
}

pub fn execute_insert_table(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    execute_insert_table_matrix(app, st, None, None)
}

/// One table cell of the tab-delimited text `table.convertFromText` reads. A paragraph break would
/// split the cell into extra rows and a tab into extra columns, so an Excel line break becomes the
/// story's forced line break (U+2028), which the table cell keeps on multiple lines.
fn table_cell_text(cell: &str) -> String {
    let normalized = cell.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(normalized.len());
    for (i, line) in normalized.split('\n').enumerate() {
        if i > 0 {
            out.push(designcraft_doc::FORCED_LINE_BREAK);
        }
        out.push_str(line.replace('\t', " ").trim());
    }
    out
}

#[allow(clippy::needless_range_loop)]
pub fn execute_insert_table_matrix(
    app: &mut DesignApp,
    st: &PubgenState,
    matrix_override: Option<&[Vec<String>]>,
    target_page_override: Option<usize>,
) -> Result<String, String> {
    let source_matrix = matrix_override.unwrap_or(&st.card2_matrix);
    if source_matrix.is_empty() {
        return Err("No tabular data to insert".into());
    }

    // Detect and extract trailing footnote/source row(s) (where only first col has data and rest of cols are empty)
    let mut footnotes = Vec::new();
    let mut matrix = source_matrix.to_vec();
    let cols_count = matrix.iter().map(Vec::len).max().unwrap_or(0);
    if cols_count > 1 {
        while matrix.len() > 1 {
            let Some(last) = matrix.last() else { break };
            let first = last.first().map(|s| s.trim()).unwrap_or("");
            let rest_empty = last.iter().skip(1).all(|c| c.trim().is_empty());
            if !first.is_empty() && rest_empty {
                footnotes.insert(0, first.to_string());
                matrix.pop();
            } else {
                break;
            }
        }
    }
    let footnote_text = if footnotes.is_empty() { None } else { Some(footnotes.join("\n")) };

    let rows_count = matrix.len();

    // Auto-detect merged header columns (split by "--", capped at max 3 stacks)
    let col_levels: Vec<Vec<String>> = if st.card2_first_row_header {
        matrix
            .first()
            .map(|row| {
                (0..cols_count)
                    .map(|c| {
                        let val = row.get(c).map(|s| s.trim()).unwrap_or("");
                        if val.contains("--") { val.splitn(3, "--").map(|s| s.trim().to_string()).collect() } else { vec![val.to_string()] }
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let merge_count = col_levels.iter().filter(|lvls| lvls.len() > 1).count();
    let num_header_rows = col_levels.iter().map(Vec::len).max().unwrap_or(1).clamp(1, 3);
    let has_merged_headers = st.card2_first_row_header && merge_count > 1 && num_header_rows > 1;

    let total_rows = if has_merged_headers {
        num_header_rows + (rows_count.saturating_sub(1)) + if st.card2_col_numbers { 1 } else { 0 }
    } else if st.card2_col_numbers {
        rows_count + 1
    } else {
        rows_count
    };

    // Format tabular data with tab separators and newlines for convertFromText
    let mut table_text = String::new();

    if has_merged_headers {
        // Build header grid: num_header_rows × cols_count
        let mut header_grid = vec![vec![String::new(); cols_count]; num_header_rows];

        for c in 0..cols_count {
            let lvls = col_levels.get(c).map(|v| v.as_slice()).unwrap_or(&[]);
            let k = lvls.len();
            for r in 0..num_header_rows {
                if r < k.saturating_sub(1) {
                    // Ancestor group level: emit text only at the start of the group
                    let is_start = c == 0 || (0..=r).any(|l| col_levels.get(c.saturating_sub(1)).and_then(|prev| prev.get(l)) != lvls.get(l));
                    if is_start && let Some(txt) = lvls.get(r) {
                        header_grid[r][c] = txt.trim().to_string();
                    }
                } else if r == k.saturating_sub(1) {
                    // Leaf level
                    if let Some(txt) = lvls.get(r) {
                        header_grid[r][c] = txt.trim().to_string();
                    }
                }
                // When r > k.saturating_sub(1), remains empty String
            }
        }

        // Push header rows
        for (r_idx, row) in header_grid.iter().enumerate() {
            if r_idx > 0 {
                table_text.push('\n');
            }
            for (c_idx, cell) in row.iter().enumerate() {
                if c_idx > 0 {
                    table_text.push('\t');
                }
                table_text.push_str(&table_cell_text(cell));
            }
        }

        // If column numbers row is enabled with header, insert immediately after header rows
        if st.card2_col_numbers {
            table_text.push('\n');
            for c_idx in 0..cols_count {
                if c_idx > 0 {
                    table_text.push('\t');
                }
                table_text.push_str(&format!("({})", c_idx + 1));
            }
        }

        // Data rows (skip row 0)
        for row in matrix.iter().skip(1) {
            table_text.push('\n');
            for (c_idx, cell) in row.iter().enumerate() {
                if c_idx > 0 {
                    table_text.push('\t');
                }
                table_text.push_str(&table_cell_text(cell));
            }
        }
    } else {
        // If column numbers row is enabled without header, insert at top
        if st.card2_col_numbers && !st.card2_first_row_header {
            for c_idx in 0..cols_count {
                if c_idx > 0 {
                    table_text.push('\t');
                }
                table_text.push_str(&format!("({})", c_idx + 1));
            }
        }

        for (r_idx, row) in matrix.iter().enumerate() {
            if !table_text.is_empty() {
                table_text.push('\n');
            }
            for (c_idx, cell) in row.iter().enumerate() {
                if c_idx > 0 {
                    table_text.push('\t');
                }
                table_text.push_str(&table_cell_text(cell));
            }
            // If column numbers row is enabled with header, insert immediately after header row (r_idx == 0)
            if st.card2_col_numbers && st.card2_first_row_header && r_idx == 0 {
                table_text.push('\n');
                for c_idx in 0..cols_count {
                    if c_idx > 0 {
                        table_text.push('\t');
                    }
                    table_text.push_str(&format!("({})", c_idx + 1));
                }
            }
        }
    }

    // Step 1: Ensure we have a text target
    let active = app.session.active().ok_or("No active document")?;
    let has_caret = active.selection.text.is_some();

    let cur_page = target_page_override.unwrap_or_else(|| target_page_index(app));
    let effective_placement = if target_page_override.is_some() { 2 } else { st.card2_placement };

    if effective_placement == 0 && !has_caret {
        return Err("No active text insertion point. Select a text frame or choose 'New frame' / 'At page'.".into());
    }

    let (story_id, target_page_info, created_frame_info, fn_target) = match effective_placement {
        0 => {
            // At cursor
            let story = active.selection.text.map(|t| t.story.0).ok_or("No active story")?;
            let mut fn_target = None;
            if let Some(fid) = active.selection.text.and_then(|t| t.frame)
                && let Some(loc) = active.doc.find(fid)
                && let designcraft_doc::SpreadRef::Doc(si) = loc.spread
                && let Some(spread) = active.doc.spreads.get(si)
                && let Some(item) = spread.items.iter().find(|it| it.id == fid)
            {
                let b = item.bounds();
                let fn_y0 = b.y1 + 6.0;
                let fn_y1 = fn_y0 + 24.0;
                fn_target = Some((si, [b.x0, fn_y0, b.x1, fn_y1]));
            }
            (story, None, None, fn_target)
        }
        2 => {
            // At page: place below Generated Page Title on active / selected page
            let (spread_idx, page_in_spread) = active.doc.page_loc(cur_page).unwrap_or((0, 0));
            let spread = active.doc.spreads.get(spread_idx).ok_or("Spread missing")?;
            let page = spread.pages.get(page_in_spread).ok_or("Page missing")?;
            let margin = page.margin_rect();
            let page_r = page.bounds();

            // Find bottom-most edge of generated page title / header items on this page
            let mut title_bottom = margin.y0;
            for it in &spread.items {
                let b = it.bounds();
                // Check if item lies on this page horizontally and in header region vertically
                if b.x0 < page_r.x1 && b.x1 > page_r.x0 && b.y0 < margin.y0 + 260.0 {
                    // Ignore text frames that already contain tables
                    let is_table = it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|st| !st.tables.is_empty());
                    if !is_table && b.y1 > title_bottom {
                        title_bottom = b.y1;
                    }
                }
            }

            let has_footnote = footnote_text.is_some();
            let gap_below_title = 12.0;
            let table_y0 = if title_bottom > margin.y0 { title_bottom + gap_below_title } else { margin.y0 };
            let available_h = (margin.y1 - table_y0).max(60.0);
            let max_table_h = if has_footnote { (available_h - 32.0).max(40.0) } else { available_h };
            let frame_h = max_table_h;

            let res = app.run(
                "frame.create",
                json!({
                    "spread": spread_idx,
                    "rect": [margin.x0, table_y0, margin.x1, table_y0 + frame_h],
                    "content": "text",
                    "caret": true
                }),
            )?;
            let sid = res["story"].as_u64().ok_or("Failed to get story ID")?;
            let fid = res["id"].as_u64().map(designcraft_doc::ItemId);
            let created_info = fid.map(|id| (id, spread_idx, margin.x0, margin.x1, table_y0, max_table_h, margin.y1));
            (sid, Some(cur_page), created_info, None)
        }
        _ => {
            // 1 or default: New frame on active / selected page
            let cur_page = target_page_index(app);
            let (spread_idx, page_in_spread) = active.doc.page_loc(cur_page).unwrap_or((0, 0));
            let spread = active.doc.spreads.get(spread_idx).ok_or("Spread missing")?;
            let page = spread.pages.get(page_in_spread).ok_or("Page missing")?;
            let margin = page.margin_rect();
            let has_footnote = footnote_text.is_some();
            let max_table_h = if has_footnote { (margin.height() - 32.0).max(40.0) } else { margin.height() };
            let frame_h = max_table_h;

            let res = app.run(
                "frame.create",
                json!({
                    "spread": spread_idx,
                    "rect": [margin.x0, margin.y0, margin.x1, margin.y0 + frame_h],
                    "content": "text",
                    "caret": true
                }),
            )?;
            let sid = res["story"].as_u64().ok_or("Failed to get story ID")?;
            let fid = res["id"].as_u64().map(designcraft_doc::ItemId);
            let created_info = fid.map(|id| (id, spread_idx, margin.x0, margin.x1, margin.y0, max_table_h, margin.y1));
            (sid, Some(cur_page), created_info, None)
        }
    };

    // Step 2: Insert the tab-delimited text into story
    app.run("text.insert", json!({"text": table_text}))?;

    // Step 3: Select all paragraphs in the inserted story and convert to table
    let text_len = {
        let act = app.session.active().ok_or("No active document")?;
        let story = act.doc.story(designcraft_doc::StoryId(story_id)).ok_or("Story not found")?;
        story.text.len()
    };
    app.run("text.select", json!({"story": story_id, "anchor": 0, "focus": text_len}))?;

    // Step 4: Convert Text to Table
    let res = app.run("table.convertFromText", json!({"columnSeparator": "tab"}))?;

    let header_rows_count = if has_merged_headers {
        num_header_rows
    } else if st.card2_first_row_header && rows_count > 1 {
        1
    } else {
        0
    };
    let footer_rows_count = if st.card2_table_footer && rows_count > header_rows_count + 1 { 1 } else { 0 };
    if header_rows_count > 0 || footer_rows_count > 0 {
        let mut opts = json!({});
        if header_rows_count > 0 {
            opts["headerRows"] = json!(header_rows_count);
        }
        if footer_rows_count > 0 {
            opts["footerRows"] = json!(footer_rows_count);
        }
        let _ = app.run("table.options", opts);
    }

    // Step 5: Table Style (if not Horizontal Rules, apply via command first)
    let style_names = ["Basic Grid", "Corporate Navy", "Minimalist", "Horizontal Rules"];
    if st.card2_style != 3
        && let Some(&style_name) = style_names.get(st.card2_style)
    {
        let border_weight = if st.card2_style == 1 { 2.0 } else { 0.5 };
        let _ = app.run(
            "style.table.create",
            json!({
                "name": style_name,
                "border": {"weight": border_weight}
            }),
        );
        let _ = app.run("style.table.apply", json!({"name": style_name}));
    }

    let table_id = res["table"]
        .as_u64()
        .or_else(|| {
            let act = app.session.active()?;
            let story = act.doc.story(designcraft_doc::StoryId(story_id))?;
            story.tables.keys().copied().next()
        })
        .ok_or("Failed to find table ID")?;

    app.session
        .edit(|doc, _| {
            if st.card2_style == 3 && !doc.styles.table.iter().any(|s| s.name == "Horizontal Rules") {
                let ts = designcraft_doc::TableStyle {
                    name: "Horizontal Rules".to_string(),
                    border: Some(designcraft_doc::CellStroke::none()),
                    ..designcraft_doc::TableStyle::default()
                };
                doc.styles_mut().table.push(ts);
            }

            let story = doc
                .story_mut(designcraft_doc::StoryId(story_id))
                .ok_or_else(|| designcraft_engine::EngineError::Other("Story not found".to_string()))?;
            let table = story.table_mut(table_id).ok_or_else(|| designcraft_engine::EngineError::Other("Table not found".to_string()))?;

            if has_merged_headers {
                // 1. Vertical merges: for columns where leaf row is before (num_header_rows - 1)
                for (c, lvls) in col_levels.iter().enumerate() {
                    let k = lvls.len();
                    if k < num_header_rows {
                        let _ = table.merge(designcraft_doc::CellRange { r0: k.saturating_sub(1), c0: c, r1: num_header_rows - 1, c1: c });
                    }
                }

                // 2. Horizontal merges: consecutive columns sharing common ancestor prefix at level r
                for r in 0..num_header_rows.saturating_sub(1) {
                    let mut c = 0;
                    while c < cols_count {
                        if col_levels.get(c).map(|lvls| lvls.len() > r + 1).unwrap_or(false) {
                            let start = c;
                            let prefix = &col_levels[c][0..=r];
                            while c + 1 < cols_count
                                && col_levels.get(c + 1).map(|lvls| lvls.len() > r + 1).unwrap_or(false)
                                && &col_levels[c + 1][0..=r] == prefix
                            {
                                c += 1;
                            }
                            let end = c;
                            if end > start {
                                let _ = table.merge(designcraft_doc::CellRange { r0: r, c0: start, r1: r, c1: end });
                            }
                        }
                        c += 1;
                    }
                }

                // 3. Center alignment & vertical centering on all header cells
                let nc = table.ncols();
                for r in 0..num_header_rows {
                    for col_idx in 0..nc {
                        if let Some(cell) = table.cell_mut(r, col_idx) {
                            cell.vj = designcraft_doc::VerticalJustification::Center;
                            for p in &mut cell.text.paras {
                                p.para.align = Some(designcraft_doc::Align::Center);
                            }
                            cell.text.rev += 1;
                        }
                    }
                }
            }

            if st.card2_style == 3 {
                table.style = "Horizontal Rules".to_string();
                table.options.border = designcraft_doc::CellStroke::none();
                table.options.borders = Default::default();
                table.options.row_strokes = None;
                table.options.column_strokes = None;
                table.options.alt_rows = None;
                table.options.alt_cols = None;

                let nr = table.nrows();
                let nc = table.ncols();

                // Clear all cell strokes and set border_overrides so no default or table borders show
                for cell in &mut table.cells {
                    cell.strokes = std::array::from_fn(|_| designcraft_doc::CellStroke::none());
                    cell.border_overrides = [true; 4];
                    cell.stroke_defined = [true; 4];
                    cell.stroke_priorities = [0; 4];
                }

                if nr > 0 && nc > 0 {
                    let rule_thick = designcraft_doc::CellStroke {
                        weight: 1.0,
                        color: designcraft_color::swatch::BLACK.into(),
                        tint: 1.0,
                        kind: designcraft_doc::StrokeType::Solid,
                        ..Default::default()
                    };
                    let rule_thin = designcraft_doc::CellStroke {
                        weight: 0.5,
                        color: designcraft_color::swatch::BLACK.into(),
                        tint: 1.0,
                        kind: designcraft_doc::StrokeType::Solid,
                        ..Default::default()
                    };

                    let header_rows = if has_merged_headers {
                        num_header_rows
                    } else if st.card2_first_row_header {
                        1
                    } else {
                        0
                    };
                    if header_rows > 0 {
                        // Top of header (row 0 top)
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(0, c) {
                                cell.strokes[0] = rule_thick.clone();
                            }
                        }
                        // Sub-header rule under group headers (if has_merged_headers)
                        if has_merged_headers {
                            for r in 0..num_header_rows.saturating_sub(1) {
                                for (c, lvls) in col_levels.iter().enumerate() {
                                    if lvls.len() > r + 1 {
                                        if let Some(cell) = table.cell_mut(r, c) {
                                            cell.strokes[2] = rule_thin.clone();
                                        }
                                        if let Some(cell) = table.cell_mut(r + 1, c) {
                                            cell.strokes[0] = rule_thin.clone();
                                        }
                                    }
                                }
                            }
                        }
                        // Bottom of header (row header_rows - 1 bottom)
                        let bottom_header_row = header_rows - 1;
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(bottom_header_row, c) {
                                cell.strokes[2] = rule_thin.clone();
                            }
                            if nr > header_rows
                                && let Some(cell) = table.cell_mut(header_rows, c)
                            {
                                cell.strokes[0] = rule_thin.clone();
                            }
                        }

                        // Col numbers row (if checked)
                        if st.card2_col_numbers && nr > header_rows {
                            let col_num_row = header_rows;
                            for c in 0..nc {
                                if let Some(cell) = table.cell_mut(col_num_row, c) {
                                    cell.strokes[2] = rule_thin.clone();
                                }
                                if nr > col_num_row + 1
                                    && let Some(cell) = table.cell_mut(col_num_row + 1, c)
                                {
                                    cell.strokes[0] = rule_thin.clone();
                                }
                            }
                        }
                    } else if st.card2_col_numbers {
                        // Col number row is row 0
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(0, c) {
                                cell.strokes[0] = rule_thick.clone();
                                cell.strokes[2] = rule_thin.clone();
                            }
                            if nr > 1
                                && let Some(cell) = table.cell_mut(1, c)
                            {
                                cell.strokes[0] = rule_thin.clone();
                            }
                        }
                    } else {
                        // Neither header nor col numbers: top of table
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(0, c) {
                                cell.strokes[0] = rule_thick.clone();
                            }
                        }
                    }

                    // Table footer row rules (if checked)
                    if st.card2_table_footer && nr > header_rows + 1 {
                        let footer_row = nr.saturating_sub(1);
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(footer_row, c) {
                                cell.strokes[0] = rule_thin.clone();
                            }
                            if footer_row > 0
                                && let Some(cell) = table.cell_mut(footer_row - 1, c)
                            {
                                cell.strokes[2] = rule_thin.clone();
                            }
                        }
                    }

                    // Row formatting conditions: if col 1 starts with Jumlah, Tahun, Kab, Kabupaten, Kec, Kecamatan
                    if st.card2_row_conditions {
                        let data_start = if header_rows > 0 {
                            if st.card2_col_numbers { header_rows + 1 } else { header_rows }
                        } else if st.card2_col_numbers {
                            1
                        } else {
                            0
                        };

                        for r in data_start..nr {
                            let matches = {
                                let col0 = table.cell(r, 0).map(|c| c.text.text.as_str()).unwrap_or("");
                                let col1 = table.cell(r, 1).map(|c| c.text.text.as_str()).unwrap_or("");
                                matches_pubgen_row_condition(col0) || (is_index_or_empty(col0) && matches_pubgen_row_condition(col1))
                            };

                            if matches {
                                for c in 0..nc {
                                    // Top border
                                    if let Some(cell) = table.cell_mut(r, c) {
                                        cell.strokes[0] = rule_thin.clone();
                                    }
                                    if r > 0
                                        && let Some(cell) = table.cell_mut(r - 1, c)
                                    {
                                        cell.strokes[2] = rule_thin.clone();
                                    }
                                    // Bottom border
                                    if r + 1 < nr {
                                        if let Some(cell) = table.cell_mut(r, c) {
                                            cell.strokes[2] = rule_thin.clone();
                                        }
                                        if let Some(cell) = table.cell_mut(r + 1, c) {
                                            cell.strokes[0] = rule_thin.clone();
                                        }
                                    } else if let Some(cell) = table.cell_mut(r, c) {
                                        cell.strokes[2] = rule_thick.clone();
                                    }
                                }
                            }
                        }
                    }

                    // Bottom of table rule
                    let last_row = nr.saturating_sub(1);
                    for c in 0..nc {
                        if let Some(cell) = table.cell_mut(last_row, c) {
                            cell.strokes[2] = rule_thick.clone();
                        }
                    }
                }
            }

            if st.card2_table_footer
                && table.nrows() > header_rows_count + 1
                && let Some(r) = table.rows.last_mut()
            {
                r.kind = designcraft_doc::RowKind::Footer;
            }

            Ok(())
        })
        .map_err(|e| e.to_string())?;

    let mut actual_fn_target = fn_target;
    if let Some((fid, spread_idx, x0, x1, y0, max_h, margin_y1)) = created_frame_info {
        let fit_h = {
            let active = app.session.active().ok_or("No active document")?;
            let cs = designcraft_compose::compose_story(&active.doc, designcraft_doc::StoryId(story_id), &Default::default());
            let measured_h = cs.frames.first().and_then(|f| f.tables.first().map(|t| t.rect.height()).or(Some(f.content_height)));

            if let Some(th) = measured_h { (th + 4.0).min(max_h).max(40.0) } else { (total_rows as f64 * 32.0 + 40.0).min(max_h).max(60.0) }
        };

        let _ = app.session.edit(|doc, _| {
            if let Some(it) = doc.item_mut(fid) {
                it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(x0, y0, x1, y0 + fit_h));
            }
            Ok(serde_json::Value::Null)
        });

        if footnote_text.is_some() {
            let fn_y0 = y0 + fit_h + 8.0;
            let fn_y1 = (fn_y0 + 24.0).min(margin_y1);
            actual_fn_target = Some((spread_idx, [x0, fn_y0, x1, fn_y1]));
        }
    }

    if let (Some(fn_str), Some((target_spread, fn_rect))) = (footnote_text.as_ref(), actual_fn_target) {
        let _ = app.run(
            "frame.create",
            json!({
                "spread": target_spread,
                "rect": fn_rect,
                "content": "text",
                "text": fn_str,
                "caret": false
            }),
        );
    }

    let target_msg = match target_page_info {
        Some(pi) => {
            let label = app.session.page_label(pi);
            format!(" on Page {label}")
        }
        None => String::new(),
    };

    let fn_msg = if footnote_text.is_some() { " + footnote" } else { "" };
    Ok(format!("Inserted table{target_msg} ({total_rows} rows × {cols_count} cols){fn_msg}"))
}

fn to_proper_case(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut chars = trimmed.chars();
    let first = match chars.next() {
        Some(c) => c.to_uppercase().to_string(),
        None => return String::new(),
    };
    let rest: String = chars.map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
    format!("{first}{rest}")
}

fn is_plausible_type(txt: &str) -> bool {
    let trimmed = txt.trim();
    let lower = trimmed.to_lowercase();
    // Exclude footnotes, sources, notes, captions
    if lower.starts_with("source")
        || lower.starts_with("sumber")
        || lower.starts_with("note")
        || lower.starts_with("catatan")
        || lower.starts_with("ket")
        || lower.contains(':')
    {
        return false;
    }
    // Publication kickers/types are short (e.g. "Tabel", "Grafik", "Bab", "Section")
    // Never more than 2 words or 20 characters
    if trimmed.len() > 20 || trimmed.split_whitespace().count() > 2 {
        return false;
    }
    true
}

fn is_plausible_number(txt: &str) -> bool {
    let trimmed = txt.trim();
    if !trimmed.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    if trimmed.len() > 12 || trimmed.split_whitespace().count() > 1 {
        return false;
    }
    trimmed.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '(' || c == ')')
}

fn find_type_and_number_on_page(doc: &designcraft_doc::Document, page_str: &str, title: &str) -> Option<(String, String)> {
    let mut target_abs = None;
    for abs in 0..doc.page_count() {
        if doc.page_name(abs) == page_str {
            target_abs = Some(abs);
            break;
        }
    }
    let abs = target_abs?;
    let (si, pi) = doc.page_loc(abs)?;
    let spread = doc.spreads.get(si)?;

    let mut type_candidate = None;
    let mut num_candidate = None;

    for item in &spread.items {
        if spread.page_at_x(item.bounds().center().x) != Some(pi) {
            continue;
        }
        let Some(tf) = item.text_frame() else { continue };
        let Some(st) = doc.story(tf.story) else { continue };

        for line in st.text.split(['\n', designcraft_doc::story::FORCED_LINE_BREAK]) {
            let txt = line.trim();
            if txt.is_empty() || txt == title {
                continue;
            }

            if is_plausible_number(txt) {
                if num_candidate.is_none() {
                    num_candidate = Some(txt.to_string());
                }
            } else if is_plausible_type(txt) {
                if type_candidate.is_none() {
                    type_candidate = Some(to_proper_case(txt));
                }
            } else {
                for prefix in &["Grafik", "Chart", "Tabel", "Table"] {
                    if let Some(rest) = txt.strip_prefix(prefix) {
                        let candidate = rest.split_whitespace().next().unwrap_or("");
                        if is_plausible_number(candidate) {
                            if type_candidate.is_none() {
                                type_candidate = Some(to_proper_case(prefix));
                            }
                            if num_candidate.is_none() {
                                num_candidate = Some(candidate.to_string());
                            }
                        }
                    } else if let Some(rest) = txt.to_lowercase().strip_prefix(&prefix.to_lowercase()) {
                        let candidate = rest.split_whitespace().next().unwrap_or("");
                        if is_plausible_number(candidate) {
                            if type_candidate.is_none() {
                                type_candidate = Some(to_proper_case(prefix));
                            }
                            if num_candidate.is_none() {
                                num_candidate = Some(candidate.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    match (type_candidate, num_candidate) {
        (Some(t), Some(n)) => Some((t, n)),
        (Some(t), None) => Some((t, String::new())),
        (None, Some(n)) => Some((String::new(), n)),
        (None, None) => None,
    }
}

/// Check if an entry line contains only a kicker type word and/or a number (e.g. "Grafik 02.01", "Tabel 01.01", "02.01")
/// with no actual descriptive title words.
fn is_type_and_number_only(t: &str) -> bool {
    let words: Vec<&str> = t.split_whitespace().collect();
    if words.is_empty() || words.len() > 2 {
        return false;
    }
    if words.len() == 1 {
        let w = words[0].trim_end_matches('.');
        w.chars().any(|c| c.is_ascii_digit()) && w.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    } else {
        let w0_lower = words[0].to_lowercase();
        let is_type = ["grafik", "chart", "tabel", "table", "bab", "section"].contains(&w0_lower.as_str());
        let w1 = words[1].trim_end_matches('.');
        let is_num = w1.chars().any(|c| c.is_ascii_digit()) && w1.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-');
        is_type && is_num
    }
}

/// Strip type and number prefixes from a title for TablePage layout (which puts the type in the column header
/// and the number in column 1, leaving only the title text for column 2).
pub fn strip_tablepage_title_prefix<'a>(title: &'a str, num: &str, typ: &str) -> &'a str {
    let mut t = title.trim();
    if !typ.is_empty() {
        if let Some(rest) = t.strip_prefix(typ) {
            t = rest.trim();
        } else if let Some(rest) = t.to_lowercase().strip_prefix(&typ.to_lowercase()) {
            t = title[title.len() - rest.len()..].trim();
        }
    } else {
        for prefix in &["Tabel", "Table", "Grafik", "Chart"] {
            if let Some(rest) = t.strip_prefix(prefix) {
                t = rest.trim();
                break;
            } else if let Some(rest) = t.to_lowercase().strip_prefix(&prefix.to_lowercase()) {
                t = title[title.len() - rest.len()..].trim();
                break;
            }
        }
    }

    let clean_num = num.trim_end_matches('.');
    if !clean_num.is_empty() {
        if let Some(rest) = t.strip_prefix(clean_num) {
            t = rest.trim();
        } else if let Some(rest) = t.strip_prefix(num) {
            t = rest.trim();
        }
    } else if let Some((first, rest)) = t.split_once(' ') {
        let f_clean = first.trim_end_matches('.');
        if f_clean.chars().any(|c| c.is_ascii_digit()) && f_clean.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
            t = rest.trim();
        }
    }

    t.trim_start_matches(['.', '-', ':', ';', ' '])
}

/// Wrap a TOC entry title using forced line breaks (`\u{2028}`) so:
/// 1. The first line wraps early (leaving ample breathing room for leader dots and page number).
/// 2. Continuation lines fit comfortably within the remaining column width after hanging indent.
pub fn wrap_toc_title(title: &str, width: f64, hanging_indent: f64, page_str: Option<&str>) -> String {
    let clean_title = title.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
    let clean_title = clean_title.trim();
    if clean_title.is_empty() {
        return String::new();
    }

    let page_w = page_str.map(estimate_text_advance).unwrap_or(20.0);
    // Reserve breathing room for tab leader dots before the page number: at least 48pt
    let min_leader_gap = 48.0;
    let right_reserve = page_w + min_leader_gap;

    let max_line0 = (width - right_reserve).max(80.0);
    let max_line_cont = (width - hanging_indent - right_reserve).max(60.0);

    // If the full title already fits on the first line with plenty of leader dots, keep it on one line.
    if estimate_text_advance(clean_title) <= max_line0 {
        return clean_title.to_string();
    }

    let words: Vec<&str> = clean_title.split_whitespace().collect();
    if words.len() <= 1 {
        return clean_title.to_string();
    }

    let mut lines: Vec<String> = Vec::new();
    let mut cur_line = String::new();
    let mut is_first = true;

    for word in words {
        let max_w = if is_first { max_line0 } else { max_line_cont };
        let candidate = if cur_line.is_empty() { word.to_string() } else { format!("{cur_line} {word}") };

        if estimate_text_advance(&candidate) <= max_w || cur_line.is_empty() {
            cur_line = candidate;
        } else {
            lines.push(cur_line);
            cur_line = word.to_string();
            is_first = false;
        }
    }

    if !cur_line.is_empty() {
        lines.push(cur_line);
    }

    lines.join(&designcraft_doc::story::FORCED_LINE_BREAK.to_string())
}

/// Wrap a title specifically for TablePage layout, where every line of the title starts
/// after `hanging_indent`, and the page number reserve sits at the right margin.
pub fn wrap_tablepage_title(title: &str, width: f64, hanging_indent: f64, page_str: Option<&str>) -> String {
    let clean_title = title.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
    let clean_title = clean_title.trim();
    if clean_title.is_empty() {
        return String::new();
    }

    let page_w = page_str.map(estimate_text_advance).unwrap_or(20.0);
    let min_leader_gap = 48.0;
    let right_reserve = page_w + min_leader_gap;
    let max_line_w = (width - hanging_indent - right_reserve).max(60.0);

    if estimate_text_advance(clean_title) <= max_line_w {
        return clean_title.to_string();
    }

    let words: Vec<&str> = clean_title.split_whitespace().collect();
    if words.len() <= 1 {
        return clean_title.to_string();
    }

    let mut lines: Vec<String> = Vec::new();
    let mut cur_line = String::new();

    for word in words {
        let candidate = if cur_line.is_empty() { word.to_string() } else { format!("{cur_line} {word}") };

        if estimate_text_advance(&candidate) <= max_line_w || cur_line.is_empty() {
            cur_line = candidate;
        } else {
            lines.push(cur_line);
            cur_line = word.to_string();
        }
    }

    if !cur_line.is_empty() {
        lines.push(cur_line);
    }

    lines.join(&designcraft_doc::story::FORCED_LINE_BREAK.to_string())
}

/// The first column header a TablePage table of contents writes: the type word of its own kind
/// ("Grafik"/"Chart" for a list of charts, "Tabel"/"Table" otherwise) in the language the source
/// records use. In dual lang mode, it writes two lines (Indonesian on line 1, English on line 2).
fn toc_header_line(plan: &TocPlan, st: &PubgenState) -> String {
    if st.bilingual_output {
        let (id_type, en_type) = if plan.kind() == 2 { ("Grafik", "Chart") } else { ("Tabel", "Table") };
        return format!("{id_type}\tHalaman{}\u{2028}{en_type}\tPage", "");
    }
    let english = st.card1_records.iter().any(|r| {
        let t = r.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("");
        t.eq_ignore_ascii_case("table") || t.eq_ignore_ascii_case("chart")
    });
    let w = if english { locale::words(Locale::En) } else { locale::words(st.locale) };
    let type_word = if plan.kind() == 2 { w.figure } else { w.table };
    format!("{type_word}\t{}", w.page_column)
}

/// Whether a line is the column header of a TablePage table of contents ("Tabel\thalaman",
/// "Grafik\t…": the word a list of tables or a list of charts writes).
fn is_toc_header_line(line: &str) -> bool {
    ["Tabel\t", "Table\t", "Grafik\t", "Chart\t"].iter().any(|p| line.starts_with(p))
}

/// Detect whether a record represents a Section header (e.g. "Bab 4", "4.3 Perumahan", or "Section 2")
/// which in TablePage layout does not receive a page number or dot leaders.
/// Lists of tables (kind 1) or charts (kind 2) never contain section headers.
fn is_section_header(toc_kind: usize, r_type: &str, num_str: &str, title: &str) -> bool {
    if toc_kind != 0 {
        return false;
    }
    let t_lower = r_type.trim().to_lowercase();
    if t_lower == "bab" || t_lower == "section" || t_lower == "subbab" || t_lower == "kategori" || t_lower == "chapter" {
        return true;
    }
    let num_clean = num_str.trim().trim_end_matches('.');
    let segs: Vec<&str> = num_clean.split('.').filter(|s| !s.is_empty()).collect();
    if (t_lower.is_empty() || t_lower == "section" || t_lower == "bab") && segs.len() == 2 && !segs.iter().any(|s| s.len() > 3) {
        let title_lower = title.trim().to_lowercase();
        if !title_lower.starts_with("tabel ") && !title_lower.starts_with("table ") && !title_lower.starts_with("grafik ") {
            return true;
        }
    }
    false
}

/// What a table of contents lists and how it reads. "Add TOC" builds it from the panel's controls;
/// a rebuild builds it from the stored table so it keeps the TOC style, title and filter the user
/// chose when it was created.
#[derive(Clone, Debug)]
/// One listed paragraph style of a table of contents and the level it nests at.
struct TocLevel {
    style: String,
    level: u8,
    /// `pageTitle` filter: list a page only when its first text box starts with one of these.
    words: Vec<String>,
    /// Prefix the entry with the page's first text box (the chapter line): a narrative page's
    /// title reads "BAB 01 Geografi" instead of "Geografi".
    chapter_number: bool,
    /// Pair bilingual headings (ID / EN) of this style into one row on Dual Lang pages.
    pair: bool,
}

#[derive(Clone, Debug)]
pub struct TocPlan {
    /// The styles the table of contents lists, outermost level first.
    levels: Vec<TocLevel>,
    /// Title line of the table of contents.
    title: String,
    /// 0 Standard, 1 TablePage (BPS catalog layout).
    layout: usize,
    hanging: bool,
    include_self: bool,
}

impl TocPlan {
    /// The plan the panel's TOC controls describe. A paragraph list nests chapter titles over
    /// the headings the imported RTF files carry.
    fn from_state(st: &PubgenState) -> Self {
        // Pairing only makes sense when the Dual Lang toggle is on: without it the pages carry no
        // second language at all, so the checkbox stays inert.
        let pair = st.toc_pair_bilingual && st.bilingual_output;
        let level = |style: String, words: Vec<String>, chapter_number: bool| TocLevel { style, level: 1, words, chapter_number, pair };
        let levels = if st.toc_kind == 3 {
            // The narrative pages (the G and Ulasan sheets) and the headings they carry.
            vec![
                level(TOC_NARRATIVE_STYLE.to_string(), Vec::new(), true),
                TocLevel { style: "Heading 1".into(), level: 2, words: Vec::new(), chapter_number: false, pair },
                TocLevel { style: "Heading 2".into(), level: 3, words: Vec::new(), chapter_number: false, pair },
            ]
        } else if st.toc_kind == 0 {
            // The plain list: every tagged page title plus the narrative pages.
            vec![
                level(st.toc_style.clone(), toc_kind_words(0).iter().map(|w| (*w).to_string()).collect(), false),
                level(TOC_NARRATIVE_STYLE.to_string(), Vec::new(), false),
            ]
        } else {
            vec![level(st.toc_style.clone(), toc_kind_words(st.toc_kind).iter().map(|w| (*w).to_string()).collect(), false)]
        };
        let is_para = st.toc_kind == 3;
        Self {
            levels,
            title: st.toc_title.trim().to_string(),
            layout: st.toc_layout_style,
            hanging: st.toc_hanging_indent && !is_para,
            include_self: st.toc_include_self,
        }
    }

    /// The plan a stored table of contents rebuilds with: its own styles, levels, title and
    /// filters, with the panel's layout and indentation options.
    fn from_toc(st: &PubgenState, toc: &designcraft_doc::Toc) -> Self {
        let levels: Vec<TocLevel> = toc
            .entries
            .iter()
            .map(|e| TocLevel {
                style: e.style.clone(),
                level: e.level,
                words: e.page_title.clone(),
                chapter_number: e.chapter_number,
                pair: e.pair_columns,
            })
            .collect();
        let levels = if levels.is_empty() {
            vec![TocLevel { style: st.toc_style.clone(), level: 1, words: Vec::new(), chapter_number: false, pair: false }]
        } else {
            levels
        };
        let is_para = levels.first().is_some_and(|l| l.style == TOC_NARRATIVE_STYLE);
        Self {
            levels,
            title: toc.title.trim().to_string(),
            layout: st.toc_layout_style,
            hanging: st.toc_hanging_indent && !is_para,
            include_self: st.toc_include_self,
        }
    }

    /// The Quick Actions TOC style this plan came from (for messages).
    fn kind(&self) -> usize {
        if self.levels.iter().any(|l| l.level > 1) { 3 } else { toc_kind_of_words(&self.levels.first().map(|l| l.words.clone()).unwrap_or_default()) }
    }

    /// A list of chapter titles over their headings, regenerated with each entry's own level.
    /// The narrative list leads with the narrative pages; the plain list leads with the tagged
    /// page titles and adds the narrative pages as a second level of the same depth.
    fn is_paragraph_toc(&self) -> bool {
        self.levels.first().is_some_and(|l| l.style == TOC_NARRATIVE_STYLE)
    }

    /// The paragraph styles this list's entries take: the contents list owns `TOC Level n`, and
    /// the Tables and Charts lists get a family of their own. The styles carry each list's tab
    /// stop and indent, so sharing them would let regenerating one list restyle the pages of
    /// another (a table list's rows moved when the contents list was rebuilt).
    fn entry_prefix(&self) -> &'static str {
        match self.kind() {
            1 => "TOC Table Level",
            2 => "TOC Chart Level",
            _ => "TOC Level",
        }
    }

    /// Documents whose pages predate the narrative style keep their chapters: the card's TOC style
    /// on pages that open with "BAB" still lists them (with the chapter line, as a new page's),
    /// and a `pageTitle` filter keeps table and chart pages out. Only used while the document has
    /// no narrative-styled paragraph at all, so regenerated pages never take this path.
    fn with_legacy_chapters(&self, st: &PubgenState) -> Self {
        let mut plan = self.clone();
        plan.levels.push(TocLevel {
            style: st.toc_style.clone(),
            level: 1,
            words: toc_kind_words(3).iter().map(|w| (*w).to_string()).collect(),
            chapter_number: true,
            pair: st.toc_pair_bilingual && st.bilingual_output,
        });
        plan
    }

    /// Whether the table of contents writes a title line (an empty field writes none, which the
    /// table/page layout needs to start straight at its header).
    fn has_title(&self) -> bool {
        !self.title.trim().is_empty()
    }

    /// The title the definition carries: "Contents" when the field is empty.
    fn title_or_contents(&self) -> &str {
        if self.has_title() { self.title.trim() } else { "Contents" }
    }

    /// The `toc.generate` entries that list this plan's pages.
    fn entries_json(&self) -> Value {
        let prefix = self.entry_prefix();
        Value::Array(
            self.levels
                .iter()
                .map(|l| {
                    let mut e = json!({
                        "style": l.style,
                        "level": l.level,
                        "entryStyle": format!("{prefix} {}", l.level),
                        "pageNumber": "after",
                        "separator": "\t"
                    });
                    if !l.words.is_empty() {
                        e["pageTitle"] = json!(l.words);
                    }
                    if l.chapter_number {
                        e["chapterNumber"] = json!(true);
                    }
                    if l.pair {
                        e["pairColumns"] = json!(true);
                    }
                    e
                })
                .collect(),
        )
    }
}

fn enrich_toc_story(
    app: &mut DesignApp,
    sid: designcraft_doc::StoryId,
    st: &PubgenState,
    plan: &TocPlan,
    width: f64,
    hanging_indent: f64,
    toc_page: usize,
) {
    // A paragraph list is generated straight from the document's styles: rewriting its text
    // would desynchronize the entries from the "TOC Level n" styles the engine gave them.
    if plan.is_paragraph_toc() {
        return;
    }
    let _ = app.session.edit(|doc, _| {
        let full_text = doc.story(sid).map(|s| s.text.clone()).ok_or(designcraft_doc::DocError::NoStory(sid))?;
        let toc_page_label = doc.page_name(toc_page);
        let self_title = plan.title_or_contents();

        let is_table_page = plan.layout == 1;

        // The header follows the table of contents' own kind: a list of charts reads "Grafik".
        let header_line = toc_header_line(plan, st);

        let mut new_lines = Vec::new();
        let mut changed = false;
        let mut used_records: Vec<bool> = vec![false; st.card1_records.len()];

        let lines: Vec<&str> = full_text.split('\n').collect();
        let orig_title_line = lines.first().copied().unwrap_or("");

        if is_table_page {
            if plan.has_title() {
                new_lines.push(plan.title.trim().to_string());
                new_lines.push(header_line.clone());
            } else {
                new_lines.push(header_line.clone());
            }
        } else {
            new_lines.push(orig_title_line.to_string());
        }

        // 1. Pre-process raw TOC lines: extract (title, page) and coalesce multi-line entries on the same page
        let mut raw_entries: Vec<(String, Option<String>)> = Vec::new();
        for (idx, line) in lines.into_iter().enumerate() {
            if idx == 0 || line.trim().is_empty() {
                continue;
            }

            let line_clean = line.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
            if is_table_page && (line_clean == header_line || is_toc_header_line(&line_clean)) {
                continue;
            }

            let (raw_title, page_part) = match line_clean.split_once('\t') {
                Some((t, p)) => (t.trim().to_string(), Some(p.trim().to_string())),
                None => (line_clean.trim().to_string(), None),
            };

            if !raw_title.is_empty() {
                raw_entries.push((raw_title, page_part));
            }
        }

        // Coalesce entries from the same page where one is type+number and the other is the title
        let mut coalesced: Vec<(String, Option<String>)> = Vec::new();
        let mut c_idx = 0;
        while c_idx < raw_entries.len() {
            let (cur_t, cur_p) = &raw_entries[c_idx];
            if c_idx + 1 < raw_entries.len() {
                let (next_t, next_p) = &raw_entries[c_idx + 1];
                if cur_p.is_some() && cur_p == next_p {
                    if is_type_and_number_only(cur_t) && !is_type_and_number_only(next_t) {
                        coalesced.push((format!("{cur_t} {next_t}"), cur_p.clone()));
                        c_idx += 2;
                        changed = true;
                        continue;
                    } else if !is_type_and_number_only(cur_t) && is_type_and_number_only(next_t) {
                        coalesced.push((format!("{next_t} {cur_t}"), cur_p.clone()));
                        c_idx += 2;
                        changed = true;
                        continue;
                    }
                }
            }
            coalesced.push((cur_t.clone(), cur_p.clone()));
            c_idx += 1;
        }

        for (raw_title, page_part) in coalesced {
            // Check if this line is an existing TOC self-entry
            let is_self_entry = raw_title.eq_ignore_ascii_case("contents")
                || raw_title.eq_ignore_ascii_case("table of contents")
                || raw_title.eq_ignore_ascii_case("daftar isi")
                || raw_title.eq_ignore_ascii_case("daftar tabel")
                || raw_title.eq_ignore_ascii_case("daftar grafik")
                || raw_title.eq_ignore_ascii_case(self_title);

            if is_self_entry {
                if plan.include_self {
                    let wrapped =
                        if plan.hanging { wrap_toc_title(self_title, width, hanging_indent, Some(&toc_page_label)) } else { self_title.to_string() };
                    new_lines.push(format!("{wrapped}\t{toc_page_label}"));
                } else {
                    changed = true;
                }
                continue;
            }

            let mut matched_type = String::new();
            let mut matched_num = String::new();
            let mut clean_title = raw_title.clone();
            let mut matched_record = false;

            // 1. Try matching against records in st.card1_records
            for (r_idx, row) in st.card1_records.iter().enumerate() {
                if used_records.get(r_idx).copied().unwrap_or(false) {
                    continue;
                }
                let r_type = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let r_num = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let r_title = row.get(st.card1_title_col.min(2)).map(String::as_str).unwrap_or("").trim();

                let titles_match = !r_title.is_empty()
                    && (raw_title == r_title || raw_title.contains(r_title) || raw_title.ends_with(r_title) || r_title.ends_with(&raw_title) || {
                        let norm_r = strip_tablepage_title_prefix(r_title, r_num, r_type);
                        let norm_raw = strip_tablepage_title_prefix(&raw_title, r_num, r_type);
                        !norm_r.is_empty() && norm_r.eq_ignore_ascii_case(norm_raw)
                    });

                if titles_match {
                    matched_record = true;
                    matched_type = r_type.to_string();
                    matched_num = r_num.to_string();
                    clean_title = strip_tablepage_title_prefix(r_title, r_num, r_type).to_string();
                    if clean_title.is_empty() {
                        clean_title = strip_tablepage_title_prefix(&raw_title, r_num, r_type).to_string();
                    }
                    if let Some(used) = used_records.get_mut(r_idx) {
                        *used = true;
                    }
                    break;
                }
            }

            // 2. Strip prefix if in raw_title (e.g. "Tabel 4.2.1 Banyaknya..." or "Grafik 02.01 Panjang Jalan...")
            if !matched_record {
                let trimmed_title = raw_title.trim();
                let (det_type, after_type) = if let Some(rest) = trimmed_title.strip_prefix("Tabel ") {
                    ("Tabel", rest.trim())
                } else if let Some(rest) = trimmed_title.strip_prefix("Table ") {
                    ("Table", rest.trim())
                } else if let Some(rest) = trimmed_title.strip_prefix("Grafik ") {
                    ("Grafik", rest.trim())
                } else if let Some(rest) = trimmed_title.strip_prefix("Chart ") {
                    ("Chart", rest.trim())
                } else {
                    let default_type = if plan.kind() == 2 {
                        "Grafik"
                    } else if plan.kind() == 1 {
                        "Tabel"
                    } else {
                        ""
                    };
                    (default_type, trimmed_title)
                };

                if let Some((first_word, rest_of_title)) = after_type.split_once(' ')
                    && first_word.chars().any(|c| c.is_ascii_digit())
                    && first_word.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-')
                {
                    matched_type = det_type.to_string();
                    matched_num = first_word.to_string();
                    clean_title = rest_of_title.trim().to_string();
                } else if after_type.chars().any(|c| c.is_ascii_digit()) && after_type.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
                    matched_type = det_type.to_string();
                    matched_num = after_type.to_string();
                    clean_title = String::new();
                }
            }

            // 3. Fallback: inspect page frames ONLY if no record in st.card1_records matched!
            if matched_num.is_empty()
                && !matched_record
                && let Some(p_str) = &page_part
                && let Some((t_val, n_val)) = find_type_and_number_on_page(doc, p_str, &raw_title)
            {
                matched_type = t_val;
                matched_num = n_val;
            }

            if matched_type.is_empty() {
                if plan.kind() == 2 {
                    matched_type = "Grafik".to_string();
                } else if plan.kind() == 1 {
                    matched_type = "Tabel".to_string();
                }
            }

            if is_table_page {
                clean_title = strip_tablepage_title_prefix(&clean_title, &matched_num, &matched_type).to_string();
                let clean_num = matched_num.trim_end_matches('.');
                let is_sec = is_section_header(plan.kind(), &matched_type, clean_num, &clean_title);
                if is_sec {
                    let entry = if !clean_num.is_empty() { format!("{clean_num}\t{clean_title}") } else { clean_title };
                    new_lines.push(entry);
                } else {
                    let wrapped =
                        if plan.hanging { wrap_tablepage_title(&clean_title, width, hanging_indent, page_part.as_deref()) } else { clean_title };
                    let entry = if let Some(p_str) = &page_part {
                        if !clean_num.is_empty() { format!("{clean_num}\t{wrapped}\t{p_str}") } else { format!("{wrapped}\t{p_str}") }
                    } else {
                        wrapped
                    };
                    new_lines.push(entry);
                }
                changed = true;
            } else {
                // Standard mode
                let mut prefix_opt = None;
                let expected_prefix =
                    [&matched_type, &matched_num].into_iter().filter(|s| !s.is_empty()).map(|s| s.as_str()).collect::<Vec<_>>().join(" ");

                let starts_with_type = !matched_type.is_empty() && raw_title.to_lowercase().starts_with(&matched_type.to_lowercase());
                if !expected_prefix.is_empty() && !raw_title.starts_with(&expected_prefix) && !starts_with_type {
                    prefix_opt = Some(expected_prefix);
                }

                let title_to_use = if let Some(prefix) = prefix_opt { format!("{prefix} {raw_title}") } else { raw_title.to_string() };

                let wrapped_title =
                    if plan.hanging { wrap_toc_title(&title_to_use, width, hanging_indent, page_part.as_deref()) } else { title_to_use };

                if let Some(p_str) = &page_part {
                    new_lines.push(format!("{wrapped_title}\t{p_str}"));
                } else {
                    new_lines.push(wrapped_title);
                }
                changed = true;
            }
        }

        // If toc_include_self is requested and not yet added, insert right after title/header
        if plan.include_self && !new_lines.is_empty() {
            let already_has = new_lines.iter().skip(1).any(|l| {
                let t = l.split_once('\t').map(|(t, _)| t.trim()).unwrap_or(l.trim());
                t.eq_ignore_ascii_case(self_title)
                    || t.eq_ignore_ascii_case("contents")
                    || t.eq_ignore_ascii_case("table of contents")
                    || t.eq_ignore_ascii_case("daftar isi")
                    || t.eq_ignore_ascii_case("daftar tabel")
                    || t.eq_ignore_ascii_case("daftar grafik")
            });
            if !already_has {
                let wrapped =
                    if plan.hanging { wrap_toc_title(self_title, width, hanging_indent, Some(&toc_page_label)) } else { self_title.to_string() };
                let insert_pos = if is_table_page && new_lines.len() > 1 { 2 } else { 1 };
                let insert_pos = insert_pos.min(new_lines.len());
                new_lines.insert(insert_pos, format!("{wrapped}\t{toc_page_label}"));
                changed = true;
            }
        }

        if changed || is_table_page {
            let new_text = new_lines.join("\n");
            let story = doc.story_mut(sid).ok_or(designcraft_doc::DocError::NoStory(sid))?;
            let len = story.len();
            story.replace(0..len, &new_text);
        }

        Ok(serde_json::Value::Null)
    });
}

fn ensure_toc_entry_style(app: &mut DesignApp, width: f64, hanging: bool, hanging_indent: f64, layout_style: usize, prefix: &str) {
    let level_one = format!("{prefix} 1");
    let _ = app.session.edit(|d, _| {
        if d.styles.char_style("TOC English").is_none() {
            d.styles_mut().character.push(designcraft_doc::CharacterStyle {
                name: "TOC English".into(),
                based_on: None,
                chars: designcraft_doc::CharAttrs { font_style: Some("Italic".into()), ..Default::default() },
                shortcut: String::new(),
            });
        }

        let tab_stop = designcraft_doc::TabStop {
            position: width.max(36.0),
            align: designcraft_doc::TabAlign::Right,
            leader: ". ".into(),
            align_on: String::new(),
        };

        if layout_style == 1 {
            // TablePage layout styles
            let tab_stop_num = designcraft_doc::TabStop {
                position: hanging_indent,
                align: designcraft_doc::TabAlign::Left,
                leader: String::new(),
                align_on: String::new(),
            };
            let (li, fi) = if hanging { (Some(hanging_indent), Some(-hanging_indent)) } else { (Some(0.0), Some(0.0)) };

            let mut found_level1 = false;
            let mut found_sec = false;
            let mut found_hdr = false;

            for style in &mut d.styles_mut().paragraph {
                if style.name == level_one {
                    found_level1 = true;
                    style.para.left_indent = li;
                    style.para.first_line_indent = fi;
                    style.para.tabs = Some(vec![tab_stop_num.clone(), tab_stop.clone()]);
                    style.para.space_before = Some(3.0);
                } else if style.name == "TOC Section Header" {
                    found_sec = true;
                    style.para.left_indent = li;
                    style.para.first_line_indent = fi;
                    style.para.tabs = Some(vec![tab_stop_num.clone()]);
                    style.para.space_before = Some(8.0);
                    style.para.space_after = Some(2.0);
                } else if style.name == "TOC TablePage Header" {
                    found_hdr = true;
                    style.para.tabs = Some(vec![designcraft_doc::TabStop {
                        position: width.max(36.0),
                        align: designcraft_doc::TabAlign::Right,
                        leader: String::new(),
                        align_on: String::new(),
                    }]);
                    style.para.space_after = Some(8.0);
                    if style.chars.size.is_none() {
                        style.chars.size = Some(10.0);
                    }
                    if style.chars.fill_tint.is_none() {
                        style.chars.fill_tint = Some(0.6);
                    }
                }
            }

            let st = d.styles_mut();
            if !found_level1 {
                st.paragraph.push(designcraft_doc::ParagraphStyle {
                    name: level_one.clone(),
                    based_on: Some(designcraft_doc::story::BASIC_PARAGRAPH.into()),
                    next_style: None,
                    para: designcraft_doc::ParaAttrs {
                        left_indent: li,
                        first_line_indent: fi,
                        space_before: Some(3.0),
                        align: Some(designcraft_doc::Align::Left),
                        tabs: Some(vec![tab_stop_num.clone(), tab_stop]),
                        ..Default::default()
                    },
                    chars: designcraft_doc::CharAttrs::default(),
                    shortcut: String::new(),
                });
            }
            if !found_sec {
                st.paragraph.push(designcraft_doc::ParagraphStyle {
                    name: "TOC Section Header".to_string(),
                    based_on: Some(designcraft_doc::story::BASIC_PARAGRAPH.into()),
                    next_style: None,
                    para: designcraft_doc::ParaAttrs {
                        left_indent: li,
                        first_line_indent: fi,
                        space_before: Some(8.0),
                        space_after: Some(2.0),
                        align: Some(designcraft_doc::Align::Left),
                        tabs: Some(vec![tab_stop_num]),
                        ..Default::default()
                    },
                    chars: designcraft_doc::CharAttrs { font_style: Some("Bold".into()), size: Some(10.5), ..Default::default() },
                    shortcut: String::new(),
                });
            }
            if !found_hdr {
                st.paragraph.push(designcraft_doc::ParagraphStyle {
                    name: "TOC TablePage Header".to_string(),
                    based_on: Some(designcraft_doc::story::BASIC_PARAGRAPH.into()),
                    next_style: None,
                    para: designcraft_doc::ParaAttrs {
                        space_after: Some(8.0),
                        align: Some(designcraft_doc::Align::Left),
                        tabs: Some(vec![designcraft_doc::TabStop {
                            position: width.max(36.0),
                            align: designcraft_doc::TabAlign::Right,
                            leader: String::new(),
                            align_on: String::new(),
                        }]),
                        ..Default::default()
                    },
                    chars: designcraft_doc::CharAttrs { size: Some(10.0), fill_tint: Some(0.6), ..Default::default() },
                    shortcut: String::new(),
                });
            }
        } else {
            let mut found = false;
            for style in &mut d.styles_mut().paragraph {
                let Some(idx) = style.name.strip_prefix(prefix).and_then(|rest| rest.strip_prefix(' ')).and_then(|n| n.parse::<usize>().ok()) else {
                    continue;
                };
                found = true;
                let base_indent = (idx.saturating_sub(1) as f64) * 12.0;
                let (li, fi) = if hanging { (Some(base_indent + hanging_indent), Some(-hanging_indent)) } else { (Some(base_indent), Some(0.0)) };
                style.para.left_indent = li;
                style.para.first_line_indent = fi;
                style.para.tabs = Some(vec![tab_stop.clone()]);
            }

            if !found {
                let (left_ind, first_ind) = if hanging { (Some(hanging_indent), Some(-hanging_indent)) } else { (Some(0.0), Some(0.0)) };
                let st = d.styles_mut();
                st.paragraph.push(designcraft_doc::ParagraphStyle {
                    name: level_one.clone(),
                    based_on: Some(designcraft_doc::story::BASIC_PARAGRAPH.into()),
                    next_style: None,
                    para: designcraft_doc::ParaAttrs {
                        left_indent: left_ind,
                        first_line_indent: first_ind,
                        space_before: Some(4.0),
                        align: Some(designcraft_doc::Align::Left),
                        tabs: Some(vec![tab_stop]),
                        ..Default::default()
                    },
                    chars: designcraft_doc::CharAttrs::default(),
                    shortcut: String::new(),
                });
            }
        }
        Ok(serde_json::Value::Null)
    });
}

fn compute_detected_prefix_width(st: &PubgenState) -> f64 {
    if st.toc_layout_style == 1 {
        // TablePage: measure width of number column only (e.g. "4.2.1", "04.01", "4.10.12")
        let max_w = st
            .card1_records
            .iter()
            .filter_map(|row| {
                let r_num = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let clean_num = r_num.trim_end_matches('.');
                if clean_num.is_empty() { None } else { Some(estimate_text_advance(&format!("{clean_num}  "))) }
            })
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(36.0);
        max_w.clamp(28.0, 54.0)
    } else {
        st.card1_records
            .iter()
            .filter_map(|row| {
                let r_type = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let r_num = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let prefix = [r_type, r_num].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
                if prefix.is_empty() { None } else { Some(estimate_text_advance(&format!("{prefix} "))) }
            })
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(68.0)
    }
}

/// The entry that lists the table of contents itself ("Daftar Isi .... 5").
///
/// The flat lists get it from `enrich_toc_story`, which rewrites their text; a paragraph list
/// keeps the engine's own level styles, so its entry is added as a paragraph of its own.
fn add_toc_self_entry(app: &mut DesignApp, sid: designcraft_doc::StoryId, plan: &TocPlan, toc_page: usize) {
    if !plan.include_self {
        return;
    }
    let title = plan.title_or_contents().to_string();
    let _ = app.session.edit(|doc, _| {
        let page_label = doc.page_name(toc_page);
        let Some(story) = doc.story_mut(sid) else {
            return Ok(serde_json::Value::Null);
        };
        // Already listed (a rebuild starts from the engine's fresh story, but a second call on the
        // same story must not add the entry twice). The title line is not an entry.
        let ranges = story.para_ranges();
        let listed =
            ranges.iter().skip(1).any(|r| story.text[r.clone()].split('\t').next().unwrap_or_default().trim().eq_ignore_ascii_case(title.trim()));
        if listed {
            return Ok(serde_json::Value::Null);
        }
        // Where it belongs in page order, like the entries around it: before the first entry whose
        // page number is higher. A label that isn't plain digits (roman front matter) or a table
        // without numbered entries keeps it at the top, right after the title.
        let page_no = page_label.trim().parse::<u64>().ok();
        let mut at = ranges.last().map_or_else(|| story.len(), |r| r.end);
        for r in ranges.iter().skip(1) {
            let label = story.text[r.clone()].rsplit('\t').next().unwrap_or_default().trim().to_string();
            let higher = match (page_no, label.parse::<u64>().ok()) {
                (Some(page), Some(other)) => other > page,
                (None, Some(_)) => true,
                _ => false,
            };
            if higher {
                at = r.start;
                break;
            }
        }
        let entry = if at >= story.len() { format!("\n{title}\t{page_label}") } else { format!("{title}\t{page_label}\n") };
        let start = if at >= story.len() { at + 1 } else { at };
        story.insert(at, &entry);
        story.format_paras(start..start, |f| *f = designcraft_doc::ParaFormat { style: "TOC Level 1".to_string(), ..Default::default() });
        Ok(serde_json::Value::Null)
    });
}

/// Wrap a paragraph TOC title line so that continuation lines do not crowd the right margin
/// and page numbers, and maintain ample dot leader space before the page number.
fn wrap_paragraph_title(clean_title: &str, max_line_w: f64) -> String {
    let raw_words: Vec<&str> = clean_title.split_whitespace().collect();
    if raw_words.len() <= 1 {
        return clean_title.to_string();
    }

    // Attach any standalone "/" to the preceding word so that lines never begin with a slash
    let mut words: Vec<String> = Vec::new();
    for w in raw_words {
        if w == "/" && !words.is_empty() {
            if let Some(prev) = words.last_mut() {
                prev.push_str(" /");
            }
        } else {
            words.push(w.to_string());
        }
    }

    let mut lines: Vec<String> = Vec::new();
    let mut cur_line = String::new();

    for word in words {
        let candidate = if cur_line.is_empty() { word.clone() } else { format!("{cur_line} {word}") };

        if estimate_text_advance(&candidate) <= max_line_w || cur_line.is_empty() {
            cur_line = candidate;
        } else {
            lines.push(cur_line);
            cur_line = word;
        }
    }

    if !cur_line.is_empty() {
        lines.push(cur_line);
    }

    lines.join(&designcraft_doc::story::FORCED_LINE_BREAK.to_string())
}

/// Early wrapping for paragraph TOC entries so that long headings wrap cleanly before the
/// page number column, while maintaining character styling (such as italic for English halves
/// in bilingual paired headings).
fn enrich_paragraph_toc_story(app: &mut DesignApp, sid: designcraft_doc::StoryId, width: f64) {
    let _ = app.session.edit(|doc, _| {
        let story = doc.story_mut(sid).ok_or(designcraft_doc::DocError::NoStory(sid))?;
        let ranges = story.para_ranges();
        if ranges.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        let mut new_lines = Vec::new();
        let mut para_styles = Vec::new();
        let mut en_spans: Vec<(std::ops::Range<usize>, String)> = Vec::new();
        let mut current_offset = 0;

        for (pi, r) in ranges.into_iter().enumerate() {
            let para_style = story.paras.get(pi).map(|p| p.style.clone()).unwrap_or_else(|| "TOC Level 1".into());
            let para_text = story.text[r].trim_end_matches('\r').to_string();

            if pi == 0 || !para_text.contains('\t') {
                let line_len = para_text.len();
                new_lines.push(para_text);
                para_styles.push(para_style);
                current_offset += line_len + 1;
                continue;
            }

            let (title, page_part) = match para_text.split_once('\t') {
                Some((t, p)) => (t, p),
                None => {
                    let line_len = para_text.len();
                    new_lines.push(para_text);
                    para_styles.push(para_style);
                    current_offset += line_len + 1;
                    continue;
                }
            };

            let clean_title = title.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
            let clean_title = clean_title.trim();

            let level = para_style.strip_prefix("TOC Level ").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
            let base_indent = (level.saturating_sub(1) as f64) * 12.0;

            let page_trim = page_part.trim();
            let page_w = estimate_text_advance(page_trim);
            let min_leader_gap = 48.0;
            let right_reserve = page_w + min_leader_gap;
            let max_line_w = (width - base_indent - right_reserve).max(80.0);

            let wrapped_title = if clean_title.is_empty() {
                String::new()
            } else if estimate_text_advance(clean_title) <= max_line_w {
                clean_title.to_string()
            } else {
                wrap_paragraph_title(clean_title, max_line_w)
            };

            let new_line = format!("{wrapped_title}\t{page_part}");
            let line_len = new_line.len();

            // Detect English half in bilingual paired heading ("Indonesian / English")
            if let Some(slash_idx) = wrapped_title.find(" / ") {
                let en_start = current_offset + slash_idx + 3;
                let en_end = current_offset + wrapped_title.len();
                if en_start < en_end {
                    en_spans.push((en_start..en_end, "TOC English".to_string()));
                }
            } else if let Some(slash_idx) = wrapped_title.find(" /") {
                let after_slash = slash_idx + 2;
                let mut en_start_rel = after_slash;
                for c in wrapped_title[after_slash..].chars() {
                    if c.is_whitespace() || c == designcraft_doc::story::FORCED_LINE_BREAK {
                        en_start_rel += c.len_utf8();
                    } else {
                        break;
                    }
                }
                let en_start = current_offset + en_start_rel;
                let en_end = current_offset + wrapped_title.len();
                if en_start < en_end {
                    en_spans.push((en_start..en_end, "TOC English".to_string()));
                }
            }

            new_lines.push(new_line);
            para_styles.push(para_style);
            current_offset += line_len + 1;
        }

        let full_text = new_lines.join("\n");
        let len = story.len();
        story.replace(0..len, &full_text);

        story.format_chars(0..story.len(), |f| f.style = designcraft_doc::NO_CHAR_STYLE.into());
        for (span, style) in en_spans {
            story.format_chars(span, |f| f.style = style.clone());
        }

        for (pi, r) in story.para_ranges().into_iter().enumerate() {
            if let Some(st_name) = para_styles.get(pi) {
                story.format_paras(r.start..r.start, |f| {
                    *f = designcraft_doc::ParaFormat { style: st_name.clone(), ..Default::default() };
                });
            }
        }

        Ok(serde_json::Value::Null)
    });
}

fn apply_toc_post_process(
    app: &mut DesignApp,
    st: &PubgenState,
    plan: &TocPlan,
    sid: designcraft_doc::StoryId,
    width: f64,
    hanging_indent: f64,
    target_page: usize,
    mut pages_added: u64,
) -> u64 {
    enrich_toc_story(app, sid, st, plan, width, hanging_indent, target_page);

    // A paragraph list keeps the engine's own level styles, so the entry that lists the table of
    // contents itself is added here: `enrich_toc_story` rewrites the flat lists and skips them.
    if plan.is_paragraph_toc() {
        add_toc_self_entry(app, sid, plan, target_page);
        enrich_paragraph_toc_story(app, sid, width);
    }

    let entry_style = format!("{} 1", plan.entry_prefix());
    let _ = app.session.edit(|doc, _| {
        if let Some(story) = doc.story_mut(sid) {
            let ranges = story.para_ranges();
            for (pi, r) in ranges.into_iter().enumerate() {
                let para_text = story.text[r.clone()].trim();
                let style_name: String = if plan.layout == 1 {
                    if pi == 0
                        && !plan.title.trim().is_empty()
                        && (para_text == plan.title.trim()
                            || para_text.eq_ignore_ascii_case("contents")
                            || para_text.eq_ignore_ascii_case("daftar isi")
                            || para_text.eq_ignore_ascii_case("daftar tabel")
                            || para_text.eq_ignore_ascii_case("daftar grafik")
                            || para_text.eq_ignore_ascii_case("daftar chart")
                            || para_text.eq_ignore_ascii_case("daftar tabel / list of tables")
                            || para_text.eq_ignore_ascii_case("daftar grafik / list of charts")
                            || para_text.eq_ignore_ascii_case("daftar isi / table of contents"))
                    {
                        "TOC Title".to_string()
                    } else if is_toc_header_line(para_text) {
                        "TOC TablePage Header".to_string()
                    } else if para_text.split('\t').count() == 2 {
                        "TOC Section Header".to_string()
                    } else {
                        entry_style.clone()
                    }
                } else if pi == 0 {
                    "TOC Title".to_string()
                } else {
                    entry_style.clone()
                };

                // A paragraph list keeps each entry's own "TOC Level n" from `toc.generate`;
                // flattening it to level 1 would lose the heading nesting.
                if plan.is_paragraph_toc() && pi > 0 {
                    continue;
                }

                // The Tables and Charts lists center their title line.
                let center = pi == 0 && style_name == "TOC Title" && plan.kind() != 0;
                story.format_paras(r.start..r.start, |f| {
                    *f = designcraft_doc::ParaFormat { style: style_name.clone(), ..Default::default() };
                    if center {
                        f.para.align = Some(designcraft_doc::Align::Center);
                    }
                });

                if style_name == "TOC TablePage Header"
                    && let Some(pos) = story.text[r.clone()].find(designcraft_doc::story::FORCED_LINE_BREAK)
                {
                    let second_line_start = r.start + pos + designcraft_doc::story::FORCED_LINE_BREAK.len_utf8();
                    let second_line_end = r.end;
                    if second_line_start < second_line_end {
                        story.format_chars(second_line_start..second_line_end, |f| {
                            f.style = "TOC English".to_string();
                            f.over.font_style = Some("Italic".to_string());
                        });
                    }
                }
            }
        }
        Ok(serde_json::Value::Null)
    });

    ensure_toc_entry_style(app, width, plan.hanging, hanging_indent, plan.layout, plan.entry_prefix());

    if let Ok(af_res) = app.run("text.autoflow", json!({"story": sid.0, "maxPages": 50}))
        && let Some(extra) = af_res.get("pagesAdded").and_then(Value::as_u64)
    {
        pages_added += extra;
    }

    pages_added = sync_toc_continuation_headers(app, st, sid, plan, pages_added);

    pages_added
}

pub const TOC_CONTINUATION_HEADER_LABEL: &str = "pubgen_toc_continuation_header";
const TOC_CONT_HEADER_H: f64 = 18.0;
const TOC_CONT_HEADER_GAP: f64 = 6.0;

pub fn remove_toc_continuation_headers(app: &mut DesignApp) {
    let _ = app.session.edit(|doc, _| {
        let mut to_remove = Vec::new();
        for spread in &doc.spreads {
            for it in &spread.items {
                if it.label == TOC_CONTINUATION_HEADER_LABEL {
                    to_remove.push(it.id);
                }
            }
        }
        for id in to_remove {
            let _ = doc.remove_item(id);
        }
        Ok(serde_json::Value::Null)
    });
}

/// Remove the continuation-page headers of the table of contents `sid` — the ones on the pages its
/// continuation frames sit on — plus any orphan (a header on a page no table of contents occupies),
/// leaving the other tables of contents alone.
pub fn remove_toc_continuation_headers_of(app: &mut DesignApp, sid: designcraft_doc::StoryId) {
    let _ = app.session.edit(|doc, _| {
        let pages_of = |story: designcraft_doc::StoryId| -> Vec<usize> {
            doc.story(story).map(|s| s.frames.iter().skip(1).filter_map(|f| doc.page_of_item(*f)).collect()).unwrap_or_default()
        };
        let mine = pages_of(sid);
        let others: Vec<usize> = doc.tocs.iter().filter(|t| t.story != sid).flat_map(|t| pages_of(t.story)).collect();
        let mut to_remove = Vec::new();
        for spread in &doc.spreads {
            for it in &spread.items {
                if it.label != TOC_CONTINUATION_HEADER_LABEL {
                    continue;
                }
                if let Some(pg) = doc.page_of_item(it.id)
                    && (mine.contains(&pg) || !others.contains(&pg))
                {
                    to_remove.push(it.id);
                }
            }
        }
        for id in to_remove {
            let _ = doc.remove_item(id);
        }
        Ok(serde_json::Value::Null)
    });
}

fn sync_toc_continuation_headers(app: &mut DesignApp, st: &PubgenState, sid: designcraft_doc::StoryId, plan: &TocPlan, mut pages_added: u64) -> u64 {
    remove_toc_continuation_headers_of(app, sid);

    let (header_text, style_name) = if plan.layout == 1 {
        let detected = app
            .session
            .active()
            .and_then(|a| a.doc.story(sid).and_then(|s| s.text.lines().find(|l| is_toc_header_line(l.trim())).map(|l| l.trim().to_string())));

        let text = detected.unwrap_or_else(|| toc_header_line(plan, st));

        (text, "TOC TablePage Header")
    } else {
        let title = plan.title_or_contents();
        let is_id = title.to_lowercase().contains("daftar") || title.to_lowercase().contains("tabel") || title.to_lowercase().contains("grafik");
        let cont_suffix = if is_id { "(Lanjutan)" } else { "(Continued)" };
        (format!("{title} {cont_suffix}"), "TOC Title")
    };

    for _iter in 0..5 {
        let active = match app.session.active() {
            Some(a) => a,
            None => break,
        };

        let frames = match active.doc.story(sid) {
            Some(s) if s.frames.len() > 1 => s.frames.clone(),
            _ => break,
        };

        let first_pg = frames.first().and_then(|f| active.doc.page_of_item(*f));

        let cs = designcraft_compose::compose_story(&active.doc, sid, &Default::default());

        struct FramePlan {
            fid: designcraft_doc::ItemId,
            spread_ref: designcraft_doc::SpreadRef,
            layer: designcraft_doc::LayerId,
            header_rect: designcraft_geom::Rect,
            flow_rect: designcraft_geom::Rect,
            needs_header: bool,
            needs_resize: bool,
        }

        let mut plans = Vec::new();

        for &fid in &frames[1..] {
            let has_text = cs.frame(fid).is_some_and(|ft| !ft.lines.is_empty());
            if !has_text && cs.overset_at.is_none() {
                continue;
            }

            let Some(pg) = active.doc.page_of_item(fid) else { continue };
            if Some(pg) == first_pg {
                continue;
            }

            let Some((si, pi)) = active.doc.page_loc(pg) else { continue };
            let Some(spread) = active.doc.spreads.get(si) else { continue };
            let Some(page) = spread.pages.get(pi) else { continue };
            let m = page.margin_rect();

            let cont_h = if header_text.contains(designcraft_doc::story::FORCED_LINE_BREAK) { 28.0 } else { TOC_CONT_HEADER_H };
            let header_rect = designcraft_geom::Rect::new(m.x0, m.y0, m.x1, m.y0 + cont_h);
            let flow_top = (m.y0 + cont_h + TOC_CONT_HEADER_GAP).min(m.y1 - 10.0);
            let flow_rect = designcraft_geom::Rect::new(m.x0, flow_top, m.x1, m.y1);

            let layer = active.doc.item(fid).map(|it| it.layer).unwrap_or_else(|| active.doc.default_layer());

            let has_hdr = spread.items.iter().any(|it| it.label == TOC_CONTINUATION_HEADER_LABEL && active.doc.page_of_item(it.id) == Some(pg));

            let current_b = active.doc.item(fid).map(|it| it.bounds()).unwrap_or(designcraft_geom::Rect::ZERO);
            let needs_resize = (current_b.y0 - flow_top).abs() > 1.0;

            if !has_hdr || needs_resize {
                plans.push(FramePlan {
                    fid,
                    spread_ref: designcraft_doc::SpreadRef::Doc(si),
                    layer,
                    header_rect,
                    flow_rect,
                    needs_header: !has_hdr,
                    needs_resize,
                });
            }
        }

        if plans.is_empty() {
            break;
        }

        let mut resized_any = false;
        let _ = app.session.edit(|doc, _| {
            for p in &plans {
                if p.needs_header {
                    let para = designcraft_doc::ParaFormat { style: style_name.to_string(), ..Default::default() };
                    if let Ok((hdr_id, hdr_sid)) = doc.add_text_frame(p.spread_ref, p.header_rect, p.layer, &header_text, para) {
                        if let Some(it) = doc.item_mut(hdr_id) {
                            it.label = TOC_CONTINUATION_HEADER_LABEL.to_string();
                            it.name = "TOC Continuation Header".to_string();
                        }
                        if let Some(hdr_story) = doc.story_mut(hdr_sid)
                            && let Some(pos) = hdr_story.text.find(designcraft_doc::story::FORCED_LINE_BREAK)
                        {
                            let start = pos + designcraft_doc::story::FORCED_LINE_BREAK.len_utf8();
                            let end = hdr_story.len();
                            if start < end {
                                hdr_story.format_chars(start..end, |f| {
                                    f.style = "TOC English".to_string();
                                    f.over.font_style = Some("Italic".to_string());
                                });
                            }
                        }
                    }
                }
                if p.needs_resize
                    && let Some(it) = doc.item_mut(p.fid)
                {
                    it.path = designcraft_geom::shapes::rectangle(p.flow_rect);
                    resized_any = true;
                }
            }
            Ok(serde_json::Value::Null)
        });

        if resized_any {
            if let Ok(af_res) = app.run("text.autoflow", json!({"story": sid.0, "maxPages": 50}))
                && let Some(extra) = af_res.get("pagesAdded").and_then(Value::as_u64)
            {
                pages_added += extra;
            }
        } else {
            break;
        }
    }

    pages_added
}

/// The story of the table of contents whose first frame is on page `page`, if any.
fn toc_story_on_page(app: &DesignApp, page: usize) -> Option<designcraft_doc::StoryId> {
    app.session.active()?.doc.toc_on_page(page).map(|t| t.story)
}

/// The page a table of contents' first frame sits on.
fn toc_story_page(app: &DesignApp, sid: designcraft_doc::StoryId) -> Option<usize> {
    let active = app.session.active()?;
    let story = active.doc.story(sid)?;
    story.frames.first().and_then(|f| active.doc.page_of_item(*f))
}

/// Text width of a page's margins (the TOC's column).
fn page_margin_width(app: &DesignApp, page: usize) -> Result<f64, String> {
    let active = app.session.active().ok_or("No active document")?;
    let (si, pi) = active.doc.page_loc(page).ok_or("Page location not found")?;
    let spread = active.doc.spreads.get(si).ok_or("Spread missing")?;
    let p = spread.pages.get(pi).ok_or("Page missing")?;
    let m = p.margin_rect();
    Ok((m.x1 - m.x0).max(100.0))
}

/// Hanging indent of wrapped TOC entries for a layout.
fn toc_hanging_indent(st: &PubgenState, layout: usize) -> f64 {
    let detected = compute_detected_prefix_width(st);
    if layout == 1 { detected.round().clamp(28.0, 60.0) } else { detected.round().clamp(48.0, 140.0) }
}

/// Generate (or regenerate) one table of contents on `page` and run the panel's post-process
/// (type/number merging, title styles, continuation pages and headers).
/// Whether any paragraph of the document carries `style` (the panel's own marker styles are only
/// present once a page has been generated with them).
fn document_uses_style(app: &DesignApp, style: &str) -> bool {
    app.session.active().is_some_and(|a| a.doc.stories.values().any(|s| s.paras.iter().any(|p| p.style == style)))
}

fn generate_toc(app: &mut DesignApp, st: &PubgenState, page: usize, plan: &TocPlan, replace: bool) -> Result<(u64, u64), String> {
    let width = page_margin_width(app, page)?;
    let hanging_indent = toc_hanging_indent(st, plan.layout);

    // A document whose pages were made before the narrative style existed still lists its
    // chapters, rather than coming up empty until its pages are generated again.
    let legacy = plan.clone();
    let plan = if plan.is_paragraph_toc() && !document_uses_style(app, TOC_NARRATIVE_STYLE) { legacy.with_legacy_chapters(st) } else { legacy };
    let plan = &plan;

    for level in &plan.levels {
        ensure_para_style(app, &level.style);
    }
    ensure_toc_entry_style(app, width, plan.hanging, hanging_indent, plan.layout, plan.entry_prefix());

    let res = app.run(
        "toc.generate",
        json!({
            "page": page + 1,
            "title": plan.title_or_contents(),
            "entries": plan.entries_json(),
            "replace": replace,
            "autoflow": true
        }),
    )?;

    let count = res.get("entries").and_then(Value::as_u64).unwrap_or(0);
    let pages_added = res.get("pagesAdded").and_then(Value::as_u64).unwrap_or(0);
    let sid_u64 = res.get("story").and_then(Value::as_u64).ok_or("TOC story missing")?;
    let sid = designcraft_doc::StoryId(sid_u64);

    let final_pages_added = apply_toc_post_process(app, st, plan, sid, width, hanging_indent, page, pages_added);
    Ok((count, final_pages_added))
}

pub fn execute_add_toc(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let cur_page = target_page_index(app);
    let cur_label = app.session.page_label(cur_page);

    let plan = TocPlan::from_state(st);
    // A table of contents already on this page is regenerated in place, so one page holds one.
    let replace = toc_story_on_page(app, cur_page).is_some();
    let (count, final_pages_added) = generate_toc(app, st, cur_page, &plan, replace)?;

    let cont_msg = if final_pages_added > 0 { format!(" across {} pages", final_pages_added + 1) } else { String::new() };

    let kind = TOC_KINDS.get(plan.kind()).copied().unwrap_or(TOC_KINDS[0]);
    Ok(format!("Added {kind} on Page {cur_label} ({count} entries{cont_msg})"))
}

/// Rebuild the table of contents on the page in view, keeping the TOC style, title and filter it
/// was created with. With none on this page it is added, as when the document had none.
pub fn execute_rebuild_toc(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let cur_page = target_page_index(app);
    let Some(sid) = toc_story_on_page(app, cur_page) else {
        return execute_add_toc(app, st);
    };

    let plan = {
        let active = app.session.active().ok_or("No active document")?;
        let toc = active.doc.toc_of_story(sid).ok_or("No table of contents exists")?;
        TocPlan::from_toc(st, toc)
    };

    let toc_label = app.session.page_label(cur_page);
    let (count, final_pages_added) = generate_toc(app, st, cur_page, &plan, true)?;

    let cont_msg = if final_pages_added > 0 { format!(" across {} pages", final_pages_added + 1) } else { String::new() };

    Ok(format!("Rebuilt TOC on Page {toc_label} ({count} entries{cont_msg})"))
}

/// Rebuild every table of contents in the document (after renumbering), each with its own TOC
/// style, title and filter.
pub fn rebuild_all_tocs(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    let story_ids: Vec<designcraft_doc::StoryId> = {
        let active = app.session.active().ok_or("No active document")?;
        active.doc.tocs.iter().map(|t| t.story).collect()
    };
    if story_ids.is_empty() {
        return Ok(String::new());
    }

    let mut rebuilt = 0usize;
    let mut entries = 0u64;
    for sid in story_ids {
        // Look the page up again each time: regenerating one table can add or remove pages.
        let Some(page) = toc_story_page(app, sid) else { continue };
        let plan = {
            let active = match app.session.active() {
                Some(a) => a,
                None => break,
            };
            match active.doc.toc_of_story(sid) {
                Some(toc) => TocPlan::from_toc(st, toc),
                None => continue,
            }
        };
        let (n, _) = generate_toc(app, st, page, &plan, true)?;
        entries += n;
        rebuilt += 1;
    }

    if rebuilt == 0 {
        return Ok(String::new());
    }
    Ok(format!("{rebuilt} table(s) of contents rebuilt ({entries} entries)"))
}

pub fn execute_renumber_page(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let cur_page = target_page_index(app);
    let old_label = app.session.page_label(cur_page);
    let start_num = st.card3_renumber_start.max(1);

    let style_str = match st.card3_renumber_style {
        1 => "lowerRoman",
        2 => "upperRoman",
        _ => "arabic",
    };

    app.run(
        "layout.section",
        json!({
            "page": cur_page + 1,
            "startNumber": start_num,
            "style": style_str
        }),
    )?;

    let new_label = app.session.page_label(cur_page);

    // If TOC exists, automatically rebuild it so the TOC entries reflect the newly numbered page!
    let toc_msg = match rebuild_all_tocs(app, st) {
        Ok(msg) if !msg.is_empty() => format!(" & {msg}"),
        _ => String::new(),
    };

    Ok(format!("Renumbered Page {old_label} → '{new_label}' (start: {start_num}){toc_msg}"))
}

pub fn execute_continue_numbering(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let cur_page = target_page_index(app);
    if cur_page == 0 {
        return Err("Page 1 is always the start of the document".into());
    }
    app.run(
        "layout.section",
        json!({
            "page": cur_page + 1,
            "remove": true
        }),
    )?;
    let new_label = app.session.page_label(cur_page);

    let toc_msg = match rebuild_all_tocs(app, st) {
        Ok(msg) if !msg.is_empty() => format!(" & {msg}"),
        _ => String::new(),
    };

    let page_num = cur_page + 1;
    Ok(format!("Continued numbering on Page {page_num} (now '{new_label}'){toc_msg}"))
}

pub fn execute_page_from_rtf(app: &mut DesignApp, st: &mut PubgenState, path: &str) -> Result<String, String> {
    let target_path = if path.trim().is_empty() { st.rtf_path.trim().to_string() } else { path.trim().to_string() };
    if target_path.is_empty() {
        return Err("No RTF file path specified".into());
    }
    let p = std::path::Path::new(&target_path);
    if !p.exists() {
        return Err(format!("File not found: {target_path}"));
    }

    let (cur_page, target_frame) = {
        let active = app.session.active().ok_or("No active document")?;
        let tf = active.selection.items.first().copied().filter(|&id| {
            active.doc.item(id).is_some_and(|it| matches!(it.content, designcraft_doc::Content::Unassigned | designcraft_doc::Content::Text(_)))
        });
        let fallback_page = app
            .session
            .active()
            .and_then(|s| s.selection.page)
            .or_else(|| crate::canvas::current_page(app))
            .unwrap_or_else(|| target_page_index(app));
        let p = tf.and_then(|id| active.doc.page_of_item(id)).unwrap_or(fallback_page);
        (p, tf)
    };
    let cur_label = app.session.page_label(cur_page);

    let res = if let Some(fid) = target_frame {
        app.run(
            "file.place",
            json!({
                "path": target_path,
                "frame": fid.0,
                "autoflow": true,
            }),
        )?
    } else {
        let _ = app.run("selection.set", json!({"ids": []}));
        app.run(
            "file.place",
            json!({
                "path": target_path,
                "page": cur_page + 1,
                "autoflow": true,
            }),
        )?
    };

    crate::canvas::go_to_page(app, cur_page);
    st.rtf_path = target_path.clone();

    let pages_added = res.get("pagesAdded").and_then(Value::as_u64).unwrap_or(0);
    let overset = res.get("overset").and_then(Value::as_bool).unwrap_or(false);
    let filename = std::path::Path::new(&target_path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| target_path.clone());

    let page_info = if pages_added > 0 { format!(" across {} pages ({} added)", pages_added + 1, pages_added) } else { String::new() };
    let overset_info = if overset { " (text still overset)" } else { "" };

    Ok(format!("Imported '{filename}' to Page {cur_label}{page_info}{overset_info}"))
}

/// Page from Image: place an image on the selected page either as an image object fitted inside
/// the page margins, or as a full-page background (page bounds, sent to the back).
pub fn execute_page_from_image(app: &mut DesignApp, st: &mut PubgenState, path: &str) -> Result<String, String> {
    let target_path = if path.trim().is_empty() {
        st.image_path.trim().trim_matches('"').trim().to_string()
    } else {
        path.trim().trim_matches('"').trim().to_string()
    };
    if target_path.is_empty() {
        return Err("No image file path specified".into());
    }
    if !std::path::Path::new(&target_path).exists() {
        return Err(format!("File not found: {target_path}"));
    }

    let cur_page =
        app.session.active().and_then(|s| s.selection.page).or_else(|| crate::canvas::current_page(app)).unwrap_or_else(|| target_page_index(app));
    let cur_label = app.session.page_label(cur_page);

    // Image object fits inside the page margins; a page background covers the whole page.
    let (spread_idx, target_rect) = {
        let active = app.session.active().ok_or("No active document")?;
        let (si, pi) = active.doc.page_loc(cur_page).ok_or("No such page")?;
        let page = &active.doc.spreads[si].pages[pi];
        let rect = if st.image_as_background { page.bounds() } else { page.margin_rect() };
        (si, rect)
    };

    // 1. Graphic frame on the target spread at the target rect.
    let res = app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [target_rect.x0, target_rect.y0, target_rect.x1, target_rect.y1],
            "content": "graphic",
            "caret": false,
        }),
    )?;
    let fid = res.get("id").and_then(Value::as_u64).ok_or("Failed to create graphic frame")?;

    // 2. Place the image into the frame.
    app.run("file.place", json!({ "path": target_path, "frame": fid }))?;

    // 3. Fit the image proportionally, center it, fit frame to content, and for background send to back.
    let _ = app.run("object.fit", json!({ "mode": "fitProportionally", "ids": [fid] }));
    let _ = app.run("object.fit", json!({ "mode": "centerContent", "ids": [fid] }));
    let _ = app.run("object.fit", json!({ "mode": "fitFrameToContent", "ids": [fid] }));
    if st.image_as_background {
        let _ = app.run("object.arrange", json!({ "to": "back", "ids": [fid] }));
    }

    let _ = app.run("selection.set", json!({ "ids": [fid] }));

    crate::canvas::go_to_page(app, cur_page);
    st.image_path = target_path.clone();

    let filename = std::path::Path::new(&target_path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| target_path.clone());
    let mode_desc =
        if st.image_as_background { "page background (fitted proportionally to page, sent to back)" } else { "image object (fitted inside margins)" };

    Ok(format!("Placed '{filename}' as {mode_desc} on Page {cur_label}"))
}

/// Map a `COVER` sheet style name (e.g. `CHEVRON MEDALLION`) to the Publication Cover style index.
fn cover_style_index(spec: &str) -> Option<usize> {
    let norm = |s: &str| -> String { s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect() };
    const NAMES: [&str; 10] = [
        "Wave Curves",
        "Geometric Medallion",
        "Folded Origami",
        "Chevron Medallion",
        "Honeycomb Hexagon",
        "Corporate Headline",
        "Diagonal Arrow",
        "Modern Horizon",
        "Business Report",
        "Urban Angle",
    ];
    let want = norm(spec);
    if want.is_empty() {
        return None;
    }
    NAMES.iter().position(|n| {
        let n = norm(n);
        n.contains(&want) || want.contains(&n)
    })
}

/// Copy the `COVER` sheet values into the Publication Cover card.
fn apply_excel_cover(st: &mut PubgenState, cover: &excel::ExcelCoverData, base_dir: Option<&Path>) {
    if let Some(idx) = cover_style_index(&cover.style) {
        st.cover_style = idx;
    }

    // The sheet's `title1` is the card's "Title" (the headline) and `title2` its "Title 2" (the
    // second headline line); see the frame slots `#slot:text:title` / `#slot:text:title#2`. The
    // sheet's `level` row has no frame to fill any more: the covers carry no kicker line.
    if !cover.title1.trim().is_empty() {
        st.cover_title = cover.title1.trim().to_string();
    }
    if !cover.title2.trim().is_empty() {
        st.cover_title2 = cover.title2.trim().to_string();
    }
    if !cover.title_en.trim().is_empty() {
        st.cover_title_en = cover.title_en.trim().to_string();
    }
    if !cover.year.trim().is_empty() {
        st.cover_year = cover.year.trim().to_string();
    }
    if !cover.volume.trim().is_empty() {
        let vol = cover.volume.trim();
        st.cover_volume = if vol.to_lowercase().starts_with("vol") { vol.to_string() } else { format!("Volume {vol}") };
    }
    if !cover.catalogue.trim().is_empty() {
        st.cover_catalogue = cover.catalogue.trim().to_string();
    }
    if !cover.code.trim().is_empty() {
        st.cover_code_num = cover.code.trim().to_string();
    }
    if !cover.agency.trim().is_empty() {
        st.cover_agency_text = cover.agency.trim().to_string();
    }

    // Relative image paths in the sheet are relative to the workbook.
    let resolve = |p: &str| -> String {
        let p = p.trim();
        if p.is_empty() || Path::new(p).is_file() {
            return p.to_string();
        }
        base_dir.map(|d| d.join(p)).filter(|j| j.is_file()).map(|j| j.to_string_lossy().to_string()).unwrap_or_else(|| p.to_string())
    };
    if !cover.image_hero.trim().is_empty() {
        st.cover_image_skyline = resolve(&cover.image_hero);
    }
    if !cover.image_landscape.trim().is_empty() {
        st.cover_image_landscape = resolve(&cover.image_landscape);
    }
    if !cover.logo.trim().is_empty() {
        st.cover_image_logo = resolve(&cover.logo);
    }
}

pub fn execute_excel_to_csvs(st: &mut PubgenState, path: &str) -> Result<String, String> {
    let pkg = excel::execute_convert_excel(path)?;
    st.excel_had_cover = pkg.cover.is_some();
    if let Some(cover) = &pkg.cover {
        apply_excel_cover(st, cover, Path::new(path).parent());
    }
    // 1. Auto-load generated catalog into Card 1 (Page from CSV)
    st.card1_path = pkg.catalog_path.clone();
    load_card1_csv(st, &pkg.catalog_path);

    // 2. Auto-load table CSVs into Card 2 (Table from CSV)
    st.card2_sheet_tables = pkg.table_paths.clone();
    st.card2_sheet_idx = 0;
    if let Some(first_table) = pkg.table_paths.first() {
        st.card2_path = first_table.clone();
        load_card2_csv(st, first_table);
    }

    // 3. Cache ZIP archive metadata and bytes for user download
    st.generated_csv_files = pkg.csv_files;
    st.generated_zip_name = pkg.zip_name.clone();
    st.generated_zip_bytes = pkg.zip_bytes;

    let folder_name = std::path::Path::new(&pkg.parent_dir).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(pkg.parent_dir);
    let count = pkg.table_count;
    let zip_name = pkg.zip_name;
    Ok(format!("Converted {count} sheets → catalog CSV + {count} table CSVs + {zip_name} in {folder_name} (loaded in Page & Table from CSV)"))
}

/// Convert multi-sheet Excel workbook and, if `st.excel_auto_generate` is enabled,
/// automatically run "Page from CSV" with default options and auto-insert each sheet's table.
pub fn execute_excel_and_generate(app: &mut DesignApp, st: &mut PubgenState, path: &str) -> Result<String, String> {
    let conv_msg = execute_excel_to_csvs(st, path)?;
    if !st.excel_auto_generate || st.card1_records.is_empty() {
        return Ok(conv_msg);
    }

    // Ensure an active document exists
    if app.session.active().is_none() {
        let _ = app.run("file.new", json!({}));
    }

    // Apply default options for automatic page and table generation
    st.card1_auto_insert_tables = true;
    st.card1_range_all = true;
    st.card2_first_row_header = true;
    st.card2_col_numbers = true;
    st.card2_table_footer = true;
    st.card2_row_conditions = true;

    let mut cover_res_msg = None;
    if st.excel_had_cover {
        // The cover page goes before Page 1. Generating it before the body pages ensures
        // that all subsequent body pages land on their permanent spreads and page sides (Left vs Right),
        // preventing spread repagination from flipping inside and outside margins.
        match execute_generate_cover(app, st) {
            Ok(cover_msg) => {
                st.cover_status = Some((cover_msg.clone(), true));
                cover_res_msg = Some(cover_msg);
            }
            Err(e) => {
                st.cover_status = Some((e.clone(), false));
                return Ok(format!("{conv_msg}, but cover failed: {e}"));
            }
        }
    }

    // Run Page from CSV (which auto-inserts the table for each sheet on its corresponding page)
    let gen_res = execute_generate_pages(app, st);
    match gen_res {
        Ok(gen_msg) => {
            st.card1_status = Some((gen_msg.clone(), true));
            let count = st.card2_sheet_tables.len();
            if let Some(cover_msg) = cover_res_msg {
                return Ok(format!("Converted {count} sheets & auto-generated pages + cover ({gen_msg}; {cover_msg})!"));
            }
            Ok(format!("Converted {count} sheets & auto-generated pages ({gen_msg})!"))
        }
        Err(e) => {
            st.card1_status = Some((e.clone(), false));
            Err(format!("{conv_msg}, but page generation failed: {e}"))
        }
    }
}

pub fn execute_download_csv_zip(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    // Ensure we have zip bytes in memory
    if st.generated_zip_bytes.is_empty() {
        let mut files: Vec<(String, String)> = Vec::new();

        if !st.generated_csv_files.is_empty() {
            files = st.generated_csv_files.clone();
        } else {
            // Check if card1_path exists and can be read
            if !st.card1_path.is_empty() {
                let p = std::path::Path::new(&st.card1_path);
                if p.is_file() {
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "catalog.csv".into());
                    if let Ok(content) = std::fs::read_to_string(p) {
                        files.push((name, content));
                    }
                }
            }

            // Check card2_sheet_tables
            for tbl_p_str in &st.card2_sheet_tables {
                let p = std::path::Path::new(tbl_p_str);
                if p.is_file() {
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "table.csv".into());
                    if let Ok(content) = std::fs::read_to_string(p)
                        && !files.iter().any(|(n, _)| n == &name)
                    {
                        files.push((name, content));
                    }
                }
            }

            // If card2_path exists and not already included
            if !st.card2_path.is_empty() {
                let p = std::path::Path::new(&st.card2_path);
                if p.is_file() {
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "table.csv".into());
                    if !files.iter().any(|(n, _)| n == &name)
                        && let Ok(content) = std::fs::read_to_string(p)
                    {
                        files.push((name, content));
                    }
                }
            }
        }

        if files.is_empty() {
            // Prompt user to pick an Excel file to convert and zip
            if let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("dataMerge")) {
                execute_excel_to_csvs(st, &path)?;
            } else {
                return Err("No CSV files available to download. Please convert an Excel file or load CSVs first.".into());
            }
        } else {
            let files_ref: Vec<(&str, &str)> = files.iter().map(|(n, c)| (n.as_str(), c.as_str())).collect();
            let bytes = excel::create_csv_zip(&files_ref)?;
            st.generated_zip_bytes = bytes;
            if st.generated_zip_name.is_empty() {
                let stem = if !st.card1_path.is_empty() {
                    std::path::Path::new(&st.card1_path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "pubgen".into())
                } else {
                    "pubgen_data".into()
                };
                st.generated_zip_name = format!("{stem}_csvs.zip");
            }
        }
    }

    if st.generated_zip_bytes.is_empty() {
        return Err("No CSV zip data generated".into());
    }

    let zip_name = if st.generated_zip_name.is_empty() { "pubgen_csvs.zip".to_string() } else { st.generated_zip_name.clone() };

    // Deliver bytes via Web download or Desktop save dialog
    if let Some(download) = app.services.download.as_mut() {
        download(&zip_name, &st.generated_zip_bytes).map_err(|e| format!("Could not download {zip_name}: {e}"))?;
        Ok(format!("Downloaded {zip_name}"))
    } else if let Some(pick_save) = app.services.pick_save.as_mut() {
        if let Some(save_path) = pick_save(&zip_name) {
            if let Some(w) = app.services.write.as_mut() {
                w(&save_path, &st.generated_zip_bytes).map_err(|e| format!("Could not save {save_path}: {e}"))?;
            } else {
                std::fs::write(&save_path, &st.generated_zip_bytes).map_err(|e| format!("Could not save {save_path}: {e}"))?;
            }
            let file_name = std::path::Path::new(&save_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(save_path);
            Ok(format!("Saved {file_name}"))
        } else {
            Ok("ZIP download cancelled".into())
        }
    } else {
        // Fallback: save to parent dir of card1_path or current dir
        let target_dir = if !st.card1_path.is_empty() {
            std::path::Path::new(&st.card1_path).parent().unwrap_or(std::path::Path::new(".")).to_path_buf()
        } else {
            std::path::PathBuf::from(".")
        };
        let save_path = target_dir.join(&zip_name);
        std::fs::write(&save_path, &st.generated_zip_bytes).map_err(|e| format!("Could not save {}: {e}", save_path.display()))?;
        Ok(format!("Saved {}", save_path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cover_card_carries_one_title_and_a_second_title_line() {
        let st = PubgenState::default();
        // The card takes a single headline and keeps the second headline line, with its dummy text.
        assert_eq!(st.cover_title, default_cover_title());
        assert_eq!(st.cover_title2, "Title 2");
    }

    #[test]
    fn test_excel_cover_sheet_fills_title_and_title2() {
        let mut st = PubgenState::default();
        let cover = excel::ExcelCoverData {
            level: "KABUPATEN".to_string(),
            title1: "PRODUK DOMESTIK REGIONAL BRUTO".to_string(),
            title2: "KABUPATEN KARAWANG".to_string(),
            ..Default::default()
        };
        apply_excel_cover(&mut st, &cover, None);
        assert_eq!(st.cover_title, "PRODUK DOMESTIK REGIONAL BRUTO");
        assert_eq!(st.cover_title2, "KABUPATEN KARAWANG");
    }

    #[test]
    fn test_parse_csv_content_rfc4180() {
        let csv = "type,number,title\n\"Chapter\",01,\"The Beginning, and Future\"\nSection,02,\"Clean Architecture\"\n";
        let rows = parse_csv_content(csv);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["type", "number", "title"]);
        assert_eq!(rows[1], vec!["Chapter", "01", "The Beginning, and Future"]);
        assert_eq!(rows[2], vec!["Section", "02", "Clean Architecture"]);
    }

    #[test]
    fn test_parse_csv_content_tab_and_semicolon() {
        let tsv = "Item\tQty\tPrice\nWidget\t10\t$5.00\n";
        let rows = parse_csv_content(tsv);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["Item", "Qty", "Price"]);
        assert_eq!(rows[1], vec!["Widget", "10", "$5.00"]);

        let semi = "ColA;ColB\nValA;ValB\n";
        let rows2 = parse_csv_content(semi);
        assert_eq!(rows2.len(), 2);
        assert_eq!(rows2[0], vec!["ColA", "ColB"]);
        assert_eq!(rows2[1], vec!["ValA", "ValB"]);
    }

    #[test]
    fn test_detect_delimiter_semicolon_with_commas_in_data() {
        let content = "type;number;title\n\
                       Tabel;01.01.;Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006\n\
                       Tabel;01.02.;Keadaan Curah Hujan Tiap Bulan menurut Kecamatan (mm), 2006\n";
        assert_eq!(detect_delimiter(content), ';');
        let rows = parse_csv_content(content);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["type", "number", "title"]);
        assert_eq!(rows[1], vec!["Tabel", "01.01.", "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"]);
        assert_eq!(rows[2], vec!["Tabel", "01.02.", "Keadaan Curah Hujan Tiap Bulan menurut Kecamatan (mm), 2006"]);
    }

    #[test]
    fn test_detect_delimiter_various_formats() {
        // Comma with quotes containing semicolon
        let csv_with_semi = "id,name,tags\n1,Alice,\"tag1;tag2\"\n2,Bob,\"tag3\"\n";
        assert_eq!(detect_delimiter(csv_with_semi), ',');
        let r1 = parse_csv_content(csv_with_semi);
        assert_eq!(r1[1], vec!["1", "Alice", "tag1;tag2"]);

        // Tab with commas in values
        let tsv_with_commas = "item\tdesc\tprice\npen\tblue, 0.5mm\t$1.50\n";
        assert_eq!(detect_delimiter(tsv_with_commas), '\t');

        // Pipe delimited
        let pipe = "ColA | ColB | ColC\nVal1 | Val2 | Val3\n";
        assert_eq!(detect_delimiter(pipe), '|');

        // Single line
        assert_eq!(detect_delimiter("type;number;title"), ';');
        assert_eq!(detect_delimiter("type,number,title"), ',');
        assert_eq!(detect_delimiter("type\tnumber\ttitle"), '\t');
        assert_eq!(detect_delimiter("type|number|title"), '|');

        // UTF-8 BOM
        let with_bom = "\u{FEFF}type;number;title\nTabel;01;Judul\n";
        assert_eq!(detect_delimiter(with_bom), ';');
        let rows_bom = parse_csv_content(with_bom);
        assert_eq!(rows_bom[0], vec!["type", "number", "title"]);
    }

    #[test]
    fn test_load_kbda_sample_file_with_semicolon() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/KBDA2026_chapters.csv");
        if !std::path::Path::new(path).exists() {
            return;
        }
        let mut st1 = PubgenState::default();
        load_card1_csv(&mut st1, path);

        assert_eq!(st1.card1_delim, Some(';'));
        assert_eq!(st1.card1_headers, vec!["type", "number", "title"]);
        assert_eq!(st1.card1_type_col, 0);
        assert_eq!(st1.card1_num_col, 1);
        assert_eq!(st1.card1_title_col, 2);
        assert_eq!(st1.card1_records.len(), 194);
        assert_eq!(st1.card1_records[0], vec!["Tabel", "01.01.", "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"]);

        let mut st2 = PubgenState::default();
        load_card2_csv(&mut st2, path);
        assert_eq!(st2.card2_delim, Some(';'));
        assert_eq!(st2.card2_matrix.len(), 195); // 1 header + 194 data
        assert_eq!(st2.card2_matrix[0], vec!["type", "number", "title"]);
    }

    #[test]
    fn test_execute_generate_pages() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let initial_pages = app.session.active().unwrap().doc.page_count();

        let st = PubgenState {
            card1_records: vec![
                vec!["Chapter".into(), "1".into(), "Getting Started".into()],
                vec!["Chapter".into(), "2".into(), "Advanced Layouts".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_range_from: 1,
            card1_range_to: 2,
            card1_preset: 0,
            card1_append: true,
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "generate_pages should succeed: {:?}", res);

        let final_pages = app.session.active().unwrap().doc.page_count();
        assert_eq!(final_pages, initial_pages + 2);

        // Verify the 1-row 3-column frames on the newly created spread
        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.last().unwrap();
        assert!(spread.items.len() >= 3, "Expected at least 3 frames on generated spread");
        let items = &spread.items;
        let n = items.len();
        let f1 = &items[n - 3];
        let f2 = &items[n - 2];
        let f3 = &items[n - 1];
        // Same row: y0 and y1 are identical
        assert_eq!(f1.bounds().y0, f2.bounds().y0);
        assert_eq!(f2.bounds().y0, f3.bounds().y0);
        assert_eq!(f1.bounds().y1, f2.bounds().y1);
        // 3 columns: side-by-side from left to right (Col 1: Type < Col 2: Number < Col 3: Title)
        assert!(f1.bounds().x1 < f2.bounds().x0);
        assert!(f2.bounds().x1 < f3.bounds().x0);
        // Col 3 leaves space at the end before margin.x1
        let page_margin = spread.pages.last().unwrap().margin_rect();
        assert!(f3.bounds().x1 < page_margin.x1);

        // Col 1 is right-aligned
        let tf1 = f1.text_frame().expect("f1 should be a text frame");
        let story1 = active.doc.story(tf1.story).expect("story1 should exist");
        assert_eq!(story1.paras[0].para.align, Some(designcraft_doc::Align::Right));
    }

    #[test]
    fn test_table_from_csv_defaults_tick_column_numbers_and_footer() {
        let st = PubgenState::default();
        assert!(st.card2_first_row_header, "First row is header is on by default");
        assert!(st.card2_col_numbers, "Add column numbers is on by default");
        assert!(st.card2_table_footer, "Table footer is on by default");
    }

    #[test]
    fn test_execute_insert_table() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Header 1".into(), "Header 2".into()],
                vec!["Cell A1".into(), "Cell A2".into()],
                vec!["Cell B1".into(), "Cell B2".into()],
            ],
            card2_first_row_header: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with Horizontal Rules should succeed: {:?}", res);
    }

    #[test]
    fn test_execute_insert_table_at_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Generate pages with Card 1 (preset 0: 1 Row · 3 Columns)
        let st_card1 = PubgenState {
            card1_records: vec![vec!["Chapter".into(), "1".into(), "Introduction to Layout".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_range_from: 1,
            card1_range_to: 1,
            card1_preset: 0,
            card1_append: true,
            ..Default::default()
        };
        let res1 = execute_generate_pages(&mut app, &st_card1);
        assert!(res1.is_ok(), "generate_pages should succeed: {:?}", res1);

        // 2. Insert table with placement = 2 ("At page")
        let st_card2 = PubgenState {
            card2_matrix: vec![vec!["Item".into(), "Price".into()], vec!["Widget A".into(), "$10".into()], vec!["Widget B".into(), "$20".into()]],
            card2_first_row_header: true,
            card2_placement: 2, // At page
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };
        let res2 = execute_insert_table(&mut app, &st_card2);
        assert!(res2.is_ok(), "insert_table at page should succeed: {:?}", res2);

        // 3. Verify table frame position is below generated title
        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.last().unwrap();
        let page = spread.pages.last().unwrap();
        let margin = page.margin_rect();

        // Find the table frame (its story contains tables)
        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|st| !st.tables.is_empty()))
            .expect("Table item should exist on spread");

        // In preset 0, title row has row_y1 = margin.y0 + 60.0 and gap_below_title = 12.0
        // The table frame should start at margin.y0 + 60.0 + 12.0 = margin.y0 + 72.0
        assert_eq!(table_item.bounds().y0, margin.y0 + 72.0);
        assert_eq!(table_item.bounds().x0, margin.x0);
        assert_eq!(table_item.bounds().x1, margin.x1);
    }

    #[test]
    fn test_execute_insert_table_with_col_numbers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Item".into(), "Category".into(), "Price".into()],
                vec!["Paper".into(), "Stock".into(), "$20".into()],
                vec!["Pen".into(), "Tools".into(), "$10".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with col numbers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // Total rows should be 4 (Header + Col numbers + 2 data rows)
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.columns.len(), 3);

        // Row 0 is header
        assert_eq!(table.cell(0, 0).unwrap().text.text.trim(), "Item");
        assert_eq!(table.cell(0, 1).unwrap().text.text.trim(), "Category");
        assert_eq!(table.cell(0, 2).unwrap().text.text.trim(), "Price");

        // Row 1 is column numbers (1), (2), (3)
        assert_eq!(table.cell(1, 0).unwrap().text.text.trim(), "(1)");
        assert_eq!(table.cell(1, 1).unwrap().text.text.trim(), "(2)");
        assert_eq!(table.cell(1, 2).unwrap().text.text.trim(), "(3)");

        // Row 2 is data
        assert_eq!(table.cell(2, 0).unwrap().text.text.trim(), "Paper");
    }

    #[test]
    fn test_execute_insert_table_horizontal_rules() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Header A".into(), "Header B".into()],
                vec!["Data 1A".into(), "Data 1B".into()],
                vec!["Data 2A".into(), "Data 2B".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: false, // pin it off: this test wants plain interior data rows
            card2_placement: 1,        // New frame
            card2_style: 3,            // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with Horizontal Rules should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 4 rows: 0 (Header), 1 (Col numbers), 2 (Data 1), 3 (Data 2)
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.columns.len(), 2);

        // Verify table-level border is none
        assert_eq!(table.options.border.weight, 0.0);

        // Row 0: Table Header
        // Top border should be 1.0pt, bottom border should be 0.5pt, left/right 0.0pt
        let r0_c0 = table.cell(0, 0).unwrap();
        assert_eq!(r0_c0.strokes[0].weight, 1.0, "Header top border must be 1.0pt");
        assert_eq!(r0_c0.strokes[2].weight, 0.5, "Header bottom border must be 0.5pt");
        assert_eq!(r0_c0.strokes[1].weight, 0.0, "No vertical left border");
        assert_eq!(r0_c0.strokes[3].weight, 0.0, "No vertical right border");

        // Row 1: Col Numbers
        // Top border 0.5pt, bottom border 0.5pt, left/right 0.0pt
        let r1_c0 = table.cell(1, 0).unwrap();
        assert_eq!(r1_c0.strokes[0].weight, 0.5, "Col numbers top border must be 0.5pt");
        assert_eq!(r1_c0.strokes[2].weight, 0.5, "Col numbers bottom border must be 0.5pt");
        assert_eq!(r1_c0.strokes[1].weight, 0.0, "No vertical left border");
        assert_eq!(r1_c0.strokes[3].weight, 0.0, "No vertical right border");

        // Row 2: Data Row 1 (Interior data row)
        // Top border 0.5pt (from row 1 bottom), bottom border 0.0pt (no border between data rows!)
        let r2_c0 = table.cell(2, 0).unwrap();
        assert_eq!(r2_c0.strokes[0].weight, 0.5);
        assert_eq!(r2_c0.strokes[2].weight, 0.0, "Interior data row bottom must have no border");
        assert_eq!(r2_c0.strokes[1].weight, 0.0, "No vertical left border");
        assert_eq!(r2_c0.strokes[3].weight, 0.0, "No vertical right border");

        // Row 3: Data Row 2 (Bottom table row)
        // Top border 0.0pt, bottom border 1.0pt (bottom of table rule!)
        let r3_c0 = table.cell(3, 0).unwrap();
        assert_eq!(r3_c0.strokes[0].weight, 0.0, "Interior data row top must have no border");
        assert_eq!(r3_c0.strokes[2].weight, 1.0, "Bottom of table must have 1.0pt border");
        assert_eq!(r3_c0.strokes[1].weight, 0.0, "No vertical left border");
        assert_eq!(r3_c0.strokes[3].weight, 0.0, "No vertical right border");
    }

    #[test]
    fn test_execute_insert_table_horizontal_rules_no_col_numbers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Header A".into(), "Header B".into()],
                vec!["Data 1A".into(), "Data 1B".into()],
                vec!["Data 2A".into(), "Data 2B".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_table_footer: false, // pin it off: this test wants plain interior data rows
            card2_placement: 1,        // New frame
            card2_style: 3,            // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok());

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 3 rows: 0 (Header), 1 (Data 1), 2 (Data 2)
        assert_eq!(table.rows.len(), 3);

        // Row 0: Header (top 1.0, bottom 0.5)
        let r0 = table.cell(0, 0).unwrap();
        assert_eq!(r0.strokes[0].weight, 1.0);
        assert_eq!(r0.strokes[2].weight, 0.5);
        assert_eq!(r0.strokes[1].weight, 0.0);
        assert_eq!(r0.strokes[3].weight, 0.0);

        // Row 1: Data 1 (top 0.5, bottom 0.0)
        let r1 = table.cell(1, 0).unwrap();
        assert_eq!(r1.strokes[0].weight, 0.5);
        assert_eq!(r1.strokes[2].weight, 0.0);
        assert_eq!(r1.strokes[1].weight, 0.0);
        assert_eq!(r1.strokes[3].weight, 0.0);

        // Row 2: Data 2 (top 0.0, bottom 1.0)
        let r2 = table.cell(2, 0).unwrap();
        assert_eq!(r2.strokes[0].weight, 0.0);
        assert_eq!(r2.strokes[2].weight, 1.0);
        assert_eq!(r2.strokes[1].weight, 0.0);
        assert_eq!(r2.strokes[3].weight, 0.0);
    }

    #[test]
    fn test_execute_insert_table_auto_merged_headers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Item Name".into(), "Category".into(), "Item--Price".into(), "Item--Stock".into(), "Rating".into()],
                vec!["A4 Coated Art Paper".into(), "Print Stock".into(), "$24.50".into(), "150".into(), "4.8".into()],
                vec!["Pantone Color Bridge".into(), "Tools".into(), "$185.00".into(), "24".into(), "4.9".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with auto merged headers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 2 header rows + 2 data rows = 4 total rows
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.columns.len(), 5);

        // Header has 2 rows
        assert_eq!(table.header_rows(), 2);

        // Row 0 Col 0 ("Item Name"): vertical merge (row_span 2, col_span 1)
        let c00 = table.cell(0, 0).unwrap();
        assert_eq!(c00.row_span, 2);
        assert_eq!(c00.col_span, 1);
        assert_eq!(c00.text.text.trim(), "Item Name");
        assert_eq!(c00.vj, designcraft_doc::VerticalJustification::Center);
        assert_eq!(c00.text.paras[0].para.align, Some(designcraft_doc::Align::Center));

        // Row 0 Col 1 ("Category"): vertical merge (row_span 2, col_span 1)
        let c01 = table.cell(0, 1).unwrap();
        assert_eq!(c01.row_span, 2);
        assert_eq!(c01.col_span, 1);
        assert_eq!(c01.text.text.trim(), "Category");
        assert_eq!(c01.vj, designcraft_doc::VerticalJustification::Center);

        // Row 0 Col 2 ("Item"): horizontal merge across Col 2 and Col 3 (row_span 1, col_span 2)
        let c02 = table.cell(0, 2).unwrap();
        assert_eq!(c02.row_span, 1);
        assert_eq!(c02.col_span, 2);
        assert_eq!(c02.text.text.trim(), "Item");
        assert_eq!(c02.vj, designcraft_doc::VerticalJustification::Center);
        assert_eq!(c02.text.paras[0].para.align, Some(designcraft_doc::Align::Center));

        // Row 1 Col 2 ("Price"): sub-header cell
        let c12 = table.cell(1, 2).unwrap();
        assert_eq!(c12.row_span, 1);
        assert_eq!(c12.col_span, 1);
        assert_eq!(c12.text.text.trim(), "Price");
        assert_eq!(c12.vj, designcraft_doc::VerticalJustification::Center);

        // Row 1 Col 3 ("Stock"): sub-header cell
        let c13 = table.cell(1, 3).unwrap();
        assert_eq!(c13.row_span, 1);
        assert_eq!(c13.col_span, 1);
        assert_eq!(c13.text.text.trim(), "Stock");
        assert_eq!(c13.vj, designcraft_doc::VerticalJustification::Center);

        // Row 0 Col 4 ("Rating"): vertical merge (row_span 2, col_span 1)
        let c04 = table.cell(0, 4).unwrap();
        assert_eq!(c04.row_span, 2);
        assert_eq!(c04.col_span, 1);
        assert_eq!(c04.text.text.trim(), "Rating");
        assert_eq!(c04.vj, designcraft_doc::VerticalJustification::Center);

        // Row 2 is first data row
        assert_eq!(table.cell(2, 0).unwrap().text.text.trim(), "A4 Coated Art Paper");
        assert_eq!(table.cell(2, 1).unwrap().text.text.trim(), "Print Stock");
        assert_eq!(table.cell(2, 2).unwrap().text.text.trim(), "$24.50");
        assert_eq!(table.cell(2, 3).unwrap().text.text.trim(), "150");
        assert_eq!(table.cell(2, 4).unwrap().text.text.trim(), "4.8");
    }

    #[test]
    fn test_execute_insert_table_auto_merged_headers_with_col_numbers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Item Name".into(), "Category".into(), "Item--Price".into(), "Item--Stock".into(), "Rating".into()],
                vec!["A4 Coated Art Paper".into(), "Print Stock".into(), "$24.50".into(), "150".into(), "4.8".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with auto merged headers + col numbers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 2 header rows + 1 col numbers row + 1 data row = 4 total rows
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.columns.len(), 5);

        // Row 2 is column numbers row: (1), (2), (3), (4), (5)
        assert_eq!(table.cell(2, 0).unwrap().text.text.trim(), "(1)");
        assert_eq!(table.cell(2, 1).unwrap().text.text.trim(), "(2)");
        assert_eq!(table.cell(2, 2).unwrap().text.text.trim(), "(3)");
        assert_eq!(table.cell(2, 3).unwrap().text.text.trim(), "(4)");
        assert_eq!(table.cell(2, 4).unwrap().text.text.trim(), "(5)");

        // Row 3 is data row
        assert_eq!(table.cell(3, 0).unwrap().text.text.trim(), "A4 Coated Art Paper");
    }

    #[test]
    fn test_multiline_csv_header_cells_stay_in_one_table_row() {
        // Excel wraps long header cells; the generated CSV quotes the line break. It must not
        // become extra table rows when the CSV is converted to a table.
        let csv = concat!(
            "Desa/Kelurahan / Village/Subdistric,\"Luas / Total Area\n(km2 / sq.km)\",",
            "\"Persentase terhadap Luas Kecamatan\n/ Percentage to District Area\"\n",
            "Warungbambu,1.65,21.32\n",
        );
        let matrix = parse_csv_content(csv);
        assert_eq!(matrix.len(), 2);
        assert!(matrix[0][1].contains('\n'));

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: matrix,
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_placement: 1,
            card2_style: 3,
            ..Default::default()
        };
        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "multiline header table should insert: {res:?}");

        let active = app.session.active().unwrap();
        let table = active.doc.stories.values().find_map(|s| s.tables.values().next()).expect("Table should exist in story");
        assert_eq!(table.rows.len(), 2, "one header row + one data row");
        assert_eq!(table.header_rows(), 1);
        assert_eq!(table.cell(0, 1).unwrap().text.text.trim(), "Luas / Total Area\u{2028}(km2 / sq.km)", "the header keeps its Excel line break");
        assert_eq!(table.cell(0, 2).unwrap().text.text.trim(), "Persentase terhadap Luas Kecamatan\u{2028}/ Percentage to District Area");
    }

    #[test]
    fn test_execute_insert_table_3stack_headers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec![
                    "Item".into(),
                    "Category".into(),
                    "Item--Price".into(),
                    "Item--Stock".into(),
                    "Item--Location--Province".into(),
                    "Item--Location--Region".into(),
                    "Rating".into(),
                ],
                vec![
                    "A4 Coated Art Paper".into(),
                    "Print Stock".into(),
                    "$24.50".into(),
                    "150".into(),
                    "West Java".into(),
                    "Bekasi".into(),
                    "48".into(),
                ],
                vec![
                    "Pantone Color Bridge".into(),
                    "Tools".into(),
                    "$185.00".into(),
                    "24".into(),
                    "West Java".into(),
                    "Karawang".into(),
                    "49".into(),
                ],
            ],
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with 3-stack headers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 3 header rows + 2 data rows = 5 total rows
        assert_eq!(table.rows.len(), 5);
        assert_eq!(table.columns.len(), 7);
        assert_eq!(table.header_rows(), 3);

        // Col 0 ("Item"): 3-row vertical merge
        let c0 = table.cell(0, 0).unwrap();
        assert_eq!(c0.row_span, 3);
        assert_eq!(c0.col_span, 1);
        assert_eq!(c0.text.text.trim(), "Item");
        assert_eq!(c0.vj, designcraft_doc::VerticalJustification::Center);
        assert_eq!(c0.text.paras[0].para.align, Some(designcraft_doc::Align::Center));

        // Col 1 ("Category"): 3-row vertical merge
        let c1 = table.cell(0, 1).unwrap();
        assert_eq!(c1.row_span, 3);
        assert_eq!(c1.col_span, 1);
        assert_eq!(c1.text.text.trim(), "Category");
        assert_eq!(c1.vj, designcraft_doc::VerticalJustification::Center);

        // Row 0 Col 2 ("Item"): 4-column horizontal merge across Cols 2..=5
        let c2_r0 = table.cell(0, 2).unwrap();
        assert_eq!(c2_r0.row_span, 1);
        assert_eq!(c2_r0.col_span, 4);
        assert_eq!(c2_r0.text.text.trim(), "Item");
        assert_eq!(c2_r0.vj, designcraft_doc::VerticalJustification::Center);

        // Row 1 Col 2 ("Price"): 2-row vertical merge across Rows 1..=2
        let c2_r1 = table.cell(1, 2).unwrap();
        assert_eq!(c2_r1.row_span, 2);
        assert_eq!(c2_r1.col_span, 1);
        assert_eq!(c2_r1.text.text.trim(), "Price");
        assert_eq!(c2_r1.vj, designcraft_doc::VerticalJustification::Center);

        // Row 1 Col 3 ("Stock"): 2-row vertical merge across Rows 1..=2
        let c3_r1 = table.cell(1, 3).unwrap();
        assert_eq!(c3_r1.row_span, 2);
        assert_eq!(c3_r1.col_span, 1);
        assert_eq!(c3_r1.text.text.trim(), "Stock");
        assert_eq!(c3_r1.vj, designcraft_doc::VerticalJustification::Center);

        // Row 1 Col 4 ("Location"): 2-column horizontal merge across Cols 4..=5
        let c4_r1 = table.cell(1, 4).unwrap();
        assert_eq!(c4_r1.row_span, 1);
        assert_eq!(c4_r1.col_span, 2);
        assert_eq!(c4_r1.text.text.trim(), "Location");
        assert_eq!(c4_r1.vj, designcraft_doc::VerticalJustification::Center);

        // Row 2 Col 4 ("Province"): 1x1 leaf cell
        let c4_r2 = table.cell(2, 4).unwrap();
        assert_eq!(c4_r2.row_span, 1);
        assert_eq!(c4_r2.col_span, 1);
        assert_eq!(c4_r2.text.text.trim(), "Province");
        assert_eq!(c4_r2.vj, designcraft_doc::VerticalJustification::Center);

        // Row 2 Col 5 ("Region"): 1x1 leaf cell
        let c5_r2 = table.cell(2, 5).unwrap();
        assert_eq!(c5_r2.row_span, 1);
        assert_eq!(c5_r2.col_span, 1);
        assert_eq!(c5_r2.text.text.trim(), "Region");
        assert_eq!(c5_r2.vj, designcraft_doc::VerticalJustification::Center);

        // Col 6 ("Rating"): 3-row vertical merge
        let c6 = table.cell(0, 6).unwrap();
        assert_eq!(c6.row_span, 3);
        assert_eq!(c6.col_span, 1);
        assert_eq!(c6.text.text.trim(), "Rating");
        assert_eq!(c6.vj, designcraft_doc::VerticalJustification::Center);

        // Row 3 (first data row)
        assert_eq!(table.cell(3, 0).unwrap().text.text.trim(), "A4 Coated Art Paper");
        assert_eq!(table.cell(3, 1).unwrap().text.text.trim(), "Print Stock");
        assert_eq!(table.cell(3, 2).unwrap().text.text.trim(), "$24.50");
        assert_eq!(table.cell(3, 3).unwrap().text.text.trim(), "150");
        assert_eq!(table.cell(3, 4).unwrap().text.text.trim(), "West Java");
        assert_eq!(table.cell(3, 5).unwrap().text.text.trim(), "Bekasi");
        assert_eq!(table.cell(3, 6).unwrap().text.text.trim(), "48");
    }

    #[test]
    fn test_execute_insert_table_3stack_headers_with_col_numbers_and_horizontal_rules() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec![
                    "Item".into(),
                    "Category".into(),
                    "Item--Price".into(),
                    "Item--Stock".into(),
                    "Item--Location--Province".into(),
                    "Item--Location--Region".into(),
                    "Rating".into(),
                ],
                vec![
                    "A4 Coated Art Paper".into(),
                    "Print Stock".into(),
                    "$24.50".into(),
                    "150".into(),
                    "West Java".into(),
                    "Bekasi".into(),
                    "48".into(),
                ],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with 3-stack headers + col numbers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 3 header rows + 1 col numbers row + 1 data row = 5 total rows
        assert_eq!(table.rows.len(), 5);
        assert_eq!(table.columns.len(), 7);

        // Row 3 is column numbers row: (1), (2), (3), (4), (5), (6), (7)
        for c in 0..7 {
            assert_eq!(table.cell(3, c).unwrap().text.text.trim(), format!("({})", c + 1));
        }

        // Row 4 is first data row
        assert_eq!(table.cell(4, 0).unwrap().text.text.trim(), "A4 Coated Art Paper");

        // Check horizontal rules stroke weights:
        // Top of table (row 0, col 0)
        assert_eq!(table.cell(0, 0).unwrap().strokes[0].weight, 1.0);
        // Sub-header under "Item" (row 0, col 2)
        assert_eq!(table.cell(0, 2).unwrap().strokes[2].weight, 0.5);
        // Sub-header under "Location" (row 1, col 4)
        assert_eq!(table.cell(1, 4).unwrap().strokes[2].weight, 0.5);
        // Bottom of header (row 2, col 0)
        assert_eq!(table.cell(2, 0).unwrap().strokes[2].weight, 0.5);
        // Bottom of column numbers row (row 3, col 0)
        assert_eq!(table.cell(3, 0).unwrap().strokes[2].weight, 0.5);
        // Bottom of table (row 4, col 0)
        assert_eq!(table.cell(4, 0).unwrap().strokes[2].weight, 1.0);
    }

    #[test]
    fn test_detect_footnote_row() {
        let rows = vec![
            vec!["Header 1".into(), "Header 2".into()],
            vec!["Data 1".into(), "Data 2".into()],
            vec!["Sources: Karawang Region Government Office".into(), "".into()],
        ];
        assert_eq!(detect_footnote_row(&rows), Some("Sources: Karawang Region Government Office".to_string()));

        let rows_normal = vec![vec!["Header 1".into(), "Header 2".into()], vec!["Data 1".into(), "Data 2".into()]];
        assert_eq!(detect_footnote_row(&rows_normal), None);

        let rows_single_col = vec![vec!["Single 1".into()], vec!["Single 2".into()]];
        assert_eq!(detect_footnote_row(&rows_single_col), None);
    }

    #[test]
    fn test_execute_insert_table_footer_and_footnote() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Item".into(), "Category".into(), "Price".into()],
                vec!["Paper".into(), "Stock".into(), "$20".into()],
                vec!["Pen".into(), "Tools".into(), "$10".into()],
                vec!["Total".into(), "".into(), "$30".into()],
                vec!["Sources: Karawang Region Government Office".into(), "".into(), "".into()],
            ],
            card2_first_row_header: true,
            card2_table_footer: true,
            card2_col_numbers: false, // pin it off: the row indices below count from the header only
            card2_placement: 1,       // New frame
            card2_style: 3,           // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with footer and footnote should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        // 1. Verify table exists and has 4 rows (1 header + 2 data + 1 total) - NOT 5 rows!
        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.columns.len(), 3);

        // Row 3 is footer row ("Total")
        assert_eq!(table.cell(3, 0).unwrap().text.text.trim(), "Total");
        assert_eq!(table.cell(3, 2).unwrap().text.text.trim(), "$30");
        assert_eq!(table.rows[3].kind, designcraft_doc::RowKind::Footer);

        // Verify Horizontal Rules styling on footer row:
        // Top border of footer row (strokes[0]) is rule_thin (0.5)
        assert_eq!(table.cell(3, 0).unwrap().strokes[0].weight, 0.5);
        // Bottom border of preceding row (strokes[2]) is rule_thin (0.5)
        assert_eq!(table.cell(2, 0).unwrap().strokes[2].weight, 0.5);
        // Bottom border of footer row (strokes[2]) is rule_thick (1.0)
        assert_eq!(table.cell(3, 0).unwrap().strokes[2].weight, 1.0);

        // 2. Verify normal text box below table was created with footnote text
        let fn_item = spread
            .items
            .iter()
            .find(|it| {
                if it.id == table_item.id {
                    return false;
                }
                it.text_frame()
                    .and_then(|tf| active.doc.story(tf.story))
                    .is_some_and(|s| s.text.contains("Sources: Karawang Region Government Office"))
            })
            .expect("Footnote text frame should exist on spread");

        // Footnote frame is positioned below table frame
        assert!(fn_item.bounds().y0 >= table_item.bounds().y1);
    }

    #[test]
    fn test_execute_insert_table_3stack_sample_file_with_footer_and_footnote() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        // Load actual sample CSV file
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/sample_3stack_table.csv");
        if !std::path::Path::new(path).exists() {
            return;
        }
        load_card2_csv(&mut st, path);
        assert_eq!(st.card2_matrix.len(), 8);
        assert_eq!(detect_footnote_row(&st.card2_matrix), Some("Sources: Karawang Region Government Office".to_string()));

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with sample_3stack_table.csv should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 3 header rows + 1 col numbers row + 5 data rows + 1 total row = 10 rows
        assert_eq!(table.rows.len(), 10);
        assert_eq!(table.columns.len(), 7);

        // Row 9 is the Total row (footer)
        assert_eq!(table.cell(9, 0).unwrap().text.text.trim(), "Total");
        assert_eq!(table.cell(9, 2).unwrap().text.text.trim(), "$298.50");
        assert_eq!(table.cell(9, 3).unwrap().text.text.trim(), "359");
        assert_eq!(table.rows[9].kind, designcraft_doc::RowKind::Footer);

        // Footer borders: top is thin (0.5), bottom is thick (1.0)
        assert_eq!(table.cell(9, 0).unwrap().strokes[0].weight, 0.5);
        assert_eq!(table.cell(9, 0).unwrap().strokes[2].weight, 1.0);

        // Footnote frame exists below table
        let fn_item = spread
            .items
            .iter()
            .find(|it| {
                if it.id == table_item.id {
                    return false;
                }
                it.text_frame()
                    .and_then(|tf| active.doc.story(tf.story))
                    .is_some_and(|s| s.text.contains("Sources: Karawang Region Government Office"))
            })
            .expect("Footnote text frame should exist on spread");

        assert!(fn_item.bounds().y0 >= table_item.bounds().y1);
    }

    #[test]
    fn test_user_reproduction_at_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Generate pages with Card 1
        let st_card1 = PubgenState {
            card1_records: vec![
                vec!["Table".into(), "01.01".into(), "The Rise of Computational Layouts".into()],
                vec!["Table".into(), "01.02".into(), "Typography in the Digital Renaissance and Print Production in the 21st Century".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_preset: 0,
            card1_append: true,
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st_card1).unwrap();

        // 2. Insert table with Card 2 at page
        let mut st_card2 = PubgenState {
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_placement: 2, // At page
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/sample_3stack_table.csv");
        if !std::path::Path::new(path).exists() {
            return;
        }
        load_card2_csv(&mut st_card2, path);

        let res = execute_insert_table(&mut app, &st_card2);
        assert!(res.is_ok(), "insert_table should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.last().unwrap();
        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        for (i, row) in table.rows.iter().enumerate() {
            let col0 = table.cell(i, 0).map(|c| c.text.text.trim().to_string()).unwrap_or_default();
            println!("Row {i} (kind={:?}): col0 = '{col0}'", row.kind);
        }
        assert_eq!(table.rows.len(), 10);

        let cs = designcraft_compose::compose_story(&active.doc, tf.story, &Default::default());
        println!("Composed story overset: {:?}", cs.is_overset());
        assert!(!cs.is_overset(), "Story must not be overset! All rows must fit in table frame");
        if let Some(frame) = cs.frames.first() {
            println!("Frame content_height: {}", frame.content_height);
            if let Some(tfrag) = frame.tables.first() {
                println!("TableFrag rect: {:?}", tfrag.rect);
                println!("TableFrag placed cells count: {}", tfrag.cells.len());
                let placed_rows: std::collections::BTreeSet<_> = tfrag.cells.iter().map(|c| c.row).collect();
                println!("Placed rows in frame: {:?}", placed_rows);
                assert_eq!(placed_rows.len(), 10, "All 10 rows must be placed in the frame");
                for r in 0..10 {
                    assert!(placed_rows.contains(&r), "Row {r} should be placed in the frame");
                }
            }
        }
    }

    #[test]
    fn test_matches_pubgen_row_condition_keywords() {
        assert!(matches_pubgen_row_condition("Jumlah"));
        assert!(matches_pubgen_row_condition("JUMLAH / TOTAL"));
        assert!(matches_pubgen_row_condition("Jumlah:"));
        assert!(matches_pubgen_row_condition("Tahun 2005"));
        assert!(matches_pubgen_row_condition("TAHUN"));
        assert!(matches_pubgen_row_condition("Kab. Karawang"));
        assert!(matches_pubgen_row_condition("Kabupaten Karawang"));
        assert!(matches_pubgen_row_condition("Kec. Klari"));
        assert!(matches_pubgen_row_condition("Kecamatan Klari"));
        assert!(matches_pubgen_row_condition("- Jumlah Subtotal"));
        assert!(matches_pubgen_row_condition("Total"));
        assert!(matches_pubgen_row_condition("Sum"));
        assert!(matches_pubgen_row_condition("Subtotal"));
        assert!(matches_pubgen_row_condition("District 1"));
        assert!(matches_pubgen_row_condition("Subdistrict"));
        assert!(matches_pubgen_row_condition("Year 2026"));

        // Negative tests
        assert!(!matches_pubgen_row_condition("Kabel Listrik"));
        assert!(!matches_pubgen_row_condition("Kecap"));
        assert!(!matches_pubgen_row_condition("Tahunan"));
        assert!(!matches_pubgen_row_condition("Meninggal Dunia"));
        assert!(!matches_pubgen_row_condition(""));
    }

    #[test]
    fn test_execute_insert_table_row_conditions_kbda_2005() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_table_footer: false, // Notice table_footer is FALSE, row condition formats it!
            card2_row_conditions: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let content = "\
Keadaan Korban (Condition of Victims);Persentase 2005 (%);Korban 2005 (Orang);Persentase 2006 (%);Korban 2006 (Orang)
Meninggal Dunia (Died);8,96;10;42,54;77
Luka Berat (Badly Injured);28,89;33;34,81;63
Luka Ringan (Slightly Injured);62,15;70;22,65;41
JUMLAH / TOTAL;100,00;113;100,00;181";
        st.card2_delim = Some(detect_delimiter(content));
        st.card2_matrix = parse_csv_content(content);
        assert_eq!(st.card2_matrix.len(), 5);
        assert_eq!(st.card2_delim, Some(';'));

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with kbda_2005.csv should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        assert_eq!(table.rows.len(), 5);
        assert_eq!(table.columns.len(), 5);

        // Row 0: Header
        assert_eq!(table.cell(0, 0).unwrap().strokes[0].weight, 1.0, "Top of table rule 1.0pt");
        assert_eq!(table.cell(0, 0).unwrap().strokes[2].weight, 0.5, "Header bottom rule 0.5pt");

        // Row 1: Data row 1 ("Meninggal Dunia")
        assert_eq!(table.cell(1, 0).unwrap().strokes[0].weight, 0.5);
        assert_eq!(table.cell(1, 0).unwrap().strokes[2].weight, 0.0, "Interior data row unbordered");

        // Row 2: Data row 2 ("Luka Berat")
        assert_eq!(table.cell(2, 0).unwrap().strokes[0].weight, 0.0);
        assert_eq!(table.cell(2, 0).unwrap().strokes[2].weight, 0.0);

        // Row 3: Data row 3 ("Luka Ringan")
        assert_eq!(table.cell(3, 0).unwrap().strokes[0].weight, 0.0);
        // Bottom of Row 3 is top of Row 4 (0.5pt)!
        assert_eq!(table.cell(3, 0).unwrap().strokes[2].weight, 0.5);

        // Row 4: JUMLAH / TOTAL
        assert_eq!(table.cell(4, 0).unwrap().text.text.trim(), "JUMLAH / TOTAL");
        assert_eq!(table.cell(4, 0).unwrap().strokes[0].weight, 0.5, "Row 4 top border 0.5pt");
        assert_eq!(table.cell(4, 0).unwrap().strokes[2].weight, 1.0, "Row 4 bottom border 1.0pt (bottom of table)");
    }

    #[test]
    fn test_execute_insert_table_intermediate_condition_rows() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Kategori".into(), "Jumlah Kasus".into()],
                vec!["Tahun 2005".into(), "100".into()], // Condition row 1 (intermediate)
                vec!["Penyakit A".into(), "40".into()],
                vec!["Penyakit B".into(), "60".into()],
                vec!["Jumlah Subtotal".into(), "100".into()], // Condition row 2 (intermediate)
                vec!["Tahun 2006".into(), "150".into()],      // Condition row 3 (intermediate)
                vec!["Penyakit A".into(), "70".into()],
                vec!["Jumlah Total".into(), "250".into()], // Condition row 4 (last row)
            ],
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_table_footer: false,
            card2_row_conditions: true,
            card2_placement: 1,
            card2_style: 3, // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok());

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        assert_eq!(table.rows.len(), 8);

        // Row 1: "Tahun 2005" (intermediate condition row)
        assert_eq!(table.cell(1, 0).unwrap().strokes[0].weight, 0.5);
        assert_eq!(table.cell(1, 0).unwrap().strokes[2].weight, 0.5);

        // Row 2: "Penyakit A" (normal data row)
        assert_eq!(table.cell(2, 0).unwrap().strokes[0].weight, 0.5); // from row 1 bottom
        assert_eq!(table.cell(2, 0).unwrap().strokes[2].weight, 0.0); // unbordered

        // Row 4: "Jumlah Subtotal" (intermediate condition row)
        assert_eq!(table.cell(4, 0).unwrap().strokes[0].weight, 0.5);
        assert_eq!(table.cell(4, 0).unwrap().strokes[2].weight, 0.5);

        // Row 7: "Jumlah Total" (last row)
        assert_eq!(table.cell(7, 0).unwrap().strokes[0].weight, 0.5);
        assert_eq!(table.cell(7, 0).unwrap().strokes[2].weight, 1.0);
    }

    #[test]
    fn test_quick_actions_switch_facing_pages() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // New document defaults to facing pages (Double-sided)
        let initial_facing = app.session.active().unwrap().doc.settings.facing_pages;
        assert!(initial_facing, "New document should default to facing pages (Double-sided)");

        // Switch to Single-sided
        app.run("layout.documentSetup", json!({"facingPages": false})).unwrap();
        let single_facing = app.session.active().unwrap().doc.settings.facing_pages;
        assert!(!single_facing, "Document should now be Single-sided");

        // Switch back to Double-sided
        app.run("layout.documentSetup", json!({"facingPages": true})).unwrap();
        let double_facing = app.session.active().unwrap().doc.settings.facing_pages;
        assert!(double_facing, "Document should be back to Double-sided");
    }

    #[test]
    fn test_quick_actions_insert_blank_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let initial_count = app.session.active().unwrap().doc.page_count();
        assert_eq!(initial_count, 1);

        // Insert blank page after page 0 (Page 1)
        app.run("layout.pages.insert", json!({"after": 0, "count": 1})).unwrap();

        let updated_count = app.session.active().unwrap().doc.page_count();
        assert_eq!(updated_count, 2);

        // Insert another page after page 0
        app.run("layout.pages.insert", json!({"after": 0, "count": 1})).unwrap();
        assert_eq!(app.session.active().unwrap().doc.page_count(), 3);
    }

    #[test]
    fn test_quick_actions_header_footer_and_table_options() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Header/Footer quick action opens headerFooter dialog
        crate::header_footer::open(&mut app).unwrap();
        assert_eq!(app.ui.dialog.as_ref().unwrap().id, "headerFooter");
        app.ui.dialog = None;

        // 2. Table Options quick action opens tableOptions dialog
        crate::dialogs::open_table_options(&mut app).unwrap();
        assert_eq!(app.ui.dialog.as_ref().unwrap().id, "tableOptions");
        assert_eq!(app.ui.dialog.as_ref().unwrap().b("inTable"), false);
    }

    #[test]
    fn test_toc_line_preset_writes_one_styled_paragraph_per_record() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Grafik".into(), "3.1.".into(), "Piramida Penduduk".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 5,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "the TOC Line preset should succeed: {res:?}");

        // One combined paragraph per record, not three separate frames.
        let active = app.session.active().unwrap();
        let mut lines: Vec<&str> =
            active.doc.stories.values().filter(|s| s.paras.first().is_some_and(|p| p.style == "TOC Entry")).map(|s| s.text.trim()).collect();
        lines.sort_unstable();
        assert_eq!(lines, ["Grafik 3.1. Piramida Penduduk", "Tabel 01.01. Jarak Antar Kecamatan"], "one styled paragraph per record");
    }

    #[test]
    fn test_add_toc_on_selected_page_collects_toc_line_entries() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Tabel".into(), "01.02.".into(), "Curah Hujan".into()],
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 5,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        // The same calls the Quick Actions "Add TOC (selected page)" button makes.
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        ensure_para_style(&mut app, &st.toc_style);
        let page = app.session.active().unwrap().doc.page_count();
        let res = app.run(
            "toc.generate",
            json!({"page": page, "title": "Contents",
                   "entries": [{"style": "TOC Entry", "level": 1, "pageNumber": "after"}],
                   "replace": true}),
        );
        assert!(res.is_ok(), "toc.generate should succeed: {res:?}");

        let sid = designcraft_doc::StoryId(res.unwrap()["story"].as_u64().unwrap());
        let text = app.session.active().unwrap().doc.story(sid).unwrap().text.clone();
        assert!(text.contains("Contents"), "{text}");
        assert!(text.contains("Jarak Antar Kecamatan"), "one line per record: {text}");
        assert!(text.contains("Curah Hujan"), "{text}");
    }

    #[test]
    fn test_default_preset_title_is_tagged_and_listed_in_toc() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        // The default preset still writes three frames, but only the title carries the TOC style.
        assert_eq!(toc_entry_count(&mut app, "TOC Entry"), 1, "the title paragraph is a TOC entry");

        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let page = app.session.active().unwrap().doc.page_count();
        let res = app.run(
            "toc.generate",
            json!({"page": page, "title": "Contents",
                   "entries": [{"style": "TOC Entry", "level": 1, "pageNumber": "after"}],
                   "replace": true}),
        );
        assert!(res.is_ok(), "toc.generate should succeed: {res:?}");

        let sid = designcraft_doc::StoryId(res.unwrap()["story"].as_u64().unwrap());
        let text = app.session.active().unwrap().doc.story(sid).unwrap().text.clone();
        assert!(text.contains("Jarak Antar Kecamatan"), "the generated page title is listed: {text}");
    }

    #[test]
    fn test_execute_generate_pages_separate_textboxes_title_only() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec![
                "Tabel".into(),
                "02.01".into(),
                "Jumlah Rukun Warga (RW) dan Rukun Tetangga (RT) Menurut Desa/Kelurahan di Kecamatan Karawang Timur, 2025".into(),
            ]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        // Check the 3 text frames created on the generated spread
        {
            let active = app.session.active().unwrap();
            let spread = active.doc.spreads.last().unwrap();
            let items = &spread.items;
            let n = items.len();
            assert!(n >= 3);
            let f1 = items[n - 3].text_frame().unwrap();
            let f2 = items[n - 2].text_frame().unwrap();
            let f3 = items[n - 1].text_frame().unwrap();

            let s1 = active.doc.story(f1.story).unwrap();
            let s2 = active.doc.story(f2.story).unwrap();
            let s3 = active.doc.story(f3.story).unwrap();

            // 1st textbox contains the localized "Tabel"
            assert_eq!(s1.text.trim(), "Tabel");
            // 2nd textbox contains: "02.01" (correct value)
            assert_eq!(s2.text.trim(), "02.01");
            // 3rd textbox contains ONLY title, NOT including type and number!
            assert_eq!(s3.text.trim(), "Jumlah Rukun Warga (RW) dan Rukun Tetangga (RT) Menurut Desa/Kelurahan di Kecamatan Karawang Timur, 2025");
        }

        // When Add TOC is executed, verify the generated TOC entry merges type + number + title:
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc_story = active.doc.story(active.doc.tocs.last().unwrap().story).unwrap();
        let clean_text = toc_story.text.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
        assert!(
            clean_text
                .contains("Tabel 02.01 Jumlah Rukun Warga (RW) dan Rukun Tetangga (RT) Menurut Desa/Kelurahan di Kecamatan Karawang Timur, 2025"),
            "TOC should contain merged type + number + title: {}",
            toc_story.text
        );
    }

    #[test]
    fn test_execute_add_toc_kbda_row_merges_type_number_title() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Row #2 from KBDA2026_chapters.csv:
        // Tabel;01.01.;Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006
        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into()],
                vec!["Tabel".into(), "01.02.".into(), "Keadaan Curah Hujan Tiap Bulan menurut Kecamatan (mm), 2006".into()],
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            card1_toc_prefix: false, // Even when false in page generation, execute_add_toc merges them!
            toc_style: "TOC Entry".into(),
            toc_hanging_indent: true,
            ..Default::default()
        };

        // Generate pages
        execute_generate_pages(&mut app, &st).unwrap();

        // Now insert TOC page and run execute_add_toc
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("Document TOC should be populated");
        let story = active.doc.story(toc.story).expect("TOC story should exist");
        let text = &story.text;

        assert!(
            text.contains("Tabel 01.01. Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"),
            "TOC should contain merged type + number + title: {text}"
        );
        assert!(
            text.contains("Tabel 01.02. Keadaan Curah Hujan Tiap Bulan menurut Kecamatan (mm), 2006"),
            "TOC should contain second merged record: {text}"
        );
    }

    #[test]
    fn test_execute_add_toc_hanging_indent_applied() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            card1_toc_prefix: false,
            toc_style: "TOC Entry".into(),
            toc_hanging_indent: true,
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        execute_add_toc(&mut app, &mut st).unwrap();

        let active = app.session.active().unwrap();
        let style = active.doc.styles.para("TOC Level 1").expect("TOC Level 1 style should exist");
        assert_eq!(style.para.left_indent, Some(78.0), "Hanging indent left_indent");
        assert_eq!(style.para.first_line_indent, Some(-78.0), "Hanging indent first_line_indent");

        let tab = style.para.tabs.as_ref().and_then(|t| t.first()).expect("Tab stop should exist");
        assert_eq!(tab.align, designcraft_doc::TabAlign::Right);
        assert_eq!(tab.leader, ". ");
    }

    #[test]
    fn test_execute_add_toc_fallback_from_page_frames() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            card1_toc_prefix: false,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();

        // Now simulate empty card1_records (e.g. session without CSV in memory)
        let mut st_empty = PubgenState { card1_records: Vec::new(), toc_style: "TOC Entry".into(), ..Default::default() };

        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let res = execute_add_toc(&mut app, &mut st_empty);
        assert!(res.is_ok(), "execute_add_toc with fallback should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = &active.doc.story(toc.story).unwrap().text;

        assert!(
            text.contains("Tabel 01.01 Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"),
            "Fallback should extract type and number from page frames: {text}"
        );
    }

    #[test]
    fn test_execute_add_toc_ignores_footnote_and_source_text_frames() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            card1_toc_prefix: true, // Page generated with prefix
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();

        // Simulate Table from CSV creating a footnote text frame on that page
        let spread_idx = 0;
        let _ = app.run(
            "frame.create",
            json!({
                "spread": spread_idx,
                "rect": [72.0, 500.0, 400.0, 530.0],
                "content": "text",
                "text": "Sources: Karawang Region Government Office",
                "caret": false
            }),
        );

        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = &active.doc.story(toc.story).unwrap().text;

        // Must NOT contain footnote text
        assert!(!text.to_lowercase().contains("sources"), "TOC must not incorporate footnote/source text: {text}");
        assert!(text.contains("Tabel 01.01. Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"), "TOC must contain clean entry: {text}");
    }

    #[test]
    fn test_execute_add_toc_generates_continuation_pages_when_overflows() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 80 entries - definitely cannot fit in 1 page (typical page holds ~35 entries)
        let records: Vec<Vec<String>> =
            (1..=80).map(|i| vec!["Tabel".into(), format!("01.{i:02}."), format!("Dataset Observation Record Number {i} in Region")]).collect();

        let mut st = PubgenState {
            card1_records: records,
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 5, // TOC Line preset
            toc_style: "TOC Entry".into(),
            toc_hanging_indent: true,
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();

        let initial_pages = app.session.active().unwrap().doc.page_count();
        // Insert TOC starting page
        app.run("layout.pages.insert", json!({"after": 0, "count": 1})).unwrap();
        crate::canvas::go_to_page(&mut app, 1);

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story_id = toc.story;

        // Verify that the TOC story has threaded across multiple frames/pages
        let frames_count = active.doc.story(story_id).map(|s| s.frames.len()).unwrap_or(0);
        assert!(frames_count > 1, "TOC should span more than 1 frame: {frames_count}");

        let current_pages = active.doc.page_count();
        assert!(
            current_pages > initial_pages + 1,
            "Continuation pages should be added to hold overflowed TOC (initial: {initial_pages}, current: {current_pages})"
        );

        // Verify the story is NOT overset
        let cs = designcraft_compose::compose_story(&active.doc, story_id, &Default::default());
        assert!(cs.overset_at.is_none(), "TOC should not be overset after autoflow");
    }

    #[test]
    fn test_wrap_toc_title_wraps_early_before_page_number() {
        let width = 440.0;
        let hanging = 68.0;

        // 1. Short title fits comfortably on line 0
        let short = "Tabel 01.01. Short Title, 2006";
        let wrapped_short = wrap_toc_title(short, width, hanging, Some("12"));
        assert_eq!(wrapped_short, short);

        // 2. Long title (Grafik 4.6. from user request) wraps early with \u{2028}
        let long = "Grafik 4.6. Banyaknya Tenaga Medis/Dokter Menurut Keahlian di Kabupaten Karawang, 2006";
        let wrapped_long = wrap_toc_title(long, width, hanging, Some("44"));
        assert!(wrapped_long.contains(designcraft_doc::story::FORCED_LINE_BREAK), "Title should be wrapped with FORCED_LINE_BREAK");
        let parts: Vec<&str> = wrapped_long.split(designcraft_doc::story::FORCED_LINE_BREAK).collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].starts_with("Grafik 4.6. Banyaknya"));
        // First line should wrap early (less than width - 48 - page_w)
        assert!(estimate_text_advance(parts[0]) <= width - 48.0 - 15.0);
        // Second line contains the remaining words
        assert!(parts[1].ends_with("2006"));
    }

    #[test]
    fn test_execute_add_toc_warps_early_and_indents_second_line() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let records = vec![
            vec!["Grafik".into(), "4.6.".into(), "Banyaknya Tenaga Medis/Dokter Menurut Keahlian di Kabupaten Karawang, 2006".into()],
            vec![
                "Tabel".into(),
                "04.01.".into(),
                "Banyaknya Sekolah, Murid dan Guru Taman Kanak-Kanak Menurut Status Sekolah Tiap Kecamatan, 2006".into(),
            ],
        ];

        let mut st = PubgenState {
            card1_records: records,
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            toc_hanging_indent: true,
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        crate::canvas::go_to_page(&mut app, 1);

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story_id = toc.story;
        let story = active.doc.story(story_id).unwrap();

        // Both long titles should have wrapped lines via \u{2028}
        assert!(
            story.text.contains(designcraft_doc::story::FORCED_LINE_BREAK),
            "Story text should contain soft line breaks for early wrapping: {}",
            story.text
        );

        // Compose the TOC story to verify layout geometry
        let cs = designcraft_compose::compose_story(&active.doc, story_id, &Default::default());
        assert!(!cs.frames.is_empty(), "Frames should exist");
        let lines = &cs.frames[0].lines;
        assert!(lines.len() >= 4, "Should compose into multiple lines: {}", lines.len());

        // Check style indentation
        let style = active.doc.styles.para("TOC Level 1").unwrap();
        let left_ind = style.para.left_indent.unwrap();
        let first_ind = style.para.first_line_indent.unwrap();
        assert!(left_ind >= 68.0, "Second line indent should be deep (closer to first char of title): {left_ind}");
        assert_eq!(first_ind, -left_ind, "First line indent must balance left indent");
    }

    #[test]
    fn test_load_card1_csv_restricts_to_first_three_columns() {
        let temp_dir = std::env::temp_dir();
        let csv_path = temp_dir.join("test_extra_cols.csv");
        let csv_content = "type;number;title;file;notes\n\
                           Tabel;01.01.;Jarak Antar Kecamatan;data1.csv;extra info\n\
                           Grafik;02.01.;Distribusi Penduduk;data2.csv;sample note\n";
        std::fs::write(&csv_path, csv_content).unwrap();

        let mut st = PubgenState::default();
        load_card1_csv(&mut st, csv_path.to_str().unwrap());

        // Column indexes must be strictly restricted to first 3 columns
        assert_eq!(st.card1_type_col, 0);
        assert_eq!(st.card1_num_col, 1);
        assert_eq!(st.card1_title_col, 2);

        // Headers and records keep extra columns (e.g. column 3 for 'file')
        assert_eq!(st.card1_headers.len(), 5);
        assert_eq!(st.card1_records.len(), 2);
        assert_eq!(st.card1_records[0].get(0..3).unwrap(), &["Tabel", "01.01.", "Jarak Antar Kecamatan"]);
        assert_eq!(st.card1_records[0].get(3).map(String::as_str), Some("data1.csv"));
        assert_eq!(st.card1_records[1].get(0..3).unwrap(), &["Grafik", "02.01.", "Distribusi Penduduk"]);
        assert_eq!(st.card1_records[1].get(3).map(String::as_str), Some("data2.csv"));

        let _ = std::fs::remove_file(csv_path);
    }

    #[test]
    fn test_execute_excel_to_csvs_populates_card1_and_card2() {
        let sample_xlsx = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("plan").join("pubgen").join("samples").join("BAB_02_2025.xlsx"))
            .unwrap_or_default();
        if !sample_xlsx.exists() {
            return;
        }

        // The converter writes its CSVs beside the workbook, so convert a copy in a scratch folder:
        // the sample folder holds tracked CSVs that a test run must not rewrite.
        let dir = std::env::temp_dir().join(format!("dc-pubgen-excel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let workbook = dir.join("BAB_02_2025.xlsx");
        std::fs::copy(&sample_xlsx, &workbook).unwrap();

        let mut st = PubgenState::default();
        let res = execute_excel_to_csvs(&mut st, workbook.to_str().unwrap());
        assert!(res.is_ok(), "Conversion should succeed: {res:?}");

        // Card 1 must have catalog loaded
        assert!(st.card1_path.ends_with("BAB_02_2025_catalog.csv"));
        assert_eq!(st.card1_records.len(), 6, "6 catalog rows in BAB_02_2025.xlsx: DIV, R02, 3 tables, C02_01");
        assert_eq!(st.card1_records.first().and_then(|r| r.first()), Some(&"Divider".to_string()), "the DIV sheet becomes the first catalog row");
        assert!(st.card1_headers.contains(&"file".to_string()));

        // Card 2 must have first table loaded and sheet table list populated
        assert!(st.card2_path.ends_with("02_01.csv"));
        assert_eq!(st.card2_sheet_tables.len(), 3, "Only the 3 tabular sheets produce table CSVs");
        assert_eq!(st.card2_sheet_idx, 0);
        assert!(!st.card2_matrix.is_empty());

        // ZIP metadata and bytes must be cached
        assert!(!st.generated_zip_bytes.is_empty(), "ZIP archive bytes should be cached in state");
        assert!(st.generated_zip_name.ends_with(".zip"), "ZIP archive name should end with .zip");
        assert_eq!(st.generated_csv_files.len(), 4, "Catalog + 3 tables in generated_csv_files");

        // The workbook has no COVER sheet, so the cover fields keep their defaults.
        assert!(!st.excel_had_cover, "this workbook has no COVER sheet");
        assert_eq!(st.cover_title, default_cover_title());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_execute_excel_and_generate_auto_runs_pages_and_tables() {
        let sample_xlsx = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("plan").join("pubgen").join("samples").join("BAB_02_2025.xlsx"))
            .unwrap_or_default();
        if !sample_xlsx.exists() {
            return;
        }

        let dir = std::env::temp_dir().join(format!("dc-pubgen-excel-autogen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let workbook = dir.join("BAB_02_2025.xlsx");
        std::fs::copy(&sample_xlsx, &workbook).unwrap();
        // The workbook's `file` cells carry paths from the machine that authored it; copy the
        // sidecar assets beside it so the file-name fallback resolves them here too.
        for sidecar in ["C02_01.png", "infografik_01.png", "ulasan_bab_1.rtf", "A02.rtf", "R02.rtf"] {
            let src = sample_xlsx.parent().unwrap().join(sidecar);
            if src.is_file() {
                std::fs::copy(&src, dir.join(sidecar)).unwrap();
            }
        }

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState::default();
        assert!(st.excel_auto_generate, "excel_auto_generate should be true by default");

        let res = execute_excel_and_generate(&mut app, &mut st, workbook.to_str().unwrap());
        assert!(res.is_ok(), "execute_excel_and_generate should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("auto-generated pages"), "Status should confirm auto-generated pages: {msg}");
        assert!(msg.contains("3 tables auto-inserted"), "Status should confirm 3 tables auto-inserted: {msg}");
        assert!(msg.contains("1 RTF chapter(s) placed"), "Status should confirm 1 RTF placed (R02 ulasan): {msg}");
        assert!(msg.contains("2 chart image(s) placed"), "Status should confirm 2 images placed (DIV divider + C02_01 chart): {msg}");
        assert!(!msg.contains("cover"), "This workbook has no COVER sheet, so no cover page: {msg}");

        let page_texts = |page: usize| -> Vec<String> {
            let doc = &app.session.active().unwrap().doc;
            doc.spreads[doc.page_loc(page).unwrap().0]
                .items
                .iter()
                .filter(|it| doc.page_of_item(it.id) == Some(page))
                .filter_map(|it| it.text_frame())
                .map(|tf| doc.stories.get(&tf.story).map(|s| s.text.clone()).unwrap_or_default())
                .collect()
        };

        let doc = &app.session.active().unwrap().doc;
        assert_eq!(doc.page_count(), 8, "6 sheet records + 1 RTF autoflow continuation page, no cover");

        // Page 1: the DIV divider (full-bleed, no margins).
        let divider = page_texts(1);
        assert!(divider.iter().any(|t| t.contains("BAB")), "Page 1 should contain Divider prefix: {divider:?}");
        assert!(divider.iter().any(|t| t.contains("PEMERINTAHAN")), "Page 1 should contain Divider title: {divider:?}");
        assert_eq!(doc.spreads[doc.page_loc(1).unwrap().0].pages[doc.page_loc(1).unwrap().1].margins.top, 0.0, "DIV page must have 0 margins");

        // Page 2: the R02 Ulasan chapter, then its autoflow continuation.
        let ulasan = page_texts(2);
        assert!(ulasan.iter().any(|t| t.contains("BAB 02")), "Page 2 should contain the chapter line: {ulasan:?}");
        assert!(ulasan.iter().any(|t| t.contains("Pemerintahan")), "Page 2 should contain the chapter title: {ulasan:?}");
        assert!(ulasan.iter().any(|t| t.contains("ULASAN")), "Page 2 should contain the RTF narrative: {ulasan:?}");

        // Content pages follow the document margins (BPS catalog).
        let pt_per_mm = 72.0 / 25.4;
        let expected_top = 20.20 * pt_per_mm;
        let expected_inside = 19.0 * pt_per_mm;
        let expected_outside = 15.0 * pt_per_mm;

        let (si2, pi2) = doc.page_loc(2).unwrap();
        let ulasan_pg = &doc.spreads[si2].pages[pi2];
        assert!((ulasan_pg.margins.top - expected_top).abs() < 0.1, "Ulasan page top margin must follow doc margin");
        assert!((ulasan_pg.margins.inside - expected_inside).abs() < 0.1, "Ulasan page inside margin must follow doc margin");
        assert!((ulasan_pg.margins.outside - expected_outside).abs() < 0.1, "Ulasan page outside margin must follow doc margin");

        let ulasan_margin = ulasan_pg.margin_rect();
        let ulasan_items: Vec<_> = doc.spreads[si2].items.iter().filter(|it| doc.page_of_item(it.id) == Some(2)).collect();
        assert!(ulasan_items.iter().any(|it| (it.bounds().x0 - ulasan_margin.x0).abs() < 1.0), "Ulasan item should start at left margin");
        for it in &ulasan_items {
            let ib = it.bounds();
            assert!(ib.x0 >= ulasan_margin.x0 - 1.0, "Item left {} must align within margin {}", ib.x0, ulasan_margin.x0);
            assert!(ib.x1 <= ulasan_margin.x1 + 1.0, "Item right {} must not exceed margin {}", ib.x1, ulasan_margin.x1);
        }

        // Verify Table page (Page 4: Tabel 02.01) follows page margins
        let (si6, pi6) = doc.page_loc(4).unwrap();
        let tbl_pg = &doc.spreads[si6].pages[pi6];
        assert!((tbl_pg.margins.top - expected_top).abs() < 0.1);
        let tbl_margin = tbl_pg.margin_rect();
        let tbl_items: Vec<_> = doc.spreads[si6].items.iter().filter(|it| doc.page_of_item(it.id) == Some(4)).collect();
        assert!(tbl_items.iter().any(|it| (it.bounds().x0 - tbl_margin.x0).abs() < 1.0), "Table item should start at left margin");
        for it in &tbl_items {
            let ib = it.bounds();
            assert!(ib.x0 >= tbl_margin.x0 - 1.0, "Item left {} must align within margin {}", ib.x0, tbl_margin.x0);
            assert!(ib.x1 <= tbl_margin.x1 + 1.0, "Item right {} must not exceed margin {}", ib.x1, tbl_margin.x1);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_excel_g02_page_keeps_the_rtf_abbreviation_list_hanging_indent() {
        // The `G02` sheet only links `G02.rtf`, whose entries are Word hanging indents
        // (`\li1710\fi-1710\tx1530`): "SI\t:\tStasiun …". The tab after the colon has no explicit
        // stop past it, so it must reach the hanging indent (85.5 pt), where wrapped lines start,
        // and not the next default half-inch stop — otherwise every wrapped line sits to the left
        // of the definition it belongs to.
        let sample_xlsx = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("plan").join("pubgen").join("samples").join("SINGLE_LANG").join("BAB_01_2025.xlsx"))
            .unwrap_or_default();
        if !sample_xlsx.exists() || !sample_xlsx.parent().is_some_and(|dir| dir.join("G02.rtf").is_file()) {
            return;
        }

        let dir = std::env::temp_dir().join(format!("dc-pubgen-g02-indent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let workbook = dir.join("BAB_01_2025.xlsx");
        std::fs::copy(&sample_xlsx, &workbook).unwrap();
        // The workbook's `file` cells carry the authoring machine's paths; place the sidecars
        // beside the copy so its file-name fallback finds them here.
        if let Some(samples) = sample_xlsx.parent() {
            for sidecar in ["A02.rtf", "B01.rtf", "B01_pict1.jpg", "B01_pict2.png", "G00.rtf", "G01.rtf", "G02.rtf", "R01.rtf"] {
                if samples.join(sidecar).is_file() {
                    let _ = std::fs::copy(samples.join(sidecar), dir.join(sidecar));
                }
            }
        }

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let mut st = PubgenState::default();
        let converted = execute_excel_to_csvs(&mut st, workbook.to_str().unwrap());
        assert!(converted.is_ok(), "the workbook converts to CSV data: {converted:?}");

        // Keep only the G02 page so the test does not need every other sheet's assets.
        let g02 = st
            .card1_records
            .iter()
            .position(|row| row.get(3).is_some_and(|file| file.to_lowercase().ends_with("g02.rtf")))
            .expect("the G02 sheet becomes a catalog row");
        st.card1_records = vec![st.card1_records[g02].clone()];
        st.card1_range_all = true;
        st.card1_auto_insert_tables = false;
        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "Page from CSV generates the G02 page: {res:?}");

        let active = app.session.active().unwrap();
        let sid = active
            .doc
            .stories
            .iter()
            .find(|(_, story)| story.text.contains("Stasiun Meterologi") && story.text.contains("SMPK"))
            .map(|(id, _)| *id)
            .expect("the G02 RTF is placed as a page");
        let (smpk, description) = {
            let story = &active.doc.stories[&sid];
            (story.text.find("SMPK").expect("SMPK"), story.text.find("Stasiun Meterologi").expect("its definition"))
        };
        let para = active.doc.stories[&sid].para_at(smpk);
        let cs = designcraft_compose::compose_story(&active.doc, sid, &Default::default());
        let lines: Vec<_> = cs.frames.iter().flat_map(|frame| frame.lines.iter().map(move |line| (frame, line))).collect();
        let (frame, first) = lines.iter().find(|(_, line)| line.para == para && line.first_in_para).copied().expect("the SMPK line");
        let col = frame.columns[first.column as usize];
        let starts_at = |line: &designcraft_compose::Line| line.glyphs.iter().find(|g| g.len > 0 && g.visible).map(|g| g.x - col.x0).expect("text");
        let description_x = first.glyphs.iter().find(|g| g.byte == description).map(|g| g.x - col.x0).expect("the definition text");
        assert!((description_x - 85.5).abs() < 0.6, "the definition starts at the hanging indent, not {description_x}");
        let continuations: Vec<_> = lines.iter().filter(|(_, line)| line.para == para && !line.first_in_para).map(|(_, line)| *line).collect();
        assert!(!continuations.is_empty(), "SMPK's definition wraps");
        for line in continuations {
            let x = starts_at(line);
            assert!((x - 85.5).abs() < 0.6, "the wrapped line starts with its definition at 85.5, not {x}");
        }
        assert!((starts_at(first)).abs() < 0.6, "the abbreviation stays at the margin");

        // The last two rows (`TDP`, `API`) carry a far `\tx4000` stop besides `\tx1530`; their
        // definitions must still start at the hanging indent, like the rows above them.
        for (abbreviation, definition) in [("TDP", "Tanda Daftar Perusahaan"), ("API", "Angka Pengenal Importir")] {
            let story = &active.doc.stories[&sid];
            let label = story.text.find(abbreviation).expect("the abbreviation");
            let para = story.para_at(label);
            let definition_byte = story.text.find(definition).expect("its definition");
            let (frame, line) =
                lines.iter().find(|(_, line)| line.para == para && line.first_in_para).copied().expect("the abbreviation's first line");
            let col = frame.columns[line.column as usize];
            let x = line.glyphs.iter().find(|g| g.byte == definition_byte && g.len > 0).map(|g| g.x - col.x0).expect("the definition text");
            assert!((x - 85.5).abs() < 0.6, "{abbreviation}'s definition starts at the hanging indent, not {x}");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_execute_excel_and_generate_when_disabled_only_converts() {
        let sample_xlsx = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("plan").join("pubgen").join("samples").join("BAB_02_2025.xlsx"))
            .unwrap_or_default();
        if !sample_xlsx.exists() {
            return;
        }

        let dir = std::env::temp_dir().join(format!("dc-pubgen-excel-noautogen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let workbook = dir.join("BAB_02_2025.xlsx");
        std::fs::copy(&sample_xlsx, &workbook).unwrap();

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState { excel_auto_generate: false, ..Default::default() };

        let res = execute_excel_and_generate(&mut app, &mut st, workbook.to_str().unwrap());
        assert!(res.is_ok(), "Conversion should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("Converted 3 sheets"), "Message should mention converted sheets: {msg}");

        // Page count should remain 1 because auto-generate was false
        let page_count = app.session.active().unwrap().doc.page_count();
        assert_eq!(page_count, 1, "Page count should remain 1 when excel_auto_generate is false");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_execute_download_csv_zip_workflow() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = std::env::temp_dir().join("pubgen_zip_test_workflow");
        let _ = std::fs::create_dir_all(&temp_dir);

        let cat_path = temp_dir.join("test_catalog.csv");
        let cat_content = "type,number,title,file\nTabel,01.01,Title 1,01_01\n";
        std::fs::write(&cat_path, cat_content).unwrap();

        let tbl_path = temp_dir.join("01_01.csv");
        let tbl_content = "Col 1,Col 2\nVal 1,Val 2\n";
        std::fs::write(&tbl_path, tbl_content).unwrap();

        let mut st = PubgenState::default();
        load_card1_csv(&mut st, cat_path.to_str().unwrap());
        st.card2_sheet_tables = vec![tbl_path.to_str().unwrap().to_string()];

        // Configure mock pick_save to save to a destination path
        let save_dest = temp_dir.join("downloaded.zip");
        let dest_clone = save_dest.to_str().unwrap().to_string();
        app.services.pick_save = Some(Box::new(move |_| Some(dest_clone.clone())));

        let res = execute_download_csv_zip(&mut app, &mut st);
        assert!(res.is_ok(), "execute_download_csv_zip should succeed: {res:?}");
        assert!(save_dest.exists(), "Downloaded ZIP archive should exist at save_dest");

        // Verify ZIP entries inside downloaded.zip
        let f = std::fs::File::open(&save_dest).unwrap();
        let mut archive = zip::ZipArchive::new(f).unwrap();
        assert_eq!(archive.len(), 2);
        assert!(archive.by_name("test_catalog.csv").is_ok());
        assert!(archive.by_name("01_01.csv").is_ok());

        // Cleanup
        let _ = std::fs::remove_file(cat_path);
        let _ = std::fs::remove_file(tbl_path);
        let _ = std::fs::remove_file(save_dest);
        let _ = std::fs::remove_dir(temp_dir);
    }

    #[test]
    fn test_execute_download_csv_zip_via_download_service() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let downloaded_data = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let d_clone = downloaded_data.clone();
        app.services.download = Some(Box::new(move |_name, bytes| {
            let mut guard = d_clone.lock().unwrap();
            *guard = bytes.to_vec();
            Ok(())
        }));

        let mut st = PubgenState {
            generated_csv_files: vec![("cat.csv".into(), "type,number,title\nT,1,Test\n".into()), ("tbl.csv".into(), "A,B\n1,2\n".into())],
            generated_zip_name: "test_export.zip".into(),
            ..Default::default()
        };

        let res = execute_download_csv_zip(&mut app, &mut st);
        assert!(res.is_ok(), "Download via service should succeed: {res:?}");

        let data = downloaded_data.lock().unwrap().clone();
        assert!(!data.is_empty(), "Downloaded bytes should be captured");

        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(data)).unwrap();
        assert_eq!(archive.len(), 2);
        assert!(archive.by_name("cat.csv").is_ok());
        assert!(archive.by_name("tbl.csv").is_ok());
    }

    #[test]
    fn test_execute_generate_pages_auto_inserts_tables_from_file_col() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = std::env::temp_dir().join("pubgen_auto_insert_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let cat_path = temp_dir.join("test_catalog.csv");
        let cat_content = "type,number,title,file\n\
                           Tabel,01.01,First Table,tbl_1\n\
                           Tabel,01.02,Second Table,tbl_2\n";
        std::fs::write(&cat_path, cat_content).unwrap();

        let tbl1_path = temp_dir.join("tbl_1.csv");
        let tbl1_content = "Col A,Col B\n10,20\n30,40\n";
        std::fs::write(&tbl1_path, tbl1_content).unwrap();

        let tbl2_path = temp_dir.join("tbl_2.csv");
        let tbl2_content = "Header X,Header Y\nfoo,bar\nbaz,qux\n";
        std::fs::write(&tbl2_path, tbl2_content).unwrap();

        let mut st = PubgenState::default();
        load_card1_csv(&mut st, cat_path.to_str().unwrap());
        st.card1_auto_insert_tables = true;

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("2 pages"), "Message should mention 2 pages: {msg}");
        assert!(msg.contains("2 tables auto-inserted"), "Message should mention 2 tables auto-inserted: {msg}");

        // Cleanup
        let _ = std::fs::remove_file(cat_path);
        let _ = std::fs::remove_file(tbl1_path);
        let _ = std::fs::remove_file(tbl2_path);
        let _ = std::fs::remove_dir(temp_dir);
    }

    #[test]
    fn test_execute_rebuild_toc_updates_entries_and_page_numbers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Tabel".into(), "01.02.".into(), "Keadaan Curah Hujan".into()],
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        // Page 1 is blank. Generate pages: Page 2 is 01.01, Page 3 is 01.02.
        execute_generate_pages(&mut app, &st).unwrap();

        // Put TOC on Page 1
        crate::canvas::go_to_page(&mut app, 0);
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().unwrap();
            let text = active.doc.story(toc.story).unwrap().text.clone();
            assert!(text.contains("Jarak Antar Kecamatan\t2") || text.contains("2"), "TOC text: {text}");
        }

        // Now insert a blank page after Page 1 (between TOC and Chapter 1), shifting Chapter 1 to Page 3 -> 4!
        app.run("layout.pages.insert", json!({"after": 0, "count": 1})).unwrap();

        // Rebuild TOC
        let res_rebuild = execute_rebuild_toc(&mut app, &mut st);
        assert!(res_rebuild.is_ok(), "execute_rebuild_toc should succeed: {res_rebuild:?}");

        {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().unwrap();
            let text = active.doc.story(toc.story).unwrap().text.clone();
            // Now Chapter 1 is on Page 3 and Chapter 2 is on Page 4!
            assert!(text.contains("\t3"), "Chapter 1 should be updated to page 3: {text}");
            assert!(text.contains("\t4"), "Chapter 2 should be updated to page 4: {text}");
        }
    }

    #[test]
    fn test_execute_renumber_page_restarts_numbering_and_rebuilds_toc() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "01.01.".into(), "Data Demografi".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        // Generates Page 2 for Data Demografi
        execute_generate_pages(&mut app, &st).unwrap();

        // Add TOC on Page 1
        crate::canvas::go_to_page(&mut app, 0);
        execute_add_toc(&mut app, &mut st).unwrap();

        // Now select Page 2 (Chapter 1) and restart page numbering at 1 with Arabic numerals
        crate::canvas::go_to_page(&mut app, 1);
        st.card3_renumber_start = 1;
        st.card3_renumber_style = 0; // "1, 2, 3…"
        let res_renum = execute_renumber_page(&mut app, &mut st);
        assert!(res_renum.is_ok(), "execute_renumber_page should succeed: {res_renum:?}");

        // Verify page label is now '1'
        assert_eq!(app.session.page_label(1), "1");

        // Verify TOC was automatically rebuilt with page number 1!
        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = active.doc.story(toc.story).unwrap().text.clone();
        assert!(text.contains("Data Demografi\t1"), "TOC entry should reflect renumbered page 1: {text}");
    }

    #[test]
    fn test_toc_include_self_option() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Bab".into(), "1".into(), "Pendahuluan".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Isi".into(),
            toc_include_self: true,
            ..Default::default()
        };

        // Generate chapter page (Page 2)
        execute_generate_pages(&mut app, &st).unwrap();

        // Generate TOC on Page 1 with toc_include_self = true
        crate::canvas::go_to_page(&mut app, 0);
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc with toc_include_self should succeed: {res:?}");

        let (toc_label, text) = {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().unwrap();
            let first_frame = active.doc.story(toc.story).unwrap().frames.first().copied().unwrap();
            let toc_pg = active.doc.page_of_item(first_frame).unwrap();
            let label = app.session.page_label(toc_pg);
            let text = active.doc.story(toc.story).unwrap().text.clone();
            (label, text)
        };
        // Should contain both the title and self entry:
        assert!(text.contains("Daftar Isi"), "Title should be Daftar Isi: {text}");
        assert!(text.contains(&format!("Daftar Isi\t{toc_label}")), "Self entry should be present: {text}");
        assert!(text.contains("Bab 1 Pendahuluan"), "Chapter entry should be present: {text}");

        // Now disable toc_include_self and rebuild TOC
        st.toc_include_self = false;
        let res_rebuild = execute_rebuild_toc(&mut app, &mut st);
        assert!(res_rebuild.is_ok(), "execute_rebuild_toc should succeed: {res_rebuild:?}");

        {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().unwrap();
            let text = active.doc.story(toc.story).unwrap().text.clone();
            assert!(!text.contains(&format!("Daftar Isi\t{toc_label}")), "Self entry should be removed when unchecked: {text}");
            assert!(text.contains("Bab 1 Pendahuluan"), "Chapter entry should remain: {text}");
        }
    }

    #[test]
    fn test_toc_paragraph_lists_a_generated_review_page_with_its_chapter_line() {
        // The whole way: an RXX row becomes a page with two text boxes (the chapter line and the
        // title), and the paragraph list names it "BAB 01 Geografi".
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Ulasan".into(), "01".into(), "BAB 01 Geografi".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_kind: 3,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Isi".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        crate::canvas::go_to_page(&mut app, 0);
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains("BAB 01 Geografi"), "the review page reads chapter line and title: {text}");
    }

    #[test]
    fn test_toc_paragraph_pairs_bilingual_column_headings_on_a_dual_lang_page() {
        // A Dual Lang RXX page: the Indonesian RTF flows into the left column frame, the English
        // into the right one; both carry `Heading 1`. With the pairing checkbox on, the paragraph
        // list shows one row per heading pair.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        ensure_para_style(&mut app, "Heading 1");
        ensure_para_style(&mut app, TOC_NARRATIVE_STYLE);
        let (si, _) = app.session.active().unwrap().doc.page_loc(0).unwrap();
        for (x0, x1, text) in [(36.0, 300.0, "Proporsi fisik daratan"), (312.0, 576.0, "Physical proportions of the landmass")] {
            let res = app
                .run("frame.create", json!({"spread": si, "rect": [x0, 120.0, x1, 240.0], "content": "text", "text": text, "caret": false}))
                .expect("frame.create");
            let sid = res["story"].as_u64().expect("story");
            app.run("text.select", json!({"story": sid, "anchor": 0, "focus": 0})).expect("text.select");
            app.run("style.paragraph.apply", json!({"name": "Heading 1"})).expect("style.paragraph.apply");
        }
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let pages = app.session.active().unwrap().doc.page_count();
        crate::canvas::go_to_page(&mut app, pages - 1);

        let mut st = PubgenState {
            toc_kind: 3,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Isi".into(),
            toc_pair_bilingual: true,
            ..Default::default()
        };
        // The checkbox alone does nothing: pairing fires only with the Dual Lang toggle on.
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");
        {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().expect("table of contents");
            let text = active.doc.story(toc.story).expect("TOC story").text.clone();
            assert!(!text.contains(" / "), "single lang mode never pairs: {text}");
        }

        st.bilingual_output = true;
        let res = execute_rebuild_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");
        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains("Proporsi fisik daratan / Physical proportions of the landmass"), "the pair merges into one row: {text}");
        assert!(!text.lines().filter(|l| l.contains("Proporsi")).any(|l| !l.contains(" / ")), "no unpaired Indonesian row: {text}");
        // The stored definition keeps the option, so Rebuild TOC pairs again.
        assert!(toc.entries.iter().any(|e| e.pair_columns), "{:?}", toc.entries);
        let res = execute_rebuild_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");
        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains("Proporsi fisik daratan / Physical proportions of the landmass"), "the rebuild keeps the pairing: {text}");
    }

    #[test]
    fn test_toc_paragraph_long_heading2_wraps_early_without_hanging_indent() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        ensure_para_style(&mut app, "Heading 2");
        ensure_para_style(&mut app, TOC_NARRATIVE_STYLE);

        let active = app.session.active().unwrap();
        let si = 0;
        let p = &active.doc.spreads[si].pages[0];
        let bounds = p.bounds();
        let mid = bounds.center().x;
        let (left_col, right_col) = ([bounds.x0 + 36.0, mid - 18.0], [mid + 18.0, bounds.x1 - 36.0]);

        // Place chapter title
        let _ = app.run("frame.create", json!({"spread": si, "rect": [bounds.x0 + 36.0, 36.0, bounds.x1 - 36.0, 72.0], "content": "text", "text": "BAB 02 Pemerintahan", "caret": false})).unwrap();

        // Place long heading 2 in left column (ID) and right column (EN)
        let id_heading = "Analisis Struktur Wilayah Administrasi dan Kapasitas Sumber Daya Manusia Kecamatan Karawang Timur";
        let en_heading = "Administrative Structure and Human Resource Capacity in Karawang Timur District";

        for (text, [x0, x1]) in [(id_heading, left_col), (en_heading, right_col)] {
            let res = app
                .run("frame.create", json!({"spread": si, "rect": [x0, 120.0, x1, 240.0], "content": "text", "text": text, "caret": false}))
                .expect("frame.create");
            let sid = res["story"].as_u64().expect("story");
            app.run("text.select", json!({"story": sid, "anchor": 0, "focus": 0})).expect("text.select");
            app.run("style.paragraph.apply", json!({"name": "Heading 2"})).expect("style.paragraph.apply");
        }

        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let pages = app.session.active().unwrap().doc.page_count();
        crate::canvas::go_to_page(&mut app, pages - 1);

        let mut st = PubgenState {
            toc_kind: 3, // TOC Paragraph
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Isi".into(),
            toc_pair_bilingual: true,
            bilingual_output: true,
            toc_hanging_indent: true, // User had this checked by default
            ..Default::default()
        };

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let story = active.doc.story(toc.story).expect("TOC story");

        // Verify style indentation for TOC Level 3 (Heading 2)
        let style = active.doc.styles.para("TOC Level 3").expect("TOC Level 3 style");
        assert_eq!(style.para.left_indent, Some(24.0), "TOC Level 3 left_indent must equal base_indent (24.0) without unnecessary hanging indent");
        assert_eq!(style.para.first_line_indent, Some(0.0), "TOC Level 3 first_line_indent must be 0.0 so continuation lines align with first line");

        // Verify the long paired heading was wrapped early using FORCED_LINE_BREAK
        assert!(
            story.text.contains(designcraft_doc::story::FORCED_LINE_BREAK),
            "Long Heading 2 should be early-wrapped with soft line breaks: {}",
            story.text
        );

        // Verify English half retains TOC English (italic) styling across the wrapped lines
        let en_pos = story.text.find("Administrative Structure").expect("English text start");
        let fmt = story.char_format_of(en_pos);
        assert_eq!(fmt.style, "TOC English", "English portion must be styled with TOC English italic");

        let en_end_pos = story.text.find("District").expect("English text end");
        let fmt_end = story.char_format_of(en_end_pos);
        assert_eq!(fmt_end.style, "TOC English", "End of English portion across line breaks must remain TOC English");

        // Verify Indonesian half retains normal style
        let id_pos = story.text.find("Analisis Struktur").expect("ID text start");
        let fmt_id = story.char_format_of(id_pos);
        assert_eq!(fmt_id.style, designcraft_doc::NO_CHAR_STYLE, "ID portion must have default char style");
    }

    #[test]
    fn test_toc_paragraph_lists_a_legacy_chapter_page_until_its_pages_are_regenerated() {
        // A page made before the narrative style existed: the chapter line in a plain box, the
        // title below it tagged with the card's TOC style.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        ensure_para_style(&mut app, "TOC Entry");
        let (si, pi) = app.session.active().unwrap().doc.page_loc(0).unwrap();
        let b = app.session.active().unwrap().doc.spreads[si].pages[pi].bounds();
        for (text, style, dy) in [("BAB 01", None, 0.0_f64), ("Geografi", Some("TOC Entry"), 40.0_f64)] {
            let rect = [b.x0 + 36.0, b.y0 + 36.0 + dy, b.x1 - 36.0, b.y0 + 76.0 + dy];
            let res =
                app.run("frame.create", json!({"spread": si, "rect": rect, "content": "text", "text": text, "caret": false})).expect("frame.create");
            if let Some(style) = style {
                let sid = res["story"].as_u64().expect("story");
                app.run("text.select", json!({"story": sid, "anchor": 0, "focus": text.chars().count() as u64})).expect("text.select");
                app.run("style.paragraph.apply", json!({"name": style})).expect("style.paragraph.apply");
            }
        }
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        let mut st = PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), toc_title: "Daftar Isi".into(), ..Default::default() };
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains("BAB 01 Geografi"), "an older document still lists its chapter: {text}");
    }

    #[test]
    fn test_toc_table_list_names_itself_when_include_self_is_on() {
        // The Tables list gets the self entry like the plain list does.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "02.01.".into(), "Jumlah Rukun Warga (RW)".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 5,
            toc_kind: 1,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Tabel".into(),
            toc_include_self: true,
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();
        crate::canvas::go_to_page(&mut app, 0);
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let first_frame = active.doc.story(toc.story).expect("TOC story").frames.first().copied().expect("frame");
        let toc_pg = active.doc.page_of_item(first_frame).expect("page");
        let label = app.session.page_label(toc_pg);
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains(&format!("Daftar Tabel\t{label}")), "the tables list names itself: {text}");
    }

    #[test]
    fn test_tablepage_toc_layout_style_generates_header_and_standalone_number_column() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "02.01.".into(), "Jumlah Rukun Warga (RW)".into()],
                vec!["Tabel".into(), "02.02.".into(), "Jumlah Pegawai Negeri Sipil".into()],
            ],
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Tabel".into(),
            toc_layout_style: 1, // TablePage mode
            ..Default::default()
        };

        // Generate pages
        execute_generate_pages(&mut app, &st).unwrap();

        // Insert blank page at beginning for TOC
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc in TablePage mode should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story = active.doc.story(toc.story).unwrap();
        let text = &story.text;

        // 1. Must contain "Daftar Tabel" title and "Tabel\thalaman" column header
        assert!(text.contains("Daftar Tabel"), "Should contain title: {text}");
        assert!(text.contains("Tabel\thalaman"), "Should contain TablePage header line: {text}");

        // 2. Entries must use standalone number column: {num}\t{title}\t{page}
        assert!(text.contains("02.01\tJumlah Rukun Warga (RW)\t2"), "Entry 1 must have standalone number column: {text}");
        assert!(text.contains("02.02\tJumlah Pegawai Negeri Sipil\t3"), "Entry 2 must have standalone number column: {text}");

        // 3. Verify paragraph styles applied
        assert!(active.doc.styles.para("TOC TablePage Header").is_some());
        assert!(active.doc.styles.para("TOC Level 1").is_some());
    }

    #[test]
    fn test_tablepage_toc_layout_section_headers_without_page_numbers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Bab".into(), "4.3".into(), "Perumahan dan Lingkungan".into()],
                vec!["Tabel".into(), "4.3.1".into(), "Banyaknya Desa Menurut Penerangan Jalan".into()],
            ],
            toc_style: "TOC Entry".into(),
            toc_title: "".into(), // Blank title: starts directly with Tabel\thalaman
            toc_layout_style: 1,  // TablePage mode
            ..Default::default()
        };

        // Generate pages
        execute_generate_pages(&mut app, &st).unwrap();

        // Insert blank page for TOC
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc in TablePage mode should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story = active.doc.story(toc.story).unwrap();
        let text = &story.text;

        // 1. Starts directly with Tabel\thalaman header
        assert!(text.starts_with("Tabel\thalaman"), "Should start with header when title is empty: {text}");

        // 2. Section header 4.3 must NOT have page number or leader dots
        assert!(text.contains("4.3\tPerumahan dan Lingkungan\n"), "Section header must not have a page number: {text}");

        // 3. Table entry 4.3.1 must have page number
        assert!(text.contains("4.3.1\tBanyaknya Desa Menurut Penerangan Jalan\t3"), "Table entry must have page number: {text}");

        // 4. Styles
        assert!(active.doc.styles.para("TOC Section Header").is_some());
    }

    #[test]
    fn test_tablepage_toc_continuation_headers_on_overflow_pages() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 80 records so TOC spans across multiple pages (at least 2 pages)
        let records: Vec<Vec<String>> =
            (1..=80).map(|i| vec!["Tabel".into(), format!("01.{i:02}."), format!("Dataset Observation Record Number {i} in Region")]).collect();

        let mut st = PubgenState {
            card1_records: records,
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 5, // TOC Line preset
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Tabel".into(),
            toc_layout_style: 1, // TablePage mode
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();

        // Insert TOC starting page at beginning
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        crate::canvas::go_to_page(&mut app, 0);

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let (cont_count, expected_flow_top, second_frame_y0) = {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().unwrap();
            let story_id = toc.story;
            let story = active.doc.story(story_id).unwrap();

            // Must have more than 1 frame because 80 entries overflow page 1
            assert!(story.frames.len() > 1, "TOC should span across multiple frames: {}", story.frames.len());

            // Find continuation header items across spreads
            let mut cont_headers = Vec::new();
            for spread in &active.doc.spreads {
                for it in &spread.items {
                    if it.label == TOC_CONTINUATION_HEADER_LABEL {
                        cont_headers.push(it.clone());
                    }
                }
            }

            assert!(!cont_headers.is_empty(), "Continuation header items must be created for overflowed pages");

            // Inspect the continuation header text
            let first_hdr = &cont_headers[0];
            let tf = first_hdr.text_frame().unwrap();
            let hdr_story = active.doc.story(tf.story).unwrap();
            assert_eq!(hdr_story.text.trim(), "Tabel\thalaman");

            // Verify the continuation text frame on page 2 starts below the header
            let second_frame_id = story.frames[1];
            let second_frame_item = active.doc.item(second_frame_id).unwrap();
            let second_pg = active.doc.page_of_item(second_frame_id).unwrap();
            let (si, pi) = active.doc.page_loc(second_pg).unwrap();
            let m = active.doc.spreads[si].pages[pi].margin_rect();

            let expected_flow_top = m.y0 + TOC_CONT_HEADER_H + TOC_CONT_HEADER_GAP;
            (cont_headers.len(), expected_flow_top, second_frame_item.bounds().y0)
        };

        assert!(
            (second_frame_y0 - expected_flow_top).abs() < 1.0,
            "Continuation text frame must start below header at {expected_flow_top}, got {second_frame_y0}"
        );

        // Test Rebuild TOC: ensure rebuild cleanly preserves single continuation header per page
        let rebuild_res = execute_rebuild_toc(&mut app, &mut st);
        assert!(rebuild_res.is_ok(), "execute_rebuild_toc should succeed: {rebuild_res:?}");

        let active2 = app.session.active().unwrap();
        let mut cont_headers_after = Vec::new();
        for spread in &active2.doc.spreads {
            for it in &spread.items {
                if it.label == TOC_CONTINUATION_HEADER_LABEL {
                    cont_headers_after.push(it.clone());
                }
            }
        }
        assert_eq!(cont_count, cont_headers_after.len(), "Rebuild should not duplicate continuation headers");
    }

    #[test]
    fn test_execute_generate_pages_g_sheet_without_title_names_its_rtf_title() {
        // The `GXX` sheets carry no title row, only the RTF link, so the page opens with the
        // RTF's own title line. Word writes that line as direct formatting (bold, caps, centred),
        // not as a `Heading 1`, so the paragraph table of contents has to read the page's heading
        // off how it looks or the page is missing from the list.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G02.rtf");
        let rtf = concat!(
            r"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}{\stylesheet{\s0 Normal;}}",
            r"\pard\plain\s0\qc\b\caps\fs30 DAFTAR SINGKATAN/list of Abbreviation\par",
            r"\pard\plain\s0\fs24 SI : Stasiun Iklim/Climate Station\par}"
        );
        std::fs::write(&rtf_path, rtf.as_bytes()).unwrap();
        let catalog_path = temp_dir.path().join("G02_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nG,02,,\"{escaped}\"\n")).unwrap();
        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        st.card1_preset = 0;
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        execute_generate_pages(&mut app, &st).expect("generate pages");

        let active = app.session.active().unwrap();
        let body = active.doc.stories.values().find(|s| s.text.contains("DAFTAR SINGKATAN")).expect("placed RTF");
        assert_eq!(body.paras[0].style, TOC_NARRATIVE_STYLE, "the RTF's title line is the page's title");
        assert_eq!(body.paras[1].style, designcraft_doc::story::BASIC_PARAGRAPH, "the body keeps its own style");
        assert_eq!(body.paras[0].para.align, Some(designcraft_doc::Align::Center), "the title keeps the RTF's centring");

        let plan = TocPlan::from_state(&PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() });
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(rows, [(1, "DAFTAR SINGKATAN/list of Abbreviation".to_string())], "the paragraph list names the page at its own level: {rows:?}");
    }

    #[test]
    fn test_execute_generate_pages_g_sheet_without_title_in_template_mode() {
        // The template flow fills its title slot with the sheet's title: empty for these sheets,
        // so the RTF's own first line is the page's title there too.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G01.rtf");
        let rtf = concat!(
            r"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}{\stylesheet{\s0 Normal;}}",
            r"\pard\plain\s0\qc\b\caps\fs30 Penjelasan Umum/Explanatory Notes\par",
            r"\pard\plain\s0\fs24 The publication covers the regency.\par}"
        );
        std::fs::write(&rtf_path, rtf.as_bytes()).unwrap();
        let catalog_path = temp_dir.path().join("G01_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nG,01,,\"{escaped}\"\n")).unwrap();

        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        st.card1_preset = 6;
        st.card1_template_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/templates/front_matter/001_publication_note.designcraft").into();
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        execute_generate_pages(&mut app, &st).expect("generate pages");

        let active = app.session.active().unwrap();
        let body = active.doc.stories.values().find(|s| s.text.contains("Penjelasan Umum")).expect("placed RTF");
        assert_eq!(body.paras[0].style, TOC_NARRATIVE_STYLE, "the RTF's title line is the page's title");

        let plan = TocPlan::from_state(&PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() });
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(rows, [(1, "Penjelasan Umum/Explanatory Notes".to_string())], "the paragraph list names the page: {rows:?}");
    }

    #[test]
    fn test_generating_the_paragraph_toc_leaves_the_table_toc_alone() {
        // Each list keeps its own entry styles now: the Tables list hangs its rows differently
        // from the Contents list, and the styles they used to share carried the tab stop and
        // indent of whichever list was generated last — regenerating the Contents moved the
        // Tables page's rows.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        for name in ["TOC Entry", TOC_NARRATIVE_STYLE] {
            ensure_para_style(&mut app, name);
        }
        add_page_text_at(&mut app, 0, "BAB 01 Geografi", TOC_NARRATIVE_STYLE, 0.0);
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        add_page_text_at(&mut app, 1, "Tabel 01.01 Jarak Antar Kecamatan", "TOC Entry", 0.0);
        app.run("layout.pages.insert", json!({"count": 2})).unwrap();

        let table_state = PubgenState {
            toc_kind: 1,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Tabel".into(),
            toc_hanging_indent: false,
            ..Default::default()
        };
        let para_state = PubgenState {
            toc_kind: 3,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Isi".into(),
            toc_hanging_indent: true,
            ..Default::default()
        };
        let table_plan = TocPlan::from_state(&table_state);
        let para_plan = TocPlan::from_state(&para_state);
        generate_toc(&mut app, &table_state, 2, &table_plan, false).expect("table toc");
        let table_text = toc_text_on_page(&app, 2).expect("the Tables list on page 2");
        assert!(table_text.contains("Tabel 01.01"), "the Tables list names the table page: {table_text:?}");

        generate_toc(&mut app, &para_state, 3, &para_plan, false).expect("paragraph toc");

        assert_eq!(toc_text_on_page(&app, 2).as_deref(), Some(table_text.as_str()), "the Tables list is untouched");
        let active = app.session.active().unwrap();
        let table_style = active.doc.styles.para("TOC Table Level 1").expect("the Tables list's own style");
        assert_eq!(table_style.para.left_indent, Some(0.0), "the Tables list keeps its (unhung) indent");
        assert_eq!(table_style.para.first_line_indent, Some(0.0));
        let contents_style = active.doc.styles.para("TOC Level 1").expect("the Contents list's style");
        assert_eq!(contents_style.para.left_indent, Some(0.0), "the Contents list keeps its unhung indent for paragraph TOC");
        assert_eq!(contents_style.para.first_line_indent, Some(0.0));
        let table_entry = active
            .doc
            .tocs
            .iter()
            .find(|t| toc_story_page(&app, t.story) == Some(2))
            .and_then(|t| active.doc.story(t.story))
            .and_then(|s| s.paras.get(1).map(|p| p.style.clone()))
            .unwrap_or_default();
        assert_eq!(table_entry, "TOC Table Level 1", "the Tables list's entry takes its own style");
    }

    #[test]
    fn test_g_sheet_title_skips_the_issn_line_and_the_book_title() {
        // The compilers' page (a `G` sheet that links only its RTF): a right-aligned ISSN line, a
        // few empty lines, then the page's own title in the largest centred type, with the
        // publication's title set smaller below it. Naming the page after its first paragraph
        // listed the ISSN line; naming it after its largest centred line lists the page.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G00.rtf");
        let rtf = concat!(
            r"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}{\stylesheet{\s0 Normal;}}",
            r"\pard\plain\s0\qr\fs24 ISSN: 2828-7346\par",
            r"\pard\plain\s0\par",
            r"\pard\plain\s0\par",
            r"\pard\plain\s0\par",
            r"\pard\plain\s0\qc\b\fs28 TIM PENYUSUN/COMPILERS\par",
            r"\pard\plain\s0\qc\b\fs22 Kecamatan Karawang Timur Dalam Angka 2025\par",
            r"\pard\plain\s0\qc\fs22 Karawang Timur District in Figures 2025\par}",
        );
        std::fs::write(&rtf_path, rtf.as_bytes()).unwrap();
        let catalog_path = temp_dir.path().join("G00_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nG,00,,\"{escaped}\"\n")).unwrap();
        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        st.card1_preset = 0;
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        execute_generate_pages(&mut app, &st).expect("generate pages");

        let active = app.session.active().unwrap();
        let body = active.doc.stories.values().find(|s| s.text.contains("TIM PENYUSUN")).expect("placed RTF");
        let style_of = |needle: &str| {
            let i = body.para_ranges().iter().position(|r| body.text.get(r.clone()).is_some_and(|t| t.contains(needle))).expect("paragraph");
            body.paras[i].style.clone()
        };
        assert_eq!(style_of("TIM PENYUSUN"), TOC_NARRATIVE_STYLE, "the page's title names the page");
        assert_eq!(style_of("ISSN"), designcraft_doc::story::BASIC_PARAGRAPH, "the ISSN line stays out of the list");
        assert_eq!(style_of("Kecamatan Karawang"), designcraft_doc::story::BASIC_PARAGRAPH, "so does the publication's title");

        let plan = TocPlan::from_state(&PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() });
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(rows, [(1, "TIM PENYUSUN/COMPILERS".to_string())], "the paragraph list names the page once, at its own level: {rows:?}");
    }

    #[test]
    fn test_g_sheet_title_skips_an_empty_opening_paragraph() {
        // The explanatory-notes sheet (G01) opens with an empty paragraph and carries its title
        // next: searching for the first non-empty paragraph's look finds the title where taking
        // the very first paragraph found nothing, so the page was missing from the list.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G01.rtf");
        let rtf = concat!(
            r"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}{\stylesheet{\s0 Normal;}}",
            r"\pard\plain\s0\par",
            r"\pard\plain\s0\qc\b\fs30 Penjelasan Umum/Explanatory Notes\par",
            r"\pard\plain\s0\ql\fs24 The publication covers the regency and its subdistricts.\par}",
        );
        std::fs::write(&rtf_path, rtf.as_bytes()).unwrap();
        let catalog_path = temp_dir.path().join("G01_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nG,01,,\"{escaped}\"\n")).unwrap();
        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        st.card1_preset = 0;
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        execute_generate_pages(&mut app, &st).expect("generate pages");

        let active = app.session.active().unwrap();
        let body = active.doc.stories.values().find(|s| s.text.contains("Penjelasan Umum")).expect("placed RTF");
        let ranges = body.para_ranges();
        let title = ranges.iter().position(|r| body.text.get(r.clone()).is_some_and(|t| t.contains("Penjelasan Umum"))).expect("title paragraph");
        assert_eq!(body.paras[title].style, TOC_NARRATIVE_STYLE, "the RTF's title line is the page's title");
        assert_eq!(body.paras[0].para.align, None, "the empty opening paragraph keeps its own look");

        let plan = TocPlan::from_state(&PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() });
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(rows, [(1, "Penjelasan Umum/Explanatory Notes".to_string())], "the paragraph list names the page: {rows:?}");
    }

    #[test]
    fn test_execute_page_from_rtf_sample_file() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let initial_pages = app.session.active().unwrap().doc.page_count();
        assert_eq!(initial_pages, 1);

        let mut st = PubgenState::default();
        let rtf_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/SINGLE_LANG/ulasan_bab_1.rtf");

        let res = execute_page_from_rtf(&mut app, &mut st, rtf_path);
        assert!(res.is_ok(), "execute_page_from_rtf should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("ulasan_bab_1.rtf"), "Success message should contain filename: {msg}");

        let active = app.session.active().unwrap();
        let story = active.doc.stories.values().find(|s| s.text.contains("Dalam bab ini membahas")).expect("Imported story must be present");

        // Verify numbered list item (Pusat Pemukiman Padat)
        let num_idx = story.text.find("Pusat Pemukiman Padat").expect("must contain Pusat Pemukiman Padat");
        let num_pi = story.para_at(num_idx);
        let num_p = &story.paras[num_pi].para;
        assert_eq!(num_p.left_indent, Some(36.0), "Numbered list left_indent must be 36pt");
        assert_eq!(num_p.first_line_indent, Some(-18.0), "Numbered list first_line_indent must be -18pt");
        assert_eq!(num_p.space_after, Some(8.0), "Numbered list space_after must be 8pt");
        assert!(num_p.tabs.as_ref().is_some_and(|t| t.iter().any(|s| (s.position - 36.0).abs() < 0.1)));

        // Verify nested sub-bullet list item (S1/Sarjana)
        let bullet_idx = story.text.find("S1/Sarjana").expect("must contain S1/Sarjana");
        let bullet_pi = story.para_at(bullet_idx);
        let bullet_p = &story.paras[bullet_pi].para;
        assert_eq!(bullet_p.left_indent, Some(72.0), "Nested bullet left_indent must be 72pt");
        assert_eq!(bullet_p.first_line_indent, Some(-18.0), "Nested bullet first_line_indent must be -18pt");
        assert_eq!(bullet_p.space_after, Some(8.0), "Nested bullet space_after must be 8pt");
        assert!(bullet_p.tabs.as_ref().is_some_and(|t| t.iter().any(|s| (s.position - 72.0).abs() < 0.1)));

        // Verify level 1 bullet (Komposisi Gender) has unicode bullet •
        let gender_idx = story.text.find("Komposisi Gender").expect("must contain Komposisi Gender");
        let gender_pi = story.para_at(gender_idx);
        let gender_ranges = story.para_ranges();
        let gender_para_text = &story.text[gender_ranges[gender_pi].clone()];
        assert!(gender_para_text.starts_with("\u{2022}\t"), "Level 1 bullet must start with •\\t: {gender_para_text:?}");
    }

    #[test]
    fn test_execute_page_from_rtf_multi_page_autoflow() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let initial_pages = app.session.active().unwrap().doc.page_count();
        assert_eq!(initial_pages, 1);

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("long_multi_page.rtf");
        let para = "The quick brown fox jumps over the lazy dog to fill the entire page with lines of text and paragraphs. \\par\n".repeat(60);
        let rtf_content = format!(r#"{{\rtf1\ansi\deff0 {{\fonttbl {{\f0 Times;}}}}\f0\fs24 {para}}}"#);
        std::fs::write(&file_path, rtf_content.as_bytes()).unwrap();

        let mut st = PubgenState::default();
        let res = execute_page_from_rtf(&mut app, &mut st, &file_path.to_string_lossy());
        assert!(res.is_ok(), "execute_page_from_rtf should succeed: {res:?}");

        let final_pages = app.session.active().unwrap().doc.page_count();
        assert!(final_pages > initial_pages, "Multi-page RTF should autoflow to add extra pages (initial: {initial_pages}, final: {final_pages})");

        // Verify threaded text frames exist across the new pages
        let active = app.session.active().unwrap();
        let text_frames_count =
            active.doc.spreads.iter().flat_map(|sp| &sp.items).filter(|it| matches!(it.content, designcraft_doc::Content::Text(_))).count();
        assert!(text_frames_count >= final_pages, "Each page should have a threaded text frame");
    }

    #[test]
    fn test_execute_page_from_rtf_into_selected_frame() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Create an empty text frame on page 0
        let add_res = app
            .run(
                "frame.create",
                json!({
                    "page": 1,
                    "rect": [72, 72, 300, 400],
                    "content": "text"
                }),
            )
            .unwrap();
        let fid = add_res.get("id").unwrap().as_u64().unwrap();

        // Select the text frame
        app.run("selection.set", json!({"ids": [fid]})).unwrap();

        // Write a small synthetic RTF
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("small_sample.rtf");
        std::fs::write(&file_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Courier;}}\f0\fs24 Hello DesignCraft RTF!}"#).unwrap();

        let mut st = PubgenState::default();
        let res = execute_page_from_rtf(&mut app, &mut st, &file_path.to_string_lossy());
        assert!(res.is_ok(), "execute_page_from_rtf should place into selected frame: {res:?}");

        let active = app.session.active().unwrap();
        let story_id = match active.doc.item(designcraft_doc::ItemId(fid)).unwrap().content {
            designcraft_doc::Content::Text(ref tf) => tf.story,
            _ => panic!("Expected text frame"),
        };
        let story_text = &active.doc.story(story_id).unwrap().text;
        assert!(story_text.contains("Hello DesignCraft RTF!"), "Story text should match: '{story_text}'");
    }

    #[test]
    fn test_execute_generate_pages_note_sheet_title_and_rtf() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("A02.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Hello DesignCraft Note!}"#).unwrap();
        let catalog_path = temp_dir.path().join("A02_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nNote,02,Keterangan Publikasi,\"{escaped}\"\n")).unwrap();

        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "generate pages should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert!(active.doc.page_count() >= 2, "the note gets its own page");

        // The title is a centred Arial text box with a 100 % transparent background.
        let title_story =
            active.doc.stories.values().find(|s| s.text.trim() == "Keterangan Publikasi").unwrap_or_else(|| panic!("note title story should exist"));
        assert_eq!(title_story.paras[0].para.align, Some(designcraft_doc::Align::Center));
        assert!(title_story.chars.iter().all(|run| run.format.over.font_family.as_deref() == Some("Arial")), "title runs should all be Arial");
        let title_item = active
            .doc
            .spreads
            .iter()
            .flat_map(|sp| &sp.items)
            .find(|it| it.text_frame().is_some_and(|tf| tf.story == title_story.id))
            .unwrap_or_else(|| panic!("title frame should exist"));
        assert_eq!(title_item.fill.swatch, "[None]", "title box has no fill");
        assert!(title_item.stroke.is_none(), "title box has no stroke");

        // The RTF note is placed on the same page below the title.
        assert!(active.doc.stories.values().any(|s| s.text.contains("Hello DesignCraft Note!")), "RTF text should be placed on the generated page");
    }

    #[test]
    fn test_kata_pengantar_dynamic_signature_placement() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let margin = designcraft_geom::Rect::new(36.0, 36.0, 559.0, 806.0);
        let (mw, mh) = (margin.x1 - margin.x0, margin.y1 - margin.y0);
        let default_img2_rect = [margin.x1 - mw * 0.244, margin.y0 + mh * 0.354, margin.x1 - mw * 0.044, margin.y0 + mh * 0.464];

        // Create a text frame for the Kata Pengantar RTF
        let rtf_rect = [margin.x0, margin.y0 + 50.0, margin.x1, margin.y1];
        let res = app.run("frame.create", json!({"spread": 0, "rect": rtf_rect, "content": "text", "caret": false})).unwrap();
        let fid = res.get("id").and_then(Value::as_u64).unwrap();
        let _ = app.run("selection.set", json!({"ids": [fid]}));

        let mut st = PubgenState::default();
        let rtf_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/DUAL_LANG/B01.rtf");
        if std::path::Path::new(rtf_path).exists() {
            let place_res = execute_page_from_rtf(&mut app, &mut st, rtf_path);
            assert!(place_res.is_ok(), "execute_page_from_rtf should place B01.rtf: {place_res:?}");

            let (target_spread, img2_rect) = compute_signature_placement(&app, fid, default_img2_rect, 0, margin);
            assert_eq!(target_spread, 0);

            // The dynamic signature rect must be anchored down near the signatory block at the bottom
            // of the text, significantly lower than the default static position (~308pt).
            assert!(
                img2_rect[1] > default_img2_rect[1] + 50.0,
                "Dynamic signature y0 ({}) should be anchored lower than default ({})",
                img2_rect[1],
                default_img2_rect[1]
            );
            assert!(img2_rect[1] >= margin.y0 && img2_rect[3] <= margin.y1);
            assert!(img2_rect[0] >= margin.x0 && img2_rect[2] <= margin.x1);
        }
    }

    #[test]
    fn test_execute_generate_pages_g_sheet_tags_its_title_for_the_paragraph_toc() {
        // `GXX` sheet rows (front matter such as the compiler page, the explanatory notes or the
        // abbreviations) keep the narrative style on their title, so a paragraph table of contents
        // names them beside the chapter reviews. The template flow tagged its title slot; the
        // preset flow left it plain.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G00.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Hello DesignCraft G00 Narrative!}"#).unwrap();
        let catalog_path = temp_dir.path().join("G00_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nG,00,TIM PENYUSUN/COMPILERS,\"{escaped}\"\n")).unwrap();
        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        st.card1_preset = 0;
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        execute_generate_pages(&mut app, &st).expect("generate pages");

        let active = app.session.active().unwrap();
        let title_story = active.doc.stories.values().find(|s| s.text.contains("TIM PENYUSUN/COMPILERS")).expect("title story");
        assert_eq!(title_story.paras[0].style, TOC_NARRATIVE_STYLE, "the page title carries the narrative style");

        let plan = TocPlan::from_state(&PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() });
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(rows, [(1, "TIM PENYUSUN/COMPILERS".to_string())], "the paragraph list names the page with its title alone: {rows:?}");
    }
    #[test]
    fn test_execute_generate_pages_note_or_g_sheet_single_row_no_header() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G00.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Hello DesignCraft G00 Narrative!}"#).unwrap();
        let catalog_path = temp_dir.path().join("G00_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        // Title column is empty (as produced when sheet only has 1 row: file)
        std::fs::write(&catalog_path, format!("type,number,title,file\nNote,00,,\"{escaped}\"\n")).unwrap();

        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "generate pages should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert!(active.doc.page_count() >= 2, "narrative sheet gets its own page");

        // Verify NO page header story exists
        assert!(
            !active.doc.stories.values().any(|s| s.text.trim() == "Keterangan Publikasi" || s.text.trim() == "Note"),
            "No Page Header story should be created when sheet only had 1 row file"
        );

        // Verify the RTF narrative is placed on the page
        assert!(
            active.doc.stories.values().any(|s| s.text.contains("Hello DesignCraft G00 Narrative!")),
            "RTF text should be placed directly on the page"
        );

        // Verify the RTF frame starts at margin.y0 (not margin.y0 + 64.0)
        let (margin, _) = {
            let sp_idx = active.doc.spreads.len().saturating_sub(1);
            let spread = &active.doc.spreads[sp_idx];
            let page = spread.pages.last().unwrap();
            (page.margin_rect(), sp_idx)
        };
        let rtf_item = active
            .doc
            .spreads
            .last()
            .unwrap()
            .items
            .iter()
            .find(|it| it.text_frame().is_some_and(|tf| active.doc.stories.get(&tf.story).is_some_and(|s| s.text.contains("Hello DesignCraft G00"))))
            .expect("RTF frame should exist");
        let bounds = rtf_item.bounds();
        assert_eq!(bounds.y0, margin.y0, "RTF frame should start at top margin without header offset");
    }

    #[test]
    fn test_execute_page_from_rtf_error_handling() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState::default();

        // Test empty path
        let res_empty = execute_page_from_rtf(&mut app, &mut st, "");
        assert!(res_empty.is_err());
        assert_eq!(res_empty.unwrap_err(), "No RTF file path specified");

        // Test non-existent path
        let res_missing = execute_page_from_rtf(&mut app, &mut st, "non_existent_file_path_12345.rtf");
        assert!(res_missing.is_err());
        assert!(res_missing.unwrap_err().contains("File not found"));
    }

    fn sample_image_path() -> String {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/app-icon/designcraft-1024.png").to_string()
    }

    #[test]
    fn test_execute_page_from_image_as_object_fits_page_margins() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState { image_as_background: false, ..PubgenState::default() };
        let res = execute_page_from_image(&mut app, &mut st, &sample_image_path());
        assert!(res.is_ok(), "execute_page_from_image (object) should succeed: {res:?}");

        // The graphic frame sits proportionally inside the page margin rect.
        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(0).unwrap();
        let expected = active.doc.spreads[si].pages[pi].margin_rect();
        let frame = active.doc.spreads[si]
            .items
            .iter()
            .find(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)))
            .unwrap_or_else(|| panic!("expected a graphic frame on the page"));
        let b = frame.bounds();
        assert!(b.x0 >= expected.x0 - 0.01 && b.y0 >= expected.y0 - 0.01, "{b:?} vs {expected:?}");
        assert!(b.x1 <= expected.x1 + 0.01 && b.y1 <= expected.y1 + 0.01, "{b:?} vs {expected:?}");
        assert!((b.width() - b.height()).abs() < 0.01, "Aspect ratio preserved: {b:?}");
        assert!(((b.x0 + b.x1) / 2.0 - (expected.x0 + expected.x1) / 2.0).abs() < 0.01);
        assert!(((b.y0 + b.y1) / 2.0 - (expected.y0 + expected.y1) / 2.0).abs() < 0.01);
    }

    #[test]
    fn test_execute_page_from_image_as_background_covers_page_and_goes_to_back() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Existing content on the page: the background must end up behind it.
        let text = app.run("frame.create", json!({ "rect": [72, 72, 300, 400], "content": "text", "text": "Hello" })).unwrap();
        let text_id = text.get("id").and_then(Value::as_u64).unwrap();

        let mut st = PubgenState { image_as_background: true, ..PubgenState::default() };
        let res = execute_page_from_image(&mut app, &mut st, &sample_image_path());
        assert!(res.is_ok(), "execute_page_from_image (background) should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(0).unwrap();
        let page = &active.doc.spreads[si].pages[pi];
        let expected = page.bounds();

        let bg_index = active.doc.spreads[si]
            .items
            .iter()
            .position(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)))
            .unwrap_or_else(|| panic!("expected a background graphic frame"));
        assert_eq!(bg_index, 0, "the background frame must be at the back of the spread");
        let text_index = active.doc.spreads[si].items.iter().position(|it| it.id.0 == text_id).unwrap();
        assert!(text_index > bg_index, "existing items stay above the background");

        let b = active.doc.spreads[si].items[bg_index].bounds();
        assert!(b.x0 >= expected.x0 - 0.01 && b.y0 >= expected.y0 - 0.01, "{b:?} vs {expected:?}");
        assert!(b.x1 <= expected.x1 + 0.01 && b.y1 <= expected.y1 + 0.01, "{b:?} vs {expected:?}");
        assert!((b.width() - b.height()).abs() < 0.01, "Aspect ratio preserved: {b:?}");
        assert!((b.width() - expected.width()).abs() < 0.01 || (b.height() - expected.height()).abs() < 0.01);
        assert!(((b.x0 + b.x1) / 2.0 - (expected.x0 + expected.x1) / 2.0).abs() < 0.01);
        assert!(((b.y0 + b.y1) / 2.0 - (expected.y0 + expected.y1) / 2.0).abs() < 0.01);
    }

    #[test]
    fn test_execute_page_from_image_error_handling() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let mut st = PubgenState::default();

        let res_empty = execute_page_from_image(&mut app, &mut st, "");
        assert!(res_empty.is_err());
        assert_eq!(res_empty.unwrap_err(), "No image file path specified");

        let res_missing = execute_page_from_image(&mut app, &mut st, "non_existent_image_12345.png");
        assert!(res_missing.is_err());
        assert!(res_missing.unwrap_err().contains("File not found"));
    }

    #[test]
    fn test_execute_page_from_image_contains_a_non_square_image_on_every_side() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // A 4:1 image: fitted into a page it must be shorter than the page, not cropped.
        let png = designcraft_render::Rendered { width: 400, height: 100, pixels: vec![200; 400 * 100 * 4] }.to_png();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wide.png");
        std::fs::write(&path, &png).unwrap();

        let mut st = PubgenState { image_as_background: true, ..PubgenState::default() };
        let res = execute_page_from_image(&mut app, &mut st, &path.to_string_lossy());
        assert!(res.is_ok(), "execute_page_from_image should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(0).unwrap();
        let page_bounds = active.doc.spreads[si].pages[pi].bounds();
        let it = active.doc.spreads[si]
            .items
            .iter()
            .find(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)))
            .unwrap_or_else(|| panic!("expected a graphic frame"));
        let designcraft_doc::Content::Graphic(g) = &it.content else { panic!("expected a graphic") };
        let (nw, nh) = g.size;
        let placed = g.xf.transform_rect_bbox(designcraft_geom::Rect::new(0.0, 0.0, nw, nh));

        // Aspect ratio kept.
        assert!((placed.width() / placed.height() - nw / nh).abs() < 0.01, "aspect kept: {placed:?}");
        // All four sides inside the page.
        assert!(placed.x0 >= page_bounds.x0 - 0.01 && placed.x1 <= page_bounds.x1 + 0.01, "{placed:?} vs {page_bounds:?}");
        assert!(placed.y0 >= page_bounds.y0 - 0.01 && placed.y1 <= page_bounds.y1 + 0.01, "{placed:?} vs {page_bounds:?}");
        // The wide image is limited by width, so it is shorter than the page (no height overflow).
        assert!(placed.height() < page_bounds.height(), "no vertical overflow: {placed:?} vs {page_bounds:?}");
        // Centred on the page.
        assert!(((placed.x0 + placed.x1) / 2.0 - page_bounds.center().x).abs() < 0.5, "centred: {placed:?}");
        assert!(((placed.y0 + placed.y1) / 2.0 - page_bounds.center().y).abs() < 0.5, "centred: {placed:?}");
    }

    #[test]
    fn test_execute_generate_cover_text_formatting() {
        for style in 0..=9 {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({})).unwrap();
            let st = PubgenState { cover_style: style, ..PubgenState::default() };
            let res = execute_generate_cover(&mut app, &st);
            assert!(res.is_ok(), "Style {style} failed: {res:?}");

            let active = app.session.active().unwrap();
            // Assert that every story on the cover page has styled character runs
            assert!(!active.doc.stories.is_empty(), "Style {style} should create stories");
            for story in active.doc.stories.values() {
                for run in &story.chars {
                    assert!(run.format.over.size.is_some(), "Story text '{}' in style {style} must have custom size", story.text);
                    assert!(run.format.over.fill.is_some(), "Story text '{}' in style {style} must have custom fill", story.text);
                    let fam = run.format.over.font_family.as_deref().unwrap_or_default();
                    let st = run.format.over.font_style.as_deref().unwrap_or_default();
                    assert!(fam == "Arial" || fam == "Arial Black", "Expected Arial family, got {fam}");
                    if story.text.contains("KARAWANG TIMUR")
                        || story.text.contains("KARAWANG\nTIMUR")
                        || story.text == "2026"
                        || story.text == "20"
                        || story.text == "26"
                    {
                        assert_eq!(fam, "Arial Black", "Title and Year must use Arial Black in story '{}'", story.text);
                    }
                    if story.text.contains("BADAN PUSAT STATISTIK") {
                        assert_eq!(fam, "Arial", "Logo text must use Arial");
                        assert_eq!(st, "Bold Italic", "Logo text must use Bold Italic");
                    }
                }
            }
        }
    }

    #[test]
    fn test_export_design_3_sample() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let mut st = PubgenState {
            cover_style: 5,
            cover_color_primary: "#6c5ce7".into(),
            cover_color_accent: "#a29bfe".into(),
            cover_color_dark: "#1e293b".into(),
            ..PubgenState::default()
        };
        st.cover_image_skyline = root.join("outputs").join("city_skyline.jpg").to_string_lossy().to_string();
        st.cover_image_logo = root.join("outputs").join("bps_logo.png").to_string_lossy().to_string();
        execute_generate_cover(&mut app, &st).unwrap();
        let _ = app.run("layout.pages.delete", json!({"page": 1}));
        // A scratch file, never `design-3.designcraft`: running the tests used to overwrite that
        // tracked sample with every run (the scratch document is the default page size, not the
        // size the sample was drawn at), so the export must not touch it.
        let path = std::env::temp_dir().join(format!("designcraft-cover-style-5-{}.designcraft", std::process::id()));
        let res = app.run("file.saveAs", json!({"path": path.to_string_lossy()}));
        assert!(res.is_ok(), "Failed to save: {res:?}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_export_design_4_sample() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let st = PubgenState {
            cover_style: 6,
            cover_image_skyline: root.join("outputs").join("city_skyline.jpg").to_string_lossy().to_string(),
            cover_image_logo: root.join("outputs").join("bps_logo.png").to_string_lossy().to_string(),
            ..PubgenState::default()
        };
        execute_generate_cover(&mut app, &st).unwrap();
        let _ = app.run("layout.pages.delete", json!({"page": 1}));
        // A scratch file, never `design-4.designcraft`: that one is the hand-authored reference this
        // style was derived from, so the export must not overwrite it.
        let path = std::env::temp_dir().join(format!("designcraft-cover-style-6-{}.designcraft", std::process::id()));
        let res = app.run("file.saveAs", json!({"path": path.to_string_lossy()}));
        assert!(res.is_ok(), "Failed to save: {res:?}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cover_style_8_text_is_options_only() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState { cover_style: BUSINESS_STYLE, ..PubgenState::default() };
        execute_generate_cover(&mut app, &st).unwrap();

        let active = app.session.active().unwrap();
        let allowed = [
            "KECAMATAN",
            "KABUPATEN",
            "KARAWANG TIMUR",
            "KARAWANG\nTIMUR",
            "2026",
            "VOLUME 23",
            "KATALOG BPS",
            "ISSN",
            "ISBN",
            "BADAN PUSAT STATISTIK",
            "DALAM ANGKA",
            "TITLE 2",
            "20",
            "26",
        ];
        for story in active.doc.stories.values() {
            if story.text.is_empty() {
                continue;
            }
            for banned in ["COMPANY NAME", "YOUR HEADLINE HERE", "LOREM IPSUM", "BUSINESS REPORT 2050"] {
                assert!(!story.text.contains(banned), "template text '{banned}' must not reach the cover: {}", story.text);
            }
            let upper = story.text.to_uppercase();
            assert!(allowed.iter().any(|a| upper.contains(a)), "text must come from the cover options: {}", story.text);
        }

        // The headline title stays on one full-width line (it used to hyphenate across two).
        let title = active.doc.stories.values().find(|s| s.text.contains("KARAWANG")).expect("the title story");
        assert!(!title.text.contains('\n'), "the headline title stays on one line: {}", title.text);

        // Every object on the cover is upright: the wedges carry the angles, not the type.
        let turned =
            active.doc.spreads.iter().flat_map(|s| s.items.iter()).any(|it| designcraft_geom::decompose::decompose(it.xf).rotation.abs() > 1.0);
        assert!(!turned, "every object on the Business Report cover is upright");
    }

    #[test]
    fn test_export_design_5_sample() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        // A4: the size the cover designs are drawn for.
        app.run("file.new", json!({"width": 595.275591, "height": 841.889764})).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let mut st = PubgenState {
            cover_style: HORIZON_STYLE,
            cover_image_skyline: root.join("outputs").join("city_skyline.jpg").to_string_lossy().to_string(),
            cover_image_logo: root.join("outputs").join("bps_logo.png").to_string_lossy().to_string(),
            ..PubgenState::default()
        };
        apply_report_palette(&mut st);
        execute_generate_cover(&mut app, &st).unwrap();
        let _ = app.run("layout.pages.delete", json!({"page": 1}));
        // A scratch file, never `design-5.designcraft`: that one is the traced reference this style
        // was derived from, so the export must not overwrite it.
        let path = std::env::temp_dir().join(format!("designcraft-cover-style-7-{}.designcraft", std::process::id()));
        let res = app.run("file.saveAs", json!({"path": path.to_string_lossy()}));
        assert!(res.is_ok(), "Failed to save: {res:?}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cover_style_9_text_is_options_only() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState { cover_style: URBAN_STYLE, ..PubgenState::default() };
        execute_generate_cover(&mut app, &st).unwrap();

        let active = app.session.active().unwrap();
        let allowed = ["KECAMATAN", "KARAWANG", "2026", "20", "26", "VOLUME", "KATALOG", "ISSN", "ISBN", "BADAN PUSAT", "DALAM ANGKA", "TITLE 2"];
        for story in active.doc.stories.values() {
            if story.text.is_empty() {
                continue;
            }
            let upper = story.text.to_uppercase();
            assert!(allowed.iter().any(|a| upper.contains(a)), "text must come from the cover options: {}", story.text);
        }
    }

    #[test]
    fn test_cover_style_6_text_is_options_only() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState { cover_style: 6, ..PubgenState::default() };
        execute_generate_cover(&mut app, &st).unwrap();

        let active = app.session.active().unwrap();
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        // The only sources of cover text: the Publication Cover options and the fixed suffix.
        let allowed = [
            "KECAMATAN",
            "KABUPATEN",
            "KARAWANG TIMUR",
            "KARAWANG\nTIMUR",
            "2026",
            "VOLUME 23",
            "KATALOG BPS",
            "ISSN",
            "ISBN",
            "BADAN PUSAT STATISTIK",
            "DALAM ANGKA",
            "TITLE 2",
        ];
        for text in &texts {
            if text.is_empty() {
                continue;
            }
            // Nothing from the reference template survives: no brand, no lorem, no subtitle.
            for banned in ["COMPANY NAME", "YOUR HEADLINE HERE", "LOREM IPSUM", "BUSINESS REPORT"] {
                assert!(!text.contains(banned), "template text '{banned}' must not reach the cover: {text}");
            }
            let upper = text.to_uppercase();
            assert!(allowed.iter().any(|a| upper.contains(a)), "text must come from the cover options: {text}");
        }

        // The year sits upright and centred in the forward wedge, and the title runs on one line.
        let year = active.doc.stories.values().find(|s| s.text == "2026").expect("the year story");
        assert_eq!(year.paras[0].para.align, Some(designcraft_doc::Align::Center), "the year is centred in the wedge");
        let title = active.doc.stories.values().find(|s| s.text.contains("KARAWANG")).expect("the title story");
        assert!(!title.text.contains('\n'), "the headline title stays on one line: {}", title.text);

        let turned =
            active.doc.spreads.iter().flat_map(|s| s.items.iter()).any(|it| designcraft_geom::decompose::decompose(it.xf).rotation.abs() > 1.0);
        assert!(!turned, "every object on the Diagonal Arrow cover is upright");
    }

    #[test]
    fn test_cover_style_7_text_is_options_only_and_position_locked() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState { cover_style: HORIZON_STYLE, ..PubgenState::default() };
        execute_generate_cover(&mut app, &st).unwrap();

        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(0).unwrap();
        let page = active.doc.spreads[si].pages[pi].bounds();

        // Nothing but the cover options and the fixed suffix reaches the page.
        let allowed = [
            "KECAMATAN",
            "KABUPATEN",
            "KARAWANG TIMUR",
            "KARAWANG\nTIMUR",
            "2026",
            "VOLUME 23",
            "KATALOG BPS",
            "ISSN",
            "ISBN",
            "BADAN PUSAT STATISTIK",
            "DALAM ANGKA",
            "TITLE 2",
        ];
        for story in active.doc.stories.values() {
            if story.text.is_empty() {
                continue;
            }
            for banned in ["COMPANY NAME", "YOUR HEADLINE HERE", "LOREM IPSUM", "BUSINESS REPORT"] {
                assert!(!story.text.contains(banned), "template text '{banned}' must not reach the cover: {}", story.text);
            }
            let upper = story.text.to_uppercase();
            assert!(allowed.iter().any(|a| upper.contains(a)), "text must come from the cover options: {}", story.text);
        }

        // Bounds of every text frame, keyed by the story it flows.
        let placed: Vec<(String, designcraft_geom::Rect)> = active.doc.spreads[si]
            .items
            .iter()
            .filter_map(|it| match &it.content {
                designcraft_doc::Content::Text(tf) => active.doc.story(tf.story).map(|s| (s.text.clone(), it.bounds())),
                _ => None,
            })
            .collect();
        let frame = |needle: &str| {
            placed.iter().find(|(text, _)| text.contains(needle)).map(|(_, r)| *r).unwrap_or_else(|| panic!("no frame holds '{needle}'"))
        };
        let frame_exact = |text_wanted: &str| {
            placed.iter().find(|(text, _)| text == text_wanted).map(|(_, r)| *r).unwrap_or_else(|| panic!("no frame holds exactly '{text_wanted}'"))
        };

        // Position lock: catalogue, then ISSN/ISBN, in the top-right corner over the photograph.
        let cat = frame("Katalog BPS");
        let code = frame("ISSN");
        assert!(cat.y1 < page.y0 + 60.0 && cat.x0 > page.center().x, "the catalogue sits in the top-right: {cat:?}");
        assert!(code.y0 >= cat.y1 - 1.0 && code.y1 < page.y0 + 70.0, "ISSN/ISBN sit under the catalogue: {code:?}");

        // Headline: the title and its second line down the left of the dark block.
        let title = frame("KARAWANG");
        let suffix = frame("Title 2");
        assert!(title.y1 <= suffix.y0 + 1.0, "headline order: {title:?} {suffix:?}");
        assert!(suffix.y1 < page.y0 + 740.0, "the headline sits inside the dark block: {suffix:?}");

        // Year in the accent block, with the volume directly underneath it.
        let year = frame_exact("2026");
        let volume = frame("Volume");
        assert!(year.x0 > page.center().x, "the year sits in the accent block: {year:?}");
        assert!(volume.y0 >= year.y1 - 1.0, "the volume sits directly under the year: {year:?} {volume:?}");

        // Position lock: agency logo text in the bottom-left corner.
        let agency = frame("BADAN PUSAT STATISTIK");
        assert!(agency.y0 > page.y0 + 700.0 && agency.x0 < page.center().x, "the agency sits in the bottom-left: {agency:?}");

        // Every object is upright: nothing on this cover is rotated.
        let turned =
            active.doc.spreads.iter().flat_map(|s| s.items.iter()).any(|it| designcraft_geom::decompose::decompose(it.xf).rotation.abs() > 1.0);
        assert!(!turned, "every object on the Modern Horizon cover is upright");
    }

    /// A paragraph list nests chapter titles over the heading styles the RTF imports carry.
    #[test]
    fn test_toc_paragraph_kind_tables_and_plan() {
        assert_eq!(toc_kind_words(3), ["BAB"]);
        assert_eq!(toc_kind_title(3, false), Some("Daftar Isi"));
        assert_eq!(toc_kind_title(3, true), Some("Daftar Isi / Table of Contents"));
        assert_eq!(toc_kind_of_words(&["BAB".into()]), 3);
        // The older kinds keep their detection.
        assert_eq!(toc_kind_of_words(&["Tabel".into(), "Table".into()]), 1);
        assert_eq!(toc_kind_of_words(&["Grafik".into(), "Chart".into()]), 2);
        assert_eq!(toc_kind_of_words(&[]), 0);

        let st = PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() };
        let plan = TocPlan::from_state(&st);
        assert!(plan.is_paragraph_toc());
        let levels: Vec<(&str, u8)> = plan.levels.iter().map(|l| (l.style.as_str(), l.level)).collect();
        assert_eq!(levels, [(TOC_NARRATIVE_STYLE, 1), ("Heading 1", 2), ("Heading 2", 3)]);
        let json = plan.entries_json();
        let arr = json.as_array().expect("entries array");
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["chapterNumber"], json!(true), "the narrative entry carries the chapter line");
        assert!(arr[0].get("pageTitle").is_none(), "the narrative style alone picks the narrative pages");
        assert!(arr[1].get("pageTitle").is_none(), "heading levels list every paragraph of the style");

        // The plain list keeps the tagged page titles and adds the narrative pages.
        let st = PubgenState { toc_kind: 0, toc_style: "TOC Entry".into(), ..Default::default() };
        let plan = TocPlan::from_state(&st);
        assert_eq!(plan.entries_json().as_array().map(Vec::len), Some(2));

        // A single-level kind still emits one entry.
        let st = PubgenState { toc_kind: 1, toc_style: "TOC Entry".into(), ..Default::default() };
        let plan = TocPlan::from_state(&st);
        assert!(!plan.is_paragraph_toc());
        assert_eq!(plan.entries_json().as_array().map(Vec::len), Some(1));
        assert_eq!(plan.kind(), 1);
    }

    /// Add a text frame on absolute `page` and tag its paragraphs with `style`.
    fn add_page_text(app: &mut DesignApp, page: usize, text: &str, style: &str) {
        add_page_text_at(app, page, text, style, 0.0);
    }

    /// The same, `dy` points below the top margin: a page with two text boxes (the chapter line
    /// and the title below it).
    fn add_page_text_at(app: &mut DesignApp, page: usize, text: &str, style: &str, dy: f64) {
        let (si, pi) = app.session.active().expect("document").doc.page_loc(page).expect("page");
        let b = app.session.active().expect("document").doc.spreads[si].pages[pi].bounds();
        let rect = [b.x0 + 36.0, b.y0 + 36.0 + dy, b.x1 - 36.0, b.y0 + 140.0 + dy];
        let res =
            app.run("frame.create", json!({"spread": si, "rect": rect, "content": "text", "text": text, "caret": false})).expect("frame.create");
        let sid = res["story"].as_u64().expect("story");
        let end = text.chars().count().max(1) as u64;
        app.run("text.select", json!({"story": sid, "anchor": 0, "focus": end})).expect("text.select");
        app.run("style.paragraph.apply", json!({"name": style})).expect("style.paragraph.apply");
    }

    /// The text of the table of contents whose first frame sits on `page`.
    fn toc_text_on_page(app: &DesignApp, page: usize) -> Option<String> {
        let active = app.session.active()?;
        let toc = active.doc.tocs.iter().find(|t| toc_story_page(app, t.story) == Some(page))?;
        active.doc.story(toc.story).map(|s| s.text.clone())
    }

    /// Four content pages (chapter, two headings, a table) plus a blank page for the TOC.
    fn paragraph_toc_doc(app: &mut DesignApp) {
        app.run("file.new", json!({})).unwrap();
        for name in ["TOC Entry", TOC_NARRATIVE_STYLE, "Heading 1", "Heading 2"] {
            ensure_para_style(app, name);
        }
        // A narrative page: the chapter line in its own text box, the title below it.
        add_page_text_at(app, 0, "BAB 01", "TOC Entry", 0.0);
        add_page_text_at(app, 0, "Geografi", TOC_NARRATIVE_STYLE, 40.0);
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        add_page_text(app, 1, "Pendahuluan", "Heading 1");
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        add_page_text(app, 2, "Latar Belakang", "Heading 2");
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        add_page_text(app, 3, "Tabel 01.01 Jarak Antar Kecamatan", "TOC Entry");
        // The table of contents lands on the last page (no canvas in tests).
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
    }

    #[test]
    fn test_toc_paragraph_scan_finds_styled_paragraphs() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        paragraph_toc_doc(&mut app);

        let st = PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() };
        let plan = TocPlan::from_state(&st);
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();

        assert_eq!(
            rows.iter().map(|(l, t)| (*l, t.as_str())).collect::<Vec<_>>(),
            [(1, "BAB 01 Geografi"), (2, "Pendahuluan"), (3, "Latar Belakang")],
            "the narrative page at level 1 with its chapter line, the headings nested under it, the table page left out: {rows:?}"
        );
    }

    #[test]
    fn test_toc_paragraph_keeps_toc_level_styles_and_daftar_isi_title() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        paragraph_toc_doc(&mut app);

        let mut st = PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), toc_title: "Daftar Isi".into(), ..Default::default() };
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");
        assert!(res.as_deref().unwrap_or_default().contains("TOC Paragraph"), "{}", res.unwrap_or_default());

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let story = active.doc.story(toc.story).expect("TOC story");
        assert!(story.text.starts_with("Daftar Isi"), "the title line: {}", story.text);
        assert!(story.text.contains("BAB 01 Geografi"), "the narrative entry reads chapter line and title: {}", story.text);
        assert!(story.text.contains("Pendahuluan") && story.text.contains("Latar Belakang"), "{}", story.text);
        assert!(!story.text.contains("Tabel 01.01"), "table pages stay out of a paragraph list: {}", story.text);

        let styles: Vec<&str> = story.paras.iter().map(|p| p.style.as_str()).collect();
        assert_eq!(
            styles,
            ["TOC Title", "TOC Level 1", "TOC Level 2", "TOC Level 3"],
            "each entry keeps its own level style instead of being flattened: {styles:?}"
        );
    }

    #[test]
    fn test_toc_include_self_lists_the_toc_page_in_a_paragraph_list() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        paragraph_toc_doc(&mut app);

        let mut st =
            PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), toc_title: "Daftar Isi".into(), toc_include_self: true, ..Default::default() };
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let first_frame = active.doc.story(toc.story).expect("TOC story").frames.first().copied().expect("frame");
        let toc_page = active.doc.page_of_item(first_frame).expect("page");
        let label = app.session.page_label(toc_page);
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains(&format!("Daftar Isi\t{label}")), "the list names the table of contents itself: {text}");

        // Without the option the entry is gone again.
        st.toc_include_self = false;
        assert!(execute_rebuild_toc(&mut app, &mut st).is_ok());
        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(!text.contains(&format!("Daftar Isi\t{label}")), "the entry is gone when the option is off: {text}");
    }

    #[test]
    fn test_toc_self_entry_sits_in_page_order() {
        // The entry that lists the table of contents itself falls where its page falls among the
        // entries: first when the table opens the document, last when it closes it.
        let cases = [(0_usize, "Daftar Isi\t1\nBAB 01 Geografi\t3"), (4, "Pendahuluan\t4\nDaftar Isi\t5")];
        for (toc_page, expected) in cases {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({})).unwrap();
            ensure_para_style(&mut app, "TOC Entry");
            app.run("layout.pages.insert", json!({"count": 4})).unwrap();
            let (si, pi) = app.session.active().unwrap().doc.page_loc(0).unwrap();
            let b = app.session.active().unwrap().doc.spreads[si].pages[pi].bounds();
            let rect = [b.x0 + 36.0, b.y0 + 36.0, b.x1 - 36.0, b.y0 + 200.0];
            let res = app
                .run(
                    "frame.create",
                    json!({"spread": si, "rect": rect, "content": "text", "caret": false, "text": "Daftar Isi\nBAB 01 Geografi\t3\nPendahuluan\t4"}),
                )
                .expect("frame.create");
            let sid = designcraft_doc::StoryId(res["story"].as_u64().expect("story"));
            let st = PubgenState {
                toc_kind: 3,
                toc_style: "TOC Entry".into(),
                toc_title: "Daftar Isi".into(),
                toc_include_self: true,
                ..Default::default()
            };
            let plan = TocPlan::from_state(&st);
            add_toc_self_entry(&mut app, sid, &plan, toc_page);

            let active = app.session.active().unwrap();
            let text = active.doc.story(sid).expect("story").text.clone();
            assert!(text.contains(expected), "on page {}: {text}", toc_page + 1);
            assert_eq!(text.split('\n').count(), 4, "one paragraph per entry: {text}");
        }
    }

    #[test]
    fn test_toc_table_style_lists_only_table_pages_and_centers_its_title() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Grafik".into(), "1.1".into(), "Curah Hujan Tiap Bulan".into()],
                vec!["Tabel".into(), "01.02.".into(), "Keadaan Curah Hujan".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_preset: 5, // "TOC Line": each page's text box reads "Tabel 01.01. Jarak …"
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        // Quick Actions TOC style = "TOC Table" (its title was filled in when it was picked).
        st.toc_kind = 1;
        st.toc_title = toc_kind_title(1, false).unwrap().to_string();
        assert_eq!(st.toc_title, "Daftar Tabel");
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story = active.doc.story(toc.story).unwrap();
        let text = story.text.clone();
        assert!(text.starts_with("Daftar Tabel"), "{text}");
        assert!(text.contains("Tabel 01.01. Jarak Antar Kecamatan"), "{text}");
        assert!(text.contains("Tabel 01.02. Keadaan Curah Hujan"), "{text}");
        assert!(!text.contains("Grafik 1.1."), "a list of tables lists no charts: {text}");
        assert_eq!(story.paras[0].style, "TOC Title");
        assert_eq!(story.paras[0].para.align, Some(designcraft_doc::Align::Center), "the title is centered");
        // The filter lives in the document, so the table stays a list of tables.
        assert_eq!(toc.entries[0].page_title, ["Tabel", "Table"]);
        assert_eq!(toc_kind_of_words(&toc.entries[0].page_title), 1);
    }

    #[test]
    fn test_toc_chart_page_layout_names_charts_in_its_column_header() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Grafik".into(), "1.1".into(), "Curah Hujan Tiap Bulan".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_preset: 5,
            toc_style: "TOC Entry".into(),
            toc_layout_style: 1, // TablePage (BPS catalog) writes a Tabel/Grafik column header
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        st.toc_kind = 2;
        st.toc_title = toc_kind_title(2, false).unwrap().to_string();
        assert_eq!(st.toc_title, "Daftar Grafik");
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = active.doc.story(toc.story).unwrap().text.clone();
        assert!(text.contains("Grafik\thalaman"), "a list of charts heads its column with Grafik: {text}");
        assert!(!text.contains("Tabel\thalaman"), "{text}");
        assert!(text.contains("Curah Hujan Tiap Bulan"), "{text}");
        assert!(!text.contains("Jarak Antar Kecamatan"), "a list of charts lists no tables: {text}");

        // The header follows the kind and the language of the source records: an English chart.
        st.card1_records = vec![vec!["Chart".into(), "1.1".into(), "Population Pyramid".into()]];
        assert_eq!(toc_header_line(&TocPlan::from_state(&st), &st), "Chart\tpage");
        st.card1_records = vec![vec!["Tabel".into(), "1.1".into(), "Luas Wilayah".into()]];
        assert_eq!(toc_header_line(&TocPlan::from_state(&st), &st), "Grafik\thalaman");
    }

    #[test]
    fn test_toc_chart_tablepage_layout_format() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let img_path = sample_image_path();
        let mut st = PubgenState {
            card1_records: vec![vec!["Grafik".into(), "02.01".into(), "Grafik 02.01 Panjang Jalan Kecamatan Karawang Timur 2026".into(), img_path]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            toc_layout_style: 1, // TablePage
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        st.toc_kind = 2; // TOC Chart
        st.toc_title = "Daftar Grafik".into();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = active.doc.story(toc.story).unwrap().text.clone();
        assert!(text.contains("Grafik\thalaman"), "header must be present: {text}");
        // Must contain single line for the chart matching TablePage layout:
        // "02.01\tPanjang Jalan Kecamatan Karawang Timur 2026\t<page>"
        assert!(
            text.contains("02.01\tPanjang Jalan Kecamatan Karawang Timur 2026"),
            "Should format chart number and title into TablePage columns: {text}"
        );
        assert!(!text.contains("Grafik 02.01\t"), "Should not contain raw 'Grafik 02.01' line: {text}");
    }

    #[test]
    fn test_toc_chart_tablepage_layout_with_existing_newline_paragraphs() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Create page 1 with 2 separate paragraphs in a single frame, both with style "TOC Entry"
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        ensure_para_style(&mut app, "TOC Entry");
        let (spread_idx, page_idx) = app.session.active().unwrap().doc.page_loc(1).unwrap();
        let margin = app.session.active().unwrap().doc.spreads[spread_idx].pages[page_idx].margin_rect();
        frame_with_centered_bold_title(
            &mut app,
            spread_idx,
            [margin.x0, margin.y0, margin.x1, margin.y0 + 48.0],
            "Grafik 02.01\nPanjang Jalan Kecamatan Karawang Timur 2026",
            Some("TOC Entry"),
        );

        let mut st = PubgenState {
            toc_kind: 2,
            toc_title: "Daftar Grafik".into(),
            toc_style: "TOC Entry".into(),
            toc_layout_style: 1, // TablePage
            ..Default::default()
        };

        crate::canvas::go_to_page(&mut app, 0);
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = active.doc.story(toc.story).unwrap().text.clone();
        assert!(text.contains("Grafik\thalaman"), "header must be present: {text}");
        assert!(
            text.contains("02.01\tPanjang Jalan Kecamatan Karawang Timur 2026\t2"),
            "Should coalesce the 2 paragraphs on page 2 into one TablePage entry: {text}"
        );
        assert!(!text.contains("Grafik 02.01\t"), "Should not contain raw 'Grafik 02.01' line: {text}");
    }

    #[test]
    fn test_rebuild_toc_rebuilds_only_the_table_of_contents_in_view() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Grafik".into(), "1.1".into(), "Curah Hujan Tiap Bulan".into()],
            ],
            card1_preset: 5,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        // A list of tables on one page, a list of charts on the next.
        app.run("layout.pages.insert", json!({"count": 2})).unwrap();
        let pages = app.session.active().unwrap().doc.page_count();
        app.canvas_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0)));
        crate::canvas::go_to_page(&mut app, pages - 2);
        st.toc_kind = 1;
        st.toc_title = "Daftar Table".into();
        execute_add_toc(&mut app, &mut st).unwrap();
        crate::canvas::go_to_page(&mut app, pages - 1);
        st.toc_kind = 2;
        st.toc_title = "Daftar Grafik".into();
        execute_add_toc(&mut app, &mut st).unwrap();

        let (tables_sid, charts_sid) = {
            let doc = &app.session.active().unwrap().doc;
            assert_eq!(doc.tocs.len(), 2, "{:?}", doc.tocs);
            (doc.tocs[0].story, doc.tocs[1].story)
        };
        let before = app.session.active().unwrap().doc.story(tables_sid).unwrap().text.clone();

        // Pressing Rebuild with the panel back on "TOC Entry"/"Contents" rebuilds the table on the
        // page in view with the TOC style it was created with, and leaves the other one alone.
        st.toc_kind = 0;
        st.toc_title = "Contents".into();
        let msg = execute_rebuild_toc(&mut app, &mut st).unwrap();
        assert!(msg.contains("Rebuilt TOC"), "{msg}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.tocs.len(), 2, "a rebuild adds no third table");
        let charts = active.doc.tocs.iter().find(|t| t.story == charts_sid).unwrap();
        assert_eq!(charts.title, "Daftar Grafik", "the title it was made with");
        assert_eq!(charts.entries[0].page_title, ["Grafik", "Chart"], "still a list of charts");
        let text = active.doc.story(charts_sid).unwrap().text.clone();
        assert!(text.starts_with("Daftar Grafik"), "{text}");
        assert!(!text.contains("Tabel 01.01."), "{text}");
        assert_eq!(active.doc.story(tables_sid).unwrap().text, before, "the other table is untouched");
    }

    #[test]
    fn test_arial_and_arial_black_resolution() {
        let db = designcraft_fonts::FontDb::with_font_dirs(designcraft_fonts::system_font_dirs());
        let face_arial = db.face("Arial", "Regular");
        println!("face Arial Regular: family='{}' style='{}'", face_arial.family, face_arial.style);
        let face_ariblk = db.face("Arial Black", "Regular");
        println!("face Arial Black Regular: family='{}' style='{}'", face_ariblk.family, face_ariblk.style);
        let face_arial_blk = db.face("Arial", "Black");
        println!("face Arial Black: family='{}' style='{}'", face_arial_blk.family, face_arial_blk.style);
        let face_arial_bi = db.face("Arial", "Bold Italic");
        println!("face Arial Bold Italic: family='{}' style='{}'", face_arial_bi.family, face_arial_bi.style);
    }

    /// Opt-in evidence for review (`CRAFT_UI_VISUAL_FIXTURES=dir cargo test --features pubgen`): the
    /// whole Pubgen panel body in one image, TOC style set to "TOC Table".
    #[test]
    fn capture_pubgen_panel_fixture() {
        let Some(directory) = std::env::var_os("CRAFT_UI_VISUAL_FIXTURES").map(std::path::PathBuf::from) else { return };
        std::fs::create_dir_all(&directory).unwrap();
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let mut ready = false;
        let mut harness = egui_kittest::Harness::builder().with_size(vec2(520.0, 2400.0)).wgpu().build_ui_state(
            move |ui, app: &mut DesignApp| {
                if !ready {
                    crate::theme::install_fonts(ui.ctx(), "en");
                    crate::theme::apply(ui.ctx(), &Tokens::for_brightness(crate::theme::Brightness::MediumDark));
                    let st = PubgenState { toc_kind: 1, toc_title: "Daftar Table".into(), ..Default::default() };
                    ui.data_mut(|d| d.insert_temp(egui::Id::new(PANEL_ID), st));
                    ready = true;
                    return;
                }
                show(app, ui);
            },
            app,
        );
        harness.input_mut().max_texture_side = Some(8192);
        harness.run_steps(4);
        harness.render().unwrap().save(directory.join("pubgen-toc-table.png")).unwrap();
    }

    #[test]
    fn test_split_chapter_title() {
        assert_eq!(split_chapter_title("BAB 01 Pemerintahan", "01"), ("BAB 01".into(), "Pemerintahan".into()));
        assert_eq!(split_chapter_title("BAB 01. Pemerintahan Daerah", "01"), ("BAB 01".into(), "Pemerintahan Daerah".into()));
        assert_eq!(split_chapter_title("BAB 2 Keadaan Alam", "02"), ("BAB 2".into(), "Keadaan Alam".into()));
        assert_eq!(split_chapter_title("Pemerintahan", "01"), ("BAB 01".into(), "Pemerintahan".into()));
    }

    #[test]
    fn test_execute_generate_pages_ulasan_rtf_layout() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("ulasan_test.rtf");
        let rtf_content = br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Bab satu ulasan pemerintahan daerah di Karawang Timur.}"#;
        std::fs::write(&rtf_path, rtf_content).unwrap();

        let st = PubgenState {
            card1_records: vec![vec!["Ulasan".into(), "01".into(), "BAB 01 Pemerintahan".into(), rtf_path.to_string_lossy().to_string()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for Ulasan should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("1 RTF chapter(s) placed"), "Summary should confirm RTF placed: {msg}");

        // Verify items on the generated page (page 1)
        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2, "1 initial page + 1 generated page");

        let (spread_idx, _) = active.doc.page_loc(1).unwrap();
        let spread = &active.doc.spreads[spread_idx];
        let texts: Vec<String> =
            spread.items.iter().filter_map(|it| it.text_frame()).filter_map(|tf| active.doc.story(tf.story)).map(|st| st.text.clone()).collect();

        // Must contain "BAB 01", "Pemerintahan", and the RTF text
        assert!(texts.iter().any(|t| t.contains("BAB 01")), "Page should contain 'BAB 01', found: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Pemerintahan")), "Page should contain 'Pemerintahan', found: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Bab satu ulasan pemerintahan")), "Page should contain imported RTF content, found: {texts:?}");
    }

    #[test]
    fn test_bxx_sheet_gets_rxx_page_layout() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("B01.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Kata pengantar untuk laporan ini.}"#).unwrap();

        // `B01` converts as `Ulasan` (see `is_b_sheet`), so its catalog row is the RXX shape.
        let st = PubgenState {
            card1_records: vec![vec!["Ulasan".into(), "01".into(), "Kata Pengantar".into(), rtf_path.to_string_lossy().to_string()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for a BXX-style Ulasan row should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("1 RTF chapter(s) placed"), "Summary should confirm RTF placed: {msg}");

        let active = app.session.active().unwrap();
        let (spread_idx, _) = active.doc.page_loc(1).unwrap();
        let spread = &active.doc.spreads[spread_idx];
        let stories: Vec<&designcraft_doc::Story> =
            spread.items.iter().filter_map(|it| it.text_frame()).filter_map(|tf| active.doc.story(tf.story)).collect();
        let texts: Vec<&str> = stories.iter().map(|s| s.text.as_str()).collect();

        assert!(!texts.iter().any(|t| t.contains("BAB 01")), "A title without a BAB prefix gets no chapter-number box, found: {texts:?}");
        assert!(texts.iter().any(|t| t.eq_ignore_ascii_case("Kata Pengantar")), "Page should contain 'Kata Pengantar', found: {texts:?}");
        let title_para = stories.iter().find(|s| s.text.contains("KATA PENGANTAR")).and_then(|s| s.paras.first()).expect("the centered title frame");
        assert_eq!(title_para.style, TOC_NARRATIVE_STYLE, "the title frame must carry the narrative style so the paragraph list names the page");
        assert_eq!(title_para.para.align, Some(designcraft_doc::Align::Center), "the front-matter title is centered on the margin");
        assert!(texts.iter().any(|t| t.contains("Kata pengantar untuk laporan ini")), "Page should contain the placed RTF body, found: {texts:?}");
    }

    #[test]
    fn test_bxx_bilingual_dual_spread_and_italics() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let id_rtf = temp_dir.path().join("B01.rtf");
        let en_rtf = temp_dir.path().join("B01_en.rtf");
        std::fs::write(&id_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Kata pengantar bahasa Indonesia.}"#).unwrap();
        std::fs::write(&en_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Preface narrative text in English.}"#).unwrap();
        let img1 = sample_image_path();
        let img2 = sample_image_path();

        let st = PubgenState {
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "Kata Pengantar".into(),
                id_rtf.to_string_lossy().to_string(),
                img1,
                img2,
                "Preface".into(),
                en_rtf.to_string_lossy().to_string(),
            ]],
            card1_headers: vec![
                "type".into(),
                "number".into(),
                "title".into(),
                "file".into(),
                "image1".into(),
                "image2".into(),
                "title_en".into(),
                "file_en".into(),
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            bilingual_output: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for bilingual BXX should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // 1 initial + 1 ID page + 1 EN page
        assert_eq!(active.doc.page_count(), 3);

        // 1. Indonesian page (page 1)
        let (id_si, _) = active.doc.page_loc(1).unwrap();
        let id_spread = &active.doc.spreads[id_si];
        let id_stories: Vec<&designcraft_doc::Story> = id_spread
            .items
            .iter()
            .filter(|it| active.doc.page_of_item(it.id) == Some(1))
            .filter_map(|it| it.text_frame())
            .filter_map(|tf| active.doc.story(tf.story))
            .collect();
        let id_texts: Vec<&str> = id_stories.iter().map(|s| s.text.as_str()).collect();

        assert!(id_texts.iter().any(|t| t.contains("KATA PENGANTAR")), "Indonesian page must contain 'KATA PENGANTAR', found: {id_texts:?}");
        assert!(!id_texts.iter().any(|t| t.contains("PREFACE")), "Indonesian page must NOT contain 'PREFACE', found: {id_texts:?}");
        assert!(!id_texts.iter().any(|t| t.contains("BAB 01")), "Indonesian front matter gets no BAB 01 box, found: {id_texts:?}");
        let id_graphic_frames = id_spread.items.iter().filter(|it| active.doc.page_of_item(it.id) == Some(1) && it.graphic().is_some()).count();
        assert_eq!(id_graphic_frames, 2, "Indonesian page must contain image1 and image2 graphic frames");

        // 2. English page (page 2)
        let (en_si, _) = active.doc.page_loc(2).unwrap();
        let en_spread = &active.doc.spreads[en_si];
        let en_stories: Vec<&designcraft_doc::Story> = en_spread
            .items
            .iter()
            .filter(|it| active.doc.page_of_item(it.id) == Some(2))
            .filter_map(|it| it.text_frame())
            .filter_map(|tf| active.doc.story(tf.story))
            .collect();
        let en_texts: Vec<&str> = en_stories.iter().map(|s| s.text.as_str()).collect();

        assert!(en_texts.iter().any(|t| t.contains("PREFACE")), "English page must contain 'PREFACE', found: {en_texts:?}");
        assert!(!en_texts.iter().any(|t| t.contains("CHAPTER 01")), "English front matter must NOT contain 'CHAPTER 01', found: {en_texts:?}");
        assert!(!en_texts.iter().any(|t| t.contains("KATA PENGANTAR")), "English page must NOT contain 'KATA PENGANTAR', found: {en_texts:?}");
        let en_graphic_frames = en_spread.items.iter().filter(|it| active.doc.page_of_item(it.id) == Some(2) && it.graphic().is_some()).count();
        assert_eq!(en_graphic_frames, 2, "English page must contain image1 and image2 graphic frames");

        // English page text should all be in italic
        for s in &en_stories {
            for run in &s.chars {
                let style = run.format.over.font_style.as_deref().unwrap_or_default();
                assert!(
                    style.eq_ignore_ascii_case("italic") || style.eq_ignore_ascii_case("bold italic"),
                    "English page text run must be italicized, found style: {style}"
                );
            }
        }
    }

    #[test]
    fn test_split_chart_title() {
        assert_eq!(
            split_chart_title("Grafik 02.01 Panjang Jalan Kecamatan Karawang Timur 2026", "02.01", "Grafik"),
            ("Grafik 02.01".into(), "Panjang Jalan Kecamatan Karawang Timur 2026".into())
        );
        assert_eq!(split_chart_title("Grafik 02.01", "02.01", "Grafik"), ("Grafik 02.01".into(), "".into()));
        assert_eq!(split_chart_title("02.01 Panjang Jalan", "02.01", "Grafik"), ("Grafik 02.01".into(), "Panjang Jalan".into()));
        assert_eq!(split_chart_title("Panjang Jalan Kecamatan", "02.01", "Grafik"), ("Grafik 02.01".into(), "Panjang Jalan Kecamatan".into()));
    }

    #[test]
    fn test_execute_generate_pages_grafik_image_layout() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let img_path = sample_image_path();

        let st = PubgenState {
            card1_records: vec![vec!["Grafik".into(), "02.01".into(), "Grafik 02.01 Panjang Jalan Kecamatan Karawang Timur 2026".into(), img_path]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for Grafik should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("1 chart image(s) placed"), "Summary should confirm chart image placed: {msg}");

        // Verify items on the generated page (page 1)
        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2, "1 initial page + 1 generated page");

        let (spread_idx, _) = active.doc.page_loc(1).unwrap();
        let spread = &active.doc.spreads[spread_idx];

        // 1. Check title text frame
        let texts: Vec<String> =
            spread.items.iter().filter_map(|it| it.text_frame()).filter_map(|tf| active.doc.story(tf.story)).map(|st| st.text.clone()).collect();
        assert!(
            texts.iter().any(|t| t.contains("Grafik 02.01") && t.contains("Panjang Jalan Kecamatan Karawang Timur 2026")),
            "Page must contain centered 2-line title, found: {texts:?}"
        );

        // 2. Check graphic frame placed inside margins
        let graphic_frame = spread.items.iter().find(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)));
        assert!(graphic_frame.is_some(), "Page must contain a placed graphic frame");
    }

    #[test]
    fn test_execute_set_page_margins() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let res = execute_set_page_margins(&mut app);
        assert!(res.is_ok(), "execute_set_page_margins should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let page = active.doc.page(0).expect("first page exists");

        const PT_PER_MM: f64 = 72.0 / 25.4;
        let expected_top = 20.20 * PT_PER_MM;
        let expected_bottom = 20.0 * PT_PER_MM;
        let expected_inside = 19.0 * PT_PER_MM;
        let expected_outside = 15.0 * PT_PER_MM;

        assert!((page.margins.top - expected_top).abs() < 1e-6, "Top margin mismatch: got {}, expected {}", page.margins.top, expected_top);
        assert!(
            (page.margins.bottom - expected_bottom).abs() < 1e-6,
            "Bottom margin mismatch: got {}, expected {}",
            page.margins.bottom,
            expected_bottom
        );
        assert!(
            (page.margins.inside - expected_inside).abs() < 1e-6,
            "Inside margin mismatch: got {}, expected {}",
            page.margins.inside,
            expected_inside
        );
        assert!(
            (page.margins.outside - expected_outside).abs() < 1e-6,
            "Outside margin mismatch: got {}, expected {}",
            page.margins.outside,
            expected_outside
        );

        // Also verify parent pages
        for sp in &active.doc.parents {
            for pg in &sp.pages {
                assert!((pg.margins.top - expected_top).abs() < 1e-6);
                assert!((pg.margins.bottom - expected_bottom).abs() < 1e-6);
                assert!((pg.margins.inside - expected_inside).abs() < 1e-6);
                assert!((pg.margins.outside - expected_outside).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn test_generate_all_canonical_templates() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let templates_dir = root.join("assets").join("templates");
        let dividers_dir = templates_dir.join("dividers");
        let tables_dir = templates_dir.join("data_tables");
        let front_matter_dir = templates_dir.join("front_matter");

        // Covers are hand-designed in `covers/` (copied from `covers_original/`), so this test
        // only generates the interior templates and never writes over them.
        std::fs::create_dir_all(&dividers_dir).unwrap();
        std::fs::create_dir_all(&tables_dir).unwrap();
        std::fs::create_dir_all(&front_matter_dir).unwrap();

        // 1. Divider
        {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({"width": 420.0, "height": 595.0, "pages": 1, "facingPages": false})).unwrap();
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, 420, 200], "content": "unassigned", "shape": "rectangle"})).unwrap();
            let _ = app.run("object.color", json!({"color": "#122647", "target": "fill"}));
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 60, 348, 40], "content": "text", "text": "BAB 1"}));
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 110, 348, 60], "content": "text", "text": "GEOGRAFI DAN IKLIM"}));
            let _ =
                app.run("frame.create", json!({"spread": 0, "rect": [36, 230, 348, 320], "content": "text", "text": "Deskripsi pengantar bab..."}));

            let active = app.session.active_mut().unwrap();
            let doc = std::sync::Arc::make_mut(&mut active.doc);
            for sp in &mut doc.spreads {
                let sp = std::sync::Arc::make_mut(sp);
                for it in &mut sp.items {
                    let it = std::sync::Arc::make_mut(it);
                    if let designcraft_doc::Content::Text(tf) = &it.content
                        && let Some(story) = doc.stories.get(&tf.story)
                    {
                        if story.text.contains("BAB 1") {
                            it.label = "#slot:text:chapter_num".to_string();
                        } else if story.text.contains("GEOGRAFI") {
                            it.label = "#slot:text:chapter_title".to_string();
                        } else {
                            it.label = "#slot:story:narrative_body".to_string();
                        }
                    }
                }
            }
            let target = dividers_dir.join("001_ulasan_chapter.designcraft");
            app.run("file.saveAs", json!({"path": target.to_string_lossy()})).unwrap();
        }

        // 2. Data Table
        {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({"width": 420.0, "height": 595.0, "pages": 1, "facingPages": false})).unwrap();
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 36, 384, 52], "content": "text", "text": "Tabel 1.1"}));
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 52, 384, 68], "content": "text", "text": "Table 1.1"}));
            let _ = app.run(
                "frame.create",
                json!({"spread": 0, "rect": [36, 70, 384, 90], "content": "text", "text": "Luas Wilayah Menurut Desa/Kelurahan"}),
            );
            let _ = app.run(
                "frame.create",
                json!({"spread": 0, "rect": [36, 90, 384, 110], "content": "text", "text": "Total Area by Village/Urban Village"}),
            );
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 120, 384, 520], "content": "unassigned", "shape": "rectangle"}));
            let _ = app
                .run("frame.create", json!({"spread": 0, "rect": [36, 530, 384, 555], "content": "text", "text": "Sumber: BPS Kabupaten Karawang"}));

            let active = app.session.active_mut().unwrap();
            let doc = std::sync::Arc::make_mut(&mut active.doc);
            for sp in &mut doc.spreads {
                let sp = std::sync::Arc::make_mut(sp);
                for it in &mut sp.items {
                    let it = std::sync::Arc::make_mut(it);
                    match &it.content {
                        designcraft_doc::Content::Text(tf) => {
                            if let Some(story_arc) = doc.stories.get_mut(&tf.story) {
                                let story = std::sync::Arc::make_mut(story_arc);
                                if story.text.contains("Tabel") {
                                    it.label = "#slot:text:table_num".to_string();
                                } else if story.text.contains("Table") {
                                    it.label = "#slot:text:table_kind_en".to_string();
                                    for r in &mut story.chars {
                                        r.format.over.font_style = Some("Italic".to_string());
                                    }
                                } else if story.text.contains("Luas") {
                                    it.label = "#slot:text:table_title".to_string();
                                } else if story.text.contains("Total Area") {
                                    it.label = "#slot:text:table_title_en".to_string();
                                    for r in &mut story.chars {
                                        r.format.over.font_style = Some("Italic".to_string());
                                    }
                                } else {
                                    it.label = "#slot:text:source".to_string();
                                }
                            }
                        }
                        designcraft_doc::Content::Unassigned | designcraft_doc::Content::Graphic(_) => {
                            it.label = "#slot:table:main".to_string();
                        }
                        _ => {}
                    }
                }
            }
            let target = tables_dir.join("001_single_table_standard.designcraft");
            app.run("file.saveAs", json!({"path": target.to_string_lossy()})).unwrap();
        }

        // 3. Publication Note
        {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({"width": 420.0, "height": 595.0, "pages": 1, "facingPages": false})).unwrap();
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 50, 348, 40], "content": "text", "text": "KATA PENGANTAR"}));
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 110, 348, 440], "content": "text", "text": "Publikasi Kecamatan Karawang Timur Dalam Angka 2026..."}));

            let active = app.session.active_mut().unwrap();
            let doc = std::sync::Arc::make_mut(&mut active.doc);
            for sp in &mut doc.spreads {
                let sp = std::sync::Arc::make_mut(sp);
                for it in &mut sp.items {
                    let it = std::sync::Arc::make_mut(it);
                    if let designcraft_doc::Content::Text(tf) = &it.content
                        && let Some(story) = doc.stories.get(&tf.story)
                    {
                        if story.text.contains("KATA PENGANTAR") {
                            it.label = "#slot:text:title".to_string();
                        } else {
                            it.label = "#slot:story:note_body".to_string();
                        }
                    }
                }
            }
            let target = front_matter_dir.join("001_publication_note.designcraft");
            app.run("file.saveAs", json!({"path": target.to_string_lossy()})).unwrap();
        }
    }

    #[test]
    fn test_execute_generate_pages_modular_template_divider() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("ch1.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Ulasan wilayah terinci.}"#).unwrap();

        let st = PubgenState {
            card1_preset: 6,
            card1_records: vec![vec!["Ulasan".into(), "01".into(), "BAB 01 Geografi".into(), rtf_path.to_string_lossy().to_string()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for modular template divider should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2);
        let (si, pi) = active.doc.page_loc(1).unwrap();
        let pg = &active.doc.spreads[si].pages[pi];
        eprintln!("ULASAN PAGE 1: side={:?}, bounds={:?}, margins={:?}, margin_rect={:?}", pg.side, pg.bounds(), pg.margins, pg.margin_rect());
        for it in &active.doc.spreads[si].items {
            eprintln!("  ITEM: label='{}', bounds={:?}", it.label, it.bounds());
        }

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("BAB 01")), "Stories should contain BAB 01: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Geografi")), "Stories should contain Geografi: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Ulasan wilayah terinci")), "Stories should contain RTF text: {texts:?}");
    }

    #[test]
    fn test_execute_generate_pages_modular_template_note() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("note.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Penjelasan teknis publikasi.}"#).unwrap();

        let st = PubgenState {
            card1_preset: 6,
            card1_records: vec![vec!["Note".into(), "".into(), "Keterangan Publikasi".into(), rtf_path.to_string_lossy().to_string()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for modular template note should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("Keterangan Publikasi")), "Stories should contain note title: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Penjelasan teknis publikasi")), "Stories should contain RTF note body: {texts:?}");
    }

    #[test]
    fn test_execute_generate_pages_modular_template_table() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let csv_path = temp_dir.path().join("table_1_1.csv");
        std::fs::write(&csv_path, "Kecamatan,Luas\nKarawang Barat,30.5\nKarawang Timur,25.2\nTotal,55.7\n").unwrap();

        let st = PubgenState {
            card1_preset: 6,
            card1_records: vec![vec!["Tabel".into(), "1.1".into(), "Luas Wilayah Menurut Kecamatan".into(), csv_path.to_string_lossy().to_string()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_auto_insert_tables: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for modular template table should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("01.01")), "Stories should contain table number: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Luas Wilayah")), "Stories should contain table title: {texts:?}");

        let has_table = active.doc.stories.values().any(|s| !s.tables.is_empty());
        assert!(has_table, "Document should contain an inserted table");
    }

    #[test]
    fn test_table_bilingual_generation() {
        // 1. Test split_table_title
        let (id, en) = locale::split_table_title("Banyaknya Penduduk / Number of Population");
        assert_eq!(id, "Banyaknya Penduduk");
        assert_eq!(en.as_deref(), Some("Number of Population"));

        let (id_single, en_none) = locale::split_table_title("Banyaknya Penduduk");
        assert_eq!(id_single, "Banyaknya Penduduk");
        assert_eq!(en_none, None);

        // 2. Test Preset 0 bilingual generation
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec!["Tabel".into(), "01.01".into(), "Luas Daerah / Total Area".into(), "".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "Bilingual Preset 0 table should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        // Col 1 has Tabel and Table joined by forced line break
        assert!(texts.iter().any(|t| t.contains("Tabel") && t.contains("Table")), "Col 1 should have Tabel and Table: {texts:?}");
        // Col 3 has Luas Daerah and Total Area
        assert!(
            texts.iter().any(|t| t.contains("Luas Daerah") && t.contains("Total Area")),
            "Col 3 should have Luas Daerah and Total Area: {texts:?}"
        );

        // 3. Test Modular Template bilingual generation
        let mut app2 = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app2.run("file.new", json!({})).unwrap();

        let mut st2 = st.clone();
        st2.card1_preset = 6;
        let res2 = execute_generate_pages(&mut app2, &st2);
        assert!(res2.is_ok(), "Bilingual Modular table should succeed: {res2:?}");
        let active2 = app2.session.active().unwrap();
        let texts2: Vec<String> = active2.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts2.iter().any(|t| t.contains("Luas Daerah")), "Template should bind table_title: {texts2:?}");
        assert!(texts2.iter().any(|t| t.contains("Total Area")), "Template should bind table_title_en: {texts2:?}");
    }

    #[test]
    fn test_table_sheet_xx_xx_bilingual_header_textboxes() {
        let first_row = vec![
            "Tabel 01.01 Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur / Land Area by Village/Sub-district in East Karawang District, 2025".to_string()
        ];
        let (t, n, title, file) = excel::parse_catalog_entry(&first_row, "01_01");
        assert_eq!(t, "Tabel");
        assert_eq!(n, "01.01");
        assert_eq!(file, "01_01");
        assert_eq!(
            title,
            "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur / Land Area by Village/Sub-district in East Karawang District, 2025"
        );

        let (id_harmonized, en_harmonized_opt) = excel::split_and_harmonize_table_title(&title, None);
        assert_eq!(id_harmonized, "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025");
        assert_eq!(en_harmonized_opt.as_deref(), Some("Land Area by Village/Sub-district in East Karawang District, 2025"));

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec![t, n, title, file]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "Page generation should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let stories: Vec<_> = active.doc.stories.values().collect();

        // 1. Textbox 1 contains 2 lines: "Tabel\nTable" with "Table" in Italic
        let box1 = stories.iter().find(|s| s.text.contains("Tabel") && s.text.contains("Table")).expect("Textbox 1 found");
        let (p_line1, p_line2) = box1.text.split_once(designcraft_doc::story::FORCED_LINE_BREAK).expect("2 lines in textbox 1");
        assert_eq!(p_line1, "Tabel");
        assert_eq!(p_line2, "Table");

        // 2. Textbox 2 contains XX.XX ("01.01")
        let box2 = stories.iter().find(|s| s.text == "01.01").expect("Textbox 2 found with 01.01");
        assert_eq!(box2.text, "01.01");

        // 3. Textbox 3 contains 2 lines:
        // Line 1: "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025"
        // Line 2: "Land Area by Village/Sub-district in East Karawang District, 2025"
        let box3 = stories
            .iter()
            .find(|s| s.text.contains("Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025"))
            .expect("Textbox 3 found");
        let (t3_line1, t3_line2) = box3.text.split_once(designcraft_doc::story::FORCED_LINE_BREAK).expect("2 lines in textbox 3");
        assert_eq!(t3_line1, "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025");
        assert_eq!(t3_line2, "Land Area by Village/Sub-district in East Karawang District, 2025");
    }

    #[test]
    fn test_chart_bilingual_parts_and_italics() {
        let title = "Grafik / Chart 02.01 Panjang Jalan / Road Length, 2026";
        let parts = locale::split_chart_title_parts(title);
        assert_eq!(parts.id_kicker, "Grafik");
        assert_eq!(parts.en_kicker.as_deref(), Some("Chart"));
        assert_eq!(parts.number, "02.01");
        assert_eq!(parts.id_subtitle, "Panjang Jalan");
        assert_eq!(parts.en_subtitle.as_deref(), Some("Road Length"));
        assert_eq!(parts.suffix, ", 2026");

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec!["Grafik".into(), "02.01".into(), title.into(), "".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "Bilingual chart generation should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let chart_story = active.doc.stories.values().find(|s| s.text.contains("Grafik / Chart")).expect("Chart story must exist");
        assert!(chart_story.text.contains("Grafik / Chart 02.01"), "Line 1 should have kicker and number");
        assert!(chart_story.text.contains("Panjang Jalan / Road Length, 2026"), "Line 2 should have subtitle and suffix");

        // Verify that 'Chart' and 'Road Length' runs have Bold Italic
        let has_italic_chart = chart_story
            .chars
            .iter()
            .any(|r| r.format.over.font_style.as_deref() == Some("Bold Italic") || r.format.over.font_style.as_deref() == Some("Italic"));
        assert!(has_italic_chart, "Chart title must have italicised English parts");
    }

    #[test]
    fn test_chart_bilingual_three_line_title_from_title_en() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec![
                "Grafik".into(),
                "02.01".into(),
                "Panjang Jalan Kecamatan Karawang Timur 2026".into(),
                "".into(),
                "".into(),
                "Chart 02.01 Road Length in East Karawang District, 2025".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "title_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "3-line bilingual chart generation should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let chart_story = active.doc.stories.values().find(|s| s.text.contains("Grafik 02.01")).expect("Chart story must exist");

        // Verify 3 lines separated by FORCED_LINE_BREAK
        let lines: Vec<&str> = chart_story.text.split(designcraft_doc::story::FORCED_LINE_BREAK).collect();
        assert_eq!(lines.len(), 3, "Expected 3 lines, got {:?}", lines);
        assert_eq!(lines[0], "Grafik 02.01");
        assert_eq!(lines[1], "Panjang Jalan Kecamatan Karawang Timur 2026");
        assert_eq!(lines[2], "Chart 02.01 Road Length in East Karawang District, 2025");

        // Verify font styling: Line 1 and Line 2 are Bold 11pt, Line 3 is Italic 9pt
        let en_line_text = "Chart 02.01 Road Length in East Karawang District, 2025";
        let en_start = chart_story.text.find(en_line_text).expect("EN line must be in text");

        let mut curr_offset = 0;
        for run in &chart_story.chars {
            let run_start = curr_offset;
            let run_end = curr_offset + run.len;
            curr_offset = run_end;

            if run_end <= en_start {
                assert_eq!(run.format.over.font_style.as_deref(), Some("Bold"), "Lines 1-2 must be Bold");
                assert_eq!(run.format.over.size, Some(11.0), "Lines 1-2 must be 11pt");
            } else if run_start >= en_start {
                assert_eq!(run.format.over.font_style.as_deref(), Some("Italic"), "Line 3 must be Italic");
                assert_eq!(run.format.over.size, Some(9.0), "Line 3 must be 9pt (smaller font)");
            }
        }
    }

    #[test]
    fn test_resolve_template_path_spec() {
        // Direct template ID resolution from assets/templates/
        let r1 = resolve_template_path_spec("", "001_ulasan_chapter");
        assert!(r1.is_some(), "Should resolve 001_ulasan_chapter: {r1:?}");
        assert!(r1.unwrap().contains("001_ulasan_chapter.designcraft"));

        // Subcategory path resolution
        let r2 = resolve_template_path_spec("", "dividers/001_ulasan_chapter");
        assert!(r2.is_some(), "Should resolve dividers/001_ulasan_chapter: {r2:?}");

        // Table template resolution
        let r3 = resolve_template_path_spec("", "001_single_table_standard");
        assert!(r3.is_some(), "Should resolve 001_single_table_standard: {r3:?}");

        // Non-existent template returns None
        let r_none = resolve_template_path_spec("", "completely_fake_template_9999");
        assert!(r_none.is_none());
    }

    #[test]
    fn test_execute_generate_pages_with_explicit_template_column() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("ch2.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Ulasan sosial Bab 2.}"#).unwrap();

        // Note: card1_preset is set to 0 (procedural preset), but column 4 explicitly
        // specifies a template. It MUST use the modular template!
        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec![
                "Ulasan".into(),
                "02".into(),
                "BAB 02 Sosial".into(),
                rtf_path.to_string_lossy().to_string(),
                "dividers/001_ulasan_chapter".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages with explicit template column should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("BAB 02")), "Stories should contain BAB 02: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Sosial")), "Stories should contain Sosial: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Ulasan sosial Bab 2")), "Stories should contain RTF text: {texts:?}");
    }

    #[test]
    fn test_execute_generate_pages_divider_with_002_template() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let img_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/infografik_01.png");

        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec!["Divider".into(), "02".into(), "BAB 02 PEMERINTAHAN".into(), img_path.into(), "002".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for divider should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        // 3 textbox values from col title:
        // first_word replacing "BAB" -> "BAB"
        assert!(texts.iter().any(|t| t.contains("BAB")), "Stories should contain BAB: {texts:?}");
        // digit number replacing "01" -> "02"
        assert!(texts.iter().any(|t| t.contains("02")), "Stories should contain 02: {texts:?}");
        // rest of text replacing "Geografi" -> "PEMERINTAHAN"
        assert!(texts.iter().any(|t| t.contains("PEMERINTAHAN")), "Stories should contain PEMERINTAHAN: {texts:?}");

        // Shape with color #e5007e has image placed into it
        let has_image = active.doc.spreads.iter().flat_map(|sp| &sp.items).any(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)));
        assert!(has_image, "Divider page should contain placed image");

        // Verify page geometry is full A4 (210 x 297 mm)
        let last_sp = active.doc.spreads.last().unwrap();
        let last_p = last_sp.pages.last().unwrap();
        assert!((last_p.width - 595.27559).abs() < 0.1, "Divider page should be full A4 width 595.28: {}", last_p.width);
        assert!((last_p.height - 841.88976).abs() < 0.1, "Divider page should be full A4 height 841.89: {}", last_p.height);

        // Verify image fits frame with fitContentToFrame
        let img_item = last_sp.items.iter().find(|it| it.label.contains("image")).unwrap();
        if let designcraft_doc::Content::Graphic(g) = &img_item.content {
            let placed_rect = g.xf.transform_rect_bbox(designcraft_geom::Rect::new(0.0, 0.0, g.size.0, g.size.1));
            let placed_w = placed_rect.width();
            let placed_h = placed_rect.height();
            let frame_bounds = img_item.inner_bounds();
            assert!(
                (placed_w - frame_bounds.width()).abs() <= 1.0,
                "Placed width {} must match frame {} for fitContentToFrame",
                placed_w,
                frame_bounds.width()
            );
            assert!(
                (placed_h - frame_bounds.height()).abs() <= 1.0,
                "Placed height {} must match frame {} for fitContentToFrame",
                placed_h,
                frame_bounds.height()
            );
        }

        // Verify divider page adjusts to Page Size (margins == 0.0)
        assert_eq!(last_p.margins.top, 0.0, "Divider page margins must be 0 (Page Size)");
        assert_eq!(last_p.margins.bottom, 0.0, "Divider page margins must be 0 (Page Size)");
        assert_eq!(last_p.margins.inside, 0.0, "Divider page margins must be 0 (Page Size)");
        assert_eq!(last_p.margins.outside, 0.0, "Divider page margins must be 0 (Page Size)");

        let mut renderer = designcraft_render::Renderer::new();
        let cache = designcraft_compose::Cache::new();
        let opts = designcraft_render::RenderOptions::default();
        let rendered = renderer.render_page(&active.doc, &cache, 1, 1.0, false, &opts);
        assert!(rendered.is_some(), "Divider page should render cleanly without panic");
    }

    #[test]
    fn test_content_pages_adjust_to_page_margins_while_cover_and_div_adjust_to_page_size() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![
                vec!["COVER".into(), "".into(), "PUBLICATION COVER".into(), "".into(), "".into()],
                vec!["Note".into(), "".into(), "CATATAN TEKNIS".into(), "".into(), "".into()],
                vec!["DIV".into(), "01".into(), "BAB 01".into(), "".into(), "".into()],
                vec!["DIV01".into(), "01".into(), "BAB 01".into(), "".into(), "".into()],
                vec!["DIV1".into(), "02".into(), "BAB 02".into(), "".into(), "".into()],
                vec!["Tabel".into(), "1.1".into(), "Luas Wilayah".into(), "".into(), "".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        // Set document margins (e.g. 54 pt ~ 19mm)
        let _ = execute_set_page_margins(&mut app);

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // Spreads / pages: Page 0 is initial blank, then 6 added pages -> total 7 pages
        assert_eq!(active.doc.page_count(), 7);

        const PT_PER_MM: f64 = 72.0 / 25.4;
        let expected_top = 20.20 * PT_PER_MM;

        // Page 1: COVER -> margins should be 0.0 (adjust to Page Size)
        let (sp1_idx, p1_idx) = active.doc.page_loc(1).unwrap();
        let p1 = &active.doc.spreads[sp1_idx].pages[p1_idx];
        assert_eq!(p1.margins.top, 0.0, "COVER page margins must be 0 (Page Size)");

        // Page 2: Note -> margins should be custom doc_margins (adjust to Page Margin)
        let (sp2_idx, p2_idx) = active.doc.page_loc(2).unwrap();
        let p2 = &active.doc.spreads[sp2_idx].pages[p2_idx];
        assert!(
            (p2.margins.top - expected_top).abs() < 0.01,
            "Note content page must retain custom doc margins (expected {expected_top}, got {})",
            p2.margins.top
        );
        let p2_m = p2.margin_rect();
        // Check that items on page 2 are inside margins
        let sp2 = &active.doc.spreads[sp2_idx];
        for it in &sp2.items {
            let ib = it.bounds();
            if ib.x0 >= p2.bounds().x0 && ib.x1 <= p2.bounds().x1 {
                assert!(ib.x0 >= p2_m.x0 - 1.0, "Item left {} must align with margin {}", ib.x0, p2_m.x0);
                assert!(ib.y0 >= p2_m.y0 - 1.0, "Item top {} must align with margin {}", ib.y0, p2_m.y0);
                assert!(ib.x1 <= p2_m.x1 + 1.0, "Item right {} must not exceed margin {}", ib.x1, p2_m.x1);
            }
        }

        // Page 3: DIV -> margins should be 0.0 (adjust to Page Size)
        let (sp3_idx, p3_idx) = active.doc.page_loc(3).unwrap();
        let p3 = &active.doc.spreads[sp3_idx].pages[p3_idx];
        assert_eq!(p3.margins.top, 0.0, "DIV page margins must be 0 (Page Size)");

        // Page 4: DIV01 -> margins should be 0.0 (adjust to Page Size)
        let (sp4_idx, p4_idx) = active.doc.page_loc(4).unwrap();
        let p4 = &active.doc.spreads[sp4_idx].pages[p4_idx];
        assert_eq!(p4.margins.top, 0.0, "DIV01 page margins must be 0 (Page Size)");

        // Page 5: DIV1 -> margins should be 0.0 (adjust to Page Size)
        let (sp5_idx, p5_idx) = active.doc.page_loc(5).unwrap();
        let p5 = &active.doc.spreads[sp5_idx].pages[p5_idx];
        assert_eq!(p5.margins.top, 0.0, "DIV1 page margins must be 0 (Page Size)");

        // Page 6: Tabel -> margins should be custom doc_margins (adjust to Page Margin)
        let (sp6_idx, p6_idx) = active.doc.page_loc(6).unwrap();
        let p6 = &active.doc.spreads[sp6_idx].pages[p6_idx];
        assert!(
            (p6.margins.top - expected_top).abs() < 0.01,
            "Tabel content page must retain custom doc margins (expected {expected_top}, got {})",
            p6.margins.top
        );
        let p6_m = p6.margin_rect();
        let sp6 = &active.doc.spreads[sp6_idx];
        for it in &sp6.items {
            let ib = it.bounds();
            if ib.x0 >= p6.bounds().x0 && ib.x1 <= p6.bounds().x1 {
                assert!(ib.x0 >= p6_m.x0 - 1.0, "Header/table item left {} must align with margin {}", ib.x0, p6_m.x0);
                assert!(ib.y0 >= p6_m.y0 - 1.0, "Header/table item top {} must align with margin {}", ib.y0, p6_m.y0);
                assert!(ib.x1 <= p6_m.x1 + 1.0, "Header/table item right {} must not exceed margin {}", ib.x1, p6_m.x1);
            }
        }
    }

    #[test]
    fn test_custom_page_margins_from_margins_and_columns_applied_to_content() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // User sets custom margins via layout.marginsAndColumns (e.g. top: 48, bottom: 50, inside: 60, outside: 40)
        app.run(
            "layout.marginsAndColumns",
            json!({
                "margins": {
                    "top": 48.0,
                    "bottom": 50.0,
                    "inside": 60.0,
                    "outside": 40.0
                }
            }),
        )
        .unwrap();

        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec!["Tabel".into(), "01.01".into(), "Data Penduduk".into(), "".into(), "".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(1).unwrap();
        let page = &active.doc.spreads[si].pages[pi];
        assert_eq!(page.margins.top, 48.0);
        assert_eq!(page.margins.bottom, 50.0);
        assert_eq!(page.margins.inside, 60.0);
        assert_eq!(page.margins.outside, 40.0);

        let m = page.margin_rect();
        let sp = &active.doc.spreads[si];
        for it in &sp.items {
            let ib = it.bounds();
            if ib.x0 >= page.bounds().x0 && ib.x1 <= page.bounds().x1 {
                assert!(ib.x0 >= m.x0 - 1.0, "Item left {} must align with margin {}", ib.x0, m.x0);
                assert!(ib.y0 >= m.y0 - 1.0, "Item top {} must align with margin {}", ib.y0, m.y0);
                assert!(ib.x1 <= m.x1 + 1.0, "Item right {} must not exceed margin {}", ib.x1, m.x1);
            }
        }
    }

    #[test]
    fn test_pubgen_state_bilingual_defaults_and_serde() {
        let st = PubgenState::default();
        assert!(!st.bilingual_output);
        assert_eq!(st.locale, Locale::Id);
        assert_eq!(st.doc_language, "");

        let json_str = serde_json::to_string(&st).unwrap();
        let deser: PubgenState = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deser.bilingual_output, st.bilingual_output);
        assert_eq!(deser.locale, st.locale);
        assert_eq!(deser.doc_language, st.doc_language);

        // Deserializing older JSON lacking the bilingual fields falls back to defaults safely
        let old_json = r#"{"card1_path":"","card1_records":[],"card1_headers":[],"card1_type_col":0,"card1_num_col":1,"card1_title_col":2,"card1_preset":0,"card1_range_all":true,"card1_range_from":1,"card1_range_to":10,"card1_append":true,"card1_status":null,"card2_path":"","card2_matrix":[],"card2_first_row_header":true,"card2_col_numbers":true,"card2_table_footer":true,"card2_style":3,"card2_placement":2,"card2_status":null}"#;
        let old_deser: PubgenState = serde_json::from_str(old_json).unwrap();
        assert!(!old_deser.bilingual_output);
        assert_eq!(old_deser.locale, Locale::Id);
        assert_eq!(old_deser.doc_language, "");
    }

    #[test]
    fn test_bilingual_toggle_flip() {
        let mut st = PubgenState::default();
        assert!(!st.bilingual_output);
        st.bilingual_output = !st.bilingual_output;
        assert!(st.bilingual_output);
        let mode_label = if st.bilingual_output { "Dual Lang" } else { "Single Lang" };
        assert_eq!(mode_label, "Dual Lang");
        st.bilingual_output = !st.bilingual_output;
        assert!(!st.bilingual_output);
    }

    #[test]
    fn test_toc_style_default_titles_single_and_dual_lang() {
        // 1. TOC Table (kind 1)
        assert_eq!(toc_kind_title(1, false), Some("Daftar Tabel"));
        assert_eq!(toc_kind_title(1, true), Some("Daftar Tabel / List of Tables"));

        // 2. TOC Chart (kind 2)
        assert_eq!(toc_kind_title(2, false), Some("Daftar Grafik"));
        assert_eq!(toc_kind_title(2, true), Some("Daftar Grafik / List of Charts"));

        // 3. TOC Paragraph (kind 3)
        assert_eq!(toc_kind_title(3, false), Some("Daftar Isi"));
        assert_eq!(toc_kind_title(3, true), Some("Daftar Isi / Table of Contents"));

        // Verify switching modes updates default title
        let mut st = PubgenState {
            toc_kind: 1, // TOC Table
            toc_title: "Daftar Tabel".into(),
            bilingual_output: false,
            ..Default::default()
        };

        // Simulate toggling to Dual Lang mode
        let was_bilingual = st.bilingual_output;
        st.bilingual_output = !st.bilingual_output;
        if let Some(old_title) = toc_kind_title(st.toc_kind, was_bilingual) {
            if st.toc_title.trim().is_empty() || st.toc_title.trim() == old_title {
                if let Some(new_title) = toc_kind_title(st.toc_kind, st.bilingual_output) {
                    st.toc_title = new_title.to_string();
                }
            }
        }
        assert_eq!(st.toc_title, "Daftar Tabel / List of Tables");

        // Simulate toggling back to Single Lang mode
        let was_bilingual = st.bilingual_output;
        st.bilingual_output = !st.bilingual_output;
        if let Some(old_title) = toc_kind_title(st.toc_kind, was_bilingual) {
            if st.toc_title.trim().is_empty() || st.toc_title.trim() == old_title {
                if let Some(new_title) = toc_kind_title(st.toc_kind, st.bilingual_output) {
                    st.toc_title = new_title.to_string();
                }
            }
        }
        assert_eq!(st.toc_title, "Daftar Tabel");

        // Custom titles are preserved and not overwritten on mode toggle
        st.toc_title = "Custom Table List".into();
        let was_bilingual = st.bilingual_output;
        st.bilingual_output = !st.bilingual_output;
        if let Some(old_title) = toc_kind_title(st.toc_kind, was_bilingual) {
            if st.toc_title.trim().is_empty() || st.toc_title.trim() == old_title {
                if let Some(new_title) = toc_kind_title(st.toc_kind, st.bilingual_output) {
                    st.toc_title = new_title.to_string();
                }
            }
        }
        assert_eq!(st.toc_title, "Custom Table List");
    }

    #[test]
    fn test_toc_header_line_uses_locale_words() {
        let mut st =
            PubgenState { card1_records: vec![vec!["Tabel".into(), "01.01".into(), "Penduduk".into()]], locale: Locale::Id, ..Default::default() };
        let plan_table = TocPlan::from_state(&st);
        assert_eq!(toc_header_line(&plan_table, &st), "Tabel\thalaman");

        st.locale = Locale::En;
        assert_eq!(toc_header_line(&plan_table, &st), "Table\tpage");

        // Dual Lang mode produces two-line header: Tabel / Table, Halaman / Page
        st.bilingual_output = true;
        assert_eq!(toc_header_line(&plan_table, &st), format!("Tabel\tHalaman{}\u{2028}Table\tPage", ""));

        // Chart list
        st.toc_kind = 2;
        let plan_chart = TocPlan::from_state(&st);
        st.bilingual_output = false;
        st.locale = Locale::Id;
        assert_eq!(toc_header_line(&plan_chart, &st), "Grafik\thalaman");

        st.locale = Locale::En;
        assert_eq!(toc_header_line(&plan_chart, &st), "Chart\tpage");

        // Dual Lang mode produces two-line header: Grafik / Chart, Halaman / Page
        st.bilingual_output = true;
        assert_eq!(toc_header_line(&plan_chart, &st), format!("Grafik\tHalaman{}\u{2028}Chart\tPage", ""));
    }

    #[test]
    fn test_toc_table_and_chart_dual_lang_two_line_header() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Luas Daerah".into()],
                vec!["Grafik".into(), "01.01.".into(), "Pertumbuhan Penduduk".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_preset: 5,
            toc_style: "TOC Entry".into(),
            toc_layout_style: 1, // TablePage layout
            bilingual_output: true,
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        // 1. Generate TOC Table in Dual Lang mode
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        st.toc_kind = 1;
        st.toc_title = toc_kind_title(1, true).unwrap().to_string();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc for TOC Table: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story = active.doc.story(toc.story).unwrap();
        assert!(story.text.contains("Tabel\tHalaman\u{2028}Table\tPage"), "TOC Table story text: {}", story.text);

        let table_pos = story.text.find("Table\tPage").expect("Table position");
        let fmt_table = story.char_format_of(table_pos);
        assert_eq!(fmt_table.style, "TOC English");
        assert_eq!(fmt_table.over.font_style.as_deref(), Some("Italic"));

        let page_pos = story.text.find("Page").expect("Page position");
        let fmt_page = story.char_format_of(page_pos);
        assert_eq!(fmt_page.style, "TOC English");
        assert_eq!(fmt_page.over.font_style.as_deref(), Some("Italic"));

        let id_table_pos = story.text.find("Tabel\t").expect("Tabel position");
        let fmt_id_table = story.char_format_of(id_table_pos);
        assert_eq!(fmt_id_table.style, designcraft_doc::NO_CHAR_STYLE);

        // 2. Generate TOC Chart in Dual Lang mode
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let pages = app.session.active().unwrap().doc.page_count();
        crate::canvas::go_to_page(&mut app, pages - 1);
        st.toc_kind = 2;
        st.toc_title = toc_kind_title(2, true).unwrap().to_string();
        let res_chart = execute_add_toc(&mut app, &mut st);
        assert!(res_chart.is_ok(), "execute_add_toc for TOC Chart: {res_chart:?}");

        let active = app.session.active().unwrap();
        let toc_chart = active.doc.tocs.last().unwrap();
        let story_chart = active.doc.story(toc_chart.story).unwrap();
        assert!(story_chart.text.contains("Grafik\tHalaman\u{2028}Chart\tPage"), "TOC Chart story text: {}", story_chart.text);

        let chart_pos = story_chart.text.find("Chart\tPage").expect("Chart position");
        let fmt_chart = story_chart.char_format_of(chart_pos);
        assert_eq!(fmt_chart.style, "TOC English");
        assert_eq!(fmt_chart.over.font_style.as_deref(), Some("Italic"));
    }

    #[test]
    fn test_split_chapter_title_locale() {
        let (ch1, t1) = split_chapter_title_locale("BAB 01 PEMERINTAHAN", "01", false);
        assert_eq!(ch1, "BAB 01");
        assert_eq!(t1, "PEMERINTAHAN");

        let (ch2, t2) = split_chapter_title_locale("CHAPTER 01 GEOGRAPHY AND CLIMATE", "01", true);
        assert_eq!(ch2, "CHAPTER 01");
        assert_eq!(t2, "GEOGRAPHY AND CLIMATE");

        let (ch3, t3) = split_chapter_title_locale("GEOGRAPHY AND CLIMATE", "01", true);
        assert_eq!(ch3, "CHAPTER 01");
        assert_eq!(t3, "GEOGRAPHY AND CLIMATE");

        let (ch4, t4) = split_chapter_title_locale("Kata Pengantar", "", false);
        assert_eq!(ch4, "Kata Pengantar");
        assert_eq!(t4, "");
    }

    #[test]
    fn test_divider_bilingual_slots() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec![
                "Divider".into(),
                "01".into(),
                "BAB 01 GEOGRAFI".into(),
                "".into(),
                "002".into(),
                "GEOGRAPHY AND CLIMATE".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "title_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for bilingual divider should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("GEOGRAFI")), "Stories should contain Indonesian title GEOGRAFI: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("GEOGRAPHY AND CLIMATE")), "Stories should contain English title: {texts:?}");

        // Dual Lang Mode: the title_en textbox is 22 pt italic, black stroke behind the fill.
        let en_story = active.doc.stories.values().find(|s| s.text.contains("GEOGRAPHY AND CLIMATE")).expect("EN title story exists");
        assert!(!en_story.chars.is_empty(), "EN title story has runs");
        for run in &en_story.chars {
            assert_eq!(run.format.over.size, Some(22.0), "title_en run size: {run:?}");
            assert_eq!(run.format.over.font_style.as_deref(), Some("Italic"), "title_en run style: {run:?}");
            assert_eq!(run.format.over.stroke.as_deref(), Some("[Black]"), "title_en run stroke: {run:?}");
            assert_eq!(run.format.over.stroke_behind_fill, Some(true), "title_en stroke behind fill: {run:?}");
        }
    }

    #[test]
    fn test_ulasan_dual_spread() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let dir = std::env::temp_dir().join(format!("designcraft_pubgen_dual_ulasan_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let id_rtf = dir.join("R01_id.rtf");
        let en_rtf = dir.join("R01_en.rtf");
        std::fs::write(&id_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Ulasan wilayah Indonesia.}"#).unwrap();
        std::fs::write(&en_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 CHAPTER 01 GEOGRAPHY\par Narrative review in English.}"#)
            .unwrap();

        let st = PubgenState {
            card1_preset: 6,
            bilingual_output: true,
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "BAB 01 Geografi".into(),
                id_rtf.to_string_lossy().to_string(),
                "".into(),
                en_rtf.to_string_lossy().to_string(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "file_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for dual Ulasan should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // Initial page (1) + Spread 1 (1) + Spread 2 (1) = 3 pages
        assert_eq!(active.doc.page_count(), 3, "Should generate two spreads for dual Ulasan");

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("Ulasan wilayah Indonesia")), "Should contain ID RTF body: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Narrative review in English")), "Should contain EN RTF body: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("GEOGRAPHY")), "Should contain derived EN chapter title: {texts:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cover_bilingual() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Single Lang mode (bilingual_output = false)
        let mut st = PubgenState {
            cover_style: 0,
            cover_title: "KARAWANG TIMUR".into(),
            cover_title2: "DALAM ANGKA".into(),
            cover_title_en: "IN FIGURES".into(),
            bilingual_output: false,
            ..Default::default()
        };

        let res = execute_generate_cover(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_cover single lang should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("KARAWANG TIMUR")), "Should contain ID title: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("DALAM ANGKA")), "Should contain ID title2: {texts:?}");
        assert!(!texts.iter().any(|t| t.contains("IN FIGURES")), "Should NOT contain title_en when bilingual_output is false: {texts:?}");

        // 2. Dual Lang mode (bilingual_output = true)
        let mut app_dual = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app_dual.run("file.new", json!({})).unwrap();
        st.bilingual_output = true;

        let res_dual = execute_generate_cover(&mut app_dual, &st);
        assert!(res_dual.is_ok(), "execute_generate_cover dual lang should succeed: {res_dual:?}");

        let active_dual = app_dual.session.active().unwrap();
        let texts_dual: Vec<String> = active_dual.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts_dual.iter().any(|t| t.contains("KARAWANG TIMUR")), "Should contain ID title: {texts_dual:?}");
        assert!(texts_dual.iter().any(|t| t.contains("DALAM ANGKA")), "Should contain ID title2: {texts_dual:?}");
        assert!(texts_dual.iter().any(|t| t.contains("IN FIGURES")), "Should contain title_en in bilingual mode: {texts_dual:?}");
    }

    #[test]
    fn test_rxx_bilingual_dual_column_layout() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let dir = std::env::temp_dir().join(format!("pubgen_rxx_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let id_rtf = dir.join("R01.rtf");
        let en_rtf = dir.join("R01_en.rtf");
        std::fs::write(&id_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Kecamatan Karawang Timur merupakan bagian tak terpisahkan.}"#)
            .unwrap();
        std::fs::write(&en_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 East Karawang District is an integral part of Karawang.}"#)
            .unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "Bab 01 Geografi".into(),
                id_rtf.to_string_lossy().to_string(),
                "".into(),
                "Chapter 01 Geography".into(),
                en_rtf.to_string_lossy().to_string(),
            ]],
            card1_headers: vec![
                "type".into(),
                "number".into(),
                "title".into(),
                "file".into(),
                "template".into(),
                "title_en".into(),
                "file_en".into(),
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for RXX bilingual should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // Page 1 created (plus initial page 0 = 2 pages)
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("BAB 01")), "Should contain Line 1 header 'BAB 01': {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Geografi")), "Should contain Line 2 header 'Geografi': {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Chapter 01 Geography")), "Should contain Line 3 header 'Chapter 01 Geography': {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Kecamatan Karawang Timur")), "Should contain Indonesian body text: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("East Karawang District")), "Should contain English body text: {texts:?}");

        // Verify side-by-side layout: left frame and right frame
        let (si, pi) = active.doc.page_loc(1).unwrap();
        let sp = &active.doc.spreads[si];
        let pg = &sp.pages[pi];
        let margin = pg.margin_rect();

        let mut left_found = false;
        let mut right_found = false;
        let mut en_story_id = None;
        let mut id_story_id = None;

        for it in &sp.items {
            if let Some(tf) = it.text_frame()
                && let Some(story) = active.doc.stories.get(&tf.story)
            {
                let b = it.bounds();
                if story.text.contains("Kecamatan Karawang Timur") {
                    left_found = true;
                    id_story_id = Some(tf.story);
                    // Left column starts at margin.x0
                    assert!((b.x0 - margin.x0).abs() < 2.0, "Left frame x0 ({}) should align with margin.x0 ({})", b.x0, margin.x0);
                    assert!(b.x1 < margin.x1 - 50.0, "Left frame x1 ({}) should be ~half width", b.x1);
                } else if story.text.contains("East Karawang District") {
                    right_found = true;
                    en_story_id = Some(tf.story);
                    // Right column ends at margin.x1
                    assert!((b.x1 - margin.x1).abs() < 2.0, "Right frame x1 ({}) should align with margin.x1 ({})", b.x1, margin.x1);
                    assert!(b.x0 > margin.x0 + 50.0, "Right frame x0 ({}) should start past center", b.x0);
                }
            }
        }

        assert!(left_found, "Should find left column Indonesian text frame");
        assert!(right_found, "Should find right column English text frame");

        // Verify font styling: English text is Italic, Indonesian text is upright
        let en_story = active.doc.stories.get(&en_story_id.unwrap()).unwrap();
        assert!(!en_story.chars.is_empty());
        for run in &en_story.chars {
            let style = run.format.over.font_style.as_deref().unwrap_or("Regular");
            assert!(
                style.eq_ignore_ascii_case("italic") || style.eq_ignore_ascii_case("bold italic"),
                "All English story runs should be italic, got: {style}"
            );
        }

        let id_story = active.doc.stories.get(&id_story_id.unwrap()).unwrap();
        assert!(!id_story.chars.is_empty());
        for run in &id_story.chars {
            let style = run.format.over.font_style.as_deref().unwrap_or("Regular");
            assert!(!style.eq_ignore_ascii_case("italic"), "Indonesian story runs should NOT be converted to italic, got: {style}");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rxx_bilingual_multi_page_continuation() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let dir = std::env::temp_dir().join(format!("pubgen_rxx_multi_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let id_rtf = dir.join("R01.rtf");
        let en_rtf = dir.join("R01_en.rtf");

        // Generate ~40 paragraphs of text to guarantee overset on a half-page column
        let mut id_body = String::from(r#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 "#);
        for i in 0..40 {
            id_body.push_str(&format!(r#"Paragraf narasi bahasa Indonesia nomor {i} mendeskripsikan secara mendalam wilayah administratif dan tata ruang Karawang Timur.\par "#));
        }
        id_body.push('}');
        std::fs::write(&id_rtf, id_body.as_bytes()).unwrap();

        let mut en_body = String::from(r#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 "#);
        for i in 0..40 {
            en_body.push_str(&format!(
                r#"English narrative paragraph number {i} describing in detail the spatial planning and governance of East Karawang.\par "#
            ));
        }
        en_body.push('}');
        std::fs::write(&en_rtf, en_body.as_bytes()).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "Bab 01 Geografi".into(),
                id_rtf.to_string_lossy().to_string(),
                "".into(),
                "Chapter 01 Geography".into(),
                en_rtf.to_string_lossy().to_string(),
            ]],
            card1_headers: vec![
                "type".into(),
                "number".into(),
                "title".into(),
                "file".into(),
                "template".into(),
                "title_en".into(),
                "file_en".into(),
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for multi-page RXX should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // Since 40 paragraphs exceed 1 page, total pages must be >= 3 (page 0 blank + page 1 + page 2...)
        assert!(active.doc.page_count() >= 3, "Should create continuation page(s) for overset, got: {}", active.doc.page_count());

        // Check continuation page (Page 2)
        let (si2, pi2) = active.doc.page_loc(2).unwrap();
        let sp2 = &active.doc.spreads[si2];
        let pg2 = &sp2.pages[pi2];
        let m2 = pg2.margin_rect();

        let mut left_cont_found = false;
        let mut right_cont_found = false;

        for it in &sp2.items {
            if active.doc.page_of_item(it.id) != Some(2) {
                continue;
            }
            if let Some(tf) = it.text_frame()
                && let Some(story) = active.doc.stories.get(&tf.story)
            {
                let b = it.bounds();
                if story.text.contains("Paragraf narasi bahasa Indonesia") {
                    left_cont_found = true;
                    // On continuation page, frame starts directly at top margin (no chapter header)
                    assert!((b.y0 - m2.y0).abs() < 2.0, "Continuation left frame y0 ({}) should be at m2.y0 ({})", b.y0, m2.y0);
                    assert!((b.x0 - m2.x0).abs() < 2.0, "Continuation left frame x0 ({}) should align with m2.x0 ({})", b.x0, m2.x0);
                } else if story.text.contains("English narrative paragraph") {
                    right_cont_found = true;
                    assert!((b.y0 - m2.y0).abs() < 2.0, "Continuation right frame y0 ({}) should be at m2.y0 ({})", b.y0, m2.y0);
                    assert!((b.x1 - m2.x1).abs() < 2.0, "Continuation right frame x1 ({}) should align with m2.x1 ({})", b.x1, m2.x1);

                    // Verify English story runs on continuation page remain italic
                    for run in &story.chars {
                        let style = run.format.over.font_style.as_deref().unwrap_or("Regular");
                        assert!(
                            style.eq_ignore_ascii_case("italic") || style.eq_ignore_ascii_case("bold italic"),
                            "English runs on continuation page should be italic, got: {style}"
                        );
                    }
                }
            }
        }

        assert!(left_cont_found, "Should find left continuation frame on Page 2");
        assert!(right_cont_found, "Should find right continuation frame on Page 2");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_execute_apply_and_remove_header_footer() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({ "facingPages": true, "pages": 4 })).unwrap();

        let st = PubgenState {
            hf_style: "Corporate Report".into(),
            hf_sided: 0,
            hf_header_left: "{title}".into(),
            hf_header_right: "{section}".into(),
            hf_footer_left: "{page}".into(),
            hf_footer_right: "Confidential".into(),
            hf_top_accent_bar: true,
            hf_top_accent_bar_height: 5.0,
            hf_top_accent_bar_bleed: true,
            hf_footer_rule: true,
            hf_color_primary: "#0072bc".into(),
            ..Default::default()
        };

        let res = execute_apply_header_footer(&mut app, &st);
        assert!(res.is_ok(), "apply should succeed: {res:?}");

        let doc = &app.session.active().unwrap().doc;
        let hf = doc.header_footer.as_ref().expect("header_footer present");
        assert_eq!(hf.preset, "Corporate Report");
        assert!(hf.objects.top_accent_bar);
        assert!(hf.accent_bar_items.left.is_some());
        assert!(hf.accent_bar_items.right.is_some());

        // Now set custom page margins and verify header_footer parent spread is synchronized
        let set_m_res = execute_set_page_margins(&mut app);
        assert!(set_m_res.is_ok());

        let doc_with_m = &app.session.active().unwrap().doc;
        let hf_with_m = doc_with_m.header_footer.as_ref().unwrap();
        let parent_sp = doc_with_m.parents.iter().find(|p| Some(p.id) == hf_with_m.parent).unwrap();
        assert!((parent_sp.pages[0].margins.inside - 19.0 * 72.0 / 25.4).abs() < 1e-3);
        assert!((parent_sp.pages[0].margins.outside - 15.0 * 72.0 / 25.4).abs() < 1e-3);

        let rem_res = execute_remove_header_footer(&mut app);
        assert!(rem_res.is_ok());
        let doc_after = &app.session.active().unwrap().doc;
        assert!(doc_after.header_footer.is_none());
    }

    #[test]
    fn test_execute_apply_header_footer_scopes_to_range_and_section() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({ "pages": 11 })).unwrap();
        // A section starting at page 5: section 0 covers pages 1-4, section 1 pages 5-11.
        app.run("layout.section", json!({ "page": 5, "startNumber": 1, "style": "arabic" })).unwrap();

        // Range scope: the template's parent lands on pages 5-8 only.
        let st = PubgenState { hf_scope: 2, hf_scope_start: 5, hf_scope_end: 8, ..Default::default() };
        execute_apply_header_footer(&mut app, &st).unwrap_or_else(|e| panic!("range apply: {e}"));
        {
            let doc = &app.session.active().unwrap().doc;
            let hf = doc.header_footer.as_ref().expect("header_footer present");
            assert_eq!(hf.scope, designcraft_doc::HfScope::Range { first: 5, last: 8 });
            let parent = hf.parent;
            let on_page = |i: usize| doc.page(i).is_some_and(|p| p.parent == parent);
            assert!((4..=7).all(on_page), "pages 5-8 carry the header/footer parent");
            assert!((0..4).all(|i| !on_page(i)) && (8..11).all(|i| !on_page(i)), "pages outside the range do not");
        }

        // Section scope: re-apply scoped to section index 1 (pages 5-11).
        let st = PubgenState { hf_scope: 1, hf_scope_section: 1, ..Default::default() };
        execute_apply_header_footer(&mut app, &st).unwrap_or_else(|e| panic!("section apply: {e}"));
        {
            let doc = &app.session.active().unwrap().doc;
            let hf = doc.header_footer.as_ref().expect("header_footer present");
            assert_eq!(hf.scope, designcraft_doc::HfScope::Section(1));
            let parent = hf.parent;
            let on_page = |i: usize| doc.page(i).is_some_and(|p| p.parent == parent);
            assert!((4..11).all(on_page), "section pages carry the parent");
            assert!((0..4).all(|i| !on_page(i)), "pages before the section do not");
        }
    }

    #[test]
    fn test_execute_save_and_delete_hf_template() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            hf_custom_name: "Annual Audit 2026".into(),
            hf_sided: 1,
            hf_header_single: "Audit Report | {page}".into(),
            hf_footer_single: "Internal Use Only".into(),
            hf_header_rule: true,
            hf_color_primary: "#990000".into(),
            ..Default::default()
        };

        let save_res = execute_save_hf_template(&mut app, &mut st);
        assert!(save_res.is_ok(), "{:?}", save_res.err());
        assert_eq!(st.hf_style, "Annual Audit 2026");
        assert!(st.hf_custom_name.is_empty());

        let templates = designcraft_engine::cmd::header_footer::all_templates(&app.session);
        assert!(templates.iter().any(|t| t.get("name").and_then(Value::as_str) == Some("Annual Audit 2026")));

        let del_res = execute_delete_hf_template(&mut app, &mut st, "Annual Audit 2026");
        assert!(del_res.is_ok());
        assert_eq!(st.hf_style, "Modern Minimal");
    }

    #[test]
    fn test_load_hf_template_to_state() {
        let mut st = PubgenState::default();
        let tpl = json!({
            "name": "Custom Editorial",
            "sided": "single",
            "headerOffset": 30.0,
            "footerOffset": 32.0,
            "header": { "single": "Chapter | {page}" },
            "footer": { "single": "Footer Notes" },
            "objects": {
                "headerRule": true,
                "headerRuleWeight": 1.25,
                "footerRule": false,
                "topAccentBar": true,
                "topAccentBarHeight": 8.0,
                "topAccentBarBleed": false
            },
            "colors": {
                "primary": "#334455",
                "accent": "#ffaa00",
                "dark": "#112233",
                "ruleColor": "#888888",
                "textColor": "#222222"
            }
        });

        load_hf_template_to_state(&mut st, &tpl);
        assert_eq!(st.hf_style, "Custom Editorial");
        assert_eq!(st.hf_sided, 1);
        assert_eq!(st.hf_header_offset, 30.0);
        assert_eq!(st.hf_footer_offset, 32.0);
        assert_eq!(st.hf_header_single, "Chapter | {page}");
        assert_eq!(st.hf_footer_single, "Footer Notes");
        assert!(st.hf_header_rule);
        assert_eq!(st.hf_header_rule_weight, 1.25);
        assert!(!st.hf_footer_rule);
        assert!(st.hf_top_accent_bar);
        assert_eq!(st.hf_top_accent_bar_height, 8.0);
        assert!(!st.hf_top_accent_bar_bleed);
        assert_eq!(st.hf_color_primary, "#334455");
        assert_eq!(st.hf_color_accent, "#ffaa00");
    }
}
