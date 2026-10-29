use super::*;
use designcraft_imagegen as imagegen;

/// Index of the "Modern Horizon" cover style — the one traced from the reference annual-report
/// design, whose palette is navy + orange rather than the amber of the other presets.
pub(crate) const HORIZON_STYLE: usize = 7;

/// Index of the "Business Report" cover style — the navy/orange diagonal cover derived from the
/// business-report reference image, sharing the Horizon palette.
pub(crate) const BUSINESS_STYLE: usize = 8;

/// Index of the "Urban Angle" cover style — the diagonal-wedge cityscape cover derived from
/// `design-6.designcraft`, sharing the Report palette.
pub(crate) const URBAN_STYLE: usize = 9;

/// Index of the "Tri-Color Bands" cover style — the executive blue/green/orange template.
pub(crate) const TRICOLOR_STYLE: usize = 10;

/// Index of the "Prism Diamonds" cover style — the jewelled diamond cascade on a midnight field.
pub(crate) const PRISM_STYLE: usize = 11;

/// Index of the "Quarter Stack" cover style — the fanned cards on a warm paper field.
pub(crate) const QUARTER_STYLE: usize = 12;

/// Theme preset index and colours of the reference annual-report cover (navy + orange).
pub(crate) const REPORT_THEME: usize = 6;

/// Select the palette the reference design uses, so a freshly picked Modern Horizon cover already
/// looks like the reference instead of the amber default.
pub(crate) fn apply_report_palette(st: &mut PubgenState) {
    st.cover_theme_preset = REPORT_THEME;
    st.cover_color_primary = "#082641".into();
    st.cover_color_accent = "#f5871f".into();
    st.cover_color_dark = "#082641".into();
}

/// The traced design-6 palette: deep petrol navy and warm marigold.
pub(crate) fn apply_urban_palette(st: &mut PubgenState) {
    st.cover_theme_preset = REPORT_THEME;
    st.cover_color_primary = "#034460".into();
    st.cover_color_accent = "#fcb91e".into();
    st.cover_color_dark = "#034460".into();
}

/// The world bounds of one item, read back through `document.inspect`.
pub(crate) fn item_bounds(app: &mut DesignApp, id: u64) -> Option<[f64; 4]> {
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
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        let from_exe = exe_dir.join(path);
        if from_exe.is_file() {
            return Some(from_exe);
        }
    }
    None
}

