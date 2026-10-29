//! Page utility commands: renumbering, continue numbering, page from RTF, page from image.

use super::*;

/// Returns the custom document margins:
/// 1. From existing document pages with local or custom (non-36pt, non-0) margins
/// 2. From parent pages with custom (non-36pt, non-0) margins
/// 3. From any spread page with non-zero margins
/// 4. From any parent page with non-zero margins
/// 5. Fallback: Standard BPS custom margins (Top 20.20 mm, Bottom 20 mm, Inside 19 mm, Outside 15 mm)
pub fn detect_document_margins(doc: &designcraft_doc::Document) -> designcraft_doc::Margins {
    const PT_PER_MM: f64 = 72.0 / 25.4;
    let bps_margins =
        designcraft_doc::Margins { top: 20.20 * PT_PER_MM, bottom: 20.0 * PT_PER_MM, inside: 19.0 * PT_PER_MM, outside: 15.0 * PT_PER_MM };

    // 1. Look for existing document pages with local margins or non-default margins (excluding COVER/DIV with 0 margins)
    if let Some(m) = doc
        .spreads
        .iter()
        .flat_map(|sp| &sp.pages)
        .find(|pg| pg.margins.top > 0.0 && pg.margins.bottom > 0.0 && (pg.local_margins || pg.margins != designcraft_doc::Margins::uniform(36.0)))
        .map(|pg| pg.margins)
    {
        return m;
    }

    // 2. Look for parents with non-default margins
    if let Some(m) = doc
        .parents
        .iter()
        .flat_map(|sp| &sp.pages)
        .find(|pg| pg.margins.top > 0.0 && pg.margins.bottom > 0.0 && pg.margins != designcraft_doc::Margins::uniform(36.0))
        .map(|pg| pg.margins)
    {
        return m;
    }

    // 3. Fallback: Standard BPS custom margins (Top 20.20 mm, Bottom 20 mm, Inside 19 mm, Outside 15 mm)
    bps_margins
}

/// Sets document margins across all spreads and parents:
/// - Top: 20.20 mm
/// - Bottom: 20.00 mm
/// - Inside: 19.00 mm
/// - Outside: 15.00 mm
pub fn execute_set_page_margins(app: &mut DesignApp) -> Result<String, String> {
    const PT_PER_MM: f64 = 72.0 / 25.4;
    let (top_mm, bottom_mm, inside_mm, outside_mm) = if let Some(ref ctx) = app.egui_ctx {
        super::with_state(ctx, |st| {
            let m = &st.active_profile().geometry.margins_mm;
            (m.top, m.bottom, m.inside, m.outside)
        })
    } else {
        (20.20, 20.0, 19.0, 15.0)
    };
    let top_pt = top_mm * PT_PER_MM;
    let bottom_pt = bottom_mm * PT_PER_MM;
    let inside_pt = inside_mm * PT_PER_MM;
    let outside_pt = outside_mm * PT_PER_MM;

    // Record which pages are COVER or DIV (already 0 margins) before layout.documentSetup
    let zero_margin_pages: Vec<usize> = app
        .session
        .active()
        .map(|s| {
            (0..s.doc.page_count())
                .filter(|&p| {
                    if let Some((si, pi)) = s.doc.page_loc(p)
                        && let Some(pg) = s.doc.spreads.get(si).and_then(|sp| sp.pages.get(pi))
                    {
                        return pg.margins.top == 0.0 && pg.margins.bottom == 0.0;
                    }
                    false
                })
                .collect()
        })
        .unwrap_or_default();

    app.run(
        "layout.documentSetup",
        json!({
            "margins": {
                "top": top_pt,
                "bottom": bottom_pt,
                "inside": inside_pt,
                "outside": outside_pt
            },
            "adjustLayout": true
        }),
    )?;

    // Keep COVER and DIV pages adjusted to Page Size (margins = 0)
    let _ = app.session.edit(|doc, _| {
        let mut abs_p = 0usize;
        for sp in &mut doc.spreads {
            let sp = std::sync::Arc::make_mut(sp);
            for pg in &mut sp.pages {
                let pb = pg.bounds();
                let is_cover_or_div = zero_margin_pages.contains(&abs_p)
                    || sp.items.iter().any(|it| {
                        let ib = it.bounds();
                        ib.x0 < pb.x1
                            && ib.x1 > pb.x0
                            && (it.label.contains("cover")
                                || it.label.contains("divider")
                                || it.label.starts_with("#slot:text:level")
                                || it.label.starts_with("#slot:text:chapter_prefix")
                                || it.label.starts_with("#slot:chapter_prefix")
                                || it.label.starts_with("#slot:text:first_word"))
                    });
                if is_cover_or_div {
                    pg.margins = designcraft_doc::Margins::uniform(0.0);
                    pg.local_margins = true;
                }
                abs_p += 1;
            }
            sp.relayout();
        }
        Ok(serde_json::Value::Null)
    });

    // If a header/footer is already applied to the document, re-apply it so its geometry,
    // frames, and hairline rules immediately follow the new page margins.
    if app.session.active().and_then(|s| s.doc.header_footer.as_ref()).is_some() {
        let _ = app.run(
            "layout.headerFooter.apply",
            json!({
                "margins": {
                    "top": top_pt,
                    "bottom": bottom_pt,
                    "inside": inside_pt,
                    "outside": outside_pt
                }
            }),
        );
    }

    Ok("Page margins set: Top 20.20 mm, Bottom 20 mm, Inside 19 mm, Outside 15 mm".to_string())
}

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
/// ponytail: unwired engine utility kept for the page-from-image tests; wire into page_utilities_ui when the panel gains the button.
#[cfg(test)]
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

