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
mod testutil;
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
pub(crate) use state::{default_cover_title, default_cover_title2};
pub(crate) use tables::*;
pub(crate) use themes::*;
pub(crate) use thumbnails::*;
pub(crate) use toc::*;
pub(crate) use widgets::*;

use cover_art::CoverArtVals;
use divider_art::DividerArtVals;

const PANEL_ID: &str = "pubgen_panel_state";

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
