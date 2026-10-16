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
use crate::widgets::{divider, outline_button, rule, section, selection_band};

const PANEL_ID: &str = "pubgen_panel_state";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    pub card1_status: Option<(String, bool)>,

    // Card 2: Table from CSV
    pub card2_path: String,
    pub card2_matrix: Vec<Vec<String>>,
    pub card2_first_row_header: bool,
    pub card2_col_numbers: bool,
    pub card2_style: usize,
    pub card2_placement: usize,
    pub card2_status: Option<(String, bool)>,
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
    let mut st: PubgenState = ui.data(|d| d.get_temp(state_id)).unwrap_or_else(|| PubgenState {
        card1_range_all: true,
        card1_range_from: 1,
        card1_range_to: 10,
        card1_append: true,
        card2_first_row_header: true,
        card2_col_numbers: false,
        card2_placement: 2, // At page
        ..Default::default()
    });

    // Card 1: Page from CSV
    card1_ui(app, ui, &mut st, &t);

    divider(ui);
    rule(ui);

    // Card 2: Table from CSV
    card2_ui(app, ui, &mut st, &t);

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
            badge(ui, t, "type", st.card1_headers.get(st.card1_type_col).map(String::as_str).unwrap_or("col 0"));
            badge(ui, t, "number", st.card1_headers.get(st.card1_num_col).map(String::as_str).unwrap_or("col 1"));
            badge(ui, t, "title", st.card1_headers.get(st.card1_title_col).map(String::as_str).unwrap_or("col 2"));
        });
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
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !st.card2_path.is_empty() {
            load_card2_csv(st, &st.card2_path.clone());
        }
    });

    ui.checkbox(&mut st.card2_first_row_header, "First row is header");
    ui.checkbox(&mut st.card2_col_numbers, "Add column numbers (1), (2)…");

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

pub fn parse_csv_content(text: &str) -> Vec<Vec<String>> {
    let delim = if text.contains('\t') && !text.contains(',') { '\t' } else if text.contains(';') && !text.contains(',') { ';' } else { ',' };
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

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

fn load_card1_csv(st: &mut PubgenState, path: &str) {
    st.card1_path = path.to_string();
    let Ok(content) = std::fs::read_to_string(path) else {
        st.card1_status = Some((format!("Cannot read file: {path}"), false));
        return;
    };
    let mut rows = parse_csv_content(&content);
    if rows.is_empty() {
        st.card1_status = Some(("File is empty".into(), false));
        return;
    }
    let headers = rows.remove(0);
    st.card1_headers = headers.clone();
    st.card1_records = rows;

    // Detect column indexes case-insensitively
    let find_col = |patterns: &[&str], fallback: usize| -> usize {
        for (i, h) in headers.iter().enumerate() {
            let lower = h.to_lowercase();
            if patterns.iter().any(|p| lower.contains(p)) {
                return i;
            }
        }
        fallback.min(headers.len().saturating_sub(1))
    };

    st.card1_type_col = find_col(&["type", "category", "kicker", "kind"], 0);
    st.card1_num_col = find_col(&["number", "no", "num", "id", "#"], 1);
    st.card1_title_col = find_col(&["title", "name", "headline", "chapter"], 2);

    st.card1_range_to = st.card1_records.len();
    st.card1_status = Some((format!("Loaded {} records", st.card1_records.len()), true));
}

fn load_card2_csv(st: &mut PubgenState, path: &str) {
    st.card2_path = path.to_string();
    let Ok(content) = std::fs::read_to_string(path) else {
        st.card2_status = Some((format!("Cannot read file: {path}"), false));
        return;
    };
    let rows = parse_csv_content(&content);
    if rows.is_empty() {
        st.card2_status = Some(("File is empty".into(), false));
        return;
    }
    st.card2_matrix = rows;
    let r = st.card2_matrix.len();
    let c = st.card2_matrix.first().map(Vec::len).unwrap_or(0);
    st.card2_status = Some((format!("Loaded {r} rows × {c} columns"), true));
}

// ---------------------------------------------------------------------------
// Execution Pipelines
// ---------------------------------------------------------------------------

/// Estimate single-line text advance width in points (based on standard 12pt proportional font).
fn estimate_text_width(text: &str, is_uppercase_kicker: bool) -> f64 {
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

        let type_val = row.get(st.card1_type_col).map(String::as_str).unwrap_or("").trim();
        let num_val = row.get(st.card1_num_col).map(String::as_str).unwrap_or("").trim();
        let title_val = row.get(st.card1_title_col).map(String::as_str).unwrap_or("").trim();

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
                    let _ = app.run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": [c3_x0, row_y0, c3_x1, row_y1],
                            "content": "text",
                            "text": title_val,
                            "caret": false
                        }),
                    );
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
                    let _ = app.run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": [margin.x0, margin.y0 + 112.0, margin.x1, margin.y0 + 220.0],
                            "content": "text",
                            "text": title_val,
                            "caret": false
                        }),
                    );
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
                    let _ = app.run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": [margin.x0, margin.y0 + 40.0, margin.x1 - 90.0, margin.y0 + 110.0],
                            "content": "text",
                            "text": title_val,
                            "caret": false
                        }),
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
                    let _ = app.run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": [margin.x0, margin.y0 + 120.0, margin.x1, margin.y0 + 200.0],
                            "content": "text",
                            "text": title_val,
                            "caret": false
                        }),
                    );
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

                    let _ = app.run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": [c3_x0, row_y0, c3_x1, row_y1],
                            "content": "text",
                            "text": title_val,
                            "caret": false
                        }),
                    );
                }
            }
        }
        generated_count = generated_count.saturating_add(1);
    }

    Ok(format!("Successfully generated {generated_count} pages!"))
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

