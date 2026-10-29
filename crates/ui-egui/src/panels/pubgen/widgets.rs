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
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));

    if resp.clicked() { Some(!bilingual) } else { None }
}

/// Load and cache the PubGen logo from `plan/pubgen/docs_images/pubgen_logo.png`.
fn pubgen_logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let key = egui::Id::unique("pubgen_logo_texture");
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
    let mut close_clicked = false;
    super::dialogs::dialog_window(ctx, "About PubGen", crate::i18n::is_rtl(&app.ui.language), false, open, 420.0, |ui| {
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
            ui.separator();
            ui.horizontal(|ui| {
                // PubGen replaces DesignCraft's menu bar by default; give the full one back here
                // too, so the About window is always a way back.
                let pubgen_on = crate::menus::pubgen_bar::pubgen_menus_on(ui.ctx());
                let toggle = if pubgen_on { "Show DesignCraft Menus" } else { "Show PubGen Menus" };
                if ui.button(toggle).clicked() {
                    crate::menus::pubgen_bar::set_pubgen_menus_on(ui.ctx(), !pubgen_on);
                }
                if ui.button("Close").clicked() {
                    close_clicked = true;
                }
            });
            ui.add_space(4.0);
        });
    });
    if close_clicked {
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
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
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
            st.ai_dialog = Some(ai::AiSetupDialog::open(app, st));
        }
        // Groq's own rate-limit header (per API key): shown once the first
        // translation has been made this session.
        if let Some(n) = designcraft_translate::remaining_requests() {
            ui.label(RichText::new(format!("Groq: ~{n} requests left today")).size(10.0).color(t.text_dim));
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
        if page_btn.clicked() && let Some(target) = table_target.clone() {
            let mut units = ai::collect_table_page_units(app, &target, &st.glossary);
            units.retain(|(k, _)| !st.translations.contains_key(k));
            if let Some((provider, key, model)) = ai::backend_from_prefs(app) {
                if !units.is_empty() {
                    st.ai_table_target = Some(target);
                    let terms = st.glossary.prompt_pairs();
                    st.translate_job = Some(ai::TranslateJob::spawn(&provider, &key, &model, units, terms, ui.ctx().clone()));
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
        } else if page_btn.clicked() && let Some(target) = review_target.as_ref() {
            let mut units = ai::collect_page_units(app, target);
            units.retain(|(k, _)| !st.translations.contains_key(k));
            if let Some((provider, key, model)) = ai::backend_from_prefs(app) {
                st.ai_page_target = Some(target.clone());
                let terms = st.glossary.prompt_pairs();
                st.translate_job = Some(ai::TranslateJob::spawn(&provider, &key, &model, units, terms, ui.ctx().clone()));
                st.header_status = Some(("Translating page with AI…".into(), true));
            } else if units.is_empty() {
                // Everything cached: re-apply from the map.
                ai::apply_page_result(app, st, target);
                st.header_status = Some(("Page translated from cache.".into(), true));
            } else {
                st.header_status = Some(("AI translation: no backend configured — click 'AI Translator' to set up.".into(), false));
            }
        }

        // "Fix Terms": enforce the glossary on this page's existing translations — the
        // correction pass for AI text written before (or against) the current glossary.
        let can_fix = (is_review || is_table) && !st.translations.is_empty() && !st.glossary.is_empty();
        let fix_btn = ui
            .add_enabled(can_fix, egui::Button::new("Fix Terms"))
            .on_hover_text("Apply the glossary to this page's existing translations: whole-cell terms are corrected in place; text missing an official term is re-translated on the next Translate click.")
            .on_disabled_hover_text("Select a translated review or table page — the glossary corrects the AI text already cached for it.");
        if fix_btn.clicked() {
            let (corrected, recheck) = ai::apply_glossary_corrections(&st.glossary, &mut st.translations);
            if let Some(target) = table_target.as_ref() {
                ai::apply_table_page_result(app, st, target);
            } else if let Some(target) = review_target.as_ref() {
                ai::apply_page_result(app, st, target);
            }
            st.header_status = Some(if recheck.is_empty() {
                if corrected == 0 {
                    ("The glossary already agrees with this page's translations.".into(), true)
                } else {
                    (format!("Glossary applied: {corrected} term(s) corrected on the page."), true)
                }
            } else {
                (
                    format!(
                        "Glossary applied: {corrected} term(s) corrected; {} text(s) marked — click 'Retranslate (Glossary)' to re-run them with the official terms.",
                        recheck.len()
                    ),
                    true,
                )
            });
        }

        // "Retranslate (Glossary)": enforce the official terms for real — deterministic
        // corrections first, then a fresh AI pass over exactly the units the glossary
        // flagged, using the prompt that carries the official terms.
        let can_retr = can_fix && backend_ready;
        let retr_btn = ui
            .add_enabled(can_retr, egui::Button::new("Retranslate (Glossary)"))
            .on_hover_text("Correct this page with the glossary's official terms, then re-translate the texts the glossary flagged (one AI request batch).")
            .on_disabled_hover_text(if !backend_ready {
                "Configure AI Provider first: click 'AI Translator' button above."
            } else {
                "Select a translated review or table page."
            });
        if retr_btn.clicked() {
            let (corrected, recheck) = ai::apply_glossary_corrections(&st.glossary, &mut st.translations);
            if let Some(target) = table_target.as_ref() {
                ai::apply_table_page_result(app, st, target);
            } else if let Some(target) = review_target.as_ref() {
                ai::apply_page_result(app, st, target);
            }
            if recheck.is_empty() {
                st.header_status = Some(if corrected == 0 {
                    ("The glossary already agrees with this page's translations.".into(), true)
                } else {
                    (format!("Glossary applied: {corrected} term(s) corrected on the page."), true)
                });
            } else if let Some((provider, key, model)) = ai::backend_from_prefs(app) {
                let mut units = if let Some(target) = table_target.as_ref() {
                    ai::collect_table_page_units(app, target, &st.glossary)
                } else if let Some(target) = review_target.as_ref() {
                    ai::collect_page_units(app, target)
                } else {
                    Vec::new()
                };
                units.retain(|(k, _)| recheck.iter().any(|r| r == k));
                if units.is_empty() {
                    st.header_status =
                        Some((format!("Glossary corrected {corrected} term(s); the {} flagged text(s) are not on this page.", recheck.len()), true));
                } else if st.translate_job.as_ref().is_some_and(|j| !j.finished) {
                    st.header_status = Some(("A translation is already running — try again when it finishes.".into(), false));
                } else {
                    let terms = st.glossary.prompt_pairs();
                    if let Some(target) = table_target.clone() {
                        st.ai_table_target = Some(target);
                    } else if let Some(target) = review_target.clone() {
                        st.ai_page_target = Some(target);
                    }
                    st.translate_job = Some(ai::TranslateJob::spawn(&provider, &key, &model, units, terms, ui.ctx().clone()));
                    st.header_status = Some((format!("Retranslating {} text(s) with the official terms…", recheck.len()), true));
                }
            } else {
                st.header_status =
                    Some((format!("Glossary corrected {corrected} term(s); {} text(s) need AI re-translation — configure the provider first.", recheck.len()), false));
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
                        all_units.extend(ai::collect_table_page_units(app, t, &st.glossary));
                    }
                    all_units.retain(|(k, _)| !st.translations.contains_key(k));
                    all_units.sort_by(|a, b| a.0.cmp(&b.0));
                    all_units.dedup_by(|a, b| a.0 == b.0);

                    if let Some((provider, key, model)) = ai::backend_from_prefs(app) {
                        if !all_units.is_empty() {
                            st.ai_table_targets_batch = targets;
                            let terms = st.glossary.prompt_pairs();
                            st.translate_job = Some(ai::TranslateJob::spawn(&provider, &key, &model, all_units, terms, ui.ctx().clone()));
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

pub(crate) fn card1_body_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
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
        let collected = super::vocab::collect_document(app, st);
        if st.auto_fill_ai_drafts {
            ai::fill_blank_drafts(app, st, ui.ctx().clone());
        }
        st.card1_status = Some(match res {
            Ok(msg) => (if collected > 0 { format!("{msg} Vocabulary: {collected} new term(s) collected.") } else { msg }, true),
            Err(err) => (err, false),
        });
    }

    if let Some((msg, ok)) = &st.card1_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

#[allow(dead_code)]
pub(crate) fn card2_body_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
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
        let collected = super::vocab::collect_document(app, st);
        if st.auto_fill_ai_drafts {
            ai::fill_blank_drafts(app, st, ui.ctx().clone());
        }
        st.card2_status = Some(match res {
            Ok(msg) => (if collected > 0 { format!("{msg} Vocabulary: {collected} new term(s) collected.") } else { msg }, true),
            Err(err) => (err, false),
        });
    }

    if let Some((msg, ok)) = &st.card2_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

#[allow(dead_code)]
pub(crate) fn card_cover_body_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    let max_w = ui.available_width();
    ui.set_max_width(max_w);
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
        let cur_style = match super::ai_cover::style_name(st.cover_style) {
            Some(name) => name,
            None => styles.get(st.cover_style).copied().unwrap_or("Wave Curves").to_string(),
        };
        // The dock can be as narrow as 220 pt: the dropdown takes what the row has left.
        let combo_w = (ui.available_width() - 80.0).clamp(110.0, 200.0);
        let prev_style = st.cover_style;
        let combo = egui::ComboBox::from_id_salt("pubgen_cover_style").selected_text(cur_style.as_str()).width(combo_w).show_ui(ui, |ui| {
            for (idx, s) in styles.iter().enumerate() {
                let resp = ui.selectable_value(&mut st.cover_style, idx, *s);
                resp.on_hover_ui(|ui| {
                    show_style_thumbnail_tooltip(ui, idx, s, t);
                });
            }
            ui.separator();
            for (i, p) in super::ai_cover::AI_PRESETS.iter().enumerate() {
                let sid = super::ai_cover::AI_STYLE_BASE + i;
                let resp = ui.selectable_value(&mut st.cover_style, sid, format!("AI {} (online)", p.name));
                resp.on_hover_ui(|ui| {
                    show_style_thumbnail_tooltip(ui, sid, p.name, t);
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
            show_style_thumbnail_tooltip(ui, st.cover_style, &cur_style, t);
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
            show_style_thumbnail_tooltip(ui, st.cover_style, &cur_style, t);
        });
    });

    ui.checkbox(&mut st.cover_include_inside_back, "Add Inside cover and Back Cover").on_hover_text(
        "When checked, automatically generates Inside Cover (grayscale & outlines) on page 2 and Back Cover at the end of the publication",
    );

    ui.checkbox(&mut st.cover_add_map, "Add Map to Cover").on_hover_text(
        "When checked, overlays a stylized geographic vector map on the cover page",
    );
    if st.cover_add_map {
        ui.indent("pubgen_cover_map_sub", |ui| {
            let stock_maps = super::charts::list_stock_cover_maps();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Map:").size(10.5).color(t.text));
                let cur_label = if st.cover_map_stock_idx == 999 {
                    "Custom GeoJSON...".to_string()
                } else if let Some(m) = stock_maps.get(st.cover_map_stock_idx) {
                    m.display_name.clone()
                } else {
                    stock_maps.first().map(|m| m.display_name.clone()).unwrap_or_else(|| "Pangkalan (Cover / Kecamatan)".to_string())
                };

                let combo_w = (ui.available_width() - 50.0).clamp(110.0, 220.0);
                craft_ui::ComboBox::from_id_salt("pubgen_cover_map_picker")
                    .selected_text(cur_label)
                    .width(combo_w)
                    .show_ui(ui, |ui| {
                        for (idx, m) in stock_maps.iter().enumerate() {
                            ui.selectable_value(&mut st.cover_map_stock_idx, idx, &m.display_name);
                        }
                        ui.separator();
                        ui.selectable_value(&mut st.cover_map_stock_idx, 999, "Custom GeoJSON...");
                    });
            });

            if st.cover_map_stock_idx == 999 {
                ui.horizontal(|ui| {
                    let w = (ui.available_width() - 65.0).max(80.0);
                    ui.add_sized(
                        [w, crate::widgets::FIELD_H],
                        egui::TextEdit::singleline(&mut st.cover_map_custom_path).hint_text("Path to .geojson file"),
                    );
                    if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                        let ctx = ui.ctx().clone();
                        let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                            super::with_state(&ctx, |st| st.cover_map_custom_path = path);
                            Ok(serde_json::Value::Null)
                        });
                    }
                });
            }

            ui.horizontal(|ui| {
                ui.label(RichText::new("Style:").size(10.5).color(t.text));
                ui.radio_value(&mut st.cover_map_style, 0, "Watermark");
                ui.radio_value(&mut st.cover_map_style, 1, "Accent Hero");
                ui.radio_value(&mut st.cover_map_style, 2, "Outlines");
            });
        });
    }

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
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                    let ctx = ui.ctx().clone();
                    let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                        super::with_state(&ctx, |st| st.cover_image_skyline = path);
                        Ok(serde_json::Value::Null)
                    });
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
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                    let ctx = ui.ctx().clone();
                    let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                        super::with_state(&ctx, |st| st.cover_image_landscape = path);
                        Ok(serde_json::Value::Null)
                    });
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
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                    let ctx = ui.ctx().clone();
                    let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                        super::with_state(&ctx, |st| st.cover_image_skyline = path);
                        Ok(serde_json::Value::Null)
                    });
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
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                    let ctx = ui.ctx().clone();
                    let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                        super::with_state(&ctx, |st| st.cover_image_skyline = path);
                        Ok(serde_json::Value::Null)
                    });
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
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                    let ctx = ui.ctx().clone();
                    let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                        super::with_state(&ctx, |st| st.cover_image_skyline = path);
                        Ok(serde_json::Value::Null)
                    });
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
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                    let ctx = ui.ctx().clone();
                    let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                        super::with_state(&ctx, |st| st.cover_image_landscape = path);
                        Ok(serde_json::Value::Null)
                    });
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
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                    let ctx = ui.ctx().clone();
                    let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                        super::with_state(&ctx, |st| st.cover_image_skyline = path);
                        Ok(serde_json::Value::Null)
                    });
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
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                    let ctx = ui.ctx().clone();
                    let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                        super::with_state(&ctx, |st| st.cover_image_skyline = path);
                        Ok(serde_json::Value::Null)
                    });
                }
            });
        }
        s if s >= super::ai_cover::AI_STYLE_BASE => {
            // AI Base styles: palette direction + Cloudflare key setup instead of image pickers.
            super::ai_cover::ai_cover_options_pane(app, ui, st, t);
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
                if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                    let ctx = ui.ctx().clone();
                    let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                        super::with_state(&ctx, |st| st.cover_image_skyline = path);
                        Ok(serde_json::Value::Null)
                    });
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
        if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
            let ctx = ui.ctx().clone();
            let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                super::with_state(&ctx, |st| st.cover_image_logo = path);
                Ok(serde_json::Value::Null)
            });
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
            if ui.add_sized([60.0, crate::widgets::FIELD_H], egui::Button::new("Browse…")).clicked() {
                let ctx = ui.ctx().clone();
                let _ = app.ask_open("place", crate::app::file_dialogs::Bound::Document, move |_app, path| {
                    super::with_state(&ctx, |st| st.cover_image_back = path);
                    Ok(serde_json::Value::Null)
                });
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

    // Replace mode: swap the existing cover page(s) for regenerated ones instead
    // of adding a new cover page in front of them.
    ui.checkbox(&mut st.cover_replace, "Replace existing cover").on_hover_text(
        "When checked, the existing cover page(s) are deleted and regenerated in place instead of adding a new cover page in front. \
It detects what is already there - front cover only, or a full set (front, inside and back cover) - and the replacement matches the Add Inside cover and Back Cover setting: checked regenerates all three, unchecked leaves only a front cover. \
When there is no cover page yet, this behaves the same as unchecked.",
    );

    // Action button
    let fw = full_width(ui);
    let btn_label = if st.cover_include_inside_back { "Generate Cover Pages (Front, Inside, Back)" } else { "Generate Cover Page" };
    let is_ai = super::ai_cover::preset_index(st.cover_style).is_some();
    let job_running = st.cover_job.as_ref().is_some_and(|j| !j.finished);

    let (btn_enabled, disabled_hint) = if job_running {
        (false, "Generating AI cover in background\u{2026}")
    } else if is_ai && !designcraft_imagegen::compiled_in() {
        (false, "This build has no online image generation (compile with --features ai-cover-online)")
    } else if is_ai && !super::ai_cover::AiCoverSetupDialog::key_configured(app) {
        (false, "AI Base needs a Cloudflare account: click AI Setup\u{2026} above to set up.")
    } else {
        (true, "")
    };

    let gen_btn = ui
        .add_enabled_ui(btn_enabled, |ui| outline_button(ui, btn_label, fw))
        .inner
        .on_hover_text("Automatically inserts cover pages and generates layout");

    let gen_btn = if !btn_enabled && !disabled_hint.is_empty() { gen_btn.on_disabled_hover_text(disabled_hint) } else { gen_btn };

    if gen_btn.clicked() {
        covers::trigger_generate_cover(app, st, ui.ctx().clone());
    }

    // Shared Supabase counter (remaining neurons / 96), persisted in state after the
    // reporting job finishes so the label stays visible under the button.
    if is_ai && let Some(n) = st.cover_quota_covers {
        ui.label(RichText::new(format!("Cloudflare AI: ~{n} covers left today")).size(10.0).color(t.text_dim));
    }

    if let Some(pending) = &st.cover_batch_pending {
        let count = pending.len();
        ui.group(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} of {} fit today's free quota; the rest will fail after ~{} — continue?",
                    designcraft_imagegen::MAX_SAFE_FREE_BATCH,
                    count,
                    designcraft_imagegen::MAX_SAFE_FREE_BATCH
                ))
                .size(11.0)
                .color(Color32::from_rgb(220, 160, 40)),
            );
            ui.horizontal(|ui| {
                if ui.button("Continue Anyway").clicked()
                    && let Some(items) = st.cover_batch_pending.take()
                {
                    covers::trigger_generate_cover_batch_confirmed(app, st, items, ui.ctx().clone());
                }
                if ui.button("Cancel").clicked() {
                    st.cover_batch_pending = None;
                    st.cover_status = Some(("Batch cover generation cancelled".into(), false));
                }
            });
        });
    }

    if let Some(job) = &st.cover_job
        && !job.finished
    {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(12.0));
            ui.label(RichText::new("Generating AI cover\u{2026}").size(10.5).color(t.text));
            if ui.button("Cancel").on_hover_text("Cancel AI cover generation").clicked() {
                job.cancel();
                st.cover_status = Some(("AI cover generation cancelled".into(), false));
            }
        });
    }

    if let Some((msg, ok)) = &st.cover_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

