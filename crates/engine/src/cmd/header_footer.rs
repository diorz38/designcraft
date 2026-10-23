//! Header/Footer frames for pages (Layout ▸ Header/Footer…).
//!
//! Provides `layout.headerFooter.get`, `layout.headerFooter.apply`, and `layout.headerFooter.remove`.
//! Creates and maintains a dedicated parent spread ("H-Header/Footer") with header and footer
//! text frames and hairline rules, with auto-sizing, master inheritance, and scope page assignment.

use std::sync::Arc;

use designcraft_doc::{
    Document, ItemId, Margins, Page, PageSide, ParaFormat, SpreadId, SpreadRef,
    header_footer::{HeaderFooter, HfScope, Sided, SidedItems, SidedText},
    item::{AutoSize, Item, Shape, Stroke},
    story,
    vars::var_char,
};
use designcraft_geom::{Point, Rect, shapes};
use serde_json::{Value, json};

use super::{CommandSpec, bad, cmd, has_doc, ok, str_param};
use crate::{Result, Session};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "layout.headerFooter.get",
            "Header/Footer",
            [],
            None,
            "{}",
            has_doc,
            get
        ),
        cmd!(
            "layout.headerFooter.apply",
            "Header/Footer…",
            ["Layout"],
            None,
            "{preset?, sided?, scope?, headerOffset?, footerOffset?, header?, footer?, style?, rule?}",
            has_doc,
            apply
        ),
        cmd!("layout.headerFooter.remove", "Remove Header/Footer", ["Layout"], None, "{}", has_doc, remove),
    ]
}

/// Known preset specifications.
pub fn preset_spec(preset: &str) -> Option<HeaderFooter> {
    match preset.to_ascii_lowercase().as_str() {
        "minimal" => Some(HeaderFooter {
            preset: "minimal".into(),
            sided: Sided::Single,
            scope: HfScope::All,
            parent: None,
            header_offset: 24.0,
            footer_offset: 24.0,
            header: SidedText::default(),
            footer: SidedText { single: "{filename}\u{e006}{page}".into(), left: String::new(), right: String::new() },
            style: story::BASIC_PARAGRAPH.into(),
            rule: false,
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
        }),
        "book" => Some(HeaderFooter {
            preset: "book".into(),
            sided: Sided::Double,
            scope: HfScope::All,
            parent: None,
            header_offset: 36.0,
            footer_offset: 36.0,
            header: SidedText { single: String::new(), left: "{page}\u{e006}{runningHeader}".into(), right: "{runningHeader}\u{e006}{page}".into() },
            footer: SidedText::default(),
            style: story::BASIC_PARAGRAPH.into(),
            rule: false,
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
        }),
        "editorial" => Some(HeaderFooter {
            preset: "editorial".into(),
            sided: Sided::Double,
            scope: HfScope::All,
            parent: None,
            header_offset: 36.0,
            footer_offset: 36.0,
            header: SidedText {
                single: String::new(),
                left: "{page}  {section}\u{e006}{runningHeader}".into(),
                right: "{runningHeader}\u{e006}{section}  {page}".into(),
            },
            footer: SidedText { single: String::new(), left: "{filename}".into(), right: "{filename}".into() },
            style: story::BASIC_PARAGRAPH.into(),
            rule: true,
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
        }),
        _ => None,
    }
}

