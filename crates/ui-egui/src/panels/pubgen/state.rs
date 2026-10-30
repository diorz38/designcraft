//! The Pubgen panel's persisted state: [`PubgenState`] and its serde defaults.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ai;
use super::ai_cover;
use super::ai_cover_job;
use super::audit;
use super::locale::Locale;

pub(super) fn default_profile_id() -> String {
    "bps_dda".into()
}

pub(super) fn default_file_col() -> usize {
    3
}

pub(super) fn default_template_col() -> usize {
    4
}

pub(super) fn default_title_en_col() -> usize {
    5
}

pub(super) fn default_file_en_col() -> usize {
    6
}

pub(super) fn default_true() -> bool {
    true
}

pub(super) fn default_false() -> bool {
    false
}

pub(super) fn default_one_u32() -> u32 {
    1
}

pub(super) fn default_one_usize() -> usize {
    1
}

pub(super) fn default_toc_title() -> String {
    "Contents".into()
}

pub(super) fn default_toc_style() -> String {
    "TOC Entry".into()
}

pub(crate) fn default_cover_title() -> String {
    "KARAWANG TIMUR".into()
}

/// The second headline line of the cover: the templates give it its own frame and style.
pub(crate) fn default_cover_title2() -> String {
    "Title 2".into()
}

pub(super) fn default_cover_year() -> String {
    "2026".into()
}

pub(super) fn default_cover_volume() -> String {
    "Volume 23 2026".into()
}

pub(super) fn default_cover_catalogue() -> String {
    "1102001.3215113".into()
}

pub(super) fn default_cover_code_num() -> String {
    "2828-7436".into()
}

pub(super) fn default_cover_primary() -> String {
    "#0072bc".into()
}

pub(super) fn default_cover_accent() -> String {
    "#f5ab19".into()
}

pub(super) fn default_cover_dark() -> String {
    "#122647".into()
}

pub(super) fn default_cover_skyline() -> String {
    "outputs/city_skyline.jpg".into()
}

pub(super) fn default_cover_landscape() -> String {
    "outputs/city_landscape.jpg".into()
}

pub(super) fn default_cover_logo() -> String {
    "outputs/bps_logo.png".into()
}

pub(super) fn default_cover_agency_text() -> String {
    "BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG".into()
}

pub(super) fn default_hf_style() -> String {
    "Modern Minimal".into()
}

pub(super) fn default_hf_offset() -> f64 {
    24.0
}

pub(super) fn default_hf_rule_weight() -> f64 {
    0.5
}

pub(super) fn default_hf_accent_bar_height() -> f64 {
    4.0
}

pub(super) fn default_hf_page_token() -> String {
    "{page}".into()
}

pub(super) fn default_hf_color_primary() -> String {
    "#111827".into()
}

pub(super) fn default_hf_color_accent() -> String {
    "#3b82f6".into()
}

pub(super) fn default_hf_color_dark() -> String {
    "#1f2937".into()
}

pub(super) fn default_hf_color_rule() -> String {
    "#e5e7eb".into()
}

