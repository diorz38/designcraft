//! Cross-domain document/frame/story helpers shared by the Pubgen engines.

use super::*;

pub(crate) fn target_page_index(app: &DesignApp) -> usize {
    if let Some(cur) = crate::canvas::current_page(app) {
        return cur;
    }
    if let Some(st) = app.session.active() {
        let sel_id = st.selection.items.first().copied().or_else(|| st.selection.text.and_then(|t| t.frame));
        if let Some(id) = sel_id {
            if let Some(p) = st.doc.page_of_item(id) {
                return p;
            }
            if let Some(loc) = st.doc.find(id)
                && let designcraft_doc::SpreadRef::Doc(si) = loc.spread
            {
                return st.doc.first_page_of_spread(si);
            }
        }
        st.doc.page_count().saturating_sub(1)
    } else {
        0
    }
}

/// Create the paragraph style `name` if the document does not already have it, so `toc.generate`
/// (which requires an existing style) can collect the "TOC Line" paragraphs.
pub(crate) fn ensure_para_style(app: &mut DesignApp, name: &str) {
    let exists = app
        .session
        .execute("style.list", &json!({}))
        .ok()
        .and_then(|v| v.get("paragraph").and_then(Value::as_array).map(|a| a.iter().any(|s| s.as_str() == Some(name))))
        .unwrap_or(false);
    if !exists {
        let _ = app.run("style.paragraph.create", json!({"name": name}));
    }
}

/// How many of a page's opening paragraphs the title search weighs.
const TITLE_SCAN: usize = 12;

/// The paragraph that reads as the page's title: the largest centred line among the page's opening
/// paragraphs (the first one on a tie).
///
/// A `Note`/`G` sheet gives no title, so the page opens with the RTF's own heading. Word writes
/// that heading as a `Heading n`, as the `Pendahuluan` style or as direct formatting (bold,
/// capitals, coloured, centred) from sheet to sheet, so the search goes by how it looks rather
/// than by its name. On the compilers' page that leaves out the ISSN line above it (right-aligned)
/// and the publication's title below it (set smaller), and takes "TIM PENYUSUN/COMPILERS".
pub(crate) fn page_title_index(doc: &designcraft_doc::Document, sid: designcraft_doc::StoryId) -> Option<usize> {
    let story = doc.story(sid)?;
    let mut seen = 0;
    let mut sized: Vec<(usize, f64, bool)> = Vec::new(); // paragraph, text size, centred
    for (i, r) in story.para_ranges().iter().enumerate() {
        if !story.text.get(r.clone()).is_some_and(|t| !t.trim().is_empty()) {
            continue;
        }
        seen += 1;
        if seen > TITLE_SCAN {
            break;
        }
        let mut size: Option<f64> = None;
        for (run, format) in story.runs() {
            if run.start >= r.end {
                break;
            }
            if run.end <= r.start {
                continue;
            }
            if let Some(s) = format.over.size {
                size = Some(size.map_or(s, |m: f64| m.max(s)));
            }
        }
        let size = size.or_else(|| story.paras.get(i).and_then(|p| doc.styles.para(&p.style)).and_then(|s| s.chars.size)).unwrap_or(0.0);
        let centred = story.paras.get(i).is_some_and(|p| p.para.align == Some(designcraft_doc::Align::Center));
        sized.push((i, size, centred));
    }
    let largest = |centred_only: bool| {
        sized
            .iter()
            .filter(|(_, _, centred)| !centred_only || *centred)
            .fold(None::<(usize, f64)>, |best, &(i, size, _)| match best {
                Some((_, b)) if b >= size => best,
                _ => Some((i, size)),
            })
            .map(|(i, _)| i)
    };
    largest(true).or_else(|| largest(false))
}

