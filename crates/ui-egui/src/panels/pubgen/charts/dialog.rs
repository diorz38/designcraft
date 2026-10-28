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

const DEFAULT_SAMPLE_CSV: &str = "\
Tahun,Pertanian,Industri,Jasa
2021,3.8,4.2,5.1
2022,4.1,4.5,5.6
2023,3.9,4.8,5.9
2024,4.3,5.1,6.2
2025,4.5,5.4,6.5";

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
    /// Editable raw CSV/TSV tabular data.
    pub raw_data: String,
    /// Target spread index where the chart will be placed.
    pub target_spread: usize,
    /// 1-based page number to import table data from.
    pub import_page: usize,
    /// If editing an existing chart on the spread, store its item ID to replace it.
    pub editing_item_id: Option<u64>,
    /// Status message (text, is_success).
    pub status: Option<(String, bool)>,
}

impl Default for ChartDialog {
    fn default() -> Self {
        Self {
            chart_type: ChartType::BarClustered,
            title_id: "Grafik 01.01 Pertumbuhan Ekonomi Menurut Sektor".into(),
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
            raw_data: DEFAULT_SAMPLE_CSV.into(),
            target_spread: 0,
            import_page: 1,
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

        let cur_page = crate::canvas::current_page(app).unwrap_or(0);
        let cur_spread = active.doc.page_loc(cur_page).map_or(0, |(si, _)| si);
        dlg.target_spread = cur_spread;
        dlg.import_page = cur_page.saturating_add(1);

        if edit {
            // Check if any selected item or spread item is a chart
            if let Some(spread) = active.doc.spreads.get(cur_spread) {
                // If user selected an item, check that first
                let target_item = spread.items.iter().find(|it| active.selection.items.contains(&it.id)).or_else(|| {
                    // Otherwise, look for an item with a Grafik/Figure title
                    spread.items.iter().find(|it| {
                        if let designcraft_doc::Content::Text(tf) = &it.content
                            && let Some(story) = active.doc.story(tf.story)
                        {
                            let t = story.text.trim();
                            return t.starts_with("Grafik") || t.starts_with("Figure");
                        }
                        false
                    })
                });

                if let Some(it) = target_item {
                    dlg.editing_item_id = Some(it.id.0);
                    if let designcraft_doc::Content::Text(tf) = &it.content
                        && let Some(story) = active.doc.story(tf.story)
                    {
                        let full = story.text.trim();
                        if let Some((id_part, en_part)) = full.split_once(" / ") {
                            dlg.title_id = id_part.trim().to_string();
                            dlg.title_en = en_part.trim().to_string();
                        } else {
                            dlg.title_id = full.to_string();
                        }
                    }
                }
            }
        }

        dlg
    }

    /// Parse the raw CSV/TSV data into `ExcelSheetData`.
    pub fn parse_sheet_data(&self) -> ExcelSheetData {
        let mut grid = Vec::new();
        for line in self.raw_data.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Auto-detect delimiter: comma, tab, or semicolon
            let sep = if trimmed.contains('\t') {
                '\t'
            } else if trimmed.contains(';') {
                ';'
            } else {
                ','
            };
            let row: Vec<String> = trimmed.split(sep).map(|cell| cell.trim().trim_matches('"').trim().to_string()).collect();
            grid.push(row);
        }

        ExcelSheetData { name: "ChartData".into(), grid, merges: Vec::new() }
    }

    /// Build the `ChartSpec` from the current dialog parameters.
    pub fn build_spec(&self) -> ChartSpec {
        let mut series = Vec::new();
        series.push(SeriesSpec {
            col: self.series_1_col.trim().to_uppercase(),
            label_id: self.series_1_label_id.trim().to_string(),
            label_en: self.series_1_label_en.trim().to_string(),
        });

        if self.series_2_enabled && !self.series_2_col.trim().is_empty() {
            series.push(SeriesSpec {
                col: self.series_2_col.trim().to_uppercase(),
                label_id: self.series_2_label_id.trim().to_string(),
                label_en: self.series_2_label_en.trim().to_string(),
            });
        }

        if self.series_3_enabled && !self.series_3_col.trim().is_empty() {
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
        }
    }

