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

pub fn show_all_dialogs(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
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
