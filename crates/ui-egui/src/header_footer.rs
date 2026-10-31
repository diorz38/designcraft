//! Layout › Header/Footer…, laid out in InDesign style with preset selection, token legend,
//! scope controls, single/double-sided template fields, and geometry controls.
//! Applies `layout.headerFooter.apply` or `layout.headerFooter.remove`.

use egui::{Align, Layout, Ui, vec2};
use serde_json::{Value, json};

use crate::DesignApp;
use crate::dialogs::Dialog;
use crate::theme::Tokens;

/// Preset identifiers and UI labels.
pub const PRESETS: [(&str, &str); 5] =
    [("minimal", "Minimal"), ("book", "Book"), ("editorial", "Editorial"), ("hf-brs", "hf-brs"), ("custom", "Custom")];

/// The options height for the modal window.
#[allow(dead_code)]
pub(crate) const BODY_H: f32 = 540.0;
const LABEL_W: f32 = 110.0;
const FIELD_W: f32 = 70.0;

/// Opens the Header/Footer dialog seeded with live document state.
pub fn open(app: &mut DesignApp) -> Result<Value, String> {
    let got = app.session.execute("layout.headerFooter.get", &json!({})).map_err(|e| e.to_string())?;
    let facing = got.get("facingPages").and_then(Value::as_bool).unwrap_or(false);
    let page_count = got.get("pageCount").and_then(Value::as_u64).unwrap_or(1);
    let active_page = got.get("activePage").and_then(Value::as_u64).unwrap_or(1);

    let hf_opt = got.get("headerFooter").filter(|v| !v.is_null());

    let (
        preset,
        sided,
        scope_type,
        scope_sec,
        scope_start,
        scope_end,
        h_offset,
        f_offset,
        h_single,
        h_left,
        h_right,
        f_single,
        f_left,
        f_right,
        style,
        rule,
    ) = if let Some(hf) = hf_opt {
        let p = hf.get("preset").and_then(Value::as_str).unwrap_or("custom");
        let s = hf.get("sided").and_then(Value::as_str).unwrap_or(if facing { "double" } else { "single" });
        let (sc_type, sc_sec, sc_start, sc_end) = match hf.get("scope") {
            Some(v) if v.get("section").is_some() => ("section", v["section"].as_u64().unwrap_or(0), 1, page_count),
            Some(v) if v.get("range").is_some() => {
                let r = v["range"].as_array();
                let st = r.and_then(|a| a.first()).and_then(Value::as_u64).unwrap_or(1);
                let en = r.and_then(|a| a.get(1)).and_then(Value::as_u64).unwrap_or(page_count);
                ("range", 0, st, en)
            }
            _ => ("all", 0, 1, page_count),
        };
        let ho = hf.get("headerOffset").or_else(|| hf.get("header_offset")).and_then(Value::as_f64).unwrap_or(24.0);
        let fo = hf.get("footerOffset").or_else(|| hf.get("footer_offset")).and_then(Value::as_f64).unwrap_or(24.0);
        let h = hf.get("header");
        let f = hf.get("footer");
        let hs = h.and_then(|x| x.get("single")).and_then(Value::as_str).unwrap_or("");
        let hl = h.and_then(|x| x.get("left")).and_then(Value::as_str).unwrap_or("");
        let hr = h.and_then(|x| x.get("right")).and_then(Value::as_str).unwrap_or("");
        let fs = f.and_then(|x| x.get("single")).and_then(Value::as_str).unwrap_or("");
        let fl = f.and_then(|x| x.get("left")).and_then(Value::as_str).unwrap_or("");
        let fr = f.and_then(|x| x.get("right")).and_then(Value::as_str).unwrap_or("");
        let st = hf.get("style").and_then(Value::as_str).unwrap_or("[Basic Paragraph]");
        let r = hf.get("rule").and_then(Value::as_bool).unwrap_or(false);
        (p, s, sc_type, sc_sec, sc_start, sc_end, ho, fo, hs, hl, hr, fs, fl, fr, st, r)
    } else {
        let p = "minimal";
        let s = if facing { "double" } else { "single" };
        let sc_type = "all";
        let ho = 24.0;
        let fo = 24.0;
        let hs = "";
        let hl = "";
        let hr = "";
        let fs = "{filename}{tab}{page}";
        let fl = "";
        let fr = "";
        let st = "[Basic Paragraph]";
        let r = false;
        (p, s, sc_type, 0, 1, page_count, ho, fo, hs, hl, hr, fs, fl, fr, st, r)
    };

    let pstyles = got.get("paragraphStyles").cloned().unwrap_or(json!([]));
    let sections = got.get("sections").cloned().unwrap_or(json!([]));
    let templates = got.get("templates").cloned().unwrap_or(json!([]));

    let fields = json!({
        "preset": preset,
        "sided": sided,
        "scopeType": scope_type,
        "scopeSection": scope_sec,
        "scopeStart": scope_start.to_string(),
        "scopeEnd": scope_end.to_string(),
        "headerOffset": format!("{h_offset:.1}"),
        "footerOffset": format!("{f_offset:.1}"),
        "headerSingle": h_single,
        "headerLeft": h_left,
        "headerRight": h_right,
        "footerSingle": f_single,
        "footerLeft": f_left,
        "footerRight": f_right,
        "style": style,
        "rule": rule,
        "facingPages": facing,
        "pageCount": page_count,
        "activePage": active_page,
        "paragraphStyles": pstyles,
        "sections": sections,
        "templates": templates,
        "status": "",
    });

    app.ui.dialog = Some(Dialog::new("headerFooter", fields));
    Ok(Value::Null)
}

