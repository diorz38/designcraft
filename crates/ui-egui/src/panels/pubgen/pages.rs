//! The page generation engine: one CSV record -> one templated document page.

use super::*;

fn post_process_template_applied_page(
    app: &mut DesignApp,
    st: &PubgenState,
    applied_info: &Value,
    is_cover_or_div: bool,
    doc_margins: designcraft_doc::Margins,
    is_narrative_row: bool,
    title_val: &str,
    type_val: &str,
) -> Option<u64> {
    let mut table_placeholder_id = None;
    let mut story_ids_to_tag = Vec::new();
    let mut body_story_ids = Vec::new();
    let mut image_slot_ids = Vec::new();
    // Dual Lang Mode: the divider's English title boxes (styled after the loop below).
    let mut title_en_stories: Vec<designcraft_doc::StoryId> = Vec::new();
    if let Some(items) = applied_info.get("items").and_then(Value::as_array) {
        if let Some(active) = app.session.active() {
            for it_id_val in items {
                if let Some(it_id) = it_id_val.as_u64() {
                    let item_id = designcraft_doc::ItemId(it_id);
                    if let Some(loc) = active.doc.find(item_id)
                        && let designcraft_doc::SpreadRef::Doc(si) = loc.spread
                        && let Some(spread) = active.doc.spreads.get(si)
                        && let Some(item) = spread.items.iter().find(|i| i.id == item_id)
                    {
                        if st.bilingual_output
                            && type_val.eq_ignore_ascii_case("Divider")
                            && item.label.starts_with("#slot:text:title_en")
                            && let Some(tf) = item.text_frame()
                            && !title_en_stories.contains(&tf.story)
                        {
                            title_en_stories.push(tf.story);
                        }
                        if item.label.starts_with("#slot:table")
                            || (item.label.contains("table") && !matches!(item.content, designcraft_doc::Content::Text(_)))
                        {
                            table_placeholder_id = Some(it_id);
                        } else if (item.label.contains("title") || item.label.contains("chapter_title"))
                            && let Some(tf) = item.text_frame()
                        {
                            story_ids_to_tag.push(tf.story);
                        } else if item.label.starts_with("#slot:story:")
                            && let Some(tf) = item.text_frame()
                        {
                            body_story_ids.push(tf.story);
                        } else if item.label.contains("image") || item.label.contains("photo") || item.label.contains("hero") {
                            image_slot_ids.push(it_id);
                        }
                    }
                }
            }
        }

        // Dual Lang Mode: the divider's title_en textbox is 22 pt italic with a black
        // stroke behind the fill.
        for sid in title_en_stories {
            let len = app.session.active().and_then(|a| a.doc.story(sid)).map(|story| story.text.len()).unwrap_or(0);
            if len > 0 {
                let _ = app.run("text.select", json!({"story": sid, "anchor": 0, "focus": len}));
                let _ = app.run("type.char", json!({"size": 22.0, "fontStyle": "Italic", "stroke": "[Black]", "strokeBehindFill": true}));
            }
        }

        for img_id in image_slot_ids {
            let fit_mode = if is_cover_or_div { "fitContentToFrame" } else { "fitProportionally" };
            let _ = app.run("object.fit", json!({"mode": fit_mode, "ids": [img_id]}));
            let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [img_id]}));
            let _ = app.run("object.fittingOptions", json!({"ids": [img_id], "fitting": fit_mode}));
            let _ = app.run("object.stroke", json!({"swatch": "[None]", "ids": [img_id]}));
        }

        if !story_ids_to_tag.is_empty()
            && let Some(active) = app.session.active_mut()
        {
            // A narrative page (the G and Ulasan sheets) carries the narrative style, so the
            // paragraph table of contents lists it and nothing else.
            let title_style = if is_narrative_row { TOC_NARRATIVE_STYLE.to_string() } else { st.toc_style.clone() };
            let doc = std::sync::Arc::make_mut(&mut active.doc);
            for sid in story_ids_to_tag {
                if let Some(story) = doc.story_mut(sid) {
                    for p in &mut story.paras {
                        p.style = title_style.clone();
                    }
                }
            }
        }

        // A sheet with no title line puts the RTF at the top of the page, so the page's title is
        // the RTF's own heading: tag that paragraph for the paragraph table of contents.
        if title_val.trim().is_empty() && (is_narrative_row || type_val.eq_ignore_ascii_case("Note")) {
            for sid in body_story_ids {
                tag_page_title(app, sid, TOC_NARRATIVE_STYLE);
            }
        }
    }

    // 1. Update page margins for modular template:
    // DIV/COVER: margins = 0.0 (adjust to Page Size)
    // Content page: margins = doc_margins (adjust to Page Margin)
    let page_margins_info = app.session.edit(|doc, _| {
        let cur_p = doc.page_count().saturating_sub(1);
        if let Some((si, pi)) = doc.page_loc(cur_p)
            && let Some(sp) = doc.spreads.get_mut(si)
        {
            let sp = std::sync::Arc::make_mut(sp);
            if let Some(pg) = sp.pages.get_mut(pi) {
                pg.margins = if is_cover_or_div { designcraft_doc::Margins::uniform(0.0) } else { doc_margins };
                pg.local_margins = true;
            }
            sp.relayout();
            if let Some(pg) = sp.pages.get(pi) {
                return Ok(serde_json::json!({
                    "x0": pg.margin_rect().x0,
                    "x1": pg.margin_rect().x1,
                    "page_x0": pg.bounds().x0,
                    "page_w": pg.bounds().width(),
                }));
            }
        }
        Ok(serde_json::Value::Null)
    });

    // For content pages (not COVER or DIV), adjust template items to follow page margins
    if !is_cover_or_div
        && let Ok(info_val) = page_margins_info
        && let (Some(m_x0), Some(m_x1), Some(pg_x0), Some(pg_w)) = (
            info_val.get("x0").and_then(Value::as_f64),
            info_val.get("x1").and_then(Value::as_f64),
            info_val.get("page_x0").and_then(Value::as_f64),
            info_val.get("page_w").and_then(Value::as_f64),
        )
        && let Some(items) = applied_info.get("items").and_then(Value::as_array)
    {
        let item_ids: Vec<designcraft_doc::ItemId> = items.iter().filter_map(Value::as_u64).map(designcraft_doc::ItemId).collect();

        let local_m_x0 = m_x0 - pg_x0;
        let local_m_x1 = m_x1 - pg_x0;
        let _ = app.session.edit(|doc, _| {
            for &it_id in &item_ids {
                if let Some(it) = doc.item_mut(it_id)
                    && let Some(pb) = it.path.bounds()
                {
                    // Template items originally placed at template's default margin (36pt)
                    if (pb.x0 - 36.0).abs() < 4.0 {
                        let is_full_width = (pb.x1 - (pg_w - 36.0)).abs() < 4.0;
                        let new_x1 = if is_full_width { local_m_x1 } else { (local_m_x0 + pb.width()).min(local_m_x1) };
                        it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(local_m_x0, pb.y0, new_x1, pb.y1));
                    }
                }
            }
            Ok(serde_json::Value::Null)
        });
    }

    table_placeholder_id
}

/// The template text/story/image slots for one Page-from-CSV record in Modular Template
/// Mode, plus how many image/RTF files resolved (for the generation status counts).
/// Pure over `(row columns, state)` — no app, no document — so it is unit-testable.
struct RowSlots {
    text: serde_json::Map<String, Value>,
    story: serde_json::Map<String, Value>,
    image: serde_json::Map<String, Value>,
    images_resolved: usize,
    rtfs_resolved: usize,
}

