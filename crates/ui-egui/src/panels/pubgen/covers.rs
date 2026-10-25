//! The cover engines: front cover, inside cover and back cover from the cover state.

use super::*;

/// The world bounds of one item, read back through `document.inspect`.
fn item_bounds(app: &mut DesignApp, id: u64) -> Option<[f64; 4]> {
    fn scan(v: &Value, id: u64) -> Option<[f64; 4]> {
        let b = v.get("bounds").and_then(Value::as_array)?;
        if v.get("id").and_then(Value::as_u64) != Some(id) || b.len() != 4 {
            return None;
        }
        Some([b[0].as_f64()?, b[1].as_f64()?, b[2].as_f64()?, b[3].as_f64()?])
    }

    fn walk(v: &Value, id: u64) -> Option<[f64; 4]> {
        if let Some(b) = scan(v, id) {
            return Some(b);
        }
        match v {
            Value::Array(items) => items.iter().find_map(|it| walk(it, id)),
            Value::Object(obj) => obj.values().find_map(|it| walk(it, id)),
            _ => None,
        }
    }

    walk(&app.run("document.inspect", json!({})).ok()?, id)
}

/// The Pubgen sample-image folder, found from the working directory or the crate location
/// (tests run with the crate dir as the working directory, the app from the repo root).
fn sample_images_dir() -> Option<std::path::PathBuf> {
    let rel = std::path::Path::new("plan").join("pubgen").join("samples").join("images");
    if rel.is_dir() {
        return Some(rel);
    }
    let from_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(&rel);
    if from_manifest.is_dir() { Some(from_manifest) } else { None }
}

/// Resolve a cover image path: as given, or the same file name in the Pubgen sample images
/// (so the default `outputs/...` paths also resolve when the app runs from another directory).
fn resolve_cover_image(path: &str) -> String {
    if std::path::Path::new(path).is_file() {
        return path.to_string();
    }
    if let Some(name) = std::path::Path::new(path).file_name()
        && let Some(dir) = sample_images_dir()
    {
        let sample = dir.join(name);
        if sample.is_file() {
            return sample.to_string_lossy().to_string();
        }
    }
    path.to_string()
}

fn resolve_template_path(path: &str) -> Option<std::path::PathBuf> {
    let p = std::path::Path::new(path);
    if p.is_file() {
        return Some(p.to_path_buf());
    }
    let from_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(path);
    if from_manifest.is_file() {
        return Some(from_manifest);
    }
    None
}

fn convert_asset_id_to_grayscale(doc: &mut designcraft_doc::Document, aid: designcraft_doc::AssetId) -> Option<designcraft_doc::AssetId> {
    let asset = doc.assets.get(&aid)?.clone();
    let raw_bytes: Option<Vec<u8>> = if !asset.data.is_empty() {
        Some((*asset.data).clone())
    } else if let Some(link) = &asset.link {
        std::fs::read(link).ok()
    } else {
        None
    };
    let bytes = raw_bytes?;
    let img = image::load_from_memory(&bytes).ok()?;
    let gray = img.grayscale();
    let mut out_bytes = Vec::new();
    gray.write_to(&mut std::io::Cursor::new(&mut out_bytes), image::ImageFormat::Png).ok()?;
    let new_aid = designcraft_doc::AssetId(doc.alloc());
    let new_asset = designcraft_doc::Asset {
        id: new_aid,
        name: format!("{}_gray.png", asset.name.trim_end_matches(".png").trim_end_matches(".jpg")),
        mime: "image/png".into(),
        link: None,
        data: std::sync::Arc::new(out_bytes),
        pixels: Some((gray.width(), gray.height())),
        page: 0,
        pdf_crop: asset.pdf_crop,
        pdf_box: asset.pdf_box,
    };
    doc.assets.insert(new_aid, std::sync::Arc::new(new_asset));
    Some(new_aid)
}

fn transform_inside_cover_item_shapes(
    it: &mut designcraft_doc::Item,
    text_stories: &mut Vec<designcraft_doc::StoryId>,
    graphics: &mut Vec<(designcraft_doc::ItemId, designcraft_doc::AssetId)>,
) {
    match &mut it.content {
        designcraft_doc::Content::Text(tf) => {
            it.fill = designcraft_doc::Fill::none();
            text_stories.push(tf.story);
        }
        designcraft_doc::Content::Graphic(g) => {
            it.fill = designcraft_doc::Fill::none();
            graphics.push((it.id, g.asset));
        }
        designcraft_doc::Content::Group { items } => {
            it.fill = designcraft_doc::Fill::none();
            for sub in items.iter_mut() {
                let sub_item = std::sync::Arc::make_mut(sub);
                transform_inside_cover_item_shapes(sub_item, text_stories, graphics);
            }
        }
        designcraft_doc::Content::Unassigned => {
            it.fill = designcraft_doc::Fill::swatch("[Paper]");
            it.stroke = designcraft_doc::Stroke {
                swatch: designcraft_color::swatch::BLACK.to_string(),
                weight: 1.0,
                kind: designcraft_doc::StrokeType::Solid,
                align: designcraft_doc::StrokeAlign::Center,
                cap: designcraft_doc::Cap::Butt,
                join: designcraft_doc::Join::Miter,
                miter_limit: 4.0,
                ..designcraft_doc::Stroke::default()
            };
            it.effects = designcraft_doc::Effects::default();
            it.opacity = 1.0;
            it.blend = designcraft_color::BlendMode::Normal;
        }
    }
}