/// Tag the paragraph that reads as a title-less `Note`/`G` page's title with `style`, so the
/// paragraph table of contents names the page. Only the style changes: the RTF's own centring,
/// boldness and size stay on the paragraph.
pub(crate) fn tag_page_title(app: &mut DesignApp, sid: designcraft_doc::StoryId, style: &str) {
    ensure_para_style(app, style);
    let _ = app.session.edit(|doc, _| {
        if let Some(i) = page_title_index(doc, sid)
            && let Some(story) = doc.story_mut(sid)
            && let Some(r) = story.para_ranges().get(i).cloned()
        {
            story.format_paras(r.start..r.start, |f| f.style = style.to_string());
        }
        Ok(serde_json::Value::Null)
    });
}

/// The story of the text frame `fid`, as the document has it now.
pub(crate) fn story_of_frame(app: &DesignApp, fid: u64) -> Option<designcraft_doc::StoryId> {
    let active = app.session.active()?;
    let id = designcraft_doc::ItemId(fid);
    let item = active.doc.spreads.iter().flat_map(|s| s.items.iter()).find(|i| i.id == id)?;
    item.text_frame().map(|tf| tf.story)
}

/// Create a text frame and tag its paragraph with the TOC paragraph style, so `toc.generate`
/// lists the frame's text as an entry (one entry per generated page).
pub(crate) fn frame_with_style(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], text: &str, style: &str) -> Option<u64> {
    let res = app.run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "text", "text": text, "caret": false})).ok()?;
    let story = res.get("story").and_then(Value::as_u64)?;
    let end = text.len().max(1) as u64;
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    let _ = app.run("style.paragraph.apply", json!({"name": style}));
    Some(story)
}

/// Create a text frame with specific font family, optional font style, font size, and optional paragraph style.
pub(crate) fn frame_with_font_full(
    app: &mut DesignApp,
    spread_idx: usize,
    rect: [f64; 4],
    text: &str,
    font_family: &str,
    font_style: Option<&str>,
    font_size: f64,
    style_opt: Option<&str>,
) -> Option<u64> {
    let res = app.run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "text", "text": text, "caret": false})).ok()?;
    let story = res.get("story").and_then(Value::as_u64)?;
    let end = text.len().max(1) as u64;
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    if let Some(style) = style_opt {
        let _ = app.run("style.paragraph.apply", json!({"name": style}));
    }
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    let mut type_char_params = json!({"fontFamily": font_family, "size": font_size});
    if let Some(fs) = font_style {
        type_char_params["fontStyle"] = json!(fs);
    }
    let _ = app.run("type.char", type_char_params);
    Some(story)
}

/// Create a text frame with specific font family, font size, and optional paragraph style.
pub(crate) fn frame_with_font_and_style(
    app: &mut DesignApp,
    spread_idx: usize,
    rect: [f64; 4],
    text: &str,
    font_family: &str,
    font_size: f64,
    style_opt: Option<&str>,
) -> Option<u64> {
    frame_with_font_full(app, spread_idx, rect, text, font_family, None, font_size, style_opt)
}

/// Check if a story is overset.
pub(crate) fn story_is_overset(app: &DesignApp, sid: designcraft_doc::StoryId) -> bool {
    if let Some(active) = app.session.active() {
        designcraft_compose::compose_story(&active.doc, sid, &Default::default()).overset_at.is_some()
    } else {
        false
    }
}

/// Set all character runs in the given story to Italic (or Bold Italic if bold).
pub(crate) fn italicize_story(app: &mut DesignApp, story_id: u64) {
    let _ = app.session.edit(|doc, _| {
        if let Some(story) = doc.story_mut(designcraft_doc::StoryId(story_id)) {
            for run in &mut story.chars {
                let cur = run.format.over.font_style.as_deref().unwrap_or("Regular");
                let next = if cur.eq_ignore_ascii_case("bold") || cur.eq_ignore_ascii_case("bold italic") { "Bold Italic" } else { "Italic" };
                run.format.over.font_style = Some(next.to_string());
            }
        }
        Ok(serde_json::Value::Null)
    });
}