/// Modal popup after an AI operation completes (Groq translation or Cloudflare AI
/// cover generation), reporting the remaining usage quota. Centered, on top of
/// everything; closed by the OK button, the backdrop, or Escape.
pub(crate) fn ai_quota_popup_ui(st: &mut PubgenState, ui: &mut Ui, t: &Tokens) {
    if st.ai_quota_popup.is_none() {
        return;
    }
    let mut close = false;
    let modal = egui::Modal::new(egui::Id::unique("pubgen_ai_quota_popup")).show(ui.ctx(), |ui| {
        ui.set_width(340.0);
        ui.vertical(|ui| {
            ui.label(RichText::new("AI usage update").size(13.0).color(t.text).strong());
            ui.add_space(3.0);
            if let Some(lines) = &st.ai_quota_popup {
                for line in lines {
                    ui.label(RichText::new(line).size(11.0).color(t.text));
                }
            }
        });
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("OK").clicked() {
                close = true;
            }
        });
    });
    if close || modal.should_close() {
        st.ai_quota_popup = None;
    }
}

pub(crate) fn card_toc_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
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

pub(crate) fn manual_import_accordion_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
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

    divider(ui);

    egui::CollapsingHeader::new(crate::rtl::widget(
        ui,
        egui::RichText::new(crate::i18n::tr(&app.ui.language, "Vector Chart (Add or Edit Chart)")).size(11.0).color(t.text_strong),
    ))
    .id_salt("pubgen_collapse_chart")
    .default_open(true)
    .show(ui, |ui| {
        card_chart_body_ui(app, ui, st, t);
    });
}