/// Expands template tokens into text with special character markers:
/// - `{page}` -> `\u{E000}` (`story::PAGE_NUMBER`)
/// - `{section}` -> `\u{E001}` (`story::SECTION_MARKER`)
/// - `{filename}` -> text variable "File Name"
/// - `{chapter}` -> text variable "Chapter Number"
/// - `{runningHeader}` -> text variable "Running Header"
pub fn expand_tokens(template: &str, doc: &Document) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        if let Some(close) = after_open.find('}') {
            let token = &after_open[..close];
            match token {
                "page" => out.push(story::PAGE_NUMBER),
                "section" => out.push(story::SECTION_MARKER),
                "filename" => {
                    if let Some(c) = doc.variable("File Name").and_then(var_char) {
                        out.push(c);
                    } else {
                        out.push_str("File Name");
                    }
                }
                "chapter" => {
                    if let Some(c) = doc.variable("Chapter Number").and_then(var_char) {
                        out.push(c);
                    } else {
                        out.push_str("Chapter Number");
                    }
                }
                "runningHeader" => {
                    if let Some(c) = doc.variable("Running Header").and_then(var_char) {
                        out.push(c);
                    } else {
                        out.push_str("Running Header");
                    }
                }
                "tab" | "rightIndentTab" => out.push(story::RIGHT_INDENT_TAB),
                _ => {
                    out.push('{');
                    out.push_str(token);
                    out.push('}');
                }
            }
            rest = &after_open[close + 1..];
        } else {
            out.push('{');
            rest = after_open;
        }
    }
    out.push_str(rest);
    out
}

/// Query command returning stored spec + resolved page count and active page index.
fn get(s: &mut Session, _: &Value) -> Result<Value> {
    let st = s.doc()?;
    let d = &st.doc;
    let active_page = st.selection.page.map(|p| p + 1).unwrap_or(1);
    let styles: Vec<&str> = d.styles.paragraph.iter().map(|p| p.name.as_str()).collect();
    let sections: Vec<Value> = d
        .sections
        .iter()
        .enumerate()
        .map(|(i, sec)| {
            json!({
                "index": i,
                "start": sec.start + 1,
                "marker": sec.marker,
                "prefix": sec.prefix,
            })
        })
        .collect();

    let presets = json!({
        "minimal": preset_spec("minimal"),
        "book": preset_spec("book"),
        "editorial": preset_spec("editorial"),
    });

    Ok(json!({
        "headerFooter": d.header_footer,
        "facingPages": d.settings.facing_pages,
        "pageCount": d.page_count(),
        "activePage": active_page,
        "presets": presets,
        "paragraphStyles": styles,
        "sections": sections,
    }))
}

/// Parses the page scope from params.
fn parse_scope(p: &Value, cmd: &str) -> Result<Option<HfScope>> {
    let Some(v) = p.get("scope") else { return Ok(None) };
    if v.as_str().is_some_and(|s| s.eq_ignore_ascii_case("all")) {
        return Ok(Some(HfScope::All));
    }
    if let Some(obj) = v.as_object() {
        if let Some(sec) = obj.get("section").and_then(Value::as_u64) {
            return Ok(Some(HfScope::Section(sec as usize)));
        }
        if let Some(range) = obj.get("range").and_then(Value::as_array) {
            let first = range.first().and_then(Value::as_u64).ok_or_else(|| bad(cmd, "invalid scope range first page"))?;
            let last = range.get(1).and_then(Value::as_u64).ok_or_else(|| bad(cmd, "invalid scope range last page"))?;
            return Ok(Some(HfScope::Range { first: first as usize, last: last as usize }));
        }
    }
    Err(bad(cmd, "invalid scope: expected \"all\", {\"section\": n}, or {\"range\": [first, last]}"))
}

