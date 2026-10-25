//! PubGen panel chrome: header band, buttons, badges, logo and the About dialog.

use super::*;

/// Toggle button for Language Mode: Dual Lang (active: blue accent fill) / Single Lang (inactive: outline).
fn toggle_lang_button(ui: &mut Ui, t: &Tokens, bilingual: bool) -> Option<bool> {
    let label = if bilingual { "Dual Lang" } else { "Single Lang" };
    let tip = if bilingual {
        "Language Mode: Dual Lang (Bilingual) active. Click to switch to Single Lang."
    } else {
        "Language Mode: Single Lang active. Click to switch to Dual Lang."
    };
    let w = 84.0;
    let h = 24.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let resp = resp.on_hover_text(tip);
    let p = ui.painter();
    if bilingual {
        let fill = if resp.is_pointer_button_down_on() {
            Color32::from_rgb((t.accent.r() as u16 * 4 / 5) as u8, (t.accent.g() as u16 * 4 / 5) as u8, (t.accent.b() as u16 * 4 / 5) as u8)
        } else if resp.hovered() {
            Color32::from_rgb(
                (t.accent.r() as u16 + 20).min(255) as u8,
                (t.accent.g() as u16 + 20).min(255) as u8,
                (t.accent.b() as u16 + 20).min(255) as u8,
            )
        } else {
            t.accent
        };
        p.rect_filled(r, 3.0, fill);
        p.rect_stroke(r, 3.0, Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
        crate::rtl::paint(p, r.center(), egui::Align2::CENTER_CENTER, label, crate::theme::semibold(11.0), Color32::WHITE);
    } else {
        if resp.is_pointer_button_down_on() {
            p.rect_filled(r, 3.0, t.well);
        } else if resp.hovered() {
            p.rect_filled(r, 3.0, t.hover);
        }
        p.rect_stroke(r, 3.0, Stroke::new(1.0, t.field_border), egui::StrokeKind::Inside);
        crate::rtl::paint(p, r.center(), egui::Align2::CENTER_CENTER, label, crate::theme::semibold(11.0), t.text_dim);
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));

    if resp.clicked() { Some(!bilingual) } else { None }
}

/// Load and cache the PubGen logo from `plan/pubgen/docs_images/pubgen_logo.png`.
fn pubgen_logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let key = egui::Id::new("pubgen_logo_texture");
    if let Some(tex) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return Some(tex);
    }
    let candidates = [
        "plan/pubgen/docs_images/pubgen_logo.png",
        "plan\\pubgen\\docs_images\\pubgen_logo.png",
        "docs_images/pubgen_logo.png",
        "pubgen_logo.png",
        "plan/pubgen/docs_images/pubgen_logo.jpg",
        "plan\\pubgen\\docs_images\\pubgen_logo.jpg",
    ];
    let mut bytes = None;
    for cand in &candidates {
        if let Ok(b) = std::fs::read(cand) {
            bytes = Some(b);
            break;
        }
    }
    let bytes = bytes?;
    let pm = designcraft_render::decode_pixmap_page(&bytes, 0)?;
    let ci = egui::ColorImage::from_rgba_premultiplied([pm.width() as usize, pm.height() as usize], pm.data_as_u8_slice());
    let fitted = crate::widgets::fit_texture(ci, 256);
    let tex = ctx.load_texture("pubgen_logo_dialog", fitted, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(key, tex.clone()));
    Some(tex)
}

