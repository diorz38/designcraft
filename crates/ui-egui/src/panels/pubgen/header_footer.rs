//! The header/footer card: template picking, preview and the apply/remove/save/delete engines.

use super::*;

/// Resolves title and release number lines for BRS publication running headers.
pub fn resolve_brs_header_texts(st: &PubgenState) -> (String, String) {
    let cov_opt = st.excel_cover.as_ref();
    let raw_title = cov_opt.map(|c| c.title1.as_str()).filter(|s| !s.trim().is_empty()).unwrap_or(if st.cover_title.trim().is_empty() {
        "Perkembangan Nilai Tukar Petani"
    } else {
        st.cover_title.trim()
    });
    let raw_month = cov_opt
        .map(|c| c.release_month.as_str())
        .filter(|s| !s.trim().is_empty())
        .or_else(|| cov_opt.map(|c| c.release_date.as_str()).filter(|s| !s.trim().is_empty()))
        .unwrap_or("");
    let fmt_month = super::covers::format_indonesian_month_year(raw_month);

    let title_line = if !fmt_month.is_empty() && !raw_title.to_lowercase().contains(&fmt_month.to_lowercase()) {
        let trimmed = raw_title.trim_end_matches(',').trim();
        format!("{trimmed}, {fmt_month}")
    } else {
        raw_title.to_string()
    };

    let release_no = cov_opt.map(|c| c.release_no.as_str()).unwrap_or("").trim();
    let release_date = cov_opt
        .map(|c| c.release_date.as_str())
        .filter(|s| !s.trim().is_empty())
        .or_else(|| cov_opt.map(|c| c.release_month.as_str()).filter(|s| !s.trim().is_empty()))
        .unwrap_or("")
        .trim();
    let release_info = if !release_no.is_empty() {
        let fmt_date = super::covers::format_indonesian_date(release_date);
        let prefix = if release_no.to_lowercase().starts_with("no") { "" } else { "No. " };
        if !fmt_date.is_empty() { format!("{prefix}{release_no}, {fmt_date}") } else { format!("{prefix}{release_no}") }
    } else {
        "No. 61/10/32/Th. XXVIII, 1 Oktober 2026".to_string()
    };

    let nopub_line = if release_info.to_uppercase().starts_with("BRS") { release_info } else { format!("BRS {release_info}") };

    (title_line, nopub_line)
}

fn load_hf_template_to_state(st: &mut PubgenState, tpl: &Value) {
    if let Some(name) = tpl.get("name").and_then(Value::as_str) {
        st.hf_style = name.to_string();
        if name.eq_ignore_ascii_case("hf-brs") || name.eq_ignore_ascii_case("brs") {
            st.hf_sided = 0;
            st.hf_header_offset = tpl.get("headerOffset").and_then(Value::as_f64).unwrap_or(24.0);
            st.hf_footer_offset = tpl.get("footerOffset").and_then(Value::as_f64).unwrap_or(24.0);
            let (title, nopub) = resolve_brs_header_texts(st);
            let combined = format!("{title}\n{nopub}");
            st.hf_header_single.clear();
            st.hf_header_left.clear();
            st.hf_header_right.clear();
            st.hf_footer_single = combined.clone();
            st.hf_footer_left = combined.clone();
            st.hf_footer_right = combined;
            st.hf_header_rule = false;
            st.hf_footer_rule = false;
            st.hf_top_accent_bar = false;
            st.hf_color_primary = "#f68839".to_string();
            st.hf_color_text = "#f68839".to_string();
            return;
        }
    }
    if let Some(sided) = tpl.get("sided").and_then(Value::as_str) {
        st.hf_sided = if sided.eq_ignore_ascii_case("single") { 1 } else { 0 };
    }
    if let Some(ho) = tpl.get("headerOffset").and_then(Value::as_f64) {
        st.hf_header_offset = ho;
    }
    if let Some(fo) = tpl.get("footerOffset").and_then(Value::as_f64) {
        st.hf_footer_offset = fo;
    }
    if let Some(h) = tpl.get("header") {
        st.hf_header_single = h.get("single").and_then(Value::as_str).unwrap_or("").to_string();
        st.hf_header_left = h.get("left").and_then(Value::as_str).unwrap_or("").to_string();
        st.hf_header_right = h.get("right").and_then(Value::as_str).unwrap_or("").to_string();
    }
    if let Some(f) = tpl.get("footer") {
        st.hf_footer_single = f.get("single").and_then(Value::as_str).unwrap_or("").to_string();
        st.hf_footer_left = f.get("left").and_then(Value::as_str).unwrap_or("").to_string();
        st.hf_footer_right = f.get("right").and_then(Value::as_str).unwrap_or("").to_string();
    }
    if let Some(obj) = tpl.get("objects") {
        st.hf_header_rule = obj.get("headerRule").and_then(Value::as_bool).unwrap_or(false);
        st.hf_header_rule_weight = obj.get("headerRuleWeight").and_then(Value::as_f64).unwrap_or(0.5);
        st.hf_footer_rule = obj.get("footerRule").and_then(Value::as_bool).unwrap_or(false);
        st.hf_footer_rule_weight = obj.get("footerRuleWeight").and_then(Value::as_f64).unwrap_or(0.5);
        st.hf_top_accent_bar = obj.get("topAccentBar").and_then(Value::as_bool).unwrap_or(false);
        st.hf_top_accent_bar_height = obj.get("topAccentBarHeight").and_then(Value::as_f64).unwrap_or(4.0);
        st.hf_top_accent_bar_bleed = obj.get("topAccentBarBleed").and_then(Value::as_bool).unwrap_or(true);
    }
    if let Some(cols) = tpl.get("colors") {
        st.hf_color_primary = cols.get("primary").and_then(Value::as_str).unwrap_or("#0072bc").to_string();
        st.hf_color_accent = cols.get("accent").and_then(Value::as_str).unwrap_or("#f5ab19").to_string();
        st.hf_color_dark = cols.get("dark").and_then(Value::as_str).unwrap_or("#122647").to_string();
        st.hf_color_rule = cols.get("ruleColor").and_then(Value::as_str).unwrap_or("#cbd5e1").to_string();
        st.hf_color_text = cols.get("textColor").and_then(Value::as_str).unwrap_or("#334155").to_string();
    }
}