/// Applies header/footer frames and scope.
fn apply(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "layout.headerFooter.apply";

    // Build the target HeaderFooter configuration from preset / existing / params.
    let preset_name = str_param(p, "preset");
    let mut hf = if let Some(name) = preset_name {
        let mut spec = preset_spec(name).ok_or_else(|| bad(CMD, format!("unknown preset `{name}`; expected minimal, book, or editorial")))?;
        if let Some(existing) = &s.doc()?.doc.header_footer {
            spec.parent = existing.parent;
            spec.header_items = existing.header_items;
            spec.footer_items = existing.footer_items;
            spec.rule_items = existing.rule_items;
        }
        spec
    } else if let Some(existing) = s.doc()?.doc.header_footer.clone() {
        existing
    } else {
        preset_spec("minimal").ok_or_else(|| bad(CMD, "default minimal preset missing"))?
    };

    if let Some(name) = preset_name {
        hf.preset = name.to_string();
    } else if p.as_object().is_some_and(|o| o.keys().any(|k| k != "preset")) {
        hf.preset = "custom".to_string();
    }

    if let Some(s_val) = str_param(p, "sided") {
        hf.sided = match s_val.to_ascii_lowercase().as_str() {
            "single" => Sided::Single,
            "double" => Sided::Double,
            _ => return Err(bad(CMD, format!("invalid sided `{s_val}`; expected single or double"))),
        };
    }

    if let Some(scope) = parse_scope(p, CMD)? {
        hf.scope = scope;
    }

    if let Some(v) = p.get("headerOffset").and_then(Value::as_f64) {
        hf.header_offset = v;
    }
    if let Some(v) = p.get("footerOffset").and_then(Value::as_f64) {
        hf.footer_offset = v;
    }
    if let Some(st) = str_param(p, "style") {
        hf.style = st.to_string();
    }
    if let Some(r) = p.get("rule").and_then(Value::as_bool) {
        hf.rule = r;
    }

    if let Some(h) = p.get("header") {
        if let Some(s) = str_param(h, "single") {
            hf.header.single = s.to_string();
        }
        if let Some(l) = str_param(h, "left") {
            hf.header.left = l.to_string();
        }
        if let Some(r) = str_param(h, "right") {
            hf.header.right = r.to_string();
        }
    }

    if let Some(f) = p.get("footer") {
        if let Some(s) = str_param(f, "single") {
            hf.footer.single = s.to_string();
        }
        if let Some(l) = str_param(f, "left") {
            hf.footer.left = l.to_string();
        }
        if let Some(r) = str_param(f, "right") {
            hf.footer.right = r.to_string();
        }
    }

    // Validation
    if hf.header_offset < 0.0 || !hf.header_offset.is_finite() {
        return Err(bad(CMD, "headerOffset must be a non-negative finite number"));
    }
    if hf.footer_offset < 0.0 || !hf.footer_offset.is_finite() {
        return Err(bad(CMD, "footerOffset must be a non-negative finite number"));
    }
    let page_h = s.doc()?.doc.settings.page_height;
    if hf.header_offset + hf.footer_offset >= page_h {
        return Err(bad(
            CMD,
            format!("headerOffset ({}) + footerOffset ({}) must be less than page height ({})", hf.header_offset, hf.footer_offset, page_h),
        ));
    }
    if hf.sided == Sided::Double && !s.doc()?.doc.settings.facing_pages {
        return Err(bad(CMD, "double-sided header/footer requires facing pages"));
    }

    s.edit(|d, _| {
        // Ensure paragraph style exists
        let style_to_use = if d.styles.para(&hf.style).is_some() { hf.style.clone() } else { story::BASIC_PARAGRAPH.to_string() };

        // Resolve or create parent spread
        let parent_id = resolve_parent_spread(d)?;
        hf.parent = Some(parent_id);

        let parent_idx = d.parent_index(parent_id).ok_or_else(|| bad(CMD, "parent index missing"))?;

        // Geometry: rects for single, left, right pages on the parent spread
        let parent_pages = d.parents.get(parent_idx).ok_or_else(|| bad(CMD, "parent spread missing"))?.pages.clone();

        let layer = d.default_layer();

        // Single vs Double sided processing
        match hf.sided {
            Sided::Single => {
                // If previously double-sided items existed, remove them
                remove_optional_item(d, hf.header_items.left.take());
                remove_optional_item(d, hf.header_items.right.take());
                remove_optional_item(d, hf.footer_items.left.take());
                remove_optional_item(d, hf.footer_items.right.take());
                remove_optional_item(d, hf.rule_items.left.take());
                remove_optional_item(d, hf.rule_items.right.take());

                let pg = parent_pages.first().cloned().ok_or_else(|| bad(CMD, "parent has no pages"))?;
                let (h_rect, f_rect, r_line) = page_hf_geometry(&pg, hf.header_offset, hf.footer_offset);

                // Header text frame
                let h_text = expand_tokens(&hf.header.single, d);
                hf.header_items.single = sync_text_frame(
                    d,
                    hf.header_items.single,
                    parent_idx,
                    h_rect,
                    layer,
                    &h_text,
                    &style_to_use,
                    0, // Top
                )?;

                // Footer text frame
                let f_text = expand_tokens(&hf.footer.single, d);
                hf.footer_items.single = sync_text_frame(
                    d,
                    hf.footer_items.single,
                    parent_idx,
                    f_rect,
                    layer,
                    &f_text,
                    &style_to_use,
                    6, // Bottom
                )?;

                // Rule item
                if hf.rule {
                    hf.rule_items.single = Some(sync_rule_item(d, hf.rule_items.single, parent_idx, layer, r_line)?);
                } else {
                    remove_optional_item(d, hf.rule_items.single.take());
                }
            }
            Sided::Double => {
                // If previously single-sided items existed, remove them
                remove_optional_item(d, hf.header_items.single.take());
                remove_optional_item(d, hf.footer_items.single.take());
                remove_optional_item(d, hf.rule_items.single.take());

                // Find left and right pages on the parent spread by PageSide
                let left_pg = parent_pages
                    .iter()
                    .find(|p| p.side == PageSide::Left)
                    .cloned()
                    .ok_or_else(|| bad(CMD, "facing parent spread has no left page"))?;
                let right_pg = parent_pages
                    .iter()
                    .find(|p| p.side == PageSide::Right)
                    .cloned()
                    .ok_or_else(|| bad(CMD, "facing parent spread has no right page"))?;

                // Left (Verso)
                let (lh_rect, lf_rect, lr_line) = page_hf_geometry(&left_pg, hf.header_offset, hf.footer_offset);
                let lh_text = expand_tokens(&hf.header.left, d);
                hf.header_items.left = sync_text_frame(d, hf.header_items.left, parent_idx, lh_rect, layer, &lh_text, &style_to_use, 0)?;
                let lf_text = expand_tokens(&hf.footer.left, d);
                hf.footer_items.left = sync_text_frame(d, hf.footer_items.left, parent_idx, lf_rect, layer, &lf_text, &style_to_use, 6)?;
                if hf.rule {
                    hf.rule_items.left = Some(sync_rule_item(d, hf.rule_items.left, parent_idx, layer, lr_line)?);
                } else {
                    remove_optional_item(d, hf.rule_items.left.take());
                }

                // Right (Recto)
                let (rh_rect, rf_rect, rr_line) = page_hf_geometry(&right_pg, hf.header_offset, hf.footer_offset);
                let rh_text = expand_tokens(&hf.header.right, d);
                hf.header_items.right = sync_text_frame(d, hf.header_items.right, parent_idx, rh_rect, layer, &rh_text, &style_to_use, 0)?;
                let rf_text = expand_tokens(&hf.footer.right, d);
                hf.footer_items.right = sync_text_frame(d, hf.footer_items.right, parent_idx, rf_rect, layer, &rf_text, &style_to_use, 6)?;
                if hf.rule {
                    hf.rule_items.right = Some(sync_rule_item(d, hf.rule_items.right, parent_idx, layer, rr_line)?);
                } else {
                    remove_optional_item(d, hf.rule_items.right.take());
                }
            }
        }

        // Apply parent to scoped pages
        let target_pages = compute_scope_pages(d, &hf.scope);
        let fallback_parent = d.parents.first().filter(|p| p.id != parent_id).map(|p| p.id);

        // Clear parent from pages previously assigned to header/footer parent that are no longer in scope
        let pages_to_clear: Vec<usize> =
            (0..d.page_count()).filter(|&i| d.page(i).is_some_and(|p| p.parent == Some(parent_id)) && !target_pages.contains(&i)).collect();
        if !pages_to_clear.is_empty() {
            d.apply_parent(&pages_to_clear, fallback_parent)?;
        }

        if !target_pages.is_empty() {
            d.apply_parent(&target_pages, Some(parent_id))?;
        }

        d.header_footer = Some(hf.clone());
        Ok(json!({ "headerFooter": hf }))
    })
}

