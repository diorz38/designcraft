//! Vector chart form options popup window for Pubgen.
//!
//! Provides interactive UI to create a new statistical vector chart or edit
//! an existing chart directly on the active document spread.

use egui::{RichText, Ui};
use serde_json::json;

use crate::DesignApp;
use super::layout::compute_layout;
use super::spec::{ChartSpec, ChartType, SeriesSpec, ValueFormat, extract_chart_data};
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

        if edit {
            // Check if any selected item or spread item is a chart
            if let Some(spread) = active.doc.spreads.get(cur_spread) {
                // If user selected an item, check that first
                let target_item = spread
                    .items
                    .iter()
                    .find(|it| active.selection.items.contains(&it.id))
                    .or_else(|| {
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
            let row: Vec<String> = trimmed
                .split(sep)
                .map(|cell| cell.trim().trim_matches('"').trim().to_string())
                .collect();
            grid.push(row);
        }

        ExcelSheetData {
            name: "ChartData".into(),
            grid,
            merges: Vec::new(),
        }
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

        let (categories, series_values) = extract_chart_data(&sheet_data, &spec)
            .map_err(|e| format!("Failed to extract chart data: {e}"))?;

        let (margin, spread_count) = {
            let active = app.session.active().ok_or("No active document open")?;
            let sp_count = active.doc.spreads.len();
            let spread_idx = self.target_spread.min(sp_count.saturating_sub(1));
            let spread = active.doc.spreads.get(spread_idx).ok_or("Invalid spread index")?;
            let page = spread.pages.first();
            let rect = if let Some(pg) = page {
                let m = pg.margin_rect();
                designcraft_geom::Rect {
                    x0: m.x0,
                    y0: m.y0 + 20.0,
                    x1: m.x1,
                    y1: m.y1 - 10.0,
                }
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
            let title_line = if bilingual && !self.title_en.is_empty() {
                format!("{} / {}", self.title_id, self.title_en)
            } else {
                self.title_id.clone()
            };
            let title_rect = [margin.x0, margin.y0 - 24.0, margin.x1, margin.y0 - 2.0];
            let _ = super::emitter::emit_text(app, target_sp, title_rect, &title_line, 10.0, true, "center", Some("#1A1A1A"));
        }

        self.status = Some((format!("Chart successfully rendered ({} vector objects emitted)", count), true));
        Ok(count as u64)
    }
}

/// Render the Chart Options dialog window in egui.
pub(crate) fn chart_dialog_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    let Some(mut dialog) = st.chart_dialog.take() else {
        return;
    };

    let mut open = true;
    let mut close = false;
    let mut apply = false;

    let lang = app.ui.language.clone();
    let is_editing = dialog.editing_item_id.is_some();
    let window_title = if is_editing {
        crate::i18n::tr(&lang, "Edit Vector Chart (Pubgen)")
    } else {
        crate::i18n::tr(&lang, "Add Vector Chart (Pubgen)")
    };

    egui::Window::new(RichText::new(window_title).strong().size(13.5).color(t.text_strong))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(540.0)
        .default_height(580.0)
        .show(ui.ctx(), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 5.0);

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
                    craft_ui::ComboBox::from_id_salt("pubgen_chart_archetype")
                        .selected_text(cur_type_name)
                        .width(200.0)
                        .show_ui(ui, |ui| {
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
                    craft_ui::ComboBox::from_id_salt("pubgen_chart_fmt")
                        .selected_text(fmt_str)
                        .width(160.0)
                        .show_ui(ui, |ui| {
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

                ui.add(
                    egui::TextEdit::multiline(&mut dialog.raw_data)
                        .code_editor()
                        .desired_rows(6)
                        .desired_width(f32::INFINITY)
                );

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
}