/// Set all character runs in all stories belonging to the given page range to Italic (or Bold Italic if bold).
pub(crate) fn italicize_page_stories(app: &mut DesignApp, start_page: usize, end_page: usize) {
    let _ = app.session.edit(|doc, _| {
        for pi in start_page..=end_page {
            if let Some((si, _)) = doc.page_loc(pi)
                && let Some(spread) = doc.spreads.get(si)
            {
                let story_ids: Vec<_> = spread
                    .items
                    .iter()
                    .filter(|it| doc.page_of_item(it.id) == Some(pi))
                    .filter_map(|it| it.text_frame())
                    .map(|tf| tf.story)
                    .collect();
                for sid in story_ids {
                    if let Some(story) = doc.story_mut(sid) {
                        for run in &mut story.chars {
                            let cur = run.format.over.font_style.as_deref().unwrap_or("Regular");
                            let next =
                                if cur.eq_ignore_ascii_case("bold") || cur.eq_ignore_ascii_case("bold italic") { "Bold Italic" } else { "Italic" };
                            run.format.over.font_style = Some(next.to_string());
                        }
                    }
                }
            }
        }
        Ok(serde_json::Value::Null)
    });
}

/// Helper to place a catalog image (e.g. "image1", "image2") on `spread_idx` with `rect` and `wrap`.
pub(crate) fn place_catalog_image(
    app: &mut DesignApp,
    st: &PubgenState,
    row: &[String],
    key: &str,
    spread_idx: usize,
    rect: [f64; 4],
    wrap: &str,
) -> Option<u64> {
    let col = |key: &str| -> Option<String> {
        let idx = st.card1_headers.iter().position(|h| h.trim().eq_ignore_ascii_case(key))?;
        let v = row.get(idx).map(|s| s.trim()).filter(|s| !s.is_empty())?;
        Some(v.to_string())
    };
    let file_str = col(key)?;
    let img = resolve_image_file(&st.card1_path, &file_str, &["png", "jpg", "jpeg"])?;
    let res = app.run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "graphic"})).ok()?;
    let fid = res.get("id").and_then(Value::as_u64)?;
    let _ = app.run("selection.set", json!({"ids": [fid]}));
    let _ = app.run("file.place", json!({"path": img.to_string_lossy(), "frame": fid}));
    let _ = app.run("object.fit", json!({"mode": "fitProportionally", "ids": [fid]}));
    if wrap == "none" {
        let _ = app.run("object.textWrap", json!({"mode": "none", "ids": [fid]}));
        let _ = app.run("object.textWrap", json!({"mode": "none", "content": true, "ids": [fid]}));
        let _ = app.session.edit(|doc, _| {
            if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
                it.wrap.mode = designcraft_doc::WrapMode::None;
                if let designcraft_doc::Content::Graphic(g) = &mut it.content {
                    g.wrap.mode = designcraft_doc::WrapMode::None;
                }
            }
            Ok(serde_json::Value::Null)
        });
    } else {
        let _ = app.run("object.textWrap", json!({"mode": wrap, "offset": 6.0, "ids": [fid]}));
    }
    Some(fid)
}