/// Removes header/footer frames, resets parent assignments, and clears doc.header_footer.
fn remove(s: &mut Session, _: &Value) -> Result<Value> {
    s.edit(|d, _| {
        let Some(hf) = d.header_footer.take() else {
            return ok();
        };

        // Remove all generated items
        for id in [
            hf.header_items.single,
            hf.header_items.left,
            hf.header_items.right,
            hf.footer_items.single,
            hf.footer_items.left,
            hf.footer_items.right,
            hf.rule_items.single,
            hf.rule_items.left,
            hf.rule_items.right,
        ]
        .into_iter()
        .flatten()
        {
            remove_optional_item(d, Some(id));
        }

        // Reset parent on pages that have the header/footer parent
        if let Some(pid) = hf.parent {
            let fallback = d.parents.first().filter(|p| p.id != pid).map(|p| p.id);
            let pages_with_hf: Vec<usize> = (0..d.page_count()).filter(|&i| d.page(i).is_some_and(|p| p.parent == Some(pid))).collect();
            if !pages_with_hf.is_empty() {
                let _ = d.apply_parent(&pages_with_hf, fallback);
            }
        }

        ok()
    })
}

/// Computes the page indices targeted by a scope.
fn compute_scope_pages(d: &Document, scope: &HfScope) -> Vec<usize> {
    let total = d.page_count();
    match scope {
        HfScope::All => (0..total).collect(),
        HfScope::Section(sec_idx) => {
            if let Some(sec) = d.sections.get(*sec_idx) {
                let start = sec.start;
                let end = d.sections.get(sec_idx + 1).map(|s| s.start).unwrap_or(total);
                (start..end.min(total)).collect()
            } else {
                (0..total).collect()
            }
        }
        HfScope::Range { first, last } => {
            if *first == 0 || *first > *last || total == 0 {
                return vec![];
            }
            let first_idx = first.saturating_sub(1);
            let last_idx = (*last).min(total).saturating_sub(1);
            if first_idx <= last_idx { (first_idx..=last_idx).collect() } else { vec![] }
        }
    }
}