fn show_hf_preview_tooltip(ui: &mut Ui, st: &PubgenState, name: &str, t: &Tokens) {
    ui.vertical(|ui| {
        ui.label(RichText::new(name).size(12.0).strong().color(t.text_strong));
        ui.add_space(4.0);

        let (rect, _) = ui.allocate_exact_size(vec2(160.0, 110.0), Sense::hover());
        // Page background
        ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(250, 250, 250));
        ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, Color32::from_rgb(200, 200, 200)), egui::StrokeKind::Outside);

        if name.eq_ignore_ascii_case("hf-brs") || name.eq_ignore_ascii_case("brs") {
            let spine_x = rect.center().x;
            ui.painter().line_segment([pos2(spine_x, rect.min.y), pos2(spine_x, rect.max.y)], egui::Stroke::new(1.0, Color32::from_gray(210)));

            let brs_orange = Color32::from_rgb(245, 136, 56);
            let footer_y = rect.max.y - 20.0;

            // Faint body lines on both pages
            let top_body_y = rect.min.y + 14.0;
            for i in 0..5 {
                let by = top_body_y + (i as f32) * 8.0;
                ui.painter().line_segment([pos2(rect.min.x + 8.0, by), pos2(spine_x - 8.0, by)], egui::Stroke::new(1.0, Color32::from_gray(230)));
                ui.painter().line_segment([pos2(spine_x + 8.0, by), pos2(rect.max.x - 8.0, by)], egui::Stroke::new(1.0, Color32::from_gray(230)));
            }

            // Verso (Left page): 2-line text on left (right-aligned), large folio on right in footer
            let left_text_x = spine_x - 32.0;
            ui.painter().text(
                pos2(left_text_x, footer_y),
                egui::Align2::RIGHT_TOP,
                "Title Line\nRelease No",
                egui::FontId::proportional(7.0),
                t.text_dim,
            );
            ui.painter().text(pos2(spine_x - 10.0, footer_y - 2.0), egui::Align2::RIGHT_TOP, "9", egui::FontId::proportional(16.0), brs_orange);

            // Recto (Right page): large folio on left, 2-line text on right (left-aligned) in footer
            ui.painter().text(pos2(spine_x + 10.0, footer_y - 2.0), egui::Align2::LEFT_TOP, "10", egui::FontId::proportional(16.0), brs_orange);
            let right_text_x = spine_x + 32.0;
            ui.painter().text(
                pos2(right_text_x, footer_y),
                egui::Align2::LEFT_TOP,
                "Title Line\nRelease No",
                egui::FontId::proportional(7.0),
                t.text_dim,
            );

            ui.add_space(3.0);
            ui.label(
                RichText::new("BPS Berita Resmi Statistik running footer with prominent folios and two-line metadata (blank header).")
                    .size(10.0)
                    .color(t.text_dim),
            );
            return;
        }

        let parse_color = |s: &str, def: Color32| {
            if let Some(c) = designcraft_color::Color::from_hex(s) {
                let [r, g, b, _] = c.to_rgba8(1.0);
                Color32::from_rgb(r, g, b)
            } else {
                def
            }
        };

        let primary_color = parse_color(&st.hf_color_primary, Color32::from_rgb(0, 114, 188));
        let rule_color = parse_color(&st.hf_color_rule, Color32::from_rgb(200, 200, 200));

        // 1. Top accent bar
        if st.hf_top_accent_bar {
            let bar_rect = egui::Rect::from_min_size(rect.min, vec2(rect.width(), 4.0));
            ui.painter().rect_filled(bar_rect, 0.0, primary_color);
        }

        // 2. Header text & rule
        let header_y = rect.min.y + 14.0;
        let left_x = rect.min.x + 8.0;
        let right_x = rect.max.x - 8.0;

        if !st.hf_header_left.is_empty() || !st.hf_header_single.is_empty() {
            ui.painter().text(pos2(left_x, header_y), egui::Align2::LEFT_CENTER, "Headline", egui::FontId::proportional(8.0), t.text_dim);
        }
        if !st.hf_header_right.is_empty() {
            ui.painter().text(pos2(right_x, header_y), egui::Align2::RIGHT_CENTER, "Section", egui::FontId::proportional(8.0), t.text_dim);
        }
        if st.hf_header_rule {
            ui.painter().line_segment([pos2(left_x, header_y + 6.0), pos2(right_x, header_y + 6.0)], egui::Stroke::new(1.0, rule_color));
        }

        // Faint body lines
        let body_y0 = header_y + 14.0;
        for i in 0..4 {
            let by = body_y0 + (i as f32) * 8.0;
            let bw = if i == 3 { rect.width() * 0.45 } else { rect.width() - 24.0 };
            ui.painter().line_segment([pos2(left_x + 4.0, by), pos2(left_x + 4.0 + bw, by)], egui::Stroke::new(1.0, Color32::from_gray(230)));
        }

        // 3. Footer text & rule
        let footer_y = rect.max.y - 12.0;
        if st.hf_footer_rule {
            ui.painter().line_segment([pos2(left_x, footer_y - 6.0), pos2(right_x, footer_y - 6.0)], egui::Stroke::new(1.0, rule_color));
        }
        ui.painter().text(pos2(left_x, footer_y), egui::Align2::LEFT_CENTER, "12", egui::FontId::proportional(8.0), t.text_dim);
        ui.painter().text(pos2(right_x, footer_y), egui::Align2::RIGHT_CENTER, "Document", egui::FontId::proportional(8.0), t.text_dim);

        ui.add_space(3.0);
        let desc = match name {
            "Modern Minimal" => "Clean, understated page number with subtle footer divider.",
            "Editorial Book" => "Refined running headers with hairline rule, ideal for magazines and books.",
            "Corporate Report" => "Full-width top accent bar, running headers, and confidentiality footer.",
            _ => "Custom user header & footer template.",
        };
        ui.label(RichText::new(desc).size(10.0).color(t.text_dim));
    });
}