    /// Apply or insert the chart on the active document spread.
    pub fn apply_chart(&mut self, app: &mut DesignApp, bilingual: bool) -> Result<u64, String> {
        let sheet_data = self.parse_sheet_data();
        let spec = self.build_spec();

        let (categories, series_values) = extract_chart_data(&sheet_data, &spec).map_err(|e| format!("Failed to extract chart data: {e}"))?;

        let (margin, spread_count) = {
            let active = app.session.active().ok_or("No active document open")?;
            let sp_count = active.doc.spreads.len();
            let spread_idx = self.target_spread.min(sp_count.saturating_sub(1));
            let spread = active.doc.spreads.get(spread_idx).ok_or("Invalid spread index")?;
            let page = spread.pages.first();
            let rect = if let Some(pg) = page {
                let m = pg.margin_rect();
                designcraft_geom::Rect { x0: m.x0, y0: m.y0 + 20.0, x1: m.x1, y1: m.y1 - 10.0 }
            } else {
                designcraft_geom::Rect { x0: 36.0, y0: 56.0, x1: 559.0, y1: 780.0 }
            };
            (rect, sp_count)
        };

        if spread_count == 0 {
            return Err("Document contains no spreads".into());
        }

        let target_sp = self.target_spread.min(spread_count.saturating_sub(1));

        // If editing an existing chart item, clear the old one first
        if let Some(old_id) = self.editing_item_id {
            let _ = app.run("edit.clear", json!({"ids": [old_id]}));
        }

        let layout = compute_layout(&spec, &categories, &series_values, margin, bilingual);
        let count = super::emitter::emit_chart(app, target_sp, &spec, &layout, &[], bilingual)?;

        if !self.title_id.is_empty() {
            let title_line =
                if bilingual && !self.title_en.is_empty() { format!("{} / {}", self.title_id, self.title_en) } else { self.title_id.clone() };
            let title_rect = [margin.x0, margin.y0 - 24.0, margin.x1, margin.y0 - 2.0];
            let _ = super::emitter::emit_text(app, target_sp, title_rect, &title_line, 10.0, true, "center", Some("#1A1A1A"));
        }

        self.status = Some((format!("Chart successfully rendered ({} vector objects emitted)", count), true));
        Ok(count as u64)
    }