/// Harmonizes indentation and alignment of the signatory block at the end of narrative text
/// (e.g. Kata Pengantar / Preface) so that all signatory lines (date, office, title, name)
/// share identical center alignment and are not placed too close to the page right edge.
pub(crate) fn harmonize_signatory_block(app: &mut DesignApp, fid: u64, margin: designcraft_geom::Rect) {
    let Some(sid) = story_of_frame(app, fid) else {
        return;
    };
    let mw = (margin.x1 - margin.x0).max(100.0);
    // Position the signatory block in the right-hand portion of the page with breathing room
    // from the right margin (at least 36pt buffer from the page right margin).
    let sig_left_indent = (mw * 0.44).round();
    let sig_right_indent = 36.0_f64;

    let _ = app.session.edit(|doc, _| {
        let Some(story) = doc.story_mut(sid) else {
            return Ok(serde_json::Value::Null);
        };
        let ranges = story.para_ranges();
        if ranges.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        // Find the last non-empty paragraph (signatory name line).
        let mut last_non_empty = None;
        for (i, r) in ranges.iter().enumerate().rev() {
            let t = story.text[r.clone()].trim();
            if !t.is_empty() {
                last_non_empty = Some(i);
                break;
            }
        }
        let Some(last_pi) = last_non_empty else {
            return Ok(serde_json::Value::Null);
        };

        // Walk backwards to find the start of the signatory block.
        // Identify the first non-empty signatory line (the date line like "Karawang, September 2025").
        let mut first_sig_non_empty = last_pi;
        for i in (0..last_pi).rev() {
            let t = story.text[ranges[i].clone()].trim();
            if t.is_empty() {
                continue;
            }
            let is_sig = t.contains("Karawang")
                || t.contains("Kepala")
                || t.contains("Head of")
                || t.contains("Regency")
                || t.contains("Office")
                || t.contains("BPS")
                || (t.contains(',') && (t.contains("202") || t.contains("203")))
                || story.paras[i].para.left_indent.unwrap_or(0.0) > 80.0;
            if is_sig {
                first_sig_non_empty = i;
            } else {
                break;
            }
        }
        let start_pi = first_sig_non_empty;

        for pi in (start_pi..=last_pi).rev() {
            let r = story.para_ranges()[pi].clone();
            let p_text = &story.text[r.clone()];
            let trim_head = p_text.chars().take_while(|c| *c == '\t' || *c == ' ').map(|c| c.len_utf8()).sum::<usize>();
            if trim_head > 0 {
                story.delete(r.start..r.start + trim_head);
            }
            let r_after = story.para_ranges()[pi].clone();
            let p_text_after = &story.text[r_after.clone()];
            let trim_tail = p_text_after.chars().rev().take_while(|c| *c == '\t' || *c == ' ').map(|c| c.len_utf8()).sum::<usize>();
            if trim_tail > 0 && trim_tail < p_text_after.len() {
                story.delete(r_after.end - trim_tail..r_after.end);
            }
        }

        for (pi, p) in story.paras.iter_mut().enumerate() {
            if pi >= start_pi && pi <= last_pi {
                p.para.align = Some(designcraft_doc::Align::Center);
                p.para.left_indent = Some(sig_left_indent);
                p.para.right_indent = Some(sig_right_indent);
                p.para.first_line_indent = Some(0.0);
                p.para.last_line_indent = Some(0.0);
                p.para.tabs = None;
                if pi == start_pi {
                    let cur_sb = p.para.space_before.unwrap_or(0.0);
                    p.para.space_before = Some(cur_sb.max(16.0));
                }
                if pi == last_pi {
                    let cur_sb = p.para.space_before.unwrap_or(0.0);
                    p.para.space_before = Some(cur_sb.max(42.0));
                }
            }
        }

        Ok(serde_json::Value::Null)
    });
}