fn execute_insert_table(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    if st.card2_matrix.is_empty() {
        return Err("No tabular data to insert".into());
    }

    let rows_count = st.card2_matrix.len();
    let cols_count = st.card2_matrix.iter().map(Vec::len).max().unwrap_or(0);

    // Auto-detect merged header columns (split by "--", capped at max 3 stacks)
    let col_levels: Vec<Vec<String>> = if st.card2_first_row_header {
        st.card2_matrix
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
        for row in st.card2_matrix.iter().skip(1) {
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

        for (r_idx, row) in st.card2_matrix.iter().enumerate() {
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

    if st.card2_placement == 0 && !has_caret {
        return Err("No active text insertion point. Select a text frame or choose 'New frame' / 'At page'.".into());
    }

    let (story_id, target_page_info) = match st.card2_placement {
        0 => {
            // At cursor
            let story = active.selection.text.map(|t| t.story.0).ok_or("No active story")?;
            (story, None)
        }
        2 => {
            // At page: place below Generated Page Title on active / selected page
            let cur_page = target_page_index(app);
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

            let gap_below_title = 20.0;
            let table_y0 = if title_bottom > margin.y0 {
                title_bottom + gap_below_title
            } else {
                margin.y0 + 20.0
            };
            let available_h = (margin.y1 - table_y0).max(60.0);
            let frame_h = (total_rows as f64 * 24.0 + 36.0).clamp(60.0, available_h);

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
            (sid, Some(cur_page))
        }
        _ => {
            // 1 or default: New frame on active / selected page
            let cur_page = target_page_index(app);
            let (spread_idx, page_in_spread) = active.doc.page_loc(cur_page).unwrap_or((0, 0));
            let spread = active.doc.spreads.get(spread_idx).ok_or("Spread missing")?;
            let page = spread.pages.get(page_in_spread).ok_or("Page missing")?;
            let margin = page.margin_rect();
            let frame_h = (total_rows as f64 * 24.0 + 36.0).clamp(60.0, margin.height());

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
            (sid, Some(cur_page))
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
    if header_rows_count > 0 {
        let _ = app.run("table.options", json!({"headerRows": header_rows_count}));
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

                    // Bottom of table rule
                    let last_row = nr.saturating_sub(1);
                    for c in 0..nc {
                        if let Some(cell) = table.cell_mut(last_row, c) {
                            cell.strokes[2] = rule_thick.clone();
                        }
                    }
                }
            }

            Ok(())
        })
        .map_err(|e| e.to_string())?;

    let target_msg = match target_page_info {
        Some(pi) => {
            let label = app.session.page_label(pi);
            format!(" on Page {label}")
        }
        None => String::new(),
    };

    Ok(format!("Inserted table{target_msg} ({total_rows} rows × {cols_count} cols)"))
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
}
