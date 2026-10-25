//! Pubgen panel: automated CSV document publishing and table insertion.
//!
//! Provides two card tools matching the InDesign-style Properties layout:
//! 1. "Page from CSV": Ingests CSV (columns `type`, `number`, `title`) and synthesizes
//!    multi-page layouts using geometric templates.
//! 2. "Table from CSV": Ingests tabular CSV matrix data and constructs native styled tables.

use egui::{Color32, Rect, Response, RichText, Sense, Stroke, Ui, pos2, vec2};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::DesignApp;
use crate::theme::Tokens;
use crate::widgets::{divider, full_width, outline_button, rule, section, selection_band};

#[path = "excel.rs"]
pub mod excel;

#[path = "locale.rs"]
pub mod locale;
pub use locale::Locale;

pub mod ai;

mod cover_art;
mod covers;
mod csv;
mod divider_art;
mod header_footer;
mod helpers;
mod page_utils;
mod pages;
mod state;
mod tables;
mod themes;
mod thumbnails;
mod toc;
mod widgets;

pub(crate) use covers::*;
pub(crate) use csv::*;
pub(crate) use excel::*;
pub(crate) use header_footer::*;
pub(crate) use helpers::*;
pub(crate) use page_utils::*;
pub(crate) use pages::*;
pub use state::PubgenState;
pub(crate) use tables::*;
pub(crate) use themes::*;
pub(crate) use thumbnails::*;
pub(crate) use toc::*;
pub(crate) use widgets::*;

use cover_art::CoverArtVals;
use divider_art::DividerArtVals;

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

/// Index of the "Tri-Color Bands" cover style — the executive blue/green/orange template.
const TRICOLOR_STYLE: usize = 10;

/// Index of the "Prism Diamonds" cover style — the jewelled diamond cascade on a midnight field.
const PRISM_STYLE: usize = 11;

/// Index of the "Quarter Stack" cover style — the fanned cards on a warm paper field.
const QUARTER_STYLE: usize = 12;

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
    let mut st: PubgenState = ui.data(|d| d.get_temp(state_id)).unwrap_or_else(|| PubgenState {
        bilingual_output: true,
        cover_include_inside_back: true,
        ..Default::default()
    });

    // AI setup modal + background job poll (progress and Done both land here).
    if let Some(mut job) = st.translate_job.take() {
        let ok = job.poll(&mut st);
        if job.finished {
            st.translate_job = None;
            if ok {
                // Page-targeted job: write the results into the page's own textboxes.
                if let Some(target) = st.ai_page_target.take() {
                    ai::apply_page_result(app, &st, &target);
                    st.header_status = Some(("Page translated with AI.".into(), true));
                    ui.data_mut(|d| d.insert_temp(state_id, st));
                    return;
                }
                if let Some(target) = st.ai_table_target.take() {
                    ai::apply_table_page_result(app, &st, &target);
                    st.header_status = Some(("Table page translated with AI.".into(), true));
                    ui.data_mut(|d| d.insert_temp(state_id, st));
                    return;
                }
                if !st.ai_table_targets_batch.is_empty() {
                    let targets = std::mem::take(&mut st.ai_table_targets_batch);
                    let mut count = 0;
                    for target in &targets {
                        count += ai::apply_table_page_result(app, &st, target);
                    }
                    st.header_status = Some((format!("{} table item(s) across {} page(s) translated with AI.", count, targets.len()), true));
                    ui.data_mut(|d| d.insert_temp(state_id, st));
                    return;
                }
                st.card1_status = Some(("AI translation ready — generating…".into(), true));
                // Excel flow: the full convert+generate runs with the translated maps.
                let res = if st.ai_pending_excel_path.is_empty() {
                    execute_generate_pages(app, &st)
                } else {
                    let path = std::mem::take(&mut st.ai_pending_excel_path);
                    execute_excel_and_generate(app, &mut st, &path)
                };
                st.card1_status = Some(match res {
                    Ok(msg) => (msg, true),
                    Err(err) => (err, false),
                });
                ui.data_mut(|d| d.insert_temp(state_id, st));
                return;
            }
            // Stopped (cancel/error): keep the partial translations, clear the slice
            // signature so the next Generate collects and translates only the rest.
            st.translations_slice_key.clear();
            st.ai_pending_excel_path.clear();
            st.ai_page_target = None;
            st.ai_table_target = None;
            st.ai_table_targets_batch.clear();
        } else {
            st.translate_job = Some(job);
        }
    }
    ai::dialog_ui(app, &mut st, ui);

    pubgen_header_band(app, ui, &mut st, &t);

    // 1. Data Source & Generate (Master Excel)
    master_excel_ui(app, ui, &mut st, &t);

    divider(ui);
    rule(ui);

    // 2. Table of Contents (Dedicated Section)
    card_toc_ui(app, ui, &mut st, &t);

    divider(ui);
    rule(ui);

    // 3. Manual Import & Fine-Tuning (Collapsible Accordion)
    manual_import_accordion_ui(app, ui, &mut st, &t);

    divider(ui);
    rule(ui);

    // 4. Page Utilities
    page_utilities_ui(app, ui, &mut st, &t);

    if st.show_about_dialog {
        about_pubgen_dialog(app, ui.ctx(), &mut st.show_about_dialog, &t);
    }

    ui.data_mut(|d| d.insert_temp(state_id, st));
}

#[allow(dead_code)]
fn card1_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Page from CSV"), true);
    card1_body_ui(app, ui, st, t);
}

fn card1_body_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
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

    // Action button. With AI translation on, generation is two-phase: the first click
    // collects the Indonesian units and spawns the worker; when the job finishes (polled
    // in show()), the same click path generates with the EN maps filled.
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

#[allow(dead_code)]
fn card2_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Table from CSV"), true);
    card2_body_ui(app, ui, st, t);
}

fn card2_body_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
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
    ui.checkbox(&mut st.card2_row_conditions, "Condition rules (Jumlah, Tahun…)");

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

    // Table Color Theme Selector
    ui.horizontal(|ui| {
        ui.label(RichText::new("Theme:").size(11.0).color(t.text));
        let cur = TABLE_COLOR_THEMES.get(st.card2_color_theme).map(|(name, _, _)| *name).unwrap_or("#FFCB56 - #FFE9A9");
        let combo_w = (ui.available_width() - 4.0).clamp(80.0, 200.0);
        let prev_theme = st.card2_color_theme;
        egui::ComboBox::from_id_salt("pubgen_card2_theme").selected_text(cur).width(combo_w).show_ui(ui, |ui| {
            for (i, (name, _, _)) in TABLE_COLOR_THEMES.iter().enumerate() {
                ui.selectable_value(&mut st.card2_color_theme, i, *name);
            }
        });
        if st.card2_color_theme != prev_theme
            && let Some((name, _, _)) = TABLE_COLOR_THEMES.get(st.card2_color_theme)
        {
            st.card2_theme_override = Some(name.to_string());
        }
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

#[allow(dead_code)]
fn card_cover_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Publication Cover"), true);
    card_cover_body_ui(app, ui, st, t);
}

fn card_cover_body_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
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
            "Tri-Color Bands",
            "Prism Diamonds",
            "Quarter Stack",
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

    ui.checkbox(&mut st.cover_include_inside_back, "Add Inside cover and Back Cover").on_hover_text(
        "When checked, automatically generates Inside Cover (grayscale & outlines) on page 2 and Back Cover at the end of the publication",
    );

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
                    TRICOLOR_STYLE => "Cityscape photograph filling the right-hand panel",
                    PRISM_STYLE => "Cityscape photograph cut as the largest diamond of the cascade",
                    QUARTER_STYLE => "Cityscape photograph set in the front card of the stack, rounded on one corner",
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

    if st.cover_include_inside_back {
        divider(ui);
        ui.label(RichText::new("Back Cover:").size(11.0).strong().color(t.text_strong));
        ui.horizontal(|ui| {
            ui.label(RichText::new("Event Theme:").size(10.5).color(t.text));
            let w = (ui.available_width() - 65.0).max(80.0);
            ui.add_sized([w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.cover_image_back).hint_text("outputs/event_theme.png"))
                .on_hover_text("Back cover event theme image (from COVER sheet event_theme)");
            if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked()
                && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("place"))
            {
                st.cover_image_back = path;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Office Address:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(80.0);
            ui.add_sized(
                [avail, crate::widgets::FIELD_H * 1.8],
                egui::TextEdit::multiline(&mut st.cover_office_address).hint_text("Jl. Cakradireja No.36 Karawang 41312..."),
            )
            .on_hover_text("Office address on back cover (from COVER sheet row 12 office_address)");
        });
    }

    // Action button
    let fw = full_width(ui);
    let btn_label = if st.cover_include_inside_back { "Generate Cover Pages (Front, Inside, Back)" } else { "Generate Cover Page" };
    let gen_btn = outline_button(ui, btn_label, fw).on_hover_text("Automatically inserts cover pages and generates layout");
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

fn master_excel_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "1. Data Source & Generate (Master Excel)"), true);

    let tip_excel = "Convert a multi-sheet Excel file (.xlsx) into 1 catalog CSV for Page from CSV + individual table CSVs (with auto-merged headers). \
        A sheet named COVER supplies the Publication Cover values (style, title1, title2, year, volume, no_catalogue, ISSN, office_name, image_1, image_lanscape, office_logo)";
    ui.checkbox(&mut st.excel_auto_generate, "Auto-generate pages, tables & cover from sheets")
        .on_hover_text("When checked, picking an Excel file automatically runs 'Page from CSV', inserts each sheet's table with default options, and generates the cover page before Page 1 from the COVER sheet");

    let resp_excel = primary_outline_button(ui, t, "Excel to CSV Data (.xlsx)…", full_width(ui)).on_hover_text(tip_excel);
    if resp_excel.clicked()
        && let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("dataMerge"))
    {
        st.excel_source_path = path.clone();
        let res = execute_excel_and_generate(app, st, &path);
        let (msg, ok) = match res {
            Ok(msg) => (msg, true),
            Err(e) => (e, false),
        };
        st.excel_status = Some((msg.clone(), ok));
        st.card3_status = Some((msg, ok));
    }

    // Active catalog / excel indicator
    let (active_label, active_color) = if !st.excel_source_path.is_empty() {
        let name = std::path::Path::new(&st.excel_source_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| st.excel_source_path.clone());
        (format!("Active: {name} ({} records)", st.card1_records.len()), t.text)
    } else if !st.card1_path.is_empty() {
        let name = std::path::Path::new(&st.card1_path).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| st.card1_path.clone());
        (format!("Active: {name} ({} records)", st.card1_records.len()), t.text)
    } else {
        (crate::i18n::tr(&app.ui.language, "No Excel file loaded").to_string(), t.text_dim)
    };
    ui.add(egui::Label::new(RichText::new(active_label).size(11.0).color(active_color)).truncate());

    let tip_zip = "Download all generated catalog and table CSVs packaged in a single .zip archive";
    let zip_label = if !st.generated_zip_name.is_empty() {
        format!("Download {}…", st.generated_zip_name)
    } else {
        "Download Generated CSVs as ZIP…".to_string()
    };
    if outline_button(ui, &zip_label, full_width(ui)).on_hover_text(tip_zip).clicked() {
        let res = execute_download_csv_zip(app, st);
        let (msg, ok) = match res {
            Ok(msg) => (msg, true),
            Err(e) => (e, false),
        };
        st.excel_status = Some((msg.clone(), ok));
        st.card3_status = Some((msg, ok));
    }

    if let Some((msg, ok)) = &st.excel_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

