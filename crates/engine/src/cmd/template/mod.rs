pub mod catalog;
pub mod types;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use designcraft_color::{Color, Swatch, SwatchValue};
use designcraft_doc::{Content, ItemId, SpreadRef, StoryId};
use designcraft_geom::Vec2;
use serde_json::{Value, json};

use super::{CommandSpec, bad, cmd, has_doc};
use crate::cmd::datamerge::fill::copy_item;
use crate::{Result, Session};

pub use catalog::{read_template_preview, scan_template_catalog};
pub use types::{SlotType, parse_slot_tag, split_slot_value};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "template.page.apply",
            "Apply Template Page…",
            ["Layout", "Templates"],
            None,
            "{path?: string, bytes?: [u8], base64?: string, before?: usize, after?: usize, target_page?: usize, position?: before|after, slots?: {text?: {...}, image?: {...}, story?: {...}}, palette?: {...}} → {page, spread, items: [id], slots_bound: count} — a text slot split over several frames labels them `#slot:text:<key>`, `#slot:text:<key>#2`, … and the value is divided between them (lines, else words, else characters: `2026` → `20`, `26`)",
            has_doc,
            apply_template_page
        ),
        cmd!(
            "template.catalog.list",
            "List Available Templates",
            ["Layout", "Templates"],
            None,
            "{dir?: string, category?: string} → [{id, name, category, path, has_preview}]",
            crate::cmd::always,
            list_template_catalog
        ),
        cmd!(
            "template.preview.get",
            "Get Template Preview PNG",
            ["Layout", "Templates"],
            None,
            "{path: string} → {has_preview: bool, bytes?: [u8], base64?: string}",
            crate::cmd::always,
            get_template_preview
        ),
    ]
}

fn list_template_catalog(_s: &mut Session, p: &Value) -> Result<Value> {
    let dir_path = p.get("dir").and_then(Value::as_str).unwrap_or("assets/templates");
    let category_filter = p.get("category").and_then(Value::as_str);

    let templates = scan_template_catalog(Path::new(dir_path));
    let filtered: Vec<_> =
        templates.into_iter().filter(|t| if let Some(cat) = category_filter { t.category.eq_ignore_ascii_case(cat) } else { true }).collect();

    serde_json::to_value(&filtered).map_err(|e| bad("template.catalog.list", e.to_string()))
}

fn get_template_preview(_s: &mut Session, p: &Value) -> Result<Value> {
    let path_str = p.get("path").and_then(Value::as_str).ok_or_else(|| bad("template.preview.get", "`path` parameter required"))?;
    let preview_bytes = read_template_preview(Path::new(path_str));
    if let Some(bytes) = preview_bytes {
        let b64 = super::file::base64_encode(&bytes);
        Ok(json!({
            "has_preview": true,
            "bytes": bytes,
            "base64": b64,
        }))
    } else {
        Ok(json!({
            "has_preview": false,
        }))
    }
}