/// About PubGen popup dialog modal styled like About DesignCraft.
pub(crate) fn about_pubgen_dialog(app: &mut DesignApp, ctx: &egui::Context, open: &mut bool, t: &Tokens) {
    let area = egui::Id::new("about_pubgen_modal");
    let instance = area.with("placement");
    let resp = crate::modal_drag::modal(ctx, area, instance).show(ctx, |ui| {
        ui.set_width(420.0);
        let margin = egui::Frame::popup(ui.style()).inner_margin;
        let top = ui.cursor().top();
        let title_row = Rect::from_min_max(
            pos2(ui.max_rect().left() - margin.leftf(), top - margin.topf()),
            pos2(ui.max_rect().right() + margin.rightf(), top + ui.spacing().interact_size.y),
        );
        crate::modal_drag::title_drag(ui, instance, title_row);

        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            if let Some(tex) = pubgen_logo_texture(ctx) {
                let logo_size = vec2(72.0, 72.0);
                ui.image((tex.id(), logo_size));
            } else {
                let (r, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
                let p = ui.painter();
                p.rect_filled(r, 6.0, t.accent);
                crate::rtl::paint(p, r.center(), egui::Align2::CENTER_CENTER, "PubGen", crate::theme::semibold(16.0), Color32::WHITE);
            }
            ui.add_space(8.0);
            ui.label(egui::RichText::new("PubGen").font(crate::theme::semibold(22.0)).color(t.text_strong));
            ui.label(egui::RichText::new("Publication Generator").font(crate::theme::semibold(14.0)).color(t.text));
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Automated multi-page statistical report & catalogue publishing generator for BPS.").size(12.0).color(t.text_dim),
            );
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(10.0);

            ui.label(egui::RichText::new("Developed by IPDS BPS Karawang\nBudi Yunior (dior@bps.go.id)").size(12.5).color(t.text_strong));
            ui.add_space(4.0);

            let mail_resp = ui
                .add(egui::Label::new(egui::RichText::new("dior@bps.go.id").size(12.0).color(t.accent).underline()).sense(Sense::click()))
                .on_hover_text("Send email to dior@bps.go.id")
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if mail_resp.clicked() {
                app.ui.pending_urls.push("mailto:dior@bps.go.id".to_string());
            }

            ui.add_space(16.0);
            if ui.button(crate::rtl::widget(ui, crate::i18n::tr(&app.ui.language, "  Close  "))).clicked() {
                *open = false;
            }
            ui.add_space(6.0);
        });
    });

    if resp.should_close() {
        *open = false;
    }
}

/// A prominent outline button for primary workflow actions (Master Excel ingest).
pub(crate) fn primary_outline_button(ui: &mut Ui, t: &Tokens, label: &str, width: f32) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 24.0), Sense::click());
    let p = ui.painter();
    if resp.is_pointer_button_down_on() {
        p.rect_filled(r, 3.0, t.well);
    } else if resp.hovered() {
        p.rect_filled(r, 3.0, t.hover);
    } else {
        let bg = Color32::from_rgba_unmultiplied(t.accent.r(), t.accent.g(), t.accent.b(), 26);
        p.rect_filled(r, 3.0, bg);
    }
    p.rect_stroke(r, 3.0, Stroke::new(1.2, t.accent), egui::StrokeKind::Inside);
    crate::rtl::paint(p, r.center(), egui::Align2::CENTER_CENTER, label, crate::theme::semibold(11.0), t.text_strong);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    resp
}

pub(crate) fn badge(ui: &mut Ui, t: &Tokens, role: &str, col_name: &str) {
    let text = format!("{role}: {col_name}");
    let galley = ui.painter().layout_no_wrap(text, egui::FontId::proportional(10.0), t.text_strong);
    let (rect, _) = ui.allocate_exact_size(vec2(galley.size().x + 8.0, 16.0), Sense::hover());
    ui.painter().rect_filled(rect, 3.0, t.panel_darker);
    ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Middle);
    ui.painter().galley(rect.min + vec2(4.0, 2.0), galley, Color32::WHITE);
}