pub fn execute_generate_inside_cover(app: &mut DesignApp) -> Result<(), String> {
    let before_ids: std::collections::HashSet<designcraft_doc::ItemId> = {
        let active = app.session.active().ok_or("No active document")?;
        active.doc.spreads.iter().flat_map(|sp| sp.items.iter().map(|it| it.id)).collect()
    };

    app.run("layout.pages.duplicateSpread", json!({ "spread": 0 })).map_err(|e| format!("Failed to duplicate cover spread: {e}"))?;

    let (inside_si, inside_pi) = {
        let active = app.session.active().ok_or("No active document")?;
        active.doc.page_loc(1).ok_or("Inside cover page missing")?
    };

    let _ = app.session.edit(|doc, _| {
        let mut text_stories = Vec::new();
        let mut graphics = Vec::new();

        if let Some(sp) = doc.spreads.get_mut(inside_si) {
            let sp = std::sync::Arc::make_mut(sp);
            if let Some(pg) = sp.pages.get_mut(inside_pi) {
                pg.margins = designcraft_doc::Margins::uniform(0.0);
                pg.local_margins = true;
            }

            for it in &mut sp.items {
                if before_ids.contains(&it.id) {
                    continue;
                }
                let it_mut = std::sync::Arc::make_mut(it);
                transform_inside_cover_item_shapes(it_mut, &mut text_stories, &mut graphics);
            }
            sp.relayout();
        }

        // Convert graphic assets to grayscale without borrowing spreads
        for (item_id, old_aid) in graphics {
            if let Some(new_aid) = convert_asset_id_to_grayscale(doc, old_aid)
                && let Some(item) = doc.item_mut(item_id)
                && let designcraft_doc::Content::Graphic(g) = &mut item.content
            {
                g.asset = new_aid;
            }
        }

        for sid in text_stories {
            if let Some(story) = doc.story_mut(sid) {
                for r in &mut story.chars {
                    r.format.over.fill = Some(designcraft_color::swatch::BLACK.to_string());
                }
            }
        }

        Ok(Value::Null)
    });

    Ok(())
}

pub fn find_back_cover_page(doc: &designcraft_doc::Document) -> Option<usize> {
    fn item_is_back_cover(it: &designcraft_doc::Item, doc: &designcraft_doc::Document) -> bool {
        if it.label.contains("back_cover") || it.name.contains("back_cover") {
            return true;
        }
        if let designcraft_doc::Content::Graphic(g) = &it.content
            && doc
                .assets
                .get(&g.asset)
                .map(|a| a.name.contains("data_cerdas") || a.link.as_deref().map(|l| l.contains("data_cerdas")).unwrap_or(false))
                .unwrap_or(false)
        {
            return true;
        }
        if let designcraft_doc::Content::Group { items } = &it.content {
            return items.iter().any(|sub| item_is_back_cover(sub, doc));
        }
        false
    }

    for (si, sp) in doc.spreads.iter().enumerate() {
        for it in &sp.items {
            if item_is_back_cover(it, doc) {
                let pi = sp.page_at_x(it.bounds().center().x).unwrap_or(0);
                let first = doc.first_page_of_spread(si);
                return Some(first + pi);
            }
        }
    }
    None
}

pub fn ensure_back_cover_at_end(app: &mut DesignApp) {
    let (back_page, total) = {
        let Some(active) = app.session.active() else { return };
        let total = active.doc.page_count();
        let back_page = find_back_cover_page(&active.doc);
        (back_page, total)
    };
    if let Some(bp) = back_page
        && total > 0
        && bp != total - 1
    {
        let _ = app.run("layout.pages.move", json!({ "from": bp, "to": total - 1 }));
    }
}

pub struct MainCoverBranding {
    pub agency_text: String,
    pub agency_color: String,
    pub agency_font: String,
    pub code_text: String,
}