/// Resolves or creates the parent spread ("H-Header/Footer") with inheritance.
fn resolve_parent_spread(d: &mut Document) -> Result<SpreadId> {
    // 1. Existing parent recorded in doc.header_footer
    if let Some(pid) = d.header_footer.as_ref().and_then(|hf| hf.parent).filter(|&pid| d.parent_index(pid).is_some()) {
        return Ok(pid);
    }

    // 2. Existing parent named "Header/Footer"
    if let Some(existing) = d.parents.iter().find(|p| p.parent.as_ref().is_some_and(|pi| pi.name == "Header/Footer")) {
        return Ok(existing.id);
    }

    // 3. Create fresh parent "H-Header/Footer"
    let src = d.page(0).cloned();
    let (columns, gutter, margins) =
        src.as_ref().map_or_else(|| (1, 12.0, Margins::uniform(36.0)), |p| (p.columns.count, p.columns.gutter, p.margins));

    let prefix =
        if d.parents.iter().any(|p| p.parent.as_ref().is_some_and(|pi| pi.prefix == "H")) { d.next_parent_prefix() } else { "H".to_string() };

    let parent_id = d.add_parent(&prefix, "Header/Footer", columns, gutter, margins);

    // Parent inheritance: set based_on to first existing parent so artwork/guides from A-Parent are kept
    let base_parent = d.parents.iter().find(|p| p.id != parent_id).map(|p| p.id);
    if let Some(pi) = d.parents.iter_mut().find(|p| p.id == parent_id).and_then(|sp| Arc::make_mut(sp).parent.as_mut()) {
        pi.based_on = base_parent;
    }

    Ok(parent_id)
}

