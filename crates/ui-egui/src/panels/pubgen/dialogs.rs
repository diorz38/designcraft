//! Modal dialog popup windows for the Pubgen menu system.
//!
//! Provides unified floating modal dialogs for each Pubgen capability,
//! matching the reference "Translate with AI" window design and centered
//! in the container.

use egui::{Align2, RichText};

use crate::DesignApp;
use crate::theme::Tokens;

use super::*;

/// Shared popup dialog shell: a title band spanning the full window width with a
/// thin ✕ hugging the corner, then the dialog body.
///
/// egui's stock title-bar ✕ sits ~24 px in from the corner (atom layout, not
/// adjustable), so we drop the stock title bar entirely (`title_bar(false)`) and
/// draw our own band. The band is also the drag handle; `WindowDrag::Off` keeps
/// the body free for DragValues and text fields. `open` is cleared when the ✕
/// is clicked. `title` doubles as the window id and must be unique per dialog.
pub fn dialog_window(
    ctx: &egui::Context,
    title: &str,
    rtl: bool,
    resizable: bool,
    open: &mut bool,
    default_width: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let area_id = egui::Id::unique(("pubgen_dialog", title));
    let pending_pos = ctx.data_mut(|d| d.get_temp::<egui::Pos2>(area_id.with("band_drag_pos")));
    let mut close_clicked = false;
    let mut window = egui::Window::new("")
        .id(area_id)
        .title_bar(false)
        .drag_area(egui::WindowDrag::Off)
        .collapsible(false)
        .resizable(resizable)
        .min_width(default_width)
        .max_width(default_width)
        .default_width(default_width)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center());
    if let Some(pivot_pos) = pending_pos {
        window = window.fixed_pos(pivot_pos);
    }
    // Zero inner margin so the band spans the full window width; the body
    // wraps itself in a margin frame below.
    window.frame(egui::Frame::window(&ctx.global_style()).inner_margin(egui::Margin::same(0))).show(ctx, |ui| {
        if title_band_ui(ui, ctx, area_id, title, rtl) {
            close_clicked = true;
        }
        egui::Frame::NONE.inner_margin(egui::Margin::same(6)).show(ui, |ui| {
            add_contents(ui);
        });
    });
    *open &= !close_clicked;
}

/// Paints the title band, handles band-drag and the corner ✕. Returns true when
/// the ✕ was clicked.
fn title_band_ui(ui: &mut egui::Ui, ctx: &egui::Context, area_id: egui::Id, title: &str, rtl: bool) -> bool {
    let visuals = ctx.global_style().visuals.clone();
    let (band_rect, band) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::click_and_drag());
    let r = visuals.window_corner_radius;
    ui.painter().add(egui::epaint::RectShape::filled(
        band_rect,
        egui::CornerRadius { nw: r.nw, ne: r.ne, sw: 0, se: 0 },
        visuals.widgets.open.weak_bg_fill,
    ));
    ui.painter().line_segment([band_rect.left_bottom(), band_rect.right_bottom()], visuals.widgets.noninteractive.bg_stroke);

    // Drag the window by its band: remember the position at drag start, then
    // reposition via `.fixed_pos` next pass (Area::end overwrites any mid-pass
    // state write, so the move lands one frame later).
    let anchor_key = area_id.with("band_drag_anchor");
    let drag_pos_key = area_id.with("band_drag_pos");
    if band.drag_started()
        && let Some(state) = egui::AreaState::load(ctx, area_id)
    {
        ctx.data_mut(|d| d.insert_temp(anchor_key, state.left_top_pos()));
    }
    if band.dragged()
        && let Some(anchor) = ctx.data_mut(|d| d.get_temp::<egui::Pos2>(anchor_key))
        && let Some(delta) = band.total_drag_delta()
        && let Some(mut state) = egui::AreaState::load(ctx, area_id)
    {
        state.set_left_top_pos(anchor + delta);
        if let Some(pivot_pos) = state.pivot_pos {
            ctx.data_mut(|d| d.insert_temp(drag_pos_key, pivot_pos));
        }
    }
    if band.drag_stopped() {
        ctx.data_mut(|d| d.remove_temp::<egui::Pos2>(anchor_key));
        ctx.data_mut(|d| d.remove_temp::<egui::Pos2>(drag_pos_key));
    }

    // Thin ✕ like egui's stock close button: two short line segments.
    let x_center = if rtl { band_rect.left() + 12.0 } else { band_rect.right() - 12.0 };
    let x_mid = egui::pos2(x_center, band_rect.center().y);
    let x_rect = egui::Rect::from_center_size(x_mid, egui::vec2(20.0, 20.0));
    let close = ui.interact(x_rect, ui.auto_id_with("dialog_close"), egui::Sense::click());
    close.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Close window"));
    if close.hovered() {
        ui.painter().rect_filled(x_rect, 3.0, visuals.widgets.hovered.weak_bg_fill);
    }
    let stroke = if close.hovered() { visuals.widgets.hovered.fg_stroke } else { visuals.widgets.noninteractive.fg_stroke };
    ui.painter().line_segment([egui::pos2(x_mid.x - 4.0, x_mid.y - 4.0), egui::pos2(x_mid.x + 4.0, x_mid.y + 4.0)], stroke);
    ui.painter().line_segment([egui::pos2(x_mid.x + 4.0, x_mid.y - 4.0), egui::pos2(x_mid.x - 4.0, x_mid.y + 4.0)], stroke);

    ui.painter().text(band_rect.center(), Align2::CENTER_CENTER, title, crate::theme::semibold(15.0), visuals.strong_text_color());

    close.clicked()
}

