//! Modal dialog popup windows for the Pubgen menu system.
//!
//! Provides unified floating modal dialogs for each Pubgen capability,
//! matching the reference "Translate with AI" window design and centered
//! in the container.

use egui::{Align2, RichText};

use crate::DesignApp;
use crate::theme::Tokens;

use super::*;

/// Standard unified popup window shell matching "Translate with AI":
/// Native egui::Window, centered in container, non-collapsible,
/// with standard title bar close button (✕), vertical scroll area,
/// and bottom Close button.
fn pubgen_dialog_modal(
    ctx: &egui::Context,
    title: &str,
    open: &mut bool,
    default_width: f32,
    max_height: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let mut is_open = *open;
    let mut close_clicked = false;
    let available_h = (ctx.content_rect().height() - 90.0).max(200.0);
    let scroll_h = max_height.min(available_h);

    egui::Window::new(RichText::new(title).size(15.0))
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .min_width(default_width)
        .max_width(default_width)
        .default_width(default_width)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .show(ctx, |ui| {
            ui.set_width(default_width);
            ui.set_max_width(default_width);
            let content_w = default_width - 24.0;
            egui::ScrollArea::vertical().max_width(default_width).max_height(scroll_h).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(content_w);
                ui.set_max_width(content_w);
                add_contents(ui);
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    close_clicked = true;
                }
            });
        });

    *open = is_open && !close_clicked;
}

/// Modal dialog for Master Excel import, sheet parsing, and 1-click batch generation.
pub fn dialog_excel_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_excel_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Master Excel (.xlsx) Import & Generate"), &mut open, 480.0, 520.0, |ui| {
        master_excel_ui(app, ui, st, &t);
    });
    st.dialog_excel_open = open;
}

/// Modal dialog for Table of Contents generation and options.
pub fn dialog_toc_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_toc_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Table of Contents (TOC) Options"), &mut open, 460.0, 520.0, |ui| {
        card_toc_ui(app, ui, st, &t);
    });
    st.dialog_toc_open = open;
}

/// Modal dialog for Publication Cover styling, palette presets, and overrides.
pub fn dialog_cover_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_cover_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Publication Cover Generator"), &mut open, 540.0, 880.0, |ui| {
        card_cover_body_ui(app, ui, st, &t);
    });
    st.dialog_cover_open = open;
}

/// Modal dialog for Header & Footer templates.
pub fn dialog_hf_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_hf_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Header & Footer Templates"), &mut open, 480.0, 540.0, |ui| {
        card_header_footer_body_ui(app, ui, st, &t);
    });
    st.dialog_hf_open = open;
}

/// Modal dialog for Manual CSV and Table Ingest (Card 1 / Card 2).
pub fn dialog_manual_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_manual_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Manual CSV & Table Ingest"), &mut open, 520.0, 580.0, |ui| {
        ui.horizontal(|ui| {
            let tab1_label = crate::i18n::tr(&app.ui.language, "Page from CSV");
            let tab2_label = crate::i18n::tr(&app.ui.language, "Table from CSV");
            if ui.selectable_label(st.dialog_manual_tab == 0, tab1_label).clicked() {
                st.dialog_manual_tab = 0;
            }
            if ui.selectable_label(st.dialog_manual_tab == 1, tab2_label).clicked() {
                st.dialog_manual_tab = 1;
            }
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        if st.dialog_manual_tab == 0 {
            card1_body_ui(app, ui, st, &t);
        } else {
            card2_body_ui(app, ui, st, &t);
        }
    });
    st.dialog_manual_open = open;
}

/// Modal dialog for Page Margins, Facing Pages, and Renumbering Utilities.
pub fn dialog_utils_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_utils_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Page Margins & Renumbering Utilities"), &mut open, 420.0, 440.0, |ui| {
        page_utilities_ui(app, ui, st, &t);
    });
    st.dialog_utils_open = open;
}

/// Renders all active Pubgen popup dialogs and modal windows for the frame.
/// Modal dialog for Statistical Vector Chart Options.
pub fn dialog_chart_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if st.chart_dialog.is_none() {
        return;
    }
    charts::dialog::chart_dialog_ui(app, ctx, st);
}