/// Applies preset templates and values to dialog fields.
fn apply_preset(d: &mut Dialog, preset: &str, facing: bool) {
    let tpl_opt = d
        .fields
        .get("templates")
        .and_then(Value::as_array)
        .and_then(|arr| arr.iter().find(|t| t.get("name").and_then(Value::as_str) == Some(preset)).cloned());

    d.fields.insert("preset".into(), json!(preset));

    if let Some(tpl) = tpl_opt {
        let sided = tpl.get("sided").and_then(Value::as_str).unwrap_or(if facing { "double" } else { "single" });
        d.fields.insert("sided".into(), json!(sided));
        if let Some(ho) = tpl.get("headerOffset").and_then(Value::as_f64) {
            d.fields.insert("headerOffset".into(), json!(format!("{ho:.1}")));
        }
        if let Some(fo) = tpl.get("footerOffset").and_then(Value::as_f64) {
            d.fields.insert("footerOffset".into(), json!(format!("{fo:.1}")));
        }
        if let Some(h) = tpl.get("header") {
            d.fields.insert("headerSingle".into(), json!(h.get("single").and_then(Value::as_str).unwrap_or("")));
            d.fields.insert("headerLeft".into(), json!(h.get("left").and_then(Value::as_str).unwrap_or("")));
            d.fields.insert("headerRight".into(), json!(h.get("right").and_then(Value::as_str).unwrap_or("")));
        }
        if let Some(f) = tpl.get("footer") {
            d.fields.insert("footerSingle".into(), json!(f.get("single").and_then(Value::as_str).unwrap_or("")));
            d.fields.insert("footerLeft".into(), json!(f.get("left").and_then(Value::as_str).unwrap_or("")));
            d.fields.insert("footerRight".into(), json!(f.get("right").and_then(Value::as_str).unwrap_or("")));
        }
        if let Some(objs) = tpl.get("objects") {
            let hr = objs.get("headerRule").and_then(Value::as_bool).unwrap_or(false);
            d.fields.insert("rule".into(), json!(hr));
            d.fields.insert("objects".into(), objs.clone());
        }
        if let Some(cols) = tpl.get("colors") {
            d.fields.insert("colors".into(), cols.clone());
        }
        return;
    }

    match preset {
        "minimal" => {
            d.fields.insert("sided".into(), json!("single"));
            d.fields.insert("headerOffset".into(), json!("24.0"));
            d.fields.insert("footerOffset".into(), json!("24.0"));
            d.fields.insert("headerSingle".into(), json!(""));
            d.fields.insert("headerLeft".into(), json!(""));
            d.fields.insert("headerRight".into(), json!(""));
            d.fields.insert("footerSingle".into(), json!("{filename}{tab}{page}"));
            d.fields.insert("footerLeft".into(), json!(""));
            d.fields.insert("footerRight".into(), json!(""));
            d.fields.insert("rule".into(), json!(false));
        }
        "book" => {
            d.fields.insert("sided".into(), json!(if facing { "double" } else { "single" }));
            d.fields.insert("headerOffset".into(), json!("36.0"));
            d.fields.insert("footerOffset".into(), json!("36.0"));
            d.fields.insert("headerSingle".into(), json!("{page}{tab}{runningHeader}"));
            d.fields.insert("headerLeft".into(), json!("{page}{tab}{runningHeader}"));
            d.fields.insert("headerRight".into(), json!("{runningHeader}{tab}{page}"));
            d.fields.insert("footerSingle".into(), json!(""));
            d.fields.insert("footerLeft".into(), json!(""));
            d.fields.insert("footerRight".into(), json!(""));
            d.fields.insert("rule".into(), json!(false));
        }
        "editorial" => {
            d.fields.insert("sided".into(), json!(if facing { "double" } else { "single" }));
            d.fields.insert("headerOffset".into(), json!("36.0"));
            d.fields.insert("footerOffset".into(), json!("36.0"));
            d.fields.insert("headerSingle".into(), json!("{page}  {section}{tab}{runningHeader}"));
            d.fields.insert("headerLeft".into(), json!("{page}  {section}{tab}{runningHeader}"));
            d.fields.insert("headerRight".into(), json!("{runningHeader}{tab}{section}  {page}"));
            d.fields.insert("footerSingle".into(), json!("{filename}"));
            d.fields.insert("footerLeft".into(), json!("{filename}"));
            d.fields.insert("footerRight".into(), json!("{filename}"));
            d.fields.insert("rule".into(), json!(true));
        }
        "hf-brs" | "brs" => {
            d.fields.insert("sided".into(), json!("double"));
            d.fields.insert("headerOffset".into(), json!("24.0"));
            d.fields.insert("footerOffset".into(), json!("24.0"));
            d.fields.insert("headerSingle".into(), json!(""));
            d.fields.insert("headerLeft".into(), json!(""));
            d.fields.insert("headerRight".into(), json!(""));
            d.fields.insert("footerSingle".into(), json!("{title}\n{nopub}"));
            d.fields.insert("footerLeft".into(), json!("{title}\n{nopub}"));
            d.fields.insert("footerRight".into(), json!("{title}\n{nopub}"));
            d.fields.insert("rule".into(), json!(false));
        }
        _ => {}
    }
}