/// Computes the dynamic placement rect for the signature (`image2`) anchored to the signatory
/// block at the end of the narrative text (e.g. Kata Pengantar / Preface).
/// Returns `(target_spread_idx, [x0, y0, x1, y1])`.
pub(crate) fn compute_signature_placement(
    app: &DesignApp,
    fid: u64,
    default_rect: [f64; 4],
    default_spread: usize,
    margin: designcraft_geom::Rect,
) -> (usize, [f64; 4]) {
    let Some(sid) = story_of_frame(app, fid) else {
        return (default_spread, default_rect);
    };
    let Some(active) = app.session.active() else {
        return (default_spread, default_rect);
    };
    let cs = designcraft_compose::compose_story(&active.doc, sid, &Default::default());

    // Find the last frame containing non-whitespace text lines.
    let mut found = None;
    for frame in cs.frames.iter().rev() {
        let non_empty: Vec<&designcraft_compose::Line> =
            frame.lines.iter().filter(|l| l.glyphs.iter().any(|g| g.visible && !g.rendered_char.is_whitespace())).collect();
        if !non_empty.is_empty() {
            let last_line = non_empty.last().copied();
            let prev_line = if non_empty.len() >= 2 { non_empty.get(non_empty.len().saturating_sub(2)).copied() } else { None };
            if let Some(ll) = last_line {
                found = Some((frame.frame, ll, prev_line));
                break;
            }
        }
    }

    let Some((target_frame_id, last_line, prev_line)) = found else {
        return (default_spread, default_rect);
    };

    let Some(frame_item) = active.doc.item(target_frame_id) else {
        return (default_spread, default_rect);
    };

    let target_spread_idx = active.doc.spreads.iter().position(|s| s.items.iter().any(|it| it.id == target_frame_id)).unwrap_or(default_spread);

    let last_visible: Vec<_> = last_line.glyphs.iter().filter(|g| g.visible && !g.rendered_char.is_whitespace()).collect();
    if last_visible.is_empty() {
        return (default_spread, default_rect);
    }

    let line_min_x = last_visible.iter().map(|g| g.x).fold(f64::INFINITY, f64::min);
    let line_max_x = last_visible.iter().map(|g| g.x + g.adv).fold(f64::NEG_INFINITY, f64::max);
    let center_x = (line_min_x + line_max_x) / 2.0;

    let spread_center_pt = frame_item.xf * designcraft_geom::Point::new(center_x, last_line.baseline);
    let spread_center_x = spread_center_pt.x;
    let spread_last_baseline_y = spread_center_pt.y;

    let prev_baseline_y = prev_line.map(|pl| (frame_item.xf * designcraft_geom::Point::new(0.0, pl.baseline)).y);

    let mw = margin.x1 - margin.x0;
    let img_w = (mw * 0.22).clamp(80.0, 110.0);

    let (y0, y1) = if let Some(pby) = prev_baseline_y {
        let gap = spread_last_baseline_y - pby;
        let h = (gap - 10.0).clamp(32.0, 52.0);
        let mid_y = (pby + spread_last_baseline_y) / 2.0;
        let mut y0 = mid_y - h / 2.0;
        let mut y1 = mid_y + h / 2.0;
        if y0 < pby + 2.0 {
            let shift = (pby + 2.0) - y0;
            y0 += shift;
            y1 += shift;
        }
        if y1 > spread_last_baseline_y - 4.0 {
            let shift = y1 - (spread_last_baseline_y - 4.0);
            y0 -= shift;
            y1 -= shift;
        }
        (y0, y1)
    } else {
        let h = 45.0;
        let bottom = spread_last_baseline_y - 6.0;
        (bottom - h, bottom)
    };

    let mut x0 = spread_center_x - img_w / 2.0;
    let mut x1 = spread_center_x + img_w / 2.0;

    // Ensure image2 never hugs the right margin too closely (keep at least 20pt buffer)
    let max_allowed_x1 = margin.x1 - 20.0;
    if x1 > max_allowed_x1 {
        let shift = x1 - max_allowed_x1;
        x0 -= shift;
        x1 -= shift;
    }
    if x0 < margin.x0 {
        let shift = margin.x0 - x0;
        x0 += shift;
        x1 += shift;
    }

    let mut final_y0 = y0;
    let mut final_y1 = y1;
    if final_y1 > margin.y1 {
        let shift = final_y1 - margin.y1;
        final_y0 -= shift;
        final_y1 -= shift;
    }
    if final_y0 < margin.y0 {
        let shift = margin.y0 - final_y0;
        final_y0 += shift;
        final_y1 += shift;
    }

    (target_spread_idx, [x0, final_y0, x1, final_y1])
}

