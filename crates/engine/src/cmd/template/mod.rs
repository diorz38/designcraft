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
            "{path?: string, bytes?: [u8], base64?: string, lang?: string, before?: usize, after?: usize, target_page?: usize, position?: before|after, slots?: {text?: {...}, image?: {...}, story?: {...}}, palette?: {...}, image_fitting?: fitProportionally|fillProportionally|fitContentToFrame|none} → {page, spread, items: [id], slots_bound: count} — a text slot split over several frames labels them `#slot:text:<key>`, `#slot:text:<key>#2`, … and the value is divided between them (lines, else words, else characters: `2026` → `20`, `26`); a part may be given its own value as `<key>#<n>` (e.g. `title#2`), which is used as written and leaves the plain value to the parts that have none; an image slot takes its picture the way the frame wants it — a photograph covers the frame (the frame's shape cuts it), a logo fits inside it",
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

/// How a template's image frame takes the picture it is given.
///
/// A photograph covers its frame — the frame's shape (a cover's X, a chevron) cuts it, and the
/// picture fills the shape. A logo fits inside its frame: cropping a mark would cut it.
fn default_image_fit(key: &str) -> &'static str {
    match key {
        "logo" | "emblem" | "mark" => "fitProportionally",
        _ => "fillProportionally",
    }
}

#[allow(clippy::cognitive_complexity)]
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

    let spread_index = p.get("spread_index").or_else(|| p.get("spread")).and_then(Value::as_u64).unwrap_or(0) as usize;
    let tpl_spread =
        tpl_doc.spreads.get(spread_index).ok_or_else(|| bad("template.page.apply", format!("Template spread {spread_index} not found")))?;
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

    let lang_param = p.get("lang").and_then(Value::as_str).filter(|s| !s.trim().is_empty()).map(|s| s.trim().to_string());

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
        image_tasks: Vec<(ItemId, String, String)>,
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
        let mut dest_page_x = 0.0;
        if let Some(dest_sp) = d.spreads.get_mut(spread_idx) {
            let dest_sp_mut = Arc::make_mut(dest_sp);
            if let Some(dest_p) = dest_sp_mut.pages.get_mut(page_in_spread) {
                dest_p.width = tpl_w;
                dest_p.height = tpl_h;
                dest_p.margins = tpl_margins;
                dest_p.columns = tpl_cols;
            }
            dest_sp_mut.relayout();
            if let Some(dest_p) = dest_sp_mut.pages.get(page_in_spread) {
                dest_page_x = dest_p.x;
            }
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

        // Adopt any missing character styles from template
        for tpl_style in &tpl_doc.styles.character {
            if !d.styles.character.iter().any(|s| s.name == tpl_style.name) {
                d.styles_mut().character.push(tpl_style.clone());
            }
        }

        // Copy template spread items into target spread (offsetting by page position within spread)
        let mut maps = crate::cmd::datamerge::fill::Maps::default();
        let spread_ref = SpreadRef::Doc(spread_idx);
        let offset = Vec2::new(dest_page_x - tpl_page.x, 0.0);
        for it in &tpl_spread.items {
            copy_item(d, &tpl_doc, it, spread_ref, offset, &mut maps)?;
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
                    let count = parts.get(&t.key).copied().unwrap_or(1);
                    // A part may carry its own value (`title#2`): it is then used as written, and
                    // the plain value is divided over the parts that have none (see
                    // `split_slot_value`).
                    let mut own: HashMap<u32, String> = HashMap::new();
                    for part in 1..=count {
                        if let Some(v) = text_slots.get(&format!("{}#{}", t.key, part)) {
                            own.insert(part, v.clone());
                        }
                    }
                    let plain = text_slots.get(&t.key).cloned();
                    if plain.is_none() && own.is_empty() {
                        continue;
                    }
                    let free_parts: Vec<u32> = (1..=count).filter(|p| !own.contains_key(p)).collect();
                    let pieces =
                        if free_parts.is_empty() { Vec::new() } else { split_slot_value(plain.as_deref().unwrap_or_default(), free_parts.len()) };
                    let piece = if let Some(v) = own.get(&t.part) {
                        Some(v.clone())
                    } else {
                        free_parts.iter().position(|p| *p == t.part).and_then(|i| pieces.get(i).cloned())
                    };
                    if let (Some(text_val), Some(sid)) = (piece, t.story)
                        && let Some(story) = d.story_mut(sid)
                    {
                        let len = story.len();
                        story.replace(0..len, &text_val);
                        if let Some(lang) = &lang_param {
                            for r in &mut story.chars {
                                r.format.over.language = Some(lang.clone());
                            }
                        }
                        slots_bound = slots_bound.saturating_add(1);
                    }
                }
                SlotType::Image => {
                    if let Some(img_path) = image_slots.get(&t.key) {
                        image_tasks.push((t.id, img_path.clone(), t.key.clone()));
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
    let fit_mode = p.get("image_fitting").and_then(Value::as_str);
    for (fid, img_path, key) in inserted.image_tasks {
        if Path::new(&img_path).exists() {
            let _ = s.execute("file.place", &json!({"frame": fid.0, "path": img_path}));
            let mode = fit_mode.unwrap_or_else(|| default_image_fit(&key));
            let _ = s.execute("object.fit", &json!({"mode": mode, "ids": [fid.0]}));
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