pub(super) fn default_hf_color_text() -> String {
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
    /// Remaining Page-from-CSV columns, resolved by header name at CSV load time
    /// (legacy fixed positions keep old CSVs working when a header is absent).
    #[serde(default = "default_file_col")]
    pub card1_file_col: usize,
    #[serde(default = "default_template_col")]
    pub card1_template_col: usize,
    #[serde(default = "default_title_en_col")]
    pub card1_title_en_col: usize,
    #[serde(default = "default_file_en_col")]
    pub card1_file_en_col: usize,
    // AI translation (docs/plans/2026-10-22-offline-ai-translation-pubgen.md).
    /// Translate the Indonesian half with AI when no `title_en`/`file_en` exists.
    #[serde(default)]
    pub translate_with_ai: bool,
    /// The setup dialog was accepted this session.
    #[serde(skip)]
    pub ai_setup_done: bool,
    /// Some(..) = the setup modal is open.
    #[serde(skip)]
    pub ai_dialog: Option<ai::AiSetupDialog>,
    /// A running background translation over the collected units.
    #[serde(skip)]
    pub translate_job: Option<ai::TranslateJob>,
    /// Indonesian source text -> English (session cache; a re-run does no inference).
    #[serde(skip)]
    pub translations: HashMap<String, String>,
    /// Source RTF -> temp translated RTF (for the EN narrative spread).
    #[serde(skip)]
    pub translated_bodies: HashMap<String, PathBuf>,
    /// Official BPS terminology (bundled asset or the user's override).
    #[serde(skip)]
    pub glossary: ai::BpsGlossary,
    /// The glossary was resolved from prefs for this panel session.
    #[serde(skip)]
    pub glossary_loaded: bool,
    /// After a collection run, ask the AI for drafts of the blank terms automatically.
    #[serde(default = "default_true")]
    pub auto_fill_ai_drafts: bool,
    /// Signature of the slice the finished job translated; a mismatch re-collects.
    #[serde(skip)]
    pub translations_slice_key: String,
    /// Targeted translation: only RXX/BXX review pages (narrative), tables and other
    /// page kinds stay Indonesian. Useful under tight provider token limits.
    #[serde(default)]
    pub ai_only_reviews: bool,
    /// When the AI job finishes, auto-run the Excel convert+generate with this workbook.
    #[serde(skip)]
    pub ai_pending_excel_path: String,
    /// Page-targeted translation: the page awaiting the AI result.
    #[serde(skip)]
    pub ai_page_target: Option<ai::PageTarget>,
    /// Table page-targeted translation: the table page awaiting the AI result.
    #[serde(skip)]
    pub ai_table_target: Option<ai::TablePageTarget>,
    /// Range of table pages awaiting batch AI translation.
    #[serde(skip)]
    pub ai_table_targets_batch: Vec<ai::TablePageTarget>,
    /// Range picker for table page translation.
    #[serde(default = "default_one_usize")]
    pub ai_table_range_from: usize,
    #[serde(default = "default_one_usize")]
    pub ai_table_range_to: usize,
    #[serde(default)]
    pub show_table_range_popup: bool,
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
    #[serde(default)]
    pub card2_color_theme: usize,
    #[serde(default)]
    pub card2_theme_override: Option<String>,
    pub card2_placement: usize,
    pub card2_status: Option<(String, bool)>,
    #[serde(default)]
    pub card2_delim: Option<char>,

    // Card 3: Quick Actions
    #[serde(default)]
    pub card3_status: Option<(String, bool)>,
    /// Active/last imported master Excel source path (.xlsx)
    #[serde(default)]
    pub excel_source_path: String,
    /// Status message for Master Excel actions
    #[serde(default)]
    pub excel_status: Option<(String, bool)>,
    /// Status message for TOC actions
    #[serde(default)]
    pub toc_status: Option<(String, bool)>,
    /// Status message for Page Utilities actions
    #[serde(default)]
    pub utilities_status: Option<(String, bool)>,
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
    /// Insert generated TOC on a new blank page after the currently selected page
    #[serde(default)]
    pub toc_insert_after: bool,
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

    /// Active publication profile ID.
    #[serde(default = "default_profile_id")]
    pub active_profile_id: String,
    /// Active custom profile overrides.
    #[serde(default)]
    pub profile_custom: Option<super::profiles::PubgenProfile>,
    /// Whether the user locked the profile against auto-detection.
    #[serde(default)]
    pub profile_user_locked: bool,
    /// Auto-detected profile ID from workbook sheets.
    #[serde(default)]
    pub profile_detected_id: Option<String>,
    /// Customize Profile modal dialog open state.
    #[serde(skip)]
    pub dialog_profile_customize_open: bool,
    /// Status message for header actions (e.g. Set Page Margin)
    #[serde(default)]
    pub header_status: Option<(String, bool)>,

    // Publication compiler (docs/plans/2026-10-27-pubgen-pillar2-binder-mini-plan.md).
    /// The manifest chosen in the Publication card; empty uses `Prefs.pubgen_project_path`.
    #[serde(default)]
    pub compile_manifest_path: String,
    /// The path [`Self::compile_summary`] was computed for.
    #[serde(skip)]
    pub compile_checked: String,
    /// What the manifest makes, or the error that stopped it loading.
    #[serde(skip)]
    pub compile_summary: String,
    /// The frame-stepped compile the panel advances one step per frame (desktop only).
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub(crate) compile_job: Option<Box<super::compile::CompileJob>>,
    /// Dialog for creating a new `pubgen_project.json` manifest.
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub(crate) compile_new_dialog: Option<Box<super::compile::NewProjectDialog>>,
    /// Vector chart form options popup dialog.
    #[serde(skip)]
    pub chart_dialog: Option<Box<super::charts::dialog::ChartDialog>>,
    /// Progress and the last report, shown under the card's buttons.
    #[serde(default)]
    pub compile_status: Option<(String, bool)>,

    /// Target locale for single-language generation mode.
    #[serde(default)]
    pub locale: Locale,
    /// Bilingual / Dual-language generation mode: false = Single Lang, true = Dual Lang.
    #[serde(default)]
    pub bilingual_output: bool,
    /// Document-level language setting: "" = leave run language untouched (default), else language name e.g. "Indonesian".
    #[serde(default)]
    pub doc_language: String,
    /// Show the PubGen About modal dialog.
    #[serde(skip)]
    pub show_about_dialog: bool,
    /// The Audit Publication dialog is open.
    #[serde(skip)]
    pub audit_open: bool,
    /// The last `preflight.pubgen` result behind the Audit Publication dialog.
    #[serde(skip)]
    pub audit_issues: Option<Vec<audit::AuditIssue>>,
    /// The document revision the last audit ran on; a different revision re-runs it silently.
    #[serde(skip)]
    pub audit_revision: u64,
    /// The distribution profile: print adds the spine/bleed guards and a 300 ppi floor.
    #[serde(default)]
    pub audit_profile: audit::AuditProfile,
    /// Triage tags per finding (`kind|message` → `false-positive` / `ignored`), kept across runs.
    #[serde(default)]
    pub audit_tags: HashMap<String, String>,
    /// Rows carrying a tag are hidden from the checklist.
    #[serde(default = "default_true")]
    pub audit_hide_tagged: bool,
    /// Effective ppi for the generic raster-resolution rule inside the audit.
    #[serde(default)]
    pub audit_min_ppi: f64,
    /// Dialog open flags for Pubgen menu popup windows.
    #[serde(skip)]
    pub dialog_excel_open: bool,
    #[serde(skip)]
    pub dialog_toc_open: bool,
    #[serde(skip)]
    pub dialog_cover_open: bool,
    #[serde(skip)]
    pub dialog_hf_open: bool,
    #[serde(skip)]
    pub dialog_manual_open: bool,
    #[serde(skip)]
    pub dialog_manual_tab: usize,
    #[serde(skip)]
    pub dialog_utils_open: bool,

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
    #[serde(default = "default_true")]
    pub cover_include_inside_back: bool,
    /// Whether to overlay a stylized vector map on the cover page.
    #[serde(default)]
    pub cover_add_map: bool,
    /// Selected stock cover map index (into `list_stock_cover_maps()`).
    #[serde(default)]
    pub cover_map_stock_idx: usize,
    /// Path to custom GeoJSON file for cover map if not using stock.
    #[serde(default)]
    pub cover_map_custom_path: String,
    /// Style of cover map: 0 = Watermark/Silhouette, 1 = Accent Hero, 2 = Outlines Only.
    #[serde(default)]
    pub cover_map_style: usize,
    /// Replace the cover page(s) already in the document instead of adding a new
    /// one in front (Publication Cover ▸ checkbox next to the generate button).
    #[serde(default)]
    pub cover_replace: bool,
    #[serde(default)]
    pub cover_image_back: String,
    #[serde(default)]
    pub cover_office_address: String,
    #[serde(default)]
    pub cover_status: Option<(String, bool)>,
    /// AI Base covers (styles 100+): the palette direction fed to the generator.
    #[serde(default)]
    pub cover_ai_palette: usize,
    /// Open AI Base Cloudflare setup dialog, if any.
    #[serde(skip)]
    pub cover_ai_dialog: Option<ai_cover::AiCoverSetupDialog>,
    /// Last AI setup/generation error shown next to the setup button.
    #[serde(skip)]
    pub cover_ai_error: Option<String>,
    /// Running AI Base cover generation job.
    #[serde(skip)]
    pub cover_job: Option<ai_cover_job::CoverJob>,
    /// Covers still available today on the shared Cloudflare free tier (from the
    /// Supabase tracker, reported by the last finished job). Persists after the job
    /// is cleared so both quota labels keep rendering.
    #[serde(skip)]
    pub cover_quota_covers: Option<u64>,
    /// Pending batch cover items awaiting quota confirmation.
    #[serde(skip)]
    pub cover_batch_pending: Option<Vec<ai_cover_job::CoverBatchItem>>,
    /// Popup shown after an AI operation completes (Groq translation or Cloudflare
    /// cover generation): one line per usage signal, e.g. "Cloudflare AI: ~N covers
    /// left today". Set by the job-completion poll; cleared when the user closes it.
    #[serde(skip)]
    pub ai_quota_popup: Option<Vec<String>>,
    /// Batch of covers parsed from Excel COVER sheet (when multiple publications/covers exist)
    #[serde(skip)]
    pub excel_batch_covers: Vec<super::excel::ExcelCoverData>,
    /// Cover parsed from Excel COVER sheet
    #[serde(skip)]
    pub excel_cover: Option<super::excel::ExcelCoverData>,

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
            card1_file_col: 3,
            card1_template_col: 4,
            card1_title_en_col: 5,
            card1_file_en_col: 6,
            translate_with_ai: false,
            ai_setup_done: false,
            ai_dialog: None,
            translate_job: None,
            translations: HashMap::new(),
            translated_bodies: HashMap::new(),
            glossary: ai::BpsGlossary::load(""),
            glossary_loaded: false,
            auto_fill_ai_drafts: true,
            translations_slice_key: String::new(),
            ai_only_reviews: false,
            ai_pending_excel_path: String::new(),
            ai_page_target: None,
            ai_table_target: None,
            ai_table_targets_batch: Vec::new(),
            ai_table_range_from: 1,
            ai_table_range_to: 1,
            show_table_range_popup: false,
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
            card2_style: 3,       // Horizontal Rules
            card2_color_theme: 0, // #FFCB56 - #FFE9A9 (Default)
            card2_theme_override: None,
            card2_placement: 2, // At page
            card2_status: None,
            card2_delim: None,
            card3_status: None,
            excel_source_path: String::new(),
            excel_status: None,
            toc_status: None,
            utilities_status: None,
            rtf_path: String::new(),
            image_path: String::new(),
            image_as_background: false,
            toc_style: default_toc_style(),
            toc_title: default_toc_title(),
            toc_include_self: false,
            toc_pair_bilingual: false,
            toc_insert_after: false,
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
            cover_include_inside_back: false,
            cover_add_map: false,
            cover_map_stock_idx: 0,
            cover_map_custom_path: String::new(),
            cover_map_style: 0,
            cover_replace: false,
            cover_image_back: String::new(),
            cover_office_address: String::new(),
            cover_status: None,
            cover_ai_palette: 0,
            cover_ai_dialog: None,
            cover_ai_error: None,
            cover_job: None,
            cover_quota_covers: None,
            cover_batch_pending: None,
            ai_quota_popup: None,
            excel_batch_covers: Vec::new(),
            excel_auto_generate: true,
            excel_had_cover: false,
            header_status: None,
            compile_manifest_path: String::new(),
            compile_checked: String::new(),
            compile_summary: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            compile_job: None,
            #[cfg(not(target_arch = "wasm32"))]
            compile_new_dialog: None,
            chart_dialog: None,
            compile_status: None,
            locale: Locale::Id,
            bilingual_output: false,
            doc_language: String::new(),
            show_about_dialog: false,
            audit_open: false,
            audit_issues: None,
            audit_revision: 0,
            audit_profile: audit::AuditProfile::Print,
            audit_tags: HashMap::new(),
            audit_hide_tagged: true,
            audit_min_ppi: 300.0,
            dialog_excel_open: false,
            dialog_toc_open: false,
            dialog_cover_open: false,
            dialog_hf_open: false,
            dialog_manual_open: false,
            dialog_manual_tab: 0,
            dialog_utils_open: false,
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
            active_profile_id: default_profile_id(),
            profile_custom: None,
            profile_user_locked: false,
            profile_detected_id: None,
            dialog_profile_customize_open: false,
            excel_cover: None,
        }
    }
}

