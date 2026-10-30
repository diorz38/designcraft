//! The page generation engine: one CSV record -> one templated document page.

use super::*;

// ---------------------------------------------------------------------------
// CSV File Ingestion (RFC 4180 parsing with quotes and delimiter sniffing)
// ---------------------------------------------------------------------------

/// Resolve a template path or template ID from a catalog specification.
///
/// Supports:
/// - Direct file path (as-is or with `.designcraft`)
/// - Path relative to the catalog file's directory
/// - Path relative to `assets/templates/` or any of its subfolders (`dividers/`, `covers/`, etc.)
/// - Template ID match from scanned template catalog (e.g. `001_ulasan_chapter` or fuzzy stem)
pub fn resolve_template_path_spec(catalog_path: &str, spec: &str) -> Option<String> {
    let clean = spec.trim().trim_matches('"').trim();
    if clean.is_empty() {
        return None;
    }

    let clean_lower = clean.to_lowercase();
    let alias_opt = match clean_lower.as_str() {
        "dividers/special_infographic_divider"
        | "special_infographic_divider"
        | "dividers/002_chapter_devider"
        | "002_chapter_devider"
        | "chapter_devider"
        | "dividers/002_batik_infographic"
        | "002_batik_infographic"
        | "batik_infographic" => Some("dividers/002_batik_infographic"),
        "dividers/001_ulasan_chapter"
        | "001_ulasan_chapter"
        | "ulasan_chapter"
        | "dividers/001_simple_chapter"
        | "001_simple_chapter"
        | "simple_chapter" => Some("dividers/001_simple_chapter"),
        _ => None,
    };

    // 1. Direct path check as-is or with .designcraft
    let direct_p = std::path::Path::new(clean);
    if direct_p.is_file() {
        return Some(clean.to_string());
    }
    let direct_ext = format!("{clean}.designcraft");
    if std::path::Path::new(&direct_ext).is_file() {
        return Some(direct_ext);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let p = dir.join(clean);
        if p.is_file() {
            return Some(p.to_string_lossy().to_string());
        }
        let p_ext = dir.join(&direct_ext);
        if p_ext.is_file() {
            return Some(p_ext.to_string_lossy().to_string());
        }
    }
    if let Some(alias) = alias_opt {
        let alias_ext = format!("{alias}.designcraft");
        if std::path::Path::new(alias).is_file() {
            return Some(alias.to_string());
        }
        if std::path::Path::new(&alias_ext).is_file() {
            return Some(alias_ext);
        }
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            let p = dir.join(alias);
            if p.is_file() {
                return Some(p.to_string_lossy().to_string());
            }
            let p_ext = dir.join(&alias_ext);
            if p_ext.is_file() {
                return Some(p_ext.to_string_lossy().to_string());
            }
        }
    }

    // 2. Relative to catalog file directory
    if !catalog_path.is_empty() {
        let cat_p = std::path::Path::new(catalog_path);
        let base_dir = if cat_p.is_file() { cat_p.parent() } else { Some(cat_p) };
        if let Some(dir) = base_dir {
            let rel = dir.join(clean);
            if rel.is_file() {
                return Some(rel.to_string_lossy().to_string());
            }
            let rel_ext = dir.join(&direct_ext);
            if rel_ext.is_file() {
                return Some(rel_ext.to_string_lossy().to_string());
            }
            let rel_tpl = dir.join("templates").join(clean);
            if rel_tpl.is_file() {
                return Some(rel_tpl.to_string_lossy().to_string());
            }
            let rel_tpl_ext = dir.join("templates").join(&direct_ext);
            if rel_tpl_ext.is_file() {
                return Some(rel_tpl_ext.to_string_lossy().to_string());
            }
        }
    }

    // 3. Search in standard assets/templates root
    let mut root_candidates = vec![
        std::path::PathBuf::from("assets/templates"),
        std::path::PathBuf::from("../../assets/templates"),
        std::path::PathBuf::from("../assets/templates"),
    ];
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        root_candidates.push(exe_dir.join("assets").join("templates"));
        root_candidates.push(exe_dir.join("templates"));
    }
    if let Some(root) = root_candidates.into_iter().find(|p| p.is_dir()) {
        // Direct under assets/templates
        let p1 = root.join(clean);
        if p1.is_file() {
            return Some(p1.to_string_lossy().to_string());
        }
        let p1_ext = root.join(&direct_ext);
        if p1_ext.is_file() {
            return Some(p1_ext.to_string_lossy().to_string());
        }

        // Try under subcategories (dividers, covers, front_matter, data_tables, infographics, articles)
        let subcats = ["dividers", "data_tables", "front_matter", "covers", "infographics", "articles"];
        for sub in subcats {
            let sub_p = root.join(sub).join(clean);
            if sub_p.is_file() {
                return Some(sub_p.to_string_lossy().to_string());
            }
            let sub_p_ext = root.join(sub).join(&direct_ext);
            if sub_p_ext.is_file() {
                return Some(sub_p_ext.to_string_lossy().to_string());
            }
        }

        // Match against template catalogue IDs (e.g. "001_ulasan_chapter" or fuzzy matching stem)
        let scanned = designcraft_engine::cmd::template::scan_template_catalog(&root);
        let clean_lower = clean.to_lowercase();
        // Exact ID match
        if let Some(found) = scanned.iter().find(|t| t.id.to_lowercase() == clean_lower) {
            return Some(found.path.to_string_lossy().to_string());
        }
        if let Some(alias) = alias_opt {
            let alias_stem = alias.strip_prefix("dividers/").unwrap_or(alias);
            if let Some(found) = scanned.iter().find(|t| t.id.to_lowercase() == alias_stem) {
                return Some(found.path.to_string_lossy().to_string());
            }
        }
        // If numeric 1-3 digits (e.g. "002" or "2"), check for divider templates with matching prefix
        if clean.len() <= 3 && clean.chars().all(|c| c.is_ascii_digit()) {
            let padded = format!("{:03}_", clean.parse::<u32>().unwrap_or(0));
            if let Some(found) = scanned.iter().find(|t| t.category == "dividers" && t.id.starts_with(&padded)) {
                return Some(found.path.to_string_lossy().to_string());
            }
            if let Some(found) = scanned.iter().find(|t| t.id.starts_with(&padded)) {
                return Some(found.path.to_string_lossy().to_string());
            }
        }
        // Substring / suffix / prefix match on ID
        if let Some(found) = scanned.iter().find(|t| {
            let id_l = t.id.to_lowercase();
            id_l.contains(&clean_lower) || clean_lower.contains(&id_l)
        }) {
            return Some(found.path.to_string_lossy().to_string());
        }
    }

    None
}

/// Post-process an applied template page: margins, text styles, and narrative styling.
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
    let mut title_slot_ids = Vec::new();
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
                            if item.label.starts_with("#slot:text:table_title") {
                                title_slot_ids.push(it_id);
                            }
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

        // A wrapped table title must grow its frame (heightOnly) instead of spilling
        // over the table slot below it.
        for id in title_slot_ids {
            let _ = app.run("object.textFrameOptions", json!({"autoSize": "heightOnly", "autoSizeReference": "topLeft", "ids": [id]}));
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
    } else if type_val.eq_ignore_ascii_case("Image") || type_val.eq_ignore_ascii_case("Graphic") {
        let note_title = title_val.trim();
        text.insert("title".into(), json!(note_title));
        if let Some(file_str) = file_opt
            && let Some(img_file) = resolve_image_file(&st.card1_path, file_str, &["png", "jpg", "jpeg"])
        {
            image.insert("image".into(), json!(img_file.to_string_lossy()));
            image.insert("photo".into(), json!(img_file.to_string_lossy()));
            images_resolved = 1;
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
            let prof = st.active_profile();
            text.insert("table_num".into(), json!(format!("{} {}", locale::table_prefix(Some(&prof), st.locale), formatted_num)));
            text.insert("table_kind_en".into(), json!(""));
            text.insert("table_title".into(), json!(table_title_val));
            text.insert("table_title_en".into(), json!(""));
            text.insert("title".into(), json!(table_title_val));
            text.insert("number".into(), json!(formatted_num));
        }
    }

    RowSlots { text, story, image, images_resolved, rtfs_resolved }
}

pub(crate) fn ensure_brs_heading_styles(app: &mut DesignApp, font: &str) {
    let _ = app.run("list.define", json!({"name": "BRS", "continueAcrossStories": true}));

    let h1_params = json!({
        "name": "Heading 1",
        "chars": {
            "fontFamily": font,
            "fontStyle": "Bold",
            "size": 13.0
        },
        "para": {
            "listType": "numbers",
            "listName": "BRS",
            "listLevel": 1,
            "numberExpression": "^#.^t",
            "restartNumbers": true,
            "restartAfterLevel": 0,
            "spaceBefore": 12.0,
            "spaceAfter": 4.0
        }
    });
    if app.run("style.paragraph.edit", h1_params.clone()).is_err() {
        let _ = app.run("style.paragraph.create", h1_params);
    }

    let h2_params = json!({
        "name": "Heading 2",
        "chars": {
            "fontFamily": font,
            "fontStyle": "Bold",
            "size": 11.0
        },
        "para": {
            "listType": "numbers",
            "listName": "BRS",
            "listLevel": 2,
            "numberExpression": "^1.^#.^t",
            "restartNumbers": true,
            "restartAfterLevel": 0,
            "spaceBefore": 8.0,
            "spaceAfter": 3.0
        }
    });
    if app.run("style.paragraph.edit", h2_params.clone()).is_err() {
        let _ = app.run("style.paragraph.create", h2_params);
    }

    let title_params = json!({
        "name": "Title",
        "chars": {
            "fontFamily": font,
            "fontStyle": "Bold",
            "size": 14.5
        },
        "para": {
            "spaceBefore": 0.0,
            "spaceAfter": 6.0
        }
    });
    if app.run("style.paragraph.edit", title_params.clone()).is_err() {
        let _ = app.run("style.paragraph.create", title_params);
    }
}

pub(crate) fn format_brs_narrative_story(app: &mut DesignApp, fid: u64, font: &str) {
    let Some(active) = app.session.active() else { return };
    let Some(it) = active.doc.item(designcraft_doc::ItemId(fid)) else { return };
    let Some(tf) = it.text_frame() else { return };
    let sid = tf.story;
    let Some(story) = active.doc.story(sid) else { return };

    let story_len = story.text.len();
    if story_len == 0 {
        return;
    }

    // Default font Lato
    let _ = app.run("text.select", json!({"story": sid.0, "anchor": 0, "focus": story_len}));
    let _ = app.run("type.char", json!({"fontFamily": font, "size": 10.0}));

    // Inspect paragraphs and determine styling without holding borrow during app.run
    #[derive(Clone, Copy)]
    enum BHeading {
        Title,
        H1,
        H2,
    }
    let mut styling_targets: Vec<(usize, usize, BHeading)> = Vec::new();

    if let Some(active) = app.session.active()
        && let Some(story) = active.doc.story(sid)
    {
        let para_ranges = story.para_ranges();
        for (i, p) in story.paras.iter().enumerate() {
            let Some(r) = para_ranges.get(i) else { continue };
            let p_text = story.text.get(r.clone()).unwrap_or("").trim();
            if p_text.is_empty() {
                continue;
            }

            let is_short_heading = p_text.len() <= 60 && !p_text.ends_with('.');

            let is_title = p.style.eq_ignore_ascii_case("Title")
                || (is_short_heading && (p_text.starts_with("A. ") || p_text.starts_with("B. ") || p_text.starts_with("C. ")));

            let is_h1 = p.style.eq_ignore_ascii_case("Heading 1")
                || (is_short_heading
                    && (p_text == "Nilai Tukar Petani (NTP)"
                        || p_text.starts_with("Indeks Harga yang Diterima")
                        || p_text.starts_with("NTP Menurut Subsektor")
                        || p_text.starts_with("Perbandingan NTP")
                        || p_text.starts_with("Nilai Tukar Usaha Petani")));

            let is_h2 = p.style.eq_ignore_ascii_case("Heading 2")
                || (is_short_heading
                    && (p_text.starts_with("Indeks Harga yang Dibayar")
                        || p_text.starts_with("NTP Tanaman Pangan")
                        || p_text.starts_with("NTP Tanaman Hortikultura")
                        || p_text.starts_with("NTP Tanaman Perkebunan")
                        || p_text.starts_with("NTP Peternakan")
                        || p_text.starts_with("NTP Perikanan")));

            if is_title {
                styling_targets.push((r.start, r.end, BHeading::Title));
            } else if is_h1 {
                styling_targets.push((r.start, r.end, BHeading::H1));
            } else if is_h2 {
                styling_targets.push((r.start, r.end, BHeading::H2));
            }
        }
    }

    for (start, end, h_kind) in styling_targets {
        let _ = app.run("text.select", json!({"story": sid.0, "anchor": start, "focus": end}));
        match h_kind {
            BHeading::Title => {
                let _ = app.run("style.paragraph.apply", json!({"name": "Title"}));
                let _ = app.run("type.char", json!({"fontFamily": font, "fontStyle": "Bold", "size": 14.5}));
            }
            BHeading::H1 => {
                let _ = app.run("style.paragraph.apply", json!({"name": "Heading 1"}));
                let _ = app.run("type.char", json!({"fontFamily": font, "fontStyle": "Bold", "size": 13.0}));
            }
            BHeading::H2 => {
                let _ = app.run("style.paragraph.apply", json!({"name": "Heading 2"}));
                let _ = app.run("type.char", json!({"fontFamily": font, "fontStyle": "Bold", "size": 11.0}));
            }
        }
    }
}

fn set_brs_page_margins(app: &mut DesignApp, page_idx: usize, margins: designcraft_doc::Margins) {
    let _ = app.session.edit(|doc, _| {
        if let Some((si, pi)) = doc.page_loc(page_idx) {
            if let Some(sp) = doc.spreads.get_mut(si) {
                let sp = std::sync::Arc::make_mut(sp);
                if let Some(pg) = sp.pages.get_mut(pi) {
                    pg.margins = margins;
                    pg.local_margins = true;
                }
                sp.relayout();
            }
        }
        Ok(Value::Null)
    });
}

fn get_brs_page_metrics(app: &DesignApp, page_idx: usize) -> Result<(designcraft_geom::Rect, designcraft_geom::Rect, usize), String> {
    let active = app.session.active().ok_or("No active document")?;
    let (si, pi) = active.doc.page_loc(page_idx).ok_or_else(|| format!("Page {page_idx} not found"))?;
    let sp = active.doc.spreads.get(si).ok_or("Spread not found")?;
    let pg = sp.pages.get(pi).ok_or("Page not found")?;

    let page_bounds = pg.bounds();

    let margin = designcraft_geom::Rect {
        x0: page_bounds.x0 + pg.margins.inside.max(36.0),
        y0: page_bounds.y0 + pg.margins.top.max(36.0),
        x1: page_bounds.x1 - pg.margins.outside.max(36.0),
        y1: page_bounds.y1 - pg.margins.bottom.max(36.0),
    };

    Ok((margin, page_bounds, si))
}

fn get_page_items_bottom(app: &DesignApp, page_idx: usize) -> Option<f64> {
    let active = app.session.active()?;
    let (si, pi) = active.doc.page_loc(page_idx)?;
    let sp = active.doc.spreads.get(si)?;
    let pg = sp.pages.get(pi)?;
    let pg_bounds = pg.bounds();

    let mut max_y = pg_bounds.y0;
    let mut found = false;
    for it in &sp.items {
        let b = it.bounds();
        if b.x0 >= pg_bounds.x0 && b.x1 <= pg_bounds.x1 + 10.0 {
            if b.y1 > max_y {
                max_y = b.y1;
                found = true;
            }
        }
    }
    if found { Some(max_y) } else { None }
}

fn shrink_frame_height(app: &mut DesignApp, fid: u64, x0: f64, y0: f64, x1: f64, y1: f64) {
    let _ = app.session.edit(|doc, _| {
        if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
            it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect { x0, y0, x1, y1 });
        }
        Ok(Value::Null)
    });
}

fn measure_frame_content_height(app: &DesignApp, fid: u64) -> Option<f64> {
    let active = app.session.active()?;
    let it = active.doc.item(designcraft_doc::ItemId(fid))?;
    let tf = it.text_frame()?;
    let cs = designcraft_compose::compose_story(&active.doc, tf.story, &Default::default());
    let cf = cs.frames.iter().find(|f| f.frame == designcraft_doc::ItemId(fid))?;
    Some(cf.content_height)
}

