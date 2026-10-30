//! Vector chart form options popup window for Pubgen.
//!
//! Provides interactive UI to create a new statistical vector chart or edit
//! an existing chart directly on the active document spread.

use egui::RichText;
use serde_json::json;

use super::layout::compute_layout;
use super::spec::{ChartSpec, ChartType, SeriesSpec, ValueFormat, extract_chart_data, index_to_col_letter, is_column_header, parse_cell_number};
use crate::DesignApp;
use crate::dock::pubgen::excel::ExcelSheetData;
use crate::dock::pubgen::state::PubgenState;
use crate::theme::Tokens;

const DEFAULT_VILLAGE_SAMPLE_CSV: &str = "\
nmdesa,title,title_en,value\n\
CINTAASIH,Grafik 01.01 Indeks Pembangunan,Chart 01.01 Development Index,78.5\n\
MULANGSARI,Grafik 01.01 Indeks Pembangunan,Chart 01.01 Development Index,82.1\n\
TAMANMEKAR,Grafik 01.01 Indeks Pembangunan,Chart 01.01 Development Index,64.3\n\
CIPTASARI,Grafik 01.01 Indeks Pembangunan,Chart 01.01 Development Index,71.0\n\
TAMANSARI,Grafik 01.01 Indeks Pembangunan,Chart 01.01 Development Index,75.4\n\
JATILAKSANA,Grafik 01.01 Indeks Pembangunan,Chart 01.01 Development Index,69.8\n\
KERTASARI,Grafik 01.01 Indeks Pembangunan,Chart 01.01 Development Index,85.2\n\
MEDALSARI,Grafik 01.01 Indeks Pembangunan,Chart 01.01 Development Index,60.5";

const DEFAULT_SAMPLE_CSV: &str = "\
Tahun,Pertanian,Industri,Jasa
2021,3.8,4.2,5.1
2022,4.1,4.5,5.6
2023,3.9,4.8,5.9
2024,4.3,5.1,6.2
2025,4.5,5.4,6.5";

/// Index of the bundled Pangkalan village map in the stock map list.
///
/// The stock list is sorted by display name, so index 0 is not Pangkalan; the bundled
/// village sample data only matches Pangkalan, hence this lookup.
fn default_map_stock_idx() -> usize {
    super::stock_maps::list_stock_map_charts().iter().position(|m| m.id == "PANGKALAN").unwrap_or(0)
}

/// Title text of a chart title frame (a frame whose story starts with Grafik/Figure/Chart).
fn chart_title_of(doc: &designcraft_doc::Document, it: &designcraft_doc::Item) -> Option<String> {
    let designcraft_doc::Content::Text(tf) = &it.content else {
        return None;
    };
    let story = doc.story(tf.story)?;
    let text = story.text.trim();
    (text.starts_with("Grafik") || text.starts_with("Figure") || text.starts_with("Chart")).then(|| text.to_string())
}

/// Finds the chart container (group or bare title frame) and its title on a spread.
///
/// `selected` prioritises the item the user picked; passing `None` keeps the first chart found.
fn find_chart_container(
    doc: &designcraft_doc::Document,
    spread: &designcraft_doc::Spread,
    selected: Option<designcraft_doc::ItemId>,
) -> Option<(u64, String)> {
    for top in &spread.items {
        if let Some(sel) = selected {
            let contains = top.id == sel
                || match &top.content {
                    designcraft_doc::Content::Group { items } => items.iter().any(|c| c.id == sel),
                    _ => false,
                };
            if !contains {
                continue;
            }
        }
        if let Some(title) = chart_title_of(doc, top) {
            return Some((top.id.0, title));
        }
        if let designcraft_doc::Content::Group { items } = &top.content
            && let Some(title) = items.iter().find_map(|c| chart_title_of(doc, c))
        {
            return Some((top.id.0, title));
        }
    }
    None
}

/// Recognized table on a page available for chart data extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableTable {
    /// 1-based page number.
    pub page_1_based: usize,
    /// Table number prefix / identifier, e.g. "01.01".
    pub table_num: String,
    /// Table clean title, e.g. "Luas Daerah Menurut Desa/Kelurahan...".
    pub title: String,
    /// Human-friendly display label for ComboBox, e.g. "Tabel 01.01 — Luas Daerah... (Page 14)".
    pub label: String,
}

/// Where a new chart page should be placed in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartPlacement {
    /// Insert a new page after the currently selected / active page (or right before back cover if on last page / back cover)
    SelectedPage,
    /// Insert a new page right after the source table's page
    AfterTable,
    /// Place onto the current selected page without creating a new page
    CurrentPage,
}

impl Default for ChartPlacement {
    fn default() -> Self {
        Self::AfterTable
    }
}

/// Form state for the Chart Options popup window.
#[derive(Debug, Clone)]
pub struct ChartDialog {
    pub chart_type: ChartType,
    pub title_id: String,
    pub title_en: String,
    pub category_col: String,
    pub series_1_col: String,
    pub series_1_label_id: String,
    pub series_1_label_en: String,
    pub series_2_enabled: bool,
    pub series_2_col: String,
    pub series_2_label_id: String,
    pub series_2_label_en: String,
    pub series_3_enabled: bool,
    pub series_3_col: String,
    pub series_3_label_id: String,
    pub series_3_label_en: String,
    pub data_start_row: usize,
    pub data_end_row: usize,
    pub value_format: ValueFormat,
    pub show_value_labels: bool,
    pub show_legend: bool,
    pub show_grid: bool,
    pub max_value_override: String,
    pub map_stock_idx: usize,
    pub map_custom_path: String,
    pub map_palette_idx: usize,
    /// Editable raw CSV/TSV tabular data.
    pub raw_data: String,
    /// Target spread index where the chart will be placed.
    pub target_spread: usize,
    /// 0-based page number that was active/selected when dialog opened.
    pub selected_page: usize,
    /// 1-based page number to import table data from.
    pub import_page: usize,
    /// Insertion location mode.
    pub placement: ChartPlacement,
    /// Whether to automatically insert a new page and place the chart on it.
    pub insert_on_new_page: bool,
    /// If editing an existing chart on the spread, store its item ID to replace it.
    pub editing_item_id: Option<u64>,
    /// Status message (text, is_success).
    pub status: Option<(String, bool)>,
}

/// Extracts number and clean title from a string like "Tabel 02.03 Jumlah Penduduk..."
fn extract_num_and_title(s: &str) -> (String, String) {
    let trimmed = s.trim();
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() >= 2
        && (words[0].eq_ignore_ascii_case("tabel")
            || words[0].eq_ignore_ascii_case("table")
            || words[0].eq_ignore_ascii_case("grafik")
            || words[0].eq_ignore_ascii_case("figure"))
    {
        let num = words[1].trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
        if num.chars().any(|c| c.is_ascii_digit()) {
            let rest = words[2..].join(" ");
            return (num.to_string(), rest);
        }
    }
    (String::new(), trimmed.to_string())
}

/// Parses a table title string into (table_number, indonesian_title, english_title).
pub fn parse_table_source_title(full_text: &str) -> (String, String, Option<String>) {
    let raw_lines: Vec<&str> = full_text.split(['\n', '\u{2028}']).map(str::trim).filter(|s| !s.is_empty()).collect();

    if raw_lines.len() == 1
        && let Some((id_part, en_part)) = raw_lines[0].split_once(" / ")
    {
        let (num, id_clean) = extract_num_and_title(id_part);
        let (_, en_clean) = extract_num_and_title(en_part);
        return (num, id_clean, Some(en_clean));
    }

    if raw_lines.is_empty() {
        return (String::new(), String::new(), None);
    }

    let (p0_num, p0_rest) = extract_num_and_title(raw_lines[0]);
    if p0_rest.is_empty() && !p0_num.is_empty() && raw_lines.len() >= 2 {
        let id_clean = raw_lines[1].trim().to_string();
        let en_clean = if raw_lines.len() >= 3 {
            let (_, en_rest) = extract_num_and_title(raw_lines[2]);
            if en_rest.is_empty() && raw_lines.len() >= 4 {
                Some(raw_lines[3].trim().to_string())
            } else if !en_rest.is_empty() {
                Some(en_rest)
            } else {
                Some(raw_lines[2].trim().to_string())
            }
        } else {
            None
        };
        return (p0_num, id_clean, en_clean);
    }

    let id_clean = if !p0_rest.is_empty() { p0_rest } else { raw_lines[0].to_string() };
    let en_clean = if raw_lines.len() >= 2 {
        let (_, en_rest) = extract_num_and_title(raw_lines[1]);
        Some(if !en_rest.is_empty() { en_rest } else { raw_lines[1].to_string() })
    } else {
        None
    };

    (p0_num, id_clean, en_clean)
}