pub(crate) fn card_chart_body_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    let lang = app.ui.language.clone();
    let has_doc = app.session.active().is_some();
    let cur_page = crate::canvas::current_page(app).unwrap_or(0);
    let cur_spread = app.session.active().and_then(|a| a.doc.page_loc(cur_page)).map_or(0, |(si, _)| si);

    // Check if current spread has a chart
    let has_chart_on_spread = app.session.active().is_some_and(|active| {
        active.doc.spreads.get(cur_spread).is_some_and(|sp| {
            sp.items.iter().any(|it| {
                if let designcraft_doc::Content::Text(tf) = &it.content
                    && let Some(story) = active.doc.story(tf.story)
                {
                    let text = story.text.trim();
                    text.starts_with("Grafik") || text.starts_with("Figure")
                } else {
                    false
                }
            })
        })
    });

    let status_str = if !has_doc {
        crate::i18n::tr(&lang, "No document open").to_string()
    } else if has_chart_on_spread {
        format!("{} (Spread {})", crate::i18n::tr(&lang, "Chart detected on current spread"), cur_spread + 1)
    } else {
        format!("{} (Spread {})", crate::i18n::tr(&lang, "No chart on current spread"), cur_spread + 1)
    };

    ui.label(RichText::new(status_str).size(10.5).color(if has_chart_on_spread { egui::Color32::from_rgb(46, 125, 50) } else { t.text_dim }));

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        let add_btn = ui.add_enabled(has_doc, egui::Button::new(RichText::new(crate::i18n::tr(&lang, "➕ Add New Chart…")).size(11.0)));
        if add_btn.clicked() {
            st.chart_dialog = Some(Box::new(super::charts::dialog::ChartDialog::new(app, false)));
        }

        let edit_btn = ui.add_enabled(
            has_doc && has_chart_on_spread,
            egui::Button::new(RichText::new(crate::i18n::tr(&lang, "✏️ Edit Current Chart…")).size(11.0)),
        );
        if edit_btn.clicked() {
            st.chart_dialog = Some(Box::new(super::charts::dialog::ChartDialog::new(app, true)));
        }
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new(crate::i18n::tr(&lang, "Supports Bar, Stacked, Horizontal, Line, Area, and Donut with BPS palette & bilingual titles."))
            .size(9.5)
            .color(t.text_dim),
    );
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    use super::*;

    fn popup_harness(st: PubgenState) -> Harness<'static, PubgenState> {
        let mut h = Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui_state(
            |ui, st: &mut PubgenState| {
                let t = crate::theme::Tokens::get(ui.ctx());
                ai_quota_popup_ui(st, ui, &t);
            },
            st,
        );
        h.run_steps(4);
        h
    }

    /// The usage popup renders its title, every queued line and the OK button, and
    /// OK closes it.
    #[test]
    fn the_ai_quota_popup_renders_and_closes_on_ok() {
        let st = PubgenState {
            ai_quota_popup: Some(vec![
                "Cloudflare AI: ~103 covers left today".into(),
                "Groq AI: ~990 requests left today".into(),
                "AI cover generation cancelled".into(),
            ]),
            ..Default::default()
        };
        let mut h = popup_harness(st);
        for want in
            ["AI usage update", "Cloudflare AI: ~103 covers left today", "Groq AI: ~990 requests left today", "AI cover generation cancelled", "OK"]
        {
            let _ = h.get_by_label(want);
        }
        h.get_by_label("OK").click();
        h.run_steps(2);
        assert!(h.state().ai_quota_popup.is_none(), "OK closed the popup");
    }

    /// Escape (or the backdrop, via `should_close`) also closes the popup.
    #[test]
    fn the_ai_quota_popup_closes_on_escape() {
        let st = PubgenState { ai_quota_popup: Some(vec!["Cloudflare AI: ~103 covers left today".into()]), ..Default::default() };
        let mut h = popup_harness(st);
        h.input_mut().events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(2);
        assert!(h.state().ai_quota_popup.is_none(), "Escape closed the popup");
    }
}