/// Modal dialog for batch translating Table pages across a range.
pub fn dialog_table_range_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.show_table_range_popup {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    let total_pages = app.session.active().map(|s| s.doc.page_count()).unwrap_or(0);
    if st.ai_table_range_to == 0 || st.ai_table_range_to > total_pages {
        st.ai_table_range_to = total_pages.max(1);
    }
    if st.ai_table_range_from == 0 {
        st.ai_table_range_from = 1;
    }

    pubgen_dialog_modal(ctx, &crate::i18n::tr(&app.ui.language, "Translate Table Pages (Batch)"), &mut open, 440.0, 360.0, |ui| {
        ui.add_space(4.0);
        ui.label(
            RichText::new(crate::i18n::tr(&app.ui.language, "Batch translate tables across document pages using AI.")).size(11.5).color(t.text_dim),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("From Page:").size(11.0));
            ui.add(egui::DragValue::new(&mut st.ai_table_range_from).range(1..=total_pages.max(1)));
            ui.add_space(12.0);
            ui.label(RichText::new("To Page:").size(11.0));
            ui.add(egui::DragValue::new(&mut st.ai_table_range_to).range(1..=total_pages.max(1)));
            ui.label(format!("(Total: {total_pages})"));
        });
        ui.add_space(8.0);

        // Pre-scan: count how many table pages in range
        let from = st.ai_table_range_from.clamp(1, total_pages.max(1)).saturating_sub(1);
        let to_inclusive = st.ai_table_range_to.clamp(st.ai_table_range_from, total_pages.max(1)).saturating_sub(1);
        let mut table_targets = Vec::new();
        for p in from..=to_inclusive {
            if let Some(target) = ai::detect_table_page_target(app, p) {
                table_targets.push(target);
            }
        }
        let found_count = table_targets.len();
        let badge_text = if found_count == 1 {
            "✓ Found 1 table page in selected range".to_string()
        } else {
            format!("✓ Found {found_count} table pages in selected range")
        };
        let badge_color = if found_count > 0 { t.accent } else { t.text_dim };
        ui.label(RichText::new(badge_text).size(11.0).color(badge_color));

        ui.add_space(12.0);
        let backend_ready = ai::backend_from_prefs(app).is_some();
        let is_running = st.translate_job.as_ref().is_some_and(|j| !j.finished);

        if is_running {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Translating table pages with AI…").size(11.5).color(t.accent));
            });
        } else {
            let can_translate = backend_ready && st.bilingual_output && found_count > 0;
            let trans_btn = ui.add_enabled(can_translate, egui::Button::new(RichText::new("Translate Range").size(12.0)));
            if trans_btn.clicked() {
                ai::execute_translate_table_range(app, st, from, to_inclusive, ui.ctx().clone());
            }
            if !backend_ready {
                ui.label(RichText::new("Configure AI backend first (AI Translate ▸ Setup…)").size(10.5).color(t.text_dim));
            } else if !st.bilingual_output {
                ui.label(RichText::new("Enable Dual Language Mode first (Pubgen ▸ Dual Language Mode)").size(10.5).color(t.text_dim));
            }
        }

        if let Some((msg, ok)) = &st.header_status {
            ui.add_space(8.0);
            let color = if *ok { egui::Color32::from_rgb(80, 200, 120) } else { egui::Color32::from_rgb(220, 80, 80) };
            ui.label(RichText::new(msg).size(11.0).color(color));
        }
    });
    st.show_table_range_popup = open;
}

pub fn show_all_dialogs(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    dialog_table_range_ui(app, ctx, st);
    dialog_excel_ui(app, ctx, st);
    dialog_chart_ui(app, ctx, st);
    dialog_toc_ui(app, ctx, st);
    dialog_cover_ui(app, ctx, st);
    dialog_hf_ui(app, ctx, st);
    dialog_manual_ui(app, ctx, st);
    dialog_utils_ui(app, ctx, st);

    if st.show_about_dialog {
        about_pubgen_dialog(app, ctx, &mut st.show_about_dialog, &Tokens::get(ctx));
    }
}

/// Global entry point for Pubgen dialogs and background tasks called every frame from `app.rs`.
pub fn pubgen_dialogs(app: &mut DesignApp, ctx: &egui::Context) {
    with_state(ctx, |st| {
        // Poll background jobs
        poll_active_jobs(app, st);

        // Show AI setup modal if open
        if st.ai_dialog.is_some() {
            egui::Area::new(egui::Id::unique("pubgen_ai_dialog_area")).show(ctx, |ui| {
                ai::dialog_ui(app, st, ui);
            });
        }

        // Show AI quota popup if any
        if st.ai_quota_popup.is_some() {
            egui::Area::new(egui::Id::unique("pubgen_quota_popup_area")).show(ctx, |ui| {
                ai_quota_popup_ui(st, ui, &Tokens::get(ctx));
            });
        }

        // Show all active popup dialogs
        show_all_dialogs(app, ctx, st);
    });
}