/// Splits a Grafik title into Line 1 ("Grafik XX.XX") and Line 2 ("Judul Data").
pub fn split_grafik_title(raw: &str) -> (String, String) {
    let trimmed = raw.trim();
    if let Some((l1, l2)) = trimmed.split_once('\n') {
        return (l1.trim().to_string(), l2.trim().to_string());
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() >= 3 && words[0].eq_ignore_ascii_case("grafik") && words[1].chars().any(|c| c.is_ascii_digit()) {
        let line1 = format!("Grafik {}", words[1]);
        let line2 = words[2..].join(" ");
        return (line1, line2);
    }
    (trimmed.to_string(), String::new())
}

impl Default for ChartDialog {
    fn default() -> Self {
        Self {
            chart_type: ChartType::BarClustered,
            title_id: "Grafik 01.01\nPertumbuhan Ekonomi Menurut Sektor".into(),
            title_en: "Figure 01.01 Economic Growth by Sector".into(),
            category_col: "A".into(),
            series_1_col: "B".into(),
            series_1_label_id: "Pertanian".into(),
            series_1_label_en: "Agriculture".into(),
            series_2_enabled: true,
            series_2_col: "C".into(),
            series_2_label_id: "Industri".into(),
            series_2_label_en: "Industry".into(),
            series_3_enabled: false,
            series_3_col: "D".into(),
            series_3_label_id: "Jasa".into(),
            series_3_label_en: "Services".into(),
            data_start_row: 2,
            data_end_row: 6,
            value_format: ValueFormat::Percent,
            show_value_labels: true,
            show_legend: true,
            show_grid: true,
            max_value_override: String::new(),
            map_stock_idx: default_map_stock_idx(),
            map_custom_path: String::new(),
            map_palette_idx: 0,
            raw_data: DEFAULT_SAMPLE_CSV.into(),
            target_spread: 0,
            selected_page: 0,
            import_page: 1,
            placement: ChartPlacement::AfterTable,
            insert_on_new_page: true,
            editing_item_id: None,
            status: None,
        }
    }
}

impl ChartDialog {
    /// Initialize with defaults, attempting to read active spread and selection.
    pub fn new(app: &DesignApp, edit: bool) -> Self {
        let mut dlg = Self::default();
        let Some(active) = app.session.active() else {
            return dlg;
        };

        let cur_page = active.selection.page.or_else(|| crate::canvas::current_page(app)).unwrap_or(0);
        let cur_spread = active.doc.page_loc(cur_page).map_or(0, |(si, _)| si);
        dlg.target_spread = cur_spread;
        dlg.selected_page = cur_page;
        dlg.import_page = cur_page.saturating_add(1);

        if edit {
            dlg.placement = ChartPlacement::CurrentPage;
            dlg.insert_on_new_page = false;
        } else {
            dlg.placement = ChartPlacement::SelectedPage;
            dlg.insert_on_new_page = true;
        }

        if edit {
            // Prefer the chart containing the selected item, then the first chart on the spread.
            // A chart is cleared as one container so editing never leaves a duplicate behind.
            if let Some(spread) = active.doc.spreads.get(cur_spread) {
                let selected = active.selection.items.first().copied();
                let found = find_chart_container(&active.doc, spread, selected).or_else(|| find_chart_container(&active.doc, spread, None));
                if let Some((container_id, full)) = found {
                    dlg.editing_item_id = Some(container_id);
                    if let Some((id_part, en_part)) = full.split_once(" / ") {
                        dlg.title_id = id_part.trim().to_string();
                        dlg.title_en = en_part.trim().to_string();
                    } else {
                        dlg.title_id = full;
                    }
                }
            }
        }

        dlg
    }

    /// Automatically detects category, series, and titles from the CSV header row.
    pub fn auto_detect_columns(&mut self) {
        let sheet = self.parse_sheet_data();
        let Some(header_row) = sheet.grid.first() else {
            return;
        };
        let headers: Vec<String> = header_row.iter().map(|c| c.trim().to_lowercase()).collect();

        for (idx, h) in headers.iter().enumerate() {
            let col_letter = super::spec::index_to_col_letter(idx);
            if matches!(h.as_str(), "nmdesa" | "desa" | "village" | "kecamatan" | "wilayah" | "category" | "kategori") {
                self.category_col = col_letter;
            } else if matches!(h.as_str(), "value" | "nilai" | "jumlah" | "total" | "persen" | "rate") {
                self.series_1_col = col_letter;
            }
        }

        if let Some(row1_cells) = sheet.grid.get(1) {
            if let Some(t_idx) = headers.iter().position(|h| h == "title" || h == "judul")
                && let Some(t_val) = row1_cells.get(t_idx).map(|c| c.trim())
                && !t_val.is_empty()
            {
                self.title_id = t_val.to_string();
            }
            if let Some(te_idx) = headers.iter().position(|h| h == "title_en" || h == "judul_en")
                && let Some(te_val) = row1_cells.get(te_idx).map(|c| c.trim())
                && !te_val.is_empty()
            {
                self.title_en = te_val.to_string();
            }

            self.data_start_row = 2;
            self.data_end_row = sheet.grid.len();
        }
    }

    /// Parse the raw CSV/TSV data into `ExcelSheetData`.
    pub fn parse_sheet_data(&self) -> ExcelSheetData {
        let delim = crate::dock::pubgen::csv::detect_delimiter(&self.raw_data);
        let grid = crate::dock::pubgen::csv::parse_csv_content_with_delim(&self.raw_data, delim)
            .into_iter()
            .map(|row| row.into_iter().map(|cell| cell.trim().to_string()).collect())
            .collect();

        ExcelSheetData { name: "ChartData".into(), grid, merges: Vec::new() }
    }

    /// Switches the active archetype and normalises options that archetype ignores.
    pub fn select_chart_type(&mut self, chart_type: ChartType) {
        self.chart_type = chart_type;
        // Map charts colour geographic regions by an absolute value (population, index, …),
        // so a percentage format would mislabel every region and legend step.
        if chart_type == ChartType::Map {
            self.value_format = ValueFormat::Number;
        }
        self.clamp_to_chart_type();
    }

    /// Loads the bundled Pangkalan village sample and selects its matching stock map.
    pub fn load_village_sample(&mut self) {
        self.raw_data = DEFAULT_VILLAGE_SAMPLE_CSV.to_string();
        self.category_col = "A".into();
        self.series_1_col = "D".into();
        self.series_1_label_id = "Nilai Indeks".into();
        self.series_1_label_en = "Index Value".into();
        self.data_start_row = 2;
        self.data_end_row = 9;
        self.title_id = "Grafik 01.01 Indeks Pembangunan Desa, 2026".into();
        self.title_en = "Chart 01.01 Village Development Index, 2026".into();
        self.map_stock_idx = default_map_stock_idx();
        self.value_format = ValueFormat::Number;
    }

    /// Build the `ChartSpec` from the current dialog parameters.
    pub fn build_spec(&self) -> ChartSpec {
        let mut series = Vec::new();
        series.push(SeriesSpec {
            col: self.series_1_col.trim().to_uppercase(),
            label_id: self.series_1_label_id.trim().to_string(),
            label_en: self.series_1_label_en.trim().to_string(),
        });

        let max_series = match self.chart_type {
            ChartType::Donut | ChartType::Gauge | ChartType::Map => 1,
            ChartType::PopulationPyramid => 2,
            _ => 3,
        };

        if max_series >= 2 && self.series_2_enabled && !self.series_2_col.trim().is_empty() {
            series.push(SeriesSpec {
                col: self.series_2_col.trim().to_uppercase(),
                label_id: self.series_2_label_id.trim().to_string(),
                label_en: self.series_2_label_en.trim().to_string(),
            });
        }

        if max_series >= 3 && self.series_3_enabled && !self.series_3_col.trim().is_empty() {
            series.push(SeriesSpec {
                col: self.series_3_col.trim().to_uppercase(),
                label_id: self.series_3_label_id.trim().to_string(),
                label_en: self.series_3_label_en.trim().to_string(),
            });
        }

        let max_val = self.max_value_override.trim().parse::<f64>().ok();

        ChartSpec {
            chart_type: self.chart_type,
            title_id: self.title_id.trim().to_string(),
            title_en: self.title_en.trim().to_string(),
            category_col: self.category_col.trim().to_uppercase(),
            series,
            data_start_row: self.data_start_row,
            data_end_row: self.data_end_row,
            value_format: self.value_format,
            show_value_labels: self.show_value_labels,
            show_legend: self.show_legend,
            show_grid: self.show_grid,
            max_value_override: max_val,
            geojson_path: if self.chart_type == ChartType::Map {
                let stock_maps = super::stock_maps::list_stock_map_charts();
                if self.map_stock_idx == 999 {
                    let custom = self.map_custom_path.trim();
                    (!custom.is_empty()).then(|| custom.to_string())
                } else {
                    stock_maps
                        .get(self.map_stock_idx)
                        .or_else(|| stock_maps.iter().find(|m| m.id == "PANGKALAN"))
                        .map(|m| m.path.to_string_lossy().to_string())
                }
            } else {
                None
            },
            map_palette_idx: self.map_palette_idx,
        }
    }

    /// Adjust series and row count to fit the active chart archetype's capacity.
    pub fn clamp_to_chart_type(&mut self) {
        if self.chart_type == ChartType::Donut || self.chart_type == ChartType::Gauge || self.chart_type == ChartType::Map {
            self.series_2_enabled = false;
            self.series_3_enabled = false;
        } else if self.chart_type == ChartType::PopulationPyramid {
            self.series_2_enabled = true;
            self.series_3_enabled = false;
        }
    }

    /// Apply or insert the chart on the active document spread.
    pub fn apply_chart(&mut self, app: &mut DesignApp, bilingual: bool) -> Result<u64, String> {
        let sheet_data = self.parse_sheet_data();
        let spec = self.build_spec();

        let (categories, series_values) = extract_chart_data(&sheet_data, &spec).map_err(|e| format!("Failed to extract chart data: {e}"))?;

        // Validate the GeoJSON before touching the document, so a bad map path cannot leave
        // an empty chart page behind.
        if spec.chart_type == ChartType::Map {
            let path = spec.geojson_path.as_deref().unwrap_or("");
            super::stock_maps::load_chart_geo_map(path).map_err(|e| format!("Cannot render map chart: {e}"))?;
        }

        // If insert_on_new_page is enabled and not editing an existing chart,
        // automatically create a new blank page for the chart according to placement mode.
        let target_page_idx = if self.insert_on_new_page && self.editing_item_id.is_none() && self.placement != ChartPlacement::CurrentPage {
            let is_back_cover = {
                let active = app.session.active();
                let total = active.as_ref().map(|a| a.doc.page_count()).unwrap_or(1);
                let bp = active.as_ref().and_then(|a| crate::dock::pubgen::covers::find_back_cover_page(&a.doc));
                bp == Some(self.selected_page) || (self.selected_page >= total.saturating_sub(1) && bp.is_some())
            };

            let ref_page = match self.placement {
                ChartPlacement::SelectedPage => {
                    if is_back_cover && self.selected_page > 0 {
                        // When user selected the back cover (or last page), insert right before the back cover
                        self.selected_page.saturating_sub(1)
                    } else {
                        self.selected_page
                    }
                }
                ChartPlacement::AfterTable => {
                    if self.import_page > 0 {
                        self.import_page.saturating_sub(1)
                    } else {
                        self.selected_page
                    }
                }
                ChartPlacement::CurrentPage => self.selected_page,
            };

            crate::dock::pubgen::page_utils::execute_add_blank_page(app, ref_page)?;
            // Keep back cover at the very end of the document if one exists
            crate::dock::pubgen::covers::ensure_back_cover_at_end(app);

            let new_page = ref_page + 1;
            crate::canvas::go_to_page(app, new_page);
            Some(new_page)
        } else if self.insert_on_new_page {
            let p = self.selected_page;
            crate::canvas::go_to_page(app, p);
            Some(p)
        } else {
            // Place on the spread chosen by the Target Spread control.
            let page = app.session.active().map(|a| a.doc.first_page_of_spread(self.target_spread.min(a.doc.spreads.len().saturating_sub(1))));
            if let Some(p) = page {
                crate::canvas::go_to_page(app, p);
            }
            None
        };

        let (margin, target_sp) = {
            let active = app.session.active().ok_or("No active document open")?;
            let sp_count = active.doc.spreads.len();
            if sp_count == 0 {
                return Err("Document contains no spreads".into());
            }

            let (spread_idx, page_idx_in_sp) = if let Some(p) = target_page_idx {
                active.doc.page_loc(p).unwrap_or((0, 0))
            } else {
                let si = self.target_spread.min(sp_count.saturating_sub(1));
                // Keep the page the user is on when the chosen spread still contains it.
                let page_idx = match active.doc.page_loc(self.selected_page) {
                    Some((selected_si, selected_pi)) if selected_si == si => selected_pi,
                    _ => 0,
                };
                (si, page_idx)
            };
            let spread = active.doc.spreads.get(spread_idx).ok_or("Invalid spread index")?;
            let page = spread.pages.get(page_idx_in_sp).or_else(|| spread.pages.first());
            let rect = if let Some(pg) = page {
                let m = pg.margin_rect();
                designcraft_geom::Rect { x0: m.x0, y0: m.y0, x1: m.x1, y1: m.y1 - 10.0 }
            } else {
                designcraft_geom::Rect { x0: 36.0, y0: 56.0, x1: 559.0, y1: 780.0 }
            };
            (rect, spread_idx)
        };

        // If editing an existing chart container, clear the whole chart (frames + title) first,
        // otherwise the re-render would leave the previous chart behind.
        if let Some(old_id) = self.editing_item_id {
            let _ = app.run("edit.clear", json!({"ids": [old_id]}));
        }

        let ids_before: Vec<u64> =
            app.session.active().and_then(|a| a.doc.spreads.get(target_sp)).map(|s| s.items.iter().map(|it| it.id.0).collect()).unwrap_or_default();

        let layout = compute_layout(&spec, &categories, &series_values, margin, bilingual)?;
        let count = super::emitter::emit_chart(app, target_sp, &spec, &layout, &[], bilingual)?;

        // `emit_chart` groups the emitted frames; keep that group so the title can join it and
        // a later edit clears the whole chart at once.
        let chart_group = app.session.active().and_then(|a| {
            let spread = a.doc.spreads.get(target_sp)?;
            let new_top: Vec<&std::sync::Arc<designcraft_doc::Item>> = spread.items.iter().filter(|it| !ids_before.contains(&it.id.0)).collect();
            match new_top.as_slice() {
                [only] if matches!(only.content, designcraft_doc::Content::Group { .. }) => Some(only.id.0),
                _ => None,
            }
        });

        let mut title_fid = None;

        if !self.title_id.is_empty() {
            let (line1, line2) = split_grafik_title(&self.title_id);
            let id_formatted = if !line2.is_empty() { format!("{line1}\n{line2}") } else { line1.clone() };

            let en_trimmed = self.title_en.trim();
            let is_bilingual_title = bilingual && !en_trimmed.is_empty();

            let title_text = if is_bilingual_title { format!("{id_formatted}\n{en_trimmed}") } else { id_formatted.clone() };

            let title_h = if is_bilingual_title { 56.0 } else { 44.0 };
            let title_rect = [margin.x0, margin.y0, margin.x1, margin.y0 + title_h];

            let fid = super::emitter::emit_text(app, target_sp, title_rect, &title_text, 11.0, true, "center", Some("#1A1A1A"))?;
            title_fid = Some(fid);

            // Format character runs:
            // Line 1 & Line 2: Bold 11pt
            // Line 3 (English): Italic 10pt
            let _ = app.session.edit(|doc, _| {
                let story_id = if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid))
                    && let designcraft_doc::Content::Text(tf) = &mut it.content
                {
                    tf.options.vertical_justification = designcraft_doc::VerticalJustification::Top;
                    Some(tf.story)
                } else {
                    None
                };

                if let Some(sid) = story_id
                    && let Some(story) = doc.story_mut(sid)
                {
                    for p in &mut story.paras {
                        p.para.align = Some(designcraft_doc::Align::Center);
                    }
                    let id_len = id_formatted.len();
                    if is_bilingual_title {
                        let en_len = en_trimmed.len();
                        story.chars = vec![
                            designcraft_doc::CharRun {
                                len: id_len + 1, // include newline
                                format: designcraft_doc::CharFormat {
                                    over: designcraft_doc::CharAttrs {
                                        font_family: Some("Arial".into()),
                                        size: Some(11.0),
                                        font_style: Some("Bold".into()),
                                        fill: Some("[Black]".into()),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                            },
                            designcraft_doc::CharRun {
                                len: en_len,
                                format: designcraft_doc::CharFormat {
                                    over: designcraft_doc::CharAttrs {
                                        font_family: Some("Arial".into()),
                                        size: Some(10.0),
                                        font_style: Some("Italic".into()),
                                        fill: Some("[Black]".into()),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                            },
                        ];
                    } else {
                        story.chars = vec![designcraft_doc::CharRun {
                            len: id_len,
                            format: designcraft_doc::CharFormat {
                                over: designcraft_doc::CharAttrs {
                                    font_family: Some("Arial".into()),
                                    size: Some(11.0),
                                    font_style: Some("Bold".into()),
                                    fill: Some("[Black]".into()),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        }];
                    }
                }
                Ok(serde_json::Value::Null)
            });
        }

        // Wrap the chart group and its title in one outer group, and remember it for edits.
        let mut chart_ids: Vec<u64> = chart_group.into_iter().collect();
        chart_ids.extend(title_fid);
        self.editing_item_id = if chart_ids.len() > 1 {
            match app.run("object.group", json!({"ids": chart_ids})) {
                Ok(res) => res.get("id").and_then(serde_json::Value::as_u64).or(chart_group).or(title_fid),
                Err(_) => chart_group.or(title_fid),
            }
        } else {
            chart_group.or(title_fid)
        };

        let page_msg = if let Some(p) = target_page_idx {
            let lbl = app.session.page_label(p);
            format!(" on Page {lbl}")
        } else {
            String::new()
        };
        self.status = Some((format!("Chart successfully rendered{page_msg} ({} vector objects emitted)", count), true));
        Ok(count as u64)
    }

    /// Scans the active document for pages containing tables and extracts their table numbers and titles.
    pub fn find_available_tables(&self, app: &DesignApp, st: Option<&PubgenState>) -> Vec<AvailableTable> {
        let Some(active) = app.session.active() else {
            return Vec::new();
        };

        let total_pages = active.doc.page_count();
        let mut tables = Vec::new();

        for p in 0..total_pages {
            let Some(target) = super::super::ai::detect_table_page_target(app, p) else {
                continue;
            };

            let page_1_based = p + 1;
            let (si, _) = match active.doc.page_loc(p) {
                Some(loc) => loc,
                None => continue,
            };
            let spread = match active.doc.spreads.get(si) {
                Some(s) => s,
                None => continue,
            };

            // 1. Detect table number (e.g. "01.01")
            let mut detected_num = None;

            // 1a. Check items on this spread that represent table number slots
            for it in &spread.items {
                let lbl = it.label.to_lowercase();
                if (lbl.contains("number") || lbl.contains("table_num") || lbl.contains("table_number"))
                    && let Some(tf) = it.text_frame()
                    && let Some(story) = active.doc.story(tf.story)
                {
                    let raw = story.text.trim();
                    let stripped = raw
                        .strip_prefix("Tabel ")
                        .or_else(|| raw.strip_prefix("tabel "))
                        .or_else(|| raw.strip_prefix("Table "))
                        .or_else(|| raw.strip_prefix("table "))
                        .unwrap_or(raw)
                        .trim();
                    if stripped.chars().any(|c| c.is_ascii_digit()) && stripped.contains('.') {
                        detected_num = Some(stripped.to_string());
                        break;
                    }
                }
            }

            // 1b. Check title frame prefix if not found in slots
            if detected_num.is_none()
                && let Some(tfid) = target.title_fid
                && let Some(item) = active.doc.item(designcraft_doc::ItemId(tfid))
                && let Some(tf) = item.text_frame()
                && let Some(tstory) = active.doc.story(tf.story)
            {
                let full = tstory.text.trim();
                let first_line = full.lines().next().unwrap_or(full);
                let (_clean_title, prefix_opt) = super::super::ai::extract_table_title_parts(first_line);
                if let Some(prefix) = prefix_opt {
                    let num_part = prefix.split_whitespace().nth(1).unwrap_or("");
                    if !num_part.is_empty() && num_part.contains('.') {
                        detected_num = Some(num_part.to_string());
                    }
                }
            }

            // 1c. Match with st.card2_sheet_tables if available
            if detected_num.is_none()
                && let Some(st_ref) = st
                && let Some(csv_p) = st_ref.card2_sheet_tables.get(tables.len())
            {
                let stem = std::path::Path::new(csv_p).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let formatted = stem.replace('_', ".");
                if formatted.chars().any(|c| c.is_ascii_digit()) && formatted.contains('.') {
                    detected_num = Some(formatted);
                }
            }

            let table_num = detected_num.unwrap_or_else(|| format!("{:02}.01", tables.len() + 1));

            // 2. Extract clean title
            let mut clean_title = String::new();
            if let Some(tfid) = target.title_fid
                && let Some(item) = active.doc.item(designcraft_doc::ItemId(tfid))
                && let Some(tf) = item.text_frame()
                && let Some(tstory) = active.doc.story(tf.story)
            {
                let full = tstory.text.trim();
                let first_line = full.lines().next().unwrap_or(full);
                let (first_part, _) = first_line.split_once(" / ").unwrap_or((first_line, ""));
                let (clean, _) = super::super::ai::extract_table_title_parts(first_part.trim());
                clean_title = clean.trim().to_string();
            }

            let short_title: String = if clean_title.is_empty() {
                format!("Tabel {}", table_num)
            } else if clean_title.chars().count() > 38 {
                format!("{}…", clean_title.chars().take(38).collect::<String>())
            } else {
                clean_title.clone()
            };

            let label = format!("Tabel {} — {} (p. {})", table_num, short_title, page_1_based);

            tables.push(AvailableTable { page_1_based, table_num, title: clean_title, label });
        }

        // If no table pages in document yet, fall back to sheet table paths
        if tables.is_empty()
            && let Some(st_ref) = st
            && !st_ref.card2_sheet_tables.is_empty()
        {
            for (idx, csv_p) in st_ref.card2_sheet_tables.iter().enumerate() {
                let stem = std::path::Path::new(csv_p).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let table_num = stem.replace('_', ".");
                let label = format!("Tabel {} (Sheet {})", table_num, idx + 1);
                tables.push(AvailableTable { page_1_based: idx + 1, table_num, title: format!("Tabel Sheet {}", idx + 1), label });
            }
        }

        tables
    }

    /// Read table from the specified 1-based page number and auto-fill dialog fields.
    #[allow(clippy::needless_range_loop, clippy::cognitive_complexity)]
    pub fn import_from_table_page(&mut self, app: &DesignApp, page_1_based: usize) -> Result<(), String> {
        let page_idx = page_1_based.saturating_sub(1);
        let target = super::super::ai::detect_table_page_target(app, page_idx).ok_or_else(|| format!("No table found on page {}", page_1_based))?;

        let active = app.session.active().ok_or("No active document open")?;
        let story = active.doc.story(target.table_sid).ok_or("Table story not found")?;
        let table = story.tables.get(&target.table_id).ok_or("Table not found")?;

        // 1. Title recognition from title frame
        let mut recognized_num = String::new();
        let mut recognized_id = String::new();
        let mut recognized_en: Option<String> = None;

        if let Some(tfid) = target.title_fid
            && let Some(item) = active.doc.item(designcraft_doc::ItemId(tfid))
            && let Some(tf) = item.text_frame()
            && let Some(tstory) = active.doc.story(tf.story)
        {
            let (num, id_clean, en_clean) = parse_table_source_title(&tstory.text);
            recognized_num = num;
            recognized_id = id_clean;
            recognized_en = en_clean;
        }

        // If table number wasn't found in title frame, fallback to page-based detection
        if recognized_num.is_empty() {
            let tables = self.find_available_tables(app, None);
            if let Some(tbl) = tables.iter().find(|t| t.page_1_based == page_1_based) {
                recognized_num = tbl.table_num.clone();
                if recognized_id.is_empty() {
                    recognized_id = tbl.title.clone();
                }
            }
        }
        if recognized_num.is_empty() {
            recognized_num = format!("{:02}.01", page_1_based);
        }

        // Format Title (ID):
        // [line#1] Grafik XX.XX
        // [line#2] Judul Sumber Data (table)
        self.title_id =
            if !recognized_id.is_empty() { format!("Grafik {}\n{}", recognized_num, recognized_id) } else { format!("Grafik {}", recognized_num) };

        // Format Title (EN):
        // Figure XX.XX English Title
        if let Some(en) = recognized_en {
            self.title_en = format!("Figure {} {}", recognized_num, en);
        } else if !recognized_id.is_empty() {
            self.title_en = format!("Figure {} {}", recognized_num, recognized_id);
        }

        // 2. Extract 2D cell text
        let nrows = table.nrows();
        let ncols = table.ncols();
        if nrows < 2 || ncols < 2 {
            return Err("Table must have at least 2 rows and 2 columns".into());
        }

        let mut raw_cells: Vec<Vec<String>> = Vec::with_capacity(nrows);
        for r in 0..nrows {
            let mut row = Vec::with_capacity(ncols);
            for c in 0..ncols {
                let txt = table.cell(r, c).map(|cell| cell.text.text.trim().to_string()).unwrap_or_default();
                row.push(txt);
            }
            raw_cells.push(row);
        }

        // 3. Helper to detect column numbering markers like "(1)", "1", "(2)", etc.
        let is_col_header_marker = |s: &str| -> bool {
            let t = s.trim();
            if is_column_header(t) {
                return true;
            }
            if t.starts_with('(') && t.ends_with(')') {
                let inner = t[1..t.len() - 1].trim();
                return !inner.is_empty() && inner.chars().all(|c| c.is_ascii_alphanumeric());
            }
            false
        };

        // 4. Header labels (ID and EN) for each column
        let mut col_headers: Vec<(String, String)> = Vec::with_capacity(ncols);
        for c in 0..ncols {
            let header_text = &raw_cells[0][c];
            let (h_id, h_en) = if let Some((p1, p2)) = header_text.split_once('\n') {
                (p1.trim().to_string(), p2.trim().to_string())
            } else if let Some((p1, p2)) = header_text.split_once(" / ") {
                (p1.trim().to_string(), p2.trim().to_string())
            } else {
                (header_text.clone(), header_text.clone())
            };
            col_headers.push((h_id, h_en));
        }

        // 5. Detect numeric data columns (starting from col 1), skipping column header rows
        let mut numeric_cols: Vec<usize> = Vec::new();
        for c in 1..ncols {
            let num_count = (1..nrows).filter(|&r| !is_col_header_marker(&raw_cells[r][0]) && parse_cell_number(&raw_cells[r][c]).is_some()).count();
            if num_count > 0 {
                numeric_cols.push(c);
            }
        }

        if numeric_cols.is_empty() {
            return Err("No numeric data columns detected in table".into());
        }

        // 6. Classify data rows vs summary rows vs empty/section rows
        let is_summary_cat = |cat: &str| -> bool {
            let l = cat.to_lowercase();
            l.starts_with("jumlah")
                || l.starts_with("total")
                || l.starts_with("rata-rata")
                || l.starts_with("average")
                || l.starts_with("kec.")
                || l.starts_with("kecamatan")
                || l.starts_with("kab.")
                || l.starts_with("kabupaten")
                || l.starts_with("prov.")
                || l.starts_with("provinsi")
        };

        let mut valid_data_rows: Vec<Vec<String>> = Vec::new();
        let mut summary_rows: Vec<Vec<String>> = Vec::new();

        for r in 1..nrows {
            let cat_cell = &raw_cells[r][0];
            let clean_cat = if let Some((id_p, _)) = cat_cell.split_once('\n') {
                id_p.trim()
            } else if let Some((id_p, _)) = cat_cell.split_once(" / ") {
                id_p.trim()
            } else {
                cat_cell.as_str()
            };

            if clean_cat.is_empty() || is_col_header_marker(clean_cat) {
                continue;
            }

            let has_num = numeric_cols.iter().any(|&c| parse_cell_number(&raw_cells[r][c]).is_some());
            if !has_num {
                // Section header row without numbers (e.g. "Desa" or "Kelurahan")
                continue;
            }

            let mut clean_row: Vec<String> = Vec::with_capacity(ncols);
            clean_row.push(clean_cat.replace(',', " "));
            for c in 1..ncols {
                let cell = &raw_cells[r][c];
                if let Some(num) = parse_cell_number(cell) {
                    if num.fract().abs() < 1e-6 {
                        clean_row.push(format!("{}", num as i64));
                    } else {
                        clean_row.push(format!("{:.2}", num));
                    }
                } else {
                    clean_row.push(cell.replace(',', " "));
                }
            }

            if is_summary_cat(clean_cat) {
                summary_rows.push(clean_row);
            } else {
                valid_data_rows.push(clean_row);
            }
        }

        if valid_data_rows.is_empty() && !summary_rows.is_empty() {
            valid_data_rows = summary_rows;
            summary_rows = Vec::new();
        }

        if valid_data_rows.is_empty() {
            return Err("No data rows found in table".into());
        }

        // 7. Assemble CSV in self.raw_data
        let mut csv_lines: Vec<String> = Vec::new();
        let header_line: Vec<String> = (0..ncols).map(|c| col_headers[c].0.replace(',', " ")).collect();
        csv_lines.push(header_line.join(","));

        for row in &valid_data_rows {
            csv_lines.push(row.join(","));
        }
        let data_end_idx = csv_lines.len(); // 1-based row index for end of normal data rows

        for row in &summary_rows {
            csv_lines.push(row.join(","));
        }
        self.raw_data = csv_lines.join("\n");

        // 8. Populate row range
        self.data_start_row = 2;
        self.data_end_row = data_end_idx;

        // 9. Populate columns and series based on chart type capacity
        self.category_col = "A".into();
        let max_series = match self.chart_type {
            ChartType::Donut | ChartType::Gauge | ChartType::Map => 1,
            ChartType::PopulationPyramid => 2,
            _ => 3,
        };

        // For Donut chart, prefer a percentage column if available
        let col1 = if self.chart_type == ChartType::Donut {
            numeric_cols
                .iter()
                .find(|&&c| {
                    let h = col_headers[c].0.to_lowercase();
                    h.contains('%') || h.contains("persen") || h.contains("percentage")
                })
                .copied()
                .unwrap_or(numeric_cols[0])
        } else {
            numeric_cols[0]
        };

        self.series_1_col = index_to_col_letter(col1);
        self.series_1_label_id = col_headers[col1].0.clone();
        self.series_1_label_en = col_headers[col1].1.clone();

        let remaining_cols: Vec<usize> = numeric_cols.iter().copied().filter(|&c| c != col1).collect();

        if max_series >= 2 && !remaining_cols.is_empty() {
            let col2 = remaining_cols[0];
            self.series_2_enabled = true;
            self.series_2_col = index_to_col_letter(col2);
            self.series_2_label_id = col_headers[col2].0.clone();
            self.series_2_label_en = col_headers[col2].1.clone();
        } else {
            self.series_2_enabled = false;
        }

        if max_series >= 3 && remaining_cols.len() >= 2 {
            let col3 = remaining_cols[1];
            self.series_3_enabled = true;
            self.series_3_col = index_to_col_letter(col3);
            self.series_3_label_id = col_headers[col3].0.clone();
            self.series_3_label_en = col_headers[col3].1.clone();
        } else {
            self.series_3_enabled = false;
        }

        // For Donut chart, cap row count to at most 6 categories for slice readability
        if self.chart_type == ChartType::Donut && (self.data_end_row.saturating_sub(self.data_start_row) + 1) > 6 {
            self.data_end_row = (self.data_start_row + 5).min(data_end_idx);
        }

        if self.chart_type == ChartType::PopulationPyramid {
            if self.series_1_label_id.is_empty() || self.series_1_label_id.starts_with("Col ") {
                self.series_1_label_id = "Laki-laki".into();
                self.series_1_label_en = "Male".into();
            }
            if self.series_2_label_id.is_empty() || self.series_2_label_id.starts_with("Col ") {
                self.series_2_label_id = "Perempuan".into();
                self.series_2_label_en = "Female".into();
            }
        }

        // 10. Infer ValueFormat
        let h_lower = col_headers[col1].0.to_lowercase();
        if h_lower.contains('%') || h_lower.contains("persen") || h_lower.contains("percentage") {
            self.value_format = ValueFormat::Percent;
        } else if h_lower.contains("rp") || h_lower.contains("rupiah") || h_lower.contains("idr") {
            self.value_format = ValueFormat::CurrencyIdr;
        } else {
            self.value_format = ValueFormat::Number;
        }

        let type_note = if self.chart_type == ChartType::Donut {
            format!("fitted to 1 series '{}', {} slices", self.series_1_label_id, self.data_end_row - self.data_start_row + 1)
        } else {
            format!("{} series columns: {}", numeric_cols.len(), numeric_cols.iter().map(|&c| index_to_col_letter(c)).collect::<Vec<_>>().join(", "))
        };

        self.status = Some((format!("✓ Imported table from page {} ({} data rows, {})", page_1_based, valid_data_rows.len(), type_note), true));

        Ok(())
    }
}

/// Render the Chart Options dialog window in egui.
pub fn chart_dialog_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    let Some(mut dialog) = st.chart_dialog.take() else {
        return;
    };

    let t = Tokens::get(ctx);
    let mut open = true;
    let mut close = false;
    let mut apply = false;

    if let Some(active) = app.session.active() {
        if let Some(p) = active.selection.page {
            dialog.selected_page = p;
        }
    }

    let lang = app.ui.language.clone();
    let is_editing = dialog.editing_item_id.is_some();
    let window_title =
        if is_editing { crate::i18n::tr(&lang, "Edit Vector Chart (Pubgen)") } else { crate::i18n::tr(&lang, "Add Vector Chart (Pubgen)") };

    let available_h = (ctx.content_rect().height() - 90.0).max(200.0);
    let scroll_h = 600.0f32.min(available_h);

    crate::dock::pubgen::dialogs::dialog_window(ctx, window_title, crate::i18n::is_rtl(&lang), false, &mut open, 540.0, |ui| {
        egui::ScrollArea::vertical().max_width(540.0).max_height(scroll_h).auto_shrink([false, true]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 5.0);

            // Section 0: Import from Table Page (Smart Auto-fill)
            ui.group(|ui| {
                let available_tables = dialog.find_available_tables(app, Some(st));

                ui.horizontal(|ui| {
                    ui.label(RichText::new("📊").size(13.0));
                    ui.label(RichText::new(crate::i18n::tr(&lang, "Import from Table:")).strong().size(11.5).color(t.text_strong));

                    if !available_tables.is_empty() {
                        let cur_idx = available_tables.iter().position(|t| t.page_1_based == dialog.import_page).unwrap_or(0);
                        let cur_label = available_tables.get(cur_idx).map(|t| t.label.as_str()).unwrap_or("Select Table");
                        let combo_w = (ui.available_width() - 110.0).clamp(160.0, 310.0);

                        let mut selected_new = None;
                        craft_ui::ComboBox::from_id_salt("pubgen_chart_table_selector").selected_text(cur_label).width(combo_w).show_ui(ui, |ui| {
                            for tbl in &available_tables {
                                let is_sel = tbl.page_1_based == dialog.import_page;
                                if ui.selectable_label(is_sel, &tbl.label).clicked() {
                                    selected_new = Some(tbl.page_1_based);
                                }
                            }
                        });

                        if let Some(pg) = selected_new {
                            dialog.import_page = pg;
                            if let Err(e) = dialog.import_from_table_page(app, pg) {
                                dialog.status = Some((e, false));
                            }
                        }
                    } else {
                        let max_pages = app.session.active().map(|s| s.doc.page_count()).unwrap_or(1).max(1);
                        ui.label(RichText::new(crate::i18n::tr(&lang, "Page:")).size(11.0).color(t.text));
                        ui.add(egui::DragValue::new(&mut dialog.import_page).range(1..=max_pages));
                    }

                    if ui.button(RichText::new(crate::i18n::tr(&lang, "📥 Auto-fill")).strong()).clicked()
                        && let Err(e) = dialog.import_from_table_page(app, dialog.import_page)
                    {
                        dialog.status = Some((e, false));
                    }
                });

                if !is_editing {
                    let sel_label = app.session.page_label(dialog.selected_page);
                    let tbl_label = format!("{}", dialog.import_page);
                    let is_last_or_back_cover = {
                        let active = app.session.active();
                        let total = active.as_ref().map(|a| a.doc.page_count()).unwrap_or(1);
                        let bp = active.as_ref().and_then(|a| crate::dock::pubgen::covers::find_back_cover_page(&a.doc));
                        dialog.selected_page >= total.saturating_sub(1) || bp == Some(dialog.selected_page)
                    };

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("📄").size(12.0));
                        ui.label(RichText::new(crate::i18n::tr(&lang, "Insert Location:")).strong().size(11.0).color(t.text_strong));

                        let opt_selected = if is_last_or_back_cover {
                            format!("New Page before Last Page / Back Cover (p. {})", sel_label)
                        } else {
                            format!("New Page after Selected Page (p. {})", sel_label)
                        };
                        let opt_table = format!("New Page after Source Table (p. {})", tbl_label);
                        let opt_current = format!("On Selected Page (p. {})", sel_label);

                        let current_text = match dialog.placement {
                            ChartPlacement::SelectedPage => &opt_selected,
                            ChartPlacement::AfterTable => &opt_table,
                            ChartPlacement::CurrentPage => &opt_current,
                        };

                        let combo_w = (ui.available_width() - 10.0).clamp(200.0, 360.0);
                        craft_ui::ComboBox::from_id_salt("pubgen_chart_placement").selected_text(current_text).width(combo_w).show_ui(ui, |ui| {
                            if ui.selectable_label(dialog.placement == ChartPlacement::SelectedPage, &opt_selected).clicked() {
                                dialog.placement = ChartPlacement::SelectedPage;
                                dialog.insert_on_new_page = true;
                            }
                            if ui.selectable_label(dialog.placement == ChartPlacement::AfterTable, &opt_table).clicked() {
                                dialog.placement = ChartPlacement::AfterTable;
                                dialog.insert_on_new_page = true;
                            }
                            if ui.selectable_label(dialog.placement == ChartPlacement::CurrentPage, &opt_current).clicked() {
                                dialog.placement = ChartPlacement::CurrentPage;
                                dialog.insert_on_new_page = false;
                            }
                        });
                    });
                }
            });

            ui.add_space(2.0);

            // Section 1: Titles & Chart Type
            ui.label(RichText::new(crate::i18n::tr(&lang, "Chart Type & Titles")).strong().size(11.5).color(t.text_strong));
            ui.horizontal(|ui| {
                ui.label(RichText::new(crate::i18n::tr(&lang, "Archetype:")).size(11.0).color(t.text));
                let cur_type_name = match dialog.chart_type {
                    ChartType::BarClustered => "Vertical Clustered Bar",
                    ChartType::BarStacked => "Vertical Stacked Bar",
                    ChartType::HorizontalBar => "Horizontal Bar (Ranked)",
                    ChartType::Line => "Line Chart (Markers)",
                    ChartType::Area => "Area Chart (Filled)",
                    ChartType::Donut => "Donut / Ring Chart",
                    ChartType::PopulationPyramid => "Population Pyramid (Piramida Penduduk)",
                    ChartType::Gauge => "Gauge / Speedometer Chart",
                    ChartType::Map => "Map / Choropleth (Peta Tematik)",
                };
                craft_ui::ComboBox::from_id_salt("pubgen_chart_archetype").selected_text(cur_type_name).width(200.0).show_ui(ui, |ui| {
                    for (ct, name) in [
                        (ChartType::BarClustered, "Vertical Clustered Bar"),
                        (ChartType::BarStacked, "Vertical Stacked Bar"),
                        (ChartType::HorizontalBar, "Horizontal Bar (Ranked)"),
                        (ChartType::Line, "Line Chart (Markers)"),
                        (ChartType::Area, "Area Chart (Filled)"),
                        (ChartType::Donut, "Donut / Ring Chart"),
                        (ChartType::PopulationPyramid, "Population Pyramid (Piramida Penduduk)"),
                        (ChartType::Gauge, "Gauge / Speedometer Chart"),
                        (ChartType::Map, "Map / Choropleth (Peta Tematik)"),
                    ] {
                        if ui.selectable_label(dialog.chart_type == ct, name).clicked() {
                            dialog.select_chart_type(ct);
                        }
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new(crate::i18n::tr(&lang, "Title (ID):")).size(11.0).color(t.text));
                ui.add_sized([ui.available_width() - 8.0, 42.0], egui::TextEdit::multiline(&mut dialog.title_id).desired_rows(2));
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new(crate::i18n::tr(&lang, "Title (EN):")).size(11.0).color(t.text));
                ui.add_sized([ui.available_width() - 8.0, 22.0], egui::TextEdit::singleline(&mut dialog.title_en));
            });

            ui.separator();

            // Section 2: Data Range & Columns
            ui.label(RichText::new(crate::i18n::tr(&lang, "Data Columns & Rows")).strong().size(11.5).color(t.text_strong));
            ui.horizontal(|ui| {
                ui.label(RichText::new(crate::i18n::tr(&lang, "Category Col:")).size(11.0).color(t.text));
                ui.add_sized([40.0, 22.0], egui::TextEdit::singleline(&mut dialog.category_col));

                ui.add_space(10.0);
                ui.label(RichText::new(crate::i18n::tr(&lang, "Rows:")).size(11.0).color(t.text));
                ui.add(egui::DragValue::new(&mut dialog.data_start_row).range(1..=1000).prefix("Start: "));
                ui.add(egui::DragValue::new(&mut dialog.data_end_row).range(1..=1000).prefix("End: "));
            });

            // Series 1
            ui.horizontal(|ui| {
                ui.label(RichText::new("Series 1:").strong().size(11.0).color(t.text));
                ui.label(RichText::new("Col:").size(10.5).color(t.text_dim));
                ui.add_sized([30.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_1_col));
                ui.label(RichText::new("Label (ID):").size(10.5).color(t.text_dim));
                ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_1_label_id));
                ui.label(RichText::new("Label (EN):").size(10.5).color(t.text_dim));
                ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_1_label_en));
            });

            // Series 2 & 3 (hidden or customized for specialized archetypes)
            if dialog.chart_type == ChartType::Donut || dialog.chart_type == ChartType::Gauge {
                ui.label(
                    RichText::new(crate::i18n::tr(&lang, "ℹ Donut and Gauge charts only fit 1 category and 1 data series."))
                        .size(10.5)
                        .color(t.text_dim),
                );
            } else if dialog.chart_type == ChartType::Map {
                ui.label(
                    RichText::new(crate::i18n::tr(
                        &lang,
                        "ℹ Map charts visualize 1 data series (value column) mapped to geographic regions (nmdesa).",
                    ))
                    .size(10.5)
                    .color(t.text_dim),
                );

                let stock_maps = super::stock_maps::list_stock_map_charts();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(crate::i18n::tr(&lang, "Stock Map:")).size(11.0).color(t.text));
                    let cur_map_label = if dialog.map_stock_idx == 999 {
                        "Custom GeoJSON...".to_string()
                    } else if let Some(m) = stock_maps.get(dialog.map_stock_idx) {
                        m.display_name.clone()
                    } else {
                        stock_maps.first().map(|m| m.display_name.clone()).unwrap_or_else(|| "Pangkalan (Desa)".to_string())
                    };

                    craft_ui::ComboBox::from_id_salt("pubgen_chart_map_stock").selected_text(cur_map_label).width(220.0).show_ui(ui, |ui| {
                        for (idx, m) in stock_maps.iter().enumerate() {
                            ui.selectable_value(&mut dialog.map_stock_idx, idx, &m.display_name);
                        }
                        ui.separator();
                        ui.selectable_value(&mut dialog.map_stock_idx, 999, "Custom GeoJSON...");
                    });
                });

                if dialog.map_stock_idx == 999 {
                    ui.horizontal(|ui| {
                        let w = (ui.available_width() - 80.0).max(120.0);
                        ui.add_sized([w, 20.0], egui::TextEdit::singleline(&mut dialog.map_custom_path).hint_text("Path to .geojson file"));
                    });
                }

                ui.horizontal(|ui| {
                    ui.label(RichText::new(crate::i18n::tr(&lang, "Color Theme:")).size(11.0).color(t.text));
                    let cur_pal_name =
                        super::palette::MAP_COLOR_PRESETS.get(dialog.map_palette_idx).map(|(n, _, _)| *n).unwrap_or("BPS Blue (Default)");
                    craft_ui::ComboBox::from_id_salt("pubgen_chart_map_palette").selected_text(cur_pal_name).width(180.0).show_ui(ui, |ui| {
                        for (idx, (pname, _, _)) in super::palette::MAP_COLOR_PRESETS.iter().enumerate() {
                            ui.selectable_value(&mut dialog.map_palette_idx, idx, *pname);
                        }
                    });
                });

                if ui.button("Load Sample Village CSV (Pangkalan)").clicked() {
                    dialog.load_village_sample();
                }
            } else if dialog.chart_type == ChartType::PopulationPyramid {
                // Series 2 (Female)
                ui.horizontal(|ui| {
                    ui.checkbox(&mut dialog.series_2_enabled, "Series 2 (Female):");
                    if dialog.series_2_enabled {
                        ui.label(RichText::new("Col:").size(10.5).color(t.text_dim));
                        ui.add_sized([30.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_2_col));
                        ui.label(RichText::new("Label (ID):").size(10.5).color(t.text_dim));
                        ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_2_label_id));
                        ui.label(RichText::new("Label (EN):").size(10.5).color(t.text_dim));
                        ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_2_label_en));
                    }
                });
                ui.label(
                    RichText::new(crate::i18n::tr(
                        &lang,
                        "ℹ Population pyramid fits 2 data series: Series 1 = Male (Laki-laki), Series 2 = Female (Perempuan).",
                    ))
                    .size(10.5)
                    .color(t.text_dim),
                );
            } else {
                // Series 2
                ui.horizontal(|ui| {
                    ui.checkbox(&mut dialog.series_2_enabled, "Series 2:");
                    if dialog.series_2_enabled {
                        ui.label(RichText::new("Col:").size(10.5).color(t.text_dim));
                        ui.add_sized([30.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_2_col));
                        ui.label(RichText::new("Label (ID):").size(10.5).color(t.text_dim));
                        ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_2_label_id));
                        ui.label(RichText::new("Label (EN):").size(10.5).color(t.text_dim));
                        ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_2_label_en));
                    }
                });

                // Series 3
                ui.horizontal(|ui| {
                    ui.checkbox(&mut dialog.series_3_enabled, "Series 3:");
                    if dialog.series_3_enabled {
                        ui.label(RichText::new("Col:").size(10.5).color(t.text_dim));
                        ui.add_sized([30.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_3_col));
                        ui.label(RichText::new("Label (ID):").size(10.5).color(t.text_dim));
                        ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_3_label_id));
                        ui.label(RichText::new("Label (EN):").size(10.5).color(t.text_dim));
                        ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut dialog.series_3_label_en));
                    }
                });
            }

            ui.separator();

            // Section 3: Value Formatting & Appearance Options
            ui.label(RichText::new(crate::i18n::tr(&lang, "Formatting & Display Options")).strong().size(11.5).color(t.text_strong));
            ui.horizontal(|ui| {
                ui.label(RichText::new(crate::i18n::tr(&lang, "Value Format:")).size(11.0).color(t.text));
                let fmt_str = match dialog.value_format {
                    ValueFormat::Number => "Number (1.234,5)",
                    ValueFormat::Percent => "Percentage (%)",
                    ValueFormat::CurrencyIdr => "Currency IDR (Rp)",
                };
                craft_ui::ComboBox::from_id_salt("pubgen_chart_fmt").selected_text(fmt_str).width(160.0).show_ui(ui, |ui| {
                    if ui.selectable_label(dialog.value_format == ValueFormat::Number, "Number (1.234,5)").clicked() {
                        dialog.value_format = ValueFormat::Number;
                    }
                    if ui.selectable_label(dialog.value_format == ValueFormat::Percent, "Percentage (%)").clicked() {
                        dialog.value_format = ValueFormat::Percent;
                    }
                    if ui.selectable_label(dialog.value_format == ValueFormat::CurrencyIdr, "Currency IDR (Rp)").clicked() {
                        dialog.value_format = ValueFormat::CurrencyIdr;
                    }
                });

                ui.add_space(8.0);
                ui.label(RichText::new("Max Override:").size(10.5).color(t.text_dim));
                ui.add_sized([50.0, 20.0], egui::TextEdit::singleline(&mut dialog.max_value_override).hint_text("Auto"));
            });

            ui.horizontal(|ui| {
                ui.checkbox(&mut dialog.show_value_labels, crate::i18n::tr(&lang, "Show Value Labels"));
                ui.checkbox(&mut dialog.show_legend, crate::i18n::tr(&lang, "Show Legend"));
                ui.checkbox(&mut dialog.show_grid, crate::i18n::tr(&lang, "Show Gridlines"));
            });

            ui.separator();

            // Section 4: Tabular Data Editor / Preview
            ui.horizontal(|ui| {
                ui.label(RichText::new(crate::i18n::tr(&lang, "Data Table (CSV / TSV):")).strong().size(11.5).color(t.text_strong));
                if ui.small_button(crate::i18n::tr(&lang, "Reset Sample Data")).clicked() {
                    if dialog.chart_type == ChartType::Map {
                        dialog.load_village_sample();
                    } else {
                        dialog.raw_data = DEFAULT_SAMPLE_CSV.to_string();
                        dialog.data_start_row = 2;
                        dialog.data_end_row = 6;
                    }
                }
                if ui.small_button(crate::i18n::tr(&lang, "Auto-Detect Columns")).clicked() {
                    dialog.auto_detect_columns();
                }
            });

            ui.add(egui::TextEdit::multiline(&mut dialog.raw_data).code_editor().desired_rows(6).desired_width(f32::INFINITY));

            if let Some(active) = app.session.active() {
                let total_sp = active.doc.spreads.len();
                if total_sp > 1 {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(crate::i18n::tr(&lang, "Target Spread:")).size(11.0).color(t.text));
                        ui.add(egui::DragValue::new(&mut dialog.target_spread).range(0..=total_sp.saturating_sub(1)));
                    });
                }
            }

            // Section 5: Status Banner
            if let Some((ref msg, is_success)) = dialog.status {
                let col = if is_success { egui::Color32::from_rgb(46, 125, 50) } else { egui::Color32::from_rgb(211, 47, 47) };
                ui.label(RichText::new(msg).size(11.0).color(col));
            }

            ui.add_space(8.0);

            // Footer Buttons
            ui.horizontal(|ui| {
                let btn_label =
                    if is_editing { crate::i18n::tr(&lang, "💾 Update Vector Chart") } else { crate::i18n::tr(&lang, "➕ Insert Vector Chart") };

                let insert_btn = ui.add(egui::Button::new(RichText::new(btn_label).strong().color(egui::Color32::WHITE)).fill(t.accent));
                if insert_btn.clicked() {
                    apply = true;
                }

                if ui.button(crate::i18n::tr(&lang, "Close")).clicked() {
                    close = true;
                }
            });
        });
    });

    if apply {
        if let Err(err) = dialog.apply_chart(app, st.bilingual_output) {
            dialog.status = Some((err, false));
        } else {
            close = true;
        }
    }

    if open && !close {
        st.chart_dialog = Some(dialog);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_map_chart_auto_detect_columns_and_build_spec() {
        let mut dlg = ChartDialog::default();
        dlg.chart_type = ChartType::Map;
        dlg.raw_data = DEFAULT_VILLAGE_SAMPLE_CSV.to_string();

        dlg.auto_detect_columns();

        assert_eq!(dlg.category_col, "A"); // nmdesa
        assert_eq!(dlg.series_1_col, "D"); // value
        assert_eq!(dlg.title_id, "Grafik 01.01 Indeks Pembangunan");
        assert_eq!(dlg.title_en, "Chart 01.01 Development Index");
        assert_eq!(dlg.data_start_row, 2);
        assert_eq!(dlg.data_end_row, 9);

        let spec = dlg.build_spec();
        assert_eq!(spec.chart_type, ChartType::Map);
        assert_eq!(spec.category_col, "A");
        assert_eq!(spec.series[0].col, "D");
        assert!(spec.geojson_path.is_some());
    }
    use super::*;

    #[test]
    fn test_map_defaults_to_pangkalan_stock_map_and_absolute_values() {
        let maps = super::super::stock_maps::list_stock_map_charts();
        let pangkalan = maps.iter().position(|m| m.id == "PANGKALAN").expect("Pangkalan stock map exists");

        let mut dlg = ChartDialog::default();
        assert_eq!(dlg.map_stock_idx, pangkalan, "default stock map must match the bundled Pangkalan sample");

        dlg.select_chart_type(ChartType::Map);
        assert_eq!(dlg.value_format, ValueFormat::Number, "Map charts must default to absolute number formatting");

        dlg.load_village_sample();
        assert_eq!(dlg.map_stock_idx, pangkalan);

        let spec = dlg.build_spec();
        let path = spec.geojson_path.expect("map path");
        assert!(path.to_uppercase().contains("PANGKALAN"), "unexpected map path {path}");
    }

    #[test]
    fn test_map_chart_unknown_geojson_reports_error() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let mut dlg = ChartDialog::default();
        dlg.select_chart_type(ChartType::Map);
        dlg.load_village_sample();
        dlg.map_stock_idx = 999;
        dlg.map_custom_path = "Z:/pubgen-missing/NOPE.geojson".into();
        dlg.insert_on_new_page = true;

        let err = dlg.apply_chart(&mut app, true).expect_err("missing GeoJSON must fail loudly");
        let lower = err.to_lowercase();
        assert!(lower.contains("geojson") || lower.contains("not found"), "unexpected error: {err}");
        let active = app.session.active().unwrap();
        assert!(active.doc.spreads[0].items.is_empty(), "no frames may be emitted when the map cannot load");
        assert_eq!(active.doc.page_count(), 1, "a failed map must not leave an empty chart page behind");
    }

    #[test]
    fn test_editing_chart_clears_the_whole_previous_chart() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let mut dlg = ChartDialog::new(&app, false);
        dlg.select_chart_type(ChartType::Map);
        dlg.load_village_sample();
        dlg.insert_on_new_page = false;
        dlg.apply_chart(&mut app, true).unwrap();

        let top_level_after_first = app.session.active().unwrap().doc.spreads[0].items.len();
        assert_eq!(top_level_after_first, 1, "chart frames and title must be grouped into one container");

        let mut edit = ChartDialog::new(&app, true);
        assert!(edit.editing_item_id.is_some(), "edit dialog must find the existing chart container");
        edit.apply_chart(&mut app, true).unwrap();

        let active = app.session.active().unwrap();
        let title_count = active.doc.stories.values().filter(|s| s.text.contains("Indeks Pembangunan Desa")).count();
        assert_eq!(title_count, 1, "editing must not leave a duplicate chart title");
        assert_eq!(active.doc.spreads[0].items.len(), 1, "editing must replace the chart container, not add another");
    }

    #[test]
    fn test_parse_sheet_data_handles_quoted_delimiters() {
        let mut dlg = ChartDialog::default();
        dlg.raw_data = "nmdesa;title;title_en;value\n\"DESA A;B\";\"Grafik 01.01; Kota\";\"Chart 01.01; City\";\"1.234,5\"\n".to_string();
        let sheet = dlg.parse_sheet_data();
        assert_eq!(sheet.grid.len(), 2);
        assert_eq!(sheet.grid[1][0], "DESA A;B");
        assert_eq!(sheet.grid[1][1], "Grafik 01.01; Kota");

        dlg.auto_detect_columns();
        assert_eq!(dlg.title_id, "Grafik 01.01; Kota");
        assert_eq!(dlg.title_en, "Chart 01.01; City");
    }

    #[test]
    fn test_chart_dialog_defaults_and_spec_building() {
        let dlg = ChartDialog::default();
        let spec = dlg.build_spec();
        assert_eq!(spec.chart_type, ChartType::BarClustered);
        assert_eq!(spec.title_id, "Grafik 01.01\nPertumbuhan Ekonomi Menurut Sektor");
        assert_eq!(spec.category_col, "A");
        assert_eq!(spec.series.len(), 2);
        assert_eq!(spec.data_start_row, 2);
        assert_eq!(spec.data_end_row, 6);

        let sheet = dlg.parse_sheet_data();
        assert_eq!(sheet.grid.len(), 6);
        assert_eq!(sheet.grid[0], vec!["Tahun", "Pertanian", "Industri", "Jasa"]);
        assert_eq!(sheet.grid[1][0], "2021");
    }

    #[test]
    fn test_chart_dialog_apply_inserts_vector_chart_on_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let mut dlg = ChartDialog::default();
        dlg.insert_on_new_page = false;
        let res = dlg.apply_chart(&mut app, true);
        assert!(res.is_ok(), "apply_chart failed: {:?}", res);
        let count = res.unwrap();
        assert!(count > 0, "must emit vector objects");

        let active = app.session.active().unwrap();
        assert!(!active.doc.spreads[0].items.is_empty(), "Spread must contain chart items");
    }

    #[test]
    fn test_chart_dialog_apply_map_chart() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let mut dlg = ChartDialog::default();
        dlg.chart_type = ChartType::Map;
        dlg.raw_data = DEFAULT_VILLAGE_SAMPLE_CSV.to_string();
        dlg.auto_detect_columns();
        dlg.insert_on_new_page = false;

        let res = dlg.apply_chart(&mut app, true);
        assert!(res.is_ok(), "apply_chart for Map failed: {:?}", res);
        let count = res.unwrap();
        assert!(count > 0, "must emit vector map objects");

        let active = app.session.active().unwrap();
        assert!(!active.doc.spreads[0].items.is_empty(), "Spread must contain map chart items");
    }

    #[test]
    fn test_map_chart_with_semicolon_delimited_csv() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let mut dlg = ChartDialog::default();
        dlg.chart_type = ChartType::Map;
        dlg.raw_data = "nmdesa;title;title_en;value\nSUKATANI;Grafik 03.02 Penduduk;Figure 03.02 Population;19561\nMEKARMAYA;Grafik 03.02 Penduduk;Figure 03.02 Population;22014".to_string();
        dlg.auto_detect_columns();
        assert_eq!(dlg.category_col, "A");
        assert_eq!(dlg.series_1_col, "D");
        assert_eq!(dlg.title_id, "Grafik 03.02 Penduduk");
        assert_eq!(dlg.title_en, "Figure 03.02 Population");
        dlg.insert_on_new_page = false;

        let res = dlg.apply_chart(&mut app, true);
        assert!(res.is_ok(), "apply_chart with semicolon failed: {:?}", res);
    }

    #[test]
    fn test_chart_dialog_editing_replaces_existing_chart() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let mut dlg = ChartDialog::default();
        dlg.insert_on_new_page = false;
        let _ = dlg.apply_chart(&mut app, true).unwrap();

        let initial_items_count = app.session.active().unwrap().doc.spreads[0].items.len();
        assert!(initial_items_count > 0);

        // Edit chart
        dlg.title_id = "Grafik 01.01 Updated Title".into();
        let res = dlg.apply_chart(&mut app, true);
        assert!(res.is_ok());

        let active = app.session.active().unwrap();
        let title_exists = active.doc.stories.values().any(|s| s.text.contains("Updated Title"));
        assert!(title_exists, "Updated chart title must exist in document story");
    }

    #[test]
    fn test_map_chart_new_page_manual_flow() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        // Mimic real UI: ChartDialog::new (insert_on_new_page=true, SelectedPage), sample CSV, Pangkalan stock map
        let mut dlg = ChartDialog::new(&app, false);
        dlg.chart_type = ChartType::Map;
        dlg.raw_data = DEFAULT_VILLAGE_SAMPLE_CSV.to_string();
        dlg.auto_detect_columns();
        let maps = super::super::stock_maps::list_stock_map_charts();
        let pangkalan = maps.iter().position(|m| m.id == "PANGKALAN").expect("Pangkalan stock map exists");
        dlg.map_stock_idx = pangkalan;
        assert!(dlg.insert_on_new_page);

        let before = app.session.active().unwrap().doc.page_count();
        let res = dlg.apply_chart(&mut app, true);
        assert!(res.is_ok(), "apply_chart failed: {:?}", res);
        let count = res.unwrap();
        assert!(count > 0, "must emit vector map objects");

        let active = app.session.active().unwrap();
        let after = active.doc.page_count();
        assert!(after > before, "page must be added (before={before} after={after})");
        let (si, _) = active.doc.page_loc(before).expect("new page loc");
        assert!(!active.doc.spreads[si].items.is_empty(), "new page spread must contain map chart items");
    }

    #[test]
    fn test_chart_dialog_ui_command_registered() {
        let exists = crate::menus::UI_COMMANDS.iter().any(|(id, ..)| *id == "pubgen.chartDialog");
        assert!(exists, "pubgen.chartDialog must be registered in UI_COMMANDS");
    }

    #[test]
    fn test_import_from_table_page_smart_recognition() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        // 1. Create Title frame on page 0
        app.run(
            "frame.create",
            json!({
                "spread": 0,
                "rect": [36.0, 36.0, 400.0, 60.0],
                "content": "text",
                "text": "Tabel 01.01 Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025",
                "caret": false
            }),
        )
        .unwrap();

        // 2. Insert Table with column numbers and section header
        let matrix = vec![
            vec!["Desa/Kelurahan".into(), "Luas Area (km2)".into(), "Persentase terhadap Luas Kecamatan".into()],
            vec!["(1)".into(), "(2)".into(), "(3)".into()],
            vec!["Desa".into(), "".into(), "".into()],
            vec!["Warungbambu".into(), "1.65".into(), "21.32".into()],
            vec!["Kondangjaya".into(), "2.44".into(), "8.84".into()],
            vec!["Jumlah".into(), "4.09".into(), "30.16".into()],
        ];
        let st = PubgenState { card2_placement: 1, card2_first_row_header: true, ..Default::default() };
        crate::dock::pubgen::execute_insert_table_matrix(&mut app, &st, Some(&matrix), Some(0)).unwrap();

        // 3. Test import_from_table_page on page 1 (1-based)
        let mut dlg = ChartDialog::default();
        let res = dlg.import_from_table_page(&app, 1);
        assert!(res.is_ok(), "import_from_table_page failed: {:?}", res);

        assert!(dlg.title_id.starts_with("Grafik 01.01"), "Title ID must start with Grafik: {}", dlg.title_id);
        assert_eq!(dlg.category_col, "A");
        assert_eq!(dlg.series_1_col, "B");
        assert!(dlg.series_1_label_id.contains("Luas Area"), "Series 1 label: {}", dlg.series_1_label_id);
        assert!(dlg.series_2_enabled, "Series 2 must be enabled");
        assert_eq!(dlg.series_2_col, "C");
        assert!(dlg.series_2_label_id.contains("Persentase"), "Series 2 label: {}", dlg.series_2_label_id);
        assert!(!dlg.series_3_enabled, "Series 3 must not be enabled");

        // Row 1 is headers, Row 2 is Warungbambu, Row 3 is Kondangjaya. Row 4 is Jumlah.
        // Data rows = 2 (Warungbambu and Kondangjaya), so data_end_row = 3, excluding Jumlah.
        assert_eq!(dlg.data_start_row, 2);
        assert_eq!(dlg.data_end_row, 3);
        assert_eq!(dlg.value_format, ValueFormat::Number);

        // Section header "Desa" without numbers must have been skipped
        assert!(!dlg.raw_data.contains("\nDesa,"), "Must not include section header row with no numbers");
        assert!(dlg.raw_data.contains("Warungbambu,1.65,21.32"));
        assert!(dlg.raw_data.contains("Jumlah,4.09,30.16"));

        // 4. Verify applying chart with the imported data succeeds
        let apply_res = dlg.apply_chart(&mut app, false);
        assert!(apply_res.is_ok(), "apply_chart with imported data must succeed: {:?}", apply_res);

        // 5. Test non-existent page error
        let mut dlg_err = ChartDialog::default();
        let err_res = dlg_err.import_from_table_page(&app, 99);
        assert!(err_res.is_err(), "Must return Err for non-existent page");
    }

    #[test]
    fn test_import_from_table_page_donut_fits_single_series() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        // Title frame on page 0
        app.run(
            "frame.create",
            json!({
                "spread": 0,
                "rect": [36.0, 36.0, 400.0, 60.0],
                "content": "text",
                "text": "Tabel 01.01 Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025",
                "caret": false
            }),
        )
        .unwrap();

        // Insert Table with 2 numeric columns: Luas Area (km2) and Persentase (%)
        let matrix = vec![
            vec!["Desa/Kelurahan".into(), "Luas Area (km2)".into(), "Persentase terhadap Luas Kecamatan".into()],
            vec!["(1)".into(), "(2)".into(), "(3)".into()],
            vec!["Warungbambu".into(), "1.65".into(), "21.32".into()],
            vec!["Kondangjaya".into(), "2.44".into(), "8.84".into()],
            vec!["Margasari".into(), "2.76".into(), "7.17".into()],
            vec!["Tegalsawah".into(), "4.38".into(), "13.48".into()],
            vec!["Jumlah".into(), "29.77".into(), "100".into()],
        ];
        let st = PubgenState { card2_placement: 1, card2_first_row_header: true, ..Default::default() };
        crate::dock::pubgen::execute_insert_table_matrix(&mut app, &st, Some(&matrix), Some(0)).unwrap();

        // User sets chart type to Donut first
        let mut dlg = ChartDialog::default();
        dlg.chart_type = ChartType::Donut;

        // Import from table page 1
        let res = dlg.import_from_table_page(&app, 1);
        assert!(res.is_ok(), "import_from_table_page failed for donut: {:?}", res);

        // Donut must only fit 1 series, and smartly select the percentage column (Col C)
        assert_eq!(dlg.category_col, "A");
        assert_eq!(dlg.series_1_col, "C", "Donut should prefer percentage column for part-to-whole");
        assert!(dlg.series_1_label_id.contains("Persentase"));
        assert!(!dlg.series_2_enabled, "Series 2 must be disabled for Donut chart");
        assert!(!dlg.series_3_enabled, "Series 3 must be disabled for Donut chart");
        assert_eq!(dlg.value_format, ValueFormat::Percent);

        // Verify build_spec produces only 1 series
        let spec = dlg.build_spec();
        assert_eq!(spec.chart_type, ChartType::Donut);
        assert_eq!(spec.series.len(), 1, "Donut spec must contain exactly 1 series");

        // Verify applying donut chart succeeds
        let apply_res = dlg.apply_chart(&mut app, false);
        assert!(apply_res.is_ok(), "apply_chart for donut must succeed: {:?}", apply_res);
    }

    #[test]
    fn test_chart_dialog_inserts_on_new_page_after_table() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"pages": 3})).unwrap();

        let initial_pages = app.session.active().unwrap().doc.page_count();
        assert_eq!(initial_pages, 3);

        let mut dlg = ChartDialog::default();
        dlg.insert_on_new_page = true;
        dlg.import_page = 2; // Insert after page 2 -> new page becomes page 3
        let res = dlg.apply_chart(&mut app, false);
        assert!(res.is_ok(), "apply_chart failed: {:?}", res);

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 4, "A new chart page should have been inserted");
    }

    #[test]
    fn test_chart_dialog_find_available_tables() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"pages": 2})).unwrap();

        let matrix =
            vec![vec!["Tabel 01.01".into(), "Luas Wilayah".into()], vec!["Desa".into(), "Luas".into()], vec!["Sukajaya".into(), "12.5".into()]];
        let st = PubgenState::default();
        crate::dock::pubgen::execute_insert_table_matrix(&mut app, &st, Some(&matrix), Some(0)).unwrap();

        let dlg = ChartDialog::default();
        let tables = dlg.find_available_tables(&app, Some(&st));
        assert!(!tables.is_empty(), "find_available_tables should find the table");
        assert_eq!(tables[0].page_1_based, 1);
        assert!(tables[0].table_num.contains("01.01") || tables[0].label.contains("01.01"));
    }

    #[test]
    fn test_chart_dialog_population_pyramid_and_gauge_capacity() {
        let mut dlg = ChartDialog::default();
        dlg.chart_type = ChartType::PopulationPyramid;
        dlg.clamp_to_chart_type();
        assert!(dlg.series_2_enabled, "Pyramid must enable Series 2 for Female");
        assert!(!dlg.series_3_enabled, "Pyramid must disable Series 3");

        let spec = dlg.build_spec();
        assert_eq!(spec.chart_type, ChartType::PopulationPyramid);
        assert_eq!(spec.series.len(), 2, "Pyramid spec must have exactly 2 series (Male & Female)");

        dlg.chart_type = ChartType::Gauge;
        dlg.clamp_to_chart_type();
        assert!(!dlg.series_2_enabled, "Gauge must disable Series 2");
        assert!(!dlg.series_3_enabled, "Gauge must disable Series 3");

        let spec_gauge = dlg.build_spec();
        assert_eq!(spec_gauge.chart_type, ChartType::Gauge);
        assert_eq!(spec_gauge.series.len(), 1, "Gauge spec must have exactly 1 series");
    }

    #[test]
    fn test_parse_and_split_table_source_titles() {
        // Single line table title
        let (num, id, en) = parse_table_source_title("Tabel 02.03 Jumlah Penduduk Menurut Jenis Kelamin di Karawang Timur, 2025");
        assert_eq!(num, "02.03");
        assert_eq!(id, "Jumlah Penduduk Menurut Jenis Kelamin di Karawang Timur, 2025");
        assert_eq!(en, None);

        // Multiline bilingual table title
        let raw = "Tabel 02.01\nPiramida Penduduk Kecamatan Karawang Timur, 2025\nFigure 02.01\nPopulation Pyramid of Karawang Timur District, 2025";
        let (num2, id2, en2) = parse_table_source_title(raw);
        assert_eq!(num2, "02.01");
        assert_eq!(id2, "Piramida Penduduk Kecamatan Karawang Timur, 2025");
        assert_eq!(en2.as_deref(), Some("Population Pyramid of Karawang Timur District, 2025"));

        // Slash-separated bilingual title
        let raw_slash = "Tabel 02.03 Penduduk / Table 02.03 Population";
        let (num3, id3, en3) = parse_table_source_title(raw_slash);
        assert_eq!(num3, "02.03");
        assert_eq!(id3, "Penduduk");
        assert_eq!(en3.as_deref(), Some("Population"));

        // Split grafik title tests
        let (l1, l2) = split_grafik_title("Grafik 02.03\nJumlah Penduduk Menurut Jenis Kelamin");
        assert_eq!(l1, "Grafik 02.03");
        assert_eq!(l2, "Jumlah Penduduk Menurut Jenis Kelamin");

        let (g1, g2) = split_grafik_title("Grafik 02.03 Jumlah Penduduk");
        assert_eq!(g1, "Grafik 02.03");
        assert_eq!(g2, "Jumlah Penduduk");
    }

    #[test]
    fn test_chart_dialog_inserts_at_selected_page_regardless_of_imported_table_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"pages": 6})).unwrap();

        let mut dlg = ChartDialog::default();
        dlg.insert_on_new_page = true;
        dlg.selected_page = 5; // User selected last page (0-indexed 5 = page 6)
        dlg.import_page = 2; // Data was imported from table on page 2!
        dlg.placement = ChartPlacement::SelectedPage;

        let res = dlg.apply_chart(&mut app, false);
        assert!(res.is_ok(), "apply_chart failed: {:?}", res);

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 7, "A new chart page should have been inserted");

        // Verify the chart was placed on page index 6 (the 7th page, inserted after page 5)
        let (sp_idx, _) = active.doc.page_loc(6).unwrap();
        assert!(!active.doc.spreads[sp_idx].items.is_empty(), "Spread for new page 6 must contain chart items");
    }

    #[test]
    fn test_chart_dialog_inserts_before_back_cover_when_on_last_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"pages": 4})).unwrap();

        let st = PubgenState::default();
        crate::dock::pubgen::covers::execute_generate_back_cover(&mut app, &st).unwrap();

        let active_initial = app.session.active().unwrap();
        let initial_pages = active_initial.doc.page_count();
        let back_initial = crate::dock::pubgen::covers::find_back_cover_page(&active_initial.doc);
        assert_eq!(back_initial, Some(initial_pages - 1), "Back cover must be initial last page");

        let last_idx = initial_pages - 1;
        let mut dlg = ChartDialog::default();
        dlg.insert_on_new_page = true;
        dlg.selected_page = last_idx; // User selected the back cover
        dlg.import_page = 1; // Table from chapter 1
        dlg.placement = ChartPlacement::SelectedPage;

        let res = dlg.apply_chart(&mut app, false);
        assert!(res.is_ok(), "apply_chart failed: {:?}", res);

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), initial_pages + 1, "Document should have 1 additional page");

        // The back cover must remain at the very end
        let back_page = crate::dock::pubgen::covers::find_back_cover_page(&active.doc);
        assert_eq!(back_page, Some(initial_pages), "Back cover must remain at the last page");

        // The chart page must be placed on page index `last_idx` (right before the back cover)
        let (chart_sp, _) = active.doc.page_loc(last_idx).unwrap();
        assert!(!active.doc.spreads[chart_sp].items.is_empty(), "Spread for page before back cover must contain chart items");
    }

    #[test]
    fn test_map_chart_new_page_before_back_cover_flow() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"pages": 4})).unwrap();

        let st = PubgenState::default();
        crate::dock::pubgen::covers::execute_generate_back_cover(&mut app, &st).unwrap();

        let active0 = app.session.active().unwrap();
        let initial_pages = active0.doc.page_count();
        let last_idx = initial_pages - 1;

        let mut dlg = ChartDialog::new(&app, false);
        dlg.selected_page = last_idx;
        dlg.placement = ChartPlacement::SelectedPage;
        dlg.chart_type = ChartType::Map;
        dlg.raw_data = DEFAULT_VILLAGE_SAMPLE_CSV.to_string();
        dlg.auto_detect_columns();
        let maps = super::super::stock_maps::list_stock_map_charts();
        let pangkalan = maps.iter().position(|m| m.id == "PANGKALAN").expect("Pangkalan stock map exists");
        dlg.map_stock_idx = pangkalan;

        let res = dlg.apply_chart(&mut app, true);
        assert!(res.is_ok(), "apply_chart failed: {:?}", res);
        let count = res.unwrap();
        assert!(count > 0, "must emit vector map objects");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), initial_pages + 1);
        let back_page = crate::dock::pubgen::covers::find_back_cover_page(&active.doc);
        assert_eq!(back_page, Some(initial_pages), "Back cover must remain last");
        let (chart_sp, _) = active.doc.page_loc(last_idx).unwrap();
        assert!(!active.doc.spreads[chart_sp].items.is_empty(), "Spread before back cover must contain map items");
    }
}