fn card_toc_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "2. Table of Contents"), true);

    let cur_page =
        app.session.active().and_then(|s| s.selection.page).or_else(|| crate::canvas::current_page(app)).unwrap_or_else(|| target_page_index(app));
    let cur_label = app.session.page_label(cur_page);

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
    ui.checkbox(&mut st.toc_insert_after, crate::i18n::tr(&app.ui.language, "Insert on new page after selected page"))
        .on_hover_text(format!("Insert a new blank page after Page {cur_label} and build the TOC on it"));

    let hw_toc = (ui.available_width() - 4.0) * 0.5;
    ui.horizontal(|ui| {
        let add_toc_text = crate::i18n::tr(&app.ui.language, "Add TOC");
        let tip_toc = if st.toc_insert_after {
            format!("Insert blank page after Page {cur_label} and generate a {} on it", TOC_KINDS.get(st.toc_kind).copied().unwrap_or(TOC_KINDS[0]))
        } else {
            format!("Insert a {} on Page {cur_label}", TOC_KINDS.get(st.toc_kind).copied().unwrap_or(TOC_KINDS[0]))
        };
        if outline_button(ui, add_toc_text, hw_toc).on_hover_text(tip_toc).clicked() {
            let res = execute_add_toc(app, st);
            let (msg, ok) = match res {
                Ok(msg) => (msg, true),
                Err(e) => (e, false),
            };
            st.toc_status = Some((msg.clone(), ok));
            st.card3_status = Some((msg, ok));
        }

        let rebuild_toc_text = crate::i18n::tr(&app.ui.language, "Rebuild TOC");
        let tip_rebuild = "Rebuild the table of contents on this page (its own TOC style, title and page numbers)";
        if outline_button(ui, rebuild_toc_text, hw_toc).on_hover_text(tip_rebuild).clicked() {
            let res = execute_rebuild_toc(app, st);
            let (msg, ok) = match res {
                Ok(msg) => (msg, true),
                Err(e) => (e, false),
            };
            st.toc_status = Some((msg.clone(), ok));
            st.card3_status = Some((msg, ok));
        }
    });

    if let Some((msg, ok)) = &st.toc_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

fn manual_import_accordion_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "3. Manual Import & Fine-Tuning"), true);

    egui::CollapsingHeader::new(crate::rtl::widget(
        ui,
        egui::RichText::new(crate::i18n::tr(&app.ui.language, "Publication Cover (Style, Theme & Overrides)")).size(11.0).color(t.text_strong),
    ))
    .id_salt("pubgen_collapse_cover")
    .default_open(false)
    .show(ui, |ui| {
        card_cover_body_ui(app, ui, st, t);
    });

    divider(ui);

    egui::CollapsingHeader::new(crate::rtl::widget(
        ui,
        egui::RichText::new(crate::i18n::tr(&app.ui.language, "Page from CSV (Column Mappings & Ingest)")).size(11.0).color(t.text_strong),
    ))
    .id_salt("pubgen_collapse_card1")
    .default_open(false)
    .show(ui, |ui| {
        card1_body_ui(app, ui, st, t);
    });

    divider(ui);

    egui::CollapsingHeader::new(crate::rtl::widget(
        ui,
        egui::RichText::new(crate::i18n::tr(&app.ui.language, "Table from CSV (Theme & Options)")).size(11.0).color(t.text_strong),
    ))
    .id_salt("pubgen_collapse_card2")
    .default_open(false)
    .show(ui, |ui| {
        card2_body_ui(app, ui, st, t);
    });

    divider(ui);

    egui::CollapsingHeader::new(crate::rtl::widget(
        ui,
        egui::RichText::new(crate::i18n::tr(&app.ui.language, "Header & Footer Template")).size(11.0).color(t.text_strong),
    ))
    .id_salt("pubgen_collapse_hf")
    .default_open(false)
    .show(ui, |ui| {
        card_header_footer_body_ui(app, ui, st, t);
    });
}

fn page_utilities_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "4. Page Utilities"), true);

    let facing = app.session.active().map(|s| s.doc.settings.facing_pages).unwrap_or(true);
    let cur_page =
        app.session.active().and_then(|s| s.selection.page).or_else(|| crate::canvas::current_page(app)).unwrap_or_else(|| target_page_index(app));
    let cur_label = app.session.page_label(cur_page);

    // Button 1: switch Facing Pages
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
            let msg = if next_facing { "Switched to Double-sided (Facing Pages)".to_string() } else { "Switched to Single-sided layout".to_string() };
            st.utilities_status = Some((msg.clone(), true));
            st.card3_status = Some((msg, true));
        }

        let tip_page = format!("Insert a blank page after Page {cur_label}");
        let resp_page = outline_button(ui, add_page_text, hw).on_hover_text(tip_page);
        if resp_page.clicked() {
            let _ = app.run("layout.pages.insert", json!({"after": cur_page, "count": 1}));
            crate::canvas::go_to_page(app, cur_page + 1);
            let next_label = app.session.page_label(cur_page + 1);
            let msg = format!("Added blank page after Page {cur_label} (new Page {next_label})");
            st.utilities_status = Some((msg.clone(), true));
            st.card3_status = Some((msg, true));
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

    let sec_text = crate::i18n::tr(&app.ui.language, "Numbering & Section Options…");
    ui.add_space(4.0);
    let resp_sec = outline_button(ui, sec_text, fw).on_hover_text("Open Layout › Numbering & Section Options dialog for the active page");
    if resp_sec.clicked() {
        crate::section_options::open(app);
    }

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
            let (msg, ok) = match res {
                Ok(msg) => (msg, true),
                Err(e) => (e, false),
            };
            st.utilities_status = Some((msg.clone(), ok));
            st.card3_status = Some((msg, ok));
        }
        let tip_cont = format!("Continue numbering on Page {cur_label} (remove section restart)");
        if outline_button(ui, "Continue #", hw_renum).on_hover_text(tip_cont).clicked() {
            let res = execute_continue_numbering(app, st);
            let (msg, ok) = match res {
                Ok(msg) => (msg, true),
                Err(e) => (e, false),
            };
            st.utilities_status = Some((msg.clone(), ok));
            st.card3_status = Some((msg, ok));
        }
    });

    let mode_str = if facing { "Double-sided" } else { "Single-sided" };
    ui.add(egui::Label::new(RichText::new(format!("Mode: {mode_str} · Selected: Page {cur_label}")).size(10.5).color(t.text_dim)).truncate());

    if let Some((msg, ok)) = &st.utilities_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

#[allow(dead_code)]
fn card3_quick_actions_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    page_utilities_ui(app, ui, st, t);
}

// ---------------------------------------------------------------------------
// CSV File Ingestion (RFC 4180 parsing with quotes and delimiter sniffing)
// ---------------------------------------------------------------------------