#[allow(dead_code)]
fn card_header_footer_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    section(ui, crate::i18n::tr(&app.ui.language, "Header & Footer Template"), true);
    card_header_footer_body_ui(app, ui, st, t);
}

pub(crate) fn card_header_footer_body_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
    // Fetch all templates (built-in + user custom) from engine
    let templates = designcraft_engine::cmd::header_footer::all_templates(&app.session);

    // Lazy initialize from default or current doc if not yet initialized
    if !st.hf_initialized {
        st.hf_initialized = true;
        if let Some(doc_hf) = app.session.active().and_then(|s| s.doc.header_footer.as_ref()) {
            st.hf_style = doc_hf.preset.clone();
            st.hf_sided = if doc_hf.sided == designcraft_doc::Sided::Single { 1 } else { 0 };
            st.hf_header_offset = doc_hf.header_offset;
            st.hf_footer_offset = doc_hf.footer_offset;
            st.hf_header_single = doc_hf.header.single.clone();
            st.hf_header_left = doc_hf.header.left.clone();
            st.hf_header_right = doc_hf.header.right.clone();
            st.hf_footer_single = doc_hf.footer.single.clone();
            st.hf_footer_left = doc_hf.footer.left.clone();
            st.hf_footer_right = doc_hf.footer.right.clone();
            st.hf_header_rule = doc_hf.objects.header_rule;
            st.hf_header_rule_weight = doc_hf.objects.header_rule_weight;
            st.hf_footer_rule = doc_hf.objects.footer_rule;
            st.hf_footer_rule_weight = doc_hf.objects.footer_rule_weight;
            st.hf_top_accent_bar = doc_hf.objects.top_accent_bar;
            st.hf_top_accent_bar_height = doc_hf.objects.top_accent_bar_height;
            st.hf_top_accent_bar_bleed = doc_hf.objects.top_accent_bar_bleed;
            st.hf_color_primary = doc_hf.colors.primary.clone();
            st.hf_color_accent = doc_hf.colors.accent.clone();
            st.hf_color_dark = doc_hf.colors.dark.clone();
            st.hf_color_rule = doc_hf.colors.rule_color.clone();
            st.hf_color_text = doc_hf.colors.text_color.clone();
        } else if let Some(first_tpl) = templates.first() {
            load_hf_template_to_state(st, first_tpl);
        }
    }

    // 1. Template Style Dropdown with Hover Preview Tooltip & Eye Icon
    ui.horizontal(|ui| {
        ui.label(RichText::new("Style:").size(11.0).color(t.text));

        let cur_style = st.hf_style.clone();
        let combo_w = (ui.available_width() - 80.0).clamp(110.0, 200.0);
        let mut picked_template: Option<Value> = None;

        let is_brs_active = st.active_profile().id == "bps_brs";
        let combo = craft_ui::ComboBox::from_id_salt("pubgen_hf_style").selected_text(&cur_style).width(combo_w).show_ui(ui, |ui| {
            for tpl in &templates {
                let name = tpl.get("name").and_then(Value::as_str).unwrap_or("Template");
                if name.eq_ignore_ascii_case("hf-brs") && !is_brs_active && !cur_style.eq_ignore_ascii_case("hf-brs") {
                    continue;
                }
                let is_builtin = tpl.get("builtIn").and_then(Value::as_bool).unwrap_or(false);
                let label = if is_builtin { name.to_string() } else { format!("{name} ★") };
                let is_selected = cur_style == name;
                let resp = ui.selectable_label(is_selected, label);
                if resp.clicked() && !is_selected {
                    picked_template = Some(tpl.clone());
                }
                resp.on_hover_ui(|ui| {
                    show_hf_preview_tooltip(ui, st, name, t);
                });
            }
        });

        if let Some(tpl) = picked_template {
            load_hf_template_to_state(st, &tpl);
        }

        combo.response.on_hover_ui(|ui| {
            show_hf_preview_tooltip(ui, st, &cur_style, t);
        });

        // Eye preview button
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
            show_hf_preview_tooltip(ui, st, &cur_style, t);
        });

        // If current template is custom (not built-in), show a small delete button
        let cur_is_builtin = templates
            .iter()
            .any(|tpl| tpl.get("name").and_then(Value::as_str) == Some(&cur_style) && tpl.get("builtIn").and_then(Value::as_bool) == Some(true));
        if !cur_is_builtin && !cur_style.is_empty() {
            let del_btn = ui.button("🗑").on_hover_text("Delete this custom template");
            if del_btn.clicked() {
                match execute_delete_hf_template(app, st, &cur_style) {
                    Ok(msg) => st.hf_status = Some((msg, true)),
                    Err(err) => st.hf_status = Some((err, false)),
                }
            }
        }
    });

    // 2. Sidedness & Offsets
    ui.horizontal(|ui| {
        ui.label(RichText::new("Sided:").size(11.0).color(t.text));
        let sided_options = ["Double-sided", "Single-sided"];
        let cur_s = sided_options.get(st.hf_sided).copied().unwrap_or("Double-sided");
        craft_ui::ComboBox::from_id_salt("pubgen_hf_sided").selected_text(cur_s).width(95.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut st.hf_sided, 0, "Double-sided");
            ui.selectable_value(&mut st.hf_sided, 1, "Single-sided");
        });

        ui.label(RichText::new("H.Off:").size(10.5).color(t.text_dim));
        ui.add(egui::DragValue::new(&mut st.hf_header_offset).range(0.0..=120.0).speed(0.5).suffix(" pt"))
            .on_hover_text("Header offset from top of page");

        ui.label(RichText::new("F.Off:").size(10.5).color(t.text_dim));
        ui.add(egui::DragValue::new(&mut st.hf_footer_offset).range(0.0..=120.0).speed(0.5).suffix(" pt"))
            .on_hover_text("Footer offset from bottom of page");
    });

    // 2b. Scope: which pages this template applies to (all, one section, or a page range).
    ui.horizontal(|ui| {
        ui.label(RichText::new("Scope:").size(11.0).color(t.text));
        let (page_count, sections) = app
            .session
            .active()
            .map(|a| (a.doc.page_count(), a.doc.sections.iter().map(|s| (s.prefix.clone(), s.start)).collect::<Vec<_>>()))
            .unwrap_or((1, Vec::new()));
        let scope_options = ["All Pages", "Section", "Pages"];
        let cur_scope = scope_options.get(st.hf_scope).copied().unwrap_or("All Pages");
        craft_ui::ComboBox::from_id_salt("pubgen_hf_scope").selected_text(cur_scope).width(85.0).show_ui(ui, |ui| {
            ui.selectable_value(&mut st.hf_scope, 0, "All Pages");
            ui.selectable_value(&mut st.hf_scope, 1, "Section");
            ui.selectable_value(&mut st.hf_scope, 2, "Pages");
        });
        match st.hf_scope {
            1 => {
                if st.hf_scope_section >= sections.len() {
                    st.hf_scope_section = 0;
                }
                let sec_label = |i: usize, s: &(String, usize)| {
                    if s.0.is_empty() { format!("Section {} (page {})", i + 1, s.1 + 1) } else { format!("{} (page {})", s.0, s.1 + 1) }
                };
                let cur_sec = sections
                    .get(st.hf_scope_section)
                    .map_or_else(|| format!("Section {}", st.hf_scope_section + 1), |s| sec_label(st.hf_scope_section, s));
                craft_ui::ComboBox::from_id_salt("pubgen_hf_scope_section").selected_text(cur_sec).width(130.0).show_ui(ui, |ui| {
                    for (i, s) in sections.iter().enumerate() {
                        if ui.selectable_label(st.hf_scope_section == i, sec_label(i, s)).clicked() {
                            st.hf_scope_section = i;
                        }
                    }
                });
            }
            2 => {
                let start = st.hf_scope_start.clamp(1, page_count.max(1));
                let end = st.hf_scope_end.clamp(start, page_count.max(1));
                (st.hf_scope_start, st.hf_scope_end) = (start, end);
                ui.add(egui::DragValue::new(&mut st.hf_scope_start).range(1..=page_count.max(1)).prefix("From: "));
                ui.add(egui::DragValue::new(&mut st.hf_scope_end).range(st.hf_scope_start..=page_count.max(1)).prefix("To: "));
            }
            _ => {}
        }
        if ui.button(RichText::new("Sections…").size(10.5).color(t.text_dim)).clicked() {
            crate::menus::activate(app, "layout.section", &Value::Null);
        }
    });

    // 3. Header Text Fields
    if st.hf_sided == 0 {
        // Double-sided headers
        ui.horizontal(|ui| {
            ui.label(RichText::new("H.Left:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_header_left).hint_text("{page} | Title"))
                .on_hover_text("Left (verso) header text. Tokens: {page}, {section}, {filename}, {runningHeader}");
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("H.Right:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_header_right).hint_text("Title | {page}"))
                .on_hover_text("Right (recto) header text. Tokens: {page}, {section}, {filename}, {runningHeader}");
        });
    } else {
        // Single-sided header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Header:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_header_single).hint_text("{page} | Document"))
                .on_hover_text("Single-sided header text. Tokens: {page}, {section}, {filename}, {runningHeader}");
        });
    }

    // 4. Footer Text Fields
    if st.hf_sided == 0 {
        // Double-sided footers
        ui.horizontal(|ui| {
            ui.label(RichText::new("F.Left:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_footer_left).hint_text("{page}"))
                .on_hover_text("Left (verso) footer text");
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("F.Right:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_footer_right).hint_text("Confidential"))
                .on_hover_text("Right (recto) footer text");
        });
    } else {
        // Single-sided footer
        ui.horizontal(|ui| {
            ui.label(RichText::new("Footer:").size(10.5).color(t.text));
            let avail = (ui.available_width() - 4.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_footer_single).hint_text("{page}"))
                .on_hover_text("Single-sided footer text");
        });
    }

    // 5. Decorative Objects (Rules & Accent Bar)
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.checkbox(&mut st.hf_header_rule, RichText::new("H.Rule").size(10.5).color(t.text)).on_hover_text("Thin hairline rule below header text");
        if st.hf_header_rule {
            ui.add(egui::DragValue::new(&mut st.hf_header_rule_weight).range(0.25..=6.0).speed(0.25).suffix("pt"));
        }

        ui.checkbox(&mut st.hf_footer_rule, RichText::new("F.Rule").size(10.5).color(t.text)).on_hover_text("Thin hairline rule above footer text");
        if st.hf_footer_rule {
            ui.add(egui::DragValue::new(&mut st.hf_footer_rule_weight).range(0.25..=6.0).speed(0.25).suffix("pt"));
        }
    });

    ui.horizontal(|ui| {
        ui.checkbox(&mut st.hf_top_accent_bar, RichText::new("Top Accent Bar").size(10.5).color(t.text))
            .on_hover_text("Colored geometric accent bar across the top page edge");
        if st.hf_top_accent_bar {
            ui.add(egui::DragValue::new(&mut st.hf_top_accent_bar_height).range(1.0..=36.0).speed(0.5).suffix("pt"))
                .on_hover_text("Height of top accent bar");
            ui.checkbox(&mut st.hf_top_accent_bar_bleed, RichText::new("Bleed").size(10.0).color(t.text_dim))
                .on_hover_text("Extend accent bar into bleed area");
        }
    });

    // 6. Template Colors
    ui.horizontal(|ui| {
        ui.label(RichText::new("Colors:").size(10.5).color(t.text_dim));
        let field_w = ((ui.available_width() - 40.0) / 4.0 - 4.0).clamp(38.0, 60.0);
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_color_primary))
            .on_hover_text("Primary / Accent Bar color hex (e.g. #0072bc)");
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_color_accent))
            .on_hover_text("Accent color hex (e.g. #f5ab19)");
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_color_rule))
            .on_hover_text("Hairline rule color hex (e.g. #cbd5e1)");
        ui.add_sized([field_w, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_color_text))
            .on_hover_text("Text color hex (e.g. #334155)");
    });

    // 7. Save As New Template (Collapsible / Toggleable)
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        let toggle_text = if st.hf_show_save_row { "▾ Save As New Template…" } else { "▸ Save As New Template…" };
        if ui.button(RichText::new(toggle_text).size(10.5).color(t.text_dim)).clicked() {
            st.hf_show_save_row = !st.hf_show_save_row;
        }
    });

    if st.hf_show_save_row {
        ui.horizontal(|ui| {
            let avail = (ui.available_width() - 84.0).max(60.0);
            ui.add_sized([avail, crate::widgets::FIELD_H], egui::TextEdit::singleline(&mut st.hf_custom_name).hint_text("Template Name…"));
            let save_btn = ui.button(RichText::new("Save").size(11.0).color(t.text_strong));
            if save_btn.clicked() {
                match execute_save_hf_template(app, st) {
                    Ok(msg) => st.hf_status = Some((msg, true)),
                    Err(err) => st.hf_status = Some((err, false)),
                }
            }
        });
    }

    // 8. Action Buttons (Apply & Remove)
    ui.add_space(4.0);
    let fw = full_width(ui);
    let apply_btn =
        outline_button(ui, "Apply Header & Footer", fw).on_hover_text("Applies the current header & footer template configuration to the document");
    if apply_btn.clicked() {
        let res = execute_apply_header_footer(app, st);
        st.hf_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(err) => (err, false),
        });
    }

    ui.horizontal(|ui| {
        if ui.button(RichText::new("Remove from Document").size(10.5).color(t.text_dim)).clicked() {
            match execute_remove_header_footer(app) {
                Ok(msg) => st.hf_status = Some((msg, true)),
                Err(err) => st.hf_status = Some((err, false)),
            }
        }
    });

    // Status message
    if let Some((msg, ok)) = &st.hf_status {
        let color = if *ok { Color32::from_rgb(80, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.add(egui::Label::new(RichText::new(msg).size(11.0).color(color)).wrap());
    }
}

pub fn execute_apply_header_footer(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    // If double-sided is requested and facingPages is currently false, ensure facingPages is true
    if st.hf_sided == 0 && app.session.active().is_some_and(|s| !s.doc.settings.facing_pages) {
        app.run("layout.documentSetup", json!({ "facingPages": true })).map_err(|e| format!("Failed to switch the document to facing pages: {e}"))?;
    }

    let is_brs = st.hf_style.eq_ignore_ascii_case("hf-brs") || st.active_profile().id == "bps_brs";

    // Same shapes the Header/Footer… dialog sends: "all", {"section": 0-based index}, {"range": [first, last]}.
    let scope = match st.hf_scope {
        1 => json!({ "section": st.hf_scope_section }),
        2 => {
            let start = st.hf_scope_start.max(1) as u64;
            let end = (st.hf_scope_end as u64).max(start);
            json!({ "range": [start, end] })
        }
        _ => {
            // For BRS, running headers should not land on cover (page 1) or back cover (last page)
            if is_brs {
                if let Some(active) = app.session.active() {
                    let total = active.doc.page_count();
                    if total > 1 {
                        let has_bc = super::covers::find_back_cover_page(&active.doc).is_some();
                        let last = if has_bc && total > 2 { total - 1 } else { total };
                        json!({ "range": [2, last as u64] })
                    } else {
                        json!("all")
                    }
                } else {
                    json!("all")
                }
            } else {
                json!("all")
            }
        }
    };

    let (brs_title, brs_nopub) = resolve_brs_header_texts(st);
    if is_brs {
        let _ = app.session.edit(|doc, _| {
            if let Some(pos) = doc.text_variables.iter().position(|v| v.name == "Title") {
                doc.text_variables[pos].kind = designcraft_doc::vars::VarKind::Custom { text: brs_title.clone() };
            } else {
                doc.text_variables
                    .push(designcraft_doc::vars::TextVariable::new("Title", designcraft_doc::vars::VarKind::Custom { text: brs_title.clone() }));
            }
            if let Some(pos) = doc.text_variables.iter().position(|v| v.name == "Release No") {
                doc.text_variables[pos].kind = designcraft_doc::vars::VarKind::Custom { text: brs_nopub.clone() };
            } else {
                doc.text_variables
                    .push(designcraft_doc::vars::TextVariable::new("Release No", designcraft_doc::vars::VarKind::Custom { text: brs_nopub.clone() }));
            }
            Ok(serde_json::Value::Null)
        });
    }

    let expand_brs_vars = |text: &str, is_footer: bool| -> String {
        let mut s = text.to_string();
        if (s.is_empty() || s == "{page}") && is_brs && is_footer {
            return format!("{brs_title}\n{brs_nopub}");
        }
        if s.contains("{title}") {
            s = s.replace("{title}", &brs_title);
        }
        if s.contains("{nopub}") {
            s = s.replace("{nopub}", &brs_nopub);
        }
        if s.contains("{releaseNo}") {
            s = s.replace("{releaseNo}", &brs_nopub);
        }
        s
    };

    let header_single = if is_brs && (st.hf_header_single == "{title}\n{nopub}" || st.hf_header_single.is_empty()) {
        String::new()
    } else {
        expand_brs_vars(&st.hf_header_single, false)
    };
    let header_left = if is_brs && (st.hf_header_left == "{title}\n{nopub}" || st.hf_header_left.is_empty()) {
        String::new()
    } else {
        expand_brs_vars(&st.hf_header_left, false)
    };
    let header_right = if is_brs && (st.hf_header_right == "{title}\n{nopub}" || st.hf_header_right.is_empty()) {
        String::new()
    } else {
        expand_brs_vars(&st.hf_header_right, false)
    };

    let footer_single = expand_brs_vars(&st.hf_footer_single, true);
    let footer_left = expand_brs_vars(&st.hf_footer_left, true);
    let footer_right = expand_brs_vars(&st.hf_footer_right, true);

    let mut params = json!({
        "preset": st.hf_style,
        "sided": if st.hf_sided == 1 { "single" } else { "double" },
        "scope": scope,
        "headerOffset": st.hf_header_offset,
        "footerOffset": st.hf_footer_offset,
        "header": {
            "single": header_single,
            "left": header_left,
            "right": header_right,
        },
        "footer": {
            "single": footer_single,
            "left": footer_left,
            "right": footer_right,
        },
        "objects": {
            "headerRule": st.hf_header_rule,
            "headerRuleWeight": st.hf_header_rule_weight,
            "footerRule": st.hf_footer_rule,
            "footerRuleWeight": st.hf_footer_rule_weight,
            "topAccentBar": st.hf_top_accent_bar,
            "topAccentBarHeight": st.hf_top_accent_bar_height,
            "topAccentBarBleed": st.hf_top_accent_bar_bleed,
        },
        "colors": {
            "primary": st.hf_color_primary,
            "accent": st.hf_color_accent,
            "dark": st.hf_color_dark,
            "ruleColor": st.hf_color_rule,
            "textColor": st.hf_color_text,
        }
    });

    let doc_margins = app.session.active().map(|s| detect_document_margins(&s.doc));
    if let Some(m) = doc_margins
        && let Some(obj) = params.as_object_mut()
    {
        obj.insert(
            "margins".to_string(),
            json!({
                "top": m.top,
                "bottom": m.bottom,
                "inside": m.inside,
                "outside": m.outside,
            }),
        );
    }

    app.run("layout.headerFooter.apply", params)?;
    Ok(format!("Header & footer template '{}' applied successfully", st.hf_style))
}

pub fn execute_remove_header_footer(app: &mut DesignApp) -> Result<String, String> {
    app.run("layout.headerFooter.remove", json!({}))?;
    Ok("Header and footer removed from document".to_string())
}

pub fn execute_save_hf_template(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let name = st.hf_custom_name.trim().to_string();
    if name.is_empty() {
        return Err("Please enter a template name".to_string());
    }
    let template = json!({
        "name": name,
        "description": "User custom template",
        "sided": if st.hf_sided == 1 { "single" } else { "double" },
        "headerOffset": st.hf_header_offset,
        "footerOffset": st.hf_footer_offset,
        "header": {
            "single": st.hf_header_single,
            "left": st.hf_header_left,
            "right": st.hf_header_right,
        },
        "footer": {
            "single": st.hf_footer_single,
            "left": st.hf_footer_left,
            "right": st.hf_footer_right,
        },
        "objects": {
            "headerRule": st.hf_header_rule,
            "headerRuleWeight": st.hf_header_rule_weight,
            "footerRule": st.hf_footer_rule,
            "footerRuleWeight": st.hf_footer_rule_weight,
            "topAccentBar": st.hf_top_accent_bar,
            "topAccentBarHeight": st.hf_top_accent_bar_height,
            "topAccentBarBleed": st.hf_top_accent_bar_bleed,
        },
        "colors": {
            "primary": st.hf_color_primary,
            "accent": st.hf_color_accent,
            "dark": st.hf_color_dark,
            "ruleColor": st.hf_color_rule,
            "textColor": st.hf_color_text,
        }
    });

    app.run("layout.headerFooter.saveTemplate", json!({ "template": template }))?;
    st.hf_style = name.to_string();
    st.hf_custom_name.clear();
    Ok(format!("Template '{name}' saved successfully"))
}

pub fn execute_delete_hf_template(app: &mut DesignApp, st: &mut PubgenState, name: &str) -> Result<String, String> {
    app.run("layout.headerFooter.deleteTemplate", json!({ "name": name }))?;
    st.hf_style = "Modern Minimal".to_string();
    Ok(format!("Template '{name}' deleted"))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

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
    fn test_resolve_brs_header_texts() {
        let mut st = PubgenState::default();
        st.cover_title.clear();
        let (def_title, def_nopub) = resolve_brs_header_texts(&st);
        assert_eq!(def_title, "Perkembangan Nilai Tukar Petani");
        assert_eq!(def_nopub, "BRS No. 61/10/32/Th. XXVIII, 1 Oktober 2026");

        st.excel_cover = Some(super::excel::ExcelCoverData {
            title1: "Perkembangan Nilai Tukar Petani Jawa Barat".into(),
            release_month: "September 2026".into(),
            release_no: "61/10/32/Th. XXVIII".into(),
            release_date: "1 Oktober 2026".into(),
            ..Default::default()
        });

        let (title, nopub) = resolve_brs_header_texts(&st);
        assert_eq!(title, "Perkembangan Nilai Tukar Petani Jawa Barat, September 2026");
        assert_eq!(nopub, "BRS No. 61/10/32/Th. XXVIII, 1 Oktober 2026");
    }

    #[test]
    fn test_execute_apply_hf_brs_facing_pages_and_scope() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        // Initially single pages (facingPages: false) with 4 pages
        app.run("file.new", json!({ "facingPages": false, "pages": 4 })).unwrap();
        assert!(!app.session.active().unwrap().doc.settings.facing_pages);

        let mut st = PubgenState {
            hf_style: "hf-brs".into(),
            hf_sided: 0,
            hf_scope: 0, // All Pages -> should auto-scope to body pages [2, 4]
            ..Default::default()
        };
        st.excel_cover = Some(super::excel::ExcelCoverData {
            title1: "Perkembangan Nilai Tukar Petani Jawa Barat".into(),
            release_month: "September 2026".into(),
            release_no: "61/10/32/Th. XXVIII".into(),
            release_date: "1 Oktober 2026".into(),
            ..Default::default()
        });

        let res = execute_apply_header_footer(&mut app, &st);
        assert!(res.is_ok(), "apply hf-brs should succeed: {res:?}");

        let doc = &app.session.active().unwrap().doc;
        // Verify facing_pages was automatically enabled
        assert!(doc.settings.facing_pages, "facing pages should be enabled for double-sided BRS");

        let hf = doc.header_footer.as_ref().expect("header_footer present");
        assert_eq!(hf.preset, "hf-brs");
        assert_eq!(hf.sided, designcraft_doc::Sided::Double);
        assert_eq!(hf.scope, designcraft_doc::HfScope::Range { first: 2, last: 4 });

        // Header is blank
        assert!(hf.header_items.left.is_none(), "left header is blank");
        assert!(hf.header_items.right.is_none(), "right header is blank");

        // Footer text and folio items are generated for verso and recto
        assert!(hf.footer_items.left.is_some(), "left footer text item exists");
        assert!(hf.footer_items.right.is_some(), "right footer text item exists");
        assert!(hf.folio_items.left.is_some(), "left folio item exists");
        assert!(hf.folio_items.right.is_some(), "right folio item exists");
    }
}