    /// Read table from the specified 1-based page number and auto-fill dialog fields.
    pub fn import_from_table_page(&mut self, app: &DesignApp, page_1_based: usize) -> Result<(), String> {
        let page_idx = page_1_based.saturating_sub(1);
        let target = super::super::ai::detect_table_page_target(app, page_idx).ok_or_else(|| format!("No table found on page {}", page_1_based))?;

        let active = app.session.active().ok_or("No active document open")?;
        let story = active.doc.story(target.table_sid).ok_or("Table story not found")?;
        let table = story.tables.get(&target.table_id).ok_or("Table not found")?;

        // 1. Title recognition from title frame
        if let Some(tfid) = target.title_fid
            && let Some(item) = active.doc.item(designcraft_doc::ItemId(tfid))
            && let Some(tf) = item.text_frame()
            && let Some(tstory) = active.doc.story(tf.story)
        {
            let full_title = tstory.text.trim();
            let (id_raw, en_raw) = if let Some((p1, p2)) = full_title.split_once('\n') {
                (p1.trim(), Some(p2.trim()))
            } else if let Some((p1, p2)) = full_title.split_once(" / ") {
                (p1.trim(), Some(p2.trim()))
            } else {
                (full_title, None)
            };

            let to_grafik = |s: &str| -> String {
                let trimmed = s.trim();
                if let Some(rest) = trimmed.strip_prefix("Tabel ") {
                    format!("Grafik {rest}")
                } else if let Some(rest) = trimmed.strip_prefix("tabel ") {
                    format!("Grafik {rest}")
                } else if let Some(rest) = trimmed.strip_prefix("Table ") {
                    format!("Grafik {rest}")
                } else {
                    trimmed.to_string()
                }
            };

            let to_figure = |s: &str| -> String {
                let trimmed = s.trim();
                if let Some(rest) = trimmed.strip_prefix("Table ") {
                    format!("Figure {rest}")
                } else if let Some(rest) = trimmed.strip_prefix("table ") {
                    format!("Figure {rest}")
                } else if let Some(rest) = trimmed.strip_prefix("Tabel ") {
                    format!("Figure {rest}")
                } else {
                    trimmed.to_string()
                }
            };

            self.title_id = to_grafik(id_raw);
            if let Some(en_part) = en_raw {
                self.title_en = to_figure(en_part);
            } else if let (clean, Some(prefix)) = super::super::ai::extract_table_title_parts(id_raw) {
                let num_part = prefix.split_whitespace().nth(1).unwrap_or("");
                if !num_part.is_empty() {
                    self.title_en = format!("Figure {num_part} {clean}");
                }
            }
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

        // 9. Populate columns and series
        self.category_col = "A".into();
        let col1 = numeric_cols[0];
        self.series_1_col = index_to_col_letter(col1);
        self.series_1_label_id = col_headers[col1].0.clone();
        self.series_1_label_en = col_headers[col1].1.clone();

        if numeric_cols.len() >= 2 {
            let col2 = numeric_cols[1];
            self.series_2_enabled = true;
            self.series_2_col = index_to_col_letter(col2);
            self.series_2_label_id = col_headers[col2].0.clone();
            self.series_2_label_en = col_headers[col2].1.clone();
        } else {
            self.series_2_enabled = false;
        }

        if numeric_cols.len() >= 3 {
            let col3 = numeric_cols[2];
            self.series_3_enabled = true;
            self.series_3_col = index_to_col_letter(col3);
            self.series_3_label_id = col_headers[col3].0.clone();
            self.series_3_label_en = col_headers[col3].1.clone();
        } else {
            self.series_3_enabled = false;
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

        self.status = Some((
            format!(
                "✓ Imported table from page {} ({} rows, {} series columns: {})",
                page_1_based,
                valid_data_rows.len(),
                numeric_cols.len(),
                numeric_cols.iter().map(|&c| index_to_col_letter(c)).collect::<Vec<_>>().join(", ")
            ),
            true,
        ));

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

    let lang = app.ui.language.clone();
    let is_editing = dialog.editing_item_id.is_some();
    let window_title =
        if is_editing { crate::i18n::tr(&lang, "Edit Vector Chart (Pubgen)") } else { crate::i18n::tr(&lang, "Add Vector Chart (Pubgen)") };

    let available_h = (ctx.content_rect().height() - 90.0).max(200.0);
    let scroll_h = 600.0f32.min(available_h);

    egui::Window::new(RichText::new(window_title).size(15.0))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(540.0)
        .min_width(540.0)
        .max_width(540.0)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().max_width(540.0).max_height(scroll_h).auto_shrink([false, true]).show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 5.0);

                // Section 0: Import from Table Page (Smart Auto-fill)
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("📊").size(13.0));
                        ui.label(RichText::new(crate::i18n::tr(&lang, "Import from Table Page:")).strong().size(11.5).color(t.text_strong));
                        let max_pages = app.session.active().map(|s| s.doc.page_count()).unwrap_or(1).max(1);
                        ui.label(RichText::new(crate::i18n::tr(&lang, "Page:")).size(11.0).color(t.text));
                        ui.add(egui::DragValue::new(&mut dialog.import_page).range(1..=max_pages));

                        if ui.button(RichText::new(crate::i18n::tr(&lang, "📥 Read & Auto-fill")).strong()).clicked() {
                            if let Err(e) = dialog.import_from_table_page(app, dialog.import_page) {
                                dialog.status = Some((e, false));
                            }
                        }
                    });
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
                    };
                    craft_ui::ComboBox::from_id_salt("pubgen_chart_archetype").selected_text(cur_type_name).width(200.0).show_ui(ui, |ui| {
                        for (ct, name) in [
                            (ChartType::BarClustered, "Vertical Clustered Bar"),
                            (ChartType::BarStacked, "Vertical Stacked Bar"),
                            (ChartType::HorizontalBar, "Horizontal Bar (Ranked)"),
                            (ChartType::Line, "Line Chart (Markers)"),
                            (ChartType::Area, "Area Chart (Filled)"),
                            (ChartType::Donut, "Donut / Ring Chart"),
                        ] {
                            if ui.selectable_label(dialog.chart_type == ct, name).clicked() {
                                dialog.chart_type = ct;
                            }
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new(crate::i18n::tr(&lang, "Title (ID):")).size(11.0).color(t.text));
                    ui.add_sized([ui.available_width() - 8.0, 22.0], egui::TextEdit::singleline(&mut dialog.title_id));
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
                        dialog.raw_data = DEFAULT_SAMPLE_CSV.to_string();
                        dialog.data_start_row = 2;
                        dialog.data_end_row = 6;
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
                    let btn_label = if is_editing {
                        crate::i18n::tr(&lang, "💾 Update Vector Chart")
                    } else {
                        crate::i18n::tr(&lang, "➕ Insert Vector Chart")
                    };

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
    use super::*;

    #[test]
    fn test_chart_dialog_defaults_and_spec_building() {
        let dlg = ChartDialog::default();
        let spec = dlg.build_spec();
        assert_eq!(spec.chart_type, ChartType::BarClustered);
        assert_eq!(spec.title_id, "Grafik 01.01 Pertumbuhan Ekonomi Menurut Sektor");
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
        let res = dlg.apply_chart(&mut app, true);
        assert!(res.is_ok(), "apply_chart failed: {:?}", res);
        let count = res.unwrap();
        assert!(count > 0, "must emit vector objects");

        let active = app.session.active().unwrap();
        assert!(!active.doc.spreads[0].items.is_empty(), "Spread must contain chart items");
    }

    #[test]
    fn test_chart_dialog_editing_replaces_existing_chart() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let mut dlg = ChartDialog::default();
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
}