/// Standard unified popup window shell matching "Translate with AI":
/// centered in container, non-collapsible, vertical scroll area,
/// and bottom Close button, inside the shared [`dialog_window`] shell.
fn pubgen_dialog_modal(
    ctx: &egui::Context,
    title: &str,
    rtl: bool,
    open: &mut bool,
    default_width: f32,
    max_height: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let available_h = (ctx.content_rect().height() - 90.0).max(200.0);
    let scroll_h = max_height.min(available_h);

    let mut close_btn = false;
    dialog_window(ctx, title, rtl, false, open, default_width, |ui| {
        let content_w = default_width - 16.0;
        egui::ScrollArea::vertical().max_width(content_w).max_height(scroll_h).auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(content_w);
            ui.set_max_width(content_w);
            add_contents(ui);
        });

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Close").clicked() {
                close_btn = true;
            }
        });
    });
    if close_btn {
        *open = false;
    }
}

/// Modal dialog for Master Excel import, sheet parsing, and 1-click batch generation.
pub fn dialog_excel_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_excel_open {
        return;
    }
    let t = Tokens::get(ctx);
    let rtl = crate::i18n::is_rtl(&app.ui.language);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Master Excel (.xlsx) Import & Generate"), rtl, &mut open, 480.0, 520.0, |ui| {
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
    let rtl = crate::i18n::is_rtl(&app.ui.language);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Table of Contents (TOC) Options"), rtl, &mut open, 460.0, 520.0, |ui| {
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
    let rtl = crate::i18n::is_rtl(&app.ui.language);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Publication Cover Generator"), rtl, &mut open, 540.0, 880.0, |ui| {
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
    let rtl = crate::i18n::is_rtl(&app.ui.language);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Header & Footer Templates"), rtl, &mut open, 480.0, 540.0, |ui| {
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
    let rtl = crate::i18n::is_rtl(&app.ui.language);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Manual CSV & Table Ingest"), rtl, &mut open, 520.0, 580.0, |ui| {
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
    let rtl = crate::i18n::is_rtl(&app.ui.language);
    let mut open = true;
    pubgen_dialog_modal(ctx, crate::i18n::tr(&app.ui.language, "Page Margins & Renumbering Utilities"), rtl, &mut open, 420.0, 440.0, |ui| {
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

    pubgen_dialog_modal(
        ctx,
        &crate::i18n::tr(&app.ui.language, "Translate Table Pages (Batch)"),
        crate::i18n::is_rtl(&app.ui.language),
        &mut open,
        440.0,
        360.0,
        |ui| {
            ui.add_space(4.0);
            ui.label(
                RichText::new(crate::i18n::tr(&app.ui.language, "Batch translate tables across document pages using AI."))
                    .size(11.5)
                    .color(t.text_dim),
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
        },
    );
    st.show_table_range_popup = open;
}

pub fn dialog_profile_customize_ui(_app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.dialog_profile_customize_open {
        return;
    }
    let rtl = false;
    let mut open = st.dialog_profile_customize_open;
    let mut prof = st.active_profile();
    let mut should_close = false;
    let mut should_apply = None;
    let mut should_reset = false;

    dialog_window(ctx, "Sesuaikan Profil Publikasi", rtl, true, &mut open, 480.0, |ui| {
        egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
            ui.add_space(4.0);
            ui.heading(format!("Profil: {}", prof.name));
            ui.label(RichText::new(&prof.description).size(10.5).color(Tokens::get(ctx).text_dim));
            ui.add_space(8.0);

            // 1. Geometri & Halaman
            ui.group(|ui| {
                ui.label(RichText::new("1. Geometri & Tata Letak Halaman").strong());
                ui.horizontal(|ui| {
                    ui.label("Ukuran Kertas:");
                    craft_ui::ComboBox::from_id_salt("profile_page_size").selected_text(&prof.geometry.page_size).show_ui(ui, |ui| {
                        for sz in &["A4", "B5", "A5", "Letter"] {
                            ui.selectable_value(&mut prof.geometry.page_size, sz.to_string(), *sz);
                        }
                    });
                });
                ui.checkbox(&mut prof.geometry.facing_pages, "Halaman Berhadapan (Facing Spreads)");

                ui.add_space(4.0);
                ui.label("Margin Halaman (mm):");
                ui.horizontal(|ui| {
                    ui.label("Atas:");
                    ui.add(egui::DragValue::new(&mut prof.geometry.margins_mm.top).range(5.0..=100.0).speed(0.5));
                    ui.label("Bawah:");
                    ui.add(egui::DragValue::new(&mut prof.geometry.margins_mm.bottom).range(5.0..=100.0).speed(0.5));
                });
                ui.horizontal(|ui| {
                    let inside_lbl = if prof.geometry.facing_pages { "Dalam (Punggung):" } else { "Kiri:" };
                    let outside_lbl = if prof.geometry.facing_pages { "Luar:" } else { "Kanan:" };
                    ui.label(inside_lbl);
                    ui.add(egui::DragValue::new(&mut prof.geometry.margins_mm.inside).range(5.0..=100.0).speed(0.5));
                    ui.label(outside_lbl);
                    ui.add(egui::DragValue::new(&mut prof.geometry.margins_mm.outside).range(5.0..=100.0).speed(0.5));
                });
            });

            ui.add_space(6.0);

            // 2. Format Tabel
            ui.group(|ui| {
                ui.label(RichText::new("2. Format & Gaya Tabel").strong());
                ui.checkbox(&mut prof.table_styling.include_column_number_row, "Tampilkan baris penomoran kolom (1) (2) (3)");
                ui.checkbox(&mut prof.table_styling.horizontal_rules_only, "Hanya garis horizontal (Gaya APA / Minimalis tanpa garis vertikal)");
                ui.checkbox(&mut prof.table_styling.alternating_row_fills, "Pewarnaan baris berselang-seling (Zebra Striping)");
            });

            ui.add_space(6.0);

            // 3. Nomenklatur & Label
            ui.group(|ui| {
                ui.label(RichText::new("3. Nomenklatur & Label").strong());
                ui.horizontal(|ui| {
                    ui.label("Label Bab:");
                    ui.text_edit_singleline(&mut prof.nomenclature.chapter_prefix);
                });
                ui.horizontal(|ui| {
                    ui.label("Label Tabel:");
                    ui.text_edit_singleline(&mut prof.nomenclature.table_prefix);
                });
                ui.horizontal(|ui| {
                    ui.label("Label Grafik:");
                    ui.text_edit_singleline(&mut prof.nomenclature.chart_prefix);
                });
                ui.horizontal(|ui| {
                    ui.label("Judul Daftar Isi:");
                    ui.text_edit_singleline(&mut prof.nomenclature.toc_title);
                });
            });

            ui.add_space(10.0);

            // Tombol Aksi
            ui.horizontal(|ui| {
                if ui.button("Terapkan Sesi Ini").clicked() {
                    should_apply = Some(prof.clone());
                    should_close = true;
                }
                if ui.button("Reset ke BPS DDA").clicked() {
                    should_reset = true;
                    should_close = true;
                }
            });
        });
    });

    if let Some(mut p) = should_apply {
        p.id = "custom".to_string();
        p.name = "Kustom (Disesuaikan)".to_string();
        p.sanitize();
        st.apply_profile(p, true);
    } else if should_reset {
        let dda = super::profiles::ProfileManager::default_profile();
        st.apply_profile(dda, true);
    }
    if should_close {
        open = false;
    }
    st.dialog_profile_customize_open = open;
}