/// Supports:
/// - Direct file path (as-is or with `.designcraft`)
/// - Path relative to the catalog file's directory
/// - Path relative to `assets/templates/` or any of its subfolders (`dividers/`, `covers/`, etc.)
/// - Template ID match from scanned template catalog (e.g. `001_ulasan_chapter` or fuzzy stem)
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

    let clean_lower = clean.to_lowercase();
    let alias_opt = match clean_lower.as_str() {
        "dividers/special_infographic_divider"
        | "special_infographic_divider"
        | "dividers/002_chapter_devider"
        | "002_chapter_devider"
        | "chapter_devider"
        | "dividers/002_batik_infographic"
        | "002_batik_infographic"
        | "batik_infographic" => Some("dividers/002_batik_infographic"),
        "dividers/001_ulasan_chapter"
        | "001_ulasan_chapter"
        | "ulasan_chapter"
        | "dividers/001_simple_chapter"
        | "001_simple_chapter"
        | "simple_chapter" => Some("dividers/001_simple_chapter"),
        _ => None,
    };

    // 1. Direct path check as-is or with .designcraft
    let direct_p = std::path::Path::new(clean);
    if direct_p.is_file() {
        return Some(clean.to_string());
    }
    let direct_ext = format!("{clean}.designcraft");
    if std::path::Path::new(&direct_ext).is_file() {
        return Some(direct_ext);
    }
    if let Some(alias) = alias_opt {
        let alias_ext = format!("{alias}.designcraft");
        if std::path::Path::new(alias).is_file() {
            return Some(alias.to_string());
        }
        if std::path::Path::new(&alias_ext).is_file() {
            return Some(alias_ext);
        }
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
        if let Some(alias) = alias_opt {
            let alias_stem = alias.strip_prefix("dividers/").unwrap_or(alias);
            if let Some(found) = scanned.iter().find(|t| t.id.to_lowercase() == alias_stem) {
                return Some(found.path.to_string_lossy().to_string());
            }
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

/// The number of paragraphs the TOC would collect for `style` right now.
#[cfg(test)]
fn toc_entry_count(app: &mut DesignApp, style: &str) -> usize {
    app.session.execute("toc.entries", &json!({"entries": [{"style": style, "level": 1}]})).ok().and_then(|v| v.as_array().map(Vec::len)).unwrap_or(0)
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
        assert!(matches_pubgen_row_condition("- Jumlah Subtotal"));
        assert!(matches_pubgen_row_condition("Total"));
        assert!(matches_pubgen_row_condition("Sum"));
        assert!(matches_pubgen_row_condition("Subtotal"));
        assert!(matches_pubgen_row_condition("Year 2026"));

        // Negative tests (Kecamatan / Kabupaten / Desa are region names, not summary break rows)
        assert!(!matches_pubgen_row_condition("Kab. Karawang"));
        assert!(!matches_pubgen_row_condition("Kabupaten Karawang"));
        assert!(!matches_pubgen_row_condition("Kec. Klari"));
        assert!(!matches_pubgen_row_condition("Kecamatan Klari"));
        assert!(!matches_pubgen_row_condition("District 1"));
        assert!(!matches_pubgen_row_condition("Subdistrict"));
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
    fn test_is_numeric_or_dash() {
        // Dashes and missing data symbols
        assert!(is_numeric_or_dash("-"));
        assert!(is_numeric_or_dash("—"));
        assert!(is_numeric_or_dash("–"));
        assert!(is_numeric_or_dash("- "));
        assert!(is_numeric_or_dash("--"));
        assert!(is_numeric_or_dash("−"));
        assert!(is_numeric_or_dash("..."));
        assert!(is_numeric_or_dash("…"));
        assert!(is_numeric_or_dash("N/A"));
        assert!(is_numeric_or_dash("n/a"));
        assert!(is_numeric_or_dash("NA"));
        assert!(is_numeric_or_dash("na"));

        // Integers and floats
        assert!(is_numeric_or_dash("0"));
        assert!(is_numeric_or_dash("7"));
        assert!(is_numeric_or_dash("4.02"));
        assert!(is_numeric_or_dash("1.65"));
        assert!(is_numeric_or_dash("-12.5"));
        assert!(is_numeric_or_dash("−12.5"));
        assert!(is_numeric_or_dash("100"));

        // Accounting parentheses
        assert!(is_numeric_or_dash("(12.5)"));
        assert!(is_numeric_or_dash("(100)"));

        // Thousands and European / Indonesian formats
        assert!(is_numeric_or_dash("1,234.56"));
        assert!(is_numeric_or_dash("1.234,56"));
        assert!(is_numeric_or_dash("1,000"));

        // Percentages
        assert!(is_numeric_or_dash("21.32%"));
        assert!(is_numeric_or_dash("100%"));

        // Text strings (must return false)
        assert!(!is_numeric_or_dash(""));
        assert!(!is_numeric_or_dash("   "));
        assert!(!is_numeric_or_dash("Warungbambu"));
        assert!(!is_numeric_or_dash("Desa / Village"));
        assert!(!is_numeric_or_dash("Jumlah"));
        assert!(!is_numeric_or_dash("Total"));
        assert!(!is_numeric_or_dash("Plawad"));
    }

    #[test]
    fn test_execute_insert_table_header_and_col_alignments() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Desa/Kelurahan".into(), "Luas Wilayah".into(), "Persentase".into(), "Keterangan".into()],
                vec!["Warungbambu".into(), "1.65".into(), "21.32".into(), "Valid".into()],
                vec!["Kondangjaya".into(), "2.44".into(), "8.84".into(), "Valid".into()],
                vec!["Tegalsawah".into(), "-".into(), "13.48".into(), "Special".into()],
                vec!["Total".into(), "4.09".into(), "43.64".into(), "Summary".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "Table insertion failed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();
        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        // 1 header row + 1 column number row + 4 data rows = 6 rows
        assert_eq!(table.rows.len(), 6);
        assert_eq!(table.columns.len(), 4);

        // Row 0: Table Header -> all cells must be Center aligned and VerticalJustification::Center
        for c in 0..4 {
            let cell = table.cell(0, c).unwrap();
            assert_eq!(cell.vj, designcraft_doc::VerticalJustification::Center, "Header col {c} vj should be Center");
            assert_eq!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Header col {c} align should be Center");
        }

        // Row 1: Column number row (1), (2), (3), (4) -> all cells must be Center aligned
        for c in 0..4 {
            let cell = table.cell(1, c).unwrap();
            assert_eq!(cell.vj, designcraft_doc::VerticalJustification::Center, "Col number {c} vj should be Center");
            assert_eq!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Col number {c} align should be Center");
        }

        // Data rows (rows 2..6):
        // Col 0: Column 1 (index 0) is excluded from centering -> align should NOT be Center
        for r in 2..6 {
            let cell = table.cell(r, 0).unwrap();
            assert_ne!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Data row {r} col 0 should remain left aligned");
        }

        // Col 1: Majority numeric / dash ("1.65", "2.44", "-", "4.09") -> must be Center aligned
        for r in 2..6 {
            let cell = table.cell(r, 1).unwrap();
            assert_eq!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Data row {r} col 1 should be Center aligned");
        }

        // Col 2: All numeric ("21.32", "8.84", "13.48", "43.64") -> must be Center aligned
        for r in 2..6 {
            let cell = table.cell(r, 2).unwrap();
            assert_eq!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Data row {r} col 2 should be Center aligned");
        }

        // Col 3: Text descriptions ("Valid", "Special", "Summary") -> must NOT be Center aligned
        for r in 2..6 {
            let cell = table.cell(r, 3).unwrap();
            assert_ne!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Data row {r} col 3 should remain left aligned");
        }
    }

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
    }

    #[test]
    fn test_execute_insert_table_color_theme_default() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Kecamatan".into(), "2021".into(), "2022".into(), "2025".into()],
                vec!["Cikampek".into(), "10".into(), "20".into(), "30".into()],
                vec!["Klari".into(), "15".into(), "25".into(), "35".into()],
                vec!["Karawang".into(), "25".into(), "45".into(), "65".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_color_theme: 0, // Default: #FFCB56 - #FFE9A9
            card2_placement: 1,   // New frame
            card2_style: 3,       // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "Table insertion failed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();
        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        // 1 header row + 1 col numbers row + 3 data rows (last is footer) = 5 rows
        assert_eq!(table.rows.len(), 5);

        // Header row (row 0) must have fill == "#FFCB56"
        for c in 0..4 {
            let cell = table.cell(0, c).unwrap();
            assert_eq!(cell.fill, "#FFCB56");
            assert_eq!(cell.fill_tint, 1.0);
        }

        // Col number row (row 1) must have fill == "#FFE9A9"
        for c in 0..4 {
            let cell = table.cell(1, c).unwrap();
            assert_eq!(cell.fill, "#FFE9A9");
            assert_eq!(cell.fill_tint, 1.0);
        }

        // Interior data rows (rows 2, 3) must have uncolored fill
        for r in 2..4 {
            for c in 0..4 {
                let cell = table.cell(r, c).unwrap();
                assert_eq!(cell.fill, designcraft_color::swatch::NONE);
            }
        }

        // Footer row (row 4) must have fill == "#FFCB56"
        for c in 0..4 {
            let cell = table.cell(4, c).unwrap();
            assert_eq!(cell.fill, "#FFCB56");
            assert_eq!(cell.fill_tint, 1.0);
        }

        // Swatches must be registered in doc
        assert!(active.doc.swatches.iter().any(|s| s.name == "#FFCB56"));
        assert!(active.doc.swatches.iter().any(|s| s.name == "#FFE9A9"));
    }

    #[test]
    fn test_execute_insert_table_color_theme_metadata_override() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["theme".into(), "#F3D2AF - #F8EADC".into()],
                vec!["Kecamatan".into(), "2021".into(), "2025".into()],
                vec!["Cikampek".into(), "10".into(), "30".into()],
                vec!["Karawang".into(), "25".into(), "65".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_color_theme: 0, // default is 0, but metadata overrides to #F3D2AF - #F8EADC
            card2_placement: 1,
            card2_style: 3,
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "Table insertion failed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();
        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        // 1 header row + 1 col numbers row + 2 data rows = 4 rows
        assert_eq!(table.rows.len(), 4);

        // Header row must have #F3D2AF
        assert_eq!(table.cell(0, 0).unwrap().fill, "#F3D2AF");
        // Col number row must have #F8EADC
        assert_eq!(table.cell(1, 0).unwrap().fill, "#F8EADC");
        // Interior data row must have NONE
        assert_eq!(table.cell(2, 0).unwrap().fill, designcraft_color::swatch::NONE);
        // Footer row must have #F3D2AF
        assert_eq!(table.cell(3, 0).unwrap().fill, "#F3D2AF");
    }

    #[test]
    fn test_execute_insert_table_bilingual_english_italic() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Kecamatan / District".into(), "Luas / Area".into(), "Keterangan\nDescription".into()],
                vec!["Cikampek".into(), "10".into(), "Aktif / Active".into()],
                vec!["Karawang".into(), "25".into(), "Selesai / Finished".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_table_footer: false,
            card2_placement: 1,
            card2_style: 3,
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "Table insertion failed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();
        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        // Check Header Cell (0, 0): "Kecamatan / District"
        let cell_0_0 = table.cell(0, 0).unwrap();
        assert_eq!(cell_0_0.text.text, "Kecamatan / District");
        // "Kecamatan / " is upright (not italic)
        let fmt_id = cell_0_0.text.char_format_of(0);
        assert_ne!(fmt_id.over.font_style.as_deref(), Some("Italic"));
        assert_ne!(fmt_id.over.font_style.as_deref(), Some("Bold Italic"));
        // "District" (starting at byte 12) must be Italic / Bold Italic
        let fmt_en = cell_0_0.text.char_format_of(12);
        let en_style = fmt_en.over.font_style.as_deref().unwrap_or("");
        assert!(en_style.contains("Italic"), "English text 'District' must be italic, got: {en_style}");

        // Check Data Cell (1, 2): "Aktif / Active"
        let cell_1_2 = table.cell(1, 2).unwrap();
        assert_eq!(cell_1_2.text.text, "Aktif / Active");
        let fmt_aktif = cell_1_2.text.char_format_of(0);
        assert_ne!(fmt_aktif.over.font_style.as_deref(), Some("Italic"));
        let fmt_active = cell_1_2.text.char_format_of(8);
        let active_style = fmt_active.over.font_style.as_deref().unwrap_or("");
        assert!(active_style.contains("Italic"), "English text 'Active' must be italic, got: {active_style}");

        // Check Cell with line break (0, 2): "Keterangan\u{2028}Description"
        let cell_0_2 = table.cell(0, 2).unwrap();
        let desc_byte = cell_0_2.text.text.find("Description").unwrap();
        let fmt_desc = cell_0_2.text.char_format_of(desc_byte);
        let desc_style = fmt_desc.over.font_style.as_deref().unwrap_or("");
        assert!(desc_style.contains("Italic"), "Line 2 English text 'Description' must be italic, got: {desc_style}");
    }

    #[test]
    fn test_execute_insert_table_with_theme_from_div_csv() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Simulate CSV generated with theme row from DIV sheet (table_theme: #9E7EB5 - #C4A4DA)
        let matrix = vec![
            vec!["Kecamatan / District".into(), "Luas / Area".into()],
            vec!["Karawang Timur".into(), "32.5".into()],
            vec!["Jumlah / Total".into(), "32.5".into()],
            vec!["theme".into(), "#9E7EB5 - #C4A4DA".into()],
        ];

        let st = PubgenState {
            card2_matrix: matrix,
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_placement: 1,
            card2_style: 3,
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "Table insertion failed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();
        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        // 4 rows in table (metadata theme row was stripped):
        // Row 0: Header (fill = #9E7EB5)
        // Row 1: Col numbers (fill = #C4A4DA)
        // Row 2: Data row (fill = NONE)
        // Row 3: Footer row (fill = #9E7EB5)
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.cell(0, 0).unwrap().fill, "#9E7EB5");
        assert_eq!(table.cell(1, 0).unwrap().fill, "#C4A4DA");
        assert_eq!(table.cell(2, 0).unwrap().fill, designcraft_color::swatch::NONE);
        assert_eq!(table.cell(3, 0).unwrap().fill, "#9E7EB5");
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

        for sample in ["B01.rtf", "B01_en.rtf"] {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({})).unwrap();

            // Place image1 with boundingBox wrap before placing RTF, matching execute_generate_pages
            let img1_rect = [margin.x0, margin.y0 + mh * 0.22, margin.x0 + mw * 0.24, margin.y0 + mh * 0.43];
            let res_img = app.run("frame.create", json!({"spread": 0, "rect": img1_rect, "content": "graphic"})).unwrap();
            let img_fid = res_img.get("id").and_then(Value::as_u64).unwrap();
            let _ = app.run("object.textWrap", json!({"mode": "boundingBox", "offset": 6.0, "ids": [img_fid]}));

            let rtf_rect = [margin.x0, margin.y0 + 50.0, margin.x1, margin.y1];
            let res = app.run("frame.create", json!({"spread": 0, "rect": rtf_rect, "content": "text", "caret": false})).unwrap();
            let fid = res.get("id").and_then(Value::as_u64).unwrap();
            let _ = app.run("selection.set", json!({"ids": [fid]}));

            let mut st = PubgenState::default();
            let rtf_path = format!("{}/../../plan/pubgen/samples/DUAL_LANG/{}", env!("CARGO_MANIFEST_DIR"), sample);
            if std::path::Path::new(&rtf_path).exists() {
                let place_res = execute_page_from_rtf(&mut app, &mut st, &rtf_path);
                assert!(place_res.is_ok(), "execute_page_from_rtf should place {sample}: {place_res:?}");

                harmonize_signatory_block(&mut app, fid, margin);

                let (target_spread, img2_rect) = compute_signature_placement(&app, fid, default_img2_rect, 0, margin);
                assert_eq!(target_spread, 0);

                // The dynamic signature rect must be anchored down near the signatory block at the bottom
                // of the text, significantly lower than the default static position (~308pt).
                assert!(
                    img2_rect[1] > default_img2_rect[1] + 50.0,
                    "Dynamic signature y0 ({}) should be anchored lower than default ({}) for {sample}",
                    img2_rect[1],
                    default_img2_rect[1]
                );
                assert!(img2_rect[1] >= margin.y0 && img2_rect[3] <= margin.y1);
                assert!(img2_rect[0] >= margin.x0 && img2_rect[2] <= margin.x1 - 20.0, "Signature must not hug right page edge for {sample}");

                // Verify composed lines: all signatory lines must be well away from the right margin
                // and must share consistent horizontal centering.
                let active = app.session.active().unwrap();
                let sid = story_of_frame(&app, fid).unwrap();
                let cs = designcraft_compose::compose_story(&active.doc, sid, &Default::default());
                let non_empty_lines: Vec<&designcraft_compose::Line> = cs
                    .frames
                    .iter()
                    .flat_map(|f| &f.lines)
                    .filter(|l| l.glyphs.iter().any(|g| g.visible && !g.rendered_char.is_whitespace()))
                    .collect();

                assert!(non_empty_lines.len() >= 4, "Too few lines for {sample}: {}", non_empty_lines.len());
                let sig_lines = &non_empty_lines[non_empty_lines.len() - 4..];
                let mut sig_centers = Vec::new();
                for l in sig_lines {
                    let min_x = l.glyphs.iter().filter(|g| g.visible && !g.rendered_char.is_whitespace()).map(|g| g.x).fold(f64::INFINITY, f64::min);
                    let max_x = l
                        .glyphs
                        .iter()
                        .filter(|g| g.visible && !g.rendered_char.is_whitespace())
                        .map(|g| g.x + g.adv)
                        .fold(f64::NEG_INFINITY, f64::max);
                    assert!(
                        max_x <= margin.x1 - 25.0,
                        "Signatory line max_x ({max_x}) must not be too close to margin.x1 ({}) in {sample}",
                        margin.x1
                    );
                    sig_centers.push((min_x + max_x) / 2.0);
                }
                // All signatory lines must share matching center alignment (within 3 points)
                let avg_center: f64 = sig_centers.iter().sum::<f64>() / sig_centers.len() as f64;
                for &c in &sig_centers {
                    assert!((c - avg_center).abs() < 3.0, "Signatory line center ({c}) does not match average center ({avg_center}) in {sample}");
                }
            }
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
        for style in 0..=10 {
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
    fn test_cover_style_10_uses_the_tri_color_template() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState {
            cover_style: TRICOLOR_STYLE,
            cover_title: "KECAMATAN KARAWANG TIMUR".into(),
            cover_title2: "DALAM ANGKA".into(),
            ..PubgenState::default()
        };
        let res = execute_generate_cover(&mut app, &st);
        assert!(res.is_ok(), "Tri-Color Bands must apply its template: {res:?}");

        let active = app.session.active().unwrap();
        // The template brings its own three-colour swatches.
        for name in ["Cover Blue", "Cover Green", "Cover Orange"] {
            assert!(active.doc.swatches.iter().any(|s| s.name == name), "swatch `{name}` comes with the template");
        }
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("KARAWANG TIMUR")), "the headline is bound: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("DALAM ANGKA")), "the second headline line is bound: {texts:?}");
    }

    #[test]
    fn test_cover_style_11_uses_the_prism_diamonds_template() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState {
            cover_style: PRISM_STYLE,
            cover_title: "KESEJAHTERAAN RAKYAT".into(),
            cover_title2: "DALAM ANGKA".into(),
            ..PubgenState::default()
        };
        let res = execute_generate_cover(&mut app, &st);
        assert!(res.is_ok(), "Prism Diamonds must apply its template: {res:?}");

        let active = app.session.active().unwrap();
        // The template brings its own midnight and jewel swatches.
        for name in ["Prism Night", "Prism Deep", "Prism Blue", "Prism Green", "Prism Orange", "Prism Mint", "Prism Ice", "Prism Sky"] {
            assert!(active.doc.swatches.iter().any(|s| s.name == name), "swatch `{name}` comes with the template");
        }
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("KESEJAHTERAAN RAKYAT")), "the headline is bound: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("DALAM ANGKA")), "the second headline line is bound: {texts:?}");
        // The photograph is cut as a diamond, not boxed in a rectangle.
        let hero = active
            .doc
            .spreads
            .iter()
            .flat_map(|s| s.items.iter())
            .find(|it| it.label == "#slot:image:hero")
            .expect("the hero photograph frame comes with the template");
        assert_eq!(hero.path.subpaths.first().map(|p| p.anchors.len()), Some(4), "the hero frame is a diamond");
    }

    #[test]
    fn test_cover_style_12_uses_the_quarter_stack_template() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState {
            cover_style: QUARTER_STYLE,
            cover_title: "KESEJAHTERAAN RAKYAT".into(),
            cover_title2: "DALAM ANGKA".into(),
            ..PubgenState::default()
        };
        let res = execute_generate_cover(&mut app, &st);
        assert!(res.is_ok(), "Quarter Stack must apply its template: {res:?}");

        let active = app.session.active().unwrap();
        // The template brings its own warm palette.
        for name in ["Stack Paper", "Stack Ink", "Stack Clay", "Stack Marigold", "Stack Sage", "Stack Blush", "Stack Teal"] {
            assert!(active.doc.swatches.iter().any(|s| s.name == name), "swatch `{name}` comes with the template");
        }
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("KESEJAHTERAAN RAKYAT")), "the headline is bound: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("DALAM ANGKA")), "the second headline line is bound: {texts:?}");
        // The photograph sits in a card with exactly one rounded corner.
        let hero = active
            .doc
            .spreads
            .iter()
            .flat_map(|s| s.items.iter())
            .find(|it| it.label == "#slot:image:hero")
            .expect("the hero photograph frame comes with the template");
        let rounded = hero.corners.corners.iter().filter(|c| c.shape != designcraft_geom::corners::CornerShape::None).count();
        assert_eq!(rounded, 1, "the hero card rounds one corner only: {:?}", hero.corners.corners);
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
            let target = dividers_dir.join("001_simple_chapter.designcraft");
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
    fn test_reordered_csv_columns_resolve_by_header() {
        // Columns are found by header name, not fixed position.
        let headers: Vec<String> = ["type", "number", "title", "template", "file", "title_en", "file_en"].into_iter().map(String::from).collect();
        assert_eq!(effective_col(3, &headers, &["file", "filename"], 3), 4, "'file' found past the legacy slot");
        assert_eq!(effective_col(4, &headers, &["template"], 4), 3, "'template' found before the legacy slot");
        // Records set directly (no load_card1_csv): stale default positions re-resolve by name.
        let headers6: Vec<String> = ["type", "number", "title", "file", "template", "file_en"].into_iter().map(String::from).collect();
        assert_eq!(effective_col(6, &headers6, &["file_en", "fileen"], 6), 5, "stale default re-resolves");
        assert_eq!(effective_col(4, &headers6, &["template"], 4), 4, "matching stored column is kept");
        // Missing header reads as absent: MAX (row.get(MAX) is None, i.e. empty text —
        // identical behaviour to reading past the end of the row).
        let headers4: Vec<String> = ["type", "number", "title", "file"].into_iter().map(String::from).collect();
        assert_eq!(effective_col(6, &headers4, &["file_en", "fileen"], 6), usize::MAX);
    }

    #[test]
    fn test_row_template_slots_divider_and_table() {
        let st = PubgenState { bilingual_output: true, ..Default::default() };

        // Divider in Dual Lang: chapter parts + title_en, no table slots.
        let s = row_template_slots("Divider", "02", "BAB 02 PEMERINTAHAN", "CHAPTER 02 GOVERNMENT", "", false, &st);
        assert_eq!(s.text.get("first_word"), Some(&json!("BAB")));
        assert_eq!(s.text.get("chapter_num"), Some(&json!("02")));
        assert_eq!(s.text.get("chapter_title"), Some(&json!("PEMERINTAHAN")));
        assert_eq!(s.text.get("title_en"), Some(&json!("CHAPTER 02 GOVERNMENT")));
        assert!(s.story.is_empty());
        assert_eq!(s.images_resolved + s.rtfs_resolved, 0, "no files on disk to resolve");

        // Divider preface never carries title_en.
        let s = row_template_slots("Divider", "01", "BAB 01 X", "EN X", "", true, &st);
        assert!(s.text.get("title_en").is_none());

        // Table single lang: localized title, no EN slots.
        let single = PubgenState::default();
        let s = row_template_slots("Tabel", "02.01", "Luas Wilayah", "", "", false, &single);
        assert!(s.text.contains_key("table_num"));
        assert_eq!(s.text.get("table_kind_en"), Some(&json!("")));
        assert_eq!(s.text.get("table_title_en"), Some(&json!("")));

        // Table with an EN title: bilingual slots.
        let s = row_template_slots("Tabel", "02.01", "Luas Wilayah", "Area", "", false, &st);
        assert_eq!(s.text.get("table_title"), Some(&json!("Luas Wilayah")));
        assert_eq!(s.text.get("table_title_en"), Some(&json!("Area")));
        assert!(s.text.get("table_kind_en").is_some());
    }

    #[test]
    fn test_effective_col_skips_foreign_fallback() {
        // User's catalog: 7 columns where image1/image2 sit at the legacy
        // title_en/file_en positions. file_en must read as ABSENT, not as image2.
        let headers: Vec<String> = ["type", "number", "title", "file", "template", "image1", "image2"].into_iter().map(String::from).collect();
        assert_eq!(effective_col(6, &headers, &["file_en", "fileen"], 6), usize::MAX);
        assert_eq!(effective_col(5, &headers, &["title_en", "titleen"], 5), usize::MAX);
        // file is found by name at its real position.
        assert_eq!(effective_col(3, &headers, &["file", "filename"], 3), 3);
        // Legacy 7-column layout keeps working.
        let legacy: Vec<String> = ["type", "number", "title", "file", "template", "title_en", "file_en"].into_iter().map(String::from).collect();
        assert_eq!(effective_col(5, &legacy, &["title_en", "titleen"], 5), 5);
        assert_eq!(effective_col(6, &legacy, &["file_en", "fileen"], 6), 6);
    }

    #[test]
    fn test_find_named_column_mapping() {
        let headers = vec!["No".into(), "Judul".into(), "Berkas".into(), "File".into(), "FILE_EN".into()];
        assert_eq!(find_named_col(&headers, &["file", "filename"], 3), 3);
        assert_eq!(find_named_col(&headers, &["file_en", "fileen"], 6), 4);
        assert_eq!(find_named_col(&headers, &["title_en", "titleen"], 5), 5, "missing header keeps legacy position");
        // 'file' must not swallow 'file_en'
        let only_en = vec!["file_en".to_string()];
        assert_eq!(find_named_col(&only_en, &["file", "filename"], 3), 3);
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
        let r1 = resolve_template_path_spec("", "001_simple_chapter");
        assert!(r1.is_some(), "Should resolve 001_simple_chapter: {r1:?}");
        assert!(r1.unwrap().contains("001_simple_chapter.designcraft"));

        // Subcategory path resolution
        let r2 = resolve_template_path_spec("", "dividers/001_simple_chapter");
        assert!(r2.is_some(), "Should resolve dividers/001_simple_chapter: {r2:?}");

        // Batik infographic divider resolution
        let r_batik = resolve_template_path_spec("", "002_batik_infographic");
        assert!(r_batik.is_some(), "Should resolve 002_batik_infographic: {r_batik:?}");
        assert!(r_batik.unwrap().contains("002_batik_infographic.designcraft"));

        // Legacy / alias resolution
        let r_legacy1 = resolve_template_path_spec("", "dividers/001_ulasan_chapter");
        assert!(r_legacy1.is_some(), "Should resolve legacy ulasan: {r_legacy1:?}");
        assert!(r_legacy1.unwrap().contains("001_simple_chapter.designcraft"));

        let r_legacy2 = resolve_template_path_spec("", "dividers/special_infographic_divider");
        assert!(r_legacy2.is_some(), "Should resolve special infographic alias: {r_legacy2:?}");
        assert!(r_legacy2.unwrap().contains("002_batik_infographic.designcraft"));

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
    fn test_ulasan_row_with_erroneous_divider_template_places_rtf() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("ch1.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Narasi bab 1 geografi dan iklim.}"#).unwrap();

        // Row has type "Ulasan", but template explicitly says "dividers/special_infographic_divider"
        // (e.g. from an Excel sheet where the user specified a divider style).
        // It MUST NOT render as a Batik Infographic divider, and MUST place the RTF text!
        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "BAB 01 Geografi".into(),
                rtf_path.to_string_lossy().to_string(),
                "dividers/special_infographic_divider".into(),
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
        assert!(res.is_ok(), "execute_generate_pages should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("Narasi bab 1 geografi")), "Ulasan RTF must be placed into the document: {texts:?}");
    }

    #[test]
    fn test_procedural_divider_art_fallback() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Simulate DIV sheet row without template on disk (e.g. portable app release)
        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec![
                "Divider".into(),
                "01".into(),
                "BAB 01 Geografi".into(),
                "".into(),
                "dividers/special_infographic_divider".into(),
                "Chapter 01 Geography".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "title_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_title_en_col: 5,
            card1_range_all: true,
            bilingual_output: true,
            cover_color_primary: "#0284c7".into(),
            cover_color_accent: "#f59e0b".into(),
            cover_color_dark: "#1e293b".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages should generate procedural divider: {res:?}");

        let active = app.session.active().unwrap();
        // Page 1 is initial, Page 2 is procedural divider
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("01")), "Stories should contain chapter num 01: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("BAB")), "Stories should contain prefix BAB: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Geografi")), "Stories should contain title Geografi: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Chapter 01 Geography")), "Stories should contain title_en: {texts:?}");
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
    fn test_execute_add_toc_insert_after_selected_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        // Place a styled title on Page 2 (0-based index 1)
        let para = designcraft_doc::ParaFormat { style: "TOC Chapter".to_string(), ..Default::default() };
        let _ = app.session.edit(|doc, _| {
            doc.add_text_frame(
                designcraft_doc::SpreadRef::Doc(1),
                designcraft_geom::Rect::new(36.0, 36.0, 200.0, 100.0),
                designcraft_doc::LayerId(1),
                "Chapter 1 Overview",
                para,
            )?;
            Ok(())
        });

        let mut st = PubgenState::default();
        st.toc_style = "TOC Chapter".to_string();
        st.toc_insert_after = true;

        // Select Page 1 (0-based index 0)
        app.session.active_mut().unwrap().selection.page = Some(0);

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        // The document should now have 3 pages:
        // Page 1: original Page 1
        // Page 2: newly inserted blank page with the TOC
        // Page 3: the chapter page (shifted from index 1 to index 2)
        let doc_pages = app.session.active().unwrap().doc.page_count();
        assert_eq!(doc_pages, 3);

        // Verify the TOC is on Page 2 (0-based index 1)
        let toc_story_id = toc_story_on_page(&app, 1).unwrap();
        assert_eq!(toc_story_page(&app, toc_story_id), Some(1));
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
    fn test_ai_translated_body_drives_the_en_spread_without_file_en() {
        // The user's report: reviews-only AI run reaches 100%, but the R01 page shows
        // Indonesian only — the English spread never appears. With the translations map
        // filled (as the worker leaves it) and no file_en, the spread must still come.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let dir = std::env::temp_dir().join(format!("designcraft_pubgen_ai_ulasan_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let id_rtf = dir.join("R01_id.rtf");
        std::fs::write(&id_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Ulasan wilayah Indonesia."#).unwrap();

        let mut st = PubgenState {
            card1_preset: 6,
            bilingual_output: true,
            translate_with_ai: true,
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "BAB 01 Geografi".into(),
                id_rtf.to_string_lossy().to_string(),
                "".into(),
                "".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "file_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            translations: HashMap::from([("Ulasan wilayah Indonesia.".to_string(), "Indonesian region review.".to_string())]),
            ..Default::default()
        };

        // The temp RTF the panel writes from the translated paragraphs.
        let en_rtf = ai::translated_body_rtf(&st, id_rtf.to_string_lossy().as_ref());
        assert!(en_rtf.is_some(), "translated body RTF should be written from the map");
        let en_rtf = en_rtf.expect("checked");
        let en_text = std::fs::read_to_string(&en_rtf).expect("temp rtf readable");
        assert!(en_text.contains("Indonesian region review"), "{en_text}");

        // Point the record's file_en column at the temp RTF, exactly as the spread gate does.
        st.card1_records[0][5] = en_rtf.to_string_lossy().to_string();

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "generate should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 3, "ID spread + EN spread expected");
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("Indonesian region review")), "EN body placed: {texts:?}");

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&en_rtf);
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

        // 3. User scenario: title2 in template already contains the English title (test across all 13 styles)
        for style in 0..13 {
            let mut app_s = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app_s.run("file.new", json!({})).unwrap();
            let st_s = PubgenState {
                cover_style: style,
                cover_title: "KECAMATAN KARAWANG".into(),
                cover_title2: "EAST KARAWANG DISTRICT IN FIGURES".into(),
                cover_title_en: "EAST KARAWANG DISTRICT IN FIGURES".into(),
                bilingual_output: true,
                ..Default::default()
            };
            let res_s = execute_generate_cover(&mut app_s, &st_s);
            assert!(res_s.is_ok(), "Style {style} should succeed: {res_s:?}");
            let active_s = app_s.session.active().unwrap();
            let texts_s: Vec<String> = active_s.doc.stories.values().map(|s| s.text.clone()).collect();
            let count_en = texts_s.iter().filter(|t| t.contains("EAST KARAWANG DISTRICT IN FIGURES")).count();
            assert_eq!(count_en, 1, "Style {style}: English title must appear exactly once, got {count_en}: {texts_s:?}");
            let en_story = active_s.doc.stories.values().find(|s| s.text.contains("EAST KARAWANG DISTRICT IN FIGURES")).unwrap();
            let has_bold_italic = en_story.chars.iter().any(|r| r.format.over.font_style.as_deref() == Some("Bold Italic"));
            assert!(has_bold_italic, "Style {style}: English title runs must be Bold Italic: {:?}", en_story.chars);
        }
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

    #[test]
    fn test_table_sheet_note_and_source_rows_extracted_to_textbox() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let matrix = vec![
            vec!["Kecamatan".into(), "Luas".into()],
            vec!["Karawang Timur".into(), "35.2".into()],
            vec!["Catatan".into(), ":".into(), "Desa pada tabel ini termasuk UPT".into()],
            vec!["Note".into(), ":".into(), "Villages in this table include UPT".into()],
            vec!["Sumber".into(), ":".into(), "Badan Pusat Statistik".into()],
            vec!["Source".into(), ":".into(), "BPS-Statistics Indonesia".into()],
        ];

        let st = PubgenState { card2_placement: 1, card2_col_numbers: false, ..Default::default() };
        let res = execute_insert_table_matrix(&mut app, &st, Some(&matrix), Some(0));
        assert!(res.is_ok(), "execute_insert_table_matrix failed: {:?}", res.err());
        let msg = res.unwrap();
        assert!(msg.contains("+ notes"), "Must indicate + notes: {msg}");

        let doc = &app.session.active().unwrap().doc;
        let spread = doc.spreads.first().unwrap();
        // The spread must have the table frame AND the note textbox frame!
        assert_eq!(spread.items.len(), 2, "Must create 2 frames: table frame and note textbox frame");

        // First frame: table frame. The table must have ONLY 2 rows (header + 1 data row)! The 4 note rows are NOT in the table!
        let table_item = &spread.items[0];
        let table_tf = table_item.text_frame().unwrap();
        let table_story = doc.story(table_tf.story).unwrap();
        let table_id = *table_story.tables.keys().next().unwrap();
        let table = table_story.tables.get(&table_id).unwrap();
        assert_eq!(table.nrows(), 2, "Table must have 2 rows, notes must be excluded from table");

        // Second frame: note textbox frame below the table.
        let note_item = &spread.items[1];
        let note_tf = note_item.text_frame().unwrap();
        let note_story = doc.story(note_tf.story).unwrap();
        assert_eq!(note_story.paras.len(), 4, "Note textbox must have 4 paragraphs");

        // Check hanging indent and tab stops on paragraphs:
        for p in &note_story.paras {
            assert!(p.para.left_indent.is_some(), "Must have left_indent");
            assert!(p.para.first_line_indent.is_some(), "Must have negative first_line_indent");
            assert_eq!(p.para.first_line_indent.unwrap(), -p.para.left_indent.unwrap());
            let tabs = p.para.tabs.as_ref().unwrap();
            assert_eq!(tabs.len(), 2, "Must have 2 tab stops: colon and text");
            assert_eq!(tabs[0].position, 46.0, "Colon tab position at 46.0");
            assert_eq!(tabs[1].position, 54.0, "Text tab position at 54.0");
        }

        // Check Italic formatting:
        // Paras: 0=Catatan (Regular), 1=Note (Italic), 2=Sumber (Regular), 3=Source (Italic)
        let ranges = note_story.para_ranges();
        let get_style_at = |pos: usize| note_story.format_after(pos).over.font_style.clone();
        assert_eq!(get_style_at(ranges[0].start), Some("Regular".to_string()));
        assert_eq!(get_style_at(ranges[1].start), Some("Italic".to_string()));
        assert_eq!(get_style_at(ranges[2].start), Some("Regular".to_string()));
        assert_eq!(get_style_at(ranges[3].start), Some("Italic".to_string()));

        // Check textbox height adjusted:
        let note_bounds = note_item.bounds();
        let table_bounds = table_item.bounds();
        assert!(note_bounds.y0 >= table_bounds.y1, "Note textbox must be placed below table");
        assert!(note_bounds.height() >= 18.0 && note_bounds.height() < 120.0, "Textbox height must be fitted to content");
    }

    #[test]
    fn test_cover_include_inside_and_back_cover_generation() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            cover_style: 0,
            cover_color_primary: "#1e3a8a".into(),
            cover_color_accent: "#d97706".into(),
            cover_image_skyline: "plan/pubgen/samples/images/bps_logo.png".into(),
            cover_include_inside_back: true,
            cover_image_back: "plan/pubgen/samples/images/se2026.png".into(),
            cover_office_address: "Jl. Cakradireja No.36 Karawang 41312\nJawa Barat".into(),
            cover_code_type: 0,
            cover_code_num: "2807-6302".into(),
            ..Default::default()
        };

        let res = execute_generate_cover(&mut app, &st);
        assert!(res.is_ok(), "Cover generation with inside and back cover should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // Document should now have 4 pages: Front Cover, Inside Cover, initial page, Back Cover
        assert_eq!(active.doc.page_count(), 4, "Expected 4 pages (Front, Inside, initial page, Back)");
        assert_eq!(find_back_cover_page(&active.doc), Some(3), "Back cover must be the last page (page 3)");

        // Spread 0: Front Cover
        let sp0 = &active.doc.spreads[0];
        assert_eq!(sp0.pages[0].margins, designcraft_doc::Margins::uniform(0.0));

        // Spread 1: Inside Cover
        let sp1 = &active.doc.spreads[1];
        assert_eq!(sp1.pages[0].margins, designcraft_doc::Margins::uniform(0.0));
        // All unassigned vector shapes on Inside Cover must have Fill [Paper] and Black stroke
        for it in &sp1.items {
            if matches!(it.content, designcraft_doc::Content::Unassigned) {
                assert_eq!(it.fill.swatch, "[Paper]", "Inside cover shape must have white fill: {:?}", it.id);
                assert_eq!(it.stroke.swatch, designcraft_color::swatch::BLACK, "Inside cover shape must have black stroke");
                assert_eq!(it.stroke.weight, 1.0);
            }
        }

        // Last Spread: Back Cover
        let sp_back = active.doc.spreads.last().unwrap();
        assert_eq!(sp_back.pages[0].margins, designcraft_doc::Margins::uniform(0.0));

        // All items on Back Cover must have locked = true
        for it in &sp_back.items {
            assert!(it.locked, "Item {:?} on Back Cover must be locked", it.id);
        }

        fn has_white_unassigned(it: &designcraft_doc::Item) -> bool {
            if matches!(it.content, designcraft_doc::Content::Unassigned) && it.fill.swatch == "[Paper]" {
                return true;
            }
            if let designcraft_doc::Content::Group { items } = &it.content {
                return items.iter().any(|sub| has_white_unassigned(sub));
            }
            false
        }

        // Shapes that had image background (hero frame) must be converted to white fill ([Paper])
        let has_white_shape = sp_back.items.iter().any(|it| has_white_unassigned(it));
        assert!(has_white_shape, "Back cover must have white shape converted from image background");

        // Must contain graphic frames (top-right stack, center slogan, camera placeholder, office logo)
        let graphic_frames_count = sp_back.items.iter().filter(|it| matches!(it.content, designcraft_doc::Content::Graphic(_))).count();
        assert!(graphic_frames_count >= 4, "Back cover must have at least 4 graphic frames (got {graphic_frames_count})");

        // Must contain text frames for agency name, office address, and ISSN/ISBN
        let text_frames_count = sp_back.items.iter().filter(|it| matches!(it.content, designcraft_doc::Content::Text(_))).count();
        assert!(text_frames_count >= 3, "Back cover must have at least 3 text frames (agency, address, ISSN)");

        let ph = sp_back.pages[0].height;
        let pw = sp_back.pages[0].width;

        // Verify Office Logo position at bottom left:
        let logo_item = sp_back.items.iter().find(|it| it.label == "#slot:back_cover:logo").expect("Logo item on back cover");
        assert!(logo_item.bounds().y0 >= ph - 110.0, "Logo must be at bottom left (y0: {})", logo_item.bounds().y0);
        assert!((logo_item.bounds().x0 - 36.0).abs() < 1.0, "Logo x0 must be at margin 36");

        // Verify Office Name position at bottom left beside logo:
        let agency_item = sp_back.items.iter().find(|it| it.label == "#slot:back_cover:agency").expect("Agency item on back cover");
        assert!(agency_item.bounds().y0 >= ph - 110.0, "Agency name must be at bottom left (y0: {})", agency_item.bounds().y0);
        assert!(agency_item.bounds().x0 >= 80.0, "Agency name x0 must be beside logo");

        // Verify Office Address position below logo & name on bottom left:
        let addr_item = sp_back.items.iter().find(|it| it.label == "#slot:back_cover:address").expect("Address item on back cover");
        assert!(addr_item.bounds().y0 >= ph - 65.0, "Address must be below logo and name (y0: {})", addr_item.bounds().y0);
        assert!((addr_item.bounds().x0 - 36.0).abs() < 1.0, "Address x0 must be at margin 36");
        if let designcraft_doc::Content::Text(tf) = &addr_item.content {
            let story = active.doc.story(tf.story).expect("Address story");
            for r in &story.chars {
                assert_eq!(r.format.over.font_style.as_deref(), Some("Regular"), "Address must not be bold");
                assert!(r.format.over.size.unwrap_or(0.0) <= 8.0, "Address must use standard font size");
            }
        }

        // Verify ISSN/ISBN position at bottom right side:
        let code_item = sp_back.items.iter().find(|it| it.label == "#slot:back_cover:code").expect("Code item on back cover");
        assert!(code_item.bounds().x0 >= pw - 180.0, "ISSN must be on bottom right side (x0: {})", code_item.bounds().x0);
        assert!(code_item.bounds().y0 >= ph - 65.0, "ISSN must be on bottom right side (y0: {})", code_item.bounds().y0);
        if let designcraft_doc::Content::Text(tf) = &code_item.content {
            let story = active.doc.story(tf.story).expect("ISSN story");
            for r in &story.chars {
                assert_eq!(r.format.over.font_style.as_deref(), Some("Regular"), "ISSN must not be bold");
                assert!(r.format.over.size.unwrap_or(0.0) <= 8.0, "ISSN must use standard font size");
            }
            for p in &story.paras {
                assert_eq!(p.para.align, Some(designcraft_doc::Align::Right), "ISSN must be right-aligned");
            }
        }

        // Test that adding body pages afterwards still keeps Back Cover at the very end
        app.run("layout.pages.insert", json!({"count": 2})).unwrap();
        ensure_back_cover_at_end(&mut app);
        let active2 = app.session.active().unwrap();
        assert_eq!(active2.doc.page_count(), 6, "Now 6 pages total");
        assert_eq!(find_back_cover_page(&active2.doc), Some(5), "Back cover must move to the last page (page 5)");
    }

    #[test]
    fn test_excel_with_cover_generates_table_sheet_page() {
        let cover_sheet = excel::ExcelSheetData {
            name: "COVER".into(),
            grid: vec![
                vec!["style".into(), "Wave Curves".into()],
                vec!["title1".into(), "KECAMATAN KARAWANG".into()],
                vec!["office_address".into(), "Jl. Cakradireja No.36".into()],
            ],
            merges: vec![],
        };
        let tbl_sheet = excel::ExcelSheetData {
            name: "01_01".into(),
            grid: vec![vec!["Tabel 01.01 Luas Wilayah".into()], vec!["Desa".into(), "Luas".into()], vec!["Adiarsa".into(), "10".into()]],
            merges: vec![],
        };
        let res = excel::convert_excel_to_csv_data(&[cover_sheet, tbl_sheet], "TEST_CONV");
        assert!(res.cover.is_some());
        assert_eq!(res.table_csvs.len(), 1);

        let dir = std::env::temp_dir().join(format!("dc-pubgen-cover-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let cat_path = dir.join("catalog.csv");
        std::fs::write(&cat_path, &res.catalog_csv_content).unwrap();

        let tbl_path = dir.join("01_01.csv");
        std::fs::write(&tbl_path, &res.table_csvs[0].1).unwrap();

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState::default();
        apply_excel_cover(&mut st, res.cover.as_ref().unwrap(), None);
        st.card1_path = cat_path.to_str().unwrap().to_string();
        load_card1_csv(&mut st, cat_path.to_str().unwrap());
        st.card2_sheet_tables = vec![tbl_path.to_str().unwrap().to_string()];
        load_card2_csv(&mut st, tbl_path.to_str().unwrap());

        st.excel_had_cover = true;
        st.card1_auto_insert_tables = true;
        st.card1_range_all = true;

        if st.excel_had_cover {
            match execute_generate_front_cover(&mut app, &st) {
                Ok(_) => {
                    if st.cover_include_inside_back {
                        execute_generate_inside_cover(&mut app).unwrap();
                    }
                }
                Err(e) => panic!("Cover failed: {e}"),
            }
        }

        let gen_res = execute_generate_pages(&mut app, &st);
        assert!(gen_res.is_ok(), "generate pages failed: {gen_res:?}");

        // Verify that the table sheet (01_01) was generated before adding back cover
        {
            let doc = &app.session.active().unwrap().doc;
            let has_table = doc.stories.values().any(|s| s.text.contains("01.01") || s.text.contains("Luas Wilayah"));
            assert!(has_table, "Table sheet 01_01 should have been generated into the document");
        }

        if st.excel_had_cover && st.cover_include_inside_back {
            execute_generate_back_cover(&mut app, &st).unwrap();
        }
        ensure_back_cover_at_end(&mut app);

        // Verify that the table sheet (01_01) was NOT deleted or corrupted by back cover generation
        let doc = &app.session.active().unwrap().doc;
        let has_table_after = doc.stories.values().any(|s| s.text.contains("01.01") || s.text.contains("Luas Wilayah"));
        assert!(has_table_after, "Table sheet 01_01 must still be present after generating back cover");

        // Verify back cover page exists at the end
        let back_page_idx = doc.page_count() - 1;
        let (back_si, _) = doc.page_loc(back_page_idx).unwrap();
        let back_spread = &doc.spreads[back_si];
        let has_back_branding = back_spread.items.iter().any(|it| {
            if let designcraft_doc::Content::Text(tf) = &it.content {
                doc.story(tf.story).is_some_and(|s| s.text.contains("BADAN PUSAT STATISTIK"))
            } else {
                false
            }
        });
        assert!(has_back_branding, "Back cover must contain agency text branding");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_table_page_ai_translation_end_to_end() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Create Title frame on page 0
        let res_title = app
            .run(
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
        let _title_fid = res_title["id"].as_u64().unwrap();

        // 2. Insert table with note rows on page 0
        let matrix = vec![
            vec!["Desa/Kelurahan".into(), "Luas (km2)".into(), "Persentase".into()],
            vec!["Warungbambu".into(), "1.65".into(), "21.32".into()],
            vec!["Jumlah".into(), "1.65".into(), "21.32".into()],
            vec!["Keterangan".into(), ":".into(), "-".into()],
            vec!["Sumber".into(), ":".into(), "BPS Kabupaten Karawang/Dinas terkait".into()],
        ];
        let st = PubgenState { card2_placement: 1, card2_first_row_header: true, card2_table_footer: true, ..Default::default() };
        let ins_res = execute_insert_table_matrix(&mut app, &st, Some(&matrix), Some(0));
        assert!(ins_res.is_ok(), "Table insert should succeed: {ins_res:?}");

        // 3. Detect table page target
        let target_opt = ai::detect_table_page_target(&app, 0);
        assert!(target_opt.is_some(), "Should detect TablePageTarget on page 0");
        let target = target_opt.unwrap();
        assert!(target.title_fid.is_some(), "Must detect title frame");
        assert!(target.notes_fid.is_some(), "Must detect notes frame");

        // 4. Collect units
        let units = ai::collect_table_page_units(&app, &target);
        let texts: Vec<&str> = units.iter().map(|(k, _)| k.as_str()).collect();
        assert!(
            texts.contains(&"Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025"),
            "Units must contain clean title: {texts:?}"
        );
        assert!(texts.contains(&"Desa/Kelurahan"), "Units must contain header cell: {texts:?}");
        assert!(texts.contains(&"Luas (km2)"), "Units must contain header cell: {texts:?}");
        assert!(texts.contains(&"Warungbambu"), "Units must contain data label: {texts:?}");
        assert!(texts.contains(&"BPS Kabupaten Karawang/Dinas terkait"), "Units must contain source text: {texts:?}");
        // Numbers and '-' must NEVER be collected
        assert!(!texts.contains(&"1.65"), "Must not contain numbers");
        assert!(!texts.contains(&"21.32"), "Must not contain numbers");
        assert!(!texts.contains(&"-"), "Must not contain dash");

        // 5. Populate translations map
        let mut translate_st = PubgenState::default();
        translate_st.translations.insert(
            "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025".into(),
            "Area by Village/Urban Village in Karawang Timur Subdistrict, 2025".into(),
        );
        translate_st.translations.insert("Desa/Kelurahan".into(), "Village/Urban Village".into());
        translate_st.translations.insert("Luas (km2)".into(), "Area (sq km)".into());
        translate_st.translations.insert("Persentase".into(), "Percentage".into());
        translate_st.translations.insert("Warungbambu".into(), "Warungbambu".into());
        translate_st
            .translations
            .insert("BPS Kabupaten Karawang/Dinas terkait".into(), "Statistics of Karawang Regency Office/Relevant Agency".into());

        // 6. Apply table page result
        let replaced = ai::apply_table_page_result(&mut app, &translate_st, &target);
        assert!(replaced > 0, "Must replace items: {replaced}");

        // 7. Verify Title: Line 1 Regular, Line 2 Italic
        let doc = &app.session.active().unwrap().doc;
        let title_it = doc.item(designcraft_doc::ItemId(target.title_fid.unwrap())).unwrap();
        let title_story = doc.story(title_it.text_frame().unwrap().story).unwrap();
        assert!(title_story.text.contains("Tabel 01.01 Luas Daerah"));
        assert!(title_story.text.contains("Table 01.01 Area by Village/Urban Village"));
        let title_lines: Vec<&str> = title_story.text.lines().collect();
        assert_eq!(title_lines.len(), 2, "Title must have 2 lines");
        let line2_pos = title_lines[0].len() + 1;
        assert_eq!(title_story.format_after(line2_pos).over.font_style.as_deref(), Some("Italic"));

        // 8. Verify Table cells: in same cell new line [italic]
        let table_story = doc.story(target.table_sid).unwrap();
        let table = table_story.tables.get(&target.table_id).unwrap();
        let header_cell = table.cell(0, 0).unwrap();
        assert!(header_cell.text.text.contains("Desa/Kelurahan\nVillage/Urban Village"));
        let en_pos = "Desa/Kelurahan\n".len();
        assert_eq!(header_cell.text.format_after(en_pos).over.font_style.as_deref(), Some("Italic"));

        let col_num_cell = table.cell(1, 0).unwrap();
        assert_eq!(col_num_cell.text.text.trim(), "(1)", "Column numbers row must be untouched");

        let jumlah_cell = table.cell(3, 0).unwrap();
        assert!(jumlah_cell.text.text.contains("Jumlah\nTotal"));
        let total_pos = "Jumlah\n".len();
        assert_eq!(jumlah_cell.text.format_after(total_pos).over.font_style.as_deref(), Some("Italic"));

        let num_cell = table.cell(2, 1).unwrap();
        assert_eq!(num_cell.text.text.trim(), "1.65", "Numeric cell must remain untouched");

        // 9. Verify Notes:
        // Keterangan : -
        // [italic]Note: -
        // Sumber: BPS Kabupaten Karawang/Dinas terkait
        // [italic]Source : Statistics of Karawang Regency Office/Relevant Agency
        let note_it = doc.item(designcraft_doc::ItemId(target.notes_fid.unwrap())).unwrap();
        let note_story = doc.story(note_it.text_frame().unwrap().story).unwrap();
        let note_lines: Vec<&str> = note_story.text.lines().collect();
        assert_eq!(note_lines.len(), 4, "Must have 4 note lines: {note_lines:?}");
        assert!(note_lines[0].starts_with("Keterangan"));
        assert!(note_lines[1].starts_with("Note"));
        assert!(note_lines[2].starts_with("Sumber"));
        assert!(note_lines[3].starts_with("Source"));

        let note_ranges = note_story.para_ranges();
        assert_ne!(note_story.format_after(note_ranges[0].start).over.font_style.as_deref(), Some("Italic"));
        assert_eq!(note_story.format_after(note_ranges[1].start).over.font_style.as_deref(), Some("Italic"));
        assert_ne!(note_story.format_after(note_ranges[2].start).over.font_style.as_deref(), Some("Italic"));
        assert_eq!(note_story.format_after(note_ranges[3].start).over.font_style.as_deref(), Some("Italic"));
    }
}
