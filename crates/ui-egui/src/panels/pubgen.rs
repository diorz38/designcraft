//! Pubgen panel: automated CSV document publishing and table insertion.
//!
//! Provides two card tools matching the InDesign-style Properties layout:
//! 1. "Page from CSV": Ingests CSV (columns `type`, `number`, `title`) and synthesizes
//!    multi-page layouts using geometric templates.
//! 2. "Table from CSV": Ingests tabular CSV matrix data and constructs native styled tables.

use egui::{Color32, RichText, Sense, Ui, vec2};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::DesignApp;
use crate::theme::Tokens;
use crate::widgets::{divider, full_width, outline_button, rule, section, selection_band};

#[path = "pubgen_excel.rs"]
pub mod excel;

const PANEL_ID: &str = "pubgen_panel_state";

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
    #[serde(default)]
    pub toc_layout_style: usize,
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
            card2_path: String::new(),
            card2_matrix: Vec::new(),
            card2_sheet_tables: Vec::new(),
            card2_sheet_idx: 0,
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_table_footer: false,
            card2_row_conditions: true,
            card2_style: 0,
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
            toc_layout_style: 0,
            card3_renumber_start: 1,
            card3_renumber_style: 0,
            generated_csv_files: Vec::new(),
            generated_zip_name: String::new(),
            generated_zip_bytes: Vec::new(),
        }
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
    selection_band(ui, "Pubgen");

    let state_id = egui::Id::new(PANEL_ID);
    let mut st: PubgenState = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();

    // Card 1: Page from CSV
    card1_ui(app, ui, &mut st, &t);

    divider(ui);
    rule(ui);

    // Card 2: Table from CSV
    card2_ui(app, ui, &mut st, &t);

    divider(ui);
    rule(ui);

    // Card 3: Quick Actions
    card3_quick_actions_ui(app, ui, &mut st, &t);

    ui.data_mut(|d| d.insert_temp(state_id, st));
}

fn card1_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Page from CSV"), true);

    // File picker row
    ui.horizontal(|ui| {
        if outline_button(ui, crate::i18n::tr(&app.ui.language, "Browse…"), 68.0).clicked() {
            if let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("dataMerge")) {
                load_card1_csv(st, &path);
            }
        }
        let (file_label, color) = if st.card1_path.is_empty() {
            (crate::i18n::tr(&app.ui.language, "No file selected").to_string(), t.text_dim)
        } else {
            let name = std::path::Path::new(&st.card1_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| st.card1_path.clone());
            (format!("{name} ({} records)", st.card1_records.len()), t.text)
        };
        ui.add(egui::Label::new(RichText::new(file_label).size(11.0).color(color)).truncate());
    });

    // Fallback path text input
    ui.horizontal(|ui| {
        ui.label(RichText::new("Path:").size(10.5).color(t.text_dim));
        let avail = (ui.available_width() - 4.0).max(60.0);
        let resp = ui.add(
            egui::TextEdit::singleline(&mut st.card1_path)
                .hint_text("C:\\path\\to\\data.csv")
                .desired_width(avail),
        );
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !st.card1_path.is_empty() {
            load_card1_csv(st, &st.card1_path.clone());
        }
    });

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
        ];
        let cur = presets.get(st.card1_preset).copied().unwrap_or("1 Row · 3 Columns");
        let combo_w = (ui.available_width() - 4.0).clamp(80.0, 180.0);
        egui::ComboBox::from_id_salt("pubgen_card1_preset")
            .selected_text(cur)
            .width(combo_w)
            .show_ui(ui, |ui| {
                for (i, p) in presets.iter().enumerate() {
                    ui.selectable_value(&mut st.card1_preset, i, *p);
                }
            });
    });

    // Every preset tags its title paragraph with the TOC style; a TOC entry's text is the
    // paragraph's text, so "prefix" makes the entry read "Tabel 01.01. Jarak …".
    ui.horizontal(|ui| {
        ui.label(RichText::new("TOC style:").size(11.0).color(t.text));
        let w = (ui.available_width() - 4.0).clamp(80.0, 180.0);
        ui.add_sized([w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.toc_style));
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
            egui::ComboBox::from_id_salt("pubgen_card2_sheet_selector")
                .selected_text(cur_name)
                .width(combo_w)
                .show_ui(ui, |ui| {
                    for (idx, p) in st.card2_sheet_tables.iter().enumerate() {
                        let name = std::path::Path::new(p)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| p.clone());
                        let is_sel = idx == st.card2_sheet_idx;
                        if ui.selectable_label(is_sel, name).clicked() {
                            new_selection = Some(idx);
                        }
                    }
                });
        });
        if let Some(idx) = new_selection {
            if let Some(p) = st.card2_sheet_tables.get(idx).cloned() {
                st.card2_sheet_idx = idx;
                st.card2_path = p.clone();
                load_card2_csv(st, &p);
            }
        }
    }

    // File picker row
    ui.horizontal(|ui| {
        if outline_button(ui, crate::i18n::tr(&app.ui.language, "Browse…"), 68.0).clicked() {
            if let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("dataMerge")) {
                load_card2_csv(st, &path);
            }
        }
        let (file_label, color) = if st.card2_path.is_empty() {
            (crate::i18n::tr(&app.ui.language, "No file selected").to_string(), t.text_dim)
        } else {
            let name = std::path::Path::new(&st.card2_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| st.card2_path.clone());
            let rows = st.card2_matrix.len();
            let cols = st.card2_matrix.first().map(Vec::len).unwrap_or(0);
            (format!("{name} ({}×{} grid)", rows, cols), t.text)
        };
        ui.add(egui::Label::new(RichText::new(file_label).size(11.0).color(color)).truncate());
    });

    // Fallback path text input
    ui.horizontal(|ui| {
        ui.label(RichText::new("Path:").size(10.5).color(t.text_dim));
        let avail = (ui.available_width() - 4.0).max(60.0);
        let resp = ui.add(
            egui::TextEdit::singleline(&mut st.card2_path)
                .hint_text("C:\\path\\to\\table.csv")
                .desired_width(avail),
        );
        if resp.lost_focus() && !st.card2_path.is_empty() {
            load_card2_csv(st, &st.card2_path.clone());
        }
    });

    ui.checkbox(&mut st.card2_first_row_header, "First row is header");
    ui.checkbox(&mut st.card2_col_numbers, "Add column numbers (1), (2)…");
    ui.checkbox(&mut st.card2_table_footer, "Table footer (last row)");
    ui.checkbox(&mut st.card2_row_conditions, "Condition rules (Jumlah, Tahun, Kab, Kec…)");

    if !st.card2_matrix.is_empty() {
        if let Some(d) = st.card2_delim {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Delimiter:").size(10.5).color(t.text_dim));
                badge(ui, t, "delim", delimiter_name(d));
            });
        }
    }

    if st.card2_first_row_header {
        let merge_count = st.card2_matrix.first().map(|r| r.iter().filter(|c| c.contains("--")).count()).unwrap_or(0);
        if merge_count > 1 {
            let num_stacks = st
                .card2_matrix
                .first()
                .map(|r| r.iter().map(|c| c.splitn(3, "--").count()).max().unwrap_or(1).clamp(1, 3))
                .unwrap_or(1);
            let label = if num_stacks > 1 {
                format!("✓ Auto-detected {num_stacks}-stack header columns (\"--\")")
            } else {
                "✓ Auto-detected merged header columns (\"--\")".to_string()
            };
            ui.add(egui::Label::new(RichText::new(label).size(10.5).color(t.accent)).wrap());
        }
    }

    if let Some(fn_text) = detect_footnote_row(&st.card2_matrix) {
        let display_fn = if fn_text.len() > 38 {
            format!("{}…", &fn_text[..38])
        } else {
            fn_text
        };
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
        egui::ComboBox::from_id_salt("pubgen_card2_style")
            .selected_text(cur)
            .width(combo_w)
            .show_ui(ui, |ui| {
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

fn card3_quick_actions_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Quick Actions"), true);

    let facing = app.session.active().map(|s| s.doc.settings.facing_pages).unwrap_or(true);
    let cur_page = app
        .session
        .active()
        .and_then(|s| s.selection.page)
        .or_else(|| crate::canvas::current_page(app))
        .unwrap_or_else(|| target_page_index(app));
    let cur_label = app.session.page_label(cur_page);

    // Button 1: switch Facing Pages
    // When currently Facing (Double-sided), button offers switching to Single-sided.
    // When currently Single-sided, button offers switching to Double-sided.
    let facing_text = if facing {
        crate::i18n::tr(&app.ui.language, "Single-sided")
    } else {
        crate::i18n::tr(&app.ui.language, "Double-sided")
    };
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
        let resp_facing = outline_button(ui, &facing_text, hw).on_hover_text(tip_facing);
        if resp_facing.clicked() {
            let next_facing = !facing;
            let _ = app.run("layout.documentSetup", json!({"facingPages": next_facing}));
            st.card3_status = Some((
                if next_facing {
                    "Switched to Double-sided (Facing Pages)".into()
                } else {
                    "Switched to Single-sided layout".into()
                },
                true,
            ));
        }

        let tip_page = format!("Insert a blank page after Page {cur_label}");
        let resp_page = outline_button(ui, &add_page_text, hw).on_hover_text(tip_page);
        if resp_page.clicked() {
            let _ = app.run("layout.pages.insert", json!({"after": cur_page, "count": 1}));
            crate::canvas::go_to_page(app, cur_page + 1);
            let next_label = app.session.page_label(cur_page + 1);
            st.card3_status = Some((format!("Added blank page after Page {cur_label} (new Page {next_label})"), true));
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
        let w = (ui.available_width() - 4.0).clamp(80.0, 180.0);
        ui.add_sized([w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.toc_style));
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

    let hw_toc = (ui.available_width() - 4.0) * 0.5;
    ui.horizontal(|ui| {
        let add_toc_text = crate::i18n::tr(&app.ui.language, "Add TOC");
        let tip_toc = format!("Insert a table of contents on Page {cur_label}");
        if outline_button(ui, add_toc_text, hw_toc).on_hover_text(tip_toc).clicked() {
            let res = execute_add_toc(app, st);
            st.card3_status = Some(match res {
                Ok(msg) => (msg, true),
                Err(e) => (e, false),
            });
        }

        let rebuild_toc_text = crate::i18n::tr(&app.ui.language, "Rebuild TOC");
        let tip_rebuild = "Rebuild and re-number existing Table of Contents across the document";
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
        egui::ComboBox::from_id_salt("pubgen_renumber_style")
            .selected_text(cur_style)
            .width(80.0)
            .show_ui(ui, |ui| {
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
        ui.add_sized(
            [w, crate::widgets::FIELD_H],
            egui::TextEdit::singleline(&mut st.rtf_path).hint_text("Path to .rtf file…"),
        );
        if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
            if let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place")) {
                st.rtf_path = path;
            }
        }
    });
    let tip_rtf = format!("Import RTF document to Page {cur_label} (auto-flows to new pages if needed)");
    if outline_button(ui, "Page from RTF", full_width(ui)).on_hover_text(tip_rtf).clicked() {
        if st.rtf_path.trim().is_empty() {
            if let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place")) {
                st.rtf_path = path;
            }
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
        ui.add_sized(
            [w, crate::widgets::FIELD_H],
            egui::TextEdit::singleline(&mut st.image_path).hint_text("Path to image file…"),
        );
        if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
            if let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place")) {
                st.image_path = path;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("As:").size(11.0).color(t.text));
        ui.radio_value(&mut st.image_as_background, false, "Image object").on_hover_text(
            "Create an image object fitted inside the page margins of the selected page",
        );
        ui.radio_value(&mut st.image_as_background, true, "Page background").on_hover_text(
            "Fit image proportionally to full page, keeping aspect ratio (sent to the back, behind all items)",
        );
    });
    let tip_img = if st.image_as_background {
        format!("Apply image as page background to Page {cur_label} (proportional, sent to back)")
    } else {
        format!("Place image fitted inside the page margins of Page {cur_label}")
    };
    if outline_button(ui, "Page from Image", full_width(ui)).on_hover_text(tip_img).clicked() {
        if st.image_path.trim().is_empty() {
            if let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place")) {
                st.image_path = path;
            }
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
    let tip_excel = "Convert a multi-sheet Excel file (.xlsx) into 1 catalog CSV for Page from CSV + individual table CSVs (with auto-merged headers)";
    let resp_excel = outline_button(ui, "Excel to CSV Data…", full_width(ui)).on_hover_text(tip_excel);
    if resp_excel.clicked() {
        if let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("dataMerge")) {
            let res = execute_excel_to_csvs(st, &path);
            st.card3_status = Some(match res {
                Ok(msg) => (msg, true),
                Err(e) => (e, false),
            });
        }
    }

    let tip_zip = "Download all generated catalog and table CSVs packaged in a single .zip archive";
    let zip_label = if !st.generated_zip_name.is_empty() {
        format!("Download {}…", st.generated_zip_name)
    } else {
        "Download CSVs as ZIP…".to_string()
    };
    if outline_button(ui, &zip_label, full_width(ui)).on_hover_text(tip_zip).clicked() {
        let res = execute_download_csv_zip(app, st);
        st.card3_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(e) => (e, false),
        });
    }

    let mode_str = if facing { "Double-sided" } else { "Single-sided" };
    ui.add(
        egui::Label::new(
            RichText::new(format!("Mode: {mode_str} · Selected: Page {cur_label}"))
                .size(10.5)
                .color(t.text_dim),
        )
        .truncate(),
    );

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
    if !first.is_empty() && rest_empty {
        Some(first.to_string())
    } else {
        None
    }
}

