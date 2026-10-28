//! Modal dialog popup windows for the Pubgen menu system.
//!
//! Provides dedicated floating modal dialogs for each Pubgen capability,
//! allowing the panel to be removed from the sidebar dock and operated entirely
//! through top-level menus and rich popup forms.

use egui::{Align2, Rect, RichText, Sense, pos2, vec2};

use crate::DesignApp;
use crate::theme::{Tokens, semibold};

use super::*;

/// Standard modal dialog shell: centered, draggable title bar, close (✕) button,
/// and vertical scrolling for the content area.
fn pubgen_dialog_modal(
    ctx: &egui::Context,
    id: &'static str,
    title: &str,
    open: &mut bool,
    width: f32,
    max_height: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let t = Tokens::get(ctx);
    let area = egui::Id::unique(id);
    let instance = area.with("placement");

    crate::modal_drag::modal(ctx, area, instance).show(ctx, |ui| {
        ui.set_width(width);
        let margin = egui::Frame::popup(ui.style()).inner_margin;
        let top = ui.cursor().top();
        let title_row = Rect::from_min_max(
            pos2(ui.max_rect().left() - margin.leftf(), top - margin.topf()),
            pos2(ui.max_rect().right() + margin.rightf(), top + 26.0),
        );
        crate::modal_drag::title_drag(ui, instance, title_row);

        // Header with title and close button (✕)
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(title).font(semibold(13.5)).color(t.text_strong));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (r, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(r, 3.0, t.hover);
                }
                ui.painter().text(r.center(), Align2::CENTER_CENTER, "✕", egui::FontId::proportional(13.0), t.text_dim);
                if resp.clicked() {
                    *open = false;
                }
            });
        });

        ui.add_space(3.0);
        ui.separator();
        ui.add_space(4.0);

        egui::ScrollArea::vertical().max_height(max_height).auto_shrink([false, false]).show(ui, add_contents);
    });
}

/// Modal dialog for Master Excel import, sheet parsing, and 1-click batch generation.
pub fn dialog_excel_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_excel_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    pubgen_dialog_modal(
        ctx,
        "pubgen_dialog_excel",
        crate::i18n::tr(&app.ui.language, "Master Excel (.xlsx) Import & Generate"),
        &mut open,
        480.0,
        520.0,
        |ui| {
            master_excel_ui(app, ui, st, &t);
        },
    );
    st.dialog_excel_open = open;
}

/// Modal dialog for Table of Contents generation and options.
pub fn dialog_toc_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_toc_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    pubgen_dialog_modal(
        ctx,
        "pubgen_dialog_toc",
        crate::i18n::tr(&app.ui.language, "Table of Contents (TOC) Options"),
        &mut open,
        460.0,
        520.0,
        |ui| {
            card_toc_ui(app, ui, st, &t);
        },
    );
    st.dialog_toc_open = open;
}

/// Modal dialog for Publication Cover styling, palette presets, and overrides.
pub fn dialog_cover_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_cover_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    pubgen_dialog_modal(
        ctx,
        "pubgen_dialog_cover",
        crate::i18n::tr(&app.ui.language, "Publication Cover Generator"),
        &mut open,
        540.0,
        640.0,
        |ui| {
            card_cover_body_ui(app, ui, st, &t);
        },
    );
    st.dialog_cover_open = open;
}

/// Modal dialog for Header & Footer templates.
pub fn dialog_hf_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_hf_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut open = true;
    pubgen_dialog_modal(ctx, "pubgen_dialog_hf", crate::i18n::tr(&app.ui.language, "Header & Footer Templates"), &mut open, 480.0, 540.0, |ui| {
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
    pubgen_dialog_modal(ctx, "pubgen_dialog_manual", crate::i18n::tr(&app.ui.language, "Manual CSV & Table Ingest"), &mut open, 520.0, 580.0, |ui| {
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
    pubgen_dialog_modal(
        ctx,
        "pubgen_dialog_utils",
        crate::i18n::tr(&app.ui.language, "Page Margins & Renumbering Utilities"),
        &mut open,
        420.0,
        440.0,
        |ui| {
            page_utilities_ui(app, ui, st, &t);
        },
    );
    st.dialog_utils_open = open;
}

/// Renders all active Pubgen popup dialogs and modal windows for the frame.
pub fn show_all_dialogs(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    dialog_excel_ui(app, ctx, st);
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
