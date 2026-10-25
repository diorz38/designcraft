//! Procedural divider art: command-by-command divider page generation when no
//! `.designcraft` template is found on disk (e.g. portable app release).

use super::add_cover_text_frame;
use crate::DesignApp;
use serde_json::{Value, json};

/// Best-effort command for procedural divider art: a failure cannot abort the style,
/// but in debug builds it prints, so a typo'd command id cannot stay silently broken.
fn fx(app: &mut DesignApp, command: &str, params: Value) {
    if cfg!(debug_assertions)
        && let Err(e) = app.run(command, params)
    {
        eprintln!("pubgen divider art: {command} failed: {e}");
    }
}

/// Shared values for procedural divider art.
#[derive(Clone, Copy)]
pub(super) struct DividerArtVals<'a> {
    pub(super) spread_idx: usize,
    pub(super) pw: f64,
    pub(super) ph: f64,
    pub(super) primary: &'a str,
    pub(super) accent: &'a str,
    pub(super) dark: &'a str,
    pub(super) first_word: &'a str,
    pub(super) chapter_num: &'a str,
    pub(super) chapter_title: &'a str,
    pub(super) title_en: &'a str,
    pub(super) image_path: Option<&'a str>,
}

/// Generate a procedural divider page matching the requested style (1 = Simple Chapter, 2 = Batik Infographic).
pub(super) fn generate_divider(app: &mut DesignApp, v: DividerArtVals, style: usize) -> Result<(), String> {
    match style {
        1 => divider_style_simple_chapter(app, v),
        _ => divider_style_batik_infographic(app, v),
    }
}