pub fn extract_main_cover_branding(doc: &designcraft_doc::Document, st: &PubgenState) -> MainCoverBranding {
    let mut agency_text = None;
    let mut agency_color = None;
    let mut agency_font = None;
    let mut code_text = None;

    if let Some(sp0) = doc.spreads.first() {
        fn inspect_item(
            it: &designcraft_doc::Item,
            doc: &designcraft_doc::Document,
            agency_text: &mut Option<String>,
            agency_color: &mut Option<String>,
            agency_font: &mut Option<String>,
            code_text: &mut Option<String>,
        ) {
            let lbl = it.label.to_lowercase();
            let nm = it.name.to_lowercase();
            if let designcraft_doc::Content::Text(tf) = &it.content {
                if let Some(story) = doc.story(tf.story) {
                    let txt = story.text.trim();
                    let is_agency =
                        lbl.contains("agency") || nm.contains("agency") || txt.contains("BADAN PUSAT STATISTIK") || txt.contains("STATISTICS");
                    let is_code = lbl.contains("code") || nm.contains("code") || txt.contains("ISSN") || txt.contains("ISBN");

                    if is_agency {
                        if agency_text.is_none() && !txt.is_empty() {
                            *agency_text = Some(txt.to_string());
                        }
                        for r in &story.chars {
                            if agency_color.is_none()
                                && let Some(ref f) = r.format.over.fill
                            {
                                *agency_color = Some(f.clone());
                            }
                            if agency_font.is_none()
                                && let Some(ref fnt) = r.format.over.font_family
                            {
                                *agency_font = Some(fnt.clone());
                            }
                        }
                    } else if is_code && code_text.is_none() && !txt.is_empty() {
                        *code_text = Some(txt.to_string());
                    }
                }
            } else if let designcraft_doc::Content::Group { items } = &it.content {
                for sub in items {
                    inspect_item(sub, doc, agency_text, agency_color, agency_font, code_text);
                }
            }
        }

        for it in &sp0.items {
            inspect_item(it, doc, &mut agency_text, &mut agency_color, &mut agency_font, &mut code_text);
        }
    }

    let default_agency = if !st.cover_agency_text.trim().is_empty() {
        st.cover_agency_text.trim().to_string()
    } else {
        "BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG\nBPS-STATISTICS KARAWANG REGENCY".to_string()
    };

    let default_code = {
        let code_type = if st.cover_code_type == 0 { "ISSN" } else { "ISBN" };
        let code_num = st.cover_code_num.trim();
        if code_num.is_empty() { code_type.to_string() } else { format!("{code_type} {code_num}") }
    };

    MainCoverBranding {
        agency_text: agency_text.unwrap_or(default_agency),
        agency_color: agency_color.unwrap_or_else(|| "#00aeef".to_string()),
        agency_font: agency_font.unwrap_or_else(|| "BIZ UDPGothic".to_string()),
        code_text: if !st.cover_code_num.trim().is_empty() { default_code } else { code_text.unwrap_or(default_code) },
    }
}

pub fn resolve_agency_swatch(doc: &mut designcraft_doc::Document, color_spec: &str) -> String {
    let clean = color_spec.trim();
    if clean.is_empty() {
        return ensure_swatch(doc, "#00aeef");
    }
    if doc.swatches.iter().any(|s| s.name.eq_ignore_ascii_case(clean)) {
        return clean.to_string();
    }
    ensure_swatch(doc, clean)
}