impl PubgenState {
    /// Gets the active publication profile (custom profile if active_profile_id == "custom", else looked up via ProfileManager).
    pub fn active_profile(&self) -> super::profiles::PubgenProfile {
        if self.active_profile_id == "custom"
            && let Some(ref c) = self.profile_custom
        {
            return c.clone();
        }
        super::profiles::ProfileManager::get_profile(&self.active_profile_id)
    }

    /// Sets the active publication profile and synchronizes table styling and numbering flags.
    pub fn apply_profile(&mut self, profile: super::profiles::PubgenProfile, user_locked: bool) {
        self.active_profile_id = profile.id.clone();
        self.card2_col_numbers = profile.table_styling.include_column_number_row;
        if profile.table_styling.horizontal_rules_only {
            self.card2_style = 3;
        }
        if user_locked {
            self.profile_user_locked = true;
        }
        if profile.id == "bps_brs" {
            self.cover_style = 13;
            self.cover_include_inside_back = false;
        }
        if profile.id == "custom" {
            self.profile_custom = Some(profile);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::*;
    use super::*;

    #[test]
    fn test_pubgen_state_bilingual_defaults_and_serde() {
        let st = PubgenState::default();
        assert!(!st.bilingual_output);
        assert!(!st.show_about_dialog);
        assert_eq!(st.locale, Locale::Id);
        assert_eq!(st.doc_language, "");

        let json_str = serde_json::to_string(&st).unwrap();
        let deser: PubgenState = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deser.bilingual_output, st.bilingual_output);
        assert_eq!(deser.locale, st.locale);
        assert_eq!(deser.doc_language, st.doc_language);
        assert_eq!(deser.excel_source_path, st.excel_source_path);
        assert_eq!(deser.excel_status, st.excel_status);
        assert_eq!(deser.toc_status, st.toc_status);
        assert_eq!(deser.utilities_status, st.utilities_status);
        assert_eq!(deser.toc_insert_after, st.toc_insert_after);

        // Deserializing older JSON lacking the bilingual fields falls back to defaults safely
        let old_json = r#"{"card1_path":"","card1_records":[],"card1_headers":[],"card1_type_col":0,"card1_num_col":1,"card1_title_col":2,"card1_preset":0,"card1_range_all":true,"card1_range_from":1,"card1_range_to":10,"card1_append":true,"card1_status":null,"card2_path":"","card2_matrix":[],"card2_first_row_header":true,"card2_col_numbers":true,"card2_table_footer":true,"card2_style":3,"card2_placement":2,"card2_status":null}"#;
        let old_deser: PubgenState = serde_json::from_str(old_json).unwrap();
        assert!(!old_deser.bilingual_output);
        assert_eq!(old_deser.locale, Locale::Id);
        assert_eq!(old_deser.doc_language, "");
        assert_eq!(old_deser.excel_source_path, "");
        assert_eq!(old_deser.excel_status, None);
        assert_eq!(old_deser.toc_status, None);
        assert_eq!(old_deser.utilities_status, None);
        assert!(!old_deser.toc_insert_after);
    }

    #[test]
    fn test_workflow_redesign_state_and_paths() {
        let mut st = PubgenState::default();
        // Master Excel auto-generate is enabled by default for seamless 1-click publishing
        assert!(st.excel_auto_generate);
        assert!(st.excel_source_path.is_empty());
        assert!(st.excel_status.is_none());
        assert!(st.toc_status.is_none());
        assert!(st.utilities_status.is_none());

        // Simulate importing an Excel file
        st.excel_source_path = "C:/data/KBDA2026.xlsx".into();
        assert_eq!(std::path::Path::new(&st.excel_source_path).file_name().unwrap(), "KBDA2026.xlsx");

        // Status messaging is localized per section
        st.excel_status = Some(("Converted 12 sheets successfully".into(), true));
        st.toc_status = Some(("Table of Contents added".into(), true));
        st.utilities_status = Some(("Switched to Single-sided layout".into(), true));

        assert!(st.excel_status.as_ref().unwrap().1);
        assert!(st.toc_status.as_ref().unwrap().1);
        assert!(st.utilities_status.as_ref().unwrap().1);
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
}