fn convert_asset_id_to_grayscale_mode(
    doc: &mut designcraft_doc::Document,
    aid: designcraft_doc::AssetId,
    is_light: bool,
) -> Option<designcraft_doc::AssetId> {
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
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let mut out_img = image::RgbaImage::new(w, h);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        if a == 0 {
            out_img.put_pixel(x, y, image::Rgba([255, 255, 255, 0]));
            continue;
        }
        let luma = 0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32);
        let val = if is_light {
            // Map [0.0, 1.0] to light grayscale range [215, 250] (high-key watermark)
            // ensuring all black text on top is 100% readable while preserving artwork details.
            let norm = luma / 255.0;
            (215.0 + norm * 35.0).round().clamp(0.0, 255.0) as u8
        } else {
            luma.round().clamp(0.0, 255.0) as u8
        };
        out_img.put_pixel(x, y, image::Rgba([val, val, val, a]));
    }
    let mut out_bytes = Vec::new();
    out_img.write_to(&mut std::io::Cursor::new(&mut out_bytes), image::ImageFormat::Png).ok()?;
    let new_aid = designcraft_doc::AssetId(doc.alloc());
    let prefix = if is_light { "light_gray" } else { "gray" };
    let new_asset = designcraft_doc::Asset {
        id: new_aid,
        name: format!("{}_{}.png", asset.name.trim_end_matches(".png").trim_end_matches(".jpg"), prefix),
        mime: "image/png".into(),
        link: None,
        data: std::sync::Arc::new(out_bytes),
        pixels: Some((w, h)),
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
    graphics: &mut Vec<(designcraft_doc::ItemId, designcraft_doc::AssetId, bool)>,
) {
    let lbl = it.label.to_lowercase();
    let nm = it.name.to_lowercase();
    match &mut it.content {
        designcraft_doc::Content::Text(tf) => {
            it.fill = designcraft_doc::Fill::none();
            text_stories.push(tf.story);
        }
        designcraft_doc::Content::Graphic(g) => {
            it.fill = designcraft_doc::Fill::none();
            let is_ai_bg = lbl.contains("background") || lbl.contains("ai_preset") || nm.contains("background");
            graphics.push((it.id, g.asset, is_ai_bg));
        }
        designcraft_doc::Content::Group { items } => {
            it.fill = designcraft_doc::Fill::none();
            for sub in items.iter_mut() {
                let sub_item = std::sync::Arc::make_mut(sub);
                transform_inside_cover_item_shapes(sub_item, text_stories, graphics);
            }
        }
        designcraft_doc::Content::Unassigned => {
            let is_band = lbl.contains("accent") || lbl.contains("band") || nm.contains("band");
            if is_band {
                it.fill = designcraft_doc::Fill::none();
                it.stroke = designcraft_doc::Stroke::none();
            } else {
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
            }
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
        for (item_id, old_aid, is_ai_bg) in graphics {
            if let Some(new_aid) = convert_asset_id_to_grayscale_mode(doc, old_aid, is_ai_bg)
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

/// Does this item (or anything nested inside a group) carry an `#slot:image:` tag?
fn item_has_image_slot(it: &designcraft_doc::Item) -> bool {
    if it.label.contains("#slot:image:") || it.name.contains("#slot:image:") {
        return true;
    }
    if let designcraft_doc::Content::Group { items } = &it.content {
        return items.iter().any(|sub| item_has_image_slot(sub));
    }
    false
}

/// A page this app generated as a cover. Every cover page it makes gets zero
/// local margins, and its artwork either carries an `#slot:image:` frame (the
/// template covers) or a full-bleed background shape (the procedural covers).
/// The zero-margins requirement is what separates a cover from the batik
/// divider template, which also has an image slot but keeps its 36 pt margins;
/// body templates never drop their margins.
pub(crate) fn is_cover_page(doc: &designcraft_doc::Document, page_idx: usize) -> bool {
    let Some((si, pi)) = doc.page_loc(page_idx) else { return false };
    let Some(sp) = doc.spreads.get(si) else { return false };
    let Some(pg) = sp.pages.get(pi) else { return false };
    if !pg.local_margins || pg.margins != designcraft_doc::Margins::uniform(0.0) {
        return false;
    }
    let page = pg.bounds();
    let page_area = page.width() * page.height();
    if page_area <= 0.0 {
        return false;
    }
    sp.items.iter().any(|it| {
        if sp.page_at_x(it.bounds().center().x) != Some(pi) {
            return false;
        }
        if item_has_image_slot(it) {
            return true;
        }
        // Procedural covers draw their background as a plain shape without a
        // slot tag: a shape covering most of the page marks the cover too.
        let inter = it.bounds().intersect(page);
        inter.width() * inter.height() >= 0.85 * page_area
    })
}

/// The cover pages already in the document: front (page 0), inside (page 1)
/// and back (wherever `find_back_cover_page` finds it). `None` when absent.
pub(crate) fn find_existing_covers(doc: &designcraft_doc::Document) -> (Option<usize>, Option<usize>, Option<usize>) {
    let front = is_cover_page(doc, 0).then_some(0);
    let inside = (doc.page_count() > 1 && is_cover_page(doc, 1)).then_some(1);
    let back = find_back_cover_page(doc);
    (front, inside, back)
}

/// With "Replace existing cover" checked, delete the cover pages already in the
/// document so regeneration starts clean instead of stacking a new cover in
/// front. Detection is smart: whatever kind of cover set is there — front only
/// or front + inside + back — goes, and regeneration then builds exactly the
/// set the current options ask for. With no cover page present this changes
/// nothing, so the box only matters when there is something to replace.
pub(crate) fn replace_existing_covers(app: &mut DesignApp, st: &PubgenState) -> Result<(), String> {
    if !st.cover_replace {
        return Ok(());
    }
    let (front, inside, back, page_count) = {
        let active = app.session.active().ok_or("No active document")?;
        let (f, i, b) = find_existing_covers(&active.doc);
        (f, i, b, active.doc.page_count())
    };
    let mut doomed: Vec<usize> = [front, inside, back].into_iter().flatten().collect();
    doomed.sort_unstable();
    doomed.dedup();
    if doomed.is_empty() {
        return Ok(());
    }
    // A document needs at least one page: when every page is a cover, first
    // keep a blank page at the end so the deletion stays legal.
    if doomed.len() >= page_count {
        app.run("layout.pages.insert", json!({ "before": page_count, "count": 1 }))
            .map_err(|e| format!("Failed to keep a page while replacing covers: {e}"))?;
    }
    // Highest index first: the indices of the remaining doomed pages stay valid.
    doomed.sort_unstable_by(|a, b| b.cmp(a));
    app.run("layout.pages.delete", json!({ "pages": doomed })).map_err(|e| format!("Failed to delete existing cover pages: {e}"))?;
    Ok(())
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

        let bounds = it.inner_bounds();
        match &mut it.content {
            designcraft_doc::Content::Text(_) => {
                // Remove all copied text frames; back cover elements are placed afresh at canonical positions
                false
            }
            designcraft_doc::Content::Graphic(g) => {
                let is_logo = lbl.contains("logo") || nm.contains("logo");
                if is_logo {
                    // Logo will be placed afresh at bottom left
                    false
                } else {
                    let is_ai_bg = lbl.contains("background") || lbl.contains("ai_preset") || nm.contains("background");
                    if is_ai_bg {
                        g.fit_align = 3; // Center-Left: crop left half of Double A4 spread for Back Cover
                        if let Some(xf) = g.fitted(bounds, designcraft_doc::Fitting::FillProportionally) {
                            g.xf = xf;
                        }
                        it.locked = true;
                        true
                    } else {
                        // "all shape that have image backgroud change to white color fill"
                        it.content = designcraft_doc::Content::Unassigned;
                        it.fill = designcraft_doc::Fill::swatch("[Paper]");
                        it.stroke = designcraft_doc::Stroke::none();
                        it.locked = true;
                        true
                    }
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

pub(crate) fn get_cover_text_vals(st: &PubgenState) -> (&str, String, &str, &str, String, String) {
    let title_text = if st.cover_title.trim().is_empty() { "KARAWANG TIMUR" } else { st.cover_title.trim() };
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

    (title_text, title2_text, year_text, volume_text, cat_text, code_text)
}

pub fn execute_generate_front_cover(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    let (title_text, title2_text, year_text, volume_text, cat_text, code_text) = get_cover_text_vals(st);

    let primary = if st.cover_color_primary.trim().is_empty() { "#0072bc" } else { st.cover_color_primary.trim() };
    let accent = if st.cover_color_accent.trim().is_empty() { "#f5ab19" } else { st.cover_color_accent.trim() };
    let dark = if st.cover_color_dark.trim().is_empty() { "#122647" } else { st.cover_color_dark.trim() };

    let skyline =
        resolve_cover_image(if st.cover_image_skyline.trim().is_empty() { "outputs/city_skyline.jpg" } else { st.cover_image_skyline.trim() });
    let landscape =
        resolve_cover_image(if st.cover_image_landscape.trim().is_empty() { "outputs/city_landscape.jpg" } else { st.cover_image_landscape.trim() });
    let logo = resolve_cover_image(if st.cover_image_logo.trim().is_empty() { "outputs/bps_logo.png" } else { st.cover_image_logo.trim() });

    // AI Base styles (100+) generate their background online; the procedural template
    // paths below never see them.
    if super::ai_cover::preset_index(st.cover_style).is_some() {
        return super::ai_cover::execute_generate_ai_cover(app, st);
    }

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
            if st.cover_add_map {
                let _ = apply_cover_map_overlay(app, st);
            }
            return Ok(sname.to_string());
        }
    }

    if st.cover_style >= 100 {
        return apply_ai_cover_result(app, st, "");
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

    if st.cover_add_map {
        let _ = apply_cover_map_overlay(app, st);
    }
    Ok(style_name.to_string())
}

pub fn execute_generate_cover(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    replace_existing_covers(app, st)?;
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

/// Dimensions of the first page (or standard A4 default if no document is open).
#[allow(dead_code)]
/// Overlays a stylized vector map on the front cover spread (Spread 0) based on `st.cover_add_map`.
pub fn apply_cover_map_overlay(app: &mut DesignApp, st: &PubgenState) -> Result<(), String> {
    let map = if st.cover_map_stock_idx == 999 && !st.cover_map_custom_path.trim().is_empty() {
        super::charts::stock_maps::load_cover_geo_map(&st.cover_map_custom_path)?
    } else {
        let stock_maps = super::charts::stock_maps::list_stock_cover_maps();
        if let Some(m) = stock_maps.get(st.cover_map_stock_idx) {
            super::charts::stock_maps::load_cover_geo_map(&m.path.to_string_lossy())?
        } else {
            super::charts::stock_maps::load_cover_geo_map("PANGKALAN")?
        }
    };

    let (pw, ph) = get_cover_page_dimensions(app);

    // Target rectangle for the map on the cover page.
    // Scaled larger and positioned across lower-middle to bottom right (behind textboxes).
    let target_rect = match st.cover_map_style {
        0 => {
            // Watermark / Background silhouette: large coverage
            designcraft_geom::Rect {
                x0: pw * 0.06,
                y0: ph * 0.34,
                x1: pw * 0.82,
                y1: ph * 0.90,
            }
        }
        1 => {
            // Accent Hero: prominent size in lower-center / right
            designcraft_geom::Rect {
                x0: pw * 0.06,
                y0: ph * 0.34,
                x1: pw * 0.82,
                y1: ph * 0.90,
            }
        }
        _ => {
            // Outlines:
            designcraft_geom::Rect {
                x0: pw * 0.06,
                y0: ph * 0.34,
                x1: pw * 0.82,
                y1: ph * 0.90,
            }
        }
    };

    let projected = map.project_features(target_rect);

    let primary = if st.cover_color_primary.trim().is_empty() { "#0072bc" } else { st.cover_color_primary.trim() };
    let accent = if st.cover_color_accent.trim().is_empty() { "#f5ab19" } else { st.cover_color_accent.trim() };
    let dark = if st.cover_color_dark.trim().is_empty() { "#122647" } else { st.cover_color_dark.trim() };

    let mut emitted_ids = Vec::new();

    for pf in &projected {
        let is_hi = map.features.get(pf.feature_idx).map(|f| f.is_highlighted()).unwrap_or(false);

        let (fill, stroke, stroke_weight) = match st.cover_map_style {
            0 => {
                // Watermark / Silhouette:
                if is_hi {
                    (accent, primary, 1.25)
                } else {
                    ("#F1F5F9", "#CBD5E1", 0.5)
                }
            }
            1 => {
                // Accent Hero:
                if is_hi {
                    (accent, dark, 1.5)
                } else {
                    (primary, "#FFFFFF", 0.75)
                }
            }
            _ => {
                // Outlines only:
                if is_hi {
                    ("[None]", accent, 1.75)
                } else {
                    ("[None]", "#94A3B8", 0.75)
                }
            }
        };

        for ring in &pf.polygons {
            if let Ok(id) = super::charts::emitter::emit_polygon(app, 0, ring, fill, stroke, stroke_weight) {
                emitted_ids.push(id);
            }
        }
    }

    let group_id = if emitted_ids.len() > 1 {
        app.run("object.group", serde_json::json!({"ids": emitted_ids}))
            .ok()
            .and_then(|v| v.get("id").and_then(serde_json::Value::as_u64))
    } else {
        None
    };

    // Ensure the decorative map is positioned behind all textboxes and logo,
    // but in front of background shapes and hero/landscape photo frames.
    let map_item_ids: std::collections::HashSet<u64> = if let Some(gid) = group_id {
        let mut s = std::collections::HashSet::new();
        s.insert(gid);
        s
    } else {
        emitted_ids.into_iter().collect()
    };

    let _ = app.session.edit(|doc, _| {
        let non_empty_stories: std::collections::HashSet<_> = doc
            .stories
            .iter()
            .filter(|(_, s)| !s.text.trim().is_empty())
            .map(|(&id, _)| id)
            .collect();

        if let Some(sp) = doc.spreads.first_mut() {
            let sp = std::sync::Arc::make_mut(sp);
            let mut non_map_items = Vec::new();
            let mut map_items = Vec::new();

            for it in sp.items.drain(..) {
                if map_item_ids.contains(&it.id.0) {
                    map_items.push(it);
                } else {
                    non_map_items.push(it);
                }
            }

            // Find the last photo/hero/landscape item index among non_map_items.
            let last_photo_idx = non_map_items.iter().rposition(|it| {
                it.label.contains("landscape")
                    || it.label.contains("hero")
                    || it.label.contains("background")
                    || it.name.contains("landscape")
                    || it.name.contains("hero")
                    || it.name.contains("background")
                    || (it.graphic().is_some() && !it.label.contains("logo") && !it.name.contains("logo"))
            });

            // Find the insertion index: strictly after any background/photo items, and before all textboxes / logo.
            let insert_idx = if let Some(photo_idx) = last_photo_idx {
                let after_photo = non_map_items.iter().enumerate().skip(photo_idx + 1).find(|(_, it)| {
                    it.is_text_frame()
                        || it.label.contains("text")
                        || it.name.contains("text")
                        || it.label.contains("logo")
                        || it.name.contains("logo")
                });
                if let Some((idx, _)) = after_photo {
                    idx
                } else {
                    (photo_idx + 1).min(non_map_items.len())
                }
            } else {
                non_map_items
                    .iter()
                    .position(|it| {
                        if it.label.contains("text") || it.name.contains("text") {
                            return true;
                        }
                        if let designcraft_doc::Content::Text(tf) = &it.content {
                            return non_empty_stories.contains(&tf.story);
                        }
                        false
                    })
                    .unwrap_or(non_map_items.len())
            };

            let mut reordered = Vec::with_capacity(non_map_items.len() + map_items.len());
            reordered.extend(non_map_items.drain(..insert_idx));
            reordered.extend(map_items);
            reordered.extend(non_map_items);
            sp.items = reordered;
            sp.relayout();
        }
        Ok(serde_json::Value::Null)
    });

    Ok(())
}

pub(crate) fn get_cover_page_dimensions(app: &DesignApp) -> (f64, f64) {
    let Some(s) = app.session.active() else { return (595.2755905511812, 841.8897637795277) };
    let Some(sp) = s.doc.spreads.first() else { return (595.2755905511812, 841.8897637795277) };
    let Some(p) = sp.pages.first() else { return (595.2755905511812, 841.8897637795277) };
    (p.width, p.height)
}

/// Apply the AI Base cover template (`100_ai_base.designcraft`) with the generated background image.
pub fn apply_ai_cover_result(app: &mut DesignApp, st: &PubgenState, bg_path: &str) -> Result<String, String> {
    let preset = super::ai_cover::preset(st.cover_style).ok_or_else(|| format!("Unsupported AI cover style: {}", st.cover_style))?;
    let palette = preset.palettes.get(st.cover_ai_palette).unwrap_or(&preset.palettes[0]);

    let tpl_path = "assets/templates/covers/100_ai_base.designcraft";
    let tpl_file = resolve_template_path(tpl_path).ok_or_else(|| format!("AI cover template not found: {tpl_path}"))?;
    let tpl_path_str = tpl_file.to_string_lossy().to_string();

    let (title_text, title2_text, year_text, volume_text, cat_text, code_text) = get_cover_text_vals(st);
    let agency_text = if st.cover_agency_text.trim().is_empty() { "BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG" } else { st.cover_agency_text.trim() };
    let logo = resolve_cover_image(if st.cover_image_logo.trim().is_empty() { "outputs/bps_logo.png" } else { st.cover_image_logo.trim() });

    let primary = if st.cover_color_primary.trim().is_empty() { palette.primary } else { st.cover_color_primary.trim() };
    let accent = if st.cover_color_accent.trim().is_empty() { palette.accent } else { st.cover_color_accent.trim() };
    let dark = if st.cover_color_dark.trim().is_empty() { palette.dark } else { st.cover_color_dark.trim() };

    let mut text_slots = serde_json::Map::new();
    text_slots.insert("title".into(), json!(title_text));
    text_slots.insert("title#2".into(), json!(title2_text));

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
                "background": bg_path,
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

    app.run("template.page.apply", apply_params).map_err(|e| format!("Failed to apply AI cover template: {e}"))?;

    crate::canvas::go_to_page(app, 0);

    // Clean up redundant title_en if empty or duplicate
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
            for item in &mut sp.items {
                if item.label.starts_with("#slot:image:background") || item.name.starts_with("#slot:image:background") {
                    let it = std::sync::Arc::make_mut(item);
                    it.label = format!("#slot:image:background #ai_preset:{}", preset.name);
                    let bounds = it.inner_bounds();
                    if let designcraft_doc::Content::Graphic(g) = &mut it.content {
                        g.fit_align = 5; // Center-Right: crop right half for Front Cover
                        if let Some(xf) = g.fitted(bounds, designcraft_doc::Fitting::FillProportionally) {
                            g.xf = xf;
                        }
                    }
                }
            }
            if !redundant_items.is_empty() {
                sp.items.retain(|it| !redundant_items.contains(&it.id));
            }
            sp.relayout();
        }
        for sid in title_en_stories {
            if let Some(story) = doc.story_mut(sid) {
                for c in &mut story.chars {
                    c.format.over.font_style = Some("Italic".to_string());
                }
            }
        }
        Ok(())
    });

    let sname = format!("AI {}", preset.name);
    if st.cover_include_inside_back {
        execute_generate_inside_cover(app)?;
        execute_generate_back_cover(app, st)?;
        ensure_back_cover_at_end(app);
        Ok(format!("Cover pages generated successfully: Front Cover, Inside Cover, Back Cover ({sname})"))
    } else {
        Ok(format!("Cover page generated successfully ({sname})"))
    }
}

/// Trigger cover generation from the UI: style < 100 runs synchronously; style >= 100 spawns CoverJob.
pub fn trigger_generate_cover(app: &mut DesignApp, st: &mut PubgenState, ctx: egui::Context) {
    if super::ai_cover::preset_index(st.cover_style).is_none() {
        let res = execute_generate_cover(app, st);
        st.cover_status = Some(match res {
            Ok(msg) => (msg, true),
            Err(err) => (err, false),
        });
        return;
    }

    let Some(preset) = super::ai_cover::preset(st.cover_style) else {
        st.cover_status = Some((format!("Unsupported cover style: {}", st.cover_style), false));
        return;
    };

    if !st.excel_batch_covers.is_empty() && st.excel_batch_covers.len() > 1 {
        let aspect = imagegen::Aspect::Wrap; // Double A4 spread layout (1216 x 832)
        let items: Vec<super::ai_cover_job::CoverBatchItem> = st
            .excel_batch_covers
            .iter()
            .map(|c| {
                let style = super::excel::cover_style_index(&c.style).unwrap_or(st.cover_style);
                let palette = super::ai_cover::preset(style)
                    .and_then(|p| p.palettes.iter().position(|pal| pal.name.eq_ignore_ascii_case(&c.palette)))
                    .unwrap_or(st.cover_ai_palette);
                let title = c.title1.clone();
                let seed = super::ai_cover_job::seed_from_title(&title);
                let prompt = if let Some(p) = super::ai_cover::preset(style) {
                    let pal = p.palettes.get(palette).unwrap_or(&p.palettes[0]);
                    super::ai_cover::build_prompt(p, pal)
                } else {
                    String::new()
                };
                let neg_prompt = imagegen::GenRequest::clean_cover_negative_prompt();
                let req = imagegen::GenRequest::new(prompt, neg_prompt, aspect, 18).with_seed(seed);
                let filename = format!("ai_cover_batch_{}_{}.png", style, seed);
                let output_path = std::path::PathBuf::from("outputs").join("ai_covers").join(filename);
                super::ai_cover_job::CoverBatchItem {
                    title,
                    title2: c.title2.clone(),
                    title_en: c.title_en.clone(),
                    year: c.year.clone(),
                    volume: c.volume.clone(),
                    catalogue: c.catalogue.clone(),
                    code: c.code.clone(),
                    agency: c.agency.clone(),
                    logo: c.logo.clone(),
                    cover_style: style,
                    palette_index: palette,
                    req,
                    output_path,
                }
            })
            .collect();
        trigger_generate_cover_batch(app, st, items, ctx);
        return;
    }

    if !imagegen::compiled_in() {
        st.cover_status = Some(("AI Base covers need a build with --features ai-cover-online.".into(), false));
        return;
    }

    if !super::ai_cover::AiCoverSetupDialog::key_configured(app) {
        st.cover_status = Some(("AI Base needs a Cloudflare account: click AI Setup\u{2026} above to set up.".into(), false));
        return;
    }

    let palette = preset.palettes.get(st.cover_ai_palette).unwrap_or(&preset.palettes[0]);
    let prompt = super::ai_cover::build_prompt(preset, palette);
    let neg_prompt = imagegen::GenRequest::clean_cover_negative_prompt();
    let aspect = imagegen::Aspect::Wrap; // Double A4 spread layout (1216 x 832)
    let (title_text, _, _, _, _, _) = get_cover_text_vals(st);
    let seed = super::ai_cover_job::seed_from_title(title_text);
    let req = imagegen::GenRequest::new(prompt, neg_prompt, aspect, 18).with_seed(seed);

    let (account_id, token) = super::ai_cover::get_credentials(&app.session.prefs);

    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let filename = format!("ai_cover_{}_{}_{}.png", st.cover_style, st.cover_ai_palette, timestamp);
    let output_path = std::path::PathBuf::from("outputs").join("ai_covers").join(filename);

    let job = super::ai_cover_job::CoverJob::spawn(&account_id, &token, req, output_path, ctx);
    st.cover_job = Some(job);
    st.cover_status = Some(("Generating AI cover\u{2026}".into(), true));
}

/// Apply one completed cover background to a spread in the document.
pub fn apply_ai_cover_batch_item(app: &mut DesignApp, st: &PubgenState, title: &str, bg_path: &str) -> Result<String, String> {
    let preset = super::ai_cover::preset(st.cover_style).unwrap_or(&super::ai_cover::AI_PRESETS[0]);
    let palette = preset.palettes.get(st.cover_ai_palette).unwrap_or(&preset.palettes[0]);
    let tpl_path = "assets/templates/covers/100_ai_base.designcraft";
    let tpl_file = resolve_template_path(tpl_path).ok_or_else(|| format!("AI cover template not found: {tpl_path}"))?;
    let tpl_path_str = tpl_file.to_string_lossy().to_string();

    let logo = resolve_cover_image(if st.cover_image_logo.trim().is_empty() { "outputs/bps_logo.png" } else { st.cover_image_logo.trim() });
    let agency_text = if st.cover_agency_text.trim().is_empty() { "BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG" } else { st.cover_agency_text.trim() };
    let primary = if st.cover_color_primary.trim().is_empty() { palette.primary } else { st.cover_color_primary.trim() };
    let accent = if st.cover_color_accent.trim().is_empty() { palette.accent } else { st.cover_color_accent.trim() };
    let dark = if st.cover_color_dark.trim().is_empty() { palette.dark } else { st.cover_color_dark.trim() };

    let mut text_slots = serde_json::Map::new();
    text_slots.insert("title".into(), json!(title));
    text_slots.insert("title#2".into(), json!(st.cover_title2));
    text_slots.insert("year".into(), json!(st.cover_year));
    text_slots.insert("volume".into(), json!(st.cover_volume));
    text_slots.insert("catalogue".into(), json!(st.cover_catalogue));
    text_slots.insert("code".into(), json!(st.cover_code_num));
    text_slots.insert("agency".into(), json!(agency_text));

    let mut apply_params = json!({
        "path": tpl_path_str,
        "slots": {
            "text": text_slots,
            "image": {
                "background": bg_path,
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

    app.run("template.page.apply", apply_params).map_err(|e| format!("Failed to apply AI cover template: {e}"))?;

    let _ = app.session.edit(|doc, _| {
        if let Some(sp) = doc.spreads.last_mut() {
            let sp = std::sync::Arc::make_mut(sp);
            if let Some(pg) = sp.pages.first_mut() {
                pg.margins = designcraft_doc::Margins::uniform(0.0);
                pg.local_margins = true;
            }
            for item in &mut sp.items {
                if item.label.starts_with("#slot:image:background") || item.name.starts_with("#slot:image:background") {
                    let it = std::sync::Arc::make_mut(item);
                    it.label = format!("#slot:image:background #ai_preset:{}", preset.name);
                    let bounds = it.inner_bounds();
                    if let designcraft_doc::Content::Graphic(g) = &mut it.content {
                        g.fit_align = 5; // Center-Right: crop right half for Front Cover
                        if let Some(xf) = g.fitted(bounds, designcraft_doc::Fitting::FillProportionally) {
                            g.xf = xf;
                        }
                    }
                }
            }
            sp.relayout();
        }
        Ok(())
    });

    Ok(format!("Cover applied for {title}"))
}

/// Trigger a batch of publication covers: checks re-run skip, pre-flight quota, and spawns worker.
pub fn trigger_generate_cover_batch(app: &mut DesignApp, st: &mut PubgenState, items: Vec<super::ai_cover_job::CoverBatchItem>, ctx: egui::Context) {
    if !imagegen::compiled_in() {
        st.cover_status = Some(("AI Base covers need a build with --features ai-cover-online.".into(), false));
        return;
    }
    if !super::ai_cover::AiCoverSetupDialog::key_configured(app) {
        st.cover_status = Some(("AI Base needs a Cloudflare account: click AI Setup\u{2026} above to set up.".into(), false));
        return;
    }

    // Filter out already generated covers
    let active_doc = app.session.active().map(|a| a.doc.clone());
    let mut pending_items = Vec::new();
    let mut already_generated = 0usize;

    for item in items {
        let preset_name = super::ai_cover::preset(item.cover_style).map(|p| p.name).unwrap_or("");
        if let Some(doc) = &active_doc
            && super::ai_cover_job::cover_page_exists(doc, &item.title, preset_name)
        {
            already_generated += 1;
            continue;
        }
        pending_items.push(item);
    }

    if pending_items.is_empty() {
        let msg = format!("All {already_generated} publication cover(s) already exist in document.");
        st.cover_status = Some((msg.clone(), true));
        st.card1_status = Some((msg.clone(), true));
        st.card3_status = Some((msg, true));
        return;
    }

    // Pre-flight quota estimate: if N > MAX_SAFE_FREE_BATCH, ask confirmation
    if let Some(warning) = super::ai_cover_job::preflight_quota_warning(pending_items.len()) {
        st.cover_batch_pending = Some(pending_items);
        st.cover_status = Some((warning.clone(), false));
        st.card1_status = Some((warning.clone(), false));
        st.card3_status = Some((warning, false));
        return;
    }

    trigger_generate_cover_batch_confirmed(app, st, pending_items, ctx);
}

/// Execute batch cover job after quota confirmation or when batch fits within safe limits.
pub fn trigger_generate_cover_batch_confirmed(
    app: &mut DesignApp,
    st: &mut PubgenState,
    items: Vec<super::ai_cover_job::CoverBatchItem>,
    ctx: egui::Context,
) {
    let (account_id, token) = super::ai_cover::get_credentials(&app.session.prefs);
    let job = super::ai_cover_job::CoverJob::spawn_batch(&account_id, &token, items, ctx);
    st.cover_job = Some(job);
    st.cover_status = Some(("Generating AI covers\u{2026}".into(), true));
}
pub(super) fn add_cover_text_frame(
    app: &mut DesignApp,
    spread: usize,
    rect: [f64; 4],
    text: &str,
    color: &str,
    font_family: &str,
    font_style: &str,
    size: f64,
    leading: Option<f64>,
    tracking: Option<f64>,
    align: &str,
) -> Result<u64, String> {
    let res = app
        .run(
            "frame.create",
            json!({
                "spread": spread,
                "rect": rect,
                "content": "text",
                "text": text,
                "caret": false,
            }),
        )
        .map_err(|e| format!("frame.create failed for '{text}': {e}"))?;

    let id = res.get("id").and_then(Value::as_u64).ok_or("Missing frame id")?;
    let story = res.get("story").and_then(Value::as_u64);

    let mut char_attrs = json!({
        "fontFamily": font_family,
        "fontStyle": font_style,
        "size": size,
    });
    if let Some(lead) = leading {
        char_attrs["leading"] = json!({"kind": "points", "value": lead});
    }
    if let Some(track) = tracking {
        char_attrs["tracking"] = json!(track);
    }

    if let Some(sid) = story {
        let _ = app.run("text.select", json!({"story": sid, "anchor": 0, "focus": text.len()}));
        let _ = app.run("object.color", json!({"color": color, "text": true}));
        let _ = app.run("type.char", json!({"attrs": char_attrs.clone()}));
        let _ = app.run("type.para", json!({"attrs": {"align": align}}));
    }

    let _ = app.run("selection.set", json!({"ids": [id]}));
    let _ = app.run("object.color", json!({"color": color, "text": true}));
    let _ = app.run("type.char", json!({"attrs": char_attrs}));
    let _ = app.run("type.para", json!({"attrs": {"align": align}}));

    Ok(id)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::state::default_cover_title;
    use super::*;

    #[test]
    fn test_cover_card_carries_one_title_and_a_second_title_line() {
        let st = PubgenState::default();
        // The card takes a single headline and keeps the second headline line, with its dummy text.
        assert_eq!(st.cover_title, default_cover_title());
        assert_eq!(st.cover_title2, "Title 2");
    }

    #[test]
    fn test_export_design_3_sample() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let mut st = PubgenState {
            cover_style: 5,
            cover_color_primary: "#6c5ce7".into(),
            cover_color_accent: "#a29bfe".into(),
            cover_color_dark: "#1e293b".into(),
            ..PubgenState::default()
        };
        st.cover_image_skyline = root.join("outputs").join("city_skyline.jpg").to_string_lossy().to_string();
        st.cover_image_logo = root.join("outputs").join("bps_logo.png").to_string_lossy().to_string();
        execute_generate_cover(&mut app, &st).unwrap();
        let _ = app.run("layout.pages.delete", json!({"page": 1}));
        // A scratch file, never `design-3.designcraft`: running the tests used to overwrite that
        // tracked sample with every run (the scratch document is the default page size, not the
        // size the sample was drawn at), so the export must not touch it.
        let path = std::env::temp_dir().join(format!("designcraft-cover-style-5-{}.designcraft", std::process::id()));
        let res = app.run("file.saveAs", json!({"path": path.to_string_lossy()}));
        assert!(res.is_ok(), "Failed to save: {res:?}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_export_design_4_sample() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let st = PubgenState {
            cover_style: 6,
            cover_image_skyline: root.join("outputs").join("city_skyline.jpg").to_string_lossy().to_string(),
            cover_image_logo: root.join("outputs").join("bps_logo.png").to_string_lossy().to_string(),
            ..PubgenState::default()
        };
        execute_generate_cover(&mut app, &st).unwrap();
        let _ = app.run("layout.pages.delete", json!({"page": 1}));
        // A scratch file, never `design-4.designcraft`: that one is the hand-authored reference this
        // style was derived from, so the export must not overwrite it.
        let path = std::env::temp_dir().join(format!("designcraft-cover-style-6-{}.designcraft", std::process::id()));
        let res = app.run("file.saveAs", json!({"path": path.to_string_lossy()}));
        assert!(res.is_ok(), "Failed to save: {res:?}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cover_style_8_text_is_options_only() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState { cover_style: BUSINESS_STYLE, ..PubgenState::default() };
        execute_generate_cover(&mut app, &st).unwrap();

        let active = app.session.active().unwrap();
        let allowed = [
            "KECAMATAN",
            "KABUPATEN",
            "KARAWANG TIMUR",
            "KARAWANG\nTIMUR",
            "2026",
            "VOLUME 23",
            "KATALOG BPS",
            "ISSN",
            "ISBN",
            "BADAN PUSAT STATISTIK",
            "DALAM ANGKA",
            "TITLE 2",
            "20",
            "26",
        ];
        for story in active.doc.stories.values() {
            if story.text.is_empty() {
                continue;
            }
            for banned in ["COMPANY NAME", "YOUR HEADLINE HERE", "LOREM IPSUM", "BUSINESS REPORT 2050"] {
                assert!(!story.text.contains(banned), "template text '{banned}' must not reach the cover: {}", story.text);
            }
            let upper = story.text.to_uppercase();
            assert!(allowed.iter().any(|a| upper.contains(a)), "text must come from the cover options: {}", story.text);
        }

        // The headline title stays on one full-width line (it used to hyphenate across two).
        let title = active.doc.stories.values().find(|s| s.text.contains("KARAWANG")).expect("the title story");
        assert!(!title.text.contains('\n'), "the headline title stays on one line: {}", title.text);

        // Every object on the cover is upright: the wedges carry the angles, not the type.
        let turned =
            active.doc.spreads.iter().flat_map(|s| s.items.iter()).any(|it| designcraft_geom::decompose::decompose(it.xf).rotation.abs() > 1.0);
        assert!(!turned, "every object on the Business Report cover is upright");
    }

    #[test]
    fn test_export_design_5_sample() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        // A4: the size the cover designs are drawn for.
        app.run("file.new", json!({"width": 595.275591, "height": 841.889764})).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let mut st = PubgenState {
            cover_style: HORIZON_STYLE,
            cover_image_skyline: root.join("outputs").join("city_skyline.jpg").to_string_lossy().to_string(),
            cover_image_logo: root.join("outputs").join("bps_logo.png").to_string_lossy().to_string(),
            ..PubgenState::default()
        };
        apply_report_palette(&mut st);
        execute_generate_cover(&mut app, &st).unwrap();
        let _ = app.run("layout.pages.delete", json!({"page": 1}));
        // A scratch file, never `design-5.designcraft`: that one is the traced reference this style
        // was derived from, so the export must not overwrite it.
        let path = std::env::temp_dir().join(format!("designcraft-cover-style-7-{}.designcraft", std::process::id()));
        let res = app.run("file.saveAs", json!({"path": path.to_string_lossy()}));
        assert!(res.is_ok(), "Failed to save: {res:?}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_cover_style_9_text_is_options_only() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState { cover_style: URBAN_STYLE, ..PubgenState::default() };
        execute_generate_cover(&mut app, &st).unwrap();

        let active = app.session.active().unwrap();
        let allowed = ["KECAMATAN", "KARAWANG", "2026", "20", "26", "VOLUME", "KATALOG", "ISSN", "ISBN", "BADAN PUSAT", "DALAM ANGKA", "TITLE 2"];
        for story in active.doc.stories.values() {
            if story.text.is_empty() {
                continue;
            }
            let upper = story.text.to_uppercase();
            assert!(allowed.iter().any(|a| upper.contains(a)), "text must come from the cover options: {}", story.text);
        }
    }

    #[test]
    fn test_cover_style_10_uses_the_tri_color_template() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState {
            cover_style: TRICOLOR_STYLE,
            cover_title: "KECAMATAN KARAWANG TIMUR".into(),
            cover_title2: "DALAM ANGKA".into(),
            ..PubgenState::default()
        };
        let res = execute_generate_cover(&mut app, &st);
        assert!(res.is_ok(), "Tri-Color Bands must apply its template: {res:?}");

        let active = app.session.active().unwrap();
        // The template brings its own three-colour swatches.
        for name in ["Cover Blue", "Cover Green", "Cover Orange"] {
            assert!(active.doc.swatches.iter().any(|s| s.name == name), "swatch `{name}` comes with the template");
        }
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("KARAWANG TIMUR")), "the headline is bound: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("DALAM ANGKA")), "the second headline line is bound: {texts:?}");
    }

    #[test]
    fn test_cover_style_11_uses_the_prism_diamonds_template() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState {
            cover_style: PRISM_STYLE,
            cover_title: "KESEJAHTERAAN RAKYAT".into(),
            cover_title2: "DALAM ANGKA".into(),
            ..PubgenState::default()
        };
        let res = execute_generate_cover(&mut app, &st);
        assert!(res.is_ok(), "Prism Diamonds must apply its template: {res:?}");

        let active = app.session.active().unwrap();
        // The template brings its own midnight and jewel swatches.
        for name in ["Prism Night", "Prism Deep", "Prism Blue", "Prism Green", "Prism Orange", "Prism Mint", "Prism Ice", "Prism Sky"] {
            assert!(active.doc.swatches.iter().any(|s| s.name == name), "swatch `{name}` comes with the template");
        }
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("KESEJAHTERAAN RAKYAT")), "the headline is bound: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("DALAM ANGKA")), "the second headline line is bound: {texts:?}");
        // The photograph is cut as a diamond, not boxed in a rectangle.
        let hero = active
            .doc
            .spreads
            .iter()
            .flat_map(|s| s.items.iter())
            .find(|it| it.label == "#slot:image:hero")
            .expect("the hero photograph frame comes with the template");
        assert_eq!(hero.path.subpaths.first().map(|p| p.anchors.len()), Some(4), "the hero frame is a diamond");
    }

    #[test]
    fn test_cover_style_12_uses_the_quarter_stack_template() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState {
            cover_style: QUARTER_STYLE,
            cover_title: "KESEJAHTERAAN RAKYAT".into(),
            cover_title2: "DALAM ANGKA".into(),
            ..PubgenState::default()
        };
        let res = execute_generate_cover(&mut app, &st);
        assert!(res.is_ok(), "Quarter Stack must apply its template: {res:?}");

        let active = app.session.active().unwrap();
        // The template brings its own warm palette.
        for name in ["Stack Paper", "Stack Ink", "Stack Clay", "Stack Marigold", "Stack Sage", "Stack Blush", "Stack Teal"] {
            assert!(active.doc.swatches.iter().any(|s| s.name == name), "swatch `{name}` comes with the template");
        }
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("KESEJAHTERAAN RAKYAT")), "the headline is bound: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("DALAM ANGKA")), "the second headline line is bound: {texts:?}");
        // The photograph sits in a card with exactly one rounded corner.
        let hero = active
            .doc
            .spreads
            .iter()
            .flat_map(|s| s.items.iter())
            .find(|it| it.label == "#slot:image:hero")
            .expect("the hero photograph frame comes with the template");
        let rounded = hero.corners.corners.iter().filter(|c| c.shape != designcraft_geom::corners::CornerShape::None).count();
        assert_eq!(rounded, 1, "the hero card rounds one corner only: {:?}", hero.corners.corners);
    }

    #[test]
    fn test_cover_style_6_text_is_options_only() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState { cover_style: 6, ..PubgenState::default() };
        execute_generate_cover(&mut app, &st).unwrap();

        let active = app.session.active().unwrap();
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        // The only sources of cover text: the Publication Cover options and the fixed suffix.
        let allowed = [
            "KECAMATAN",
            "KABUPATEN",
            "KARAWANG TIMUR",
            "KARAWANG\nTIMUR",
            "2026",
            "VOLUME 23",
            "KATALOG BPS",
            "ISSN",
            "ISBN",
            "BADAN PUSAT STATISTIK",
            "DALAM ANGKA",
            "TITLE 2",
        ];
        for text in &texts {
            if text.is_empty() {
                continue;
            }
            // Nothing from the reference template survives: no brand, no lorem, no subtitle.
            for banned in ["COMPANY NAME", "YOUR HEADLINE HERE", "LOREM IPSUM", "BUSINESS REPORT"] {
                assert!(!text.contains(banned), "template text '{banned}' must not reach the cover: {text}");
            }
            let upper = text.to_uppercase();
            assert!(allowed.iter().any(|a| upper.contains(a)), "text must come from the cover options: {text}");
        }

        // The year sits upright and centred in the forward wedge, and the title runs on one line.
        let year = active.doc.stories.values().find(|s| s.text == "2026").expect("the year story");
        assert_eq!(year.paras[0].para.align, Some(designcraft_doc::Align::Center), "the year is centred in the wedge");
        let title = active.doc.stories.values().find(|s| s.text.contains("KARAWANG")).expect("the title story");
        assert!(!title.text.contains('\n'), "the headline title stays on one line: {}", title.text);

        let turned =
            active.doc.spreads.iter().flat_map(|s| s.items.iter()).any(|it| designcraft_geom::decompose::decompose(it.xf).rotation.abs() > 1.0);
        assert!(!turned, "every object on the Diagonal Arrow cover is upright");
    }

    #[test]
    fn test_cover_style_7_text_is_options_only_and_position_locked() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = PubgenState { cover_style: HORIZON_STYLE, ..PubgenState::default() };
        execute_generate_cover(&mut app, &st).unwrap();

        let active = app.session.active().unwrap();
        let (si, pi) = active.doc.page_loc(0).unwrap();
        let page = active.doc.spreads[si].pages[pi].bounds();

        // Nothing but the cover options and the fixed suffix reaches the page.
        let allowed = [
            "KECAMATAN",
            "KABUPATEN",
            "KARAWANG TIMUR",
            "KARAWANG\nTIMUR",
            "2026",
            "VOLUME 23",
            "KATALOG BPS",
            "ISSN",
            "ISBN",
            "BADAN PUSAT STATISTIK",
            "DALAM ANGKA",
            "TITLE 2",
        ];
        for story in active.doc.stories.values() {
            if story.text.is_empty() {
                continue;
            }
            for banned in ["COMPANY NAME", "YOUR HEADLINE HERE", "LOREM IPSUM", "BUSINESS REPORT"] {
                assert!(!story.text.contains(banned), "template text '{banned}' must not reach the cover: {}", story.text);
            }
            let upper = story.text.to_uppercase();
            assert!(allowed.iter().any(|a| upper.contains(a)), "text must come from the cover options: {}", story.text);
        }

        // Bounds of every text frame, keyed by the story it flows.
        let placed: Vec<(String, designcraft_geom::Rect)> = active.doc.spreads[si]
            .items
            .iter()
            .filter_map(|it| match &it.content {
                designcraft_doc::Content::Text(tf) => active.doc.story(tf.story).map(|s| (s.text.clone(), it.bounds())),
                _ => None,
            })
            .collect();
        let frame = |needle: &str| {
            placed.iter().find(|(text, _)| text.contains(needle)).map(|(_, r)| *r).unwrap_or_else(|| panic!("no frame holds '{needle}'"))
        };
        let frame_exact = |text_wanted: &str| {
            placed.iter().find(|(text, _)| text == text_wanted).map(|(_, r)| *r).unwrap_or_else(|| panic!("no frame holds exactly '{text_wanted}'"))
        };

        // Position lock: catalogue, then ISSN/ISBN, in the top-right corner over the photograph.
        let cat = frame("Katalog BPS");
        let code = frame("ISSN");
        assert!(cat.y1 < page.y0 + 60.0 && cat.x0 > page.center().x, "the catalogue sits in the top-right: {cat:?}");
        assert!(code.y0 >= cat.y1 - 1.0 && code.y1 < page.y0 + 70.0, "ISSN/ISBN sit under the catalogue: {code:?}");

        // Headline: the title and its second line down the left of the dark block.
        let title = frame("KARAWANG");
        let suffix = frame("Title 2");
        assert!(title.y0 < suffix.y0 && title.y1 <= suffix.y0 + 5.0, "headline order: {title:?} {suffix:?}");
        assert!(suffix.y1 < page.y0 + 740.0, "the headline sits inside the dark block: {suffix:?}");

        // Year in the accent block, with the volume directly underneath it.
        let year = frame_exact("2026");
        let volume = frame("Volume");
        assert!(year.x0 > page.center().x, "the year sits in the accent block: {year:?}");
        assert!(volume.y0 >= year.y1 - 1.0, "the volume sits directly under the year: {year:?} {volume:?}");

        // Position lock: agency logo text in the bottom-left corner.
        let agency = frame("BADAN PUSAT STATISTIK");
        assert!(agency.y0 > page.y0 + 700.0 && agency.x0 < page.center().x, "the agency sits in the bottom-left: {agency:?}");

        // Every object is upright: nothing on this cover is rotated.
        let turned =
            active.doc.spreads.iter().flat_map(|s| s.items.iter()).any(|it| designcraft_geom::decompose::decompose(it.xf).rotation.abs() > 1.0);
        assert!(!turned, "every object on the Modern Horizon cover is upright");
    }

    /// A paragraph list nests chapter titles over the heading styles the RTF imports carry.
    #[test]
    fn test_cover_bilingual() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Single Lang mode (bilingual_output = false)
        let mut st = PubgenState {
            cover_style: 0,
            cover_title: "KARAWANG TIMUR".into(),
            cover_title2: "DALAM ANGKA".into(),
            cover_title_en: "IN FIGURES".into(),
            bilingual_output: false,
            ..Default::default()
        };

        let res = execute_generate_cover(&mut app, &st);
        assert!(res.is_ok(), "execute_generate_cover single lang should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("KARAWANG TIMUR")), "Should contain ID title: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("DALAM ANGKA")), "Should contain ID title2: {texts:?}");
        assert!(!texts.iter().any(|t| t.contains("IN FIGURES")), "Should NOT contain title_en when bilingual_output is false: {texts:?}");

        // 2. Dual Lang mode (bilingual_output = true)
        let mut app_dual = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app_dual.run("file.new", json!({})).unwrap();
        st.bilingual_output = true;

        let res_dual = execute_generate_cover(&mut app_dual, &st);
        assert!(res_dual.is_ok(), "execute_generate_cover dual lang should succeed: {res_dual:?}");

        let active_dual = app_dual.session.active().unwrap();
        let texts_dual: Vec<String> = active_dual.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts_dual.iter().any(|t| t.contains("KARAWANG TIMUR")), "Should contain ID title: {texts_dual:?}");
        assert!(texts_dual.iter().any(|t| t.contains("DALAM ANGKA")), "Should contain ID title2: {texts_dual:?}");
        assert!(texts_dual.iter().any(|t| t.contains("IN FIGURES")), "Should contain title_en in bilingual mode: {texts_dual:?}");

        // 3. User scenario: title2 in template already contains the English title (test across all 13 styles)
        for style in 0..13 {
            let mut app_s = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app_s.run("file.new", json!({})).unwrap();
            let st_s = PubgenState {
                cover_style: style,
                cover_title: "KECAMATAN KARAWANG".into(),
                cover_title2: "EAST KARAWANG DISTRICT IN FIGURES".into(),
                cover_title_en: "EAST KARAWANG DISTRICT IN FIGURES".into(),
                bilingual_output: true,
                ..Default::default()
            };
            let res_s = execute_generate_cover(&mut app_s, &st_s);
            assert!(res_s.is_ok(), "Style {style} should succeed: {res_s:?}");
            let active_s = app_s.session.active().unwrap();
            let texts_s: Vec<String> = active_s.doc.stories.values().map(|s| s.text.clone()).collect();
            let count_en = texts_s.iter().filter(|t| t.contains("EAST KARAWANG DISTRICT IN FIGURES")).count();
            assert_eq!(count_en, 1, "Style {style}: English title must appear exactly once, got {count_en}: {texts_s:?}");
            let en_story = active_s.doc.stories.values().find(|s| s.text.contains("EAST KARAWANG DISTRICT IN FIGURES")).unwrap();
            let has_bold_italic = en_story.chars.iter().any(|r| r.format.over.font_style.as_deref() == Some("Bold Italic"));
            assert!(has_bold_italic, "Style {style}: English title runs must be Bold Italic: {:?}", en_story.chars);
        }
    }

    #[test]
    fn test_cover_include_inside_and_back_cover_generation() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            cover_style: 0,
            cover_color_primary: "#1e3a8a".into(),
            cover_color_accent: "#d97706".into(),
            cover_image_skyline: "plan/pubgen/samples/images/bps_logo.png".into(),
            cover_include_inside_back: true,
            cover_image_back: "plan/pubgen/samples/images/se2026.png".into(),
            cover_office_address: "Jl. Cakradireja No.36 Karawang 41312\nJawa Barat".into(),
            cover_code_type: 0,
            cover_code_num: "2807-6302".into(),
            ..Default::default()
        };

        let res = execute_generate_cover(&mut app, &st);
        assert!(res.is_ok(), "Cover generation with inside and back cover should succeed: {res:?}");

        let active = app.session.active().unwrap();
        // Document should now have 4 pages: Front Cover, Inside Cover, initial page, Back Cover
        assert_eq!(active.doc.page_count(), 4, "Expected 4 pages (Front, Inside, initial page, Back)");
        assert_eq!(find_back_cover_page(&active.doc), Some(3), "Back cover must be the last page (page 3)");

        // Spread 0: Front Cover
        let sp0 = &active.doc.spreads[0];
        assert_eq!(sp0.pages[0].margins, designcraft_doc::Margins::uniform(0.0));

        // Spread 1: Inside Cover
        let sp1 = &active.doc.spreads[1];
        assert_eq!(sp1.pages[0].margins, designcraft_doc::Margins::uniform(0.0));
        // All unassigned vector shapes on Inside Cover must have Fill [Paper] and Black stroke
        for it in &sp1.items {
            if matches!(it.content, designcraft_doc::Content::Unassigned) {
                assert_eq!(it.fill.swatch, "[Paper]", "Inside cover shape must have white fill: {:?}", it.id);
                assert_eq!(it.stroke.swatch, designcraft_color::swatch::BLACK, "Inside cover shape must have black stroke");
                assert_eq!(it.stroke.weight, 1.0);
            }
        }

        // Last Spread: Back Cover
        let sp_back = active.doc.spreads.last().unwrap();
        assert_eq!(sp_back.pages[0].margins, designcraft_doc::Margins::uniform(0.0));

        // All items on Back Cover must have locked = true
        for it in &sp_back.items {
            assert!(it.locked, "Item {:?} on Back Cover must be locked", it.id);
        }

        fn has_white_unassigned(it: &designcraft_doc::Item) -> bool {
            if matches!(it.content, designcraft_doc::Content::Unassigned) && it.fill.swatch == "[Paper]" {
                return true;
            }
            if let designcraft_doc::Content::Group { items } = &it.content {
                return items.iter().any(|sub| has_white_unassigned(sub));
            }
            false
        }

        // Shapes that had image background (hero frame) must be converted to white fill ([Paper])
        let has_white_shape = sp_back.items.iter().any(|it| has_white_unassigned(it));
        assert!(has_white_shape, "Back cover must have white shape converted from image background");

        // Must contain graphic frames (top-right stack, center slogan, camera placeholder, office logo)
        let graphic_frames_count = sp_back.items.iter().filter(|it| matches!(it.content, designcraft_doc::Content::Graphic(_))).count();
        assert!(graphic_frames_count >= 4, "Back cover must have at least 4 graphic frames (got {graphic_frames_count})");

        // Must contain text frames for agency name, office address, and ISSN/ISBN
        let text_frames_count = sp_back.items.iter().filter(|it| matches!(it.content, designcraft_doc::Content::Text(_))).count();
        assert!(text_frames_count >= 3, "Back cover must have at least 3 text frames (agency, address, ISSN)");

        let ph = sp_back.pages[0].height;
        let pw = sp_back.pages[0].width;

        // Verify Office Logo position at bottom left:
        let logo_item = sp_back.items.iter().find(|it| it.label == "#slot:back_cover:logo").expect("Logo item on back cover");
        assert!(logo_item.bounds().y0 >= ph - 110.0, "Logo must be at bottom left (y0: {})", logo_item.bounds().y0);
        assert!((logo_item.bounds().x0 - 36.0).abs() < 1.0, "Logo x0 must be at margin 36");

        // Verify Office Name position at bottom left beside logo:
        let agency_item = sp_back.items.iter().find(|it| it.label == "#slot:back_cover:agency").expect("Agency item on back cover");
        assert!(agency_item.bounds().y0 >= ph - 110.0, "Agency name must be at bottom left (y0: {})", agency_item.bounds().y0);
        assert!(agency_item.bounds().x0 >= 80.0, "Agency name x0 must be beside logo");

        // Verify Office Address position below logo & name on bottom left:
        let addr_item = sp_back.items.iter().find(|it| it.label == "#slot:back_cover:address").expect("Address item on back cover");
        assert!(addr_item.bounds().y0 >= ph - 65.0, "Address must be below logo and name (y0: {})", addr_item.bounds().y0);
        assert!((addr_item.bounds().x0 - 36.0).abs() < 1.0, "Address x0 must be at margin 36");
        if let designcraft_doc::Content::Text(tf) = &addr_item.content {
            let story = active.doc.story(tf.story).expect("Address story");
            for r in &story.chars {
                assert_eq!(r.format.over.font_style.as_deref(), Some("Regular"), "Address must not be bold");
                assert!(r.format.over.size.unwrap_or(0.0) <= 8.0, "Address must use standard font size");
            }
        }

        // Verify ISSN/ISBN position at bottom right side:
        let code_item = sp_back.items.iter().find(|it| it.label == "#slot:back_cover:code").expect("Code item on back cover");
        assert!(code_item.bounds().x0 >= pw - 180.0, "ISSN must be on bottom right side (x0: {})", code_item.bounds().x0);
        assert!(code_item.bounds().y0 >= ph - 65.0, "ISSN must be on bottom right side (y0: {})", code_item.bounds().y0);
        if let designcraft_doc::Content::Text(tf) = &code_item.content {
            let story = active.doc.story(tf.story).expect("ISSN story");
            for r in &story.chars {
                assert_eq!(r.format.over.font_style.as_deref(), Some("Regular"), "ISSN must not be bold");
                assert!(r.format.over.size.unwrap_or(0.0) <= 8.0, "ISSN must use standard font size");
            }
            for p in &story.paras {
                assert_eq!(p.para.align, Some(designcraft_doc::Align::Right), "ISSN must be right-aligned");
            }
        }

        // Test that adding body pages afterwards still keeps Back Cover at the very end
        app.run("layout.pages.insert", json!({"count": 2})).unwrap();
        ensure_back_cover_at_end(&mut app);
        let active2 = app.session.active().unwrap();
        assert_eq!(active2.doc.page_count(), 6, "Now 6 pages total");
        assert_eq!(find_back_cover_page(&active2.doc), Some(5), "Back cover must move to the last page (page 5)");
    }

    fn cover_replace_state(style: usize, include: bool, replace: bool) -> PubgenState {
        PubgenState {
            cover_style: style,
            cover_image_skyline: "plan/pubgen/samples/images/bps_logo.png".into(),
            cover_image_back: "plan/pubgen/samples/images/se2026.png".into(),
            cover_include_inside_back: include,
            cover_replace: replace,
            ..Default::default()
        }
    }

    #[test]
    fn test_cover_replace_swaps_the_front_cover_in_place() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = cover_replace_state(0, false, false);
        execute_generate_cover(&mut app, &st).unwrap();
        assert_eq!(app.session.active().unwrap().doc.page_count(), 2, "cover + the initial body page");

        // Replace checked: the cover is swapped in place, not stacked in front.
        let st = cover_replace_state(0, false, true);
        execute_generate_cover(&mut app, &st).unwrap();

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2, "replace keeps the page count: a third page would be a stacked cover");
        assert!(is_cover_page(&active.doc, 0), "page 0 is the regenerated cover");
        assert!(!is_cover_page(&active.doc, 1), "page 1 stays the original body page");
    }

    #[test]
    fn test_cover_replace_collapses_a_full_set_to_what_the_checkbox_asks_for() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = cover_replace_state(0, true, false);
        execute_generate_cover(&mut app, &st).unwrap();
        {
            let active = app.session.active().unwrap();
            assert_eq!(active.doc.page_count(), 4, "front, inside, body, back");
            assert_eq!(find_back_cover_page(&active.doc), Some(3));
        }

        // Replace with the inside/back box unchecked: the full set collapses to a front cover.
        let st = cover_replace_state(0, false, true);
        execute_generate_cover(&mut app, &st).unwrap();
        {
            let active = app.session.active().unwrap();
            assert_eq!(active.doc.page_count(), 2, "front + body only");
            assert!(is_cover_page(&active.doc, 0), "the front cover was regenerated");
            assert_eq!(find_back_cover_page(&active.doc), None, "the old back cover is gone");
        }

        // Replace with the inside/back box checked again: the full set comes back.
        let st = cover_replace_state(0, true, true);
        execute_generate_cover(&mut app, &st).unwrap();
        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 4, "front, inside, body, back");
        assert!(is_cover_page(&active.doc, 0) && is_cover_page(&active.doc, 1), "front and inside cover regenerated");
        assert_eq!(find_back_cover_page(&active.doc), Some(3), "back cover regenerated at the end");
    }

    #[test]
    fn test_cover_replace_without_an_existing_cover_just_adds() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let st = cover_replace_state(0, false, true);
        execute_generate_cover(&mut app, &st).unwrap();
        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), 2, "with nothing to replace, the box behaves like unchecked");
        assert!(is_cover_page(&active.doc, 0));
        assert!(!is_cover_page(&active.doc, 1));
    }

    #[test]
    fn test_cover_detection_needs_cover_artwork_not_just_zero_margins() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let _ = app.session.edit(|doc, _| {
            if let Some(sp) = doc.spreads.first_mut() {
                let sp = std::sync::Arc::make_mut(sp);
                if let Some(pg) = sp.pages.first_mut() {
                    pg.margins = designcraft_doc::Margins::uniform(0.0);
                    pg.local_margins = true;
                }
            }
            Ok(serde_json::Value::Null)
        });
        let active = app.session.active().unwrap();
        // Zero margins alone do not make a cover page: there is no cover artwork on it.
        assert!(!is_cover_page(&active.doc, 0));
    }

    #[test]
    fn test_ai_cover_branch_dispatch() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Procedural style (< 100) generates synchronously
        let mut st_proc = PubgenState { cover_style: 0, ..Default::default() };
        let ctx = egui::Context::default();
        trigger_generate_cover(&mut app, &mut st_proc, ctx.clone());
        assert!(st_proc.cover_job.is_none(), "Procedural cover should not spawn CoverJob");
        let (msg, ok) = st_proc.cover_status.as_ref().unwrap();
        assert!(*ok, "Procedural generation should succeed: {msg}");

        // 2. AI style without credentials (or without feature) reports actionable error
        let mut st_ai = PubgenState { cover_style: super::ai_cover::AI_STYLE_BASE, ..Default::default() };
        app.session.prefs.cover_ai_cloudflare_account_id.clear();
        app.session.prefs.cover_ai_cloudflare_api_token.clear();
        trigger_generate_cover(&mut app, &mut st_ai, ctx.clone());
        assert!(st_ai.cover_job.is_none());
        let (err_msg, ok) = st_ai.cover_status.as_ref().unwrap();
        assert!(!*ok);
        if !imagegen::compiled_in() {
            assert!(err_msg.contains("ai-cover-online"), "Should report missing feature: {err_msg}");
        } else {
            assert!(err_msg.contains("AI Setup"), "Should guide user to AI Setup: {err_msg}");

            // 3. AI style with credentials configured spawns CoverJob
            app.session.prefs.cover_ai_cloudflare_account_id = "test_acct".into();
            app.session.prefs.cover_ai_cloudflare_api_token = "test_tok".into();
            trigger_generate_cover(&mut app, &mut st_ai, ctx);
            assert!(st_ai.cover_job.is_some(), "Configured AI cover must spawn CoverJob");
            let (status_msg, ok) = st_ai.cover_status.as_ref().unwrap();
            assert!(*ok);
            assert!(status_msg.contains("Generating AI cover"));
        }
    }

    #[test]
    fn test_ai_cover_cancel_keeps_document_unchanged() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let initial_pages = app.session.active().unwrap().doc.page_count();
        let initial_items = app.session.active().unwrap().doc.spreads[0].items.len();

        let tmp = tempfile::tempdir().unwrap();
        let out_file = tmp.path().join("cover.png");
        let fake_provider = Box::new(super::ai_cover_job::FakeImageGenProvider(|_req, _cancel| Ok(vec![1, 2, 3])));
        let req = imagegen::GenRequest::new("prompt", "", imagegen::Aspect::Iso, 18);
        let ctx = egui::Context::default();

        let job = super::ai_cover_job::CoverJob::spawn_with_provider(fake_provider, req, out_file.clone(), ctx);
        job.cancel();

        let mut st = PubgenState { cover_style: super::ai_cover::AI_STYLE_BASE, cover_job: Some(job), ..Default::default() };

        // Poll job until worker finishes processing cancel
        if let Some(mut j) = st.cover_job.take() {
            let start = std::time::Instant::now();
            while !j.finished && start.elapsed().as_secs() < 3 {
                let _ = j.poll(&mut st);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert!(j.finished);
            assert!(j.completed_path.is_none());
            st.cover_job = None;
        }

        // Verify document structure remains entirely untouched
        let active = app.session.active().unwrap();
        assert_eq!(active.doc.page_count(), initial_pages, "Page count must stay unchanged after cancel");
        assert_eq!(active.doc.spreads[0].items.len(), initial_items, "Items count must stay unchanged after cancel");
        assert!(!out_file.exists(), "Cancelled job must not leave output image");
    }

    #[test]
    fn test_ai_cover_apply_result_success() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Write a tiny 1x1 valid PNG for placement
        let tmp = tempfile::tempdir().unwrap();
        let png_path = tmp.path().join("test_bg.png");
        // Minimal 1x1 PNG bytes
        let png_bytes = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01,
            0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&png_path, png_bytes).unwrap();

        let st = PubgenState {
            cover_style: super::ai_cover::AI_STYLE_BASE, // Retro Waves
            cover_ai_palette: 0,                         // Warm Terracotta
            cover_title: "AI KABUPATEN TEST".into(),
            cover_title2: "SUBTITLE LINE 2".into(),
            cover_year: "2027".into(),
            ..Default::default()
        };

        let res = apply_ai_cover_result(&mut app, &st, &png_path.to_string_lossy());
        assert!(res.is_ok(), "Applying AI cover template should succeed: {res:?}");
        assert!(res.as_ref().unwrap().contains("AI Retro Waves"));

        let active = app.session.active().unwrap();
        // Page 0 should be the front cover
        let sp0 = &active.doc.spreads[0];
        let texts: Vec<String> = active.doc.stories.values().map(|s| s.text.clone()).collect();
        assert!(texts.iter().any(|t| t.contains("AI KABUPATEN TEST")), "Should contain title: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("SUBTITLE LINE 2")), "Should contain title2: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("20") || t.contains("27") || t.contains("2027")), "Should contain year: {texts:?}");

        // Background graphic frame exists on spread 0
        let has_bg_slot = sp0.items.iter().any(|it| it.label.contains("background") || it.name.contains("background"));
        assert!(has_bg_slot, "Background frame must be present on front cover spread");
    }

    #[test]
    fn test_ai_cover_bifold_double_a4_and_inside_cover() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let png_path = tmp.path().join("test_bg.png");
        let dark_img = image::RgbaImage::from_pixel(10, 10, image::Rgba([10, 25, 49, 255]));
        dark_img.save(&png_path).unwrap();

        let st = PubgenState {
            cover_style: super::ai_cover::AI_STYLE_BASE, // AI Retro Waves
            cover_ai_palette: 0,                         // Corporate Blue
            cover_title: "BIFOLD SPREAD TEST".into(),
            cover_title2: "SUBTITLE".into(),
            cover_year: "2027".into(),
            cover_include_inside_back: true,
            ..Default::default()
        };

        let res = apply_ai_cover_result(&mut app, &st, &png_path.to_string_lossy());
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        // 1. Front Cover (Spread 0): fit_align must be 5 (Center-Right)
        let sp0 = &active.doc.spreads[0];
        let front_bg = sp0.items.iter().find(|it| it.label.contains("background")).expect("front bg");
        if let designcraft_doc::Content::Graphic(g) = &front_bg.content {
            assert_eq!(g.fit_align, 5, "Front Cover background must align to right edge (fit_align = 5)");
        } else {
            panic!("Front bg is not a graphic");
        }

        // 2. Inside Cover (Page 1):
        // All text stories must be black, and AI image must be light gray-scale (all pixels >= 210)
        let (inside_si, _inside_pi) = active.doc.page_loc(1).expect("inside page loc");
        let inside_sp = &active.doc.spreads[inside_si];
        let inside_bg = inside_sp.items.iter().find(|it| it.label.contains("background")).expect("inside bg");
        if let designcraft_doc::Content::Graphic(g) = &inside_bg.content {
            let asset = active.doc.assets.get(&g.asset).expect("inside bg asset");
            assert!(asset.name.contains("light_gray"), "Inside cover asset name: {}", asset.name);
            let img = image::load_from_memory(&asset.data).expect("load inside asset");
            for p in img.to_rgba8().pixels() {
                if p[3] > 0 {
                    assert!(p[0] >= 210, "Light gray-scale pixel value {} must be >= 210 for readability", p[0]);
                }
            }
        }
        for it in &inside_sp.items {
            if let designcraft_doc::Content::Text(tf) = &it.content
                && let Some(story) = active.doc.story(tf.story)
            {
                for c in &story.chars {
                    assert_eq!(c.format.over.fill.as_deref(), Some(designcraft_color::swatch::BLACK), "Inside text must be Black");
                }
            }
        }

        // 3. Back Cover (last page):
        // Must contain the background graphic with fit_align = 3 (Center-Left)
        let last_page = active.doc.page_count() - 1;
        let (back_si, _) = active.doc.page_loc(last_page).expect("back page loc");
        let back_sp = &active.doc.spreads[back_si];
        let back_bg = back_sp.items.iter().find(|it| it.label.contains("background")).expect("back bg");
        if let designcraft_doc::Content::Graphic(g) = &back_bg.content {
            assert_eq!(g.fit_align, 3, "Back Cover background must align to left edge (fit_align = 3)");
        } else {
            panic!("Back bg is not a graphic");
        }
    }

    #[test]
    fn test_cover_map_overlay_uses_cover_geojson_with_highlight() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            cover_add_map: true,
            cover_map_stock_idx: 0,
            cover_map_style: 1, // Accent Hero
            cover_color_primary: "#0072BC".into(),
            cover_color_accent: "#F5871F".into(),
            ..Default::default()
        };

        let res = apply_cover_map_overlay(&mut app, &st);
        assert!(res.is_ok(), "apply_cover_map_overlay failed: {res:?}");

        let active = app.session.active().unwrap();
        let sp0 = &active.doc.spreads[0];
        // Must emit polygon items on spread 0
        println!("sp0 items count: {}", sp0.items.len());
        assert!(!sp0.items.is_empty(), "Expected map item/group to be emitted on cover spread");
     }

    #[test]
    fn test_cover_map_overlay_z_order_behind_textboxes_and_larger_bounds() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let (pw, ph) = get_cover_page_dimensions(&app);

        // 1. Create a background rectangle
        app.run(
            "frame.create",
            json!({"spread": 0, "rect": [0.0, 0.0, pw, ph], "content": "unassigned", "shape": "rectangle"}),
        )
        .unwrap();

        // 2. Create a photo frame (landscape)
        let _ = app.run(
            "frame.create",
            json!({
                "spread": 0,
                "rect": [pw * 0.5, ph * 0.2, pw * 0.95, ph * 0.6],
                "content": "graphic",
                "shape": "rectangle"
            }),
        );
        let _ = app.session.edit(|doc, _| {
            if let Some(sp) = doc.spreads.first_mut() {
                let sp = std::sync::Arc::make_mut(sp);
                if let Some(it) = sp.items.last_mut() {
                    let it = std::sync::Arc::make_mut(it);
                    it.label = "#slot:image:landscape".into();
                }
            }
            Ok(serde_json::Value::Null)
        });

        // 3. Create text boxes (Title and Agency)
        add_cover_text_frame(
            &mut app,
            0,
            [pw * 0.08, ph * 0.35, pw * 0.5, ph * 0.45],
            "KECAMATAN KARAWANG TIMUR DALAM ANGKA",
            "#ffffff",
            "Arial",
            "Bold",
            18.0,
            None,
            None,
            "left",
        )
        .unwrap();

        add_cover_text_frame(
            &mut app,
            0,
            [pw * 0.08, ph * 0.85, pw * 0.4, ph * 0.92],
            "BADAN PUSAT STATISTIK",
            "#ffffff",
            "Arial",
            "Regular",
            9.0,
            None,
            None,
            "left",
        )
        .unwrap();

        let st = PubgenState {
            cover_add_map: true,
            cover_map_stock_idx: 0,
            cover_map_style: 1, // Accent Hero
            cover_color_primary: "#0072BC".into(),
            cover_color_accent: "#F5871F".into(),
            ..Default::default()
        };

        let res = apply_cover_map_overlay(&mut app, &st);
        assert!(res.is_ok(), "apply_cover_map_overlay failed: {res:?}");

        let active = app.session.active().unwrap();
        let sp0 = &active.doc.spreads[0];

        // Find the index of the map group (or polygon)
        let map_idx = sp0
            .items
            .iter()
            .position(|it| it.is_group() || it.shape == designcraft_doc::Shape::Polygon)
            .expect("Map item/group must be present on spread 0");

        // Find the indices of photo and textboxes
        let photo_idx = sp0
            .items
            .iter()
            .position(|it| it.label.contains("landscape") || it.name.contains("landscape"))
            .expect("Photo frame must be present");

        let text_indices: Vec<usize> = sp0
            .items
            .iter()
            .enumerate()
            .filter(|(_, it)| it.is_text_frame())
            .map(|(i, _)| i)
            .collect();

        assert!(!text_indices.is_empty(), "Text frames must be present");

        // The map must be in front of the photo frame:
        assert!(
            map_idx > photo_idx,
            "Map (index {map_idx}) must be in front of photo frame (index {photo_idx})"
        );

        // The map must be behind ALL textboxes:
        for &t_idx in &text_indices {
            assert!(
                map_idx < t_idx,
                "Map (index {map_idx}) must be behind textbox (index {t_idx})"
            );
        }
    }
}