pub fn execute_generate_back_cover(app: &mut DesignApp, st: &PubgenState) -> Result<(), String> {
    let branding = {
        let active = app.session.active().ok_or("No active document")?;
        extract_main_cover_branding(&active.doc, st)
    };

    // Record existing item IDs so we NEVER modify or delete items belonging to existing pages
    // if the duplicated spread shares a spread with a previous body page (e.g. facing-pages mode).
    let before_ids: std::collections::HashSet<designcraft_doc::ItemId> = {
        let active = app.session.active().ok_or("No active document")?;
        active.doc.spreads.iter().flat_map(|sp| sp.items.iter().map(|it| it.id)).collect()
    };

    // 1. Copy all shapes from main cover by duplicating spread 0 at the end of document
    app.run("layout.pages.duplicateSpread", json!({ "spread": 0, "atEnd": true }))
        .map_err(|e| format!("Failed to duplicate cover spread for back cover: {e}"))?;

    ensure_back_cover_at_end(app);

    let last_page_idx = app.session.active().map(|s| s.doc.page_count().saturating_sub(1)).unwrap_or(0);
    let (back_si, back_pi) = {
        let active = app.session.active().ok_or("No active document")?;
        active.doc.page_loc(last_page_idx).ok_or("Back cover page location missing")?
    };

    let _ = app.session.edit(|doc, _| {
        if let Some(sp) = doc.spreads.get_mut(back_si) {
            let sp = std::sync::Arc::make_mut(sp);
            sp.allow_shuffle = false;
            if let Some(pg) = sp.pages.get_mut(back_pi) {
                pg.margins = designcraft_doc::Margins::uniform(0.0);
                pg.local_margins = true;
            }
            sp.relayout();
        }
        Ok(Value::Null)
    });

    let (pw, ph, page_x) = {
        let active = app.session.active().ok_or("No active document")?;
        let pg = &active.doc.spreads[back_si].pages[back_pi];
        (pg.width, pg.height, pg.x)
    };

    // Helper closure to create and place an image frame with locked positioning
    let place_locked_image = |app: &mut DesignApp, rect: [f64; 4], path: &str, label: &str| -> Result<Option<u64>, String> {
        let res = app.run(
            "frame.create",
            json!({
                "spread": back_si,
                "rect": rect,
                "content": "graphic",
                "caret": false,
            }),
        )?;
        let Some(fid) = res.get("id").and_then(Value::as_u64) else {
            return Ok(None);
        };
        let resolved = resolve_cover_image(path);
        if std::path::Path::new(&resolved).is_file() {
            let _ = app.run("file.place", json!({ "path": resolved, "frame": fid }));
            let _ = app.run("object.fit", json!({ "mode": "fitProportionally", "ids": [fid] }));
            let _ = app.run("object.fit", json!({ "mode": "centerContent", "ids": [fid] }));
        }
        let _ = app.session.edit(|doc, _| {
            if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
                it.stroke = designcraft_doc::Stroke::none();
                it.fill = designcraft_doc::Fill::none();
                it.locked = true;
                it.label = label.to_string();
            }
            Ok(Value::Null)
        });
        Ok(Some(fid))
    };

    fn transform_back_cover_item(it: &mut designcraft_doc::Item, dx: f64) -> bool {
        let lbl = it.label.to_lowercase();
        let nm = it.name.to_lowercase();

        if dx.abs() > 0.001 {
            it.xf = designcraft_geom::Affine::translate((dx, 0.0)) * it.xf;
        }

        match &mut it.content {
            designcraft_doc::Content::Text(_) => {
                // Remove all copied text frames; back cover elements are placed afresh at canonical positions
                false
            }
            designcraft_doc::Content::Graphic(_) => {
                let is_logo = lbl.contains("logo") || nm.contains("logo");
                if is_logo {
                    // Logo will be placed afresh at bottom left
                    false
                } else {
                    // "all shape that have image backgroud change to white color fill"
                    it.content = designcraft_doc::Content::Unassigned;
                    it.fill = designcraft_doc::Fill::swatch("[Paper]");
                    it.stroke = designcraft_doc::Stroke::none();
                    it.locked = true;
                    true
                }
            }
            designcraft_doc::Content::Group { items } => {
                it.locked = true;
                items.retain_mut(|sub| {
                    let sub_item = std::sync::Arc::make_mut(sub);
                    transform_back_cover_item(sub_item, dx)
                });
                true
            }
            _ => {
                let is_logo = lbl.contains("logo") || nm.contains("logo");
                let is_image_slot = lbl.contains("hero") || lbl.contains("photo") || lbl.contains("landscape") || (lbl.contains("image") && !is_logo);
                if is_image_slot {
                    it.content = designcraft_doc::Content::Unassigned;
                    it.fill = designcraft_doc::Fill::swatch("[Paper]");
                    it.stroke = designcraft_doc::Stroke::none();
                }
                it.locked = true;
                true
            }
        }
    }

    // 2. Filter copied items:
    // - Remove all text and old logo
    // - Convert shapes with image backgrounds to white fill ([Paper])
    // - Keep all other background decorative shapes
    let _ = app.session.edit(|doc, _| {
        if let Some(sp) = doc.spreads.get_mut(back_si) {
            let sp = std::sync::Arc::make_mut(sp);
            sp.items.retain_mut(|arc_it| {
                if before_ids.contains(&arc_it.id) {
                    return true;
                }
                let it = std::sync::Arc::make_mut(arc_it);
                transform_back_cover_item(it, page_x)
            });
            sp.relayout();
        }
        Ok(Value::Null)
    });

    // 3. Office Logo: ALWAYS set position on bottom left of page
    let logo_x = page_x + 36.0;
    let logo_y = ph - 100.0;
    let logo_w = 45.0;
    let logo_h = 40.0;
    let logo_path = if !st.cover_image_logo.trim().is_empty() { st.cover_image_logo.trim() } else { "plan/pubgen/samples/images/bps_logo.png" };
    let _ = place_locked_image(app, [logo_x, logo_y, logo_x + logo_w, logo_y + logo_h], logo_path, "#slot:back_cover:logo")?;

    // 4. Office Name: placed beside logo on bottom left, matching main cover color and font, bold
    let agency_x = logo_x + logo_w + 10.0;
    let agency_y = logo_y - 2.0;
    let agency_w = 260.0;
    let agency_h = logo_h + 4.0;
    let res_agency = app.run(
        "frame.create",
        json!({
            "spread": back_si,
            "rect": [agency_x, agency_y, agency_x + agency_w, agency_y + agency_h],
            "content": "text",
            "text": branding.agency_text,
            "caret": false,
        }),
    )?;
    if let Some(fid) = res_agency.get("id").and_then(Value::as_u64) {
        let _ = app.session.edit(|doc, _| {
            let agency_swatch = resolve_agency_swatch(doc, &branding.agency_color);
            let mut story_id = None;
            if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
                it.fill = designcraft_doc::Fill::none();
                it.stroke = designcraft_doc::Stroke::none();
                it.locked = true;
                it.label = "#slot:back_cover:agency".to_string();
                if let designcraft_doc::Content::Text(tf) = &it.content {
                    story_id = Some(tf.story);
                }
            }
            if let Some(sid) = story_id
                && let Some(story) = doc.story_mut(sid)
            {
                for r in &mut story.chars {
                    r.format.over.size = Some(8.5);
                    r.format.over.font_style = Some("Bold".to_string());
                    r.format.over.fill = Some(agency_swatch.clone());
                    r.format.over.font_family = Some(branding.agency_font.clone());
                }
            }
            Ok(Value::Null)
        });
    }

    // 5. Office Address: placed after (below) logo and name on bottom left, matching main cover color and font, standard font size (no bold)
    let address_text = if !st.cover_office_address.trim().is_empty() {
        st.cover_office_address.trim().to_string()
    } else {
        "Jl. Cakradireja No.36 Karawang 41312\nJawa Barat Telp.: (0267) 402250; Fax.: (0267) 8452148; Email: kabkarawang@bps.go.id; pst3215@bps.go.id"
            .to_string()
    };
    let addr_x = logo_x;
    let addr_y = logo_y + logo_h + 4.0;
    let addr_w = 340.0;
    let addr_h = 24.0;
    let res_addr = app.run(
        "frame.create",
        json!({
            "spread": back_si,
            "rect": [addr_x, addr_y, addr_x + addr_w, addr_y + addr_h],
            "content": "text",
            "text": address_text,
            "caret": false,
        }),
    )?;
    if let Some(fid) = res_addr.get("id").and_then(Value::as_u64) {
        let _ = app.session.edit(|doc, _| {
            let agency_swatch = resolve_agency_swatch(doc, &branding.agency_color);
            let mut story_id = None;
            if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
                it.fill = designcraft_doc::Fill::none();
                it.stroke = designcraft_doc::Stroke::none();
                it.locked = true;
                it.label = "#slot:back_cover:address".to_string();
                if let designcraft_doc::Content::Text(tf) = &it.content {
                    story_id = Some(tf.story);
                }
            }
            if let Some(sid) = story_id
                && let Some(story) = doc.story_mut(sid)
            {
                for r in &mut story.chars {
                    r.format.over.size = Some(7.5);
                    r.format.over.font_style = Some("Regular".to_string());
                    r.format.over.fill = Some(agency_swatch.clone());
                    r.format.over.font_family = Some(branding.agency_font.clone());
                }
            }
            Ok(Value::Null)
        });
    }

    // 6. ISSN / ISBN: position is bottom right side, font name and text color same exactly like office_name with standard font size (no bold)
    let issn_w = 140.0;
    let issn_x = page_x + pw - 36.0 - issn_w;
    let issn_y = addr_y;
    let issn_h = addr_h;
    let res_issn = app.run(
        "frame.create",
        json!({
            "spread": back_si,
            "rect": [issn_x, issn_y, page_x + pw - 36.0, issn_y + issn_h],
            "content": "text",
            "text": branding.code_text,
            "caret": false,
        }),
    )?;
    if let Some(fid) = res_issn.get("id").and_then(Value::as_u64) {
        let _ = app.session.edit(|doc, _| {
            let agency_swatch = resolve_agency_swatch(doc, &branding.agency_color);
            let mut story_id = None;
            if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
                it.fill = designcraft_doc::Fill::none();
                it.stroke = designcraft_doc::Stroke::none();
                it.locked = true;
                it.label = "#slot:back_cover:code".to_string();
                if let designcraft_doc::Content::Text(tf) = &mut it.content {
                    story_id = Some(tf.story);
                }
            }
            if let Some(sid) = story_id
                && let Some(story) = doc.story_mut(sid)
            {
                for r in &mut story.chars {
                    r.format.over.size = Some(7.5);
                    r.format.over.font_style = Some("Regular".to_string());
                    r.format.over.fill = Some(agency_swatch.clone());
                    r.format.over.font_family = Some(branding.agency_font.clone());
                }
                for p in &mut story.paras {
                    p.para.align = Some(designcraft_doc::Align::Right);
                }
            }
            Ok(Value::Null)
        });
    }

    // 7. Add Image specifically only for back cover:
    // 7a. Top-right Stack: 3 images (event_theme, berakhlak, bangga_melayani_bangsa)
    let tr_w = 85.0;
    let tr_x = page_x + pw - 36.0 - tr_w;
    let event_theme_path = if !st.cover_image_back.trim().is_empty() { st.cover_image_back.trim() } else { "plan/pubgen/samples/images/se2026.png" };
    let _ = place_locked_image(app, [tr_x, 36.0, tr_x + tr_w, 72.0], event_theme_path, "#slot:back_cover:top_right")?;
    let _ = place_locked_image(app, [tr_x, 78.0, tr_x + tr_w, 112.0], "plan/pubgen/samples/images/berakhlak.png", "#slot:back_cover:top_right")?;
    let _ = place_locked_image(
        app,
        [tr_x, 118.0, tr_x + tr_w, 154.0],
        "plan/pubgen/samples/images/bangga_melayani_bangsa.png",
        "#slot:back_cover:top_right",
    )?;

    // 7b. Center Section: Slogan (data_cerdas.png) & Graphic Placeholder Frame
    let center_logo_w = 300.0;
    let center_logo_h = 110.0;
    let center_x = page_x + (pw - center_logo_w) / 2.0;
    let center_y = ph * 0.36;
    let _ = place_locked_image(
        app,
        [center_x, center_y, center_x + center_logo_w, center_y + center_logo_h],
        "plan/pubgen/samples/images/data_cerdas.png",
        "#slot:back_cover:center",
    )?;

    let cam_w = 140.0;
    let cam_h = 100.0;
    let cam_x = page_x + (pw - cam_w) / 2.0;
    let cam_y = ph * 0.54;
    let res_cam = app.run(
        "frame.create",
        json!({
            "spread": back_si,
            "rect": [cam_x, cam_y, cam_x + cam_w, cam_y + cam_h],
            "content": "graphic",
            "caret": false,
        }),
    )?;
    if let Some(fid) = res_cam.get("id").and_then(Value::as_u64) {
        let _ = app.session.edit(|doc, _| {
            if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
                it.stroke = designcraft_doc::Stroke::none();
                it.fill = designcraft_doc::Fill::none();
                it.locked = true;
                it.label = "#slot:back_cover:camera".to_string();
            }
            Ok(Value::Null)
        });
    }

    // 8. Ensure all items on the back cover spread are locked
    let _ = app.session.edit(|doc, _| {
        if let Some(sp) = doc.spreads.get_mut(back_si) {
            let sp = std::sync::Arc::make_mut(sp);
            for it in &mut sp.items {
                let it_mut = std::sync::Arc::make_mut(it);
                it_mut.locked = true;
            }
        }
        Ok(Value::Null)
    });

    ensure_back_cover_at_end(app);

    Ok(())
}