/// Check if a cell starts with one of the summary/break keywords:
/// Jumlah, Tahun, Kab, Kabupaten, Kec, Kecamatan (or Total).
pub fn matches_pubgen_row_condition(text: &str) -> bool {
    let trimmed = text
        .trim()
        .trim_start_matches(|c: char| c == '*' || c == '-' || c == '•' || c == '–' || c == '—')
        .trim();
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
}

pub fn is_index_or_empty(text: &str) -> bool {
    let t = text.trim();
    t.is_empty()
        || t.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | ')' | '(' | '-' | '#'))
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

    // Every preset tags its title (or the combined TOC line) with the TOC paragraph style, so the
    // Quick Actions "Add TOC" lists one entry per generated page.
    ensure_para_style(app, &st.toc_style);

    for row in slice {
        // 1. Insert Page
        app.run("layout.pages.insert", json!({"count": 1}))?;

        // 2. Query Page Margins
        let (margin, spread_idx) = {
            let active = app.session.active().ok_or("No active document")?;
            let doc = &active.doc;
            let sp_idx = doc.spreads.len().saturating_sub(1);
            let spread = doc.spreads.get(sp_idx).ok_or("Spread missing")?;
            let page = spread.pages.last().ok_or("Page missing")?;
            (page.margin_rect(), sp_idx)
        };

        let type_val = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let num_val = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let title_val = row.get(st.card1_title_col.min(2)).map(String::as_str).unwrap_or("").trim();

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

                let row_y0 = margin.y0 + 20.0;
                let row_y1 = margin.y0 + 80.0;

                if !type_val.is_empty() {
                    let _ = app.run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": [c1_x0, row_y0, c1_x1, row_y1],
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
                            "rect": [c2_x0, row_y0, c2_x1, row_y1],
                            "content": "text",
                            "text": num_val,
                            "caret": false
                        }),
                    );
                }
                if !title_val.is_empty() {
                    frame_with_style(app, spread_idx, [c3_x0, row_y0, c3_x1, row_y1], title_val, &st.toc_style);
                }
            }
            2 => {
                // Editorial Chapter (Stacked)
                if !type_val.is_empty() {
                    let _ = app.run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": [margin.x0, margin.y0 + 20.0, margin.x1, margin.y0 + 42.0],
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
                            "rect": [margin.x0, margin.y0 + 46.0, margin.x1, margin.y0 + 106.0],
                            "content": "text",
                            "text": num_val,
                            "caret": false
                        }),
                    );
                }
                if !title_val.is_empty() {
                    frame_with_style(app, spread_idx, [margin.x0, margin.y0 + 112.0, margin.x1, margin.y0 + 220.0], title_val, &st.toc_style);
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
                    frame_with_style(app, spread_idx, [margin.x0, margin.y0 + 40.0, margin.x1 - 90.0, margin.y0 + 110.0], title_val, &st.toc_style);
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
                    frame_with_style(app, spread_idx, [margin.x0, margin.y0 + 20.0, margin.x1, margin.y0 + 60.0], &line, &st.toc_style);
                }
            }
            _ => {
                // 0 or Default: 1 Row · 3 Columns (Col 1 & 2 fit tightly to content, Col 3 takes rest of width with space at end)
                let gutter = 8.0;
                let end_space = 20.0;
                let row_y0 = margin.y0 + 20.0;
                let row_y1 = margin.y0 + 80.0;
                let mut cur_x = margin.x0;

                // Col 1: Type (Fit tightly to content, right-aligned)
                if !type_val.is_empty() {
                    let formatted_type = type_val.to_uppercase();
                    let c1_w = estimate_text_width(&formatted_type, true);
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
                    }
                }

                // Col 2: Number (Fit tightly to content)
                if !num_val.is_empty() {
                    let c2_w = estimate_text_width(num_val, false);
                    let c2_x0 = cur_x;
                    let c2_x1 = (c2_x0 + c2_w).min(margin.x1 - 60.0);
                    cur_x = c2_x1 + gutter;

                    let _ = app.run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": [c2_x0, row_y0, c2_x1, row_y1],
                            "content": "text",
                            "text": num_val,
                            "caret": false
                        }),
                    );
                }

                // Col 3: Title (Takes rest of width with space in the end)
                if !title_val.is_empty() {
                    let c3_x0 = cur_x;
                    let max_x1 = (margin.x1 - end_space).max(c3_x0 + 40.0);
                    let c3_x1 = max_x1.min(margin.x1);

                    frame_with_style(app, spread_idx, [c3_x0, row_y0, c3_x1, row_y1], title_val, &st.toc_style);
                }
            }
        }
        // Auto-insert table from 'file' column if enabled
        if st.card1_auto_insert_tables {
            if let Some(file_str) = row.get(3).map(|s| s.trim()).filter(|s| !s.is_empty()) {
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

                if let Some(tbl_file) = resolved {
                    if let Ok(content) = std::fs::read_to_string(&tbl_file) {
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
        }
        generated_count = generated_count.saturating_add(1);
    }

    let tbl_msg = if inserted_tables_count > 0 {
        format!(" (with {inserted_tables_count} tables auto-inserted)")
    } else {
        String::new()
    };
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
            if let Some(loc) = st.doc.find(id) {
                if let designcraft_doc::SpreadRef::Doc(si) = loc.spread {
                    return st.doc.first_page_of_spread(si);
                }
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

/// Create a text frame and tag its paragraph with the TOC paragraph style, so `toc.generate`
/// lists the frame's text as an entry (one entry per generated page).
fn frame_with_style(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], text: &str, style: &str) -> Option<u64> {
    let res = app
        .run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "text", "text": text, "caret": false}))
        .ok()?;
    let story = res.get("story").and_then(Value::as_u64)?;
    let end = text.chars().count().max(1) as u64;
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    let _ = app.run("style.paragraph.apply", json!({"name": style}));
    Some(story)
}