pub fn show_all_dialogs(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    dialog_profile_customize_ui(app, ctx, st);
    dialog_table_range_ui(app, ctx, st);
    dialog_excel_ui(app, ctx, st);
    dialog_chart_ui(app, ctx, st);
    dialog_toc_ui(app, ctx, st);
    dialog_cover_ui(app, ctx, st);
    dialog_hf_ui(app, ctx, st);
    dialog_manual_ui(app, ctx, st);
    dialog_utils_ui(app, ctx, st);
    super::audit::audit_dialog_ui(app, ctx, st);

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

#[cfg(test)]
mod tests {
    /// What the shared [`super::dialog_window`] shell painted: the window frame fill,
    /// the title-band fill, and the ✕ (two thin diagonal line segments).
    struct Painted {
        frame: egui::Rect,
        band: Option<egui::Rect>,
        glyph_right: f32,
    }

    fn collect_shapes(shape: &egui::Shape, out: &mut Painted) {
        match shape {
            egui::Shape::Rect(r) if r.rect.width() > 300.0 => {
                if r.rect.height() < 50.0 {
                    out.band = Some(match out.band {
                        Some(b) => b.union(r.rect),
                        None => r.rect,
                    });
                } else if r.stroke.width > 0.0 {
                    // The window frame fill has the window stroke; the shadow rect has none.
                    out.frame = out.frame.union(r.rect);
                }
            }
            // The ✕ is two short diagonal line segments.
            egui::Shape::LineSegment { points, .. }
                if (points[1].x - points[0].x).abs() > 2.0 && ((points[1].x - points[0].x).abs() - (points[1].y - points[0].y).abs()).abs() < 0.1 =>
            {
                out.glyph_right = out.glyph_right.max(points[0].x).max(points[1].x);
            }
            egui::Shape::Vec(shapes) => {
                for s in shapes {
                    collect_shapes(s, out);
                }
            }
            _ => {}
        }
    }

    fn run_pass(ctx: &egui::Context, events: Vec<egui::Event>, open: &mut bool) -> Painted {
        let mut painted = Painted { frame: egui::Rect::NOTHING, band: None, glyph_right: f32::MIN };
        let mut output = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), events, ..Default::default() },
            |ui| {
                super::dialog_window(ui.ctx(), "Title", false, false, open, 480.0, |ui| {
                    ui.label("body");
                });
            },
        );
        output.textures_delta.clear();
        for clipped in &output.shapes {
            collect_shapes(&clipped.shape, &mut painted);
        }
        painted
    }

    /// The ✕ must hug the corner (egui's stock one sat ~24 px in) and the title band
    /// must span the full window width, like a normal window header.
    #[test]
    fn dialog_band_spans_the_window_and_close_hugs_the_corner() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx, "en");
        let mut open = true;
        let mut painted = Painted { frame: egui::Rect::NOTHING, band: None, glyph_right: f32::MIN };
        for _ in 0..3 {
            painted = run_pass(&ctx, Vec::new(), &mut open);
        }
        assert!(painted.frame.is_positive(), "window not painted");
        let band = painted.band.expect("title band not painted");
        assert!(band.width() >= painted.frame.width() - 4.0, "band {:.1} wide but window frame {:.1} wide", band.width(), painted.frame.width());
        assert!(
            (band.left() - painted.frame.left()).abs() < 3.0 && (band.right() - painted.frame.right()).abs() < 3.0,
            "band not flush with the window edges"
        );
        assert!(painted.glyph_right.is_finite(), "✕ glyph not painted");
        let gap = painted.frame.right() - painted.glyph_right;
        assert!(gap < 14.0, "✕ sits {gap:.1} px from the window's right edge");
    }

    /// Clicking the ✕ closes the dialog.
    #[test]
    fn corner_close_click_closes_the_dialog() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx, "en");
        let mut open = true;
        let mut painted = Painted { frame: egui::Rect::NOTHING, band: None, glyph_right: f32::MIN };
        for _ in 0..3 {
            painted = run_pass(&ctx, Vec::new(), &mut open);
        }
        let pos = egui::pos2(painted.frame.right() - 12.0, painted.frame.top() + 15.0);
        let events = vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() },
            egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() },
        ];
        run_pass(&ctx, events, &mut open);
        assert!(!open, "clicking the ✕ must close the dialog");
    }

    /// Dragging the title band moves the window.
    #[test]
    fn title_band_drag_moves_the_window() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx, "en");
        let mut open = true;
        let mut painted = Painted { frame: egui::Rect::NOTHING, band: None, glyph_right: f32::MIN };
        for _ in 0..3 {
            painted = run_pass(&ctx, Vec::new(), &mut open);
        }
        assert!(painted.frame.is_positive(), "window not painted");
        let start_left = painted.frame.left();
        let grab = egui::pos2(painted.frame.left() + 40.0, painted.frame.top() + 15.0);
        let moved = egui::pos2(grab.x + 120.0, grab.y + 60.0);
        run_pass(
            &ctx,
            vec![
                egui::Event::PointerMoved(grab),
                egui::Event::PointerButton { pos: grab, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() },
            ],
            &mut open,
        );
        // One pass to settle the press before moving, as a real pointer would.
        run_pass(&ctx, vec![egui::Event::PointerMoved(grab)], &mut open);
        run_pass(&ctx, vec![egui::Event::PointerMoved(moved)], &mut open);
        painted = run_pass(
            &ctx,
            vec![egui::Event::PointerButton { pos: moved, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }],
            &mut open,
        );
        assert!(
            painted.frame.left() > start_left + 80.0,
            "window did not follow the band drag: left {:.1} → {:.1}",
            start_left,
            painted.frame.left()
        );
    }
}