pub fn execute_generate_front_cover(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    let title_text = if st.cover_title.trim().is_empty() { "KARAWANG TIMUR" } else { st.cover_title.trim() };
    // The second headline line has its own frame in the cover templates (`title#2`), styled
    // smaller and in another colour than the headline above it. An empty field leaves that
    // frame blank.
    let title2_text = if st.bilingual_output {
        let t2 = st.cover_title2.trim();
        let ten = if !st.cover_title_en.trim().is_empty() {
            st.cover_title_en.trim()
        } else if st.translate_with_ai {
            st.translations.get(title_text.trim()).map(String::as_str).unwrap_or("")
        } else {
            ""
        };

        if !t2.is_empty() && !ten.is_empty() {
            if t2.eq_ignore_ascii_case(ten) || t2.to_lowercase().contains(&ten.to_lowercase()) {
                t2.to_string()
            } else if ten.to_lowercase().contains(&t2.to_lowercase()) {
                ten.to_string()
            } else {
                format!("{t2} / {ten}")
            }
        } else if !t2.is_empty() {
            t2.to_string()
        } else {
            ten.to_string()
        }
    } else if !st.cover_title2.trim().is_empty() {
        st.cover_title2.trim().to_string()
    } else {
        st.cover_title_en.trim().to_string()
    };
    let year_text = if st.cover_year.trim().is_empty() { "2026" } else { st.cover_year.trim() };
    let volume_text = if st.cover_volume.trim().is_empty() { "Volume 23 2026" } else { st.cover_volume.trim() };

    let cat_raw = if st.cover_catalogue.trim().is_empty() { "1102001.3215113" } else { st.cover_catalogue.trim() };
    let cat_text = if cat_raw.to_lowercase().starts_with("katalog") { cat_raw.to_string() } else { format!("Katalog BPS: {cat_raw}") };

    let code_type = if st.cover_code_type == 0 { "ISSN" } else { "ISBN" };
    let code_num = if st.cover_code_num.trim().is_empty() { "2828-7436" } else { st.cover_code_num.trim() };
    let code_text = format!("{code_type} {code_num}");

    let primary = if st.cover_color_primary.trim().is_empty() { "#0072bc" } else { st.cover_color_primary.trim() };
    let accent = if st.cover_color_accent.trim().is_empty() { "#f5ab19" } else { st.cover_color_accent.trim() };
    let dark = if st.cover_color_dark.trim().is_empty() { "#122647" } else { st.cover_color_dark.trim() };

    let skyline =
        resolve_cover_image(if st.cover_image_skyline.trim().is_empty() { "outputs/city_skyline.jpg" } else { st.cover_image_skyline.trim() });
    let landscape =
        resolve_cover_image(if st.cover_image_landscape.trim().is_empty() { "outputs/city_landscape.jpg" } else { st.cover_image_landscape.trim() });
    let logo = resolve_cover_image(if st.cover_image_logo.trim().is_empty() { "outputs/bps_logo.png" } else { st.cover_image_logo.trim() });

    let agency_text = if st.cover_agency_text.trim().is_empty() { "BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG" } else { st.cover_agency_text.trim() };

    // The hand-designed covers in `covers/`: A4 artboards with the artwork's transforms intact
    // and every text/photo frame carrying a `#slot:` tag (a headline split over two frames uses
    // `title` and `title#2`). `covers_original/` keeps the untouched sources.
    let canonical_templates = [
        "assets/templates/covers/000_wave_curves.designcraft",
        "assets/templates/covers/001_geometric_medallion.designcraft",
        "assets/templates/covers/002_folded_origami.designcraft",
        "assets/templates/covers/003_chevron_medallion.designcraft",
        "assets/templates/covers/004_honeycomb_hexagon.designcraft",
        "assets/templates/covers/005_corporate_headline.designcraft",
        "assets/templates/covers/006_diagonal_arrow.designcraft",
        "assets/templates/covers/007_modern_horizon.designcraft",
        "assets/templates/covers/008_business_report.designcraft",
        "assets/templates/covers/009_urban_angle.designcraft",
        "assets/templates/covers/010_tri_color_bands.designcraft",
        "assets/templates/covers/011_prism_diamonds.designcraft",
        "assets/templates/covers/012_quarter_stack.designcraft",
    ];

    if let Some(&tpl_path) = canonical_templates.get(st.cover_style)
        && let Some(tpl_file) = resolve_template_path(tpl_path)
    {
        let tpl_path_str = tpl_file.to_string_lossy().to_string();
        let mut text_slots = serde_json::Map::new();
        text_slots.insert("title".into(), json!(title_text));
        text_slots.insert("title#2".into(), json!(title2_text));
        // If a template has an explicit separate title_en slot, only bind it if title#2 does not already incorporate it.
        let title_en_candidate = if !st.cover_title_en.trim().is_empty() {
            st.cover_title_en.trim()
        } else if st.translate_with_ai {
            st.translations.get(title_text.trim()).map(String::as_str).unwrap_or("")
        } else {
            ""
        };
        if st.bilingual_output
            && !title_en_candidate.is_empty()
            && !title2_text.eq_ignore_ascii_case(title_en_candidate)
            && !title2_text.to_lowercase().contains(&title_en_candidate.to_lowercase())
        {
            text_slots.insert("title_en".into(), json!(title_en_candidate));
        }
        text_slots.insert("year".into(), json!(year_text));
        text_slots.insert("volume".into(), json!(volume_text));
        text_slots.insert("catalogue".into(), json!(cat_text));
        text_slots.insert("code".into(), json!(code_text));
        text_slots.insert("agency".into(), json!(agency_text));

        let mut apply_params = json!({
            "path": tpl_path_str,
            "before": 0,
            "slots": {
                "text": text_slots,
                "image": {
                    "hero": skyline,
                    "landscape": landscape,
                    "logo": logo,
                }
            },
            "palette": {
                "Primary": primary,
                "Accent": accent,
                "Dark": dark,
            }
        });
        if !st.doc_language.trim().is_empty() {
            apply_params["lang"] = json!(st.doc_language.trim());
        }

        let res = app.run("template.page.apply", apply_params);
        if res.is_ok() {
            crate::canvas::go_to_page(app, 0);
            let _ = app.session.edit(|doc, _| {
                let mut title_en_stories = Vec::new();
                let mut redundant_items = Vec::new();
                if let Some(sp) = doc.spreads.first() {
                    for item in &sp.items {
                        if item.label.starts_with("#slot:text:title_en") || item.name.starts_with("#slot:text:title_en") {
                            if let designcraft_doc::Content::Text(tf) = &item.content
                                && let Some(story) = doc.story(tf.story)
                            {
                                if story.text.trim().is_empty() || title2_text.eq_ignore_ascii_case(story.text.trim()) {
                                    redundant_items.push(item.id);
                                } else {
                                    title_en_stories.push(tf.story);
                                }
                            } else {
                                redundant_items.push(item.id);
                            }
                        }
                    }
                }
                if let Some(sp) = doc.spreads.first_mut() {
                    let sp = std::sync::Arc::make_mut(sp);
                    if let Some(pg) = sp.pages.first_mut() {
                        pg.margins = designcraft_doc::Margins::uniform(0.0);
                        pg.local_margins = true;
                    }
                    if !redundant_items.is_empty() {
                        sp.items.retain(|it| !redundant_items.contains(&it.id));
                    }
                    sp.relayout();
                }
                for sid in title_en_stories {
                    if let Some(story) = doc.story_mut(sid) {
                        for r in &mut story.chars {
                            r.format.over.font_style = Some("Bold Italic".to_string());
                        }
                    }
                }
                if st.bilingual_output {
                    let mut title2_stories = Vec::new();
                    if let Some(sp) = doc.spreads.first() {
                        for item in &sp.items {
                            if (item.label.starts_with("#slot:text:title#2") || item.name.starts_with("#slot:text:title#2"))
                                && let designcraft_doc::Content::Text(tf) = &item.content
                            {
                                title2_stories.push(tf.story);
                            }
                        }
                    }
                    for sid in title2_stories {
                        if let Some(story) = doc.story_mut(sid) {
                            if let Some(slash_idx) = story.text.find(" / ") {
                                let split_char_pos = slash_idx + 3;
                                let mut offset = 0;
                                for r in &mut story.chars {
                                    let run_end = offset + r.len;
                                    offset = run_end;
                                    if run_end > split_char_pos {
                                        r.format.over.font_style = Some("Bold Italic".to_string());
                                    }
                                }
                            } else {
                                for r in &mut story.chars {
                                    r.format.over.font_style = Some("Bold Italic".to_string());
                                }
                            }
                        }
                    }
                }
                Ok(serde_json::Value::Null)
            });
            let style_names = [
                "Wave Curves",
                "Geometric Medallion",
                "Folded Origami",
                "Chevron & Medallion",
                "Honeycomb Hexagon",
                "Corporate Headline",
                "Diagonal Arrow",
                "Modern Horizon",
                "Business Report",
                "Urban Angle",
                "Tri-Color Bands",
                "Prism Diamonds",
                "Quarter Stack",
            ];
            let sname = style_names.get(st.cover_style).copied().unwrap_or("Template Cover");
            return Ok(sname.to_string());
        }
    }

    // 1. Automatically insert a new page before the first page (Page 1)
    app.run("layout.pages.insert", json!({"before": 0, "count": 1})).map_err(|e| format!("Failed to insert cover page: {e}"))?;
    crate::canvas::go_to_page(app, 0);
    let _ = app.session.edit(|doc, _| {
        if let Some(sp) = doc.spreads.first_mut() {
            let sp = std::sync::Arc::make_mut(sp);
            if let Some(pg) = sp.pages.first_mut() {
                pg.margins = designcraft_doc::Margins::uniform(0.0);
                pg.local_margins = true;
            }
            sp.relayout();
        }
        Ok(serde_json::Value::Null)
    });

    let (pw, ph) = {
        let active = app.session.active().ok_or("No active document")?;
        let spread = active.doc.spreads.first().ok_or("Spread missing")?;
        let page = spread.pages.first().ok_or("Page missing")?;
        (page.width, page.height)
    };

    let art = CoverArtVals {
        pw,
        ph,
        primary,
        accent,
        dark,
        title_text,
        title2_text: &title2_text,
        year_text,
        volume_text,
        cat_text: cat_text.as_str(),
        code_text: code_text.as_str(),
        agency_text,
        logo: logo.as_str(),
        skyline: skyline.as_str(),
        landscape: landscape.as_str(),
    };
    let style_name = match st.cover_style {
        0 => {
            cover_art::cover_style_0(app, art)?;
            "Wave Curves"
        }
        1 => {
            cover_art::cover_style_1(app, art)?;
            "Geometric & Medallion"
        }
        2 => {
            cover_art::cover_style_2(app, art)?;
            "Folded Origami"
        }
        3 => {
            cover_art::cover_style_3(app, art)?;
            "Chevron & Medallion"
        }
        4 => {
            cover_art::cover_style_4(app, art)?;
            "Honeycomb Hexagon"
        }
        5 => {
            cover_art::cover_style_5(app, art)?;
            "Corporate Headline"
        }
        6 => {
            cover_art::cover_style_6(app, art)?;
            "Diagonal Arrow"
        }
        7 => {
            cover_art::cover_style_7(app, art)?;
            "Modern Horizon"
        }
        8 => {
            cover_art::cover_style_8(app, art)?;
            "Business Report"
        }
        9 => {
            cover_art::cover_style_9(app, art)?;
            "Urban Angle"
        }
        10 => {
            return Err("The Tri-Color Bands cover template could not be applied".into());
        }
        11 => {
            return Err("The Prism Diamonds cover template could not be applied".into());
        }
        12 => {
            return Err("The Quarter Stack cover template could not be applied".into());
        }
        _ => return Err(format!("Unsupported cover style: {}", st.cover_style)),
    };

    Ok(style_name.to_string())
}

pub fn execute_generate_cover(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    let sname = execute_generate_front_cover(app, st)?;
    if st.cover_include_inside_back {
        execute_generate_inside_cover(app)?;
        execute_generate_back_cover(app, st)?;
        ensure_back_cover_at_end(app);
        Ok(format!("Cover pages generated successfully: Front Cover, Inside Cover, Back Cover ({sname})"))
    } else {
        Ok(format!("Cover page generated successfully ({sname})"))
    }
}