/// Split title like "BAB 01 Pemerintahan" into ("BAB 01", "Pemerintahan")
pub(crate) fn split_chapter_title(title: &str, num_val: &str) -> (String, String) {
    split_chapter_title_locale(title, num_val, false)
}

pub(crate) fn split_chapter_title_locale(title: &str, num_val: &str, is_en: bool) -> (String, String) {
    let t = title.trim();
    let words: Vec<&str> = t.split_whitespace().collect();
    if words.len() >= 3 && (words[0].eq_ignore_ascii_case("bab") || words[0].eq_ignore_ascii_case("chapter")) {
        let ch_num = words[1].trim_matches(|c: char| c == '.' || c == ':' || c == ';');
        let chapter_line = format!("{} {}", words[0].to_uppercase(), ch_num);
        let title_line = words[2..].join(" ");
        return (chapter_line, title_line);
    }
    if words.len() >= 2 && (words[0].eq_ignore_ascii_case("bab") || words[0].eq_ignore_ascii_case("chapter")) {
        let ch_num = words[1].trim_matches(|c: char| c == '.' || c == ':' || c == ';');
        return (format!("{} {}", words[0].to_uppercase(), ch_num), String::new());
    }

    if !num_val.is_empty() {
        let prefix = if is_en { "CHAPTER" } else { "BAB" };
        let chapter_line = format!("{prefix} {num_val}");
        let clean = t
            .strip_prefix("CHAPTER")
            .or_else(|| t.strip_prefix("chapter"))
            .or_else(|| t.strip_prefix("BAB"))
            .or_else(|| t.strip_prefix("bab"))
            .unwrap_or(t)
            .trim();
        let clean2 = clean.strip_prefix(num_val).unwrap_or(clean).trim();
        let clean3 = clean2.trim_matches(|c: char| c == '.' || c == ':' || c == ';').trim();
        let final_title = if clean3.is_empty() { t.to_string() } else { clean3.to_string() };
        return (chapter_line, final_title);
    }

    (t.to_string(), String::new())
}

/// Create a centered text frame with Arial Bold and optional paragraph style.
pub(crate) fn frame_with_centered_bold_title(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], text: &str, style_opt: Option<&str>) -> Option<u64> {
    let res = app.run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "text", "text": text, "caret": false})).ok()?;
    let fid = res.get("id").and_then(Value::as_u64)?;
    let story = res.get("story").and_then(Value::as_u64)?;
    let end = text.len().max(1) as u64;
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    if let Some(style) = style_opt {
        let _ = app.run("style.paragraph.apply", json!({"name": style}));
    }
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    let _ = app.run("type.char", json!({"fontFamily": "Arial", "fontStyle": "Bold", "size": 11.0}));
    let _ = app.run("selection.set", json!({"ids": [fid]}));
    let _ = app.run("type.alignCenter", json!({}));
    Some(story)
}

/// Centred Arial title in a 100 % transparent text box (no fill, no stroke).
pub(crate) fn frame_with_centered_arial(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], text: &str, style: Option<&str>) -> Option<u64> {
    let res = app.run("frame.create", json!({"spread": spread_idx, "rect": rect, "content": "text", "text": text, "caret": false})).ok()?;
    let fid = res.get("id").and_then(Value::as_u64)?;
    let story = res.get("story").and_then(Value::as_u64)?;
    let end = text.len().max(1) as u64;
    let _ = app.run("text.select", json!({"story": story, "anchor": 0, "focus": end}));
    if let Some(style) = style {
        let _ = app.run("style.paragraph.apply", json!({"name": style}));
    }
    let _ = app.run("type.char", json!({"fontFamily": "Arial", "size": 11.0}));
    let _ = app.run("selection.set", json!({"ids": [fid]}));
    let _ = app.run("type.alignCenter", json!({}));
    let _ = app.run("object.fill", json!({"ids": [fid], "swatch": "[None]"}));
    let _ = app.run("object.stroke", json!({"ids": [fid], "swatch": "[None]"}));
    Some(fid)
}