/// Validates fields and dispatches `layout.headerFooter.apply`.
pub fn confirm(app: &mut DesignApp, d: &Dialog) -> Result<Value, String> {
    let preset = d.s("preset");
    let sided = d.s("sided");
    let style = d.s("style");
    let rule = d.b("rule");

    // If double-sided is requested and facingPages is currently false, ensure facingPages is true
    if sided == "double" && app.session.active().is_some_and(|s| !s.doc.settings.facing_pages) {
        let _ = app.run("layout.documentSetup", json!({ "facingPages": true }));
    }

    // Geometry validation
    let header_offset = d.pt("headerOffset").unwrap_or(0.0);
    let footer_offset = d.pt("footerOffset").unwrap_or(0.0);

    if header_offset < 0.0 {
        return Err("Header offset must be non-negative.".into());
    }
    if footer_offset < 0.0 {
        return Err("Footer offset must be non-negative.".into());
    }

    if let Some(st) = app.session.active() {
        let page_h = st.doc.settings.page_height;
        if header_offset + footer_offset >= page_h {
            return Err(format!(
                "Header and footer offsets ({:.1} pt + {:.1} pt) exceed page height ({:.1} pt).",
                header_offset, footer_offset, page_h
            ));
        }
    }

    // Scope
    let scope_val = match d.s("scopeType").as_str() {
        "section" => {
            let sec_idx = d.fields.get("scopeSection").and_then(Value::as_u64).unwrap_or(0);
            json!({"section": sec_idx})
        }
        "range" => {
            let start = d.n("scopeStart").unwrap_or(1.0).max(1.0) as u64;
            let end = d.n("scopeEnd").unwrap_or(1.0).max(start as f64) as u64;
            json!({"range": [start, end]})
        }
        _ => json!("all"),
    };

    let mut params = json!({
        "preset": preset,
        "sided": sided,
        "scope": scope_val,
        "headerOffset": header_offset,
        "footerOffset": footer_offset,
        "style": style,
        "rule": rule,
        "header": {
            "single": d.s("headerSingle"),
            "left": d.s("headerLeft"),
            "right": d.s("headerRight"),
        },
        "footer": {
            "single": d.s("footerSingle"),
            "left": d.s("footerLeft"),
            "right": d.s("footerRight"),
        },
    });
    if let Some(objs) = d.fields.get("objects") {
        params["objects"] = objs.clone();
    }
    if let Some(cols) = d.fields.get("colors") {
        params["colors"] = cols.clone();
    }

    app.run("layout.headerFooter.apply", params)
}