pub(crate) fn pubgen_header_band(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        let g = crate::rtl::plain(ui.ctx(), "Pubgen", crate::theme::semibold(11.5), t.text_strong);
        let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 6.0, 28.0), Sense::hover());
        ui.painter().galley(pos2(r.min.x, r.center().y - g.size().y / 2.0), g, t.text_strong);

        // Language Mode toggle button: Dual Lang (active blue) / Single Lang (inactive outline)
        if let Some(new_mode) = toggle_lang_button(ui, t, st.bilingual_output) {
            let was_bilingual = st.bilingual_output;
            st.bilingual_output = new_mode;
            let mode_str = if st.bilingual_output { "Dual Lang (Bilingual)" } else { "Single Lang" };
            st.header_status = Some((format!("Mode: {mode_str}"), true));
            if let Some(old_title) = toc_kind_title(st.toc_kind, was_bilingual)
                && (st.toc_title.trim().is_empty() || st.toc_title.trim() == old_title)
                && let Some(new_title) = toc_kind_title(st.toc_kind, st.bilingual_output)
            {
                st.toc_title = new_title.to_string();
            }
        }

        // AI Translator button
        let backend_ready = ai::backend_from_prefs(app).is_some();
        let ai_setup_label = if backend_ready { "AI Translator \u{2713}" } else { "AI Translator" };
        let ai_setup_tip = if backend_ready {
            "AI Provider configured and ready. Click to change settings, API key, or model."
        } else {
            "Click to set up AI translation provider (Groq API key & model selection, or offline translator)."
        };
        let ai_setup_resp = outline_button(ui, ai_setup_label, 90.0).on_hover_text(ai_setup_tip);
        if ai_setup_resp.clicked() {
            st.ai_dialog = Some(ai::AiSetupDialog::open(app));
        }

        // Justify-between: right-aligned About icon button
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let about_btn = ui
                .add(egui::Button::new(RichText::new("\u{2139}").size(13.0).color(t.text_dim)).frame(false))
                .on_hover_text("About PubGen")
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if about_btn.clicked() {
                st.show_about_dialog = true;
            }
        });
    });

    ui.horizontal(|ui| {
        ui.set_min_height(26.0);
        let lang = app.ui.language.clone();
        let backend_ready = ai::backend_from_prefs(app).is_some();

        // "Translate this Page" button (enabled when Dual Lang + backend setup + (RXX or Table page selected))
        let page_idx = app.session.active().and_then(|s| s.selection.page).or_else(|| crate::canvas::current_page(app));
        let review_target = page_idx.and_then(|p| ai::detect_page_target(app, p));
        let table_target = page_idx.and_then(|p| ai::detect_table_page_target(app, p));
        let is_review = review_target.is_some();
        let is_table = table_target.is_some();
        let can_page_ai = (is_review || is_table) && backend_ready && st.bilingual_output;

        let btn_label = if is_table {
            "Translate Table on this Page"
        } else if is_review {
            "Translate Review on this Page"
        } else {
            "Translate this Page"
        };

        let page_btn = ui
            .add_enabled(can_page_ai, egui::Button::new(btn_label))
            .on_hover_text(if is_table {
                "Translate the selected Table page in place: title (new line Italic), header/footer cells (in same cell new line [italic]), and notes/source box (next row [italic])."
            } else {
                "Translate the selected RXX review page in place: the chapter title (textbox 3) and the right-column body are replaced with English while preserving all paragraph styles."
            })
            .on_disabled_hover_text(if !backend_ready {
                "Configure AI Provider first: click 'AI Translator' button above to set up your Groq key and model."
            } else if !st.bilingual_output {
                "Switch to Dual Lang mode first (button above)."
            } else {
                "Select a generated RXX review page or Table page to translate in place."
            });
        if page_btn.clicked() && let Some(target) = table_target {
            let mut units = ai::collect_table_page_units(app, &target);
            units.retain(|(k, _)| !st.translations.contains_key(k));
            if let Some((provider, key, model)) = ai::backend_from_prefs(app) {
                if !units.is_empty() {
                    st.ai_table_target = Some(target);
                    st.translate_job = Some(ai::TranslateJob::spawn(&provider, &key, &model, units, ui.ctx().clone()));
                    st.header_status = Some(("Translating table page with AI…".into(), true));
                } else {
                    ai::apply_table_page_result(app, st, &target);
                    st.header_status = Some(("Table page translated from cache.".into(), true));
                }
            } else if units.is_empty() {
                ai::apply_table_page_result(app, st, &target);
                st.header_status = Some(("Table page translated from cache.".into(), true));
            } else {
                st.header_status = Some(("AI translation: no backend configured — click 'AI Translator' to set up.".into(), false));
            }
        } else if page_btn.clicked() && let Some(target) = review_target {
            let mut units = ai::collect_page_units(app, &target);
            units.retain(|(k, _)| !st.translations.contains_key(k));
            if let Some((provider, key, model)) = ai::backend_from_prefs(app) {
                st.ai_page_target = Some(target);
                st.translate_job = Some(ai::TranslateJob::spawn(&provider, &key, &model, units, ui.ctx().clone()));
                st.header_status = Some(("Translating page with AI…".into(), true));
            } else if units.is_empty() {
                // Everything cached: re-apply from the map.
                ai::apply_page_result(app, st, &target);
                st.header_status = Some(("Page translated from cache.".into(), true));
            } else {
                st.header_status = Some(("AI translation: no backend configured — click 'AI Translator' to set up.".into(), false));
            }
        }

        let total_pages = app.session.active().map(|s| s.doc.page_count()).unwrap_or(0);
        let batch_btn = ui
            .add_enabled(backend_ready && st.bilingual_output && total_pages > 0, egui::Button::new("Translate Tables…"))
            .on_hover_text("Batch translate table pages across a page range using AI.");
        if batch_btn.clicked() {
            st.show_table_range_popup = !st.show_table_range_popup;
            if st.ai_table_range_to == 0 || st.ai_table_range_to > total_pages {
                st.ai_table_range_to = total_pages.max(1);
            }
        }

        let tip = "Set document margins:\nTop: 20.20 mm\nBottom: 20 mm\nInside: 19 mm\nOutside: 15 mm";
        let resp = outline_button(ui, crate::i18n::tr(&lang, "Set Page Margin"), 105.0).on_hover_text(tip);
        if resp.clicked() {
            match execute_set_page_margins(app) {
                Ok(msg) => st.header_status = Some((msg, true)),
                Err(err) => st.header_status = Some((err, false)),
            }
        }

        // Active mode indicator badge
        let mode_str = if st.bilingual_output { "Dual Lang" } else { "Single Lang" };
        badge(ui, t, "mode", mode_str);
    });

    if st.show_table_range_popup {
        let total_pages = app.session.active().map(|s| s.doc.page_count()).unwrap_or(0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Translate Tables:").size(10.5));
            ui.label("From Page:");
            ui.add(egui::DragValue::new(&mut st.ai_table_range_from).range(1..=total_pages.max(1)));
            ui.label("To:");
            ui.add(egui::DragValue::new(&mut st.ai_table_range_to).range(1..=total_pages.max(1)));
            if ui.button("Translate Range").clicked() {
                st.show_table_range_popup = false;
                let from = st.ai_table_range_from.clamp(1, total_pages.max(1)).saturating_sub(1);
                let to_inclusive = st.ai_table_range_to.clamp(st.ai_table_range_from, total_pages.max(1)).saturating_sub(1);
                let mut targets = Vec::new();
                for p in from..=to_inclusive {
                    if let Some(t) = ai::detect_table_page_target(app, p) {
                        targets.push(t);
                    }
                }
                if targets.is_empty() {
                    st.header_status = Some(("No table pages found in selected range.".into(), false));
                } else {
                    let mut all_units = Vec::new();
                    for t in &targets {
                        all_units.extend(ai::collect_table_page_units(app, t));
                    }
                    all_units.retain(|(k, _)| !st.translations.contains_key(k));
                    all_units.sort_by(|a, b| a.0.cmp(&b.0));
                    all_units.dedup_by(|a, b| a.0 == b.0);

                    if let Some((provider, key, model)) = ai::backend_from_prefs(app) {
                        if !all_units.is_empty() {
                            st.ai_table_targets_batch = targets;
                            st.translate_job = Some(ai::TranslateJob::spawn(&provider, &key, &model, all_units, ui.ctx().clone()));
                            st.header_status = Some(("Translating table pages with AI…".into(), true));
                        } else {
                            let mut count = 0;
                            for t in &targets {
                                count += ai::apply_table_page_result(app, st, t);
                            }
                            st.header_status =
                                Some((format!("{} table item(s) across {} page(s) translated from cache.", count, targets.len()), true));
                        }
                    } else if all_units.is_empty() {
                        let mut count = 0;
                        for t in &targets {
                            count += ai::apply_table_page_result(app, st, t);
                        }
                        st.header_status = Some((format!("{} table item(s) across {} page(s) translated from cache.", count, targets.len()), true));
                    } else {
                        st.header_status = Some(("AI translation: no backend configured — click 'AI Translator' to set up.".into(), false));
                    }
                }
            }
            if ui.button("Close").clicked() {
                st.show_table_range_popup = false;
            }
        });
    }

    rule(ui);

    if let Some((msg, ok)) = &st.header_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(10.5).color(color)).wrap());
    }
}