fn apply_template_page(s: &mut Session, p: &Value) -> Result<Value> {
    // 1. Resolve template bytes
    let tpl_bytes = if let Some(path_str) = p.get("path").and_then(Value::as_str) {
        std::fs::read(path_str).map_err(|e| bad("template.page.apply", format!("Failed to read template `{path_str}`: {e}")))?
    } else if let Some(b64) = p.get("base64").and_then(Value::as_str) {
        super::file::base64_decode(b64)
    } else if let Some(arr) = p.get("bytes").and_then(Value::as_array) {
        arr.iter().filter_map(Value::as_u64).map(|b| b as u8).collect()
    } else {
        return Err(bad("template.page.apply", "`path`, `base64`, or `bytes` parameter required"));
    };

    // 2. Load template document
    let tpl_doc = designcraft_format::load(&tpl_bytes).map_err(|e| bad("template.page.apply", format!("Invalid .designcraft template: {e}")))?;

    let tpl_spread = tpl_doc.spreads.first().ok_or_else(|| bad("template.page.apply", "Template has no spreads"))?;
    let tpl_page = tpl_spread.pages.first().ok_or_else(|| bad("template.page.apply", "Template has no pages"))?;
    let tpl_w = tpl_page.width;
    let tpl_h = tpl_page.height;
    let tpl_margins = tpl_page.margins;
    let tpl_cols = tpl_page.columns.clone();

    // 3. Determine insertion index
    let active_doc = s.doc()?;
    let n = active_doc.doc.page_count();
    let after_idx = if let Some(b) = p.get("before").and_then(Value::as_u64) {
        let bi = b as usize;
        if bi == 0 { None } else { Some(bi.saturating_sub(1).min(n.saturating_sub(1))) }
    } else if let Some(a) = p.get("after").and_then(Value::as_u64) {
        Some((a as usize).min(n.saturating_sub(1)))
    } else if p.get("position").and_then(Value::as_str) == Some("before") {
        let tgt = p.get("target_page").and_then(Value::as_u64).unwrap_or(0) as usize;
        if tgt == 0 { None } else { Some(tgt.saturating_sub(1).min(n.saturating_sub(1))) }
    } else if let Some(tgt) = p.get("target_page").and_then(Value::as_u64) {
        Some((tgt as usize).min(n.saturating_sub(1)))
    } else {
        Some(n.saturating_sub(1))
    };

    // 4. Extract slots & palette overrides
    let text_slots: HashMap<String, String> = p
        .get("slots")
        .and_then(|s| s.get("text"))
        .and_then(Value::as_object)
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
        .unwrap_or_default();

    let image_slots: HashMap<String, String> = p
        .get("slots")
        .and_then(|s| s.get("image").or_else(|| s.get("images")))
        .and_then(Value::as_object)
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
        .unwrap_or_default();

    let story_slots: HashMap<String, String> = p
        .get("slots")
        .and_then(|s| s.get("story").or_else(|| s.get("rtf")))
        .and_then(Value::as_object)
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
        .unwrap_or_default();

    let palette_overrides: HashMap<String, String> = p
        .get("palette")
        .and_then(Value::as_object)
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
        .unwrap_or_default();

    // 5. Insert new page and copy items into document
    struct InsertedInfo {
        page_idx: usize,
        spread_idx: usize,
        new_items: Vec<ItemId>,
        image_tasks: Vec<(ItemId, String)>,
        story_tasks: Vec<(ItemId, String)>,
        slots_bound: usize,
    }

    let inserted = s.edit(|d, _| {
        // Insert new page into document
        let page_ids = d.insert_pages(after_idx, 1, None)?;
        let first_id = page_ids.first().copied().ok_or_else(|| bad("template.page.apply", "Failed to insert page"))?;
        let page_idx = d.pages().position(|p| p.2.id == first_id).unwrap_or(0);
        let (spread_idx, page_in_spread) = d.page_loc(page_idx).ok_or_else(|| bad("template.page.apply", "Page location not found"))?;

        // Update page geometry to match template page
        if let Some(dest_sp) = d.spreads.get_mut(spread_idx) {
            let dest_sp_mut = Arc::make_mut(dest_sp);
            if let Some(dest_p) = dest_sp_mut.pages.get_mut(page_in_spread) {
                dest_p.width = tpl_w;
                dest_p.height = tpl_h;
                dest_p.margins = tpl_margins;
                dest_p.columns = tpl_cols;
            }
            dest_sp_mut.relayout();
        }

        // Apply palette overrides & swatches
        for (name, hex) in &palette_overrides {
            if let Some(col) = Color::from_hex(hex) {
                let mut found = false;
                for sw in &mut d.swatches {
                    if sw.name.eq_ignore_ascii_case(name) {
                        sw.value = SwatchValue::Color { color: col, color_type: designcraft_color::ColorType::Process };
                        found = true;
                        break;
                    }
                }
                if !found {
                    d.swatches.push(Swatch::color(name, col));
                }
            }
        }

        // Adopt any missing swatches from template
        for tpl_sw in &tpl_doc.swatches {
            if !d.swatches.iter().any(|s| s.name.eq_ignore_ascii_case(&tpl_sw.name)) {
                d.swatches.push(tpl_sw.clone());
            }
        }

        // Adopt any missing paragraph styles from template
        for tpl_style in &tpl_doc.styles.paragraph {
            if !d.styles.paragraph.iter().any(|s| s.name == tpl_style.name) {
                d.styles_mut().paragraph.push(tpl_style.clone());
            }
        }

        // Copy template spread items into target spread
        let mut maps = crate::cmd::datamerge::fill::Maps::default();
        let spread_ref = SpreadRef::Doc(spread_idx);
        for it in &tpl_spread.items {
            copy_item(d, &tpl_doc, it, spread_ref, Vec2::ZERO, &mut maps)?;
        }

        let mut image_tasks = Vec::new();
        let mut story_tasks = Vec::new();
        let mut slots_bound = 0usize;
        let mut new_items = Vec::new();

        // Read every tagged item first: how many parts a text slot is divided into is the highest
        // `#n` suffix any frame of that key carries.
        struct Tagged {
            id: ItemId,
            kind: SlotType,
            key: String,
            part: u32,
            story: Option<StoryId>,
        }
        let mut tagged = Vec::new();
        for (&_old_id, &new_id) in &maps.items {
            new_items.push(new_id);
            let Some(it) = d.item(new_id) else { continue };
            let Some(tag) = parse_slot_tag(&it.label).or_else(|| parse_slot_tag(&it.name)) else { continue };
            let story = match &it.content {
                Content::Text(tf) => Some(tf.story),
                _ => None,
            };
            tagged.push(Tagged { id: new_id, kind: tag.kind, key: tag.key.to_string(), part: tag.part, story });
        }
        let mut parts = HashMap::<String, u32>::new();
        for t in &tagged {
            if t.kind == SlotType::Text {
                let e = parts.entry(t.key.clone()).or_insert(1);
                *e = (*e).max(t.part);
            }
        }

        // Bind the slots on the new items.
        for t in tagged {
            match t.kind {
                SlotType::Text => {
                    let Some(raw) = text_slots.get(&t.key) else { continue };
                    let count = parts.get(&t.key).copied().unwrap_or(1);
                    // One frame takes the value whole; several take one part each (see
                    // `split_slot_value`).
                    let piece = if count <= 1 {
                        Some(raw.clone())
                    } else {
                        split_slot_value(raw, count as usize).into_iter().nth(t.part.saturating_sub(1) as usize)
                    };
                    if let (Some(text_val), Some(sid)) = (piece, t.story)
                        && let Some(story) = d.story_mut(sid)
                    {
                        let len = story.len();
                        story.replace(0..len, &text_val);
                        slots_bound = slots_bound.saturating_add(1);
                    }
                }
                SlotType::Image => {
                    if let Some(img_path) = image_slots.get(&t.key) {
                        image_tasks.push((t.id, img_path.clone()));
                        slots_bound = slots_bound.saturating_add(1);
                    }
                }
                SlotType::Story => {
                    if let Some(story_path) = story_slots.get(&t.key) {
                        story_tasks.push((t.id, story_path.clone()));
                        slots_bound = slots_bound.saturating_add(1);
                    }
                }
                SlotType::Table => {
                    // Table slot handled in subsequent task
                }
            }
        }

        Ok(InsertedInfo { page_idx, spread_idx, new_items, image_tasks, story_tasks, slots_bound })
    })?;

    // 6. Execute external file placing tasks (Images and RTF Stories) via session commands
    let fit_mode = p.get("image_fitting").and_then(Value::as_str).unwrap_or("fitProportionally");
    for (fid, img_path) in inserted.image_tasks {
        if Path::new(&img_path).exists() {
            let _ = s.execute("file.place", &json!({"frame": fid.0, "path": img_path}));
            let _ = s.execute("object.fit", &json!({"mode": fit_mode, "ids": [fid.0]}));
            let _ = s.execute("object.fit", &json!({"mode": "centerContent", "ids": [fid.0]}));
        }
    }

    for (fid, rtf_path) in inserted.story_tasks {
        if Path::new(&rtf_path).exists() {
            let _ = s.execute("file.place", &json!({"frame": fid.0, "path": rtf_path}));
        }
    }

    Ok(json!({
        "page": inserted.page_idx,
        "spread": inserted.spread_idx,
        "items": inserted.new_items.iter().map(|id| id.0).collect::<Vec<_>>(),
        "slots_bound": inserted.slots_bound,
    }))
}