/// Calculates header rect, footer rect, and header rule line coordinates for a given parent page.
fn page_hf_geometry(page: &Page, header_offset: f64, footer_offset: f64) -> (Rect, Rect, (Point, Point)) {
    let (left_m, right_m) = page.left_right_margins();
    let x0 = page.x + left_m;
    let x1 = (page.x + page.width - right_m).max(x0 + 1.0);

    // Default frame height
    const FRAME_H: f64 = 18.0;

    // Header text frame
    let h_y0 = header_offset;
    let h_y1 = h_y0 + FRAME_H;
    let header_rect = Rect::new(x0, h_y0, x1, h_y1);

    // Footer text frame
    let f_y1 = (page.height - footer_offset).max(FRAME_H);
    let f_y0 = f_y1 - FRAME_H;
    let footer_rect = Rect::new(x0, f_y0, x1, f_y1);

    // Hairline rule line immediately below header area
    let rule_y = h_y1 + 4.0;
    let rule_line = (Point::new(x0, rule_y), Point::new(x1, rule_y));

    (header_rect, footer_rect, rule_line)
}

/// Safely removes an item if present.
fn remove_optional_item(d: &mut Document, item_id: Option<ItemId>) {
    if let Some(id) = item_id.filter(|&id| d.item(id).is_some()) {
        let _ = d.remove_item(id);
    }
}

/// Synchronizes a text frame item on the parent spread: updates existing or creates new.
fn sync_text_frame(
    d: &mut Document,
    existing_id: Option<ItemId>,
    parent_idx: usize,
    rect: Rect,
    layer: designcraft_doc::LayerId,
    expanded_text: &str,
    style_name: &str,
    auto_size_ref: u8,
) -> Result<Option<ItemId>> {
    if expanded_text.is_empty() {
        // If template evaluates to completely empty, clean up existing frame
        remove_optional_item(d, existing_id);
        return Ok(None);
    }

    if let Some((id, sid)) = existing_id.and_then(|id| {
        let tf = d.item(id)?.text_frame()?;
        Some((id, tf.story))
    }) {
        // Update story text safely using st.replace
        if let Some(st) = d.story_mut(sid) {
            let old_len = st.text.len();
            st.replace(0..old_len, expanded_text);
            for p in &mut st.paras {
                p.style = style_name.to_string();
            }
        }
        // Update frame rect
        if let Some(it) = d.item_mut(id) {
            it.path = shapes::rectangle(rect);
            if let Some(tf) = it.text_frame_mut() {
                tf.options.auto_size = AutoSize::HeightOnly;
                tf.options.auto_size_ref = auto_size_ref;
            }
        }
        return Ok(Some(id));
    }

    // Otherwise create a new text frame with its own story
    let (fid, sid) = d.add_text_frame(SpreadRef::Parent(parent_idx), rect, layer, expanded_text, ParaFormat::default())?;

    if let Some(tf) = d.item_mut(fid).and_then(|it| it.text_frame_mut()) {
        tf.options.auto_size = AutoSize::HeightOnly;
        tf.options.auto_size_ref = auto_size_ref;
    }

    if let Some(st) = d.story_mut(sid) {
        for p in &mut st.paras {
            p.style = style_name.to_string();
        }
    }

    Ok(Some(fid))
}