/// A row layout helper respecting UI direction.
fn row<R>(ui: &mut Ui, rtl: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    let layout = if rtl { Layout::right_to_left(Align::Center) } else { Layout::left_to_right(Align::Center) };
    ui.allocate_ui_with_layout(vec2(ui.available_width(), 24.0), layout, add).inner
}

/// A caption ending at the field it names.
fn caption(ui: &mut Ui, rtl: bool, text: &str) {
    let layout = if rtl { Layout::left_to_right(Align::Center) } else { Layout::right_to_left(Align::Center) };
    ui.allocate_ui_with_layout(vec2(LABEL_W, 22.0), layout, |ui| {
        ui.set_width(LABEL_W);
        crate::rtl::label(ui, text);
    });
}

fn text(ui: &mut Ui, d: &mut Dialog, key: &str, width: f32) -> bool {
    let mut s = d.s(key);
    let changed = ui.add(egui::TextEdit::singleline(&mut s).desired_width(width)).changed();
    if changed {
        d.fields.insert(key.into(), json!(s));
    }
    changed
}

/// A titled, outlined group of options.
fn group(ui: &mut Ui, rtl: bool, title: &str, add: impl FnOnce(&mut Ui)) {
    let t = Tokens::get(ui.ctx());
    row(ui, rtl, |ui| {
        ui.add_space(4.0);
        crate::rtl::label(ui, egui::RichText::new(title).color(t.text_strong));
    });
    egui::Frame::NONE.stroke(egui::Stroke::new(1.0, t.divider)).corner_radius(3.0).inner_margin(egui::Margin::symmetric(10, 8)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 6.0;
        add(ui);
    });
}