fn execute_generate_pages_brs(
    app: &mut DesignApp,
    st: &PubgenState,
    slice: &[Vec<String>],
    doc_margins: designcraft_doc::Margins,
) -> Result<String, String> {
    let brs_font = st.body_font();
    ensure_brs_heading_styles(app, &brs_font);

    let mut generated_count = 0usize;
    let mut inserted_tables_count = 0usize;
    let mut inserted_rtfs_count = 0usize;
    let mut inserted_images_count = 0usize;
    let mut vector_charts_count = 0usize;

    let (has_back_cover, active_pc) = {
        let active = app.session.active();
        let pc = active.as_ref().map(|s| s.doc.page_count()).unwrap_or(0);
        let bc = active.as_ref().and_then(|s| super::covers::find_back_cover_page(&s.doc));
        (bc, pc)
    };

    let mut cur_page_idx = if let Some(bc_idx) = has_back_cover {
        app.run("layout.pages.insert", json!({"before": bc_idx, "count": 1}))?;
        bc_idx
    } else if active_pc > 0 && st.excel_had_cover {
        app.run("layout.pages.insert", json!({"after": active_pc.saturating_sub(1), "count": 1}))?;
        active_pc
    } else if active_pc > 0 {
        active_pc.saturating_sub(1)
    } else {
        app.run("layout.pages.insert", json!({"count": 1}))?;
        0
    };

    set_brs_page_margins(app, cur_page_idx, doc_margins);

    let (mut margin, _, mut spread_idx) = get_brs_page_metrics(app, cur_page_idx)?;
    let mut cur_y = margin.y0;
    let avail_h = (margin.y1 - margin.y0).max(100.0);
    let half_page_h = (avail_h * 0.48).clamp(280.0, 370.0);
    let gap = 12.0;

    let mut last_narrative_fid: Option<u64> = None;

    let file_col = effective_col(st.card1_file_col, &st.card1_headers, &["file", "filename"], 3);
    let theme_col = effective_col(999, &st.card1_headers, &["theme", "themes", "table_theme", "color", "warna"], 999);

    for row in slice {
        let type_val = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let num_val = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let title_val = row.get(st.card1_title_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let file_val = row.get(file_col).map(String::as_str).unwrap_or("").trim();
        let theme_val = row.get(theme_col).map(String::as_str).unwrap_or("").trim();

        let is_chart = is_chart_type(type_val) || type_val.eq_ignore_ascii_case("line") || title_val.to_lowercase().starts_with("gambar");
        let is_table = type_val.eq_ignore_ascii_case("tabel") || type_val.eq_ignore_ascii_case("table") || excel::is_table_sheet_name(file_val);
        let is_image = type_val.eq_ignore_ascii_case("image")
            || type_val.eq_ignore_ascii_case("gambar")
            || file_val.to_lowercase().ends_with(".png")
            || file_val.to_lowercase().ends_with(".jpg");
        let is_narrative = !is_chart && !is_table && !is_image;

        if is_chart {
            // Check if chart fits in remaining page height
            if cur_y + half_page_h > margin.y1 {
                // Advance to next page
                app.run("layout.pages.insert", json!({"after": cur_page_idx, "count": 1}))?;
                cur_page_idx += 1;
                set_brs_page_margins(app, cur_page_idx, doc_margins);
                let metrics = get_brs_page_metrics(app, cur_page_idx)?;
                margin = metrics.0;
                spread_idx = metrics.2;
                cur_y = margin.y0;
            }

            let chart_margin = designcraft_geom::Rect { x0: margin.x0, y0: cur_y, x1: margin.x1, y1: cur_y + half_page_h };

            let mut rendered = false;
            let candidate = resolve_catalog_related_file(&st.card1_path, file_val, "csv");
            if let Some(csv_path) = candidate
                && let Ok(content) = std::fs::read_to_string(&csv_path)
            {
                let delim = detect_delimiter(&content);
                let grid = parse_csv_content_with_delim(&content, delim);
                let sheet_data = excel::ExcelSheetData { name: num_val.to_string(), grid, merges: Vec::new() };
                if let Some(mut chart_spec) = super::charts::spec::parse_chart_spec(&sheet_data) {
                    chart_spec.include_data_table = true;
                    if let Ok((cats, vals)) = super::charts::spec::extract_chart_data(&sheet_data, &chart_spec) {
                        if let Ok(layout) = super::charts::layout::compute_layout(&chart_spec, &cats, &vals, chart_margin, st.bilingual_output) {
                            let palette = if !theme_val.is_empty() { vec![theme_val.to_string()] } else { Vec::new() };
                            let _ =
                                super::charts::emitter::emit_chart(app, spread_idx, &chart_spec, &layout, &palette, st.bilingual_output, &brs_font);
                            rendered = true;
                            vector_charts_count = vector_charts_count.saturating_add(1);
                        }
                    }
                }
            }

            if !rendered {
                let chart_h = 240.0;
                let r = [margin.x0, cur_y, margin.x1, cur_y + chart_h];
                let label = if !title_val.is_empty() { title_val } else { "Grafik" };
                frame_with_font_and_style(app, spread_idx, r, label, &brs_font, 11.0, None);
            }

            let chart_bottom = get_page_items_bottom(app, cur_page_idx).unwrap_or(cur_y + half_page_h);
            cur_y = (chart_bottom + gap).min(margin.y1);
            generated_count = generated_count.saturating_add(1);
        } else if is_table {
            let mut matrix: Vec<Vec<String>> = Vec::new();
            let table_file = resolve_catalog_related_file(&st.card1_path, file_val, "csv");
            if let Some(ref tf) = table_file
                && let Ok(content) = std::fs::read_to_string(tf)
            {
                let delim = detect_delimiter(&content);
                matrix = parse_csv_content_with_delim(&content, delim);
            }

            let est_h = (matrix.len() as f64 * 18.0 + 50.0).clamp(100.0, avail_h);
            if cur_y + est_h > margin.y1 || (margin.y1 - cur_y < 120.0) {
                // Jump to next page
                app.run("layout.pages.insert", json!({"after": cur_page_idx, "count": 1}))?;
                cur_page_idx += 1;
                set_brs_page_margins(app, cur_page_idx, doc_margins);
                let metrics = get_brs_page_metrics(app, cur_page_idx)?;
                margin = metrics.0;
                spread_idx = metrics.2;
                cur_y = margin.y0;
            }

            // Draw table title if present
            if !title_val.is_empty() {
                let full_title = if !num_val.is_empty()
                    && !title_val.to_lowercase().starts_with("tabel")
                    && !title_val.to_lowercase().starts_with("table")
                {
                    let pfx = locale::table_prefix(Some(&st.active_profile()), st.locale);
                    format!("{pfx} {num_val}    {title_val}")
                } else {
                    title_val.to_string()
                };
                let title_h = 16.0;
                let title_rect = [margin.x0, cur_y, margin.x1, cur_y + title_h];
                if let Some(sid) = frame_with_font_full(app, spread_idx, title_rect, &full_title, &brs_font, Some("Bold"), 10.5, None) {
                    let theme_hex = if !theme_val.is_empty() {
                        parse_table_color_theme(theme_val).map(|t| t.header_footer)
                    } else if let Some(override_th) = &st.card2_theme_override {
                        parse_table_color_theme(override_th).map(|t| t.header_footer)
                    } else {
                        None
                    };
                    if let Some(hex) = theme_hex.filter(|h| !h.is_empty()) {
                        let swatch_name = app.session.edit(|doc, _| {
                            Ok(serde_json::Value::String(ensure_swatch(doc, &hex)))
                        }).ok().and_then(|v| v.as_str().map(|s| s.to_string()));
                        if let Some(swatch) = swatch_name {
                            let text_len = full_title.len() as u64;
                            let _ = app.run("text.select", json!({"story": sid, "anchor": 0, "focus": text_len}));
                            let _ = app.run("type.char", json!({"fill": swatch}));
                        }
                    }
                    if let Some(fid) = app.session.active().and_then(|a| a.doc.story(designcraft_doc::StoryId(sid)).and_then(|s| s.frames.first().copied())) {
                        let _ = app.run("object.textFrameOptions", json!({"autoSize": "heightOnly", "autoSizeReference": "topLeft", "ids": [fid.0]}));
                        let _ = app.run("object.fit", json!({"mode": "fitFrameToContent", "axis": "height", "ids": [fid.0]}));
                    }
                }
            }

            // Insert table
            if !matrix.is_empty() {
                let mut table_st = st.clone();
                table_st.card2_table_footer = false;
                table_st.bilingual_output = false;
                table_st.card2_col_numbers = true;
                if !theme_val.is_empty() {
                    table_st.card2_theme_override = Some(theme_val.to_string());
                }
                if let Ok(info) = execute_insert_table_matrix_info(app, &table_st, Some(&matrix), Some(cur_page_idx)) {
                    let added = thread_overset_table(app, &mut cur_page_idx, info.story_id, info.frame_id, doc_margins);
                    if added > 0 {
                        let metrics = get_brs_page_metrics(app, cur_page_idx)?;
                        margin = metrics.0;
                        spread_idx = metrics.2;
                    }
                    inserted_tables_count = inserted_tables_count.saturating_add(1);
                }
            }

            let tbl_bottom = get_page_items_bottom(app, cur_page_idx).unwrap_or(cur_y + est_h);
            cur_y = (tbl_bottom + gap).min(margin.y1);
            generated_count = generated_count.saturating_add(1);
        } else if is_narrative {
            let rtf_file = resolve_catalog_related_file(&st.card1_path, file_val, "rtf");
            if margin.y1 - cur_y < 120.0 {
                // Jump to next page
                app.run("layout.pages.insert", json!({"after": cur_page_idx, "count": 1}))?;
                cur_page_idx += 1;
                set_brs_page_margins(app, cur_page_idx, doc_margins);
                let metrics = get_brs_page_metrics(app, cur_page_idx)?;
                margin = metrics.0;
                spread_idx = metrics.2;
                cur_y = margin.y0;
            }

            let target_rect = [margin.x0, cur_y, margin.x1, margin.y1];
            if let Ok(res) = app.run(
                "frame.create",
                json!({
                    "spread": spread_idx,
                    "rect": target_rect,
                    "content": "text",
                    "caret": false
                }),
            ) && let Some(fid) = res.get("id").and_then(Value::as_u64)
            {
                if let Some(ref rf) = rtf_file {
                    let _ = app.run(
                        "file.place",
                        json!({
                            "path": rf.to_string_lossy().to_string(),
                            "frame": fid,
                            "autoflow": false
                        }),
                    );
                    inserted_rtfs_count = inserted_rtfs_count.saturating_add(1);
                }

                // Thread with previous narrative frame if present (appends this frame's text to the continuous story)
                if let Some(prev_fid) = last_narrative_fid {
                    let _ = app.session.edit(|doc, _| Ok(doc.thread(designcraft_doc::ItemId(prev_fid), designcraft_doc::ItemId(fid))?));
                }

                format_brs_narrative_story(app, fid, &brs_font);

                let fit_h = measure_frame_content_height(app, fid).unwrap_or(180.0);
                let clamped_h = (fit_h + 4.0).min(margin.y1 - cur_y);
                shrink_frame_height(app, fid, margin.x0, cur_y, margin.x1, cur_y + clamped_h);

                cur_y += clamped_h + gap;
                last_narrative_fid = Some(fid);
                generated_count = generated_count.saturating_add(1);
            }
        } else if is_image {
            if margin.y1 - cur_y < 160.0 {
                // Jump to next page
                app.run("layout.pages.insert", json!({"after": cur_page_idx, "count": 1}))?;
                cur_page_idx += 1;
                set_brs_page_margins(app, cur_page_idx, doc_margins);
                let metrics = get_brs_page_metrics(app, cur_page_idx)?;
                margin = metrics.0;
                spread_idx = metrics.2;
                cur_y = margin.y0;
            }

            if let Some(file_str) = Some(file_val).filter(|s| !s.is_empty())
                && let Some(img_file) = resolve_image_file(&st.card1_path, file_str, &["png", "jpg", "jpeg"])
            {
                let img_rect = [margin.x0, cur_y, margin.x1, margin.y1];
                if let Ok(res) = app.run(
                    "frame.create",
                    json!({
                        "spread": spread_idx,
                        "rect": img_rect,
                        "content": "graphic",
                        "caret": false
                    }),
                ) && let Some(fid) = res.get("id").and_then(Value::as_u64)
                {
                    let _ = app.run("file.place", json!({"path": img_file.to_string_lossy().to_string(), "frame": fid}));
                    let _ = app.run("object.fit", json!({"mode": "fitProportionally", "ids": [fid]}));
                    let _ = app.run("object.fit", json!({"mode": "centerContent", "ids": [fid]}));
                    inserted_images_count = inserted_images_count.saturating_add(1);
                }
            }

            cur_y = margin.y1;
            generated_count = generated_count.saturating_add(1);
        }
    }

    super::covers::ensure_back_cover_at_end(app);

    Ok(format!(
        "Generated {generated_count} BRS objects ({inserted_tables_count} tables, {vector_charts_count} charts, {inserted_rtfs_count} narratives, {inserted_images_count} images)"
    ))
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
    let mut vector_charts_count = 0usize;

    // Every preset tags its title (or the combined TOC line) with the TOC paragraph style, so the
    // Quick Actions "Add TOC" lists one entry per generated page. The narrative pages (the G and
    // Ulasan sheets) carry the narrative style instead, so the paragraph list names them alone.
    ensure_para_style(app, &st.toc_style);
    ensure_para_style(app, TOC_NARRATIVE_STYLE);

    let doc_margins = {
        let active = app.session.active().ok_or("No active document")?;
        detect_document_margins(&active.doc)
    };

    if st.active_profile().id == "bps_brs" {
        return execute_generate_pages_brs(app, st, slice, doc_margins);
    }

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
                            let display_matrix = if st.bilingual_output && st.translate_with_ai {
                                ai::pair_cells(&tbl_matrix, &st.translations, &st.glossary)
                            } else {
                                tbl_matrix
                            };
                            let mut cur_page_idx = applied_info
                                .get("page")
                                .and_then(Value::as_u64)
                                .map(|p| p as usize)
                                .unwrap_or_else(|| app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0));
                            let mut table_st = st.clone();
                            if !theme_val.is_empty() {
                                table_st.card2_theme_override = Some(theme_val.to_string());
                            }
                            if let Ok(info) = execute_insert_table_matrix_info(app, &table_st, Some(&display_matrix), Some(cur_page_idx)) {
                                thread_overset_table(app, &mut cur_page_idx, info.story_id, info.frame_id, doc_margins);
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
            // Chart / Grafik / Map layout:
            let dir = std::path::Path::new(&st.card1_path).parent().unwrap_or(std::path::Path::new("."));
            let clean_num = excel::sanitize_filename(num_val);
            let mut detected_map_title = None;
            let mut detected_map_title_en = None;

            let c_candidates = [
                if file_val.ends_with(".csv") { dir.join(file_val) } else { std::path::PathBuf::new() },
                dir.join(format!("{file_val}.csv")),
                dir.join(format!("{clean_num}.csv")),
                dir.join(format!("C{clean_num}.csv")),
                dir.join(format!("c{clean_num}.csv")),
                dir.join(format!("C{}.csv", clean_num.replace('.', "_"))),
                dir.join(format!("c{}.csv", clean_num.replace('.', "_"))),
                dir.join(format!("M{clean_num}.csv")),
                dir.join(format!("m{clean_num}.csv")),
                dir.join(format!("M{}.csv", clean_num.replace('.', "_"))),
                dir.join(format!("m{}.csv", clean_num.replace('.', "_"))),
            ];

            // Peek at candidates to discover title and title_en from map CSV if needed
            for candidate in &c_candidates {
                if candidate.as_os_str().is_empty() || !candidate.is_file() {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(candidate) {
                    let delim = detect_delimiter(&content);
                    let grid = parse_csv_content_with_delim(&content, delim);
                    let sheet_data = excel::ExcelSheetData { name: num_val.to_string(), grid, merges: Vec::new() };
                    if let Some(map_info) = excel::parse_map_sheet(&sheet_data) {
                        let data_rel = map_info.data_csv_path.replace('\\', "/");
                        let data_candidate = resolve_catalog_related_file(&st.card1_path, &data_rel, "csv")
                            .or_else(|| {
                                let p = dir.join(&data_rel);
                                if p.is_file() { Some(p) } else { None }
                            })
                            .or_else(|| {
                                let p = std::path::PathBuf::from(&data_rel);
                                if p.is_file() { Some(p) } else { None }
                            });
                        if let Some(csv_p) = data_candidate {
                            if let Ok(csv_text) = std::fs::read_to_string(&csv_p) {
                                if let Ok(parsed_data) = super::charts::spec::parse_map_data_csv(&csv_text) {
                                    if !parsed_data.title.is_empty() {
                                        detected_map_title = Some(parsed_data.title);
                                    }
                                    if !parsed_data.title_en.is_empty() {
                                        detected_map_title_en = Some(parsed_data.title_en);
                                    }
                                    break;
                                }
                            }
                        }
                    } else if let Ok(parsed_data) = super::charts::spec::parse_map_data_csv(&content) {
                        if !parsed_data.title.is_empty() {
                            detected_map_title = Some(parsed_data.title);
                        }
                        if !parsed_data.title_en.is_empty() {
                            detected_map_title_en = Some(parsed_data.title_en);
                        }
                        break;
                    }
                }
            }

            let eff_id_title = detected_map_title.as_deref().unwrap_or(clean_id_title);
            let eff_title_en_val = if let Some(ref dte) = detected_map_title_en {
                dte.as_str()
            } else if !title_en_val.is_empty() {
                title_en_val
            } else if clean_en_title != clean_id_title {
                clean_en_title
            } else {
                ""
            };

            let has_dual_title_en =
                !eff_title_en_val.is_empty() && (!title_val.contains(" / ") || !title_en_val.is_empty() || detected_map_title_en.is_some());
            let title_h = if has_dual_title_en { 60.0 } else { 48.0 };

            if has_dual_title_en {
                let (ch_line, title_line) = split_chart_title(eff_id_title, num_val, type_val);
                let full_text = if title_line.is_empty() {
                    format!("{ch_line}{}{eff_title_en_val}", designcraft_doc::story::FORCED_LINE_BREAK)
                } else {
                    format!(
                        "{ch_line}{}{title_line}{}{eff_title_en_val}",
                        designcraft_doc::story::FORCED_LINE_BREAK,
                        designcraft_doc::story::FORCED_LINE_BREAK
                    )
                };

                let title_rect = [margin.x0, margin.y0, margin.x1, margin.y0 + title_h];
                if let Some(story) = frame_with_centered_bold_title(app, spread_idx, title_rect, &full_text, Some(&st.toc_style)) {
                    let b_start = full_text.len().saturating_sub(eff_title_en_val.len());
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

            // 2. Vector chart / map engine first, with raster image fallback
            let mut vector_rendered = false;
            for candidate in &c_candidates {
                if candidate.as_os_str().is_empty() || !candidate.exists() || !candidate.is_file() {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(candidate) {
                    let delim = detect_delimiter(&content);
                    let grid = parse_csv_content_with_delim(&content, delim);
                    let sheet_data = excel::ExcelSheetData { name: num_val.to_string(), grid, merges: Vec::new() };

                    // 2a. Map Chart (MXX_XX sheet with map and data)
                    if let Some(map_info) = excel::parse_map_sheet(&sheet_data) {
                        let data_rel = map_info.data_csv_path.replace('\\', "/");
                        let data_candidate = resolve_catalog_related_file(&st.card1_path, &data_rel, "csv")
                            .or_else(|| {
                                let p = dir.join(&data_rel);
                                if p.is_file() { Some(p) } else { None }
                            })
                            .or_else(|| {
                                let p = std::path::PathBuf::from(&data_rel);
                                if p.is_file() { Some(p) } else { None }
                            });
                        if let Some(csv_p) = data_candidate {
                            if let Ok(csv_text) = std::fs::read_to_string(&csv_p) {
                                if let Ok(parsed_data) = super::charts::spec::parse_map_data_csv(&csv_text) {
                                    let pal_idx = map_info
                                        .theme
                                        .as_deref()
                                        .or(if !theme_val.is_empty() { Some(theme_val) } else { None })
                                        .map(super::charts::palette::find_map_palette_idx)
                                        .unwrap_or(0);
                                    let map_spec = super::charts::spec::ChartSpec {
                                        chart_type: super::charts::spec::ChartType::Map,
                                        title_id: if !parsed_data.title.is_empty() { parsed_data.title.clone() } else { eff_id_title.to_string() },
                                        title_en: if !parsed_data.title_en.is_empty() {
                                            parsed_data.title_en.clone()
                                        } else {
                                            eff_title_en_val.to_string()
                                        },
                                        category_col: "A".into(),
                                        series: vec![super::charts::spec::SeriesSpec {
                                            col: "B".into(),
                                            label_id: "Nilai".into(),
                                            label_en: "Value".into(),
                                        }],
                                        data_start_row: 1,
                                        data_end_row: parsed_data.categories.len(),
                                        value_format: super::charts::spec::ValueFormat::Number,
                                        show_value_labels: true,
                                        show_legend: true,
                                        show_grid: false,
                                        max_value_override: None,
                                        geojson_path: Some(map_info.map_name_or_path.clone()),
                                        map_palette_idx: pal_idx,
                                        include_data_table: false,
                                    };
                                    // `compute_layout` reserves the title band itself, so the chart
                                    // margin starts at the page margin (no extra title offset).
                                    let chart_margin = designcraft_geom::Rect { x0: margin.x0, y0: margin.y0, x1: margin.x1, y1: margin.y1 };
                                    let layout = match super::charts::layout::compute_layout(
                                        &map_spec,
                                        &parsed_data.categories,
                                        &[parsed_data.values],
                                        chart_margin,
                                        st.bilingual_output,
                                    ) {
                                        Ok(layout) => layout,
                                        Err(e) => {
                                            log::warn!("Vector map chart layout failed: {e}; falling back to raster");
                                            continue;
                                        }
                                    };
                                    let palette = if let Some(th) = &map_info.theme {
                                        vec![th.clone()]
                                    } else if !theme_val.is_empty() {
                                        vec![theme_val.to_string()]
                                    } else if let Some(ref th) = st.card2_theme_override {
                                        vec![th.clone()]
                                    } else if !st.cover_color_primary.is_empty() {
                                        vec![st.cover_color_primary.clone(), st.cover_color_accent.clone()]
                                    } else {
                                        Vec::new()
                                    };
                                    match super::charts::emitter::emit_chart(
                                        app,
                                        spread_idx,
                                        &map_spec,
                                        &layout,
                                        &palette,
                                        st.bilingual_output,
                                        &st.body_font(),
                                    ) {
                                        Ok(_n) => {
                                            vector_charts_count = vector_charts_count.saturating_add(1);
                                            vector_rendered = true;
                                            break;
                                        }
                                        Err(e) => {
                                            log::warn!("Vector map chart emit failed: {e}; falling back to raster");
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 2b. Standard CXX_XX chart spec
                    if let Some(spec) = super::charts::spec::parse_chart_spec(&sheet_data) {
                        match super::charts::spec::extract_chart_data(&sheet_data, &spec) {
                            Ok((categories, series_values)) => {
                                let chart_margin = designcraft_geom::Rect { x0: margin.x0, y0: margin.y0, x1: margin.x1, y1: margin.y1 };
                                let layout = match super::charts::layout::compute_layout(
                                    &spec,
                                    &categories,
                                    &series_values,
                                    chart_margin,
                                    st.bilingual_output,
                                ) {
                                    Ok(layout) => layout,
                                    Err(e) => {
                                        log::warn!("Vector chart layout failed: {e}; falling back to raster");
                                        continue;
                                    }
                                };
                                let palette = if !theme_val.is_empty() {
                                    vec![theme_val.to_string()]
                                } else if let Some(ref th) = st.card2_theme_override {
                                    vec![th.clone()]
                                } else if !st.cover_color_primary.is_empty() {
                                    vec![st.cover_color_primary.clone(), st.cover_color_accent.clone()]
                                } else {
                                    Vec::new()
                                };
                                match super::charts::emitter::emit_chart(
                                    app,
                                    spread_idx,
                                    &spec,
                                    &layout,
                                    &palette,
                                    st.bilingual_output,
                                    &st.body_font(),
                                ) {
                                    Ok(_n) => {
                                        vector_charts_count = vector_charts_count.saturating_add(1);
                                        vector_rendered = true;
                                        break;
                                    }
                                    Err(e) => {
                                        log::warn!("Vector chart emit failed: {e}; falling back to raster");
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("Vector chart data extraction failed: {e}; falling back to raster");
                            }
                        }
                    }
                }
            }

            if !vector_rendered && let Some(file_str) = Some(file_val).filter(|s| !s.is_empty()) {
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
                        let display_matrix = if st.bilingual_output && st.translate_with_ai {
                            ai::pair_cells(&tbl_matrix, &st.translations, &st.glossary)
                        } else {
                            tbl_matrix
                        };
                        let mut cur_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
                        let mut table_st = st.clone();
                        if !theme_val.is_empty() {
                            table_st.card2_theme_override = Some(theme_val.to_string());
                        }
                        if let Ok(info) = execute_insert_table_matrix_info(app, &table_st, Some(&display_matrix), Some(cur_page_idx)) {
                            thread_overset_table(app, &mut cur_page_idx, info.story_id, info.frame_id, doc_margins);
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
    if vector_charts_count > 0 {
        details.push(format!("{vector_charts_count} vector chart(s) rendered"));
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

#[cfg(test)]
pub(crate) mod tests {
    use super::testutil::*;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_execute_generate_pages() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let initial_pages = app.session.active().unwrap().doc.page_count();

        let st = PubgenState {
            card1_records: vec![
                vec!["Chapter".into(), "1".into(), "Getting Started".into()],
                vec!["Chapter".into(), "2".into(), "Advanced Layouts".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_range_from: 1,
            card1_range_to: 2,
            card1_preset: 0,
            card1_append: true,
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "generate_pages should succeed: {:?}", res);

        let final_pages = app.session.active().unwrap().doc.page_count();
        assert_eq!(final_pages, initial_pages + 2);

        // Verify the 1-row 3-column frames on the newly created spread
        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.last().unwrap();
        assert!(spread.items.len() >= 3, "Expected at least 3 frames on generated spread");
        let items = &spread.items;
        let n = items.len();
        let f1 = &items[n - 3];
        let f2 = &items[n - 2];
        let f3 = &items[n - 1];
        // Same row: y0 and y1 are identical
        assert_eq!(f1.bounds().y0, f2.bounds().y0);
        assert_eq!(f2.bounds().y0, f3.bounds().y0);
        assert_eq!(f1.bounds().y1, f2.bounds().y1);
        // 3 columns: side-by-side from left to right (Col 1: Type < Col 2: Number < Col 3: Title)
        assert!(f1.bounds().x1 < f2.bounds().x0);
        assert!(f2.bounds().x1 < f3.bounds().x0);
        // Col 3 leaves space at the end before margin.x1
        let page_margin = spread.pages.last().unwrap().margin_rect();
        assert!(f3.bounds().x1 < page_margin.x1);

        // Col 1 is right-aligned
        let tf1 = f1.text_frame().expect("f1 should be a text frame");
        let story1 = active.doc.story(tf1.story).expect("story1 should exist");
        assert_eq!(story1.paras[0].para.align, Some(designcraft_doc::Align::Right));
    }

    #[test]
    fn test_execute_generate_pages_separate_textboxes_title_only() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec![
                "Tabel".into(),
                "02.01".into(),
                "Jumlah Rukun Warga (RW) dan Rukun Tetangga (RT) Menurut Desa/Kelurahan di Kecamatan Karawang Timur, 2025".into(),
            ]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        // Check the 3 text frames created on the generated spread
        {
            let active = app.session.active().unwrap();
            let spread = active.doc.spreads.last().unwrap();
            let items = &spread.items;
            let n = items.len();
            assert!(n >= 3);
            let f1 = items[n - 3].text_frame().unwrap();
            let f2 = items[n - 2].text_frame().unwrap();
            let f3 = items[n - 1].text_frame().unwrap();

            let s1 = active.doc.story(f1.story).unwrap();
            let s2 = active.doc.story(f2.story).unwrap();
            let s3 = active.doc.story(f3.story).unwrap();

            // 1st textbox contains the localized "Tabel"
            assert_eq!(s1.text.trim(), "Tabel");
            // 2nd textbox contains: "02.01" (correct value)
            assert_eq!(s2.text.trim(), "02.01");
            // 3rd textbox contains ONLY title, NOT including type and number!
            assert_eq!(s3.text.trim(), "Jumlah Rukun Warga (RW) dan Rukun Tetangga (RT) Menurut Desa/Kelurahan di Kecamatan Karawang Timur, 2025");
        }

        // When Add TOC is executed, verify the generated TOC entry merges type + number + title:
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc_story = active.doc.story(active.doc.tocs.last().unwrap().story).unwrap();
        let clean_text = toc_story.text.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
        assert!(
            clean_text
                .contains("Tabel 02.01 Jumlah Rukun Warga (RW) dan Rukun Tetangga (RT) Menurut Desa/Kelurahan di Kecamatan Karawang Timur, 2025"),
            "TOC should contain merged type + number + title: {}",
            toc_story.text
        );
    }

    #[test]
    fn test_execute_generate_pages_auto_inserts_tables_from_file_col() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = std::env::temp_dir().join("pubgen_auto_insert_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let cat_path = temp_dir.join("test_catalog.csv");
        let cat_content = "type,number,title,file\n\
                           Tabel,01.01,First Table,tbl_1\n\
                           Tabel,01.02,Second Table,tbl_2\n";
        std::fs::write(&cat_path, cat_content).unwrap();

        let tbl1_path = temp_dir.join("tbl_1.csv");
        let tbl1_content = "Col A,Col B\n10,20\n30,40\n";
        std::fs::write(&tbl1_path, tbl1_content).unwrap();

        let tbl2_path = temp_dir.join("tbl_2.csv");
        let tbl2_content = "Header X,Header Y\nfoo,bar\nbaz,qux\n";
        std::fs::write(&tbl2_path, tbl2_content).unwrap();

        let mut st = PubgenState::default();
        load_card1_csv(&mut st, cat_path.to_str().unwrap());
        st.card1_auto_insert_tables = true;

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("2 pages"), "Message should mention 2 pages: {msg}");
        assert!(msg.contains("2 tables auto-inserted"), "Message should mention 2 tables auto-inserted: {msg}");

        // Cleanup
        let _ = std::fs::remove_file(cat_path);
        let _ = std::fs::remove_file(tbl1_path);
        let _ = std::fs::remove_file(tbl2_path);
        let _ = std::fs::remove_dir(temp_dir);
    }

    #[test]
    fn test_execute_generate_pages_g_sheet_without_title_names_its_rtf_title() {
        // The `GXX` sheets carry no title row, only the RTF link, so the page opens with the
        // RTF's own title line. Word writes that line as direct formatting (bold, caps, centred),
        // not as a `Heading 1`, so the paragraph table of contents has to read the page's heading
        // off how it looks or the page is missing from the list.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G02.rtf");
        let rtf = concat!(
            r"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}{\stylesheet{\s0 Normal;}}",
            r"\pard\plain\s0\qc\b\caps\fs30 DAFTAR SINGKATAN/list of Abbreviation\par",
            r"\pard\plain\s0\fs24 SI : Stasiun Iklim/Climate Station\par}"
        );
        std::fs::write(&rtf_path, rtf.as_bytes()).unwrap();
        let catalog_path = temp_dir.path().join("G02_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nG,02,,\"{escaped}\"\n")).unwrap();
        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        st.card1_preset = 0;
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        execute_generate_pages(&mut app, &st).expect("generate pages");

        let active = app.session.active().unwrap();
        let body = active.doc.stories.values().find(|s| s.text.contains("DAFTAR SINGKATAN")).expect("placed RTF");
        assert_eq!(body.paras[0].style, TOC_NARRATIVE_STYLE, "the RTF's title line is the page's title");
        assert_eq!(body.paras[1].style, designcraft_doc::story::BASIC_PARAGRAPH, "the body keeps its own style");
        assert_eq!(body.paras[0].para.align, Some(designcraft_doc::Align::Center), "the title keeps the RTF's centring");

        let plan = TocPlan::from_state(&PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() });
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(rows, [(1, "DAFTAR SINGKATAN/list of Abbreviation".to_string())], "the paragraph list names the page at its own level: {rows:?}");
    }

    #[test]
    fn test_execute_generate_pages_g_sheet_without_title_in_template_mode() {
        // The template flow fills its title slot with the sheet's title: empty for these sheets,
        // so the RTF's own first line is the page's title there too.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G01.rtf");
        let rtf = concat!(
            r"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}{\stylesheet{\s0 Normal;}}",
            r"\pard\plain\s0\qc\b\caps\fs30 Penjelasan Umum/Explanatory Notes\par",
            r"\pard\plain\s0\fs24 The publication covers the regency.\par}"
        );
        std::fs::write(&rtf_path, rtf.as_bytes()).unwrap();
        let catalog_path = temp_dir.path().join("G01_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nG,01,,\"{escaped}\"\n")).unwrap();

        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        st.card1_preset = 6;
        st.card1_template_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/templates/front_matter/001_publication_note.designcraft").into();
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        execute_generate_pages(&mut app, &st).expect("generate pages");

        let active = app.session.active().unwrap();
        let body = active.doc.stories.values().find(|s| s.text.contains("Penjelasan Umum")).expect("placed RTF");
        assert_eq!(body.paras[0].style, TOC_NARRATIVE_STYLE, "the RTF's title line is the page's title");

        let plan = TocPlan::from_state(&PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() });
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(rows, [(1, "Penjelasan Umum/Explanatory Notes".to_string())], "the paragraph list names the page: {rows:?}");
    }

    #[test]
    fn test_g_sheet_title_skips_the_issn_line_and_the_book_title() {
        // The compilers' page (a `G` sheet that links only its RTF): a right-aligned ISSN line, a
        // few empty lines, then the page's own title in the largest centred type, with the
        // publication's title set smaller below it. Naming the page after its first paragraph
        // listed the ISSN line; naming it after its largest centred line lists the page.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G00.rtf");
        let rtf = concat!(
            r"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}{\stylesheet{\s0 Normal;}}",
            r"\pard\plain\s0\qr\fs24 ISSN: 2828-7346\par",
            r"\pard\plain\s0\par",
            r"\pard\plain\s0\par",
            r"\pard\plain\s0\par",
            r"\pard\plain\s0\qc\b\fs28 TIM PENYUSUN/COMPILERS\par",
            r"\pard\plain\s0\qc\b\fs22 Kecamatan Karawang Timur Dalam Angka 2025\par",
            r"\pard\plain\s0\qc\fs22 Karawang Timur District in Figures 2025\par}",
        );
        std::fs::write(&rtf_path, rtf.as_bytes()).unwrap();
        let catalog_path = temp_dir.path().join("G00_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nG,00,,\"{escaped}\"\n")).unwrap();
        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        st.card1_preset = 0;
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        execute_generate_pages(&mut app, &st).expect("generate pages");

        let active = app.session.active().unwrap();
        let body = active.doc.stories.values().find(|s| s.text.contains("TIM PENYUSUN")).expect("placed RTF");
        let style_of = |needle: &str| {
            let i = body.para_ranges().iter().position(|r| body.text.get(r.clone()).is_some_and(|t| t.contains(needle))).expect("paragraph");
            body.paras[i].style.clone()
        };
        assert_eq!(style_of("TIM PENYUSUN"), TOC_NARRATIVE_STYLE, "the page's title names the page");
        assert_eq!(style_of("ISSN"), designcraft_doc::story::BASIC_PARAGRAPH, "the ISSN line stays out of the list");
        assert_eq!(style_of("Kecamatan Karawang"), designcraft_doc::story::BASIC_PARAGRAPH, "so does the publication's title");

        let plan = TocPlan::from_state(&PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() });
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(rows, [(1, "TIM PENYUSUN/COMPILERS".to_string())], "the paragraph list names the page once, at its own level: {rows:?}");
    }

    #[test]
    fn test_g_sheet_title_skips_an_empty_opening_paragraph() {
        // The explanatory-notes sheet (G01) opens with an empty paragraph and carries its title
        // next: searching for the first non-empty paragraph's look finds the title where taking
        // the very first paragraph found nothing, so the page was missing from the list.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G01.rtf");
        let rtf = concat!(
            r"{\rtf1\ansi\deff0{\fonttbl{\f0 Arial;}}{\stylesheet{\s0 Normal;}}",
            r"\pard\plain\s0\par",
            r"\pard\plain\s0\qc\b\fs30 Penjelasan Umum/Explanatory Notes\par",
            r"\pard\plain\s0\ql\fs24 The publication covers the regency and its subdistricts.\par}",
        );
        std::fs::write(&rtf_path, rtf.as_bytes()).unwrap();
        let catalog_path = temp_dir.path().join("G01_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nG,01,,\"{escaped}\"\n")).unwrap();
        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        st.card1_preset = 0;
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        execute_generate_pages(&mut app, &st).expect("generate pages");

        let active = app.session.active().unwrap();
        let body = active.doc.stories.values().find(|s| s.text.contains("Penjelasan Umum")).expect("placed RTF");
        let ranges = body.para_ranges();
        let title = ranges.iter().position(|r| body.text.get(r.clone()).is_some_and(|t| t.contains("Penjelasan Umum"))).expect("title paragraph");
        assert_eq!(body.paras[title].style, TOC_NARRATIVE_STYLE, "the RTF's title line is the page's title");
        assert_eq!(body.paras[0].para.align, None, "the empty opening paragraph keeps its own look");

        let plan = TocPlan::from_state(&PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() });
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(rows, [(1, "Penjelasan Umum/Explanatory Notes".to_string())], "the paragraph list names the page: {rows:?}");
    }

    #[test]
    fn test_execute_generate_pages_note_sheet_title_and_rtf() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("A02.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Hello DesignCraft Note!}"#).unwrap();
        let catalog_path = temp_dir.path().join("A02_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nNote,02,Keterangan Publikasi,\"{escaped}\"\n")).unwrap();

        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "generate pages should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert!(active.doc.page_count() >= 2, "the note gets its own page");

        // The title is a centred Arial text box with a 100 % transparent background.
        let title_story =
            active.doc.stories.values().find(|s| s.text.trim() == "Keterangan Publikasi").unwrap_or_else(|| panic!("note title story should exist"));
        assert_eq!(title_story.paras[0].para.align, Some(designcraft_doc::Align::Center));
        assert!(title_story.chars.iter().all(|run| run.format.over.font_family.as_deref() == Some("Arial")), "title runs should all be Arial");
        let title_item = active
            .doc
            .spreads
            .iter()
            .flat_map(|sp| &sp.items)
            .find(|it| it.text_frame().is_some_and(|tf| tf.story == title_story.id))
            .unwrap_or_else(|| panic!("title frame should exist"));
        assert_eq!(title_item.fill.swatch, "[None]", "title box has no fill");
        assert!(title_item.stroke.is_none(), "title box has no stroke");

        // The RTF note is placed on the same page below the title.
        assert!(active.doc.stories.values().any(|s| s.text.contains("Hello DesignCraft Note!")), "RTF text should be placed on the generated page");
    }

    #[test]
    fn test_kata_pengantar_dynamic_signature_placement() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let margin = designcraft_geom::Rect::new(36.0, 36.0, 559.0, 806.0);
        let (mw, mh) = (margin.x1 - margin.x0, margin.y1 - margin.y0);
        let default_img2_rect = [margin.x1 - mw * 0.244, margin.y0 + mh * 0.354, margin.x1 - mw * 0.044, margin.y0 + mh * 0.464];

        for sample in ["B01.rtf", "B01_en.rtf"] {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({})).unwrap();

            // Place image1 with boundingBox wrap before placing RTF, matching execute_generate_pages
            let img1_rect = [margin.x0, margin.y0 + mh * 0.22, margin.x0 + mw * 0.24, margin.y0 + mh * 0.43];
            let res_img = app.run("frame.create", json!({"spread": 0, "rect": img1_rect, "content": "graphic"})).unwrap();
            let img_fid = res_img.get("id").and_then(Value::as_u64).unwrap();
            let _ = app.run("object.textWrap", json!({"mode": "boundingBox", "offset": 6.0, "ids": [img_fid]}));

            let rtf_rect = [margin.x0, margin.y0 + 50.0, margin.x1, margin.y1];
            let res = app.run("frame.create", json!({"spread": 0, "rect": rtf_rect, "content": "text", "caret": false})).unwrap();
            let fid = res.get("id").and_then(Value::as_u64).unwrap();
            let _ = app.run("selection.set", json!({"ids": [fid]}));

            let mut st = PubgenState::default();
            let rtf_path = format!("{}/../../plan/pubgen/samples/DUAL_LANG/{}", env!("CARGO_MANIFEST_DIR"), sample);
            if std::path::Path::new(&rtf_path).exists() {
                let place_res = execute_page_from_rtf(&mut app, &mut st, &rtf_path);
                assert!(place_res.is_ok(), "execute_page_from_rtf should place {sample}: {place_res:?}");

                harmonize_signatory_block(&mut app, fid, margin);

                let (target_spread, img2_rect) = compute_signature_placement(&app, fid, default_img2_rect, 0, margin);
                assert_eq!(target_spread, 0);

                // The dynamic signature rect must be anchored down near the signatory block at the bottom
                // of the text, significantly lower than the default static position (~308pt).
                assert!(
                    img2_rect[1] > default_img2_rect[1] + 50.0,
                    "Dynamic signature y0 ({}) should be anchored lower than default ({}) for {sample}",
                    img2_rect[1],
                    default_img2_rect[1]
                );
                assert!(img2_rect[1] >= margin.y0 && img2_rect[3] <= margin.y1);
                assert!(img2_rect[0] >= margin.x0 && img2_rect[2] <= margin.x1 - 20.0, "Signature must not hug right page edge for {sample}");

                // Verify composed lines: all signatory lines must be well away from the right margin
                // and must share consistent horizontal centering.
                let active = app.session.active().unwrap();
                let sid = story_of_frame(&app, fid).unwrap();
                let cs = designcraft_compose::compose_story(&active.doc, sid, &Default::default());
                let non_empty_lines: Vec<&designcraft_compose::Line> = cs
                    .frames
                    .iter()
                    .flat_map(|f| &f.lines)
                    .filter(|l| l.glyphs.iter().any(|g| g.visible && !g.rendered_char.is_whitespace()))
                    .collect();

                assert!(non_empty_lines.len() >= 4, "Too few lines for {sample}: {}", non_empty_lines.len());
                let sig_lines = &non_empty_lines[non_empty_lines.len() - 4..];
                let mut sig_centers = Vec::new();
                for l in sig_lines {
                    let min_x = l.glyphs.iter().filter(|g| g.visible && !g.rendered_char.is_whitespace()).map(|g| g.x).fold(f64::INFINITY, f64::min);
                    let max_x = l
                        .glyphs
                        .iter()
                        .filter(|g| g.visible && !g.rendered_char.is_whitespace())
                        .map(|g| g.x + g.adv)
                        .fold(f64::NEG_INFINITY, f64::max);
                    assert!(
                        max_x <= margin.x1 - 25.0,
                        "Signatory line max_x ({max_x}) must not be too close to margin.x1 ({}) in {sample}",
                        margin.x1
                    );
                    sig_centers.push((min_x + max_x) / 2.0);
                }
                // All signatory lines must share matching center alignment (within 3 points)
                let avg_center: f64 = sig_centers.iter().sum::<f64>() / sig_centers.len() as f64;
                for &c in &sig_centers {
                    assert!((c - avg_center).abs() < 3.0, "Signatory line center ({c}) does not match average center ({avg_center}) in {sample}");
                }
            }
        }
    }

    #[test]
    fn test_execute_generate_pages_g_sheet_tags_its_title_for_the_paragraph_toc() {
        // `GXX` sheet rows (front matter such as the compiler page, the explanatory notes or the
        // abbreviations) keep the narrative style on their title, so a paragraph table of contents
        // names them beside the chapter reviews. The template flow tagged its title slot; the
        // preset flow left it plain.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G00.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Hello DesignCraft G00 Narrative!}"#).unwrap();
        let catalog_path = temp_dir.path().join("G00_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        std::fs::write(&catalog_path, format!("type,number,title,file\nG,00,TIM PENYUSUN/COMPILERS,\"{escaped}\"\n")).unwrap();
        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        st.card1_preset = 0;
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        execute_generate_pages(&mut app, &st).expect("generate pages");

        let active = app.session.active().unwrap();
        let title_story = active.doc.stories.values().find(|s| s.text.contains("TIM PENYUSUN/COMPILERS")).expect("title story");
        assert_eq!(title_story.paras[0].style, TOC_NARRATIVE_STYLE, "the page title carries the narrative style");

        let plan = TocPlan::from_state(&PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() });
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();
        assert_eq!(rows, [(1, "TIM PENYUSUN/COMPILERS".to_string())], "the paragraph list names the page with its title alone: {rows:?}");
    }
    #[test]
    fn test_execute_generate_pages_note_or_g_sheet_single_row_no_header() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("G00.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Hello DesignCraft G00 Narrative!}"#).unwrap();
        let catalog_path = temp_dir.path().join("G00_catalog.csv");
        let escaped = rtf_path.to_string_lossy().replace('"', "\"\"");
        // Title column is empty (as produced when sheet only has 1 row: file)
        std::fs::write(&catalog_path, format!("type,number,title,file\nNote,00,,\"{escaped}\"\n")).unwrap();

        let mut st = PubgenState { card1_range_all: true, ..PubgenState::default() };
        load_card1_csv(&mut st, &catalog_path.to_string_lossy());
        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "generate pages should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert!(active.doc.page_count() >= 2, "narrative sheet gets its own page");

        // Verify NO page header story exists
        assert!(
            !active.doc.stories.values().any(|s| s.text.trim() == "Keterangan Publikasi" || s.text.trim() == "Note"),
            "No Page Header story should be created when sheet only had 1 row file"
        );

        // Verify the RTF narrative is placed on the page
        assert!(
            active.doc.stories.values().any(|s| s.text.contains("Hello DesignCraft G00 Narrative!")),
            "RTF text should be placed directly on the page"
        );

        // Verify the RTF frame starts at margin.y0 (not margin.y0 + 64.0)
        let (margin, _) = {
            let sp_idx = active.doc.spreads.len().saturating_sub(1);
            let spread = &active.doc.spreads[sp_idx];
            let page = spread.pages.last().unwrap();
            (page.margin_rect(), sp_idx)
        };
        let rtf_item = active
            .doc
            .spreads
            .last()
            .unwrap()
            .items
            .iter()
            .find(|it| it.text_frame().is_some_and(|tf| active.doc.stories.get(&tf.story).is_some_and(|s| s.text.contains("Hello DesignCraft G00"))))
            .expect("RTF frame should exist");
        let bounds = rtf_item.bounds();
        assert_eq!(bounds.y0, margin.y0, "RTF frame should start at top margin without header offset");
    }

    #[test]
    fn test_execute_generate_cover_text_formatting() {
        for style in 0..=10 {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({})).unwrap();
            let st = PubgenState { cover_style: style, ..PubgenState::default() };
            let res = execute_generate_cover(&mut app, &st);
            assert!(res.is_ok(), "Style {style} failed: {res:?}");

            let active = app.session.active().unwrap();
            // Assert that every story on the cover page has styled character runs
            assert!(!active.doc.stories.is_empty(), "Style {style} should create stories");
            for story in active.doc.stories.values() {
                if story.text.is_empty() {
                    continue;
                }
                for run in &story.chars {
                    assert!(run.format.over.size.is_some(), "Story text '{}' in style {style} must have custom size", story.text);
                    assert!(run.format.over.fill.is_some(), "Story text '{}' in style {style} must have custom fill", story.text);
                    let fam = run.format.over.font_family.as_deref().unwrap_or_default();
                    let st = run.format.over.font_style.as_deref().unwrap_or_default();
                    assert!(fam == "Arial" || fam == "Arial Black", "Expected Arial family, got {fam}");
                    if story.text.contains("KARAWANG TIMUR")
                        || story.text.contains("KARAWANG\nTIMUR")
                        || story.text == "2026"
                        || story.text == "20"
                        || story.text == "26"
                    {
                        assert_eq!(fam, "Arial Black", "Title and Year must use Arial Black in story '{}'", story.text);
                    }
                    if story.text.contains("BADAN PUSAT STATISTIK") {
                        assert_eq!(fam, "Arial", "Logo text must use Arial");
                        assert_eq!(st, "Bold Italic", "Logo text must use Bold Italic");
                    }
                }
            }
        }
    }

    #[test]
    fn test_arial_and_arial_black_resolution() {
        let db = designcraft_fonts::FontDb::with_font_dirs(designcraft_fonts::system_font_dirs());
        let face_arial = db.face("Arial", "Regular");
        println!("face Arial Regular: family='{}' style='{}'", face_arial.family, face_arial.style);
        let face_ariblk = db.face("Arial Black", "Regular");
        println!("face Arial Black Regular: family='{}' style='{}'", face_ariblk.family, face_ariblk.style);
        let face_arial_blk = db.face("Arial", "Black");
        println!("face Arial Black: family='{}' style='{}'", face_arial_blk.family, face_arial_blk.style);
        let face_arial_bi = db.face("Arial", "Bold Italic");
        println!("face Arial Bold Italic: family='{}' style='{}'", face_arial_bi.family, face_arial_bi.style);
    }

    /// Opt-in evidence for review (`CRAFT_UI_VISUAL_FIXTURES=dir cargo test --features pubgen`): the
    /// whole Pubgen panel body in one image, TOC style set to "TOC Table".
    #[test]
    fn test_execute_generate_pages_ulasan_rtf_layout() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("ulasan_test.rtf");
        let rtf_content = br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Bab satu ulasan pemerintahan daerah di Karawang Timur.}"#;
        std::fs::write(&rtf_path, rtf_content).unwrap();

        let st = PubgenState {
            card1_records: vec![vec!["Ulasan".into(), "01".into(), "BAB 01 Pemerintahan".into(), rtf_path.to_string_lossy().to_string()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for Ulasan should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("1 RTF chapter(s) placed"), "Summary should confirm RTF placed: {msg}");

        // Verify items on the generated page (page 1)
        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2, "1 initial page + 1 generated page");

        let (spread_idx, _) = active.doc.page_loc(1).unwrap();
        let spread = &active.doc.spreads[spread_idx];
        let texts: Vec<String> =
            spread.items.iter().filter_map(|it| it.text_frame()).filter_map(|tf| active.doc.story(tf.story)).map(|st| st.text.clone()).collect();

        // Must contain "BAB 01", "Pemerintahan", and the RTF text
        assert!(texts.iter().any(|t| t.contains("BAB 01")), "Page should contain 'BAB 01', found: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Pemerintahan")), "Page should contain 'Pemerintahan', found: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Bab satu ulasan pemerintahan")), "Page should contain imported RTF content, found: {texts:?}");
    }

    #[test]
    fn test_bxx_sheet_gets_rxx_page_layout() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("B01.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Kata pengantar untuk laporan ini.}"#).unwrap();

        // `B01` converts as `Ulasan` (see `is_b_sheet`), so its catalog row is the RXX shape.
        let st = PubgenState {
            card1_records: vec![vec!["Ulasan".into(), "01".into(), "Kata Pengantar".into(), rtf_path.to_string_lossy().to_string()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for a BXX-style Ulasan row should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("1 RTF chapter(s) placed"), "Summary should confirm RTF placed: {msg}");

        let active = app.session.active().unwrap();
        let (spread_idx, _) = active.doc.page_loc(1).unwrap();
        let spread = &active.doc.spreads[spread_idx];
        let stories: Vec<&designcraft_doc::Story> =
            spread.items.iter().filter_map(|it| it.text_frame()).filter_map(|tf| active.doc.story(tf.story)).collect();
        let texts: Vec<&str> = stories.iter().map(|s| s.text.as_str()).collect();

        assert!(!texts.iter().any(|t| t.contains("BAB 01")), "A title without a BAB prefix gets no chapter-number box, found: {texts:?}");
        assert!(texts.iter().any(|t| t.eq_ignore_ascii_case("Kata Pengantar")), "Page should contain 'Kata Pengantar', found: {texts:?}");
        let title_para = stories.iter().find(|s| s.text.contains("KATA PENGANTAR")).and_then(|s| s.paras.first()).expect("the centered title frame");
        assert_eq!(title_para.style, TOC_NARRATIVE_STYLE, "the title frame must carry the narrative style so the paragraph list names the page");
        assert_eq!(title_para.para.align, Some(designcraft_doc::Align::Center), "the front-matter title is centered on the margin");
        assert!(texts.iter().any(|t| t.contains("Kata pengantar untuk laporan ini")), "Page should contain the placed RTF body, found: {texts:?}");
    }

    #[test]
    fn test_bxx_bilingual_dual_spread_and_italics() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let id_rtf = temp_dir.path().join("B01.rtf");
        let en_rtf = temp_dir.path().join("B01_en.rtf");
        std::fs::write(&id_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Kata pengantar bahasa Indonesia.}"#).unwrap();
        std::fs::write(&en_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Preface narrative text in English.}"#).unwrap();
        let img1 = sample_image_path();
        let img2 = sample_image_path();

        let st = PubgenState {
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "Kata Pengantar".into(),
                id_rtf.to_string_lossy().to_string(),
                img1,
                img2,
                "Preface".into(),
                en_rtf.to_string_lossy().to_string(),
            ]],
            card1_headers: vec![
                "type".into(),
                "number".into(),
                "title".into(),
                "file".into(),
                "image1".into(),
                "image2".into(),
                "title_en".into(),
                "file_en".into(),
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            bilingual_output: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for bilingual BXX should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // 1 initial + 1 ID page + 1 EN page
        assert_eq!(active.doc.page_count(), 3);

        // 1. Indonesian page (page 1)
        let (id_si, _) = active.doc.page_loc(1).unwrap();
        let id_spread = &active.doc.spreads[id_si];
        let id_stories: Vec<&designcraft_doc::Story> = id_spread
            .items
            .iter()
            .filter(|it| active.doc.page_of_item(it.id) == Some(1))
            .filter_map(|it| it.text_frame())
            .filter_map(|tf| active.doc.story(tf.story))
            .collect();
        let id_texts: Vec<&str> = id_stories.iter().map(|s| s.text.as_str()).collect();

        assert!(id_texts.iter().any(|t| t.contains("KATA PENGANTAR")), "Indonesian page must contain 'KATA PENGANTAR', found: {id_texts:?}");
        assert!(!id_texts.iter().any(|t| t.contains("PREFACE")), "Indonesian page must NOT contain 'PREFACE', found: {id_texts:?}");
        assert!(!id_texts.iter().any(|t| t.contains("BAB 01")), "Indonesian front matter gets no BAB 01 box, found: {id_texts:?}");
        let id_graphic_frames = id_spread.items.iter().filter(|it| active.doc.page_of_item(it.id) == Some(1) && it.graphic().is_some()).count();
        assert_eq!(id_graphic_frames, 2, "Indonesian page must contain image1 and image2 graphic frames");

        // 2. English page (page 2)
        let (en_si, _) = active.doc.page_loc(2).unwrap();
        let en_spread = &active.doc.spreads[en_si];
        let en_stories: Vec<&designcraft_doc::Story> = en_spread
            .items
            .iter()
            .filter(|it| active.doc.page_of_item(it.id) == Some(2))
            .filter_map(|it| it.text_frame())
            .filter_map(|tf| active.doc.story(tf.story))
            .collect();
        let en_texts: Vec<&str> = en_stories.iter().map(|s| s.text.as_str()).collect();

        assert!(en_texts.iter().any(|t| t.contains("PREFACE")), "English page must contain 'PREFACE', found: {en_texts:?}");
        assert!(!en_texts.iter().any(|t| t.contains("CHAPTER 01")), "English front matter must NOT contain 'CHAPTER 01', found: {en_texts:?}");
        assert!(!en_texts.iter().any(|t| t.contains("KATA PENGANTAR")), "English page must NOT contain 'KATA PENGANTAR', found: {en_texts:?}");
        let en_graphic_frames = en_spread.items.iter().filter(|it| active.doc.page_of_item(it.id) == Some(2) && it.graphic().is_some()).count();
        assert_eq!(en_graphic_frames, 2, "English page must contain image1 and image2 graphic frames");

        // English page text should all be in italic
        for s in &en_stories {
            for run in &s.chars {
                let style = run.format.over.font_style.as_deref().unwrap_or_default();
                assert!(
                    style.eq_ignore_ascii_case("italic") || style.eq_ignore_ascii_case("bold italic"),
                    "English page text run must be italicized, found style: {style}"
                );
            }
        }
    }

    #[test]
    fn test_execute_generate_pages_grafik_image_layout() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let img_path = sample_image_path();

        let st = PubgenState {
            card1_records: vec![vec!["Grafik".into(), "02.01".into(), "Grafik 02.01 Panjang Jalan Kecamatan Karawang Timur 2026".into(), img_path]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for Grafik should succeed: {res:?}");
        let msg = res.unwrap();
        assert!(msg.contains("1 chart image(s) placed"), "Summary should confirm chart image placed: {msg}");

        // Verify items on the generated page (page 1)
        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2, "1 initial page + 1 generated page");

        let (spread_idx, _) = active.doc.page_loc(1).unwrap();
        let spread = &active.doc.spreads[spread_idx];

        // 1. Check title text frame
        let texts: Vec<String> =
            spread.items.iter().filter_map(|it| it.text_frame()).filter_map(|tf| active.doc.story(tf.story)).map(|st| st.text.clone()).collect();
        assert!(
            texts.iter().any(|t| t.contains("Grafik 02.01") && t.contains("Panjang Jalan Kecamatan Karawang Timur 2026")),
            "Page must contain centered 2-line title, found: {texts:?}"
        );

        // 2. Check graphic frame placed inside margins
        let graphic_frame = spread.items.iter().find(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)));
        assert!(graphic_frame.is_some(), "Page must contain a placed graphic frame");
    }

    #[test]
    fn test_generate_all_canonical_templates() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let templates_dir = root.join("assets").join("templates");
        let dividers_dir = templates_dir.join("dividers");
        let tables_dir = templates_dir.join("data_tables");
        let front_matter_dir = templates_dir.join("front_matter");

        // Covers are hand-designed in `covers/` (copied from `covers_original/`), so this test
        // only generates the interior templates and never writes over them.
        std::fs::create_dir_all(&dividers_dir).unwrap();
        std::fs::create_dir_all(&tables_dir).unwrap();
        std::fs::create_dir_all(&front_matter_dir).unwrap();

        // 1. Divider
        {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({"width": 420.0, "height": 595.0, "pages": 1, "facingPages": false})).unwrap();
            app.run("frame.create", json!({"spread": 0, "rect": [0, 0, 420, 200], "content": "unassigned", "shape": "rectangle"})).unwrap();
            let _ = app.run("object.color", json!({"color": "#122647", "target": "fill"}));
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 60, 348, 40], "content": "text", "text": "BAB 1"}));
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 110, 348, 60], "content": "text", "text": "GEOGRAFI DAN IKLIM"}));
            let _ =
                app.run("frame.create", json!({"spread": 0, "rect": [36, 230, 348, 320], "content": "text", "text": "Deskripsi pengantar bab..."}));

            let active = app.session.active_mut().unwrap();
            let doc = std::sync::Arc::make_mut(&mut active.doc);
            for sp in &mut doc.spreads {
                let sp = std::sync::Arc::make_mut(sp);
                for it in &mut sp.items {
                    let it = std::sync::Arc::make_mut(it);
                    if let designcraft_doc::Content::Text(tf) = &it.content
                        && let Some(story) = doc.stories.get(&tf.story)
                    {
                        if story.text.contains("BAB 1") {
                            it.label = "#slot:text:chapter_num".to_string();
                        } else if story.text.contains("GEOGRAFI") {
                            it.label = "#slot:text:chapter_title".to_string();
                        } else {
                            it.label = "#slot:story:narrative_body".to_string();
                        }
                    }
                }
            }
            let target = dividers_dir.join("001_simple_chapter.designcraft");
            app.run("file.saveAs", json!({"path": target.to_string_lossy()})).unwrap();
        }

        // 2. Data Table
        {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({"width": 420.0, "height": 595.0, "pages": 1, "facingPages": false})).unwrap();
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 36, 384, 52], "content": "text", "text": "Tabel 1.1"}));
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 52, 384, 68], "content": "text", "text": "Table 1.1"}));
            let _ = app.run(
                "frame.create",
                json!({"spread": 0, "rect": [36, 70, 384, 90], "content": "text", "text": "Luas Wilayah Menurut Desa/Kelurahan"}),
            );
            let _ = app.run(
                "frame.create",
                json!({"spread": 0, "rect": [36, 90, 384, 110], "content": "text", "text": "Total Area by Village/Urban Village"}),
            );
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 120, 384, 520], "content": "unassigned", "shape": "rectangle"}));
            let _ = app
                .run("frame.create", json!({"spread": 0, "rect": [36, 530, 384, 555], "content": "text", "text": "Sumber: BPS Kabupaten Karawang"}));

            let active = app.session.active_mut().unwrap();
            let doc = std::sync::Arc::make_mut(&mut active.doc);
            for sp in &mut doc.spreads {
                let sp = std::sync::Arc::make_mut(sp);
                for it in &mut sp.items {
                    let it = std::sync::Arc::make_mut(it);
                    match &it.content {
                        designcraft_doc::Content::Text(tf) => {
                            if let Some(story_arc) = doc.stories.get_mut(&tf.story) {
                                let story = std::sync::Arc::make_mut(story_arc);
                                if story.text.contains("Tabel") {
                                    it.label = "#slot:text:table_num".to_string();
                                } else if story.text.contains("Table") {
                                    it.label = "#slot:text:table_kind_en".to_string();
                                    for r in &mut story.chars {
                                        r.format.over.font_style = Some("Italic".to_string());
                                    }
                                } else if story.text.contains("Luas") {
                                    it.label = "#slot:text:table_title".to_string();
                                } else if story.text.contains("Total Area") {
                                    it.label = "#slot:text:table_title_en".to_string();
                                    for r in &mut story.chars {
                                        r.format.over.font_style = Some("Italic".to_string());
                                    }
                                } else {
                                    it.label = "#slot:text:source".to_string();
                                }
                            }
                        }
                        designcraft_doc::Content::Unassigned | designcraft_doc::Content::Graphic(_) => {
                            it.label = "#slot:table:main".to_string();
                        }
                        _ => {}
                    }
                }
            }
            let target = tables_dir.join("001_single_table_standard.designcraft");
            app.run("file.saveAs", json!({"path": target.to_string_lossy()})).unwrap();
        }

        // 3. Publication Note
        {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({"width": 420.0, "height": 595.0, "pages": 1, "facingPages": false})).unwrap();
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 50, 348, 40], "content": "text", "text": "KATA PENGANTAR"}));
            let _ = app.run("frame.create", json!({"spread": 0, "rect": [36, 110, 348, 440], "content": "text", "text": "Publikasi Kecamatan Karawang Timur Dalam Angka 2026..."}));

            let active = app.session.active_mut().unwrap();
            let doc = std::sync::Arc::make_mut(&mut active.doc);
            for sp in &mut doc.spreads {
                let sp = std::sync::Arc::make_mut(sp);
                for it in &mut sp.items {
                    let it = std::sync::Arc::make_mut(it);
                    if let designcraft_doc::Content::Text(tf) = &it.content
                        && let Some(story) = doc.stories.get(&tf.story)
                    {
                        if story.text.contains("KATA PENGANTAR") {
                            it.label = "#slot:text:title".to_string();
                        } else {
                            it.label = "#slot:story:note_body".to_string();
                        }
                    }
                }
            }
            let target = front_matter_dir.join("001_publication_note.designcraft");
            app.run("file.saveAs", json!({"path": target.to_string_lossy()})).unwrap();
        }
    }

    #[test]
    fn test_execute_generate_pages_modular_template_divider() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("ch1.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Ulasan wilayah terinci.}"#).unwrap();

        let st = PubgenState {
            card1_preset: 6,
            card1_records: vec![vec!["Ulasan".into(), "01".into(), "BAB 01 Geografi".into(), rtf_path.to_string_lossy().to_string()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for modular template divider should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2);
        let (si, pi) = active.doc.page_loc(1).unwrap();
        let pg = &active.doc.spreads[si].pages[pi];
        eprintln!("ULASAN PAGE 1: side={:?}, bounds={:?}, margins={:?}, margin_rect={:?}", pg.side, pg.bounds(), pg.margins, pg.margin_rect());
        for it in &active.doc.spreads[si].items {
            eprintln!("  ITEM: label='{}', bounds={:?}", it.label, it.bounds());
        }

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("BAB 01")), "Stories should contain BAB 01: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Geografi")), "Stories should contain Geografi: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Ulasan wilayah terinci")), "Stories should contain RTF text: {texts:?}");
    }

    #[test]
    fn test_reordered_csv_columns_resolve_by_header() {
        // Columns are found by header name, not fixed position.
        let headers: Vec<String> = ["type", "number", "title", "template", "file", "title_en", "file_en"].into_iter().map(String::from).collect();
        assert_eq!(effective_col(3, &headers, &["file", "filename"], 3), 4, "'file' found past the legacy slot");
        assert_eq!(effective_col(4, &headers, &["template"], 4), 3, "'template' found before the legacy slot");
        // Records set directly (no load_card1_csv): stale default positions re-resolve by name.
        let headers6: Vec<String> = ["type", "number", "title", "file", "template", "file_en"].into_iter().map(String::from).collect();
        assert_eq!(effective_col(6, &headers6, &["file_en", "fileen"], 6), 5, "stale default re-resolves");
        assert_eq!(effective_col(4, &headers6, &["template"], 4), 4, "matching stored column is kept");
        // Missing header reads as absent: MAX (row.get(MAX) is None, i.e. empty text —
        // identical behaviour to reading past the end of the row).
        let headers4: Vec<String> = ["type", "number", "title", "file"].into_iter().map(String::from).collect();
        assert_eq!(effective_col(6, &headers4, &["file_en", "fileen"], 6), usize::MAX);
    }

    #[test]
    fn test_row_template_slots_divider_and_table() {
        let st = PubgenState { bilingual_output: true, ..Default::default() };

        // Divider in Dual Lang: chapter parts + title_en, no table slots.
        let s = row_template_slots("Divider", "02", "BAB 02 PEMERINTAHAN", "CHAPTER 02 GOVERNMENT", "", false, &st);
        assert_eq!(s.text.get("first_word"), Some(&json!("BAB")));
        assert_eq!(s.text.get("chapter_num"), Some(&json!("02")));
        assert_eq!(s.text.get("chapter_title"), Some(&json!("PEMERINTAHAN")));
        assert_eq!(s.text.get("title_en"), Some(&json!("CHAPTER 02 GOVERNMENT")));
        assert!(s.story.is_empty());
        assert_eq!(s.images_resolved + s.rtfs_resolved, 0, "no files on disk to resolve");

        // Divider preface never carries title_en.
        let s = row_template_slots("Divider", "01", "BAB 01 X", "EN X", "", true, &st);
        assert!(s.text.get("title_en").is_none());

        // Table single lang: localized title, no EN slots.
        let single = PubgenState::default();
        let s = row_template_slots("Tabel", "02.01", "Luas Wilayah", "", "", false, &single);
        assert!(s.text.contains_key("table_num"));
        assert_eq!(s.text.get("table_kind_en"), Some(&json!("")));
        assert_eq!(s.text.get("table_title_en"), Some(&json!("")));

        // Table with an EN title: bilingual slots.
        let s = row_template_slots("Tabel", "02.01", "Luas Wilayah", "Area", "", false, &st);
        assert_eq!(s.text.get("table_title"), Some(&json!("Luas Wilayah")));
        assert_eq!(s.text.get("table_title_en"), Some(&json!("Area")));
        assert!(s.text.get("table_kind_en").is_some());
    }

    #[test]
    fn test_effective_col_skips_foreign_fallback() {
        // User's catalog: 7 columns where image1/image2 sit at the legacy
        // title_en/file_en positions. file_en must read as ABSENT, not as image2.
        let headers: Vec<String> = ["type", "number", "title", "file", "template", "image1", "image2"].into_iter().map(String::from).collect();
        assert_eq!(effective_col(6, &headers, &["file_en", "fileen"], 6), usize::MAX);
        assert_eq!(effective_col(5, &headers, &["title_en", "titleen"], 5), usize::MAX);
        // file is found by name at its real position.
        assert_eq!(effective_col(3, &headers, &["file", "filename"], 3), 3);
        // Legacy 7-column layout keeps working.
        let legacy: Vec<String> = ["type", "number", "title", "file", "template", "title_en", "file_en"].into_iter().map(String::from).collect();
        assert_eq!(effective_col(5, &legacy, &["title_en", "titleen"], 5), 5);
        assert_eq!(effective_col(6, &legacy, &["file_en", "fileen"], 6), 6);
    }

    #[test]
    fn test_find_named_column_mapping() {
        let headers = vec!["No".into(), "Judul".into(), "Berkas".into(), "File".into(), "FILE_EN".into()];
        assert_eq!(find_named_col(&headers, &["file", "filename"], 3), 3);
        assert_eq!(find_named_col(&headers, &["file_en", "fileen"], 6), 4);
        assert_eq!(find_named_col(&headers, &["title_en", "titleen"], 5), 5, "missing header keeps legacy position");
        // 'file' must not swallow 'file_en'
        let only_en = vec!["file_en".to_string()];
        assert_eq!(find_named_col(&only_en, &["file", "filename"], 3), 3);
    }

    #[test]
    fn test_execute_generate_pages_modular_template_note() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("note.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Penjelasan teknis publikasi.}"#).unwrap();

        let st = PubgenState {
            card1_preset: 6,
            card1_records: vec![vec!["Note".into(), "".into(), "Keterangan Publikasi".into(), rtf_path.to_string_lossy().to_string()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for modular template note should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("Keterangan Publikasi")), "Stories should contain note title: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Penjelasan teknis publikasi")), "Stories should contain RTF note body: {texts:?}");
    }

    #[test]
    fn test_execute_generate_pages_modular_template_table() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let csv_path = temp_dir.path().join("table_1_1.csv");
        std::fs::write(&csv_path, "Kecamatan,Luas\nKarawang Barat,30.5\nKarawang Timur,25.2\nTotal,55.7\n").unwrap();

        let st = PubgenState {
            card1_preset: 6,
            card1_records: vec![vec!["Tabel".into(), "1.1".into(), "Luas Wilayah Menurut Kecamatan".into(), csv_path.to_string_lossy().to_string()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_auto_insert_tables: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for modular template table should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("01.01")), "Stories should contain table number: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Luas Wilayah")), "Stories should contain table title: {texts:?}");

        let has_table = active.doc.stories.values().any(|s| !s.tables.is_empty());
        assert!(has_table, "Document should contain an inserted table");
    }

    #[test]
    fn test_table_bilingual_generation() {
        // 1. Test split_table_title
        let (id, en) = locale::split_table_title("Banyaknya Penduduk / Number of Population");
        assert_eq!(id, "Banyaknya Penduduk");
        assert_eq!(en.as_deref(), Some("Number of Population"));

        let (id_single, en_none) = locale::split_table_title("Banyaknya Penduduk");
        assert_eq!(id_single, "Banyaknya Penduduk");
        assert_eq!(en_none, None);

        // 2. Test Preset 0 bilingual generation
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec!["Tabel".into(), "01.01".into(), "Luas Daerah / Total Area".into(), "".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "Bilingual Preset 0 table should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        // Col 1 has Tabel and Table joined by forced line break
        assert!(texts.iter().any(|t| t.contains("Tabel") && t.contains("Table")), "Col 1 should have Tabel and Table: {texts:?}");
        // Col 3 has Luas Daerah and Total Area
        assert!(
            texts.iter().any(|t| t.contains("Luas Daerah") && t.contains("Total Area")),
            "Col 3 should have Luas Daerah and Total Area: {texts:?}"
        );

        // 3. Test Modular Template bilingual generation
        let mut app2 = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app2.run("file.new", json!({})).unwrap();

        let mut st2 = st.clone();
        st2.card1_preset = 6;
        let res2 = execute_generate_pages(&mut app2, &st2);
        assert!(res2.is_ok(), "Bilingual Modular table should succeed: {res2:?}");
        let active2 = app2.session.active().unwrap();
        let texts2: Vec<String> = active2.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts2.iter().any(|t| t.contains("Luas Daerah")), "Template should bind table_title: {texts2:?}");
        assert!(texts2.iter().any(|t| t.contains("Total Area")), "Template should bind table_title_en: {texts2:?}");
    }

    #[test]
    fn test_table_sheet_xx_xx_bilingual_header_textboxes() {
        let first_row = vec![
            "Tabel 01.01 Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur / Land Area by Village/Sub-district in East Karawang District, 2025".to_string()
        ];
        let (t, n, title, file) = excel::parse_catalog_entry(&first_row, "01_01");
        assert_eq!(t, "Tabel");
        assert_eq!(n, "01.01");
        assert_eq!(file, "01_01");
        assert_eq!(
            title,
            "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur / Land Area by Village/Sub-district in East Karawang District, 2025"
        );

        let (id_harmonized, en_harmonized_opt) = excel::split_and_harmonize_table_title(&title, None);
        assert_eq!(id_harmonized, "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025");
        assert_eq!(en_harmonized_opt.as_deref(), Some("Land Area by Village/Sub-district in East Karawang District, 2025"));

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec![t, n, title, file]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "Page generation should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let stories: Vec<_> = active.doc.stories.values().collect();

        // 1. Textbox 1 contains 2 lines: "Tabel\nTable" with "Table" in Italic
        let box1 = stories.iter().find(|s| s.text.contains("Tabel") && s.text.contains("Table")).expect("Textbox 1 found");
        let (p_line1, p_line2) = box1.text.split_once(designcraft_doc::story::FORCED_LINE_BREAK).expect("2 lines in textbox 1");
        assert_eq!(p_line1, "Tabel");
        assert_eq!(p_line2, "Table");

        // 2. Textbox 2 contains XX.XX ("01.01")
        let box2 = stories.iter().find(|s| s.text == "01.01").expect("Textbox 2 found with 01.01");
        assert_eq!(box2.text, "01.01");

        // 3. Textbox 3 contains 2 lines:
        // Line 1: "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025"
        // Line 2: "Land Area by Village/Sub-district in East Karawang District, 2025"
        let box3 = stories
            .iter()
            .find(|s| s.text.contains("Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025"))
            .expect("Textbox 3 found");
        let (t3_line1, t3_line2) = box3.text.split_once(designcraft_doc::story::FORCED_LINE_BREAK).expect("2 lines in textbox 3");
        assert_eq!(t3_line1, "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025");
        assert_eq!(t3_line2, "Land Area by Village/Sub-district in East Karawang District, 2025");
    }

    #[test]
    fn test_chart_bilingual_parts_and_italics() {
        let title = "Grafik / Chart 02.01 Panjang Jalan / Road Length, 2026";
        let parts = locale::split_chart_title_parts(title);
        assert_eq!(parts.id_kicker, "Grafik");
        assert_eq!(parts.en_kicker.as_deref(), Some("Chart"));
        assert_eq!(parts.number, "02.01");
        assert_eq!(parts.id_subtitle, "Panjang Jalan");
        assert_eq!(parts.en_subtitle.as_deref(), Some("Road Length"));
        assert_eq!(parts.suffix, ", 2026");

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec!["Grafik".into(), "02.01".into(), title.into(), "".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "Bilingual chart generation should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let chart_story = active.doc.stories.values().find(|s| s.text.contains("Grafik / Chart")).expect("Chart story must exist");
        assert!(chart_story.text.contains("Grafik / Chart 02.01"), "Line 1 should have kicker and number");
        assert!(chart_story.text.contains("Panjang Jalan / Road Length, 2026"), "Line 2 should have subtitle and suffix");

        // Verify that 'Chart' and 'Road Length' runs have Bold Italic
        let has_italic_chart = chart_story
            .chars
            .iter()
            .any(|r| r.format.over.font_style.as_deref() == Some("Bold Italic") || r.format.over.font_style.as_deref() == Some("Italic"));
        assert!(has_italic_chart, "Chart title must have italicised English parts");
    }

    #[test]
    fn test_chart_bilingual_three_line_title_from_title_en() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec![
                "Grafik".into(),
                "02.01".into(),
                "Panjang Jalan Kecamatan Karawang Timur 2026".into(),
                "".into(),
                "".into(),
                "Chart 02.01 Road Length in East Karawang District, 2025".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "title_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "3-line bilingual chart generation should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let chart_story = active.doc.stories.values().find(|s| s.text.contains("Grafik 02.01")).expect("Chart story must exist");

        // Verify 3 lines separated by FORCED_LINE_BREAK
        let lines: Vec<&str> = chart_story.text.split(designcraft_doc::story::FORCED_LINE_BREAK).collect();
        assert_eq!(lines.len(), 3, "Expected 3 lines, got {:?}", lines);
        assert_eq!(lines[0], "Grafik 02.01");
        assert_eq!(lines[1], "Panjang Jalan Kecamatan Karawang Timur 2026");
        assert_eq!(lines[2], "Chart 02.01 Road Length in East Karawang District, 2025");

        // Verify font styling: Line 1 and Line 2 are Bold 11pt, Line 3 is Italic 9pt
        let en_line_text = "Chart 02.01 Road Length in East Karawang District, 2025";
        let en_start = chart_story.text.find(en_line_text).expect("EN line must be in text");

        let mut curr_offset = 0;
        for run in &chart_story.chars {
            let run_start = curr_offset;
            let run_end = curr_offset + run.len;
            curr_offset = run_end;

            if run_end <= en_start {
                assert_eq!(run.format.over.font_style.as_deref(), Some("Bold"), "Lines 1-2 must be Bold");
                assert_eq!(run.format.over.size, Some(11.0), "Lines 1-2 must be 11pt");
            } else if run_start >= en_start {
                assert_eq!(run.format.over.font_style.as_deref(), Some("Italic"), "Line 3 must be Italic");
                assert_eq!(run.format.over.size, Some(9.0), "Line 3 must be 9pt (smaller font)");
            }
        }
    }

    #[test]
    fn test_resolve_template_path_spec() {
        // Direct template ID resolution from assets/templates/
        let r1 = resolve_template_path_spec("", "001_simple_chapter");
        assert!(r1.is_some(), "Should resolve 001_simple_chapter: {r1:?}");
        assert!(r1.unwrap().contains("001_simple_chapter.designcraft"));

        // Subcategory path resolution
        let r2 = resolve_template_path_spec("", "dividers/001_simple_chapter");
        assert!(r2.is_some(), "Should resolve dividers/001_simple_chapter: {r2:?}");

        // Batik infographic divider resolution
        let r_batik = resolve_template_path_spec("", "002_batik_infographic");
        assert!(r_batik.is_some(), "Should resolve 002_batik_infographic: {r_batik:?}");
        assert!(r_batik.unwrap().contains("002_batik_infographic.designcraft"));

        // Legacy / alias resolution
        let r_legacy1 = resolve_template_path_spec("", "dividers/001_ulasan_chapter");
        assert!(r_legacy1.is_some(), "Should resolve legacy ulasan: {r_legacy1:?}");
        assert!(r_legacy1.unwrap().contains("001_simple_chapter.designcraft"));

        let r_legacy2 = resolve_template_path_spec("", "dividers/special_infographic_divider");
        assert!(r_legacy2.is_some(), "Should resolve special infographic alias: {r_legacy2:?}");
        assert!(r_legacy2.unwrap().contains("002_batik_infographic.designcraft"));

        // Table template resolution
        let r3 = resolve_template_path_spec("", "001_single_table_standard");
        assert!(r3.is_some(), "Should resolve 001_single_table_standard: {r3:?}");

        // Non-existent template returns None
        let r_none = resolve_template_path_spec("", "completely_fake_template_9999");
        assert!(r_none.is_none());
    }

    #[test]
    fn test_execute_generate_pages_with_explicit_template_column() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("ch2.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Ulasan sosial Bab 2.}"#).unwrap();

        // Note: card1_preset is set to 0 (procedural preset), but column 4 explicitly
        // specifies a template. It MUST use the modular template!
        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec![
                "Ulasan".into(),
                "02".into(),
                "BAB 02 Sosial".into(),
                rtf_path.to_string_lossy().to_string(),
                "dividers/001_ulasan_chapter".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages with explicit template column should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("BAB 02")), "Stories should contain BAB 02: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Sosial")), "Stories should contain Sosial: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Ulasan sosial Bab 2")), "Stories should contain RTF text: {texts:?}");
    }

    #[test]
    fn test_ulasan_row_with_erroneous_divider_template_places_rtf() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let rtf_path = temp_dir.path().join("ch1.rtf");
        std::fs::write(&rtf_path, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Narasi bab 1 geografi dan iklim.}"#).unwrap();

        // Row has type "Ulasan", but template explicitly says "dividers/special_infographic_divider"
        // (e.g. from an Excel sheet where the user specified a divider style).
        // It MUST NOT render as a Batik Infographic divider, and MUST place the RTF text!
        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "BAB 01 Geografi".into(),
                rtf_path.to_string_lossy().to_string(),
                "dividers/special_infographic_divider".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("Narasi bab 1 geografi")), "Ulasan RTF must be placed into the document: {texts:?}");
    }

    #[test]
    fn test_procedural_divider_art_fallback() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Simulate DIV sheet row without template on disk (e.g. portable app release)
        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec![
                "Divider".into(),
                "01".into(),
                "BAB 01 Geografi".into(),
                "".into(),
                "dividers/special_infographic_divider".into(),
                "Chapter 01 Geography".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "title_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_title_en_col: 5,
            card1_range_all: true,
            bilingual_output: true,
            cover_color_primary: "#0284c7".into(),
            cover_color_accent: "#f59e0b".into(),
            cover_color_dark: "#1e293b".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages should generate procedural divider: {res:?}");

        let active = app.session.active().unwrap();
        // Page 1 is initial, Page 2 is procedural divider
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("01")), "Stories should contain chapter num 01: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("BAB")), "Stories should contain prefix BAB: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Geografi")), "Stories should contain title Geografi: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Chapter 01 Geography")), "Stories should contain title_en: {texts:?}");
    }

    #[test]
    fn test_execute_generate_pages_divider_with_002_template() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let img_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/infografik_01.png");

        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec!["Divider".into(), "02".into(), "BAB 02 PEMERINTAHAN".into(), img_path.into(), "002".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for divider should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        // 3 textbox values from col title:
        // first_word replacing "BAB" -> "BAB"
        assert!(texts.iter().any(|t| t.contains("BAB")), "Stories should contain BAB: {texts:?}");
        // digit number replacing "01" -> "02"
        assert!(texts.iter().any(|t| t.contains("02")), "Stories should contain 02: {texts:?}");
        // rest of text replacing "Geografi" -> "PEMERINTAHAN"
        assert!(texts.iter().any(|t| t.contains("PEMERINTAHAN")), "Stories should contain PEMERINTAHAN: {texts:?}");

        // Shape with color #e5007e has image placed into it
        let has_image = active.doc.spreads.iter().flat_map(|sp| &sp.items).any(|it| matches!(it.content, designcraft_doc::Content::Graphic(_)));
        assert!(has_image, "Divider page should contain placed image");

        // Verify page geometry is full A4 (210 x 297 mm)
        let last_sp = active.doc.spreads.last().unwrap();
        let last_p = last_sp.pages.last().unwrap();
        assert!((last_p.width - 595.27559).abs() < 0.1, "Divider page should be full A4 width 595.28: {}", last_p.width);
        assert!((last_p.height - 841.88976).abs() < 0.1, "Divider page should be full A4 height 841.89: {}", last_p.height);

        // Verify image fits frame with fitContentToFrame
        let img_item = last_sp.items.iter().find(|it| it.label.contains("image")).unwrap();
        if let designcraft_doc::Content::Graphic(g) = &img_item.content {
            let placed_rect = g.xf.transform_rect_bbox(designcraft_geom::Rect::new(0.0, 0.0, g.size.0, g.size.1));
            let placed_w = placed_rect.width();
            let placed_h = placed_rect.height();
            let frame_bounds = img_item.inner_bounds();
            assert!(
                (placed_w - frame_bounds.width()).abs() <= 1.0,
                "Placed width {} must match frame {} for fitContentToFrame",
                placed_w,
                frame_bounds.width()
            );
            assert!(
                (placed_h - frame_bounds.height()).abs() <= 1.0,
                "Placed height {} must match frame {} for fitContentToFrame",
                placed_h,
                frame_bounds.height()
            );
        }

        // Verify divider page adjusts to Page Size (margins == 0.0)
        assert_eq!(last_p.margins.top, 0.0, "Divider page margins must be 0 (Page Size)");
        assert_eq!(last_p.margins.bottom, 0.0, "Divider page margins must be 0 (Page Size)");
        assert_eq!(last_p.margins.inside, 0.0, "Divider page margins must be 0 (Page Size)");
        assert_eq!(last_p.margins.outside, 0.0, "Divider page margins must be 0 (Page Size)");

        let mut renderer = designcraft_render::Renderer::new();
        let cache = designcraft_compose::Cache::new();
        let opts = designcraft_render::RenderOptions::default();
        let rendered = renderer.render_page(&active.doc, &cache, 1, 1.0, false, &opts);
        assert!(rendered.is_some(), "Divider page should render cleanly without panic");
    }

    #[test]
    fn test_content_pages_adjust_to_page_margins_while_cover_and_div_adjust_to_page_size() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![
                vec!["COVER".into(), "".into(), "PUBLICATION COVER".into(), "".into(), "".into()],
                vec!["Note".into(), "".into(), "CATATAN TEKNIS".into(), "".into(), "".into()],
                vec!["DIV".into(), "01".into(), "BAB 01".into(), "".into(), "".into()],
                vec!["DIV01".into(), "01".into(), "BAB 01".into(), "".into(), "".into()],
                vec!["DIV1".into(), "02".into(), "BAB 02".into(), "".into(), "".into()],
                vec!["Tabel".into(), "1.1".into(), "Luas Wilayah".into(), "".into(), "".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        // Set document margins (e.g. 54 pt ~ 19mm)
        let _ = execute_set_page_margins(&mut app);

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // Spreads / pages: Page 0 is initial blank, then 6 added pages -> total 7 pages
        assert_eq!(active.doc.page_count(), 7);

        const PT_PER_MM: f64 = 72.0 / 25.4;
        let expected_top = 20.20 * PT_PER_MM;

        // Page 1: COVER -> margins should be 0.0 (adjust to Page Size)
        let (sp1_idx, p1_idx) = active.doc.page_loc(1).unwrap();
        let p1 = &active.doc.spreads[sp1_idx].pages[p1_idx];
        assert_eq!(p1.margins.top, 0.0, "COVER page margins must be 0 (Page Size)");

        // Page 2: Note -> margins should be custom doc_margins (adjust to Page Margin)
        let (sp2_idx, p2_idx) = active.doc.page_loc(2).unwrap();
        let p2 = &active.doc.spreads[sp2_idx].pages[p2_idx];
        assert!(
            (p2.margins.top - expected_top).abs() < 0.01,
            "Note content page must retain custom doc margins (expected {expected_top}, got {})",
            p2.margins.top
        );
        let p2_m = p2.margin_rect();
        // Check that items on page 2 are inside margins
        let sp2 = &active.doc.spreads[sp2_idx];
        for it in &sp2.items {
            let ib = it.bounds();
            if ib.x0 >= p2.bounds().x0 && ib.x1 <= p2.bounds().x1 {
                assert!(ib.x0 >= p2_m.x0 - 1.0, "Item left {} must align with margin {}", ib.x0, p2_m.x0);
                assert!(ib.y0 >= p2_m.y0 - 1.0, "Item top {} must align with margin {}", ib.y0, p2_m.y0);
                assert!(ib.x1 <= p2_m.x1 + 1.0, "Item right {} must not exceed margin {}", ib.x1, p2_m.x1);
            }
        }

        // Page 3: DIV -> margins should be 0.0 (adjust to Page Size)
        let (sp3_idx, p3_idx) = active.doc.page_loc(3).unwrap();
        let p3 = &active.doc.spreads[sp3_idx].pages[p3_idx];
        assert_eq!(p3.margins.top, 0.0, "DIV page margins must be 0 (Page Size)");

        // Page 4: DIV01 -> margins should be 0.0 (adjust to Page Size)
        let (sp4_idx, p4_idx) = active.doc.page_loc(4).unwrap();
        let p4 = &active.doc.spreads[sp4_idx].pages[p4_idx];
        assert_eq!(p4.margins.top, 0.0, "DIV01 page margins must be 0 (Page Size)");

        // Page 5: DIV1 -> margins should be 0.0 (adjust to Page Size)
        let (sp5_idx, p5_idx) = active.doc.page_loc(5).unwrap();
        let p5 = &active.doc.spreads[sp5_idx].pages[p5_idx];
        assert_eq!(p5.margins.top, 0.0, "DIV1 page margins must be 0 (Page Size)");

        // Page 6: Tabel -> margins should be custom doc_margins (adjust to Page Margin)
        let (sp6_idx, p6_idx) = active.doc.page_loc(6).unwrap();
        let p6 = &active.doc.spreads[sp6_idx].pages[p6_idx];
        assert!(
            (p6.margins.top - expected_top).abs() < 0.01,
            "Tabel content page must retain custom doc margins (expected {expected_top}, got {})",
            p6.margins.top
        );
        let p6_m = p6.margin_rect();
        let sp6 = &active.doc.spreads[sp6_idx];
        for it in &sp6.items {
            let ib = it.bounds();
            if ib.x0 >= p6.bounds().x0 && ib.x1 <= p6.bounds().x1 {
                assert!(ib.x0 >= p6_m.x0 - 1.0, "Header/table item left {} must align with margin {}", ib.x0, p6_m.x0);
                assert!(ib.y0 >= p6_m.y0 - 1.0, "Header/table item top {} must align with margin {}", ib.y0, p6_m.y0);
                assert!(ib.x1 <= p6_m.x1 + 1.0, "Header/table item right {} must not exceed margin {}", ib.x1, p6_m.x1);
            }
        }
    }

    #[test]
    fn test_custom_page_margins_from_margins_and_columns_applied_to_content() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // User sets custom margins via layout.marginsAndColumns (e.g. top: 48, bottom: 50, inside: 60, outside: 40)
        app.run(
            "layout.marginsAndColumns",
            json!({
                "margins": {
                    "top": 48.0,
                    "bottom": 50.0,
                    "inside": 60.0,
                    "outside": 40.0
                }
            }),
        )
        .unwrap();

        let st = PubgenState {
            card1_preset: 0,
            card1_records: vec![vec!["Tabel".into(), "01.01".into(), "Data Penduduk".into(), "".into(), "".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(1).unwrap();
        let page = &active.doc.spreads[si].pages[pi];
        assert_eq!(page.margins.top, 48.0);
        assert_eq!(page.margins.bottom, 50.0);
        assert_eq!(page.margins.inside, 60.0);
        assert_eq!(page.margins.outside, 40.0);

        let m = page.margin_rect();
        let sp = &active.doc.spreads[si];
        for it in &sp.items {
            let ib = it.bounds();
            if ib.x0 >= page.bounds().x0 && ib.x1 <= page.bounds().x1 {
                assert!(ib.x0 >= m.x0 - 1.0, "Item left {} must align with margin {}", ib.x0, m.x0);
                assert!(ib.y0 >= m.y0 - 1.0, "Item top {} must align with margin {}", ib.y0, m.y0);
                assert!(ib.x1 <= m.x1 + 1.0, "Item right {} must not exceed margin {}", ib.x1, m.x1);
            }
        }
    }

    #[test]
    fn test_divider_bilingual_slots() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec![
                "Divider".into(),
                "01".into(),
                "BAB 01 GEOGRAFI".into(),
                "".into(),
                "002".into(),
                "GEOGRAPHY AND CLIMATE".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "title_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for bilingual divider should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("GEOGRAFI")), "Stories should contain Indonesian title GEOGRAFI: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("GEOGRAPHY AND CLIMATE")), "Stories should contain English title: {texts:?}");

        // Dual Lang Mode: the title_en textbox is 22 pt italic, black stroke behind the fill.
        let en_story = active.doc.stories.values().find(|s| s.text.contains("GEOGRAPHY AND CLIMATE")).expect("EN title story exists");
        assert!(!en_story.chars.is_empty(), "EN title story has runs");
        for run in &en_story.chars {
            assert_eq!(run.format.over.size, Some(22.0), "title_en run size: {run:?}");
            assert_eq!(run.format.over.font_style.as_deref(), Some("Italic"), "title_en run style: {run:?}");
            assert_eq!(run.format.over.stroke.as_deref(), Some("[Black]"), "title_en run stroke: {run:?}");
            assert_eq!(run.format.over.stroke_behind_fill, Some(true), "title_en stroke behind fill: {run:?}");
        }
    }

    #[test]
    fn test_ulasan_dual_spread() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let dir = std::env::temp_dir().join(format!("designcraft_pubgen_dual_ulasan_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let id_rtf = dir.join("R01_id.rtf");
        let en_rtf = dir.join("R01_en.rtf");
        std::fs::write(&id_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Ulasan wilayah Indonesia.}"#).unwrap();
        std::fs::write(&en_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 CHAPTER 01 GEOGRAPHY\par Narrative review in English.}"#)
            .unwrap();

        let st = PubgenState {
            card1_preset: 6,
            bilingual_output: true,
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "BAB 01 Geografi".into(),
                id_rtf.to_string_lossy().to_string(),
                "".into(),
                en_rtf.to_string_lossy().to_string(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "file_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for dual Ulasan should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // Initial page (1) + Spread 1 (1) + Spread 2 (1) = 3 pages
        assert_eq!(active.doc.page_count(), 3, "Should generate two spreads for dual Ulasan");

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("Ulasan wilayah Indonesia")), "Should contain ID RTF body: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Narrative review in English")), "Should contain EN RTF body: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("GEOGRAPHY")), "Should contain derived EN chapter title: {texts:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ai_translated_body_drives_the_en_spread_without_file_en() {
        // The user's report: reviews-only AI run reaches 100%, but the R01 page shows
        // Indonesian only — the English spread never appears. With the translations map
        // filled (as the worker leaves it) and no file_en, the spread must still come.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let dir = std::env::temp_dir().join(format!("designcraft_pubgen_ai_ulasan_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let id_rtf = dir.join("R01_id.rtf");
        std::fs::write(&id_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Ulasan wilayah Indonesia."#).unwrap();

        let mut st = PubgenState {
            card1_preset: 6,
            bilingual_output: true,
            translate_with_ai: true,
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "BAB 01 Geografi".into(),
                id_rtf.to_string_lossy().to_string(),
                "".into(),
                "".into(),
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "file_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            translations: HashMap::from([("Ulasan wilayah Indonesia.".to_string(), "Indonesian region review.".to_string())]),
            ..Default::default()
        };

        // The temp RTF the panel writes from the translated paragraphs.
        let en_rtf = ai::translated_body_rtf(&st, id_rtf.to_string_lossy().as_ref());
        assert!(en_rtf.is_some(), "translated body RTF should be written from the map");
        let en_rtf = en_rtf.expect("checked");
        let en_text = std::fs::read_to_string(&en_rtf).expect("temp rtf readable");
        assert!(en_text.contains("Indonesian region review"), "{en_text}");

        // Point the record's file_en column at the temp RTF, exactly as the spread gate does.
        st.card1_records[0][5] = en_rtf.to_string_lossy().to_string();

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "generate should succeed: {res:?}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 3, "ID spread + EN spread expected");
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("Indonesian region review")), "EN body placed: {texts:?}");

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&en_rtf);
    }

    #[test]
    fn test_rxx_bilingual_dual_column_layout() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let dir = std::env::temp_dir().join(format!("pubgen_rxx_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let id_rtf = dir.join("R01.rtf");
        let en_rtf = dir.join("R01_en.rtf");
        std::fs::write(&id_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 Kecamatan Karawang Timur merupakan bagian tak terpisahkan.}"#)
            .unwrap();
        std::fs::write(&en_rtf, br#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 East Karawang District is an integral part of Karawang.}"#)
            .unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "Bab 01 Geografi".into(),
                id_rtf.to_string_lossy().to_string(),
                "".into(),
                "Chapter 01 Geography".into(),
                en_rtf.to_string_lossy().to_string(),
            ]],
            card1_headers: vec![
                "type".into(),
                "number".into(),
                "title".into(),
                "file".into(),
                "template".into(),
                "title_en".into(),
                "file_en".into(),
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for RXX bilingual should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // Page 1 created (plus initial page 0 = 2 pages)
        assert_eq!(active.doc.page_count(), 2);

        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("BAB 01")), "Should contain Line 1 header 'BAB 01': {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Geografi")), "Should contain Line 2 header 'Geografi': {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Chapter 01 Geography")), "Should contain Line 3 header 'Chapter 01 Geography': {texts:?}");
        assert!(texts.iter().any(|t| t.contains("Kecamatan Karawang Timur")), "Should contain Indonesian body text: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("East Karawang District")), "Should contain English body text: {texts:?}");

        // Verify side-by-side layout: left frame and right frame
        let (si, pi) = active.doc.page_loc(1).unwrap();
        let sp = &active.doc.spreads[si];
        let pg = &sp.pages[pi];
        let margin = pg.margin_rect();

        let mut left_found = false;
        let mut right_found = false;
        let mut en_story_id = None;
        let mut id_story_id = None;

        for it in &sp.items {
            if let Some(tf) = it.text_frame()
                && let Some(story) = active.doc.stories.get(&tf.story)
            {
                let b = it.bounds();
                if story.text.contains("Kecamatan Karawang Timur") {
                    left_found = true;
                    id_story_id = Some(tf.story);
                    // Left column starts at margin.x0
                    assert!((b.x0 - margin.x0).abs() < 2.0, "Left frame x0 ({}) should align with margin.x0 ({})", b.x0, margin.x0);
                    assert!(b.x1 < margin.x1 - 50.0, "Left frame x1 ({}) should be ~half width", b.x1);
                } else if story.text.contains("East Karawang District") {
                    right_found = true;
                    en_story_id = Some(tf.story);
                    // Right column ends at margin.x1
                    assert!((b.x1 - margin.x1).abs() < 2.0, "Right frame x1 ({}) should align with margin.x1 ({})", b.x1, margin.x1);
                    assert!(b.x0 > margin.x0 + 50.0, "Right frame x0 ({}) should start past center", b.x0);
                }
            }
        }

        assert!(left_found, "Should find left column Indonesian text frame");
        assert!(right_found, "Should find right column English text frame");

        // Verify font styling: English text is Italic, Indonesian text is upright
        let en_story = active.doc.stories.get(&en_story_id.unwrap()).unwrap();
        assert!(!en_story.chars.is_empty());
        for run in &en_story.chars {
            let style = run.format.over.font_style.as_deref().unwrap_or("Regular");
            assert!(
                style.eq_ignore_ascii_case("italic") || style.eq_ignore_ascii_case("bold italic"),
                "All English story runs should be italic, got: {style}"
            );
        }

        let id_story = active.doc.stories.get(&id_story_id.unwrap()).unwrap();
        assert!(!id_story.chars.is_empty());
        for run in &id_story.chars {
            let style = run.format.over.font_style.as_deref().unwrap_or("Regular");
            assert!(!style.eq_ignore_ascii_case("italic"), "Indonesian story runs should NOT be converted to italic, got: {style}");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rxx_bilingual_multi_page_continuation() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let dir = std::env::temp_dir().join(format!("pubgen_rxx_multi_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let id_rtf = dir.join("R01.rtf");
        let en_rtf = dir.join("R01_en.rtf");

        // Generate ~40 paragraphs of text to guarantee overset on a half-page column
        let mut id_body = String::from(r#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 "#);
        for i in 0..40 {
            id_body.push_str(&format!(r#"Paragraf narasi bahasa Indonesia nomor {i} mendeskripsikan secara mendalam wilayah administratif dan tata ruang Karawang Timur.\par "#));
        }
        id_body.push('}');
        std::fs::write(&id_rtf, id_body.as_bytes()).unwrap();

        let mut en_body = String::from(r#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}\f0\fs24 "#);
        for i in 0..40 {
            en_body.push_str(&format!(
                r#"English narrative paragraph number {i} describing in detail the spatial planning and governance of East Karawang.\par "#
            ));
        }
        en_body.push('}');
        std::fs::write(&en_rtf, en_body.as_bytes()).unwrap();

        let st = PubgenState {
            card1_preset: 0,
            bilingual_output: true,
            card1_records: vec![vec![
                "Ulasan".into(),
                "01".into(),
                "Bab 01 Geografi".into(),
                id_rtf.to_string_lossy().to_string(),
                "".into(),
                "Chapter 01 Geography".into(),
                en_rtf.to_string_lossy().to_string(),
            ]],
            card1_headers: vec![
                "type".into(),
                "number".into(),
                "title".into(),
                "file".into(),
                "template".into(),
                "title_en".into(),
                "file_en".into(),
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_pages for multi-page RXX should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // Since 40 paragraphs exceed 1 page, total pages must be >= 3 (page 0 blank + page 1 + page 2...)
        assert!(active.doc.page_count() >= 3, "Should create continuation page(s) for overset, got: {}", active.doc.page_count());

        // Check continuation page (Page 2)
        let (si2, pi2) = active.doc.page_loc(2).unwrap();
        let sp2 = &active.doc.spreads[si2];
        let pg2 = &sp2.pages[pi2];
        let m2 = pg2.margin_rect();

        let mut left_cont_found = false;
        let mut right_cont_found = false;

        for it in &sp2.items {
            if active.doc.page_of_item(it.id) != Some(2) {
                continue;
            }
            if let Some(tf) = it.text_frame()
                && let Some(story) = active.doc.stories.get(&tf.story)
            {
                let b = it.bounds();
                if story.text.contains("Paragraf narasi bahasa Indonesia") {
                    left_cont_found = true;
                    // On continuation page, frame starts directly at top margin (no chapter header)
                    assert!((b.y0 - m2.y0).abs() < 2.0, "Continuation left frame y0 ({}) should be at m2.y0 ({})", b.y0, m2.y0);
                    assert!((b.x0 - m2.x0).abs() < 2.0, "Continuation left frame x0 ({}) should align with m2.x0 ({})", b.x0, m2.x0);
                } else if story.text.contains("English narrative paragraph") {
                    right_cont_found = true;
                    assert!((b.y0 - m2.y0).abs() < 2.0, "Continuation right frame y0 ({}) should be at m2.y0 ({})", b.y0, m2.y0);
                    assert!((b.x1 - m2.x1).abs() < 2.0, "Continuation right frame x1 ({}) should align with m2.x1 ({})", b.x1, m2.x1);

                    // Verify English story runs on continuation page remain italic
                    for run in &story.chars {
                        let style = run.format.over.font_style.as_deref().unwrap_or("Regular");
                        assert!(
                            style.eq_ignore_ascii_case("italic") || style.eq_ignore_ascii_case("bold italic"),
                            "English runs on continuation page should be italic, got: {style}"
                        );
                    }
                }
            }
        }

        assert!(left_cont_found, "Should find left continuation frame on Page 2");
        assert!(right_cont_found, "Should find right continuation frame on Page 2");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_table_sheet_note_and_source_rows_extracted_to_textbox() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let matrix = vec![
            vec!["Kecamatan".into(), "Luas".into()],
            vec!["Karawang Timur".into(), "35.2".into()],
            vec!["Catatan".into(), ":".into(), "Desa pada tabel ini termasuk UPT".into()],
            vec!["Note".into(), ":".into(), "Villages in this table include UPT".into()],
            vec!["Sumber".into(), ":".into(), "Badan Pusat Statistik".into()],
            vec!["Source".into(), ":".into(), "BPS-Statistics Indonesia".into()],
        ];

        let st = PubgenState { card2_placement: 1, card2_col_numbers: false, ..Default::default() };
        let res = execute_insert_table_matrix(&mut app, &st, Some(&matrix), Some(0));
        assert!(res.is_ok(), "execute_insert_table_matrix failed: {:?}", res.err());
        let msg = res.unwrap();
        assert!(msg.contains("+ notes"), "Must indicate + notes: {msg}");

        let doc = &app.session.active().unwrap().doc;
        let spread = doc.spreads.first().unwrap();
        // The spread must have the table frame AND the note textbox frame!
        assert_eq!(spread.items.len(), 2, "Must create 2 frames: table frame and note textbox frame");

        // First frame: table frame. The table must have ONLY 2 rows (header + 1 data row)! The 4 note rows are NOT in the table!
        let table_item = &spread.items[0];
        let table_tf = table_item.text_frame().unwrap();
        let table_story = doc.story(table_tf.story).unwrap();
        let table_id = *table_story.tables.keys().next().unwrap();
        let table = table_story.tables.get(&table_id).unwrap();
        assert_eq!(table.nrows(), 2, "Table must have 2 rows, notes must be excluded from table");

        // Second frame: note textbox frame below the table.
        let note_item = &spread.items[1];
        let note_tf = note_item.text_frame().unwrap();
        let note_story = doc.story(note_tf.story).unwrap();
        assert_eq!(note_story.paras.len(), 4, "Note textbox must have 4 paragraphs");

        // Check hanging indent and tab stops on paragraphs:
        for p in &note_story.paras {
            assert!(p.para.left_indent.is_some(), "Must have left_indent");
            assert!(p.para.first_line_indent.is_some(), "Must have negative first_line_indent");
            assert_eq!(p.para.first_line_indent.unwrap(), -p.para.left_indent.unwrap());
            let tabs = p.para.tabs.as_ref().unwrap();
            assert_eq!(tabs.len(), 2, "Must have 2 tab stops: colon and text");
            assert_eq!(tabs[0].position, 46.0, "Colon tab position at 46.0");
            assert_eq!(tabs[1].position, 54.0, "Text tab position at 54.0");
        }

        // Check Italic formatting:
        // Paras: 0=Catatan (Regular), 1=Note (Italic), 2=Sumber (Regular), 3=Source (Italic)
        let ranges = note_story.para_ranges();
        let get_style_at = |pos: usize| note_story.format_after(pos).over.font_style.clone();
        assert_eq!(get_style_at(ranges[0].start), Some("Regular".to_string()));
        assert_eq!(get_style_at(ranges[1].start), Some("Italic".to_string()));
        assert_eq!(get_style_at(ranges[2].start), Some("Regular".to_string()));
        assert_eq!(get_style_at(ranges[3].start), Some("Italic".to_string()));

        // Check textbox height adjusted:
        let note_bounds = note_item.bounds();
        let table_bounds = table_item.bounds();
        assert!(note_bounds.y0 >= table_bounds.y1, "Note textbox must be placed below table");
        assert!(note_bounds.height() >= 18.0 && note_bounds.height() < 120.0, "Textbox height must be fitted to content");
    }

    #[test]
    fn test_brs_template_title_wraps_above_table_and_source_follows() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let tpl = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/templates/data_tables/001_single_table_standard.designcraft");
        let long_title = "Luas Wilayah Menurut Kabupaten dan Kota serta Kepadatan Penduduk di Provinsi Jawa Barat Tahun 2024";
        let res = app
            .run(
                "template.page.apply",
                json!({
                    "path": tpl,
                    "slots": {"text": {"table_num": "Tabel 1.1", "table_title": long_title, "source": "Sumber: BPS Jawa Barat"}}
                }),
            )
            .unwrap();

        let bounds_by_label = |app: &DesignApp| -> Vec<(String, designcraft_geom::Rect)> {
            let active = app.session.active().unwrap();
            let mut out = Vec::new();
            for sp in &active.doc.spreads {
                for it in &sp.items {
                    if !it.label.is_empty() {
                        out.push((it.label.clone(), it.bounds()));
                    }
                }
            }
            out
        };

        let title_h_before = bounds_by_label(&app).iter().find(|(l, _)| l == "#slot:text:table_title").map(|(_, b)| b.height()).unwrap();

        let st = PubgenState::default();
        let doc_margins = designcraft_doc::Margins::uniform(36.0);
        let _table_slot = post_process_template_applied_page(&mut app, &st, &res, false, doc_margins, false, long_title, "Tabel");

        let matrix = vec![
            vec!["Kecamatan".to_string(), "Luas".to_string()],
            vec!["Karawang Timur".to_string(), "35.2".to_string()],
            vec!["Sumber".to_string(), ":".to_string(), "BPS".to_string()],
        ];
        let cur = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
        let ins = execute_insert_table_matrix(&mut app, &st, Some(&matrix), Some(cur));
        assert!(ins.is_ok(), "execute_insert_table_matrix failed: {:?}", ins.err());

        let by_label = bounds_by_label(&app);
        let title_b = by_label.iter().find(|(l, _)| l == "#slot:text:table_title").map(|(_, b)| *b).unwrap();
        let src_b = by_label.iter().find(|(l, _)| l == "#slot:text:source").map(|(_, b)| *b).unwrap();
        let active = app.session.active().unwrap();
        let tbl_b = active
            .doc
            .spreads
            .iter()
            .flat_map(|sp| sp.items.iter())
            .find(|it| it.text_frame().is_some_and(|tf| active.doc.story(tf.story).is_some_and(|s| !s.tables.is_empty())))
            .map(|it| it.bounds())
            .unwrap();

        // The wrapped title grew its frame instead of spilling over the table.
        assert!(title_b.height() > title_h_before + 2.0, "Title frame must grow when the title wraps: {} -> {}", title_h_before, title_b.height());
        assert!(title_b.y1 < tbl_b.y0, "Wrapped title must sit above the table: title y1 {} vs table y0 {}", title_b.y1, tbl_b.y0);
        // The template's fixed source slot slid below the inserted table + footnote.
        assert!(src_b.y0 >= tbl_b.y1, "Source slot must follow the table: src y0 {} vs table y1 {}", src_b.y0, tbl_b.y1);
        assert!(src_b.y0 < 530.0, "Source slot must move up under a short table, still at {}", src_b.y0);
    }

    #[test]
    fn test_vector_chart_sheet_renders_vector_and_skips_raster() {
        let dir = tempfile::tempdir().unwrap();
        let chart_csv_path = dir.path().join("C02_01.csv");
        let chart_content = "chart_type;bar_clustered\n\
title;Grafik 02.01 Penduduk Menurut Desa\n\
title_en;Chart 02.01 Population by Village\n\
category_col;A\n\
series_1_col;B\n\
series_1_label;Laki-laki\n\
series_1_label_en;Male\n\
data_start_row;9\n\
data_end_row;10\n\
Desa A;100\n\
Desa B;200\n";
        std::fs::write(&chart_csv_path, chart_content).unwrap();

        let catalog_path = dir.path().join("catalog.csv");
        let catalog_content = "type,number,title,file\n\
Grafik,02.01,Grafik 02.01 Penduduk Menurut Desa,C02_01.csv\n";
        std::fs::write(&catalog_path, catalog_content).unwrap();

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let st = PubgenState {
            card1_path: catalog_path.to_string_lossy().to_string(),
            card1_records: vec![vec!["Grafik".into(), "02.01".into(), "Grafik 02.01 Penduduk Menurut Desa".into(), "C02_01.csv".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_file_col: 3,
            card1_range_all: true,
            bilingual_output: false,
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st).unwrap();
        assert!(res.contains("1 vector chart(s) rendered"), "Summary should report vector chart: {res}");

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.last().unwrap();
        // Emitted chart is grouped
        let has_group = spread.items.iter().any(|it| matches!(it.content, designcraft_doc::Content::Group { .. }));
        assert!(has_group, "Vector chart must be emitted and grouped on the page");
    }

    #[test]
    fn test_raster_fallback_used_when_no_chart_type_row() {
        let dir = tempfile::tempdir().unwrap();
        let img_path = dir.path().join("chart.png");
        // Minimal 1x1 png
        let png_bytes = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01,
            0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&img_path, png_bytes).unwrap();

        let catalog_path = dir.path().join("catalog.csv");
        let catalog_content = "type,number,title,file\n\
Grafik,02.01,Grafik 02.01 Image Only,chart.png\n";
        std::fs::write(&catalog_path, catalog_content).unwrap();

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let st = PubgenState {
            card1_path: catalog_path.to_string_lossy().to_string(),
            card1_records: vec![vec!["Grafik".into(), "02.01".into(), "Grafik 02.01 Image Only".into(), "chart.png".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_file_col: 3,
            card1_range_all: true,
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st).unwrap();
        assert!(res.contains("1 chart image(s) placed"), "Summary should report raster image placed: {res}");
    }

    #[test]
    fn test_vector_chart_appears_in_list_of_charts_toc() {
        let dir = tempfile::tempdir().unwrap();
        let chart_csv_path = dir.path().join("C01_01.csv");
        let chart_content = "chart_type;bar_clustered\n\
title;Grafik 01.01 Pertumbuhan Ekonomi\n\
category_col;A\n\
series_1_col;B\n\
series_1_label;Nilai\n\
data_start_row;7\n\
data_end_row;8\n\
2024;5.2\n\
2025;5.5\n";
        std::fs::write(&chart_csv_path, chart_content).unwrap();

        let catalog_path = dir.path().join("catalog.csv");
        let catalog_content = "type,number,title,file\n\
Grafik,01.01,Grafik 01.01 Pertumbuhan Ekonomi,C01_01.csv\n";
        std::fs::write(&catalog_path, catalog_content).unwrap();

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let mut st = PubgenState {
            card1_path: catalog_path.to_string_lossy().to_string(),
            card1_records: vec![vec!["Grafik".into(), "01.01".into(), "Grafik 01.01 Pertumbuhan Ekonomi".into(), "C01_01.csv".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_file_col: 3,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            toc_kind: 2, // List of Charts (Daftar Grafik)
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();

        // Now generate TOC kind 2 (Daftar Grafik)
        let toc_res = super::toc::execute_add_toc(&mut app, &mut st);
        assert!(toc_res.is_ok(), "execute_add_toc should succeed: {toc_res:?}");

        let active = app.session.active().unwrap();
        let has_chart_entry = active.doc.stories.values().any(|s| s.text.contains("Pertumbuhan Ekonomi") || s.text.contains("01.01"));
        assert!(has_chart_entry, "List of Charts TOC should contain the vector chart entry");
    }

    #[test]
    fn test_bilingual_legend_labels_have_italic_english() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let spec = super::charts::spec::ChartSpec {
            chart_type: super::charts::spec::ChartType::BarClustered,
            title_id: "Grafik Penduduk".into(),
            title_en: "Population Chart".into(),
            category_col: "A".into(),
            series: vec![
                super::charts::spec::SeriesSpec { col: "B".into(), label_id: "Laki-laki".into(), label_en: "Male".into() },
                super::charts::spec::SeriesSpec { col: "C".into(), label_id: "Perempuan".into(), label_en: "Female".into() },
            ],
            data_start_row: 1,
            data_end_row: 2,
            value_format: super::charts::spec::ValueFormat::Number,
            show_value_labels: false,
            show_legend: true,
            show_grid: false,
            max_value_override: None,
            geojson_path: None,
            map_palette_idx: 0,
            include_data_table: false,
        };
        let categories = vec!["Desa A".into()];
        let values = vec![vec![100.0, 80.0]];
        let margin = designcraft_geom::Rect { x0: 36.0, y0: 36.0, x1: 500.0, y1: 700.0 };

        let layout = super::charts::layout::compute_layout(&spec, &categories, &values, margin, true).unwrap();
        super::charts::emitter::emit_chart(&mut app, 0, &spec, &layout, &[], true, "Lato").unwrap();

        let active = app.session.active().unwrap();
        let leg_story = active.doc.stories.values().find(|s| s.text.contains("Laki-laki / Male")).expect("Legend story must exist");
        let en_pos = leg_story.text.find("Male").unwrap();
        let en_fmt = leg_story.char_format_of(en_pos);
        assert_eq!(en_fmt.over.font_style.as_deref(), Some("Italic"), "English legend part must be Italic");
    }

    #[test]
    fn test_long_x_labels_are_rotated() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let spec = super::charts::spec::ChartSpec {
            chart_type: super::charts::spec::ChartType::BarClustered,
            title_id: "Grafik".into(),
            title_en: "".into(),
            category_col: "A".into(),
            series: vec![super::charts::spec::SeriesSpec { col: "B".into(), label_id: "Val".into(), label_en: "".into() }],
            data_start_row: 1,
            data_end_row: 2,
            value_format: super::charts::spec::ValueFormat::Number,
            show_value_labels: false,
            show_legend: false,
            show_grid: false,
            max_value_override: None,
            geojson_path: None,
            map_palette_idx: 0,
            include_data_table: false,
        };
        let categories = vec!["Desa Karanganyar Subang".into()]; // > 12 chars
        let values = vec![vec![100.0]];
        let margin = designcraft_geom::Rect { x0: 36.0, y0: 36.0, x1: 500.0, y1: 700.0 };

        let layout = super::charts::layout::compute_layout(&spec, &categories, &values, margin, false).unwrap();
        assert!(layout.x_tick_labels[0].rotated, "Label longer than 12 chars must have rotated flag");

        let _ = super::charts::emitter::emit_chart(&mut app, 0, &spec, &layout, &[], false, "Lato").unwrap();
        let active = app.session.active().unwrap();
        // Item should have rotation applied
        let mut all_items: Vec<std::sync::Arc<designcraft_doc::Item>> = Vec::new();
        for it in &active.doc.spreads[0].items {
            all_items.push(it.clone());
            if let designcraft_doc::Content::Group { items } = &it.content {
                all_items.extend(items.iter().cloned());
            }
        }
        let rotated_frame = all_items.iter().any(|it| designcraft_geom::decompose::decompose(it.xf).rotation.abs() > 1.0);
        assert!(rotated_frame, "Rotated X-tick frame should have non-zero rotation transform");
    }

    #[test]
    fn test_all_error_paths_fall_back_without_panic() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        // Empty data sheet
        let dir = tempfile::tempdir().unwrap();
        let bad_csv_path = dir.path().join("bad.csv");
        std::fs::write(&bad_csv_path, "chart_type;bar_clustered\ncategory_col;A\nseries_1_col;B\ndata_start_row;5\ndata_end_row;3\n").unwrap();

        let catalog_path = dir.path().join("catalog.csv");
        std::fs::write(&catalog_path, "type,number,title,file\nGrafik,01.01,Broken Chart,bad.csv\n").unwrap();

        let st = PubgenState {
            card1_path: catalog_path.to_string_lossy().to_string(),
            card1_records: vec![vec!["Grafik".into(), "01.01".into(), "Broken Chart".into(), "bad.csv".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_file_col: 3,
            card1_range_all: true,
            ..Default::default()
        };

        // Must succeed without panic
        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "Error in chart data must cleanly fall back: {res:?}");
    }

    #[test]
    fn test_mxx_map_chart_generation_and_three_line_title() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let maps_dir = dir.path().join("maps");
        std::fs::create_dir_all(&maps_dir).unwrap();
        let data_csv_path = maps_dir.join("contoh.csv");
        let data_csv = "nmdesa;title;title_en;value\nSUKATANI;Grafik 03.02 Perbandingan Jumlah Penduduk Kecamatan Cilamaya Wetan, 2025;Figure 03.02 Comparison of Population Size in Cilamaya Wetan District, 2025;19561\nMEKARMAYA;Grafik 03.02 Perbandingan Jumlah Penduduk Kecamatan Cilamaya Wetan, 2025;Figure 03.02 Comparison of Population Size in Cilamaya Wetan District, 2025;22014\nMUARABARU;Grafik 03.02 Perbandingan Jumlah Penduduk Kecamatan Cilamaya Wetan, 2025;Figure 03.02 Comparison of Population Size in Cilamaya Wetan District, 2025;18277\nSUKAKERTA;Grafik 03.02 Perbandingan Jumlah Penduduk Kecamatan Cilamaya Wetan, 2025;Figure 03.02 Comparison of Population Size in Cilamaya Wetan District, 2025;15579\nTEGALWARU;Grafik 03.02 Perbandingan Jumlah Penduduk Kecamatan Cilamaya Wetan, 2025;Figure 03.02 Comparison of Population Size in Cilamaya Wetan District, 2025;7442\nTEGALSARI;Grafik 03.02 Perbandingan Jumlah Penduduk Kecamatan Cilamaya Wetan, 2025;Figure 03.02 Comparison of Population Size in Cilamaya Wetan District, 2025;15905\n";
        std::fs::write(&data_csv_path, data_csv).unwrap();

        let m_sheet_csv = dir.path().join("M03_02.csv");
        std::fs::write(&m_sheet_csv, "map;CILAMAYA WETAN\ndata;maps\\contoh.csv\n").unwrap();

        let catalog_path = dir.path().join("catalog.csv");
        let cat_csv = "type;number;title;file;title_en\nGrafik;03.02;Grafik 03.02 Perbandingan Jumlah Penduduk Kecamatan Cilamaya Wetan, 2025;M03_02.csv;Figure 03.02 Comparison of Population Size in Cilamaya Wetan District, 2025\n";
        std::fs::write(&catalog_path, cat_csv).unwrap();

        let st = PubgenState {
            card1_path: catalog_path.to_string_lossy().to_string(),
            card1_records: vec![vec![
                "Grafik".into(),
                "03.02".into(),
                "Grafik 03.02 Perbandingan Jumlah Penduduk Kecamatan Cilamaya Wetan, 2025".into(),
                "M03_02.csv".into(),
                "".into(),                                                                            // template
                "Figure 03.02 Comparison of Population Size in Cilamaya Wetan District, 2025".into(), // title_en
            ]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into(), "template".into(), "title_en".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_file_col: 3,
            card1_template_col: 4,
            card1_title_en_col: 5,
            card1_range_all: true,
            bilingual_output: true,
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "Map chart generation should succeed: {res:?}");
        let summary = res.unwrap();
        assert!(summary.contains("1 vector chart(s) rendered"), "Summary should report vector map chart rendered: {summary}");

        let active = app.session.active().unwrap();
        // Check page title 3 lines
        let title_story = active.doc.stories.values().find(|s| s.text.contains("Grafik 03.02")).expect("Title story must exist");
        let lines: Vec<&str> = title_story.text.split(designcraft_doc::story::FORCED_LINE_BREAK).collect();
        assert_eq!(lines.len(), 3, "Title must have exactly 3 lines: {:?}", lines);
        assert_eq!(lines[0], "Grafik 03.02");
        assert_eq!(lines[1], "Perbandingan Jumlah Penduduk Kecamatan Cilamaya Wetan, 2025");
        assert_eq!(lines[2], "Figure 03.02 Comparison of Population Size in Cilamaya Wetan District, 2025");

        // Verify English title (line 3) is italicized
        let en_line_text = lines[2];
        let en_start = title_story.text.find(en_line_text).expect("EN line must be in text");
        let en_fmt = title_story.char_format_of(en_start + 2);
        assert_eq!(en_fmt.over.font_style.as_deref(), Some("Italic"), "English line must be formatted Italic");

        // Verify village map label (line 1 nmdesa, line 2 value)
        let sukatani_story = active.doc.stories.values().find(|s| s.text.contains("SUKATANI")).expect("Sukatani map label must exist");
        let s_text = sukatani_story.text.replace(designcraft_doc::story::FORCED_LINE_BREAK, "\n");
        let s_lines: Vec<&str> = s_text.lines().collect();
        assert_eq!(s_lines[0], "SUKATANI", "Line 1 of map label must be nmdesa: {:?}", s_lines);
        assert_eq!(s_lines[1], "19.561", "Line 2 of map label must be formatted value: {:?}", s_lines);
    }

    #[test]
    fn test_brs_two_multipage_tables_thread_consecutively_without_jumping_or_overlapping() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"facingPages": false})).unwrap();

        let dir = tempfile::tempdir().unwrap();

        // Generate BRS front cover, point page, and back cover first
        let st_cover = PubgenState {
            active_profile_id: "bps_brs".into(),
            cover_style: 13,
            ..Default::default()
        };
        super::covers::execute_generate_brs_cover(&mut app, &st_cover).unwrap();
        let pages_with_covers = app.session.active().unwrap().doc.page_count();
        assert!(pages_with_covers >= 3);
        let back_cover_page_initial = super::covers::find_back_cover_page(&app.session.active().unwrap().doc).unwrap();
        assert_eq!(back_cover_page_initial, pages_with_covers - 1);

        // Create 2 CSV tables with 150 rows each (so each table oversets a single page)
        let t1_path = dir.path().join("t1.csv");
        let mut t1_csv = String::from("Kecamatan;Luas\n");
        for i in 0..150 {
            t1_csv.push_str(&format!("Kecamatan {i};{}\n", i * 10));
        }
        std::fs::write(&t1_path, t1_csv).unwrap();

        let t2_path = dir.path().join("t2.csv");
        let mut t2_csv = String::from("Komoditas;Produksi\n");
        for i in 0..150 {
            t2_csv.push_str(&format!("Komoditas {i};{}\n", i * 25));
        }
        std::fs::write(&t2_path, t2_csv).unwrap();

        // Catalog: Narrative 1, Table 1 (overset), Narrative 2, Table 2 (overset), Narrative 3
        let cat_path = dir.path().join("catalog.csv");
        let cat_csv = "type;number;title;file\n\
Narrative;1;Perkembangan Perekonomian;narrative1.rtf\n\
Tabel;1;Tabel 1 Luas Wilayah;t1.csv\n\
Narrative;2;Perkembangan Produksi;narrative2.rtf\n\
Tabel;2;Tabel 2 Produksi Komoditas;t2.csv\n\
Narrative;3;Catatan Akhir;narrative3.rtf\n";
        std::fs::write(&cat_path, cat_csv).unwrap();

        let n1_path = dir.path().join("narrative1.rtf");
        std::fs::write(&n1_path, r#"{\rtf1\ansi Paragraph 1}"#).unwrap();
        let n2_path = dir.path().join("narrative2.rtf");
        std::fs::write(&n2_path, r#"{\rtf1\ansi Paragraph 2}"#).unwrap();
        let n3_path = dir.path().join("narrative3.rtf");
        std::fs::write(&n3_path, r#"{\rtf1\ansi Paragraph 3}"#).unwrap();

        let st = PubgenState {
            active_profile_id: "bps_brs".into(),
            card1_path: cat_path.to_string_lossy().to_string(),
            card1_records: vec![
                vec!["Narrative".into(), "1".into(), "Perkembangan Perekonomian".into(), "narrative1.rtf".into()],
                vec!["Tabel".into(), "1".into(), "Tabel 1 Luas Wilayah".into(), "t1.csv".into()],
                vec!["Narrative".into(), "2".into(), "Perkembangan Produksi".into(), "narrative2.rtf".into()],
                vec!["Tabel".into(), "2".into(), "Tabel 2 Produksi Komoditas".into(), "t2.csv".into()],
                vec!["Narrative".into(), "3".into(), "Catatan Akhir".into(), "narrative3.rtf".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_file_col: 3,
            card1_range_all: true,
            card1_auto_insert_tables: true,
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "Page generation failed: {res:?}");

        let active = app.session.active().unwrap();
        let doc = &active.doc;

        // Locate Table 1 and Table 2 stories and their frames
        let mut t1_story_id = None;
        let mut t2_story_id = None;
        for (sid, story) in &doc.stories {
            for table in story.tables.values() {
                let cell_0 = table.cell(0, 0).map(|c| c.text.text.clone());
                let cell_1 = table.cell(1, 0).map(|c| c.text.text.clone());
                if cell_0.as_deref().unwrap_or("").contains("Kecamatan") || cell_1.as_deref().unwrap_or("").contains("Kecamatan") {
                    t1_story_id = Some(*sid);
                } else if cell_0.as_deref().unwrap_or("").contains("Komoditas") || cell_1.as_deref().unwrap_or("").contains("Komoditas") {
                    t2_story_id = Some(*sid);
                }
            }
        }

        let sid1 = t1_story_id.expect("Table 1 story must exist");
        let sid2 = t2_story_id.expect("Table 2 story must exist");

        let frames1 = &doc.story(sid1).unwrap().frames;
        let frames2 = &doc.story(sid2).unwrap().frames;

        assert!(frames1.len() >= 2, "Table 1 must have at least 2 threaded frames, got {}", frames1.len());
        assert!(frames2.len() >= 2, "Table 2 must have at least 2 threaded frames, got {}", frames2.len());

        let page_of = |fid: designcraft_doc::ItemId| -> usize {
            doc.page_of_item(fid).unwrap()
        };

        let p1_start = page_of(frames1[0]);
        let p1_cont = page_of(frames1[1]);
        let p2_start = page_of(frames2[0]);
        let p2_cont = page_of(frames2[1]);

        // Table 1 continuation must be on the page immediately following its start!
        assert_eq!(p1_cont, p1_start + 1, "Table 1 continuation must be immediately after start page: start={}, cont={}", p1_start, p1_cont);

        // Table 2 must start AFTER Table 1 has finished!
        assert!(p2_start > p1_cont, "Table 2 must start after Table 1 continuation: t1_cont={}, t2_start={}", p1_cont, p2_start);

        // Table 2 continuation must be immediately after Table 2's start page!
        assert_eq!(p2_cont, p2_start + 1, "Table 2 continuation must be immediately after Table 2 start: start={}, cont={}", p2_start, p2_cont);

        // Table 1 and Table 2 must NOT overlap on the same continuation page!
        assert_ne!(p1_cont, p2_cont, "Table 1 continuation and Table 2 continuation must NOT be on the same page!");

        // Back cover must still be the last page!
        let total_pages = doc.page_count();
        let back_page = super::covers::find_back_cover_page(doc).expect("Back cover must exist");
        assert_eq!(back_page, total_pages - 1, "Back cover must be the very last page");
    }
}