/// Resolve a catalog `file` value beside the catalog CSV: as written, relative to the CSV's
/// folder, the same name with `extension` appended, or just the file name beside the catalog
/// (so paths copied from another machine still resolve).
pub(crate) fn resolve_catalog_related_file(catalog_path: &str, file_str: &str, extension: &str) -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(catalog_path).parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut candidates = vec![dir.join(file_str), std::path::PathBuf::from(file_str), dir.join(format!("{file_str}.{extension}"))];
    if let Some(name) = std::path::Path::new(file_str).file_name() {
        candidates.push(dir.join(name));
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Split chart title like "Grafik 02.01 Panjang Jalan..." into ("Grafik 02.01", "Panjang Jalan...")
pub(crate) fn split_chart_title(title: &str, num_val: &str, type_val: &str) -> (String, String) {
    let t = title.trim();
    let words: Vec<&str> = t.split_whitespace().collect();
    let eff_type = if type_val.trim().is_empty() { "Grafik" } else { type_val.trim() };

    if words.len() >= 3 && is_chart_type(words[0]) {
        let ch_num = words[1].trim_matches(|c: char| c == ':' || c == ';');
        let line1 = format!("{} {}", words[0], ch_num);
        let line2 = words[2..].join(" ");
        return (line1, line2);
    }

    if words.len() >= 2 && is_chart_type(words[0]) {
        let ch_num = words[1].trim_matches(|c: char| c == ':' || c == ';');
        return (format!("{} {}", words[0], ch_num), String::new());
    }

    if words.len() >= 2 && words[0].contains('.') && words[0].chars().any(|c| c.is_ascii_digit()) {
        let ch_num = words[0].trim_matches(|c: char| c == ':' || c == ';');
        let line1 = format!("{eff_type} {ch_num}");
        let line2 = words[1..].join(" ");
        return (line1, line2);
    }

    if !num_val.is_empty() {
        let line1 = format!("{eff_type} {num_val}");
        let clean = words
            .iter()
            .copied()
            .filter(|w| !is_chart_type(w) && !w.trim_matches(|c: char| c == ':' || c == ';' || c == '.').eq_ignore_ascii_case(num_val))
            .collect::<Vec<_>>()
            .join(" ");
        let final_title = if clean.is_empty() { t.to_string() } else { clean };
        return (line1, final_title);
    }

    (t.to_string(), String::new())
}

pub(crate) fn is_ulasan_type(type_val: &str) -> bool {
    let s = type_val.trim();
    if s.is_empty() {
        return false;
    }
    let lower = s.to_lowercase();
    if lower == "ulasan" || lower == "review" || lower == "editorial" || lower == "chapter" {
        return true;
    }
    if (lower.starts_with('r') || lower.starts_with('b')) && lower[1..].chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    false
}

pub(crate) fn is_divider_type(type_val: &str) -> bool {
    let s = type_val.trim();
    if s.is_empty() {
        return false;
    }
    let lower = s.to_lowercase();
    if lower == "div" || lower == "divider" || lower == "dividers" || lower == "pembatas" {
        return true;
    }
    if lower.starts_with("divider") || lower.starts_with("pembatas") {
        return true;
    }
    if let Some(rest) = lower.strip_prefix("div") {
        let rest_clean = rest.trim_start_matches(['_', '-', ' ', '.']);
        if rest_clean.is_empty() || rest_clean.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    false
}

pub(crate) fn is_chart_type(type_val: &str) -> bool {
    ["grafik", "graph", "chart", "bagan", "diagram"].iter().any(|k| type_val.eq_ignore_ascii_case(k))
}

/// The number of paragraphs the TOC would collect for `style` right now.
#[cfg(test)]