/// Renders the Header/Footer dialog body.
pub fn body(app: &mut DesignApp, ui: &mut Ui, d: &mut Dialog) {
    let lang = app.ui.language.clone();
    let tr = |s: &'static str| crate::i18n::tr(&lang, s);
    let rtl = crate::i18n::is_rtl(&lang);
    let t = Tokens::get(ui.ctx());

    ui.spacing_mut().item_spacing.y = 8.0;

    // Error status banner
    let status = d.s("status");
    if !status.is_empty() {
        ui.add_space(2.0);
        crate::rtl::label(ui, egui::RichText::new(&status).color(t.text_strong));
        ui.add_space(4.0);
    }

    let facing = d.b("facingPages");
    let current_preset = d.s("preset");

    // 1. Preset Selection
    row(ui, rtl, |ui| {
        caption(ui, rtl, tr("Preset:"));
        let mut picked_preset: Option<String> = None;
        let templates = d.fields.get("templates").and_then(Value::as_array);
        if let Some(tpls) = templates {
            let cur_label = if current_preset.is_empty() { "Custom" } else { &current_preset };
            craft_ui::ComboBox::from_id_salt("hf_dialog_preset").selected_text(cur_label).width(180.0).show_ui(ui, |ui| {
                for t in tpls {
                    if let Some(name) = t.get("name").and_then(Value::as_str)
                        && ui.selectable_label(current_preset == name, name).clicked()
                    {
                        picked_preset = Some(name.to_string());
                    }
                }
                if ui.selectable_label(current_preset == "custom", tr("Custom")).clicked() {
                    picked_preset = Some("custom".to_string());
                }
            });
        } else {
            for (id, label) in PRESETS {
                let is_sel = current_preset == id;
                if ui.radio(is_sel, crate::rtl::widget(ui, tr(label))).clicked() && !is_sel {
                    picked_preset = Some(id.to_string());
                }
            }
        }
        if let Some(p) = picked_preset {
            apply_preset(d, &p, facing);
        }
    });

    // 2. Scope & Sidedness
    group(ui, rtl, tr("Scope & Spreads"), |ui| {
        let sided = d.s("sided");
        row(ui, rtl, |ui| {
            caption(ui, rtl, tr("Page Sidedness:"));
            if ui.radio(sided == "single", crate::rtl::widget(ui, tr("Single-sided"))).clicked() {
                d.fields.insert("sided".into(), json!("single"));
                d.fields.insert("preset".into(), json!("custom"));
            }
            ui.add_enabled_ui(facing, |ui| {
                if ui.radio(sided == "double", crate::rtl::widget(ui, tr("Double-sided"))).clicked() {
                    d.fields.insert("sided".into(), json!("double"));
                    d.fields.insert("preset".into(), json!("custom"));
                }
            });
        });

        let scope_type = d.s("scopeType");
        row(ui, rtl, |ui| {
            caption(ui, rtl, tr("Page Scope:"));
            if ui.radio(scope_type == "all", crate::rtl::widget(ui, tr("All Pages"))).clicked() {
                d.fields.insert("scopeType".into(), json!("all"));
            }
            if ui.radio(scope_type == "section", crate::rtl::widget(ui, tr("Section"))).clicked() {
                d.fields.insert("scopeType".into(), json!("section"));
            }
            if ui.radio(scope_type == "range", crate::rtl::widget(ui, tr("Range:"))).clicked() {
                d.fields.insert("scopeType".into(), json!("range"));
            }
        });

        if scope_type == "section" {
            let sections = d.fields.get("sections").and_then(Value::as_array).cloned().unwrap_or_default();
            let cur_sec = d.fields.get("scopeSection").and_then(Value::as_u64).unwrap_or(0);
            row(ui, rtl, |ui| {
                caption(ui, rtl, tr("Target Section:"));
                let label = if let Some(s) = sections.get(cur_sec as usize) {
                    let p = s.get("prefix").and_then(Value::as_str).unwrap_or("");
                    let st = s.get("start").and_then(Value::as_u64).unwrap_or(0);
                    format!("Section {} (page {})", if p.is_empty() { format!("{}", cur_sec + 1) } else { p.to_string() }, st + 1)
                } else {
                    format!("Section {}", cur_sec + 1)
                };
                craft_ui::ComboBox::from_id_salt("hf_section").selected_text(label).width(180.0).show_ui(ui, |ui| {
                    for (i, s) in sections.iter().enumerate() {
                        let p = s.get("prefix").and_then(Value::as_str).unwrap_or("");
                        let st = s.get("start").and_then(Value::as_u64).unwrap_or(0);
                        let text = format!("Section {} (page {})", if p.is_empty() { format!("{}", i + 1) } else { p.to_string() }, st + 1);
                        if ui.selectable_label(cur_sec == i as u64, text).clicked() {
                            d.fields.insert("scopeSection".into(), json!(i as u64));
                        }
                    }
                });
            });
        } else if scope_type == "range" {
            row(ui, rtl, |ui| {
                caption(ui, rtl, tr("Page Range:"));
                text(ui, d, "scopeStart", FIELD_W);
                crate::rtl::label(ui, tr("to"));
                text(ui, d, "scopeEnd", FIELD_W);
            });
        }
    });

    // 3. Geometry & Styling
    group(ui, rtl, tr("Geometry & Formatting"), |ui| {
        row(ui, rtl, |ui| {
            caption(ui, rtl, tr("Header Offset:"));
            if text(ui, d, "headerOffset", FIELD_W) {
                d.fields.insert("preset".into(), json!("custom"));
            }
            crate::rtl::label(ui, "pt");

            ui.add_space(16.0);
            caption(ui, rtl, tr("Footer Offset:"));
            if text(ui, d, "footerOffset", FIELD_W) {
                d.fields.insert("preset".into(), json!("custom"));
            }
            crate::rtl::label(ui, "pt");
        });

        row(ui, rtl, |ui| {
            caption(ui, rtl, tr("Paragraph Style:"));
            let cur_style = d.s("style");
            let pstyles = d.fields.get("paragraphStyles").and_then(Value::as_array).cloned().unwrap_or_default();
            craft_ui::ComboBox::from_id_salt("hf_style").selected_text(&cur_style).width(160.0).show_ui(ui, |ui| {
                if ui.selectable_label(cur_style == "[Basic Paragraph]", "[Basic Paragraph]").clicked() {
                    d.fields.insert("style".into(), json!("[Basic Paragraph]"));
                    d.fields.insert("preset".into(), json!("custom"));
                }
                for sty in &pstyles {
                    if let Some(name) = sty.as_str()
                        && name != "[Basic Paragraph]"
                        && ui.selectable_label(cur_style == name, name).clicked()
                    {
                        d.fields.insert("style".into(), json!(name));
                        d.fields.insert("preset".into(), json!("custom"));
                    }
                }
            });

            ui.add_space(16.0);
            let mut rule = d.b("rule");
            if ui.checkbox(&mut rule, crate::rtl::widget(ui, tr("Hairline Rule"))).changed() {
                d.fields.insert("rule".into(), json!(rule));
                d.fields.insert("preset".into(), json!("custom"));
            }
        });
    });

    // 4. Template Strings
    let is_double = d.s("sided") == "double";
    group(ui, rtl, tr("Header & Footer Templates"), |ui| {
        if !is_double {
            row(ui, rtl, |ui| {
                caption(ui, rtl, tr("Header:"));
                if text(ui, d, "headerSingle", 280.0) {
                    d.fields.insert("preset".into(), json!("custom"));
                }
            });
            row(ui, rtl, |ui| {
                caption(ui, rtl, tr("Footer:"));
                if text(ui, d, "footerSingle", 280.0) {
                    d.fields.insert("preset".into(), json!("custom"));
                }
            });
        } else {
            row(ui, rtl, |ui| {
                caption(ui, rtl, tr("Left Header:"));
                if text(ui, d, "headerLeft", 280.0) {
                    d.fields.insert("preset".into(), json!("custom"));
                }
            });
            row(ui, rtl, |ui| {
                caption(ui, rtl, tr("Right Header:"));
                if text(ui, d, "headerRight", 280.0) {
                    d.fields.insert("preset".into(), json!("custom"));
                }
            });
            row(ui, rtl, |ui| {
                caption(ui, rtl, tr("Left Footer:"));
                if text(ui, d, "footerLeft", 280.0) {
                    d.fields.insert("preset".into(), json!("custom"));
                }
            });
            row(ui, rtl, |ui| {
                caption(ui, rtl, tr("Right Footer:"));
                if text(ui, d, "footerRight", 280.0) {
                    d.fields.insert("preset".into(), json!("custom"));
                }
            });
        }
    });

    // 5. Token Legend Banner
    egui::Frame::NONE.fill(t.well).corner_radius(4.0).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        row(ui, rtl, |ui| {
            crate::rtl::label(
                ui,
                egui::RichText::new(tr("Available Tokens:  {page}  {section}  {chapter}  {filename}  {title}  {nopub}  {runningHeader}  {tab}"))
                    .color(t.text_dim)
                    .size(11.0),
            );
        });
    });

    // 6. Direct Removal Action
    ui.add_space(4.0);
    row(ui, rtl, |ui| {
        if ui.button(crate::rtl::widget(ui, tr("Remove Header/Footer"))).clicked() {
            let _ = app.run("layout.headerFooter.remove", json!({}));
            app.ui.dialog = None;
        }
    });
}