/// The number of paragraphs the TOC would collect for `style` right now.
#[cfg(test)]
fn toc_entry_count(app: &mut DesignApp, style: &str) -> usize {
    app.session
        .execute("toc.entries", &json!({"entries": [{"style": style, "level": 1}]}))
        .ok()
        .and_then(|v| v.as_array().map(Vec::len))
        .unwrap_or(0)
}

pub fn execute_insert_table(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    execute_insert_table_matrix(app, st, None, None)
}

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
            let last = matrix.last().unwrap();
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
    let footnote_text = if footnotes.is_empty() {
        None
    } else {
        Some(footnotes.join("\n"))
    };

    let rows_count = matrix.len();

    // Auto-detect merged header columns (split by "--", capped at max 3 stacks)
    let col_levels: Vec<Vec<String>> = if st.card2_first_row_header {
        matrix
            .first()
            .map(|row| {
                (0..cols_count)
                    .map(|c| {
                        let val = row.get(c).map(|s| s.trim()).unwrap_or("");
                        if val.contains("--") {
                            val.splitn(3, "--").map(|s| s.trim().to_string()).collect()
                        } else {
                            vec![val.to_string()]
                        }
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
                    let is_start = c == 0
                        || (0..=r).any(|l| {
                            col_levels.get(c.saturating_sub(1)).and_then(|prev| prev.get(l)) != lvls.get(l)
                        });
                    if is_start {
                        if let Some(txt) = lvls.get(r) {
                            header_grid[r][c] = txt.trim().to_string();
                        }
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
                table_text.push_str(cell);
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
                table_text.push_str(cell.trim());
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
                table_text.push_str(cell.trim());
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
            if let Some(fid) = active.selection.text.and_then(|t| t.frame) {
                if let Some(loc) = active.doc.find(fid) {
                    if let designcraft_doc::SpreadRef::Doc(si) = loc.spread {
                        if let Some(spread) = active.doc.spreads.get(si) {
                            if let Some(item) = spread.items.iter().find(|it| it.id == fid) {
                                let b = item.bounds();
                                let fn_y0 = b.y1 + 6.0;
                                let fn_y1 = fn_y0 + 24.0;
                                fn_target = Some((si, [b.x0, fn_y0, b.x1, fn_y1]));
                            }
                        }
                    }
                }
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
            let gap_below_title = 20.0;
            let table_y0 = if title_bottom > margin.y0 {
                title_bottom + gap_below_title
            } else {
                margin.y0 + 20.0
            };
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

    let header_rows_count = if has_merged_headers { num_header_rows } else if st.card2_first_row_header && rows_count > 1 { 1 } else { 0 };
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
    if st.card2_style != 3 {
        if let Some(&style_name) = style_names.get(st.card2_style) {
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
    }

    let table_id = res["table"].as_u64().or_else(|| {
        let act = app.session.active()?;
        let story = act.doc.story(designcraft_doc::StoryId(story_id))?;
        story.tables.keys().copied().next()
    }).ok_or("Failed to find table ID")?;

    app.session
        .edit(|doc, _| {
            if st.card2_style == 3 && !doc.styles.table.iter().any(|s| s.name == "Horizontal Rules") {
                let mut ts = designcraft_doc::TableStyle::default();
                ts.name = "Horizontal Rules".to_string();
                ts.border = Some(designcraft_doc::CellStroke::none());
                doc.styles_mut().table.push(ts);
            }

            let story = doc
                .story_mut(designcraft_doc::StoryId(story_id))
                .ok_or_else(|| designcraft_engine::EngineError::Other("Story not found".to_string()))?;
            let table = story
                .table_mut(table_id)
                .ok_or_else(|| designcraft_engine::EngineError::Other("Table not found".to_string()))?;

            if has_merged_headers {
                // 1. Vertical merges: for columns where leaf row is before (num_header_rows - 1)
                for (c, lvls) in col_levels.iter().enumerate() {
                    let k = lvls.len();
                    if k < num_header_rows {
                        let _ = table.merge(designcraft_doc::CellRange {
                            r0: k.saturating_sub(1),
                            c0: c,
                            r1: num_header_rows - 1,
                            c1: c,
                        });
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
                                let _ = table.merge(designcraft_doc::CellRange {
                                    r0: r,
                                    c0: start,
                                    r1: r,
                                    c1: end,
                                });
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

                    let header_rows = if has_merged_headers { num_header_rows } else if st.card2_first_row_header { 1 } else { 0 };
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
                            if nr > header_rows {
                                if let Some(cell) = table.cell_mut(header_rows, c) {
                                    cell.strokes[0] = rule_thin.clone();
                                }
                            }
                        }

                        // Col numbers row (if checked)
                        if st.card2_col_numbers && nr > header_rows {
                            let col_num_row = header_rows;
                            for c in 0..nc {
                                if let Some(cell) = table.cell_mut(col_num_row, c) {
                                    cell.strokes[2] = rule_thin.clone();
                                }
                                if nr > col_num_row + 1 {
                                    if let Some(cell) = table.cell_mut(col_num_row + 1, c) {
                                        cell.strokes[0] = rule_thin.clone();
                                    }
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
                            if nr > 1 {
                                if let Some(cell) = table.cell_mut(1, c) {
                                    cell.strokes[0] = rule_thin.clone();
                                }
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
                            if footer_row > 0 {
                                if let Some(cell) = table.cell_mut(footer_row - 1, c) {
                                    cell.strokes[2] = rule_thin.clone();
                                }
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
                                matches_pubgen_row_condition(col0)
                                    || (is_index_or_empty(col0) && matches_pubgen_row_condition(col1))
                            };

                            if matches {
                                for c in 0..nc {
                                    // Top border
                                    if let Some(cell) = table.cell_mut(r, c) {
                                        cell.strokes[0] = rule_thin.clone();
                                    }
                                    if r > 0 {
                                        if let Some(cell) = table.cell_mut(r - 1, c) {
                                            cell.strokes[2] = rule_thin.clone();
                                        }
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

            if st.card2_table_footer && table.nrows() > header_rows_count + 1 {
                table.rows.last_mut().map(|r| r.kind = designcraft_doc::RowKind::Footer);
            }

            Ok(())
        })
        .map_err(|e| e.to_string())?;

    let mut actual_fn_target = fn_target;
    if let Some((fid, spread_idx, x0, x1, y0, max_h, margin_y1)) = created_frame_info {
        let fit_h = {
            let active = app.session.active().ok_or("No active document")?;
            let cs = designcraft_compose::compose_story(&active.doc, designcraft_doc::StoryId(story_id), &Default::default());
            let measured_h = cs.frames.first().and_then(|f| {
                f.tables.first().map(|t| t.rect.height()).or(Some(f.content_height))
            });

            if let Some(th) = measured_h {
                (th + 4.0).min(max_h).max(40.0)
            } else {
                (total_rows as f64 * 32.0 + 40.0).min(max_h).max(60.0)
            }
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
        let txt = st.text.trim();
        if txt.is_empty() || txt == title {
            continue;
        }

        if is_plausible_number(txt) {
            num_candidate = Some(txt.to_string());
        } else if is_plausible_type(txt) {
            type_candidate = Some(to_proper_case(txt));
        }
    }

    match (type_candidate, num_candidate) {
        (Some(t), Some(n)) => Some((t, n)),
        (Some(t), None) => Some((t, String::new())),
        (None, Some(n)) => Some((String::new(), n)),
        (None, None) => None,
    }
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
        let candidate = if cur_line.is_empty() {
            word.to_string()
        } else {
            format!("{cur_line} {word}")
        };

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
        let candidate = if cur_line.is_empty() {
            word.to_string()
        } else {
            format!("{cur_line} {word}")
        };

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

/// Detect whether a record represents a Section header (e.g. "Bab 4", "4.3 Perumahan", or "Section 2")
/// which in TablePage layout does not receive a page number or dot leaders.
fn is_section_header(r_type: &str, num_str: &str, title: &str) -> bool {
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

fn enrich_toc_story(
    app: &mut DesignApp,
    sid: designcraft_doc::StoryId,
    st: &PubgenState,
    width: f64,
    hanging_indent: f64,
    toc_page: usize,
) {
    let _ = app.session.edit(|doc, _| {
        let full_text = doc.story(sid).map(|s| s.text.clone()).ok_or(designcraft_doc::DocError::NoStory(sid))?;
        let toc_page_label = doc.page_name(toc_page);
        let self_title = if st.toc_title.trim().is_empty() {
            "Contents"
        } else {
            st.toc_title.trim()
        };

        let is_table_page = st.toc_layout_style == 1;

        let has_table = st.card1_records.iter().any(|r| {
            let t = r.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("");
            t.eq_ignore_ascii_case("table")
        });
        let header_type = if has_table { "Table" } else { "Tabel" };
        let header_page = if header_type == "Table" { "page" } else { "halaman" };
        let header_line = format!("{header_type}\t{header_page}");

        let mut new_lines = Vec::new();
        let mut changed = false;
        let mut used_records: Vec<bool> = vec![false; st.card1_records.len()];

        let lines: Vec<&str> = full_text.split('\n').collect();
        let orig_title_line = lines.first().copied().unwrap_or("");

        if is_table_page {
            if !st.toc_title.trim().is_empty() {
                new_lines.push(st.toc_title.trim().to_string());
                new_lines.push(header_line.clone());
            } else {
                new_lines.push(header_line.clone());
            }
        } else {
            new_lines.push(orig_title_line.to_string());
        }

        for (idx, line) in lines.into_iter().enumerate() {
            if idx == 0 || line.trim().is_empty() {
                continue;
            }

            let line_clean = line.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
            if is_table_page && (line_clean == header_line || line_clean.starts_with("Tabel\t") || line_clean.starts_with("Table\t") || line_clean.starts_with("Grafik\t")) {
                continue;
            }

            let (raw_title, page_part) = match line_clean.split_once('\t') {
                Some((t, p)) => (t.trim(), Some(p.trim())),
                None => (line_clean.trim(), None),
            };

            if raw_title.is_empty() {
                continue;
            }

            // Check if this line is an existing TOC self-entry
            let is_self_entry = raw_title.eq_ignore_ascii_case("contents")
                || raw_title.eq_ignore_ascii_case("table of contents")
                || raw_title.eq_ignore_ascii_case("daftar isi")
                || raw_title.eq_ignore_ascii_case("daftar tabel")
                || raw_title.eq_ignore_ascii_case(self_title);

            if is_self_entry {
                if st.toc_include_self {
                    let wrapped = if st.toc_hanging_indent {
                        wrap_toc_title(self_title, width, hanging_indent, Some(&toc_page_label))
                    } else {
                        self_title.to_string()
                    };
                    new_lines.push(format!("{wrapped}\t{toc_page_label}"));
                } else {
                    changed = true;
                }
                continue;
            }

            let mut matched_type = String::new();
            let mut matched_num = String::new();
            let mut clean_title = raw_title.to_string();
            let mut matched_record = false;

            // 1. Try matching against records in st.card1_records
            for (r_idx, row) in st.card1_records.iter().enumerate() {
                if used_records.get(r_idx).copied().unwrap_or(false) {
                    continue;
                }
                let r_type = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let r_num = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let r_title = row.get(st.card1_title_col.min(2)).map(String::as_str).unwrap_or("").trim();

                if !r_title.is_empty() && (raw_title == r_title || raw_title.ends_with(r_title)) {
                    matched_record = true;
                    matched_type = r_type.to_string();
                    matched_num = r_num.to_string();
                    clean_title = r_title.to_string();
                    if let Some(used) = used_records.get_mut(r_idx) {
                        *used = true;
                    }
                    break;
                }
            }

            // 2. Strip prefix if in raw_title (e.g. "Tabel 4.2.1 Banyaknya...")
            if !matched_record {
                let trimmed_title = raw_title.trim();
                let (det_type, after_type) = if let Some(rest) = trimmed_title.strip_prefix("Tabel ") {
                    ("Tabel", rest.trim())
                } else if let Some(rest) = trimmed_title.strip_prefix("Table ") {
                    ("Table", rest.trim())
                } else if let Some(rest) = trimmed_title.strip_prefix("Grafik ") {
                    ("Grafik", rest.trim())
                } else {
                    ("", trimmed_title)
                };

                if let Some((first_word, rest_of_title)) = after_type.split_once(' ') {
                    if first_word.chars().any(|c| c.is_ascii_digit()) && first_word.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
                        matched_type = det_type.to_string();
                        matched_num = first_word.to_string();
                        clean_title = rest_of_title.trim().to_string();
                    }
                }
            }

            // 3. Fallback: inspect page frames ONLY if no record in st.card1_records matched!
            if matched_num.is_empty() && !matched_record {
                if let Some(p_str) = page_part {
                    if let Some((t_val, n_val)) = find_type_and_number_on_page(doc, p_str, raw_title) {
                        matched_type = t_val;
                        matched_num = n_val;
                    }
                }
            }

            if is_table_page {
                let clean_num = matched_num.trim_end_matches('.');
                let is_sec = is_section_header(&matched_type, clean_num, &clean_title);
                if is_sec {
                    let entry = if !clean_num.is_empty() {
                        format!("{clean_num}\t{clean_title}")
                    } else {
                        clean_title
                    };
                    new_lines.push(entry);
                } else {
                    let wrapped = if st.toc_hanging_indent {
                        wrap_tablepage_title(&clean_title, width, hanging_indent, page_part)
                    } else {
                        clean_title
                    };
                    let entry = if let Some(p_str) = page_part {
                        if !clean_num.is_empty() {
                            format!("{clean_num}\t{wrapped}\t{p_str}")
                        } else {
                            format!("{wrapped}\t{p_str}")
                        }
                    } else {
                        wrapped
                    };
                    new_lines.push(entry);
                }
                changed = true;
            } else {
                // Standard mode
                let mut prefix_opt = None;
                let expected_prefix = [&matched_type, &matched_num]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");

                let starts_with_type = !matched_type.is_empty() && raw_title.to_lowercase().starts_with(&matched_type.to_lowercase());
                if !expected_prefix.is_empty() && !raw_title.starts_with(&expected_prefix) && !starts_with_type {
                    prefix_opt = Some(expected_prefix);
                }

                let title_to_use = if let Some(prefix) = prefix_opt {
                    format!("{prefix} {raw_title}")
                } else {
                    raw_title.to_string()
                };

                let wrapped_title = if st.toc_hanging_indent {
                    wrap_toc_title(&title_to_use, width, hanging_indent, page_part)
                } else {
                    title_to_use
                };

                let original_entry_title = line.split_once('\t').map(|(t, _)| t).unwrap_or(line);
                if wrapped_title != original_entry_title {
                    changed = true;
                }

                if let Some(p_str) = page_part {
                    new_lines.push(format!("{wrapped_title}\t{p_str}"));
                } else {
                    new_lines.push(wrapped_title);
                }
            }
        }

        // If toc_include_self is requested and not yet added, insert right after title/header
        if st.toc_include_self && !new_lines.is_empty() {
            let already_has = new_lines.iter().skip(1).any(|l| {
                let t = l.split_once('\t').map(|(t, _)| t.trim()).unwrap_or(l.trim());
                t.eq_ignore_ascii_case(self_title)
                    || t.eq_ignore_ascii_case("contents")
                    || t.eq_ignore_ascii_case("table of contents")
                    || t.eq_ignore_ascii_case("daftar isi")
                    || t.eq_ignore_ascii_case("daftar tabel")
            });
            if !already_has {
                let wrapped = if st.toc_hanging_indent {
                    wrap_toc_title(self_title, width, hanging_indent, Some(&toc_page_label))
                } else {
                    self_title.to_string()
                };
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

fn ensure_toc_entry_style(app: &mut DesignApp, width: f64, hanging: bool, hanging_indent: f64, layout_style: usize) {
    let _ = app.session.edit(|d, _| {
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
            let (li, fi) = if hanging {
                (Some(hanging_indent), Some(-hanging_indent))
            } else {
                (Some(0.0), Some(0.0))
            };

            let mut found_level1 = false;
            let mut found_sec = false;
            let mut found_hdr = false;

            for style in &mut d.styles_mut().paragraph {
                if style.name == "TOC Level 1" {
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
                    name: "TOC Level 1".to_string(),
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
                    chars: designcraft_doc::CharAttrs {
                        font_style: Some("Bold".into()),
                        size: Some(10.5),
                        ..Default::default()
                    },
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
                    chars: designcraft_doc::CharAttrs {
                        size: Some(10.0),
                        fill_tint: Some(0.6),
                        ..Default::default()
                    },
                    shortcut: String::new(),
                });
            }
        } else {
            let style_name = "TOC Level 1";
            let mut found = false;
            for style in &mut d.styles_mut().paragraph {
                if style.name.starts_with("TOC Level") {
                    found = true;
                    let level_idx: usize = style.name.strip_prefix("TOC Level ").and_then(|s| s.parse().ok()).unwrap_or(1);
                    let base_indent = (level_idx.saturating_sub(1) as f64) * 12.0;
                    let (li, fi) = if hanging {
                        (Some(base_indent + hanging_indent), Some(-hanging_indent))
                    } else {
                        (Some(base_indent), Some(0.0))
                    };
                    style.para.left_indent = li;
                    style.para.first_line_indent = fi;
                    style.para.tabs = Some(vec![tab_stop.clone()]);
                }
            }

            if !found {
                let (left_ind, first_ind) = if hanging {
                    (Some(hanging_indent), Some(-hanging_indent))
                } else {
                    (Some(0.0), Some(0.0))
                };
                let st = d.styles_mut();
                st.paragraph.push(designcraft_doc::ParagraphStyle {
                    name: style_name.to_string(),
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
        let max_w = st.card1_records
            .iter()
            .filter_map(|row| {
                let r_num = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let clean_num = r_num.trim_end_matches('.');
                if clean_num.is_empty() {
                    None
                } else {
                    Some(estimate_text_advance(&format!("{clean_num}  ")))
                }
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
                if prefix.is_empty() {
                    None
                } else {
                    Some(estimate_text_advance(&format!("{prefix} ")))
                }
            })
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(68.0)
    }
}

fn apply_toc_post_process(
    app: &mut DesignApp,
    st: &PubgenState,
    sid: designcraft_doc::StoryId,
    width: f64,
    hanging_indent: f64,
    target_page: usize,
    mut pages_added: u64,
) -> u64 {
    enrich_toc_story(app, sid, st, width, hanging_indent, target_page);

    let _ = app.session.edit(|doc, _| {
        if let Some(story) = doc.story_mut(sid) {
            let ranges = story.para_ranges();
            for (pi, r) in ranges.into_iter().enumerate() {
                let para_text = story.text[r.clone()].trim();
                let style_name = if st.toc_layout_style == 1 {
                    if pi == 0 && !st.toc_title.trim().is_empty() && (para_text == st.toc_title.trim() || para_text.eq_ignore_ascii_case("contents") || para_text.eq_ignore_ascii_case("daftar isi") || para_text.eq_ignore_ascii_case("daftar tabel")) {
                        "TOC Title"
                    } else if para_text.starts_with("Tabel\t") || para_text.starts_with("Table\t") || para_text.starts_with("Grafik\t") {
                        "TOC TablePage Header"
                    } else if para_text.split('\t').count() == 2 {
                        "TOC Section Header"
                    } else {
                        "TOC Level 1"
                    }
                } else {
                    if pi == 0 { "TOC Title" } else { "TOC Level 1" }
                };

                story.format_paras(r.start..r.start, |f| *f = designcraft_doc::ParaFormat {
                    style: style_name.to_string(),
                    ..Default::default()
                });
            }
        }
        Ok(serde_json::Value::Null)
    });

    ensure_toc_entry_style(app, width, st.toc_hanging_indent, hanging_indent, st.toc_layout_style);

    if let Ok(af_res) = app.run("text.autoflow", json!({"story": sid.0, "maxPages": 50})) {
        if let Some(extra) = af_res.get("pagesAdded").and_then(Value::as_u64) {
            pages_added += extra;
        }
    }

    pages_added = sync_toc_continuation_headers(app, st, sid, pages_added);

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

fn sync_toc_continuation_headers(
    app: &mut DesignApp,
    st: &PubgenState,
    sid: designcraft_doc::StoryId,
    mut pages_added: u64,
) -> u64 {
    remove_toc_continuation_headers(app);

    let (header_text, style_name) = if st.toc_layout_style == 1 {
        let detected = app.session.active().and_then(|a| {
            a.doc.story(sid).and_then(|s| {
                s.text.lines().find(|l| {
                    let clean = l.trim();
                    clean.starts_with("Tabel\t") || clean.starts_with("Table\t") || clean.starts_with("Grafik\t")
                }).map(|l| l.trim().to_string())
            })
        });

        let text = detected.unwrap_or_else(|| {
            let has_table = st.card1_records.iter().any(|r| {
                let t = r.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("");
                t.eq_ignore_ascii_case("table")
            });
            let header_type = if has_table { "Table" } else { "Tabel" };
            let header_page = if header_type == "Table" { "page" } else { "halaman" };
            format!("{header_type}\t{header_page}")
        });

        (text, "TOC TablePage Header")
    } else {
        let title = if st.toc_title.trim().is_empty() { "Contents" } else { st.toc_title.trim() };
        let is_id = title.to_lowercase().contains("daftar") || title.to_lowercase().contains("tabel");
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

            let header_rect = designcraft_geom::Rect::new(m.x0, m.y0, m.x1, m.y0 + TOC_CONT_HEADER_H);
            let flow_top = (m.y0 + TOC_CONT_HEADER_H + TOC_CONT_HEADER_GAP).min(m.y1 - 10.0);
            let flow_rect = designcraft_geom::Rect::new(m.x0, flow_top, m.x1, m.y1);

            let layer = active.doc.item(fid).map(|it| it.layer).unwrap_or_else(|| active.doc.default_layer());

            let has_hdr = spread.items.iter().any(|it| {
                it.label == TOC_CONTINUATION_HEADER_LABEL && active.doc.page_of_item(it.id) == Some(pg)
            });

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
                    let para = designcraft_doc::ParaFormat {
                        style: style_name.to_string(),
                        ..Default::default()
                    };
                    if let Ok((hdr_id, _)) = doc.add_text_frame(p.spread_ref, p.header_rect, p.layer, &header_text, para) {
                        if let Some(it) = doc.item_mut(hdr_id) {
                            it.label = TOC_CONTINUATION_HEADER_LABEL.to_string();
                            it.name = "TOC Continuation Header".to_string();
                        }
                    }
                }
                if p.needs_resize {
                    if let Some(it) = doc.item_mut(p.fid) {
                        it.path = designcraft_geom::shapes::rectangle(p.flow_rect);
                        resized_any = true;
                    }
                }
            }
            Ok(serde_json::Value::Null)
        });

        if resized_any {
            if let Ok(af_res) = app.run("text.autoflow", json!({"story": sid.0, "maxPages": 50})) {
                if let Some(extra) = af_res.get("pagesAdded").and_then(Value::as_u64) {
                    pages_added += extra;
                }
            }
        } else {
            break;
        }
    }

    pages_added
}

pub fn execute_add_toc(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let cur_page = target_page_index(app);
    let cur_label = app.session.page_label(cur_page);

    ensure_para_style(app, &st.toc_style);

    let (margin, _spread_idx) = {
        let active = app.session.active().ok_or("No active document")?;
        let (si, pi) = active.doc.page_loc(cur_page).ok_or("Current page location not found")?;
        let spread = active.doc.spreads.get(si).ok_or("Spread missing")?;
        let page = spread.pages.get(pi).ok_or("Page missing")?;
        (page.margin_rect(), si)
    };

    let width = (margin.x1 - margin.x0).max(100.0);
    let detected_prefix_w = compute_detected_prefix_width(st);
    let hanging_indent = if st.toc_layout_style == 1 {
        detected_prefix_w.round().clamp(28.0, 60.0)
    } else {
        detected_prefix_w.round().clamp(48.0, 140.0)
    };

    ensure_toc_entry_style(app, width, st.toc_hanging_indent, hanging_indent, st.toc_layout_style);

    let replace_existing = {
        let active = app.session.active().ok_or("No active document")?;
        if let Some(toc) = &active.doc.toc {
            let first_frame = active.doc.story(toc.story).and_then(|s| s.frames.first().copied());
            let pg = first_frame.and_then(|f| active.doc.page_of_item(f));
            pg == Some(cur_page)
        } else {
            false
        }
    };

    if replace_existing {
        remove_toc_continuation_headers(app);
    }

    let title = if st.toc_title.trim().is_empty() { "Contents" } else { st.toc_title.trim() };

    let res = app.run(
        "toc.generate",
        json!({
            "page": cur_page + 1,
            "title": title,
            "entries": [{
                "style": st.toc_style.as_str(),
                "level": 1,
                "pageNumber": "after",
                "separator": "\t"
            }],
            "replace": replace_existing,
            "autoflow": true
        }),
    )?;

    let count = res.get("entries").and_then(Value::as_u64).unwrap_or(0);
    let pages_added = res.get("pagesAdded").and_then(Value::as_u64).unwrap_or(0);
    let sid_u64 = res.get("story").and_then(Value::as_u64).ok_or("TOC story missing")?;
    let sid = designcraft_doc::StoryId(sid_u64);

    let final_pages_added = apply_toc_post_process(app, st, sid, width, hanging_indent, cur_page, pages_added);

    let cont_msg = if final_pages_added > 0 {
        format!(" across {} pages", final_pages_added + 1)
    } else {
        String::new()
    };

    Ok(format!("Added TOC on Page {cur_label} ({count} entries{cont_msg})"))
}

pub fn execute_rebuild_toc(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let active_opt = app.session.active();
    let has_toc = active_opt.as_ref().and_then(|a| a.doc.toc.as_ref()).is_some();
    if !has_toc {
        return execute_add_toc(app, st);
    }

    remove_toc_continuation_headers(app);

    let (_existing_sid, toc_page, width) = {
        let active = app.session.active().ok_or("No active document")?;
        let toc = active.doc.toc.as_ref().ok_or("No table of contents exists")?;
        let sid = toc.story;
        let story = active.doc.story(sid).ok_or("TOC story missing")?;
        let first_frame = story.frames.first().copied().ok_or("TOC frame missing")?;
        let pg = active.doc.page_of_item(first_frame).unwrap_or(0);
        let (si, pi) = active.doc.page_loc(pg).ok_or("TOC page missing")?;
        let spread = active.doc.spreads.get(si).ok_or("Spread missing")?;
        let page = spread.pages.get(pi).ok_or("Page missing")?;
        let m = page.margin_rect();
        let w = (m.x1 - m.x0).max(100.0);
        (sid, pg, w)
    };

    let toc_label = app.session.page_label(toc_page);
    ensure_para_style(app, &st.toc_style);

    let detected_prefix_w = compute_detected_prefix_width(st);
    let hanging_indent = if st.toc_layout_style == 1 {
        detected_prefix_w.round().clamp(28.0, 60.0)
    } else {
        detected_prefix_w.round().clamp(48.0, 140.0)
    };

    ensure_toc_entry_style(app, width, st.toc_hanging_indent, hanging_indent, st.toc_layout_style);

    let title = if st.toc_title.trim().is_empty() { "Contents" } else { st.toc_title.trim() };

    let res = app.run(
        "toc.generate",
        json!({
            "page": toc_page + 1,
            "title": title,
            "entries": [{
                "style": st.toc_style.as_str(),
                "level": 1,
                "pageNumber": "after",
                "separator": "\t"
            }],
            "replace": true,
            "autoflow": true
        }),
    )?;

    let count = res.get("entries").and_then(Value::as_u64).unwrap_or(0);
    let pages_added = res.get("pagesAdded").and_then(Value::as_u64).unwrap_or(0);
    let sid_u64 = res.get("story").and_then(Value::as_u64).ok_or("TOC story missing")?;
    let sid = designcraft_doc::StoryId(sid_u64);

    let final_pages_added = apply_toc_post_process(app, st, sid, width, hanging_indent, toc_page, pages_added);

    let cont_msg = if final_pages_added > 0 {
        format!(" across {} pages", final_pages_added + 1)
    } else {
        String::new()
    };

    Ok(format!("Rebuilt TOC on Page {toc_label} ({count} entries{cont_msg})"))
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
    let toc_msg = if app.session.active().and_then(|a| a.doc.toc.as_ref()).is_some() {
        if let Ok(msg) = execute_rebuild_toc(app, st) {
            format!(" & {msg}")
        } else {
            String::new()
        }
    } else {
        String::new()
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

    let toc_msg = if app.session.active().and_then(|a| a.doc.toc.as_ref()).is_some() {
        if let Ok(msg) = execute_rebuild_toc(app, st) {
            format!(" & {msg}")
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let page_num = cur_page + 1;
    Ok(format!("Continued numbering on Page {page_num} (now '{new_label}'){toc_msg}"))
}

pub fn execute_page_from_rtf(
    app: &mut DesignApp,
    st: &mut PubgenState,
    path: &str,
) -> Result<String, String> {
    let target_path = if path.trim().is_empty() {
        st.rtf_path.trim().to_string()
    } else {
        path.trim().to_string()
    };
    if target_path.is_empty() {
        return Err("No RTF file path specified".into());
    }
    let p = std::path::Path::new(&target_path);
    if !p.exists() {
        return Err(format!("File not found: {target_path}"));
    }

    let cur_page = app
        .session
        .active()
        .and_then(|s| s.selection.page)
        .or_else(|| crate::canvas::current_page(app))
        .unwrap_or_else(|| target_page_index(app));
    let cur_label = app.session.page_label(cur_page);

    // If a text frame on the selected page is selected, place into that frame.
    // Otherwise clear any other selection so place_text creates a new text frame on cur_page.
    let target_frame = {
        let active = app.session.active().ok_or("No active document")?;
        active.selection.items.first().copied().filter(|&id| {
            active.doc.page_of_item(id) == Some(cur_page)
                && active.doc.item(id).is_some_and(|it| {
                    matches!(
                        it.content,
                        designcraft_doc::Content::Unassigned | designcraft_doc::Content::Text(_)
                    )
                })
        })
    };

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
    let filename = std::path::Path::new(&target_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| target_path.clone());

    let page_info = if pages_added > 0 {
        format!(" across {} pages ({} added)", pages_added + 1, pages_added)
    } else {
        String::new()
    };
    let overset_info = if overset { " (text still overset)" } else { "" };

    Ok(format!("Imported '{filename}' to Page {cur_label}{page_info}{overset_info}"))
}

/// Page from Image: place an image on the selected page either as an image object fitted inside
/// the page margins, or as a full-page background (page bounds, sent to the back).
pub fn execute_page_from_image(
    app: &mut DesignApp,
    st: &mut PubgenState,
    path: &str,
) -> Result<String, String> {
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

    let cur_page = app
        .session
        .active()
        .and_then(|s| s.selection.page)
        .or_else(|| crate::canvas::current_page(app))
        .unwrap_or_else(|| target_page_index(app));
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

    let filename = std::path::Path::new(&target_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| target_path.clone());
    let mode_desc = if st.image_as_background {
        "page background (fitted proportionally to page, sent to back)"
    } else {
        "image object (fitted inside margins)"
    };

    Ok(format!("Placed '{filename}' as {mode_desc} on Page {cur_label}"))
}

pub fn execute_excel_to_csvs(st: &mut PubgenState, path: &str) -> Result<String, String> {
    let pkg = excel::execute_convert_excel(path)?;
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

    let folder_name = std::path::Path::new(&pkg.parent_dir)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or(pkg.parent_dir);
    let count = pkg.table_count;
    let zip_name = pkg.zip_name;
    Ok(format!("Converted {count} sheets → catalog CSV + {count} table CSVs + {zip_name} in {folder_name} (loaded in Page & Table from CSV)"))
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
                    if let Ok(content) = std::fs::read_to_string(p) {
                        if !files.iter().any(|(n, _)| n == &name) {
                            files.push((name, content));
                        }
                    }
                }
            }

            // If card2_path exists and not already included
            if !st.card2_path.is_empty() {
                let p = std::path::Path::new(&st.card2_path);
                if p.is_file() {
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "table.csv".into());
                    if !files.iter().any(|(n, _)| n == &name) {
                        if let Ok(content) = std::fs::read_to_string(p) {
                            files.push((name, content));
                        }
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
                    std::path::Path::new(&st.card1_path)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "pubgen".into())
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

    let zip_name = if st.generated_zip_name.is_empty() {
        "pubgen_csvs.zip".to_string()
    } else {
        st.generated_zip_name.clone()
    };

    // Deliver bytes via Web download or Desktop save dialog
    if let Some(download) = app.services.download.as_mut() {
        download(&zip_name, &st.generated_zip_bytes)
            .map_err(|e| format!("Could not download {zip_name}: {e}"))?;
        Ok(format!("Downloaded {zip_name}"))
    } else if let Some(pick_save) = app.services.pick_save.as_mut() {
        if let Some(save_path) = pick_save(&zip_name) {
            if let Some(w) = app.services.write.as_mut() {
                w(&save_path, &st.generated_zip_bytes)
                    .map_err(|e| format!("Could not save {save_path}: {e}"))?;
            } else {
                std::fs::write(&save_path, &st.generated_zip_bytes)
                    .map_err(|e| format!("Could not save {save_path}: {e}"))?;
            }
            let file_name = std::path::Path::new(&save_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or(save_path);
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
        std::fs::write(&save_path, &st.generated_zip_bytes)
            .map_err(|e| format!("Could not save {}: {e}", save_path.display()))?;
        Ok(format!("Saved {}", save_path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut st1 = PubgenState::default();
        load_card1_csv(&mut st1, path);

        assert_eq!(st1.card1_delim, Some(';'));
        assert_eq!(st1.card1_headers, vec!["type", "number", "title"]);
        assert_eq!(st1.card1_type_col, 0);
        assert_eq!(st1.card1_num_col, 1);
        assert_eq!(st1.card1_title_col, 2);
        assert_eq!(st1.card1_records.len(), 194);
        assert_eq!(
            st1.card1_records[0],
            vec!["Tabel", "01.01.", "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"]
        );

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
            card2_style: 0,
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table should succeed: {:?}", res);
    }

    #[test]
    fn test_execute_insert_table_at_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Generate pages with Card 1 (preset 0: 1 Row · 3 Columns)
        let st_card1 = PubgenState {
            card1_records: vec![
                vec!["Chapter".into(), "1".into(), "Introduction to Layout".into()],
            ],
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
            card2_matrix: vec![
                vec!["Item".into(), "Price".into()],
                vec!["Widget A".into(), "$10".into()],
                vec!["Widget B".into(), "$20".into()],
            ],
            card2_first_row_header: true,
            card2_placement: 2, // At page
            card2_style: 0,
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
        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|st| !st.tables.is_empty())
        }).expect("Table item should exist on spread");

        // In preset 0, title row has row_y1 = margin.y0 + 80.0
        // The table frame should start at margin.y0 + 80.0 + 20.0 = margin.y0 + 100.0
        assert_eq!(table_item.bounds().y0, margin.y0 + 100.0);
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
            card2_style: 0,
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with col numbers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with Horizontal Rules should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok());

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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
            card2_style: 0,     // Basic Grid
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with auto merged headers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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

        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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
            card2_style: 0,     // Basic Grid
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with 3-stack headers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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

        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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

        let rows_normal = vec![
            vec!["Header 1".into(), "Header 2".into()],
            vec!["Data 1".into(), "Data 2".into()],
        ];
        assert_eq!(detect_footnote_row(&rows_normal), None);

        let rows_single_col = vec![
            vec!["Single 1".into()],
            vec!["Single 2".into()],
        ];
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
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with footer and footnote should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        // 1. Verify table exists and has 4 rows (1 header + 2 data + 1 total) - NOT 5 rows!
        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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
        let fn_item = spread.items.iter().find(|it| {
            if it.id == table_item.id {
                return false;
            }
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| s.text.contains("Sources: Karawang Region Government Office"))
        }).expect("Footnote text frame should exist on spread");

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
        load_card2_csv(&mut st, path);
        assert_eq!(st.card2_matrix.len(), 8);
        assert_eq!(detect_footnote_row(&st.card2_matrix), Some("Sources: Karawang Region Government Office".to_string()));

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with sample_3stack_table.csv should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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
        let fn_item = spread.items.iter().find(|it| {
            if it.id == table_item.id {
                return false;
            }
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| s.text.contains("Sources: Karawang Region Government Office"))
        }).expect("Footnote text frame should exist on spread");

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
        load_card2_csv(&mut st_card2, path);

        let res = execute_insert_table(&mut app, &st_card2);
        assert!(res.is_ok(), "insert_table should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.last().unwrap();
        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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

        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).expect("Table item should exist on spread");

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
                vec!["Tahun 2006".into(), "150".into()], // Condition row 3 (intermediate)
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
        let table_item = spread.items.iter().find(|it| {
            it.text_frame()
                .and_then(|tf| active.doc.story(tf.story))
                .is_some_and(|s| !s.tables.is_empty())
        }).unwrap();

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
        let mut lines: Vec<&str> = active
            .doc
            .stories
            .values()
            .filter(|s| s.paras.first().is_some_and(|p| p.style == "TOC Entry"))
            .map(|s| s.text.trim())
            .collect();
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

            // 1st textbox contains: "TABEL" (correct value)
            assert_eq!(s1.text.trim(), "TABEL");
            // 2nd textbox contains: "02.01" (correct value)
            assert_eq!(s2.text.trim(), "02.01");
            // 3rd textbox contains ONLY title, NOT including type and number!
            assert_eq!(
                s3.text.trim(),
                "Jumlah Rukun Warga (RW) dan Rukun Tetangga (RT) Menurut Desa/Kelurahan di Kecamatan Karawang Timur, 2025"
            );
        }

        // When Add TOC is executed, verify the generated TOC entry merges type + number + title:
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc_story = active.doc.story(active.doc.toc.as_ref().unwrap().story).unwrap();
        let clean_text = toc_story.text.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
        assert!(
            clean_text.contains("Tabel 02.01 Jumlah Rukun Warga (RW) dan Rukun Tetangga (RT) Menurut Desa/Kelurahan di Kecamatan Karawang Timur, 2025"),
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
                vec![
                    "Tabel".into(),
                    "01.01.".into(),
                    "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into(),
                ],
                vec![
                    "Tabel".into(),
                    "01.02.".into(),
                    "Keadaan Curah Hujan Tiap Bulan menurut Kecamatan (mm), 2006".into(),
                ],
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
        let toc = active.doc.toc.as_ref().expect("Document TOC should be populated");
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
            card1_records: vec![vec![
                "Tabel".into(),
                "01.01.".into(),
                "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into(),
            ]],
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
            card1_records: vec![vec![
                "Tabel".into(),
                "01.01.".into(),
                "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into(),
            ]],
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
        let mut st_empty = PubgenState {
            card1_records: Vec::new(),
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let res = execute_add_toc(&mut app, &mut st_empty);
        assert!(res.is_ok(), "execute_add_toc with fallback should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.toc.as_ref().unwrap();
        let text = &active.doc.story(toc.story).unwrap().text;

        assert!(
            text.contains("Tabel 01.01. Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"),
            "Fallback should extract type and number from page frames: {text}"
        );
    }

    #[test]
    fn test_execute_add_toc_ignores_footnote_and_source_text_frames() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec![
                "Tabel".into(),
                "01.01.".into(),
                "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into(),
            ]],
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
        let toc = active.doc.toc.as_ref().unwrap();
        let text = &active.doc.story(toc.story).unwrap().text;

        // Must NOT contain footnote text
        assert!(
            !text.to_lowercase().contains("sources"),
            "TOC must not incorporate footnote/source text: {text}"
        );
        assert!(
            text.contains("Tabel 01.01. Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"),
            "TOC must contain clean entry: {text}"
        );
    }

    #[test]
    fn test_execute_add_toc_generates_continuation_pages_when_overflows() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 80 entries - definitely cannot fit in 1 page (typical page holds ~35 entries)
        let records: Vec<Vec<String>> = (1..=80)
            .map(|i| {
                vec![
                    "Tabel".into(),
                    format!("01.{i:02}."),
                    format!("Dataset Observation Record Number {i} in Region"),
                ]
            })
            .collect();

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
        let toc = active.doc.toc.as_ref().unwrap();
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
        assert!(
            wrapped_long.contains(designcraft_doc::story::FORCED_LINE_BREAK),
            "Title should be wrapped with FORCED_LINE_BREAK"
        );
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
            vec![
                "Grafik".into(),
                "4.6.".into(),
                "Banyaknya Tenaga Medis/Dokter Menurut Keahlian di Kabupaten Karawang, 2006".into(),
            ],
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
        let toc = active.doc.toc.as_ref().unwrap();
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
        assert_eq!(
            st.card1_records[0].get(0..3).unwrap(),
            &["Tabel", "01.01.", "Jarak Antar Kecamatan"]
        );
        assert_eq!(st.card1_records[0].get(3).map(String::as_str), Some("data1.csv"));
        assert_eq!(
            st.card1_records[1].get(0..3).unwrap(),
            &["Grafik", "02.01.", "Distribusi Penduduk"]
        );
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

        let mut st = PubgenState::default();
        let res = execute_excel_to_csvs(&mut st, sample_xlsx.to_str().unwrap());
        assert!(res.is_ok(), "Conversion should succeed: {res:?}");

        // Card 1 must have catalog loaded
        assert!(st.card1_path.ends_with("BAB_02_2025_catalog.csv"));
        assert_eq!(st.card1_records.len(), 3);
        assert!(st.card1_headers.contains(&"file".to_string()));

        // Card 2 must have first table loaded and sheet table list populated
        assert!(st.card2_path.ends_with("02_01.csv"));
        assert_eq!(st.card2_sheet_tables.len(), 3);
        assert_eq!(st.card2_sheet_idx, 0);
        assert!(!st.card2_matrix.is_empty());

        // ZIP metadata and bytes must be cached
        assert!(!st.generated_zip_bytes.is_empty(), "ZIP archive bytes should be cached in state");
        assert!(st.generated_zip_name.ends_with(".zip"), "ZIP archive name should end with .zip");
        assert_eq!(st.generated_csv_files.len(), 4, "Catalog + 3 tables in generated_csv_files");
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
            generated_csv_files: vec![
                ("cat.csv".into(), "type,number,title\nT,1,Test\n".into()),
                ("tbl.csv".into(), "A,B\n1,2\n".into()),
            ],
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
            let toc = active.doc.toc.as_ref().unwrap();
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
            let toc = active.doc.toc.as_ref().unwrap();
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
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Data Demografi".into()],
            ],
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
        let toc = active.doc.toc.as_ref().unwrap();
        let text = active.doc.story(toc.story).unwrap().text.clone();
        assert!(text.contains("Data Demografi\t1"), "TOC entry should reflect renumbered page 1: {text}");
    }

    #[test]
    fn test_toc_include_self_option() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Bab".into(), "1".into(), "Pendahuluan".into()],
            ],
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
            let toc = active.doc.toc.as_ref().unwrap();
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
            let toc = active.doc.toc.as_ref().unwrap();
            let text = active.doc.story(toc.story).unwrap().text.clone();
            assert!(!text.contains(&format!("Daftar Isi\t{toc_label}")), "Self entry should be removed when unchecked: {text}");
            assert!(text.contains("Bab 1 Pendahuluan"), "Chapter entry should remain: {text}");
        }
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
        let toc = active.doc.toc.as_ref().unwrap();
        let story = active.doc.story(toc.story).unwrap();
        let text = &story.text;

        // 1. Must contain "Daftar Tabel" title and "Tabel\thalaman" column header
        assert!(text.contains("Daftar Tabel"), "Should contain title: {text}");
        assert!(text.contains("Tabel\thalaman"), "Should contain TablePage header line: {text}");

        // 2. Entries must use standalone number column: {num}\t{title}\t{page}
        assert!(
            text.contains("02.01\tJumlah Rukun Warga (RW)\t2"),
            "Entry 1 must have standalone number column: {text}"
        );
        assert!(
            text.contains("02.02\tJumlah Pegawai Negeri Sipil\t3"),
            "Entry 2 must have standalone number column: {text}"
        );

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
        let toc = active.doc.toc.as_ref().unwrap();
        let story = active.doc.story(toc.story).unwrap();
        let text = &story.text;

        // 1. Starts directly with Tabel\thalaman header
        assert!(text.starts_with("Tabel\thalaman"), "Should start with header when title is empty: {text}");

        // 2. Section header 4.3 must NOT have page number or leader dots
        assert!(
            text.contains("4.3\tPerumahan dan Lingkungan\n"),
            "Section header must not have a page number: {text}"
        );

        // 3. Table entry 4.3.1 must have page number
        assert!(
            text.contains("4.3.1\tBanyaknya Desa Menurut Penerangan Jalan\t3"),
            "Table entry must have page number: {text}"
        );

        // 4. Styles
        assert!(active.doc.styles.para("TOC Section Header").is_some());
    }

    #[test]
    fn test_tablepage_toc_continuation_headers_on_overflow_pages() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 80 records so TOC spans across multiple pages (at least 2 pages)
        let records: Vec<Vec<String>> = (1..=80)
            .map(|i| {
                vec![
                    "Tabel".into(),
                    format!("01.{i:02}."),
                    format!("Dataset Observation Record Number {i} in Region"),
                ]
            })
            .collect();

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
            let toc = active.doc.toc.as_ref().unwrap();
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
        assert_eq!(
            cont_count,
            cont_headers_after.len(),
            "Rebuild should not duplicate continuation headers"
        );
    }

    #[test]
    fn test_execute_page_from_rtf_sample_file() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let initial_pages = app.session.active().unwrap().doc.page_count();
        assert_eq!(initial_pages, 1);

        let mut st = PubgenState::default();
        let rtf_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/ulasan_bab_1.rtf");

        let res = execute_page_from_rtf(&mut app, &mut st, rtf_path);
        assert!(res.is_ok(), "execute_page_from_rtf should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("ulasan_bab_1.rtf"), "Success message should contain filename: {msg}");

        let active = app.session.active().unwrap();
        let story = active
            .doc
            .stories
            .values()
            .find(|s| s.text.contains("Dalam bab ini membahas"))
            .expect("Imported story must be present");

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
        let text_frames_count = active.doc.spreads.iter().flat_map(|sp| &sp.items).filter(|it| matches!(it.content, designcraft_doc::Content::Text(_))).count();
        assert!(text_frames_count >= final_pages, "Each page should have a threaded text frame");
    }

    #[test]
    fn test_execute_page_from_rtf_into_selected_frame() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Create an empty text frame on page 0
        let add_res = app.run(
            "frame.create",
            json!({
                "page": 1,
                "rect": [72, 72, 300, 400],
                "content": "text"
            }),
        ).unwrap();
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

        let mut st = PubgenState::default();
        st.image_as_background = false;
        let res = execute_page_from_image(&mut app, &mut st, &sample_image_path());
        assert!(res.is_ok(), "execute_page_from_image (object) should succeed: {res:?}");

        // The graphic frame sits proportionally inside the page margin rect.
        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(0).unwrap();
        let expected = active.doc.spreads[si].pages[pi].margin_rect();
        let frame = active
            .doc
            .spreads[si]
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
        let text = app
            .run("frame.create", json!({ "rect": [72, 72, 300, 400], "content": "text", "text": "Hello" }))
            .unwrap();
        let text_id = text.get("id").and_then(Value::as_u64).unwrap();

        let mut st = PubgenState::default();
        st.image_as_background = true;
        let res = execute_page_from_image(&mut app, &mut st, &sample_image_path());
        assert!(res.is_ok(), "execute_page_from_image (background) should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(0).unwrap();
        let page = &active.doc.spreads[si].pages[pi];
        let expected = page.bounds();

        let bg_index = active
            .doc
            .spreads[si]
            .items
            .iter()
            .position(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)))
            .unwrap_or_else(|| panic!("expected a background graphic frame"));
        assert_eq!(bg_index, 0, "the background frame must be at the back of the spread");
        let text_index = active
            .doc
            .spreads[si]
            .items
            .iter()
            .position(|it| it.id.0 == text_id)
            .unwrap();
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
}