#[allow(clippy::too_many_arguments)]
fn row_template_slots(
    type_val: &str,
    num_val: &str,
    title_val: &str,
    title_en_val: &str,
    file_val: &str,
    is_preface: bool,
    st: &PubgenState,
) -> RowSlots {
    let mut text = serde_json::Map::new();
    let mut story = serde_json::Map::new();
    let mut image = serde_json::Map::new();
    let mut images_resolved = 0usize;
    let mut rtfs_resolved = 0usize;

    let file_opt = Some(file_val).filter(|s| !s.is_empty());

    if is_divider_type(type_val) {
        let (first_word, digit_num, rest_text) = excel::split_divider_title(title_val);
        let prefix = if first_word.is_empty() || first_word.chars().all(|c| c.is_ascii_digit()) {
            "BAB".to_string()
        } else if !rest_text.is_empty() {
            first_word.clone()
        } else {
            "BAB".to_string()
        };
        text.insert("first_word".into(), json!(prefix));
        text.insert("prefix".into(), json!(prefix));
        text.insert("chapter_prefix".into(), json!(prefix));
        text.insert("title_prefix".into(), json!(prefix));

        // 2. digit number (replacing text "01")
        let final_digit = if !digit_num.is_empty() { digit_num } else { num_val.to_string() };
        text.insert("chapter_num".into(), json!(final_digit));
        text.insert("number".into(), json!(final_digit));
        text.insert("digit".into(), json!(final_digit));
        text.insert("num".into(), json!(final_digit));

        // 3. rest of text (replacing text "Geografi")
        let mut final_title = if !rest_text.is_empty() {
            rest_text
        } else if !first_word.eq_ignore_ascii_case("bab") && !first_word.chars().all(|c| c.is_ascii_digit()) {
            first_word
        } else {
            title_val.to_string()
        };

        let title_en = if st.bilingual_output {
            if !title_en_val.is_empty() {
                title_en_val.to_string()
            } else if let Some(en) = st
                .translations
                .get(final_title.trim())
                .or_else(|| st.translations.get(title_val.trim()))
                .map(String::as_str)
                .filter(|s| !s.is_empty())
            {
                // AI translation (plan §4): explicit title_en always wins, then the map.
                en.to_string()
            } else if let Some((id_t, en_t)) = locale::split_pair(&final_title) {
                let (id_s, en_s) = (id_t.to_string(), en_t.to_string());
                final_title = id_s;
                en_s
            } else {
                String::new()
            }
        } else {
            if let Some((id_t, en_t)) = locale::split_pair(&final_title) {
                let (id_s, en_s) = (id_t.to_string(), en_t.to_string());
                final_title = match st.locale {
                    locale::Locale::Id => id_s,
                    locale::Locale::En => en_s,
                };
            }
            String::new()
        };

        text.insert("chapter_title".into(), json!(final_title));
        text.insert("title".into(), json!(final_title));
        text.insert("rest".into(), json!(final_title));
        text.insert("title_rest".into(), json!(final_title));
        if !is_preface {
            text.insert("title_en".into(), json!(title_en));
            text.insert("chapter_title_en".into(), json!(title_en));
        }

        // Shape with color #e5007e is placeholder for image in col image
        if let Some(file_str) = file_opt
            && let Some(img_file) = resolve_image_file(&st.card1_path, file_str, &["png", "jpg", "jpeg", "pdf"])
        {
            image.insert("image".into(), json!(img_file.to_string_lossy()));
            image.insert("photo".into(), json!(img_file.to_string_lossy()));
            image.insert("hero".into(), json!(img_file.to_string_lossy()));
            image.insert("divider_image".into(), json!(img_file.to_string_lossy()));
            images_resolved = 1;
        }
    } else if is_ulasan_type(type_val) {
        let (clean_id_title, _) = match locale::split_pair(title_val) {
            Some((id_t, en_t)) => (id_t, en_t),
            None => (title_val, ""),
        };
        let (ch_line, title_line) = split_chapter_title(clean_id_title, num_val);
        text.insert("chapter_num".into(), json!(ch_line));
        text.insert("chapter_title".into(), json!(title_line));
        text.insert("title".into(), json!(title_line));
        text.insert("number".into(), json!(ch_line));
        if let Some(file_str) = file_opt
            && let Some(rtf_file) = resolve_catalog_related_file(&st.card1_path, file_str, "rtf")
        {
            story.insert("narrative_body".into(), json!(rtf_file.to_string_lossy()));
            rtfs_resolved = 1;
        }
    } else if type_val.eq_ignore_ascii_case("Note") || type_val.eq_ignore_ascii_case("G") {
        let note_title = title_val.trim();
        text.insert("title".into(), json!(note_title));
        text.insert("note_title".into(), json!(note_title));
        if let Some(file_str) = file_opt
            && let Some(rtf_file) = resolve_catalog_related_file(&st.card1_path, file_str, "rtf")
        {
            story.insert("note_body".into(), json!(rtf_file.to_string_lossy()));
            rtfs_resolved = 1;
        }
    } else if is_chart_type(type_val) {
        let (ch_line, title_line) = split_chart_title(title_val, num_val, type_val);
        text.insert("chart_num".into(), json!(ch_line));
        text.insert("chart_title".into(), json!(title_line));
        text.insert("title".into(), json!(title_line));
        text.insert("number".into(), json!(ch_line));
        if st.bilingual_output && !title_en_val.is_empty() {
            text.insert("chart_title_en".into(), json!(title_en_val));
            text.insert("title_en".into(), json!(title_en_val));
        } else if st.bilingual_output
            && let Some(en) =
                st.translations.get(title_line.trim()).or_else(|| st.translations.get(title_val.trim())).map(String::as_str).filter(|s| !s.is_empty())
        {
            text.insert("chart_title_en".into(), json!(en));
            text.insert("title_en".into(), json!(en));
        }
        if let Some(file_str) = file_opt
            && let Some(img_file) = resolve_image_file(&st.card1_path, file_str, &["png", "jpg", "jpeg"])
        {
            image.insert("chart_image".into(), json!(img_file.to_string_lossy()));
            image.insert("image".into(), json!(img_file.to_string_lossy()));
            images_resolved = 1;
        }
    } else {
        let (id_title, en_title_opt) =
            excel::split_and_harmonize_table_title(title_val, if title_en_val.is_empty() { None } else { Some(title_en_val) });
        // AI translation (plan §4): an explicit ` / ` pair wins, else the session map.
        let en_title_opt = match en_title_opt {
            Some(en) => Some(en),
            None if st.bilingual_output => st.translations.get(id_title.trim()).map(String::as_str).filter(|s| !s.is_empty()).map(str::to_string),
            None => None,
        };
        let formatted_num = excel::format_table_number(num_val, file_val);
        if st.bilingual_output || en_title_opt.is_some() {
            let kind_en = format!("{} {}", locale::words(locale::Locale::En).table, formatted_num);
            text.insert("table_num".into(), json!(format!("{} {}", locale::words(locale::Locale::Id).table, formatted_num)));
            text.insert("table_kind_en".into(), json!(kind_en));
            text.insert("table_title".into(), json!(id_title));
            text.insert("table_title_en".into(), json!(en_title_opt.unwrap_or_default()));
            text.insert("title".into(), json!(id_title));
            text.insert("number".into(), json!(formatted_num));
        } else {
            let table_title_val = match st.locale {
                locale::Locale::Id => id_title,
                locale::Locale::En => en_title_opt.unwrap_or(id_title),
            };
            text.insert("table_num".into(), json!(formatted_num));
            text.insert("table_kind_en".into(), json!(""));
            text.insert("table_title".into(), json!(table_title_val));
            text.insert("table_title_en".into(), json!(""));
            text.insert("title".into(), json!(table_title_val));
            text.insert("number".into(), json!(formatted_num));
        }
    }

    RowSlots { text, story, image, images_resolved, rtfs_resolved }
}

