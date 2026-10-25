//! Page utility commands: renumbering, continue numbering, page from RTF, page from image.

use super::*;

pub fn execute_renumber_page(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let cur_page = target_page_index(app);
    let old_label = app.session.page_label(cur_page);
    let start_num = st.card3_renumber_start.max(1);

    let style_str = match st.card3_renumber_style {
        1 => "lowerRoman",
        2 => "upperRoman",
        _ => "arabic",
    };

    app.run(
        "layout.section",
        json!({
            "page": cur_page + 1,
            "startNumber": start_num,
            "style": style_str
        }),
    )?;

    let new_label = app.session.page_label(cur_page);

    // If TOC exists, automatically rebuild it so the TOC entries reflect the newly numbered page!
    let toc_msg = match rebuild_all_tocs(app, st) {
        Ok(msg) if !msg.is_empty() => format!(" & {msg}"),
        _ => String::new(),
    };

    Ok(format!("Renumbered Page {old_label} → '{new_label}' (start: {start_num}){toc_msg}"))
}

pub fn execute_continue_numbering(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let cur_page = target_page_index(app);
    if cur_page == 0 {
        return Err("Page 1 is always the start of the document".into());
    }
    app.run(
        "layout.section",
        json!({
            "page": cur_page + 1,
            "remove": true
        }),
    )?;
    let new_label = app.session.page_label(cur_page);

    let toc_msg = match rebuild_all_tocs(app, st) {
        Ok(msg) if !msg.is_empty() => format!(" & {msg}"),
        _ => String::new(),
    };

    let page_num = cur_page + 1;
    Ok(format!("Continued numbering on Page {page_num} (now '{new_label}'){toc_msg}"))
}

pub fn execute_page_from_rtf(app: &mut DesignApp, st: &mut PubgenState, path: &str) -> Result<String, String> {
    let target_path = if path.trim().is_empty() { st.rtf_path.trim().to_string() } else { path.trim().to_string() };
    if target_path.is_empty() {
        return Err("No RTF file path specified".into());
    }
    let p = std::path::Path::new(&target_path);
    if !p.exists() {
        return Err(format!("File not found: {target_path}"));
    }

    let (cur_page, target_frame) = {
        let active = app.session.active().ok_or("No active document")?;
        let tf = active.selection.items.first().copied().filter(|&id| {
            active.doc.item(id).is_some_and(|it| matches!(it.content, designcraft_doc::Content::Unassigned | designcraft_doc::Content::Text(_)))
        });
        let fallback_page = app
            .session
            .active()
            .and_then(|s| s.selection.page)
            .or_else(|| crate::canvas::current_page(app))
            .unwrap_or_else(|| target_page_index(app));
        let p = tf.and_then(|id| active.doc.page_of_item(id)).unwrap_or(fallback_page);
        (p, tf)
    };
    let cur_label = app.session.page_label(cur_page);

    let res = if let Some(fid) = target_frame {
        app.run(
            "file.place",
            json!({
                "path": target_path,
                "frame": fid.0,
                "autoflow": true,
            }),
        )?
    } else {
        let _ = app.run("selection.set", json!({"ids": []}));
        app.run(
            "file.place",
            json!({
                "path": target_path,
                "page": cur_page + 1,
                "autoflow": true,
            }),
        )?
    };

    crate::canvas::go_to_page(app, cur_page);
    st.rtf_path = target_path.clone();

    let pages_added = res.get("pagesAdded").and_then(Value::as_u64).unwrap_or(0);
    let overset = res.get("overset").and_then(Value::as_bool).unwrap_or(false);
    let filename = std::path::Path::new(&target_path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| target_path.clone());

    let page_info = if pages_added > 0 { format!(" across {} pages ({} added)", pages_added + 1, pages_added) } else { String::new() };
    let overset_info = if overset { " (text still overset)" } else { "" };

    Ok(format!("Imported '{filename}' to Page {cur_label}{page_info}{overset_info}"))
}

/// Page from Image: place an image on the selected page either as an image object fitted inside
/// the page margins, or as a full-page background (page bounds, sent to the back).
pub fn execute_page_from_image(app: &mut DesignApp, st: &mut PubgenState, path: &str) -> Result<String, String> {
    let target_path = if path.trim().is_empty() {
        st.image_path.trim().trim_matches('"').trim().to_string()
    } else {
        path.trim().trim_matches('"').trim().to_string()
    };
    if target_path.is_empty() {
        return Err("No image file path specified".into());
    }
    if !std::path::Path::new(&target_path).exists() {
        return Err(format!("File not found: {target_path}"));
    }

    let cur_page =
        app.session.active().and_then(|s| s.selection.page).or_else(|| crate::canvas::current_page(app)).unwrap_or_else(|| target_page_index(app));
    let cur_label = app.session.page_label(cur_page);

    // Image object fits inside the page margins; a page background covers the whole page.
    let (spread_idx, target_rect) = {
        let active = app.session.active().ok_or("No active document")?;
        let (si, pi) = active.doc.page_loc(cur_page).ok_or("No such page")?;
        let page = &active.doc.spreads[si].pages[pi];
        let rect = if st.image_as_background { page.bounds() } else { page.margin_rect() };
        (si, rect)
    };

    // 1. Graphic frame on the target spread at the target rect.
    let res = app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [target_rect.x0, target_rect.y0, target_rect.x1, target_rect.y1],
            "content": "graphic",
            "caret": false,
        }),
    )?;
    let fid = res.get("id").and_then(Value::as_u64).ok_or("Failed to create graphic frame")?;

    // 2. Place the image into the frame.
    app.run("file.place", json!({ "path": target_path, "frame": fid }))?;

    // 3. Fit the image proportionally, center it, fit frame to content, and for background send to back.
    let _ = app.run("object.fit", json!({ "mode": "fitProportionally", "ids": [fid] }));
    let _ = app.run("object.fit", json!({ "mode": "centerContent", "ids": [fid] }));
    let _ = app.run("object.fit", json!({ "mode": "fitFrameToContent", "ids": [fid] }));
    if st.image_as_background {
        let _ = app.run("object.arrange", json!({ "to": "back", "ids": [fid] }));
    }

    let _ = app.run("selection.set", json!({ "ids": [fid] }));

    crate::canvas::go_to_page(app, cur_page);
    st.image_path = target_path.clone();

    let filename = std::path::Path::new(&target_path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| target_path.clone());
    let mode_desc =
        if st.image_as_background { "page background (fitted proportionally to page, sent to back)" } else { "image object (fitted inside margins)" };

    Ok(format!("Placed '{filename}' as {mode_desc} on Page {cur_label}"))
}

pub(crate) fn page_utilities_ui(app: &mut DesignApp, ui: &mut Ui, st: &mut PubgenState, t: &Tokens) {
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