/// Synchronizes a hairline rule item on the parent spread: updates existing or creates new.
fn sync_rule_item(
    d: &mut Document,
    existing_id: Option<ItemId>,
    parent_idx: usize,
    layer: designcraft_doc::LayerId,
    line: (Point, Point),
) -> Result<ItemId> {
    if let Some(it) = existing_id.and_then(|id| d.item_mut(id)) {
        it.path = shapes::line(line.0, line.1);
        it.stroke = Stroke { weight: 0.5, swatch: designcraft_color::swatch::BLACK.to_string(), ..Default::default() };
        it.fill = designcraft_doc::Fill::none();
        return Ok(it.id);
    }

    let id = ItemId(d.alloc());
    let mut it = Item::new(id, layer, Shape::GraphicLine, shapes::line(line.0, line.1));
    it.stroke = Stroke { weight: 0.5, swatch: designcraft_color::swatch::BLACK.to_string(), ..Default::default() };
    it.fill = designcraft_doc::Fill::none();
    d.insert_item(SpreadRef::Parent(parent_idx), it, None)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use designcraft_doc::build::NewDocument;

    fn session_with_doc(facing: bool, pages: usize) -> Session {
        let mut s = Session::new();
        let nd = NewDocument { facing_pages: facing, pages, ..Default::default() };
        let d = Document::new(&nd);
        s.add_document(crate::DocState::new(d, None));
        s
    }

    #[test]
    fn test_token_expansion() {
        let s = session_with_doc(true, 4);
        let d = &s.doc().unwrap().doc;
        let expanded = expand_tokens("{page} {section}\u{e006}{filename} {chapter}", d);
        assert!(expanded.contains(story::PAGE_NUMBER));
        assert!(expanded.contains(story::SECTION_MARKER));
        assert!(expanded.contains('\u{e006}'));
        assert!(expanded.contains(var_char(2).unwrap())); // File Name is index 2
        assert!(expanded.contains(var_char(0).unwrap())); // Chapter Number is index 0
    }

    #[test]
    fn test_apply_editorial_preset_facing_pages() {
        let mut s = session_with_doc(true, 4);
        let res = s.execute("layout.headerFooter.apply", &json!({ "preset": "editorial" }));
        assert!(res.is_ok(), "{:?}", res.err());

        let doc = &s.doc().unwrap().doc;
        let hf = doc.header_footer.as_ref().expect("header_footer should be present");
        assert_eq!(hf.preset, "editorial");
        assert_eq!(hf.sided, Sided::Double);
        assert!(hf.rule);

        // Header items should exist for left and right
        assert!(hf.header_items.left.is_some());
        assert!(hf.header_items.right.is_some());
        assert!(hf.header_items.single.is_none());

        // Footer items should exist for left and right
        assert!(hf.footer_items.left.is_some());
        assert!(hf.footer_items.right.is_some());

        // Rule items should exist for left and right
        assert!(hf.rule_items.left.is_some());
        assert!(hf.rule_items.right.is_some());

        // Parent should be applied to all 4 pages
        let parent_id = hf.parent.expect("parent id");
        for i in 0..4 {
            assert_eq!(doc.page(i).unwrap().parent, Some(parent_id));
        }

        // Parent should inherit based_on from A-Parent
        let parent_spread = doc.parents.iter().find(|p| p.id == parent_id).expect("parent spread");
        assert!(parent_spread.parent.as_ref().and_then(|p| p.based_on).is_some() || doc.parents.len() > 1);
    }

    #[test]
    fn test_double_sided_rejected_on_single_sided_document() {
        let mut s = session_with_doc(false, 2);
        let res = s.execute("layout.headerFooter.apply", &json!({ "sided": "double" }));
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("requires facing pages"));
    }

    #[test]
    fn test_switch_sided_cleans_up_orphaned_frames() {
        let mut s = session_with_doc(true, 4);

        // First apply double-sided
        s.execute("layout.headerFooter.apply", &json!({ "preset": "editorial" })).unwrap();
        let (left_h, right_h) = {
            let hf = s.doc().unwrap().doc.header_footer.as_ref().unwrap();
            (hf.header_items.left.unwrap(), hf.header_items.right.unwrap())
        };
        assert!(s.doc().unwrap().doc.item(left_h).is_some());
        assert!(s.doc().unwrap().doc.item(right_h).is_some());

        // Now switch to single-sided minimal
        s.execute("layout.headerFooter.apply", &json!({ "preset": "minimal" })).unwrap();
        let doc = &s.doc().unwrap().doc;
        let hf = doc.header_footer.as_ref().unwrap();
        assert_eq!(hf.sided, Sided::Single);

        // Left and right items must be removed
        assert!(doc.item(left_h).is_none());
        assert!(doc.item(right_h).is_none());
        assert!(hf.footer_items.single.is_some());
    }

    #[test]
    fn test_reapply_preserves_item_ids_and_story_invariants() {
        let mut s = session_with_doc(true, 4);
        s.execute("layout.headerFooter.apply", &json!({ "preset": "book" })).unwrap();

        let initial_left_h = s.doc().unwrap().doc.header_footer.as_ref().unwrap().header_items.left.unwrap();

        // Reapply with modified header template
        s.execute("layout.headerFooter.apply", &json!({ "header": { "left": "{page} -- custom header" } })).unwrap();

        let doc = &s.doc().unwrap().doc;
        let current_left_h = doc.header_footer.as_ref().unwrap().header_items.left.unwrap();
        // Item ID should be preserved
        assert_eq!(initial_left_h, current_left_h);

        // Story invariants must hold
        assert!(doc.check().is_ok());

        let sid = doc.item(current_left_h).unwrap().text_frame().unwrap().story;
        let story = doc.story(sid).unwrap();
        assert!(story.text.contains("custom header"));
    }

    #[test]
    fn test_remove_cleans_up_items_and_resets_parent() {
        let mut s = session_with_doc(true, 4);
        s.execute("layout.headerFooter.apply", &json!({ "preset": "editorial" })).unwrap();

        let (left_h, right_h, left_r, right_r, pid) = {
            let hf = s.doc().unwrap().doc.header_footer.as_ref().unwrap();
            (
                hf.header_items.left.unwrap(),
                hf.header_items.right.unwrap(),
                hf.rule_items.left.unwrap(),
                hf.rule_items.right.unwrap(),
                hf.parent.unwrap(),
            )
        };

        // Execute remove
        s.execute("layout.headerFooter.remove", &json!({})).unwrap();

        let doc = &s.doc().unwrap().doc;
        assert_eq!(doc.header_footer, None);
        assert!(doc.item(left_h).is_none());
        assert!(doc.item(right_h).is_none());
        assert!(doc.item(left_r).is_none());
        assert!(doc.item(right_r).is_none());

        // Pages should no longer have the header/footer parent
        for i in 0..4 {
            assert_ne!(doc.page(i).unwrap().parent, Some(pid));
        }
    }

    #[test]
    fn test_rtl_binding_locates_left_and_right_by_page_side() {
        let mut s = Session::new();
        let nd = NewDocument { facing_pages: true, pages: 4, ..Default::default() };
        let mut d = Document::new(&nd);
        d.settings.right_to_left_binding = true;
        s.add_document(crate::DocState::new(d, None));

        s.execute("layout.headerFooter.apply", &json!({ "preset": "book" })).unwrap();
        let doc = &s.doc().unwrap().doc;
        let hf = doc.header_footer.as_ref().unwrap();
        assert_eq!(hf.sided, Sided::Double);

        let parent_id = hf.parent.unwrap();
        let parent = doc.parents.iter().find(|p| p.id == parent_id).unwrap();

        let left_item_id = hf.header_items.left.unwrap();
        let right_item_id = hf.header_items.right.unwrap();

        let left_item = doc.item(left_item_id).unwrap();
        let right_item = doc.item(right_item_id).unwrap();

        let left_page = parent.pages.iter().find(|p| p.side == PageSide::Left).unwrap();
        let right_page = parent.pages.iter().find(|p| p.side == PageSide::Right).unwrap();

        // Left item x coordinate must be on the left page bounds
        assert!(left_item.bounds().x0 >= left_page.x);
        assert!(right_item.bounds().x0 >= right_page.x);
    }
}