/// Adds a blank page after `cur_page` (0-based) while preserving document facing-page rules.
/// In Double-sided mode, ensures spreads strictly form facing pairs [Left, Right] rather than
/// corrupted multi-page island spreads.
pub fn execute_add_blank_page(app: &mut DesignApp, cur_page: usize) -> Result<String, String> {
    let cur_label = app.session.page_label(cur_page);

    // 1. Ensure all spreads allow shuffle so repaginate() can maintain proper facing-pages [Left, Right] spreads.
    let _ = app.session.edit(|doc, _| {
        for sp in &mut doc.spreads {
            let sp = std::sync::Arc::make_mut(sp);
            sp.allow_shuffle = true;
        }
        Ok(serde_json::Value::Null)
    });

    // 2. Insert 1 page after cur_page
    app.run("layout.pages.insert", json!({"after": cur_page, "count": 1})).map_err(|e| format!("Failed to insert page: {e}"))?;

    let next_page = cur_page + 1;

    // 3. Apply standard document margins to the new page and ensure repaginate() cleans up spreads
    let _ = app.session.edit(|doc, _| {
        let margins = detect_document_margins(doc);
        if let Some((si, pi)) = doc.page_loc(next_page) {
            if let Some(sp) = doc.spreads.get_mut(si) {
                let sp = std::sync::Arc::make_mut(sp);
                if let Some(pg) = sp.pages.get_mut(pi) {
                    pg.margins = margins;
                    pg.local_margins = true;
                }
                sp.relayout();
            }
        }
        doc.repaginate();
        Ok(serde_json::Value::Null)
    });

    crate::canvas::go_to_page(app, next_page);
    let next_label = app.session.page_label(next_page);
    Ok(format!("Added blank page after Page {cur_label} (new Page {next_label})"))
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
            let res = execute_add_blank_page(app, cur_page);
            let (msg, ok) = match res {
                Ok(m) => (m, true),
                Err(e) => (e, false),
            };
            st.utilities_status = Some((msg.clone(), ok));
            st.card3_status = Some((msg, ok));
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

#[cfg(test)]
pub(crate) mod tests {
    use super::testutil::*;
    use super::*;

    #[test]
    fn test_quick_actions_switch_facing_pages() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // New document defaults to facing pages (Double-sided)
        let initial_facing = app.session.active().unwrap().doc.settings.facing_pages;
        assert!(initial_facing, "New document should default to facing pages (Double-sided)");

        // Switch to Single-sided
        app.run("layout.documentSetup", json!({"facingPages": false})).unwrap();
        let single_facing = app.session.active().unwrap().doc.settings.facing_pages;
        assert!(!single_facing, "Document should now be Single-sided");

        // Switch back to Double-sided
        app.run("layout.documentSetup", json!({"facingPages": true})).unwrap();
        let double_facing = app.session.active().unwrap().doc.settings.facing_pages;
        assert!(double_facing, "Document should be back to Double-sided");
    }

    #[test]
    fn test_quick_actions_insert_blank_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let initial_count = app.session.active().unwrap().doc.page_count();
        assert_eq!(initial_count, 1);

        // Insert blank page after page 0 (Page 1)
        app.run("layout.pages.insert", json!({"after": 0, "count": 1})).unwrap();

        let updated_count = app.session.active().unwrap().doc.page_count();
        assert_eq!(updated_count, 2);

        // Insert another page after page 0
        app.run("layout.pages.insert", json!({"after": 0, "count": 1})).unwrap();
        assert_eq!(app.session.active().unwrap().doc.page_count(), 3);
    }

    #[test]
    fn test_quick_actions_header_footer_and_table_options() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Header/Footer quick action opens headerFooter dialog
        crate::header_footer::open(&mut app).unwrap();
        assert_eq!(app.ui.dialog.as_ref().unwrap().id, "headerFooter");
        app.ui.dialog = None;

        // 2. Table Options quick action opens tableOptions dialog
        crate::dialogs::open_table_options(&mut app).unwrap();
        assert_eq!(app.ui.dialog.as_ref().unwrap().id, "tableOptions");
        assert!(!app.ui.dialog.as_ref().unwrap().b("inTable"));
    }

    #[test]
    fn test_execute_renumber_page_restarts_numbering_and_rebuilds_toc() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "01.01.".into(), "Data Demografi".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        // Generates Page 2 for Data Demografi
        execute_generate_pages(&mut app, &st).unwrap();

        // Add TOC on Page 1
        crate::canvas::go_to_page(&mut app, 0);
        execute_add_toc(&mut app, &mut st).unwrap();

        // Now select Page 2 (Chapter 1) and restart page numbering at 1 with Arabic numerals
        crate::canvas::go_to_page(&mut app, 1);
        st.card3_renumber_start = 1;
        st.card3_renumber_style = 0; // "1, 2, 3…"
        let res_renum = execute_renumber_page(&mut app, &mut st);
        assert!(res_renum.is_ok(), "execute_renumber_page should succeed: {res_renum:?}");

        // Verify page label is now '1'
        assert_eq!(app.session.page_label(1), "1");

        // Verify TOC was automatically rebuilt with page number 1!
        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = active.doc.story(toc.story).unwrap().text.clone();
        assert!(text.contains("Data Demografi\t1"), "TOC entry should reflect renumbered page 1: {text}");
    }

    #[test]
    fn test_execute_page_from_rtf_sample_file() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let initial_pages = app.session.active().unwrap().doc.page_count();
        assert_eq!(initial_pages, 1);

        let mut st = PubgenState::default();
        let rtf_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/SINGLE_LANG/ulasan_bab_1.rtf");

        let res = execute_page_from_rtf(&mut app, &mut st, rtf_path);
        assert!(res.is_ok(), "execute_page_from_rtf should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("ulasan_bab_1.rtf"), "Success message should contain filename: {msg}");

        let active = app.session.active().unwrap();
        let story = active.doc.stories.values().find(|s| s.text.contains("Dalam bab ini membahas")).expect("Imported story must be present");

        // Verify numbered list item (Pusat Pemukiman Padat)
        let num_idx = story.text.find("Pusat Pemukiman Padat").expect("must contain Pusat Pemukiman Padat");
        let num_pi = story.para_at(num_idx);
        let num_p = &story.paras[num_pi].para;
        assert_eq!(num_p.left_indent, Some(36.0), "Numbered list left_indent must be 36pt");
        assert_eq!(num_p.first_line_indent, Some(-18.0), "Numbered list first_line_indent must be -18pt");
        assert_eq!(num_p.space_after, Some(8.0), "Numbered list space_after must be 8pt");
        assert!(num_p.tabs.as_ref().is_some_and(|t| t.iter().any(|s| (s.position - 36.0).abs() < 0.1)));

        // Verify nested sub-bullet list item (S1/Sarjana)
        let bullet_idx = story.text.find("S1/Sarjana").expect("must contain S1/Sarjana");
        let bullet_pi = story.para_at(bullet_idx);
        let bullet_p = &story.paras[bullet_pi].para;
        assert_eq!(bullet_p.left_indent, Some(72.0), "Nested bullet left_indent must be 72pt");
        assert_eq!(bullet_p.first_line_indent, Some(-18.0), "Nested bullet first_line_indent must be -18pt");
        assert_eq!(bullet_p.space_after, Some(8.0), "Nested bullet space_after must be 8pt");
        assert!(bullet_p.tabs.as_ref().is_some_and(|t| t.iter().any(|s| (s.position - 72.0).abs() < 0.1)));

        // Verify level 1 bullet (Komposisi Gender) has unicode bullet •
        let gender_idx = story.text.find("Komposisi Gender").expect("must contain Komposisi Gender");
        let gender_pi = story.para_at(gender_idx);
        let gender_ranges = story.para_ranges();
        let gender_para_text = &story.text[gender_ranges[gender_pi].clone()];
        assert!(gender_para_text.starts_with("\u{2022}\t"), "Level 1 bullet must start with •\\t: {gender_para_text:?}");
    }

    #[test]
    fn test_execute_page_from_rtf_multi_page_autoflow() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let initial_pages = app.session.active().unwrap().doc.page_count();
        assert_eq!(initial_pages, 1);

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("long_multi_page.rtf");
        let para = "The quick brown fox jumps over the lazy dog to fill the entire page with lines of text and paragraphs. \\par\n".repeat(60);
        let rtf_content = format!(r#"{{\rtf1\ansi\deff0 {{\fonttbl {{\f0 Times;}}}}\f0\fs24 {para}}}"#);
        std::fs::write(&file_path, rtf_content.as_bytes()).unwrap();

        let mut st = PubgenState::default();
        let res = execute_page_from_rtf(&mut app, &mut st, &file_path.to_string_lossy());
        assert!(res.is_ok(), "execute_page_from_rtf should succeed: {res:?}");

        let final_pages = app.session.active().unwrap().doc.page_count();
        assert!(final_pages > initial_pages, "Multi-page RTF should autoflow to add extra pages (initial: {initial_pages}, final: {final_pages})");

        // Verify threaded text frames exist across the new pages
        let active = app.session.active().unwrap();
        let text_frames_count =
            active.doc.spreads.iter().flat_map(|sp| &sp.items).filter(|it| matches!(it.content, designcraft_doc::Content::Text(_))).count();
        assert!(text_frames_count >= final_pages, "Each page should have a threaded text frame");
    }

    #[test]
    fn test_execute_page_from_rtf_into_selected_frame() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Create an empty text frame on page 0
        let add_res = app
            .run(
                "frame.create",
                json!({
                    "page": 1,
                    "rect": [72, 72, 300, 400],
                    "content": "text"
                }),
            )
            .unwrap();
        let fid = add_res.get("id").unwrap().as_u64().unwrap();

        // Select the text frame
        app.run("selection.set", json!({"ids": [fid]})).unwrap();

        // Write a small synthetic RTF
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("small_sample.rtf");
        std::fs::write(&file_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Courier;}}\f0\fs24 Hello DesignCraft RTF!}"#).unwrap();

        let mut st = PubgenState::default();
        let res = execute_page_from_rtf(&mut app, &mut st, &file_path.to_string_lossy());
        assert!(res.is_ok(), "execute_page_from_rtf should place into selected frame: {res:?}");

        let active = app.session.active().unwrap();
        let story_id = match active.doc.item(designcraft_doc::ItemId(fid)).unwrap().content {
            designcraft_doc::Content::Text(ref tf) => tf.story,
            _ => panic!("Expected text frame"),
        };
        let story_text = &active.doc.story(story_id).unwrap().text;
        assert!(story_text.contains("Hello DesignCraft RTF!"), "Story text should match: '{story_text}'");
    }

    #[test]
    fn test_execute_page_from_rtf_error_handling() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState::default();

        // Test empty path
        let res_empty = execute_page_from_rtf(&mut app, &mut st, "");
        assert!(res_empty.is_err());
        assert_eq!(res_empty.unwrap_err(), "No RTF file path specified");

        // Test non-existent path
        let res_missing = execute_page_from_rtf(&mut app, &mut st, "non_existent_file_path_12345.rtf");
        assert!(res_missing.is_err());
        assert!(res_missing.unwrap_err().contains("File not found"));
    }

    #[test]
    fn test_execute_page_from_image_as_object_fits_page_margins() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState { image_as_background: false, ..PubgenState::default() };
        let res = execute_page_from_image(&mut app, &mut st, &sample_image_path());
        assert!(res.is_ok(), "execute_page_from_image (object) should succeed: {res:?}");

        // The graphic frame sits proportionally inside the page margin rect.
        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(0).unwrap();
        let expected = active.doc.spreads[si].pages[pi].margin_rect();
        let frame = active.doc.spreads[si]
            .items
            .iter()
            .find(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)))
            .unwrap_or_else(|| panic!("expected a graphic frame on the page"));
        let b = frame.bounds();
        assert!(b.x0 >= expected.x0 - 0.01 && b.y0 >= expected.y0 - 0.01, "{b:?} vs {expected:?}");
        assert!(b.x1 <= expected.x1 + 0.01 && b.y1 <= expected.y1 + 0.01, "{b:?} vs {expected:?}");
        assert!((b.width() - b.height()).abs() < 0.01, "Aspect ratio preserved: {b:?}");
        assert!(((b.x0 + b.x1) / 2.0 - (expected.x0 + expected.x1) / 2.0).abs() < 0.01);
        assert!(((b.y0 + b.y1) / 2.0 - (expected.y0 + expected.y1) / 2.0).abs() < 0.01);
    }

    #[test]
    fn test_execute_page_from_image_as_background_covers_page_and_goes_to_back() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Existing content on the page: the background must end up behind it.
        let text = app.run("frame.create", json!({ "rect": [72, 72, 300, 400], "content": "text", "text": "Hello" })).unwrap();
        let text_id = text.get("id").and_then(Value::as_u64).unwrap();

        let mut st = PubgenState { image_as_background: true, ..PubgenState::default() };
        let res = execute_page_from_image(&mut app, &mut st, &sample_image_path());
        assert!(res.is_ok(), "execute_page_from_image (background) should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(0).unwrap();
        let page = &active.doc.spreads[si].pages[pi];
        let expected = page.bounds();

        let bg_index = active.doc.spreads[si]
            .items
            .iter()
            .position(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)))
            .unwrap_or_else(|| panic!("expected a background graphic frame"));
        assert_eq!(bg_index, 0, "the background frame must be at the back of the spread");
        let text_index = active.doc.spreads[si].items.iter().position(|it| it.id.0 == text_id).unwrap();
        assert!(text_index > bg_index, "existing items stay above the background");

        let b = active.doc.spreads[si].items[bg_index].bounds();
        assert!(b.x0 >= expected.x0 - 0.01 && b.y0 >= expected.y0 - 0.01, "{b:?} vs {expected:?}");
        assert!(b.x1 <= expected.x1 + 0.01 && b.y1 <= expected.y1 + 0.01, "{b:?} vs {expected:?}");
        assert!((b.width() - b.height()).abs() < 0.01, "Aspect ratio preserved: {b:?}");
        assert!((b.width() - expected.width()).abs() < 0.01 || (b.height() - expected.height()).abs() < 0.01);
        assert!(((b.x0 + b.x1) / 2.0 - (expected.x0 + expected.x1) / 2.0).abs() < 0.01);
        assert!(((b.y0 + b.y1) / 2.0 - (expected.y0 + expected.y1) / 2.0).abs() < 0.01);
    }

    #[test]
    fn test_execute_page_from_image_error_handling() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let mut st = PubgenState::default();

        let res_empty = execute_page_from_image(&mut app, &mut st, "");
        assert!(res_empty.is_err());
        assert_eq!(res_empty.unwrap_err(), "No image file path specified");

        let res_missing = execute_page_from_image(&mut app, &mut st, "non_existent_image_12345.png");
        assert!(res_missing.is_err());
        assert!(res_missing.unwrap_err().contains("File not found"));
    }

    #[test]
    fn test_execute_page_from_image_contains_a_non_square_image_on_every_side() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // A 4:1 image: fitted into a page it must be shorter than the page, not cropped.
        let png = designcraft_render::Rendered { width: 400, height: 100, pixels: vec![200; 400 * 100 * 4] }.to_png();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wide.png");
        std::fs::write(&path, &png).unwrap();

        let mut st = PubgenState { image_as_background: true, ..PubgenState::default() };
        let res = execute_page_from_image(&mut app, &mut st, &path.to_string_lossy());
        assert!(res.is_ok(), "execute_page_from_image should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(0).unwrap();
        let page_bounds = active.doc.spreads[si].pages[pi].bounds();
        let it = active.doc.spreads[si]
            .items
            .iter()
            .find(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)))
            .unwrap_or_else(|| panic!("expected a graphic frame"));
        let designcraft_doc::Content::Graphic(g) = &it.content else { panic!("expected a graphic") };
        let (nw, nh) = g.size;
        let placed = g.xf.transform_rect_bbox(designcraft_geom::Rect::new(0.0, 0.0, nw, nh));

        // Aspect ratio kept.
        assert!((placed.width() / placed.height() - nw / nh).abs() < 0.01, "aspect kept: {placed:?}");
        // All four sides inside the page.
        assert!(placed.x0 >= page_bounds.x0 - 0.01 && placed.x1 <= page_bounds.x1 + 0.01, "{placed:?} vs {page_bounds:?}");
        assert!(placed.y0 >= page_bounds.y0 - 0.01 && placed.y1 <= page_bounds.y1 + 0.01, "{placed:?} vs {page_bounds:?}");
        // The wide image is limited by width, so it is shorter than the page (no height overflow).
        assert!(placed.height() < page_bounds.height(), "no vertical overflow: {placed:?} vs {page_bounds:?}");
        // Centred on the page.
        assert!(((placed.x0 + placed.x1) / 2.0 - page_bounds.center().x).abs() < 0.5, "centred: {placed:?}");
        assert!(((placed.y0 + placed.y1) / 2.0 - page_bounds.center().y).abs() < 0.5, "centred: {placed:?}");
    }

    #[test]
    fn test_execute_set_page_margins() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let res = execute_set_page_margins(&mut app);
        assert!(res.is_ok(), "execute_set_page_margins should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let page = active.doc.page(0).expect("first page exists");

        const PT_PER_MM: f64 = 72.0 / 25.4;
        let expected_top = 20.20 * PT_PER_MM;
        let expected_bottom = 20.0 * PT_PER_MM;
        let expected_inside = 19.0 * PT_PER_MM;
        let expected_outside = 15.0 * PT_PER_MM;

        assert!((page.margins.top - expected_top).abs() < 1e-6, "Top margin mismatch: got {}, expected {}", page.margins.top, expected_top);
        assert!(
            (page.margins.bottom - expected_bottom).abs() < 1e-6,
            "Bottom margin mismatch: got {}, expected {}",
            page.margins.bottom,
            expected_bottom
        );
        assert!(
            (page.margins.inside - expected_inside).abs() < 1e-6,
            "Inside margin mismatch: got {}, expected {}",
            page.margins.inside,
            expected_inside
        );
        assert!(
            (page.margins.outside - expected_outside).abs() < 1e-6,
            "Outside margin mismatch: got {}, expected {}",
            page.margins.outside,
            expected_outside
        );

        // Also verify parent pages
        for sp in &active.doc.parents {
            for pg in &sp.pages {
                assert!((pg.margins.top - expected_top).abs() < 1e-6);
                assert!((pg.margins.bottom - expected_bottom).abs() < 1e-6);
                assert!((pg.margins.inside - expected_inside).abs() < 1e-6);
                assert!((pg.margins.outside - expected_outside).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn test_add_blank_page_preserves_facing_pages_spreads() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"pages": 5})).unwrap();

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.spreads.len(), 3);
        assert_eq!(active.doc.spreads[1].pages.len(), 2);

        let res = execute_add_blank_page(&mut app, 2);
        assert!(res.is_ok(), "execute_add_blank_page failed: {:?}", res);

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 6);
        for (i, sp) in active.doc.spreads.iter().enumerate() {
            assert!(sp.pages.len() <= 2, "Spread {i} has {} pages (> 2), violated facing pages rule!", sp.pages.len());
        }
    }
}