pub(crate) fn execute_generate_pages(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    if st.card1_records.is_empty() {
        return Err("No records to generate".into());
    }

    let total = st.card1_records.len();
    let (start_idx, end_idx) = if st.card1_range_all {
        (0, total)
    } else {
        let from = st.card1_range_from.clamp(1, total).saturating_sub(1);
        let to = st.card1_range_to.clamp(from + 1, total);
        (from, to)
    };

    let slice = st.card1_records.get(start_idx..end_idx).ok_or("Invalid range")?;
    let mut generated_count = 0usize;
    let mut inserted_tables_count = 0usize;
    let mut inserted_rtfs_count = 0usize;
    let mut inserted_images_count = 0usize;

    // Every preset tags its title (or the combined TOC line) with the TOC paragraph style, so the
    // Quick Actions "Add TOC" lists one entry per generated page. The narrative pages (the G and
    // Ulasan sheets) carry the narrative style instead, so the paragraph list names them alone.
    ensure_para_style(app, &st.toc_style);
    ensure_para_style(app, TOC_NARRATIVE_STYLE);

    let doc_margins = {
        let active = app.session.active().ok_or("No active document")?;
        detect_document_margins(&active.doc)
    };

    let file_col = effective_col(st.card1_file_col, &st.card1_headers, &["file", "filename"], 3);
    let template_col = effective_col(st.card1_template_col, &st.card1_headers, &["template"], 4);
    let title_en_col = effective_col(st.card1_title_en_col, &st.card1_headers, &["title_en", "titleen"], 5);
    let file_en_col = effective_col(st.card1_file_en_col, &st.card1_headers, &["file_en", "fileen"], 6);
    let theme_col = effective_col(999, &st.card1_headers, &["theme", "themes", "table_theme", "tabletheme", "color", "colortheme", "warna"], 999);

    for row in slice {
        let type_val = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let num_val = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let title_val = row.get(st.card1_title_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let title_en_val = row.get(title_en_col).map(String::as_str).unwrap_or("").trim();
        let file_en_val = row.get(file_en_col).map(String::as_str).unwrap_or("").trim();
        let theme_val = row.get(theme_col).map(String::as_str).unwrap_or("").trim();
        let file_val = row.get(file_col).map(String::as_str).unwrap_or("").trim();
        let (clean_id_title, clean_en_title) = if let Some((id_t, en_t)) = locale::split_pair(title_val) {
            (id_t, if title_en_val.is_empty() { en_t } else { title_en_val })
        } else {
            (title_val, title_en_val)
        };
        let has_chapter_prefix = clean_id_title.split_whitespace().next().is_some_and(|w| w.eq_ignore_ascii_case("bab"));
        // The preface stays single-language: it is not processed in Dual Lang Mode.
        let is_preface = title_val.eq_ignore_ascii_case("preface") || clean_id_title.eq_ignore_ascii_case("preface");
        let is_div = is_divider_type(type_val);
        let is_cover_or_div = is_div || type_val.eq_ignore_ascii_case("COVER");
        // The narrative sheets: `GXX` (the front matter) and `RXX` (the chapter reviews). Their
        // pages are the ones the paragraph table of contents lists.
        let is_narrative_row = type_val.eq_ignore_ascii_case("Ulasan") || type_val.eq_ignore_ascii_case("G");

        let row_tpl_spec = row.get(template_col).map(String::as_str).unwrap_or("").trim();
        let explicit_tpl_path = if !row_tpl_spec.is_empty() { resolve_template_path_spec(&st.card1_path, row_tpl_spec) } else { None };

        // GUARD: An Ulasan narrative review chapter must NEVER use a graphic divider-only template
        // (such as 002_batik_infographic) because it has no story slot for the narrative RTF body.
        let explicit_tpl_path = if is_ulasan_type(type_val) {
            if let Some(ref ep) = explicit_tpl_path {
                let ep_lower = ep.to_lowercase();
                if ep_lower.contains("002")
                    || ep_lower.contains("batik")
                    || ep_lower.contains("infographic")
                    || ep_lower.contains("chapter_devider")
                    || ep_lower.contains("special_infographic_divider")
                {
                    if st.card1_preset == 6 { resolve_template_path_spec(&st.card1_path, "001_simple_chapter") } else { None }
                } else {
                    Some(ep.clone())
                }
            } else {
                None
            }
        } else {
            explicit_tpl_path
        };

        if explicit_tpl_path.is_some() || st.card1_preset == 6 || is_div {
            // Modular Template Mode: delegate to template engine
            let raw_tpl = if let Some(ref ep) = explicit_tpl_path {
                ep.clone()
            } else if !st.card1_template_path.trim().is_empty() {
                let custom_tpl = st.card1_template_path.trim().to_string();
                if is_ulasan_type(type_val) {
                    let custom_lower = custom_tpl.to_lowercase();
                    if custom_lower.contains("002")
                        || custom_lower.contains("batik")
                        || custom_lower.contains("infographic")
                        || custom_lower.contains("chapter_devider")
                        || custom_lower.contains("special_infographic_divider")
                    {
                        "assets/templates/dividers/001_simple_chapter.designcraft".to_string()
                    } else {
                        custom_tpl
                    }
                } else {
                    custom_tpl
                }
            } else if is_divider_type(type_val) {
                "assets/templates/dividers/002_batik_infographic.designcraft".to_string()
            } else if is_ulasan_type(type_val) {
                "assets/templates/dividers/001_simple_chapter.designcraft".to_string()
            } else if type_val.eq_ignore_ascii_case("Note") || type_val.eq_ignore_ascii_case("G") {
                "assets/templates/front_matter/001_publication_note.designcraft".to_string()
            } else {
                "assets/templates/data_tables/001_single_table_standard.designcraft".to_string()
            };

            let tpl_path = resolve_template_path_spec(&st.card1_path, &raw_tpl).unwrap_or(raw_tpl);

            if std::path::Path::new(&tpl_path).exists() {
                let RowSlots { text: text_slots, story: story_slots, image: image_slots, images_resolved, rtfs_resolved } =
                    row_template_slots(type_val, num_val, title_val, title_en_val, file_val, is_preface, st);
                inserted_images_count = inserted_images_count.saturating_add(images_resolved);
                inserted_rtfs_count = inserted_rtfs_count.saturating_add(rtfs_resolved);

                let mut apply_params = json!({
                    "path": tpl_path,
                    "slots": {
                        "text": text_slots,
                        "story": story_slots,
                        "image": image_slots,
                    },
                    "image_fitting": if is_div { "fitContentToFrame" } else { "fitProportionally" },
                    "palette": {
                        "Primary": st.cover_color_primary.trim(),
                        "Accent": st.cover_color_accent.trim(),
                        "Dark": st.cover_color_dark.trim(),
                    }
                });
                if !st.doc_language.trim().is_empty() {
                    apply_params["lang"] = json!(st.doc_language.trim());
                }
                let apply_res = app.run("template.page.apply", apply_params);

                if let Ok(applied_info) = apply_res {
                    let table_placeholder_id = post_process_template_applied_page(
                        app,
                        st,
                        &applied_info,
                        is_cover_or_div,
                        doc_margins,
                        is_narrative_row,
                        title_val,
                        type_val,
                    );
                    generated_count = generated_count.saturating_add(1);

                    // Auto-insert table from CSV if enabled and present
                    if st.card1_auto_insert_tables
                        && let Some(file_str) = Some(file_val).filter(|s| !s.is_empty())
                        && let Some(tbl_file) = resolve_catalog_related_file(&st.card1_path, file_str, "csv")
                        && let Ok(content) = std::fs::read_to_string(&tbl_file)
                    {
                        if let Some(tbl_slot_id) = table_placeholder_id {
                            let _ = app.run("edit.clear", json!({"ids": [tbl_slot_id]}));
                        }
                        let delim = detect_delimiter(&content);
                        let tbl_matrix = parse_csv_content_with_delim(&content, delim);
                        if !tbl_matrix.is_empty() {
                            // Dual Lang + AI: one table, two lines per cell (id\u{2028}en);
                            // skipped cells (numbers, aliases) stay single-line (plan §4.1).
                            let display_matrix =
                                if st.bilingual_output && st.translate_with_ai { ai::pair_cells(&tbl_matrix, &st.translations) } else { tbl_matrix };
                            let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                            let mut table_st = st.clone();
                            if !theme_val.is_empty() {
                                table_st.card2_theme_override = Some(theme_val.to_string());
                            }
                            if execute_insert_table_matrix(app, &table_st, Some(&display_matrix), Some(cur_page_idx)).is_ok() {
                                inserted_tables_count = inserted_tables_count.saturating_add(1);
                            }
                        }
                    }

                    // Sequential EN spread for Ulasan — the preface is not processed in Dual
                    // Lang Mode. With AI translation, a missing file_en uses the temp RTF
                    // built from the translated paragraphs (plan §4).
                    let en_rtf_resolved = if !file_en_val.is_empty() {
                        resolve_catalog_related_file(&st.card1_path, file_en_val, "rtf")
                    } else if st.translate_with_ai && !file_val.is_empty() {
                        ai::translated_body_rtf(st, file_val)
                    } else {
                        None
                    };
                    if st.bilingual_output
                        && !is_preface
                        && is_ulasan_type(type_val)
                        && let Some(en_rtf_file) = en_rtf_resolved
                    {
                        let (en_ch_line, en_title_line) = if has_chapter_prefix {
                            if !clean_en_title.is_empty() {
                                (format!("CHAPTER {num_val}"), clean_en_title.to_string())
                            } else if let Ok(en_bytes) = std::fs::read(&en_rtf_file)
                                && let Ok(imp) = designcraft_textimport::import("review_en.rtf", &en_bytes)
                                && let Some(first_para) = imp.story.text.lines().map(str::trim).find(|l| !l.is_empty())
                            {
                                split_chapter_title_locale(first_para, num_val, true)
                            } else {
                                (format!("CHAPTER {num_val}"), clean_id_title.to_string())
                            }
                        } else {
                            (String::new(), if !clean_en_title.is_empty() { clean_en_title.to_string() } else { clean_id_title.to_string() })
                        };

                        let mut en_text_slots = serde_json::Map::new();
                        en_text_slots.insert("chapter_num".into(), json!(en_ch_line));
                        en_text_slots.insert("chapter_title".into(), json!(en_title_line));
                        en_text_slots.insert("title".into(), json!(en_title_line));
                        en_text_slots.insert("number".into(), json!(en_ch_line));

                        let mut en_story_slots = serde_json::Map::new();
                        en_story_slots.insert("narrative_body".into(), json!(en_rtf_file.to_string_lossy()));

                        let mut en_apply_params = json!({
                            "path": tpl_path,
                            "slots": {
                                "text": en_text_slots,
                                "story": en_story_slots,
                                "image": serde_json::Map::new(),
                            },
                            "image_fitting": "fitProportionally",
                            "palette": {
                                "Primary": st.cover_color_primary.trim(),
                                "Accent": st.cover_color_accent.trim(),
                                "Dark": st.cover_color_dark.trim(),
                            }
                        });
                        if !st.doc_language.trim().is_empty() {
                            en_apply_params["lang"] = json!(st.doc_language.trim());
                        }

                        if let Ok(en_applied) = app.run("template.page.apply", en_apply_params) {
                            post_process_template_applied_page(app, st, &en_applied, false, doc_margins, is_narrative_row, title_val, type_val);
                            let en_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                            italicize_page_stories(app, en_page_idx, en_page_idx);
                            inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                            generated_count = generated_count.saturating_add(1);
                        }
                    }

                    continue;
                }
            }

            // Procedural Divider Fallback (Option 1: like Cover template, works without assets on disk)
            if is_div {
                app.run("layout.pages.insert", json!({"count": 1}))?;
                let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                crate::canvas::go_to_page(app, cur_page_idx);
                let _ = app.session.edit(|doc, _| {
                    if let Some((si, pi)) = doc.page_loc(cur_page_idx)
                        && let Some(sp) = doc.spreads.get_mut(si)
                    {
                        let sp = std::sync::Arc::make_mut(sp);
                        if let Some(pg) = sp.pages.get_mut(pi) {
                            pg.margins = designcraft_doc::Margins::uniform(0.0);
                            pg.local_margins = true;
                        }
                        sp.relayout();
                    }
                    Ok(serde_json::Value::Null)
                });

                let (pw, ph, spread_idx) = {
                    let active = app.session.active().ok_or("No active document")?;
                    let (si, pi) = active.doc.page_loc(cur_page_idx).ok_or("Page loc missing")?;
                    let spread = active.doc.spreads.get(si).ok_or("Spread missing")?;
                    let page = spread.pages.get(pi).ok_or("Page missing")?;
                    (page.width, page.height, si)
                };

                let (first_word, digit_num, rest_text) = excel::split_divider_title(title_val);
                let final_num = if !digit_num.is_empty() { digit_num } else { num_val.to_string() };
                let prefix = if first_word.is_empty() || first_word.chars().all(|c| c.is_ascii_digit()) {
                    "BAB".to_string()
                } else if !rest_text.is_empty() {
                    first_word.clone()
                } else {
                    "BAB".to_string()
                };
                let mut chapter_title = if !rest_text.is_empty() {
                    rest_text
                } else if !first_word.eq_ignore_ascii_case("bab") && !first_word.chars().all(|c| c.is_ascii_digit()) {
                    first_word
                } else {
                    title_val.to_string()
                };

                let title_en = if st.bilingual_output {
                    if !title_en_val.is_empty() {
                        title_en_val.to_string()
                    } else if let Some((id_t, en_t)) = locale::split_pair(&chapter_title) {
                        let (id_s, en_s) = (id_t.to_string(), en_t.to_string());
                        chapter_title = id_s;
                        en_s
                    } else {
                        String::new()
                    }
                } else {
                    if let Some((id_t, en_t)) = locale::split_pair(&chapter_title) {
                        let (id_s, en_s) = (id_t.to_string(), en_t.to_string());
                        chapter_title = match st.locale {
                            locale::Locale::Id => id_s,
                            locale::Locale::En => en_s,
                        };
                    }
                    String::new()
                };

                let resolved_img = if let Some(file_str) = Some(file_val).filter(|s| !s.is_empty()) {
                    resolve_image_file(&st.card1_path, file_str, &["png", "jpg", "jpeg"])
                } else {
                    None
                };
                let img_str = resolved_img.as_ref().map(|p| p.to_string_lossy().to_string());

                let div_vals = DividerArtVals {
                    spread_idx,
                    pw,
                    ph,
                    primary: st.cover_color_primary.trim(),
                    accent: st.cover_color_accent.trim(),
                    dark: st.cover_color_dark.trim(),
                    first_word: &prefix,
                    chapter_num: &final_num,
                    chapter_title: &chapter_title,
                    title_en: &title_en,
                    image_path: img_str.as_deref(),
                };

                let row_tpl_lower = row_tpl_spec.to_lowercase();
                let style_choice = if row_tpl_lower.contains("simple") || row_tpl_lower.contains("001") { 1 } else { 2 };

                divider_art::generate_divider(app, div_vals, style_choice)?;
                if img_str.is_some() {
                    inserted_images_count = inserted_images_count.saturating_add(1);
                }
                generated_count = generated_count.saturating_add(1);
                continue;
            }
        }

        // 1. Insert Page
        app.run("layout.pages.insert", json!({"count": 1}))?;

        // Adjust page margins:
        // - DIV/COVER sheet: adjusts to Page Size (margins = 0)
        // - Content pages: adjust with Page Margin (margins = doc_margins)
        let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
        let _ = app.session.edit(|doc, _| {
            if let Some((si, pi)) = doc.page_loc(cur_page_idx)
                && let Some(sp) = doc.spreads.get_mut(si)
            {
                let sp = std::sync::Arc::make_mut(sp);
                if let Some(pg) = sp.pages.get_mut(pi) {
                    pg.margins = if is_cover_or_div { designcraft_doc::Margins::uniform(0.0) } else { doc_margins };
                    pg.local_margins = true;
                }
                sp.relayout();
            }
            Ok(serde_json::Value::Null)
        });

        // 2. Query Page Bounds & Margins
        let (margin, page_bounds, spread_idx) = {
            let active = app.session.active().ok_or("No active document")?;
            let doc = &active.doc;
            let (si, pi) = doc.page_loc(cur_page_idx).ok_or("Page loc missing")?;
            let spread = doc.spreads.get(si).ok_or("Spread missing")?;
            let page = spread.pages.get(pi).ok_or("Page missing")?;
            (page.margin_rect(), page.bounds(), si)
        };

        if st.card1_preset == 5 {
            // TOC Line: one combined paragraph per record (type + number + title), so each record
            // becomes a single table-of-contents entry.
            let parts: Vec<&str> = [type_val, num_val, title_val].into_iter().filter(|s| !s.is_empty()).collect();
            let line = parts.join(" ");
            if !line.is_empty() {
                frame_with_style(app, spread_idx, [margin.x0, margin.y0, margin.x1, margin.y0 + 40.0], &line, &st.toc_style);
            }
        } else if type_val.eq_ignore_ascii_case("Ulasan") {
            // Ulasan / Editorial Chapter layout:
            // Split title "BAB 01 Pemerintahan" into "BAB 01" and "Pemerintahan"
            let (ch_line, title_line) = split_chapter_title(clean_id_title, num_val);
            let resolved_en_rtf = if st.bilingual_output && !file_en_val.is_empty() && !is_preface {
                resolve_catalog_related_file(&st.card1_path, file_en_val, "rtf")
            } else if st.bilingual_output && st.translate_with_ai && !file_val.is_empty() && !is_preface {
                ai::translated_body_rtf(st, file_val)
            } else {
                None
            };

            if st.bilingual_output
                && has_chapter_prefix
                && let Some(en_rtf_file) = resolved_en_rtf
            {
                // Bilingual Review Chapter (RXX):
                // 3-Line Header on Page 1:
                // Line 1: "BAB 01" (Arial Black, 24.0)
                // Line 2: "Geografi" (Arial Black, 20.0, tagged with TOC style)
                // Line 3: "Chapter 01 Geography" (Arial Bold Italic, 15.0)
                if !ch_line.is_empty() {
                    frame_with_font_full(
                        app,
                        spread_idx,
                        [margin.x0, margin.y0, margin.x1, margin.y0 + 36.0],
                        &ch_line,
                        "Arial Black",
                        None,
                        24.0,
                        None,
                    );
                }
                // 2nd Textbox: "Pemerintahan" (font Arial Black, size 20.0) -> second row
                // Tag with the narrative style so the paragraph table of contents lists this
                // chapter (and only the narrative pages).
                if !title_line.is_empty() {
                    frame_with_font_full(
                        app,
                        spread_idx,
                        [margin.x0, margin.y0 + 40.0, margin.x1, margin.y0 + 72.0],
                        &title_line,
                        "Arial Black",
                        None,
                        20.0,
                        Some(TOC_NARRATIVE_STYLE),
                    );
                }
                let en_header_text = if !clean_en_title.is_empty() {
                    let t = clean_en_title.trim();
                    if t.to_lowercase().starts_with("chapter") {
                        t.to_string()
                    } else if !num_val.is_empty() {
                        format!("Chapter {num_val} {t}")
                    } else {
                        t.to_string()
                    }
                } else if !title_en_val.is_empty() {
                    title_en_val.to_string()
                } else if let Some(en) = st.translations.get(title_val.trim()).map(String::as_str).filter(|s| !s.is_empty()) {
                    // AI: the English half of the row title ("Bab 01 Geografi" -> ...).
                    let t = en.trim();
                    if t.to_lowercase().starts_with("chapter") {
                        t.to_string()
                    } else if !num_val.is_empty() {
                        format!("Chapter {num_val} {t}")
                    } else {
                        t.to_string()
                    }
                } else {
                    format!("Chapter {num_val} {title_line}")
                };
                if !en_header_text.is_empty() {
                    frame_with_font_full(
                        app,
                        spread_idx,
                        [margin.x0, margin.y0 + 76.0, margin.x1, margin.y0 + 104.0],
                        &en_header_text,
                        "Arial",
                        Some("Bold Italic"),
                        15.0,
                        None,
                    );
                }

                let header_h = 114.0;
                let gutter = 14.17; // 5 mm
                let total_w = (margin.x1 - margin.x0).max(40.0);
                let col_w = ((total_w - gutter) / 2.0).max(10.0);
                let left_rect = [margin.x0, margin.y0 + header_h, margin.x0 + col_w, margin.y1];
                let right_rect = [margin.x0 + col_w + gutter, margin.y0 + header_h, margin.x1, margin.y1];

                let left_fid = app
                    .run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": left_rect,
                            "content": "text",
                            "caret": false
                        }),
                    )
                    .ok()
                    .and_then(|res| res.get("id").and_then(Value::as_u64));

                let right_fid = app
                    .run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": right_rect,
                            "content": "text",
                            "caret": false
                        }),
                    )
                    .ok()
                    .and_then(|res| res.get("id").and_then(Value::as_u64));

                let mut cur_left_fid = left_fid;
                let mut cur_right_fid = right_fid;
                let mut id_over = false;
                let mut en_over = false;
                let mut en_story_id = None;

                if let Some(fid) = left_fid
                    && let Some(file_str) = Some(file_val).filter(|s| !s.is_empty())
                    && let Some(rtf_file) = resolve_catalog_related_file(&st.card1_path, file_str, "rtf")
                {
                    let rtf_path_str = rtf_file.to_string_lossy().to_string();
                    let res = app.run(
                        "file.place",
                        json!({
                            "path": rtf_path_str,
                            "frame": fid,
                            "autoflow": false,
                        }),
                    );
                    if let Err(e) = &res {
                        eprintln!("pubgen ai: EN body place failed: {e}");
                    }
                    if let Ok(val) = res {
                        let sid = val.get("story").and_then(Value::as_u64).map(designcraft_doc::StoryId);
                        id_over = val.get("overset").and_then(Value::as_bool).unwrap_or(false) || sid.is_some_and(|s| story_is_overset(app, s));
                        inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                    }
                }

                if let Some(fid) = right_fid {
                    let en_path_str = en_rtf_file.to_string_lossy().to_string();
                    let res = app.run(
                        "file.place",
                        json!({
                            "path": en_path_str,
                            "frame": fid,
                            "autoflow": false,
                        }),
                    );
                    if let Err(e) = &res {
                        eprintln!("pubgen ai: EN body place failed: {e}");
                    }
                    if let Ok(val) = res {
                        en_story_id = val.get("story").and_then(Value::as_u64);
                        let sid = en_story_id.map(designcraft_doc::StoryId);
                        en_over = val.get("overset").and_then(Value::as_bool).unwrap_or(false) || sid.is_some_and(|s| story_is_overset(app, s));
                        inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                    }
                }

                if en_story_id.is_none()
                    && let Some(fid) = right_fid
                    && let Some(active) = app.session.active()
                {
                    en_story_id = active.doc.item(designcraft_doc::ItemId(fid)).and_then(|it| it.text_frame()).map(|tf| tf.story.0);
                }

                let mut page_loop_count = 0usize;
                while (id_over || en_over) && page_loop_count < 50 {
                    page_loop_count = page_loop_count.saturating_add(1);
                    let _ = app.run("layout.pages.insert", json!({"count": 1}));
                    let new_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);

                    let _ = app.session.edit(|doc, _| {
                        if let Some((si, pi)) = doc.page_loc(new_page_idx)
                            && let Some(sp) = doc.spreads.get_mut(si)
                        {
                            let sp = std::sync::Arc::make_mut(sp);
                            if let Some(pg) = sp.pages.get_mut(pi) {
                                pg.margins = doc_margins;
                                pg.local_margins = true;
                            }
                            sp.relayout();
                        }
                        Ok(serde_json::Value::Null)
                    });

                    let (new_margin, new_spread_idx) = {
                        if let Some(active) = app.session.active()
                            && let Some((si, pi)) = active.doc.page_loc(new_page_idx)
                            && let Some(spread) = active.doc.spreads.get(si)
                            && let Some(page) = spread.pages.get(pi)
                        {
                            (page.margin_rect(), si)
                        } else {
                            (margin, spread_idx)
                        }
                    };

                    let new_total_w = (new_margin.x1 - new_margin.x0).max(40.0);
                    let new_col_w = ((new_total_w - gutter) / 2.0).max(10.0);
                    let new_left_rect = [new_margin.x0, new_margin.y0, new_margin.x0 + new_col_w, new_margin.y1];
                    let new_right_rect = [new_margin.x0 + new_col_w + gutter, new_margin.y0, new_margin.x1, new_margin.y1];

                    if id_over && let Some(prev_fid) = cur_left_fid {
                        let res = app.run(
                            "text.thread",
                            json!({
                                "from": prev_fid,
                                "spread": new_spread_idx,
                                "rect": new_left_rect,
                                "autoflow": false,
                            }),
                        );
                        if let Err(e) = &res {
                            eprintln!("pubgen ai: EN body place failed: {e}");
                        }
                        if let Ok(val) = res {
                            cur_left_fid = val.get("to").and_then(Value::as_u64);
                            let sid = val.get("story").and_then(Value::as_u64).map(designcraft_doc::StoryId);
                            id_over = val.get("overset").and_then(Value::as_bool).unwrap_or(false) || sid.is_some_and(|s| story_is_overset(app, s));
                        } else {
                            id_over = false;
                        }
                    } else {
                        id_over = false;
                    }

                    if en_over && let Some(prev_fid) = cur_right_fid {
                        let res = app.run(
                            "text.thread",
                            json!({
                                "from": prev_fid,
                                "spread": new_spread_idx,
                                "rect": new_right_rect,
                                "autoflow": false,
                            }),
                        );
                        if let Err(e) = &res {
                            eprintln!("pubgen ai: EN body place failed: {e}");
                        }
                        if let Ok(val) = res {
                            cur_right_fid = val.get("to").and_then(Value::as_u64);
                            let sid = val.get("story").and_then(Value::as_u64).map(designcraft_doc::StoryId);
                            en_over = val.get("overset").and_then(Value::as_bool).unwrap_or(false) || sid.is_some_and(|s| story_is_overset(app, s));
                        } else {
                            en_over = false;
                        }
                    } else {
                        en_over = false;
                    }

                    generated_count = generated_count.saturating_add(1);
                }

                if let Some(sid) = en_story_id {
                    italicize_story(app, sid);
                }
            } else {
                let header_h = if has_chapter_prefix { 90.0 } else { 44.0 };

                if has_chapter_prefix {
                    // 1st Textbox: "BAB 01" (font Arial Black, size 24.0) -> first row
                    if !ch_line.is_empty() {
                        frame_with_font_and_style(
                            app,
                            spread_idx,
                            [margin.x0, margin.y0, margin.x1, margin.y0 + 36.0],
                            &ch_line,
                            "Arial Black",
                            24.0,
                            None,
                        );
                    }
                    // 2nd Textbox: "Pemerintahan" (font Arial Black, size 20.0) -> second row
                    // Tag with the narrative style so the paragraph table of contents lists this
                    // chapter (and only the narrative pages).
                    if !title_line.is_empty() {
                        frame_with_font_and_style(
                            app,
                            spread_idx,
                            [margin.x0, margin.y0 + 40.0, margin.x1, margin.y0 + 80.0],
                            &title_line,
                            "Arial Black",
                            20.0,
                            Some(TOC_NARRATIVE_STYLE),
                        );
                    }
                } else if !clean_id_title.is_empty() {
                    // Front matter: the title alone, centered across the page margins.
                    if let Some(sid) = frame_with_font_and_style(
                        app,
                        spread_idx,
                        [margin.x0, margin.y0, margin.x1, margin.y0 + 40.0],
                        &clean_id_title.to_uppercase(),
                        "Arial",
                        12.0,
                        Some(TOC_NARRATIVE_STYLE),
                    ) {
                        let _ = app.run("text.select", json!({"story": sid, "anchor": 0, "focus": clean_id_title.chars().count() as u64}));
                        let _ = app.run("type.alignCenter", json!({}));
                    }
                }

                let (mw, mh) = (margin.x1 - margin.x0, margin.y1 - margin.y0);
                // Images of a front-matter sheet: `image1` (official photo) wraps tight around the text (Word "Tight")
                // placed before RTF placement; `image2` (signature) is anchored dynamically to the signatory block.
                if !has_chapter_prefix {
                    let img1_rect = [margin.x0, margin.y0 + mh * 0.22, margin.x0 + mw * 0.24, margin.y0 + mh * 0.43];
                    if place_catalog_image(app, st, row, "image1", spread_idx, img1_rect, "boundingBox").is_some() {
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }
                }

                // Place RTF narrative body into a frame below the headers
                let mut placed_rtf_fid: Option<u64> = None;
                if let Some(file_str) = Some(file_val).filter(|s| !s.is_empty()) {
                    let resolved_rtf = resolve_catalog_related_file(&st.card1_path, file_str, "rtf");

                    if let Some(rtf_file) = resolved_rtf {
                        let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                        crate::canvas::go_to_page(app, cur_page_idx);
                        let rtf_y0 = margin.y0 + header_h;
                        let rtf_rect = [margin.x0, rtf_y0, margin.x1, margin.y1];
                        let created_fid = app
                            .run(
                                "frame.create",
                                json!({
                                    "spread": spread_idx,
                                    "rect": rtf_rect,
                                    "content": "text",
                                    "caret": false
                                }),
                            )
                            .ok()
                            .and_then(|res| res.get("id").and_then(Value::as_u64));

                        if let Some(fid) = created_fid {
                            let _ = app.run("selection.set", json!({"ids": [fid]}));
                            let mut st_copy = st.clone();
                            let rtf_path_str = rtf_file.to_string_lossy().to_string();
                            if execute_page_from_rtf(app, &mut st_copy, &rtf_path_str).is_ok() {
                                inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                                placed_rtf_fid = Some(fid);
                                if !has_chapter_prefix {
                                    harmonize_signatory_block(app, fid, margin);
                                }
                            }
                        }
                    }
                }

                // Dynamically anchor image2 (signature) around the signatory name line in the RTF
                if !has_chapter_prefix {
                    let default_img2_rect = [margin.x1 - mw * 0.244, margin.y0 + mh * 0.354, margin.x1 - mw * 0.044, margin.y0 + mh * 0.464];
                    let (target_spread, img2_rect) = placed_rtf_fid
                        .map(|fid| compute_signature_placement(app, fid, default_img2_rect, spread_idx, margin))
                        .unwrap_or((spread_idx, default_img2_rect));
                    if place_catalog_image(app, st, row, "image2", target_spread, img2_rect, "none").is_some() {
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }
                }

                // Sequential EN spread for front-matter Ulasan in procedural mode
                if st.bilingual_output
                    && !file_en_val.is_empty()
                    && !has_chapter_prefix
                    && let Some(en_rtf_file) = resolve_catalog_related_file(&st.card1_path, file_en_val, "rtf")
                {
                    let en_title_line = if !clean_en_title.is_empty() {
                        clean_en_title.to_string()
                    } else {
                        st.translations.get(title_val.trim()).map(String::as_str).filter(|s| !s.is_empty()).unwrap_or(clean_id_title).to_string()
                    };

                    let _ = app.run("layout.pages.insert", json!({"count": 1}));
                    let en_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                    let _ = app.session.edit(|doc, _| {
                        if let Some((si, pi)) = doc.page_loc(en_page_idx)
                            && let Some(sp) = doc.spreads.get_mut(si)
                        {
                            let sp = std::sync::Arc::make_mut(sp);
                            if let Some(pg) = sp.pages.get_mut(pi) {
                                pg.margins = doc_margins;
                                pg.local_margins = true;
                            }
                            sp.relayout();
                        }
                        Ok(serde_json::Value::Null)
                    });

                    let (en_margin, en_spread_idx) = {
                        if let Some(active) = app.session.active()
                            && let Some((si, pi)) = active.doc.page_loc(en_page_idx)
                            && let Some(spread) = active.doc.spreads.get(si)
                            && let Some(page) = spread.pages.get(pi)
                        {
                            (page.margin_rect(), si)
                        } else {
                            (margin, spread_idx)
                        }
                    };

                    let en_header_h = 44.0;
                    if !en_title_line.is_empty()
                        && let Some(sid) = frame_with_font_and_style(
                            app,
                            en_spread_idx,
                            [en_margin.x0, en_margin.y0, en_margin.x1, en_margin.y0 + 40.0],
                            &en_title_line.to_uppercase(),
                            "Arial",
                            12.0,
                            Some(&st.toc_style),
                        )
                    {
                        let _ = app.run("text.select", json!({"story": sid, "anchor": 0, "focus": en_title_line.chars().count() as u64}));
                        let _ = app.run("type.alignCenter", json!({}));
                    }

                    let (mw, mh) = (en_margin.x1 - en_margin.x0, en_margin.y1 - en_margin.y0);
                    // Place image1 before English RTF text so textWrap: "boundingBox" wraps body text around it.
                    let img1_rect = [en_margin.x0, en_margin.y0 + mh * 0.22, en_margin.x0 + mw * 0.24, en_margin.y0 + mh * 0.43];
                    if place_catalog_image(app, st, row, "image1", en_spread_idx, img1_rect, "boundingBox").is_some() {
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }

                    crate::canvas::go_to_page(app, en_page_idx);
                    let rtf_y0 = en_margin.y0 + en_header_h;
                    let rtf_rect = [en_margin.x0, rtf_y0, en_margin.x1, en_margin.y1];
                    let created_fid = app
                        .run(
                            "frame.create",
                            json!({
                                "spread": en_spread_idx,
                                "rect": rtf_rect,
                                "content": "text",
                                "caret": false
                            }),
                        )
                        .ok()
                        .and_then(|res| res.get("id").and_then(Value::as_u64));

                    let mut placed_en_rtf_fid: Option<u64> = None;
                    if let Some(fid) = created_fid {
                        let _ = app.run("selection.set", json!({"ids": [fid]}));
                        let mut st_copy = st.clone();
                        let rtf_path_str = en_rtf_file.to_string_lossy().to_string();
                        if execute_page_from_rtf(app, &mut st_copy, &rtf_path_str).is_ok() {
                            inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                            placed_en_rtf_fid = Some(fid);
                            harmonize_signatory_block(app, fid, en_margin);
                        }
                    }
                    let end_page = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(en_page_idx);
                    italicize_page_stories(app, en_page_idx, end_page);

                    // Dynamically anchor image2 (signature) around the signatory name line in the English Preface
                    let default_img2_rect =
                        [en_margin.x1 - mw * 0.244, en_margin.y0 + mh * 0.354, en_margin.x1 - mw * 0.044, en_margin.y0 + mh * 0.464];
                    let (target_spread, img2_rect) = placed_en_rtf_fid
                        .map(|fid| compute_signature_placement(app, fid, default_img2_rect, en_spread_idx, en_margin))
                        .unwrap_or((en_spread_idx, default_img2_rect));
                    if place_catalog_image(app, st, row, "image2", target_spread, img2_rect, "none").is_some() {
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }

                    generated_count = generated_count.saturating_add(1);
                }
            }
        } else if is_div {
            let (first_word, digit_num, rest_text) = excel::split_divider_title(title_val);
            let final_num = if !digit_num.is_empty() { digit_num } else { num_val.to_string() };
            let prefix = if first_word.is_empty() || first_word.chars().all(|c| c.is_ascii_digit()) {
                "BAB".to_string()
            } else if !rest_text.is_empty() {
                first_word.clone()
            } else {
                "BAB".to_string()
            };
            let mut chapter_title = if !rest_text.is_empty() {
                rest_text
            } else if !first_word.eq_ignore_ascii_case("bab") && !first_word.chars().all(|c| c.is_ascii_digit()) {
                first_word
            } else {
                title_val.to_string()
            };

            let title_en = if st.bilingual_output {
                if !title_en_val.is_empty() {
                    title_en_val.to_string()
                } else if let Some((id_t, en_t)) = locale::split_pair(&chapter_title) {
                    let (id_s, en_s) = (id_t.to_string(), en_t.to_string());
                    chapter_title = id_s;
                    en_s
                } else {
                    String::new()
                }
            } else {
                if let Some((id_t, en_t)) = locale::split_pair(&chapter_title) {
                    let (id_s, en_s) = (id_t.to_string(), en_t.to_string());
                    chapter_title = match st.locale {
                        locale::Locale::Id => id_s,
                        locale::Locale::En => en_s,
                    };
                }
                String::new()
            };

            // 1. DIV background / hero image adjusts to full Page Size with fitContentToFrame
            if let Some(file_str) = Some(file_val).filter(|s| !s.is_empty()) {
                let resolved_img = resolve_image_file(&st.card1_path, file_str, &["png", "jpg", "jpeg"]);

                if let Some(img_file) = resolved_img {
                    let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                    crate::canvas::go_to_page(app, cur_page_idx);
                    let img_rect = [page_bounds.x0, page_bounds.y0, page_bounds.x1, page_bounds.y1];
                    let created_fid = app
                        .run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": img_rect,
                                "content": "graphic",
                                "caret": false
                            }),
                        )
                        .ok()
                        .and_then(|res| res.get("id").and_then(Value::as_u64));

                    if let Some(fid) = created_fid {
                        let img_path_str = img_file.to_string_lossy().to_string();
                        let _ = app.run("file.place", json!({"path": img_path_str, "frame": fid}));
                        let _ = app.run("object.fit", json!({"mode": "fitContentToFrame", "ids": [fid]}));
                        let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [fid]}));
                        let _ = app.run("object.fittingOptions", json!({"ids": [fid], "fitting": "fitContentToFrame"}));
                        let _ = app.run("selection.set", json!({"ids": [fid]}));
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }
                }
            }

            // 2. Textboxes drawn on top of divider page
            if !prefix.is_empty() {
                frame_with_font_and_style(
                    app,
                    spread_idx,
                    [page_bounds.x0 + 36.0, page_bounds.y0 + 36.0, page_bounds.x1 - 100.0, page_bounds.y0 + 72.0],
                    &prefix,
                    "Arial Black",
                    24.0,
                    None,
                );
            }

            if !final_num.is_empty() {
                frame_with_font_and_style(
                    app,
                    spread_idx,
                    [page_bounds.x1 - 90.0, page_bounds.y0 + 36.0, page_bounds.x1 - 36.0, page_bounds.y0 + 96.0],
                    &final_num,
                    "Arial Black",
                    40.0,
                    None,
                );
            }

            if !chapter_title.is_empty() {
                frame_with_font_and_style(
                    app,
                    spread_idx,
                    [page_bounds.x0 + 36.0, page_bounds.y0 + 78.0, page_bounds.x1 - 100.0, page_bounds.y0 + 118.0],
                    &chapter_title,
                    "Arial Black",
                    20.0,
                    Some(&st.toc_style),
                );
            }

            if st.bilingual_output && !title_en.is_empty() {
                frame_with_font_and_style(
                    app,
                    spread_idx,
                    [page_bounds.x0 + 36.0, page_bounds.y0 + 120.0, page_bounds.x1 - 100.0, page_bounds.y0 + 150.0],
                    &title_en,
                    "Arial",
                    16.0,
                    None,
                );
            }
        } else if is_chart_type(type_val) {
            // Chart / Grafik layout:
            let en_title_text = if !title_en_val.is_empty() {
                title_en_val
            } else if clean_en_title != clean_id_title {
                clean_en_title
            } else {
                ""
            };
            let has_dual_title_en = st.bilingual_output && !en_title_text.is_empty() && (!title_val.contains(" / ") || !title_en_val.is_empty());
            let title_h = if has_dual_title_en { 60.0 } else { 48.0 };

            if has_dual_title_en {
                let (ch_line, title_line) = split_chart_title(clean_id_title, num_val, type_val);
                let full_text = if title_line.is_empty() {
                    format!("{ch_line}{}{en_title_text}", designcraft_doc::story::FORCED_LINE_BREAK)
                } else {
                    format!(
                        "{ch_line}{}{title_line}{}{en_title_text}",
                        designcraft_doc::story::FORCED_LINE_BREAK,
                        designcraft_doc::story::FORCED_LINE_BREAK
                    )
                };

                let title_rect = [margin.x0, margin.y0, margin.x1, margin.y0 + title_h];
                if let Some(story) = frame_with_centered_bold_title(app, spread_idx, title_rect, &full_text, Some(&st.toc_style)) {
                    let b_start = full_text.len().saturating_sub(en_title_text.len());
                    let b_end = full_text.len();
                    let _ = app.run(
                        "text.select",
                        json!({
                            "story": story,
                            "anchor": b_start,
                            "focus": b_end,
                        }),
                    );
                    let _ = app.run(
                        "type.char",
                        json!({
                            "fontFamily": "Arial",
                            "fontStyle": "Italic",
                            "size": 9.0,
                        }),
                    );
                }
            } else if st.bilingual_output && title_val.contains(" / ") {
                let parts = locale::split_chart_title_parts(title_val);
                let line1 = format!(
                    "{} / {}{}",
                    parts.id_kicker,
                    parts.en_kicker.as_deref().unwrap_or(""),
                    if parts.number.is_empty() { String::new() } else { format!(" {}", parts.number) }
                );
                let line2 = format!("{} / {}{}", parts.id_subtitle, parts.en_subtitle.as_deref().unwrap_or(""), parts.suffix);
                let full_text = format!("{line1}{}{line2}", designcraft_doc::story::FORCED_LINE_BREAK);
                let title_rect = [margin.x0, margin.y0, margin.x1, margin.y0 + title_h];
                if let Some(story) = frame_with_centered_bold_title(app, spread_idx, title_rect, &full_text, Some(&st.toc_style)) {
                    let chart_start = parts.id_kicker.chars().count() + 3;
                    let chart_end = chart_start + parts.en_kicker.as_deref().unwrap_or("").chars().count();

                    let line2_start = line1.chars().count() + 1;
                    let sub_start = line2_start + parts.id_subtitle.chars().count() + 3;
                    let sub_end = sub_start + parts.en_subtitle.as_deref().unwrap_or("").chars().count();

                    let mut ranges = Vec::new();
                    if chart_start < chart_end {
                        ranges.push((chart_start, chart_end));
                    }
                    if sub_start < sub_end {
                        ranges.push((sub_start, sub_end));
                    }
                    locale::italicize_ranges(app, story, &full_text, &ranges);
                }
            } else {
                let (ch_line, title_line) = split_chart_title(title_val, num_val, type_val);
                let full_text =
                    if title_line.is_empty() { ch_line } else { format!("{ch_line}{}{title_line}", designcraft_doc::story::FORCED_LINE_BREAK) };

                let title_rect = [margin.x0, margin.y0, margin.x1, margin.y0 + title_h];
                frame_with_centered_bold_title(app, spread_idx, title_rect, &full_text, Some(&st.toc_style));
            }

            // 2. Place image from 'file' column inside margins below the title
            if let Some(file_str) = Some(file_val).filter(|s| !s.is_empty()) {
                let resolved_img = resolve_image_file(&st.card1_path, file_str, &["png", "jpg", "jpeg"]);

                if let Some(img_file) = resolved_img {
                    let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                    crate::canvas::go_to_page(app, cur_page_idx);
                    let img_y0 = margin.y0 + title_h + 8.0;
                    let img_rect = [margin.x0, img_y0, margin.x1, margin.y1];
                    let created_fid = app
                        .run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": img_rect,
                                "content": "graphic",
                                "caret": false
                            }),
                        )
                        .ok()
                        .and_then(|res| res.get("id").and_then(Value::as_u64));

                    if let Some(fid) = created_fid {
                        let img_path_str = img_file.to_string_lossy().to_string();
                        let _ = app.run("file.place", json!({"path": img_path_str, "frame": fid}));
                        let _ = app.run("object.fit", json!({"mode": "fitProportionally", "ids": [fid]}));
                        let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [fid]}));
                        let _ = app.run("object.fit", json!({"mode": "fitFrameToContent", "ids": [fid]}));
                        let _ = app.run("selection.set", json!({"ids": [fid]}));
                        inserted_images_count = inserted_images_count.saturating_add(1);
                    }
                }
            }
        } else if type_val.eq_ignore_ascii_case("Note") || type_val.eq_ignore_ascii_case("G") {
            // Publication note (A02) or narrative RTF sheet (G):
            // When title is provided, add centred Arial title box header and place RTF below it.
            // When sheet only had 1 row (file) with no title, don't add Page Header, just add RTF to page.
            let has_header = !title_val.trim().is_empty();
            if has_header {
                let title_rect = [margin.x0, margin.y0, margin.x1, margin.y0 + 36.0];
                // Tag the title with the narrative style so the paragraph table of contents names
                // the page, as the template flow does for its title slot.
                frame_with_centered_arial(app, spread_idx, title_rect, title_val.trim(), Some(TOC_NARRATIVE_STYLE));
            }

            if let Some(file_str) = Some(file_val).filter(|s| !s.is_empty())
                && let Some(rtf_file) = resolve_catalog_related_file(&st.card1_path, file_str, "rtf")
            {
                let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                crate::canvas::go_to_page(app, cur_page_idx);
                let rtf_rect =
                    if has_header { [margin.x0, margin.y0 + 44.0, margin.x1, margin.y1] } else { [margin.x0, margin.y0, margin.x1, margin.y1] };
                let created_fid = app
                    .run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": rtf_rect,
                            "content": "text",
                            "caret": false
                        }),
                    )
                    .ok()
                    .and_then(|res| res.get("id").and_then(Value::as_u64));

                if let Some(fid) = created_fid {
                    let _ = app.run("selection.set", json!({"ids": [fid]}));
                    let mut st_copy = st.clone();
                    let rtf_path_str = rtf_file.to_string_lossy().to_string();
                    if execute_page_from_rtf(app, &mut st_copy, &rtf_path_str).is_ok() {
                        inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                        // No title line on the page, so the page's title is the RTF's own
                        // heading: tag that paragraph for the paragraph table of contents.
                        if !has_header && let Some(sid) = story_of_frame(app, fid) {
                            tag_page_title(app, sid, TOC_NARRATIVE_STYLE);
                        }
                    }
                }
            }
        } else {
            // 3. Compute Frame Geometry per Preset
            match st.card1_preset {
                1 => {
                    // 1 Row · 3 Equal Columns (1/3 width each)
                    let gutter = 12.0;
                    let total_w = (margin.x1 - margin.x0).max(120.0);
                    let avail = (total_w - 2.0 * gutter).max(60.0);
                    let col_w = avail / 3.0;

                    let c1_x0 = margin.x0;
                    let c1_x1 = c1_x0 + col_w;

                    let c2_x0 = c1_x1 + gutter;
                    let c2_x1 = c2_x0 + col_w;

                    let c3_x0 = c2_x1 + gutter;
                    let c3_x1 = margin.x1;

                    let row_y0 = margin.y0;
                    let row_y1 = margin.y0 + 60.0;

                    let is_table = type_val.eq_ignore_ascii_case("tabel")
                        || type_val.eq_ignore_ascii_case("table")
                        || excel::is_table_sheet_name(file_val)
                        || title_val.contains(" / ");

                    let is_bilingual_table = is_table && (st.bilingual_output || title_val.contains(" / ") || !title_en_val.is_empty());

                    if !type_val.is_empty() || is_table {
                        let (formatted_type, italic_second) = if is_bilingual_table {
                            (format!("Tabel{}Table", designcraft_doc::story::FORCED_LINE_BREAK), true)
                        } else if is_table {
                            (if st.locale == locale::Locale::En { "Table".to_string() } else { "Tabel".to_string() }, false)
                        } else {
                            (type_val.to_string(), false)
                        };

                        if let Ok(res) = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [c1_x0, row_y0, c1_x1, row_y1],
                                "content": "text",
                                "text": formatted_type,
                                "caret": false
                            }),
                        ) && italic_second
                            && let Some(story) = res.get("story").and_then(Value::as_u64)
                        {
                            let (b_start_1, b_end_1) = locale::char_to_byte_range(&formatted_type, 0, 5);
                            let _ = app.run("text.select", json!({"story": story, "anchor": b_start_1, "focus": b_end_1}));
                            let _ = app.run("type.char", json!({"attrs": {"fontStyle": "Regular"}}));

                            locale::italicize_ranges_style(app, story, &formatted_type, &[(6, 11)], "Italic");
                        }
                    }

                    let formatted_num = if is_table { excel::format_table_number(num_val, file_val) } else { num_val.to_string() };

                    if !formatted_num.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [c2_x0, row_y0, c2_x1, row_y1],
                                "content": "text",
                                "text": formatted_num,
                                "caret": false
                            }),
                        );
                    }

                    if !title_val.is_empty() {
                        let (id_t, en_t_opt) =
                            excel::split_and_harmonize_table_title(title_val, if title_en_val.is_empty() { None } else { Some(title_en_val) });
                        let en_t_opt = match en_t_opt {
                            Some(en) => Some(en),
                            None if st.bilingual_output => {
                                st.translations.get(id_t.trim()).map(String::as_str).filter(|s| !s.is_empty()).map(str::to_string)
                            }
                            None => None,
                        };

                        if let Some(en_t) = en_t_opt {
                            let full_title = format!("{id_t}{}{en_t}", designcraft_doc::story::FORCED_LINE_BREAK);
                            if let Some(story) = frame_with_style(app, spread_idx, [c3_x0, row_y0, c3_x1, row_y1], &full_title, &st.toc_style) {
                                let (b_start_1, b_end_1) = locale::char_to_byte_range(&full_title, 0, id_t.chars().count());
                                let _ = app.run("text.select", json!({"story": story, "anchor": b_start_1, "focus": b_end_1}));
                                let _ = app.run("type.char", json!({"attrs": {"fontStyle": "Regular"}}));

                                let en_start = id_t.chars().count() + 1;
                                let en_end = en_start + en_t.chars().count();
                                locale::italicize_ranges_style(app, story, &full_title, &[(en_start, en_end)], "Italic");
                            }
                        } else {
                            frame_with_style(app, spread_idx, [c3_x0, row_y0, c3_x1, row_y1], &id_t, &st.toc_style);
                        }
                    }
                }
                2 => {
                    // Editorial Chapter (Stacked)
                    if !type_val.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [margin.x0, margin.y0, margin.x1, margin.y0 + 22.0],
                                "content": "text",
                                "text": type_val.to_uppercase(),
                                "caret": false
                            }),
                        );
                    }
                    if !num_val.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [margin.x0, margin.y0 + 26.0, margin.x1, margin.y0 + 86.0],
                                "content": "text",
                                "text": num_val,
                                "caret": false
                            }),
                        );
                    }
                    if !title_val.is_empty() {
                        frame_with_style(app, spread_idx, [margin.x0, margin.y0 + 92.0, margin.x1, margin.y0 + 200.0], title_val, &st.toc_style);
                    }
                }
                3 => {
                    // Technical Spec Card
                    // Accent banner (graphic rectangle)
                    let _ = app.run(
                        "frame.create",
                        json!({
                            "spread": spread_idx,
                            "rect": [margin.x0, margin.y0, margin.x1, margin.y0 + 6.0],
                            "content": "unassigned"
                        }),
                    );
                    // Numeral box at top right
                    if !num_val.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [margin.x1 - 80.0, margin.y0 + 16.0, margin.x1, margin.y0 + 56.0],
                                "content": "text",
                                "text": num_val,
                                "caret": false
                            }),
                        );
                    }
                    // Kicker
                    if !type_val.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [margin.x0, margin.y0 + 16.0, margin.x1 - 90.0, margin.y0 + 36.0],
                                "content": "text",
                                "text": type_val.to_uppercase(),
                                "caret": false
                            }),
                        );
                    }
                    // Title
                    if !title_val.is_empty() {
                        frame_with_style(
                            app,
                            spread_idx,
                            [margin.x0, margin.y0 + 40.0, margin.x1 - 90.0, margin.y0 + 110.0],
                            title_val,
                            &st.toc_style,
                        );
                    }
                }
                4 => {
                    // Minimal Zine
                    if !num_val.is_empty() {
                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [margin.x0, margin.y0 + 60.0, margin.x1, margin.y0 + 110.0],
                                "content": "text",
                                "text": num_val,
                                "caret": false
                            }),
                        );
                    }
                    if !title_val.is_empty() {
                        frame_with_style(app, spread_idx, [margin.x0, margin.y0 + 120.0, margin.x1, margin.y0 + 200.0], title_val, &st.toc_style);
                    }
                }
                5 => {
                    // TOC Line: one combined paragraph per record (type + number + title), so each record
                    // becomes a single table-of-contents entry.
                    let parts: Vec<&str> = [type_val, num_val, title_val].into_iter().filter(|s| !s.is_empty()).collect();
                    let line = parts.join(" ");
                    if !line.is_empty() {
                        frame_with_style(app, spread_idx, [margin.x0, margin.y0, margin.x1, margin.y0 + 40.0], &line, &st.toc_style);
                    }
                }
                _ => {
                    // 0 or Default: 1 Row · 3 Columns (Col 1 & 2 fit tightly to content, Col 3 takes rest of width with space at end)
                    let gutter = 8.0;
                    let end_space = 20.0;
                    let row_y0 = margin.y0;
                    let row_y1 = margin.y0 + 60.0;
                    let mut cur_x = margin.x0;

                    let is_table = type_val.eq_ignore_ascii_case("tabel")
                        || type_val.eq_ignore_ascii_case("table")
                        || excel::is_table_sheet_name(file_val)
                        || title_val.contains(" / ");

                    let is_bilingual_table = is_table && (st.bilingual_output || title_val.contains(" / ") || !title_en_val.is_empty());

                    // Col 1: Type (Fit tightly to content, starting flush with margin.x0)
                    if !type_val.is_empty() || is_table {
                        let (formatted_type, italic_second) = if is_bilingual_table {
                            (format!("Tabel{}Table", designcraft_doc::story::FORCED_LINE_BREAK), true)
                        } else if is_table {
                            (if st.locale == locale::Locale::En { "Table".to_string() } else { "Tabel".to_string() }, false)
                        } else {
                            (type_val.to_string(), false)
                        };
                        let c1_w = estimate_text_width(if italic_second { "Table" } else { &formatted_type }, false);
                        let c1_x0 = cur_x;
                        let c1_x1 = (c1_x0 + c1_w).min(margin.x1 - 100.0);
                        cur_x = c1_x1 + gutter;

                        if let Ok(res) = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [c1_x0, row_y0, c1_x1, row_y1],
                                "content": "text",
                                "text": formatted_type,
                                "caret": false
                            }),
                        ) {
                            if let Some(id) = res.get("id").and_then(Value::as_u64) {
                                let _ = app.run("selection.set", json!({"ids": [id]}));
                            }
                            let _ = app.run("type.alignRight", json!({}));
                            if italic_second && let Some(story) = res.get("story").and_then(Value::as_u64) {
                                let (b_start_1, b_end_1) = locale::char_to_byte_range(&formatted_type, 0, 5);
                                let _ = app.run("text.select", json!({"story": story, "anchor": b_start_1, "focus": b_end_1}));
                                let _ = app.run("type.char", json!({"attrs": {"fontStyle": "Regular"}}));

                                locale::italicize_ranges_style(app, story, &formatted_type, &[(6, 11)], "Italic");
                            }
                        }
                    }

                    // Col 2: Number (Fit tightly to content)
                    let formatted_num = if is_table { excel::format_table_number(num_val, file_val) } else { num_val.to_string() };

                    if !formatted_num.is_empty() {
                        let c2_w = estimate_text_width(&formatted_num, false);
                        let c2_x0 = cur_x;
                        let c2_x1 = (c2_x0 + c2_w).min(margin.x1 - 60.0);
                        cur_x = c2_x1 + gutter;

                        let _ = app.run(
                            "frame.create",
                            json!({
                                "spread": spread_idx,
                                "rect": [c2_x0, row_y0, c2_x1, row_y1],
                                "content": "text",
                                "text": formatted_num,
                                "caret": false
                            }),
                        );
                    }

                    // Col 3: Title (Takes rest of width with space in the end)
                    if !title_val.is_empty() {
                        let c3_x0 = cur_x;
                        let max_x1 = (margin.x1 - end_space).max(c3_x0 + 40.0);
                        let c3_x1 = max_x1.min(margin.x1);

                        let (id_t, en_t_opt) =
                            excel::split_and_harmonize_table_title(title_val, if title_en_val.is_empty() { None } else { Some(title_en_val) });
                        let en_t_opt = match en_t_opt {
                            Some(en) => Some(en),
                            None if st.bilingual_output => {
                                st.translations.get(id_t.trim()).map(String::as_str).filter(|s| !s.is_empty()).map(str::to_string)
                            }
                            None => None,
                        };

                        if let Some(en_t) = en_t_opt {
                            let full_title = format!("{id_t}{}{en_t}", designcraft_doc::story::FORCED_LINE_BREAK);
                            if let Some(story) = frame_with_style(app, spread_idx, [c3_x0, row_y0, c3_x1, row_y1], &full_title, &st.toc_style) {
                                let (b_start_1, b_end_1) = locale::char_to_byte_range(&full_title, 0, id_t.chars().count());
                                let _ = app.run("text.select", json!({"story": story, "anchor": b_start_1, "focus": b_end_1}));
                                let _ = app.run("type.char", json!({"attrs": {"fontStyle": "Regular"}}));

                                let en_start = id_t.chars().count() + 1;
                                let en_end = en_start + en_t.chars().count();
                                locale::italicize_ranges_style(app, story, &full_title, &[(en_start, en_end)], "Italic");
                            }
                        } else {
                            frame_with_style(app, spread_idx, [c3_x0, row_y0, c3_x1, row_y1], &id_t, &st.toc_style);
                        }
                    }
                }
            }
            // Auto-insert table from 'file' column if enabled
            if st.card1_auto_insert_tables
                && let Some(file_str) = Some(file_val).filter(|s| !s.is_empty())
            {
                let catalog_p = std::path::Path::new(&st.card1_path);
                let dir = catalog_p.parent().unwrap_or(std::path::Path::new("."));
                let candidate1 = dir.join(file_str);
                let candidate2 = dir.join(format!("{file_str}.csv"));
                let candidate3 = dir.join(format!("{}.csv", excel::sanitize_filename(file_str)));
                let resolved = if candidate1.exists() && candidate1.is_file() {
                    Some(candidate1)
                } else if candidate2.exists() && candidate2.is_file() {
                    Some(candidate2)
                } else if candidate3.exists() && candidate3.is_file() {
                    Some(candidate3)
                } else {
                    None
                };

                if let Some(tbl_file) = resolved
                    && let Ok(content) = std::fs::read_to_string(&tbl_file)
                {
                    let delim = detect_delimiter(&content);
                    let tbl_matrix = parse_csv_content_with_delim(&content, delim);
                    if !tbl_matrix.is_empty() {
                        let display_matrix =
                            if st.bilingual_output && st.translate_with_ai { ai::pair_cells(&tbl_matrix, &st.translations) } else { tbl_matrix };
                        let cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                        let mut table_st = st.clone();
                        if !theme_val.is_empty() {
                            table_st.card2_theme_override = Some(theme_val.to_string());
                        }
                        if execute_insert_table_matrix(app, &table_st, Some(&display_matrix), Some(cur_page_idx)).is_ok() {
                            inserted_tables_count = inserted_tables_count.saturating_add(1);
                        }
                    }
                }
            }
        }
        generated_count = generated_count.saturating_add(1);
    }

    let mut details = Vec::new();
    if inserted_rtfs_count > 0 {
        details.push(format!("{inserted_rtfs_count} RTF chapter(s) placed"));
    }
    if inserted_images_count > 0 {
        details.push(format!("{inserted_images_count} chart image(s) placed"));
    }
    if inserted_tables_count > 0 {
        details.push(format!("{inserted_tables_count} tables auto-inserted"));
    }
    let tbl_msg = if !details.is_empty() { format!(" (with {})", details.join(", ")) } else { String::new() };
    ensure_back_cover_at_end(app);
    Ok(format!("Successfully generated {generated_count} pages{tbl_msg}!"))
}