/// Style 2 (Default): Batik / Infographic Divider (`002_batik_infographic`)
/// Full-bleed spread with dark header band, bookmark ribbon with large chapter number,
/// bilingual titles, and an infographic illustration graphic frame.
pub(super) fn divider_style_batik_infographic(app: &mut DesignApp, v: DividerArtVals) -> Result<(), String> {
    let DividerArtVals { spread_idx, pw, ph, primary: _, accent, dark, first_word, chapter_num, chapter_title, title_en, image_path } = v;

    let dark_color = if dark.trim().is_empty() { "#1e293b" } else { dark.trim() };
    let accent_color = if accent.trim().is_empty() { "#f59e0b" } else { accent.trim() };

    // 1. Top dark header background
    app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [0.0, 0.0, pw, 138.0],
            "content": "unassigned",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "object.color", json!({"color": dark_color, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Accent divider stripe separating header and content
    app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [0.0, 138.0, pw, 144.0],
            "content": "unassigned",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "object.color", json!({"color": accent_color, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 3. Bookmark ribbon with chevron notch on right side
    let ribbon_x0 = (pw - 140.0).max(10.0);
    let ribbon_x1 = (pw - 40.0).max(ribbon_x0 + 40.0);
    let ribbon_mid_x = (ribbon_x0 + ribbon_x1) * 0.5;

    app.run(
        "path.create",
        json!({
            "spread": spread_idx,
            "closed": true,
            "anchors": [
                {"p": [ribbon_x0, 0.0]},
                {"p": [ribbon_x1, 0.0]},
                {"p": [ribbon_x1, 158.0]},
                {"p": [ribbon_mid_x, 138.0]},
                {"p": [ribbon_x0, 158.0]}
            ]
        }),
    )?;
    fx(app, "object.color", json!({"color": accent_color, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 4, "size": 8, "opacity": 0.25}));

    // 4. Large chapter number inside ribbon
    if !chapter_num.is_empty() {
        let num_font_size = if chapter_num.len() > 2 { 36.0 } else { 48.0 };
        add_cover_text_frame(
            app,
            spread_idx,
            [ribbon_x0, 24.0, ribbon_x1, 116.0],
            chapter_num,
            "#ffffff",
            "Arial Black",
            "Bold",
            num_font_size,
            Some(num_font_size * 1.1),
            None,
            "center",
        )?;
    }

    // 5. Left side header titles
    let text_x1 = ribbon_x0 - 20.0;
    let fw = if first_word.is_empty() { "BAB" } else { first_word };

    // 5a. First word ("BAB" / "CHAPTER")
    add_cover_text_frame(app, spread_idx, [36.0, 16.0, text_x1, 50.0], fw, "#ffffff", "Arial Black", "Bold", 20.0, Some(24.0), None, "left")?;

    // 5b. Chapter title ("Geografi")
    let title_font_size = if chapter_title.len() > 30 { 20.0 } else { 25.0 };
    add_cover_text_frame(
        app,
        spread_idx,
        [36.0, 48.0, text_x1, 96.0],
        chapter_title,
        accent_color,
        "Arial Black",
        "Bold",
        title_font_size,
        Some(title_font_size * 1.15),
        None,
        "left",
    )?;

    // 5c. English subtitle if present
    if !title_en.is_empty() {
        add_cover_text_frame(
            app,
            spread_idx,
            [36.0, 96.0, text_x1, 130.0],
            title_en,
            "#ffffff",
            "Arial",
            "Bold Italic",
            14.0,
            Some(18.0),
            None,
            "left",
        )?;
    }

    // 6. Infographic illustration frame (occupies main page body)
    let body_top = 144.0;
    let body_bottom = ph - 24.0;

    let graphic_res = app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [0.0, body_top, pw, body_bottom],
            "content": "graphic",
            "shape": "rectangle",
            "caret": false
        }),
    )?;

    if let Some(fid) = graphic_res.get("id").and_then(Value::as_u64) {
        fx(app, "object.stroke", json!({"swatch": "[None]"}));
        if let Some(img_p) = image_path.filter(|p| !p.trim().is_empty()) {
            if std::path::Path::new(img_p).exists() {
                fx(app, "file.place", json!({"path": img_p, "frame": fid}));
                fx(app, "object.fit", json!({"mode": "fitContentToFrame", "ids": [fid]}));
                fx(app, "object.fit", json!({"mode": "centerContent", "ids": [fid]}));
                fx(app, "object.fittingOptions", json!({"ids": [fid], "fitting": "fitContentToFrame"}));
            }
        } else {
            // Subtle clean placeholder fill
            fx(app, "object.color", json!({"color": "#f8fafc", "target": "fill"}));
        }
    }

    // 7. Bottom accent strip
    app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [0.0, ph - 24.0, pw, ph],
            "content": "unassigned",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "object.color", json!({"color": dark_color, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [0.0, ph - 28.0, pw, ph - 24.0],
            "content": "unassigned",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "object.color", json!({"color": accent_color, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    Ok(())
}

/// Style 1: Simple Chapter Divider (`001_simple_chapter`)
/// Clean editorial divider with colored primary header, accent badge, and narrative space.
pub(super) fn divider_style_simple_chapter(app: &mut DesignApp, v: DividerArtVals) -> Result<(), String> {
    let DividerArtVals { spread_idx, pw, ph, primary, accent, dark: _, first_word, chapter_num, chapter_title, title_en, image_path } = v;

    let primary_color = if primary.trim().is_empty() { "#0284c7" } else { primary.trim() };
    let accent_color = if accent.trim().is_empty() { "#f59e0b" } else { accent.trim() };

    // 1. Top primary header band
    app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [0.0, 0.0, pw, 180.0],
            "content": "unassigned",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "object.color", json!({"color": primary_color, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Accent stripe below header
    app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [0.0, 180.0, pw, 184.0],
            "content": "unassigned",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "object.color", json!({"color": accent_color, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 3. Chapter badge
    let fw = if first_word.is_empty() { "BAB" } else { first_word };
    let badge_text = if !chapter_num.is_empty() { format!("{fw} {chapter_num}") } else { fw.to_string() };
    let badge_w = (badge_text.len() as f64 * 11.0 + 32.0).clamp(90.0, 220.0);

    app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [36.0, 32.0, 36.0 + badge_w, 68.0],
            "content": "unassigned",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "object.color", json!({"color": accent_color, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(
        app,
        spread_idx,
        [36.0, 38.0, 36.0 + badge_w, 64.0],
        &badge_text,
        "#ffffff",
        "Arial Black",
        "Bold",
        15.0,
        Some(18.0),
        None,
        "center",
    )?;

    // 4. Chapter title
    let title_font_size = if chapter_title.len() > 30 { 22.0 } else { 28.0 };
    add_cover_text_frame(
        app,
        spread_idx,
        [36.0, 80.0, pw - 36.0, 130.0],
        chapter_title,
        "#ffffff",
        "Arial Black",
        "Bold",
        title_font_size,
        Some(title_font_size * 1.15),
        None,
        "left",
    )?;

    // 5. English subtitle
    if !title_en.is_empty() {
        add_cover_text_frame(
            app,
            spread_idx,
            [36.0, 134.0, pw - 36.0, 168.0],
            title_en,
            "#ffffff",
            "Arial",
            "Bold Italic",
            15.0,
            Some(19.0),
            None,
            "left",
        )?;
    }

    // 6. Body content frame
    let body_res = app.run(
        "frame.create",
        json!({
            "spread": spread_idx,
            "rect": [36.0, 210.0, pw - 36.0, ph - 40.0],
            "content": if image_path.is_some() { "graphic" } else { "text" },
            "shape": "rectangle",
            "caret": false
        }),
    )?;

    if let Some(fid) = body_res.get("id").and_then(Value::as_u64)
        && let Some(img_p) = image_path.filter(|p| !p.trim().is_empty())
        && std::path::Path::new(img_p).exists()
    {
        fx(app, "file.place", json!({"path": img_p, "frame": fid}));
        fx(app, "object.fit", json!({"mode": "fitProportionally", "ids": [fid]}));
        fx(app, "object.fit", json!({"mode": "centerContent", "ids": [fid]}));
    }

    Ok(())
}
