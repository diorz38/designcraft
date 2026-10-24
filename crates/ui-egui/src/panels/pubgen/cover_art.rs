//! Procedural cover art: the hand-drawn fallback styles, drawn command-by-command when no
//! `.designcraft` cover template matches the chosen preset.

use super::*;

/// Best-effort command for the procedural cover art: a failure cannot abort the style, but
/// in debug builds it prints, so a typo'd command id cannot stay silently broken forever.
fn fx(app: &mut DesignApp, command: &str, params: Value) {
    if cfg!(debug_assertions)
        && let Err(e) = app.run(command, params)
    {
        eprintln!("pubgen cover art: {command} failed: {e}");
    }
}

/// Shared values for the procedural (no-template) cover styles.
#[derive(Clone, Copy)]
pub(super) struct CoverArtVals<'a> {
    pub(super) pw: f64,
    pub(super) ph: f64,
    pub(super) primary: &'a str,
    pub(super) accent: &'a str,
    pub(super) dark: &'a str,
    pub(super) title_text: &'a str,
    pub(super) title2_text: &'a str,
    pub(super) year_text: &'a str,
    pub(super) volume_text: &'a str,
    pub(super) cat_text: &'a str,
    pub(super) code_text: &'a str,
    pub(super) agency_text: &'a str,
    pub(super) logo: &'a str,
    pub(super) skyline: &'a str,
    pub(super) landscape: &'a str,
}

/// Procedural cover style 0 — Wave Curves. The hand-drawn fallback when no template matches.
pub(super) fn cover_style_0(app: &mut DesignApp, v: CoverArtVals) -> Result<(), String> {
    let CoverArtVals {
        pw,
        ph,
        primary,
        accent,
        dark,
        title_text,
        title2_text,
        year_text,
        volume_text,
        cat_text,
        code_text,
        agency_text,
        logo,
        skyline,
        ..
    } = v;
    // Style 0: Wave Curves
    // 1. Dark charcoal/theme full background
    app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": dark, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Concentric Bottom Cradle
    app.run("frame.create", json!({"spread": 0, "rect": [123.64, 246.0, 471.64, 594.0], "content": "unassigned", "shape": "ellipse"}))?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.3}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run("frame.create", json!({"spread": 0, "rect": [130.64, 253.0, 464.64, 587.0], "content": "unassigned", "shape": "ellipse"}))?;
    fx(app, "object.color", json!({"color": adjust_hex(accent, 1.15), "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 3. Blue Wave Swoosh
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [pw, 280.0]}, {"p": [pw, 430.0], "in": [pw, 380.0], "out": [510.0, 490.0]},
            {"p": [380.0, 565.0], "in": [450.0, 545.0], "out": [320.0, 580.0]},
            {"p": [240.0, 560.0], "in": [270.0, 568.0], "out": [240.0, 560.0]},
            {"p": [330.0, 545.0], "in": [260.0, 550.0], "out": [430.0, 500.0]},
            {"p": [pw, 280.0], "in": [490.0, 390.0], "out": [pw, 280.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 8, "size": 16, "opacity": 0.32}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 4. White Top Wave
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 0.0]}, {"p": [pw, 0.0]}, {"p": [pw, 280.0], "out": [490.0, 390.0]},
            {"p": [330.0, 545.0], "in": [430.0, 500.0], "out": [260.0, 550.0]},
            {"p": [160.0, 542.0], "in": [200.0, 545.0], "out": [80.0, 530.0]},
            {"p": [0.0, 490.0], "in": [50.0, 505.0]}, {"p": [0.0, 0.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 8, "size": 16, "opacity": 0.22}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 5. Circular Medallion Rings
    app.run("frame.create", json!({"spread": 0, "rect": [145.64, 268.0, 449.64, 572.0], "content": "unassigned", "shape": "ellipse"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.25}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run("frame.create", json!({"spread": 0, "rect": [151.64, 274.0, 443.64, 566.0], "content": "unassigned", "shape": "ellipse"}))?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run("frame.create", json!({"spread": 0, "rect": [158.64, 281.0, 436.64, 559.0], "content": "unassigned", "shape": "ellipse"}))?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // Photo frame
    app.run("frame.create", json!({"spread": 0, "rect": [161.64, 284.0, 433.64, 556.0], "content": "graphic", "shape": "ellipse"}))?;
    fx(app, "file.place", json!({"path": skyline}));
    fx(app, "object.fit", json!({"mode": "fillProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 6. Top-Right Metadata (#5 and #6)
    add_cover_text_frame(app, 0, [320.0, 24.0, 555.0, 42.0], cat_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;
    add_cover_text_frame(app, 0, [320.0, 42.0, 555.0, 60.0], code_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;

    // 7. Headline Block
    add_cover_text_frame(app, 0, [20.0, 126.0, 575.0, 186.0], title_text, primary, "Arial Black", "Regular", 33.0, Some(39.0), Some(30.0), "center")?;
    // The second headline line: smaller, and in the accent colour.
    add_cover_text_frame(app, 0, [20.0, 186.0, 575.0, 225.0], title2_text, accent, "Arial Black", "Regular", 24.0, Some(28.0), Some(30.0), "center")?;

    // 8. Bottom-Left BPS Logo
    app.run("frame.create", json!({"spread": 0, "rect": [40.0, 742.0, 84.0, 788.0], "content": "graphic", "shape": "rectangle"}))?;
    fx(app, "file.place", json!({"path": logo}));
    fx(app, "object.fit", json!({"mode": "fitProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(app, 0, [92.0, 746.0, 290.0, 788.0], agency_text, "#ffffff", "Arial", "Bold Italic", 10.0, Some(13.5), Some(20.0), "left")?;

    // 9. Bottom-Right Year & Volume
    add_cover_text_frame(app, 0, [320.0, 712.0, 555.0, 768.0], year_text, accent, "Arial Black", "Regular", 48.0, None, Some(30.0), "right")?;
    add_cover_text_frame(app, 0, [320.0, 768.0, 555.0, 795.0], volume_text, "#cbd5e1", "Arial", "Bold", 12.0, None, Some(20.0), "right")?;
    Ok(())
}

/// Procedural cover style 1 — Geometric Medallion. The hand-drawn fallback when no template matches.
pub(super) fn cover_style_1(app: &mut DesignApp, v: CoverArtVals) -> Result<(), String> {
    let CoverArtVals {
        pw,
        ph,
        primary,
        accent,
        dark,
        title_text,
        title2_text,
        year_text,
        volume_text,
        cat_text,
        code_text,
        agency_text,
        logo,
        skyline,
        landscape,
        ..
    } = v;
    // Style 1: Geometric & Medallion
    // 1. Clean White Base
    app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Top-Right Metadata (#5 and #6)
    add_cover_text_frame(app, 0, [320.0, 24.0, 555.0, 42.0], cat_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;
    add_cover_text_frame(app, 0, [320.0, 42.0, 555.0, 60.0], code_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;

    // 3. Diagonal Polygon Graphic Frame (Landscape)
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 390.0]}, {"p": [170.0, 230.0]}, {"p": [410.0, 230.0]},
            {"p": [pw, 75.0]}, {"p": [pw, 550.0]}, {"p": [340.0, 550.0]},
            {"p": [160.0, 580.0]}, {"p": [0.0, 680.0]}
        ]}),
    )?;
    fx(app, "object.content", json!({"type": "graphic"}));
    fx(app, "file.place", json!({"path": landscape}));
    fx(app, "object.fit", json!({"mode": "fillProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // Multiply color overlay
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 390.0]}, {"p": [170.0, 230.0]}, {"p": [410.0, 230.0]},
            {"p": [pw, 75.0]}, {"p": [pw, 550.0]}, {"p": [340.0, 550.0]},
            {"p": [160.0, 580.0]}, {"p": [0.0, 680.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.opacity", json!({"opacity": 0.82, "blend": "multiply"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // Accent stripes
    app.run("path.create", json!({"spread": 0, "closed": false, "anchors": [{"p": [10.0, 365.0]}, {"p": [155.0, 222.0]}]}))?;
    fx(app, "object.stroke", json!({"weight": 7.0}));
    fx(app, "object.color", json!({"color": accent, "target": "stroke"}));

    app.run("path.create", json!({"spread": 0, "closed": false, "anchors": [{"p": [435.0, 220.0]}, {"p": [585.0, 95.0]}]}))?;
    fx(app, "object.stroke", json!({"weight": 7.0}));
    fx(app, "object.color", json!({"color": accent, "target": "stroke"}));

    // Bottom-left accent ribbon
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 680.0]}, {"p": [155.0, 575.0]}, {"p": [170.0, 590.0]}, {"p": [0.0, 715.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // Ribbon fold
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [140.0, 588.0]}, {"p": [170.0, 590.0]}, {"p": [162.0, 565.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": adjust_hex(accent, 0.85), "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // Circular photo cutout
    app.run("frame.create", json!({"spread": 0, "rect": [217.0, 292.0, 533.0, 608.0], "content": "unassigned", "shape": "ellipse"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 8, "size": 16, "opacity": 0.28}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run("frame.create", json!({"spread": 0, "rect": [223.0, 298.0, 527.0, 602.0], "content": "graphic", "shape": "ellipse"}))?;
    fx(app, "file.place", json!({"path": skyline}));
    fx(app, "object.fit", json!({"mode": "fillProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // Accent arc around circle
    app.run(
        "path.create",
        json!({"spread": 0, "closed": false, "anchors": [
            {"p": [340.0, 615.0], "out": [390.0, 625.0]},
            {"p": [480.0, 595.0], "in": [435.0, 620.0]}
        ]}),
    )?;
    fx(app, "object.stroke", json!({"weight": 5.0}));
    fx(app, "object.color", json!({"color": accent, "target": "stroke"}));

    // Headline Block
    add_cover_text_frame(app, 0, [35.0, 90.0, 440.0, 142.0], title_text, accent, "Arial Black", "Regular", 34.0, None, Some(30.0), "left")?;
    add_cover_text_frame(app, 0, [35.0, 142.0, 440.0, 195.0], title2_text, dark, "Arial Black", "Regular", 24.0, None, Some(30.0), "left")?;

    // Bottom-Left Logo
    app.run("frame.create", json!({"spread": 0, "rect": [35.0, 742.0, 82.0, 788.0], "content": "graphic", "shape": "rectangle"}))?;
    fx(app, "file.place", json!({"path": logo}));
    fx(app, "object.fit", json!({"mode": "fitProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(app, 0, [90.0, 746.0, 280.0, 788.0], agency_text, dark, "Arial", "Bold Italic", 10.0, Some(13.5), Some(20.0), "left")?;

    // Bottom-Right Year & Volume
    add_cover_text_frame(app, 0, [320.0, 712.0, 560.0, 768.0], year_text, accent, "Arial Black", "Regular", 48.0, None, Some(30.0), "right")?;
    add_cover_text_frame(app, 0, [320.0, 768.0, 560.0, 795.0], volume_text, dark, "Arial", "Bold", 12.0, None, Some(20.0), "right")?;
    Ok(())
}

/// Procedural cover style 2 — Folded Origami. The hand-drawn fallback when no template matches.
pub(super) fn cover_style_2(app: &mut DesignApp, v: CoverArtVals) -> Result<(), String> {
    let CoverArtVals {
        pw,
        ph,
        primary,
        accent,
        dark,
        title_text,
        title2_text,
        year_text,
        volume_text,
        cat_text,
        code_text,
        agency_text,
        logo,
        skyline,
        ..
    } = v;
    // Style 2: Folded Origami
    // 1. Clean White Base
    app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Photo Polygon Frame (Top-Right Hexagonal Cutout)
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [235.0, 0.0]}, {"p": [pw, 0.0]}, {"p": [pw, 290.0]},
            {"p": [325.0, 405.0]}, {"p": [105.0, 235.0]}, {"p": [105.0, 75.0]}
        ]}),
    )?;
    fx(app, "object.content", json!({"type": "graphic"}));
    fx(app, "file.place", json!({"path": skyline}));
    fx(app, "object.fit", json!({"mode": "fillProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 3. Ribbon Border along Photo Lower Edge
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [105.0, 235.0]}, {"p": [325.0, 405.0]}, {"p": [pw, 290.0]},
            {"p": [pw, 305.0]}, {"p": [325.0, 420.0]}, {"p": [105.0, 248.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 4. Top-Left Accent Corner
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 0.0]}, {"p": [235.0, 0.0]}, {"p": [105.0, 75.0]},
            {"p": [105.0, 160.0]}, {"p": [0.0, 95.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 5. Dark Navy / Theme Dark Polygon
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 95.0]}, {"p": [105.0, 160.0]}, {"p": [105.0, 248.0]},
            {"p": [265.0, 360.0]}, {"p": [145.0, 435.0]}, {"p": [0.0, 350.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": dark, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 6. Amber / Shaded Facet on Left
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 350.0]}, {"p": [145.0, 435.0]}, {"p": [0.0, 500.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": adjust_hex(accent, 0.90), "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 7. Folded Origami Ribbon Facets
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [145.0, 435.0]}, {"p": [265.0, 360.0]}, {"p": [325.0, 420.0]}, {"p": [325.0, 540.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": adjust_hex(accent, 0.98), "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [325.0, 420.0]}, {"p": [pw, 305.0]}, {"p": [pw, 350.0]},
            {"p": [460.0, 460.0]}, {"p": [325.0, 540.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": adjust_hex(accent, 1.08), "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [460.0, 460.0]}, {"p": [pw, 350.0]}, {"p": [pw, 540.0]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": adjust_hex(accent, 0.92), "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 8. Top-Right Metadata (#5 and #6)
    add_cover_text_frame(app, 0, [320.0, 24.0, 555.0, 42.0], cat_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;
    add_cover_text_frame(app, 0, [320.0, 42.0, 555.0, 60.0], code_text, dark, "Arial", "Bold", 8.5, None, None, "right")?;

    // 9. Headline Block
    add_cover_text_frame(app, 0, [40.0, 597.0, 550.0, 647.0], title_text, primary, "Arial Black", "Regular", 38.0, None, Some(20.0), "left")?;
    add_cover_text_frame(app, 0, [40.0, 647.0, 550.0, 697.0], title2_text, accent, "Arial Black", "Regular", 28.0, None, Some(20.0), "left")?;

    // 10. Bottom-Left Logo
    app.run("frame.create", json!({"spread": 0, "rect": [40.0, 742.0, 84.0, 788.0], "content": "graphic", "shape": "rectangle"}))?;
    fx(app, "file.place", json!({"path": logo}));
    fx(app, "object.fit", json!({"mode": "fitProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(app, 0, [92.0, 746.0, 290.0, 788.0], agency_text, dark, "Arial", "Bold Italic", 10.0, Some(13.5), Some(20.0), "left")?;

    // 11. Bottom-Right Year & Volume
    add_cover_text_frame(app, 0, [320.0, 712.0, 555.0, 768.0], year_text, accent, "Arial Black", "Regular", 48.0, None, Some(30.0), "right")?;
    add_cover_text_frame(app, 0, [320.0, 768.0, 555.0, 795.0], volume_text, dark, "Arial", "Bold", 12.0, None, Some(20.0), "right")?;
    Ok(())
}

/// Procedural cover style 3 — Chevron & Medallion. The hand-drawn fallback when no template matches.
pub(super) fn cover_style_3(app: &mut DesignApp, v: CoverArtVals) -> Result<(), String> {
    let CoverArtVals {
        pw,
        ph,
        primary,
        accent,
        dark,
        title_text,
        title2_text,
        year_text,
        volume_text,
        cat_text,
        code_text,
        agency_text,
        logo,
        skyline,
        ..
    } = v;
    // Style 3: Chevron & Medallion (from design-1.designcraft)
    let sx = pw / 595.275591;
    let sy = ph / 841.889764;
    let s_avg = (sx + sy) / 2.0;

    // 1. Crisp white base (Item 11)
    app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Mid-body subtle tint (Item 12)
    app.run("frame.create", json!({"spread": 0, "rect": [0, 117.84 * sy, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": "#f8fafc", "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 3. Oval Photo Medallion (Item 17) - Resized & placed behind chevron
    app.run(
        "frame.create",
        json!({
            "spread": 0,
            "rect": [162.68 * sx, 339.71 * sy, 437.95 * sx, 629.56 * sy],
            "content": "graphic",
            "shape": "ellipse"
        }),
    )?;
    fx(app, "file.place", json!({"path": skyline}));
    fx(app, "object.fit", json!({"mode": "fillProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 4. Downward Chevron (Item 13)
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.71 * sx, 0.0], "out": [151.68 * sx, 0.0]},
            {"p": [pw, 0.0], "in": [377.97 * sx, 0.0], "out": [pw, 42.97 * sy]},
            {"p": [pw, 147.47 * sy], "in": [pw, 94.38 * sy], "out": [449.39 * sx, 292.88 * sy]},
            {"p": [294.74 * sx, 447.03 * sy]},
            {"p": [0.71 * sx, 153.94 * sy], "in": [126.35 * sx, 279.18 * sy], "out": [0.71 * sx, 107.50 * sy]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.22}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 5. Accent bottom stroke along Chevron (Item 14)
    app.run(
        "path.create",
        json!({"spread": 0, "closed": false, "anchors": [
            {"p": [0.71 * sx, 153.94 * sy], "out": [126.35 * sx, 279.18 * sy]},
            {"p": [294.74 * sx, 447.03 * sy]},
            {"p": [pw, 147.47 * sy], "in": [449.39 * sx, 292.88 * sy]}
        ]}),
    )?;
    fx(app, "object.stroke", json!({"weight": 4.25 * s_avg}));
    fx(app, "object.color", json!({"color": accent, "target": "stroke"}));

    // 6. Horseshoe Medallion Ring (Item 15)
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [116.22 * sx, 484.53 * sy], "in": [116.22 * sx, 585.77 * sy], "out": [116.22 * sx, 428.43 * sy]},
            {"p": [181.35 * sx, 344.59 * sy], "in": [141.51 * sx, 378.21 * sy], "out": [188.35 * sx, 351.56 * sy]},
            {"p": [202.65 * sx, 365.80 * sy], "in": [195.50 * sx, 358.69 * sy], "out": [168.58 * sx, 393.82 * sy]},
            {"p": [146.87 * sx, 483.69 * sy], "in": [146.87 * sx, 436.22 * sy], "out": [146.87 * sx, 568.05 * sy]},
            {"p": [300.12 * sx, 636.43 * sy], "in": [215.48 * sx, 636.43 * sy], "out": [384.74 * sx, 636.43 * sy]},
            {"p": [453.36 * sx, 483.69 * sy], "in": [453.36 * sx, 568.05 * sy], "out": [453.36 * sx, 433.45 * sy]},
            {"p": [391.47 * sx, 361.04 * sy], "in": [429.02 * sx, 388.87 * sy], "out": [400.42 * sx, 352.11 * sy]},
            {"p": [412.85 * sx, 339.71 * sy], "in": [407.95 * sx, 344.59 * sy], "out": [456.14 * sx, 373.24 * sy]},
            {"p": [484.01 * sx, 484.53 * sy], "in": [484.01 * sx, 425.64 * sy], "out": [484.01 * sx, 585.77 * sy]},
            {"p": [300.12 * sx, 667.83 * sy], "in": [401.67 * sx, 667.83 * sy], "out": [198.55 * sx, 667.83 * sy]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 6, "size": 16, "opacity": 0.28}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 7. Bottom Footer Bar (Item 19)
    app.run("frame.create", json!({"spread": 0, "rect": [0.0, 757.00 * sy, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": dark, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 8. Footer Top Accent Stroke (Item 20)
    app.run(
        "path.create",
        json!({"spread": 0, "closed": false, "anchors": [
            {"p": [0.0, 757.00 * sy]},
            {"p": [pw, 757.00 * sy]}
        ]}),
    )?;
    fx(app, "object.stroke", json!({"weight": 3.5 * sy}));
    fx(app, "object.color", json!({"color": accent, "target": "stroke"}));

    // 9. Top-Right Metadata (Items 21 & 23)
    add_cover_text_frame(
        app,
        0,
        [311.49 * sx, 22.63 * sy, 569.74 * sx, 42.43 * sy],
        cat_text,
        "#ffffff",
        "Arial",
        "Bold",
        8.0 * s_avg,
        None,
        None,
        "right",
    )?;
    add_cover_text_frame(
        app,
        0,
        [311.49 * sx, 42.43 * sy, 569.74 * sx, 62.23 * sy],
        code_text,
        "#ffffff",
        "Arial",
        "Bold",
        8.0 * s_avg,
        None,
        None,
        "right",
    )?;

    // 10. Headline Block Inside Chevron (Items 25 & 27)
    add_cover_text_frame(
        app,
        0,
        [21.28 * sx, 201.46 * sy, 573.99 * sx, 275.00 * sy],
        title_text,
        "#ffffff",
        "Arial Black",
        "Regular",
        26.0 * s_avg,
        Some(31.0 * s_avg),
        Some(20.0),
        "center",
    )?;
    // The second headline line: smaller, and in the accent colour.
    add_cover_text_frame(
        app,
        0,
        [21.28 * sx, 275.00 * sy, 573.99 * sx, 330.00 * sy],
        title2_text,
        accent,
        "Arial Black",
        "Regular",
        19.0 * s_avg,
        Some(23.0 * s_avg),
        Some(20.0),
        "center",
    )?;

    // 11. Center Medallion Year (Item 33) - 65pt Arial Black in Accent Gold!
    add_cover_text_frame(
        app,
        0,
        [160.0 * sx, 440.0 * sy, 440.0 * sx, 530.0 * sy],
        year_text,
        accent,
        "Arial Black",
        "Regular",
        65.0 * s_avg,
        None,
        Some(20.0),
        "center",
    )?;

    // 12. Bottom-Left Logo & Agency Text (Items 29 & 31)
    app.run(
        "frame.create",
        json!({
            "spread": 0,
            "rect": [28.38 * sx, 770.79 * sy, 82.30 * sx, 827.36 * sy],
            "content": "graphic",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "file.place", json!({"path": logo}));
    fx(app, "object.fit", json!({"mode": "fitProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(
        app,
        0,
        [93.65 * sx, 773.61 * sy, 368.92 * sx, 830.19 * sy],
        agency_text,
        "#ffffff",
        "Arial",
        "Bold Italic",
        9.5 * s_avg,
        Some(12.5 * s_avg),
        Some(15.0),
        "left",
    )?;

    // 13. Bottom-Right Volume (Item 35)
    add_cover_text_frame(
        app,
        0,
        [368.92 * sx, 766.80 * sy, 569.74 * sx, 810.0 * sy],
        volume_text,
        "#cbd5e1",
        "Arial",
        "Bold",
        9.0 * s_avg,
        None,
        Some(15.0),
        "right",
    )?;
    Ok(())
}

/// Procedural cover style 4 — Honeycomb Hexagon. The hand-drawn fallback when no template matches.
pub(super) fn cover_style_4(app: &mut DesignApp, v: CoverArtVals) -> Result<(), String> {
    let CoverArtVals {
        pw,
        ph,
        primary,
        accent,
        dark,
        title_text,
        title2_text,
        year_text,
        volume_text,
        cat_text,
        code_text,
        agency_text,
        logo,
        landscape,
        ..
    } = v;
    // Style 4: Honeycomb Hexagon (from design-2.designcraft)
    let sx = pw / 595.275591;
    let sy = ph / 841.889764;
    let s_avg = (sx + sy) / 2.0;

    // 1. Deep Midnight Executive Background (Item 24)
    app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": dark, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Three Vertical Accent Stripes (Items 26, 27, 28)
    app.run("frame.create", json!({"spread": 0, "rect": [355.03 * sx, 0.0, 407.97 * sx, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.opacity", json!({"opacity": 0.85}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run("frame.create", json!({"spread": 0, "rect": [415.03 * sx, 0.0, 443.94 * sx, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.opacity", json!({"opacity": 0.60}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run("frame.create", json!({"spread": 0, "rect": [451.03 * sx, 0.0, 462.92 * sx, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.opacity", json!({"opacity": 0.35}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 3. Left Horizontal Chevron Banner (Item 13) in Corporate Sapphire
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 199.74 * sy]},
            {"p": [176.59 * sx, 199.74 * sy]},
            {"p": [300.92 * sx, 324.07 * sy]},
            {"p": [180.99 * sx, 444.00 * sy]},
            {"p": [0.0, 444.00 * sy]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.30}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // Gold bottom accent rule along chevron
    app.run(
        "path.create",
        json!({"spread": 0, "closed": false, "anchors": [
            {"p": [0.0, 444.00 * sy]},
            {"p": [180.99 * sx, 444.00 * sy]},
            {"p": [300.92 * sx, 324.07 * sy]}
        ]}),
    )?;
    fx(app, "object.stroke", json!({"weight": 4.0 * s_avg}));
    fx(app, "object.color", json!({"color": accent, "target": "stroke"}));

    // 4. Right Horizontal Banner Extension (Item 17)
    app.run(
        "frame.create",
        json!({
            "spread": 0,
            "rect": [514.35 * sx, 304.60 * sy, pw, 534.00 * sy],
            "content": "unassigned",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run(
        "path.create",
        json!({"spread": 0, "closed": false, "anchors": [
            {"p": [514.35 * sx, 534.00 * sy]},
            {"p": [pw, 534.00 * sy]}
        ]}),
    )?;
    fx(app, "object.stroke", json!({"weight": 4.0 * s_avg}));
    fx(app, "object.color", json!({"color": accent, "target": "stroke"}));

    // 5. Tri-Hexagon Honeycomb Outer White Matting & Shadow (Item 19)
    let tri_hex_anchors = json!([
        {"p": [180.81 * sx, 424.99 * sy]},
        {"p": [243.85 * sx, 324.44 * sy]},
        {"p": [346.12 * sx, 325.68 * sy]},
        {"p": [408.51 * sx, 223.50 * sy]},
        {"p": [526.58 * sx, 224.97 * sy]},
        {"p": [579.76 * sx, 332.72 * sy]},
        {"p": [522.00 * sx, 428.00 * sy]},
        {"p": [576.26 * sx, 527.77 * sy]},
        {"p": [512.15 * sx, 626.18 * sy]},
        {"p": [395.52 * sx, 624.82 * sy]},
        {"p": [344.69 * sx, 528.28 * sy]},
        {"p": [232.47 * sx, 526.92 * sy]}
    ]);

    app.run("path.create", json!({"spread": 0, "closed": true, "anchors": tri_hex_anchors}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 6, "size": 18, "opacity": 0.35}));
    fx(app, "object.stroke", json!({"weight": 8.0 * s_avg}));
    fx(app, "object.color", json!({"color": "#ffffff", "target": "stroke"}));

    // 6. Tri-Hexagon Photo Frame
    let hex_res = app.run("path.create", json!({"spread": 0, "closed": true, "anchors": tri_hex_anchors}))?;
    let hex_id = hex_res.get("id").and_then(serde_json::Value::as_u64);
    fx(app, "object.content", json!({"type": "graphic"}));
    fx(app, "file.place", json!({"path": landscape, "frame": hex_id}));
    fx(app, "object.fit", json!({"mode": "fillProportionally", "ids": [hex_id]}));
    fx(app, "object.fit", json!({"mode": "centerContent", "ids": [hex_id]}));
    fx(app, "object.stroke", json!({"swatch": "[None]", "ids": [hex_id]}));

    // Inner gold piping around Tri-Hexagon
    app.run("path.create", json!({"spread": 0, "closed": true, "anchors": tri_hex_anchors}))?;
    fx(app, "object.stroke", json!({"weight": 2.5 * s_avg}));
    fx(app, "object.color", json!({"color": accent, "target": "stroke"}));

    // 7. Top-Left Logo & Agency Text
    app.run(
        "frame.create",
        json!({
            "spread": 0,
            "rect": [36.0 * sx, 32.0 * sy, 74.0 * sx, 72.0 * sy],
            "content": "graphic",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "file.place", json!({"path": logo}));
    fx(app, "object.fit", json!({"mode": "fitProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(
        app,
        0,
        [82.0 * sx, 34.0 * sy, 320.0 * sx, 72.0 * sy],
        agency_text,
        "#ffffff",
        "Arial",
        "Bold Italic",
        9.0 * s_avg,
        Some(12.0 * s_avg),
        Some(15.0),
        "left",
    )?;

    // 8. Top-Right Metadata
    add_cover_text_frame(
        app,
        0,
        [pw - 250.0 * sx, 34.0 * sy, pw - 36.0 * sx, 50.0 * sy],
        cat_text,
        "#ffffff",
        "Arial",
        "Bold",
        8.0 * s_avg,
        None,
        None,
        "right",
    )?;
    add_cover_text_frame(
        app,
        0,
        [pw - 250.0 * sx, 50.0 * sy, pw - 36.0 * sx, 66.0 * sy],
        code_text,
        "#ffffff",
        "Arial",
        "Bold",
        8.0 * s_avg,
        None,
        None,
        "right",
    )?;

    // 9. Headline Block (Inside Left Chevron Banner)
    add_cover_text_frame(
        app,
        0,
        [36.0 * sx, 265.0 * sy, 270.0 * sx, 340.0 * sy],
        title_text,
        "#ffffff",
        "Arial Black",
        "Regular",
        25.0 * s_avg,
        Some(30.0 * s_avg),
        Some(15.0),
        "left",
    )?;
    // The second headline line: smaller, and in the accent colour.
    add_cover_text_frame(
        app,
        0,
        [36.0 * sx, 340.0 * sy, 270.0 * sx, 395.0 * sy],
        title2_text,
        accent,
        "Arial Black",
        "Regular",
        18.0 * s_avg,
        Some(22.0 * s_avg),
        Some(15.0),
        "left",
    )?;

    // 10. Lower-Left Focal Year Block
    add_cover_text_frame(
        app,
        0,
        [36.0 * sx, 640.0 * sy, 260.0 * sx, 715.0 * sy],
        year_text,
        "#ffffff",
        "Arial Black",
        "Regular",
        60.0 * s_avg,
        None,
        Some(15.0),
        "left",
    )?;

    app.run(
        "path.create",
        json!({"spread": 0, "closed": false, "anchors": [
            {"p": [36.0 * sx, 722.0 * sy]},
            {"p": [200.0 * sx, 722.0 * sy]}
        ]}),
    )?;
    fx(app, "object.stroke", json!({"weight": 4.0 * s_avg}));
    fx(app, "object.color", json!({"color": accent, "target": "stroke"}));

    add_cover_text_frame(
        app,
        0,
        [36.0 * sx, 730.0 * sy, 260.0 * sx, 755.0 * sy],
        volume_text,
        "#cbd5e1",
        "Arial",
        "Bold",
        11.0 * s_avg,
        None,
        Some(20.0),
        "left",
    )?;

    // 11. Bottom Subtle Rule
    app.run(
        "path.create",
        json!({"spread": 0, "closed": false, "anchors": [
            {"p": [36.0 * sx, 795.0 * sy]},
            {"p": [pw - 36.0 * sx, 795.0 * sy]}
        ]}),
    )?;
    fx(app, "object.stroke", json!({"weight": 1.0 * s_avg}));
    fx(app, "object.color", json!({"color": "#334155", "target": "stroke"}));

    add_cover_text_frame(
        app,
        0,
        [36.0 * sx, 804.0 * sy, 300.0 * sx, 824.0 * sy],
        "PUBLIKASI BPS KABUPATEN KARAWANG",
        "#94a3b8",
        "Arial",
        "Bold",
        8.5 * s_avg,
        None,
        None,
        "left",
    )?;
    add_cover_text_frame(
        app,
        0,
        [pw - 200.0 * sx, 804.0 * sy, pw - 36.0 * sx, 824.0 * sy],
        "bps.go.id",
        accent,
        "Arial",
        "Bold",
        8.5 * s_avg,
        None,
        None,
        "right",
    )?;
    Ok(())
}

/// Procedural cover style 5 — Corporate Headline. The hand-drawn fallback when no template matches.
pub(super) fn cover_style_5(app: &mut DesignApp, v: CoverArtVals) -> Result<(), String> {
    let CoverArtVals {
        pw, ph, primary, dark, title_text, title2_text, year_text, volume_text, cat_text, code_text, agency_text, logo, skyline, ..
    } = v;
    // Style 5: Corporate Headline (from design-3.designcraft)
    let sx = pw / 595.275591;
    let sy = ph / 841.889764;
    let s_avg = (sx + sy) / 2.0;

    // 1. Base white background
    app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Top accent bar
    app.run("frame.create", json!({"spread": 0, "rect": [0.0, 0.0, 548.0 * sx, 16.0 * sy], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 3. Top-Right Metadata (Position Lock: Line #1 Catalogue, Line #2 ISSN/ISBN)
    add_cover_text_frame(
        app,
        0,
        [pw - 260.0 * sx, 24.0 * sy, pw - 36.0 * sx, 42.0 * sy],
        cat_text,
        dark,
        "Arial",
        "Bold",
        8.5 * s_avg,
        None,
        None,
        "right",
    )?;
    add_cover_text_frame(
        app,
        0,
        [pw - 260.0 * sx, 42.0 * sy, pw - 36.0 * sx, 60.0 * sy],
        code_text,
        dark,
        "Arial",
        "Bold",
        8.5 * s_avg,
        None,
        None,
        "right",
    )?;

    // 4. Bottom-right purple block
    app.run("frame.create", json!({"spread": 0, "rect": [438.0 * sx, 780.0 * sy, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 5. Mid-right accent tab
    app.run("frame.create", json!({"spread": 0, "rect": [565.0 * sx, 680.0 * sy, pw, 720.0 * sy], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 6. Center-right vertical photo frame
    let photo_res = app.run(
        "frame.create",
        json!({
            "spread": 0,
            "rect": [221.0 * sx, 168.0 * sy, 565.0 * sx, 780.0 * sy],
            "content": "graphic",
            "shape": "rectangle"
        }),
    )?;
    let photo_id = photo_res.get("id").and_then(Value::as_u64);
    fx(app, "file.place", json!({"path": skyline, "frame": photo_id}));
    fx(app, "object.fit", json!({"mode": "fillProportionally", "ids": [photo_id]}));
    fx(app, "object.fit", json!({"mode": "centerContent", "ids": [photo_id]}));
    fx(app, "object.stroke", json!({"swatch": "[None]", "ids": [photo_id]}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 8, "size": 18, "opacity": 0.22, "ids": [photo_id]}));

    // 7. Top-left L-bracket framing the photo
    app.run(
        "frame.create",
        json!({"spread": 0, "rect": [213.0 * sx, 161.0 * sy, 325.0 * sx, 170.0 * sy], "content": "unassigned", "shape": "rectangle"}),
    )?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    app.run(
        "frame.create",
        json!({"spread": 0, "rect": [213.0 * sx, 161.0 * sy, 221.0 * sx, 265.0 * sy], "content": "unassigned", "shape": "rectangle"}),
    )?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 8. Top-right purple accent card (without placeholder "Corporate Design" text)
    app.run("frame.create", json!({"spread": 0, "rect": [342.0 * sx, 108.0 * sy, pw, 234.0 * sy], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.22}));

    // 9. Middle-left Big Year & Volume (replacing Business Report)
    let (y1, y2) = if year_text.len() >= 4 { (&year_text[..2], &year_text[2..]) } else { (year_text, "") };
    add_cover_text_frame(
        app,
        0,
        [36.0 * sx, 260.0 * sy, 118.0 * sx, 335.0 * sy],
        y1,
        dark,
        "Arial Black",
        "Regular",
        62.0 * s_avg,
        None,
        Some(10.0),
        "left",
    )?;
    if !y2.is_empty() {
        add_cover_text_frame(
            app,
            0,
            [116.0 * sx, 260.0 * sy, 215.0 * sx, 335.0 * sy],
            y2,
            primary,
            "Arial Black",
            "Regular",
            62.0 * s_avg,
            None,
            Some(10.0),
            "left",
        )?;
    }

    // Volume text directly under Year numerals
    add_cover_text_frame(
        app,
        0,
        [36.0 * sx, 340.0 * sy, 220.0 * sx, 360.0 * sy],
        volume_text,
        dark,
        "Arial",
        "Bold",
        11.0 * s_avg,
        None,
        Some(25.0),
        "left",
    )?;

    // Accent underline rule
    app.run(
        "path.create",
        json!({"spread": 0, "closed": false, "anchors": [
            {"p": [36.0 * sx, 364.0 * sy]},
            {"p": [205.0 * sx, 364.0 * sy]}
        ]}),
    )?;
    fx(app, "object.stroke", json!({"weight": 2.5 * s_avg}));
    fx(app, "object.color", json!({"color": primary, "target": "stroke"}));

    // 10. Headline Purple Block (Bottom-Left)
    let r_corner = 46.0 * s_avg;
    app.run(
        "path.create",
        json!({
            "spread": 0,
            "closed": true,
            "anchors": [
                {"p": [0.0, 458.0 * sy]},
                {"p": [340.0 * sx - r_corner, 458.0 * sy], "out": [340.0 * sx - r_corner + r_corner * 0.552, 458.0 * sy]},
                {"p": [340.0 * sx, 458.0 * sy + r_corner], "in": [340.0 * sx, 458.0 * sy + r_corner - r_corner * 0.552]},
                {"p": [340.0 * sx, 680.0 * sy]},
                {"p": [0.0, 680.0 * sy]}
            ]
        }),
    )?;
    fx(app, "object.color", json!({"color": primary, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));
    fx(app, "object.opacity", json!({"opacity": 0.85}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 8, "size": 16, "opacity": 0.28}));

    let formatted_title = if title_text.contains(' ') { title_text.replace(' ', "\n") } else { title_text.to_string() };
    add_cover_text_frame(
        app,
        0,
        [26.0 * sx, 508.0 * sy, 330.0 * sx, 616.0 * sy],
        &formatted_title,
        "#ffffff",
        "Arial Black",
        "Regular",
        34.0 * s_avg,
        Some(39.0 * s_avg),
        Some(10.0),
        "left",
    )?;
    add_cover_text_frame(
        app,
        0,
        [28.0 * sx, 624.0 * sy, 320.0 * sx, 654.0 * sy],
        title2_text,
        "#ffffff",
        "Arial",
        "Bold",
        15.0 * s_avg,
        None,
        Some(50.0),
        "left",
    )?;

    // 11. Bottom-Left Logo & Agency Text (Position Lock)
    app.run(
        "frame.create",
        json!({
            "spread": 0,
            "rect": [36.0 * sx, 738.0 * sy, 78.0 * sx, 784.0 * sy],
            "content": "graphic",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "file.place", json!({"path": logo}));
    fx(app, "object.fit", json!({"mode": "fitProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(
        app,
        0,
        [86.0 * sx, 742.0 * sy, 360.0 * sx, 786.0 * sy],
        agency_text,
        dark,
        "Arial",
        "Bold Italic",
        10.0 * s_avg,
        Some(13.5 * s_avg),
        None,
        "left",
    )?;
    Ok(())
}

/// Procedural cover style 6 — Diagonal Arrow. The hand-drawn fallback when no template matches.
pub(super) fn cover_style_6(app: &mut DesignApp, v: CoverArtVals) -> Result<(), String> {
    let CoverArtVals {
        pw, ph, accent, dark, title_text, title2_text, year_text, volume_text, cat_text, code_text, agency_text, logo, skyline, ..
    } = v;
    // Style 6: Diagonal Arrow (from design-4.designcraft)
    let sx = pw / 595.275591;
    let sy = ph / 841.889764;
    let s_avg = (sx + sy) / 2.0;

    // 1. Base white background
    app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Full-height cityscape panel on the right (half-bleed)
    let photo_res = app.run(
        "frame.create",
        json!({
            "spread": 0,
            "rect": [286.0 * sx, 0.0, pw, ph],
            "content": "graphic",
            "shape": "rectangle"
        }),
    )?;
    let photo_id = photo_res.get("id").and_then(Value::as_u64);
    fx(app, "file.place", json!({"path": skyline, "frame": photo_id}));
    fx(app, "object.fit", json!({"mode": "fillProportionally", "ids": [photo_id]}));
    fx(app, "object.fit", json!({"mode": "centerContent", "ids": [photo_id]}));
    fx(app, "object.stroke", json!({"swatch": "[None]", "ids": [photo_id]}));

    // 3. Dark diagonal banner across the top, leaning over the photo
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [113.20 * sx, 0.0]},
            {"p": [399.74 * sx, 0.0]},
            {"p": [286.54 * sx, 256.0 * sy]},
            {"p": [0.0, 256.0 * sy]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": dark, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 4. Accent corner wedge in the top-left, off the banner's leading edge
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 0.0]},
            {"p": [86.87 * sx, 0.0]},
            {"p": [0.0, 196.46 * sy]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 5. Top-right metadata over the photograph (Position Lock: Catalogue, then ISSN/ISBN)
    add_cover_text_frame(
        app,
        0,
        [330.0 * sx, 17.0 * sy, 560.0 * sx, 35.0 * sy],
        cat_text,
        "#e2e8f0",
        "Arial",
        "Bold",
        8.5 * s_avg,
        None,
        None,
        "right",
    )?;
    add_cover_text_frame(
        app,
        0,
        [330.0 * sx, 35.0 * sy, 560.0 * sx, 53.0 * sy],
        code_text,
        "#e2e8f0",
        "Arial",
        "Bold",
        8.5 * s_avg,
        None,
        None,
        "right",
    )?;

    // 6. Forward wedge crossing from the white field into the photo
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 519.0 * sy]},
            {"p": [352.0 * sx, 591.0 * sy]},
            {"p": [0.0, 663.0 * sy]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": dark, "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 6, "size": 14, "opacity": 0.18}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 7. Year and Volume, centred inside the wedge
    add_cover_text_frame(
        app,
        0,
        [6.15 * sx, 569.0 * sy, 196.15 * sx, 639.0 * sy],
        year_text,
        "#ffffff",
        "Arial Black",
        "Regular",
        44.0 * s_avg,
        None,
        Some(20.0),
        "center",
    )?;
    add_cover_text_frame(
        app,
        0,
        [0.0, 610.0 * sy, 172.30 * sx, 632.0 * sy],
        volume_text,
        "#e2e8f0",
        "Arial",
        "Bold",
        11.0 * s_avg,
        None,
        Some(40.0),
        "center",
    )?;

    // 8. Middle-left headline: the title (accent), then the standard publication suffix
    // One full-width line: the frame is as wide as the page, so a title never hyphenates here.
    add_cover_text_frame(
        app,
        0,
        [36.0 * sx, 394.0 * sy, 559.28 * sx, 428.0 * sy],
        title_text,
        accent,
        "Arial Black",
        "Regular",
        30.0 * s_avg,
        Some(34.0 * s_avg),
        Some(10.0),
        "left",
    )?;
    add_cover_text_frame(
        app,
        0,
        [36.0 * sx, 431.0 * sy, 384.0 * sx, 465.0 * sy],
        title2_text,
        dark,
        "Arial Black",
        "Regular",
        28.0 * s_avg,
        None,
        Some(20.0),
        "left",
    )?;

    // 9. Bottom-Left Logo & Agency Text (Position Lock)
    app.run(
        "frame.create",
        json!({
            "spread": 0,
            "rect": [36.0 * sx, 742.0 * sy, 76.0 * sx, 786.0 * sy],
            "content": "graphic",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "file.place", json!({"path": logo}));
    fx(app, "object.fit", json!({"mode": "fitProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(
        app,
        0,
        [84.0 * sx, 744.0 * sy, 340.0 * sx, 786.0 * sy],
        agency_text,
        dark,
        "Arial",
        "Bold Italic",
        10.0 * s_avg,
        Some(13.5 * s_avg),
        None,
        "left",
    )?;
    Ok(())
}

/// Procedural cover style 7 — Modern Horizon. The hand-drawn fallback when no template matches.
pub(super) fn cover_style_7(app: &mut DesignApp, v: CoverArtVals) -> Result<(), String> {
    let CoverArtVals {
        pw, ph, accent, dark, title_text, title2_text, year_text, volume_text, cat_text, code_text, agency_text, logo, skyline, ..
    } = v;
    // Style 7: Modern Horizon (from design-5.designcraft)
    // A full-bleed photograph under a sweeping arc, an accent wedge that follows the arc and
    // turns into the accent block on the right, and a dark block across the lower page whose
    // rounded top-left corner carries the headline.
    let sx = pw / 595.275591;
    let sy = ph / 841.889764;
    let s_avg = (sx + sy) / 2.0;
    let x = |v: f64| v * sx;
    let y = |v: f64| v * sy;

    // 1. Base white background
    app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Hero photograph, clipped by the arc that sweeps across the upper page (the traced
    //    geometry of design-5.designcraft, which runs from the left edge into the right edge).
    let photo_res = app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, 0.0]},
            {"p": [x(595.776), 0.0]},
            {"p": [x(595.776), y(469.637)], "out": [x(284.631), y(445.387)]},
            {"p": [0.0, y(172.068)], "in": [x(40.517), y(324.164)]}
        ]}),
    )?;
    let photo_id = photo_res.get("id").and_then(Value::as_u64);
    fx(app, "file.place", json!({"path": skyline, "frame": photo_id}));
    fx(app, "object.fit", json!({"mode": "fillProportionally", "ids": [photo_id]}));
    fx(app, "object.fit", json!({"mode": "centerContent", "ids": [photo_id]}));
    fx(app, "object.stroke", json!({"swatch": "[None]", "ids": [photo_id]}));

    // 3. Dark block across the lower page, with the traced rounded top-left corner.
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, y(528.159)], "in": [0.0, y(567.917)], "out": [x(29.018), y(465.043)]},
            {"p": [x(194.405), y(420.0)], "in": [x(105.107), y(419.999)], "out": [x(215.156), y(419.999)]},
            {"p": [x(254.093), y(426.959)], "in": [x(235.193), y(422.433)], "out": [x(300.0), y(440.0)]},
            {"p": [x(372.305), y(462.0)], "in": [x(330.0), y(450.0)]},
            {"p": [x(595.276), y(462.0)]},
            {"p": [x(595.276), y(841.890)]},
            {"p": [0.0, y(841.890)]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": dark, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 4. Accent wedge following the arc, opening into the accent block on the right. One path:
    //    its lower boundary tracks the dark block's top edge, leaving a hairline of white.
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, y(175.068)], "in": [0.0, y(300.0)], "out": [x(40.517), y(327.164)]},
            {"p": [x(595.776), y(472.637)], "in": [x(284.631), y(448.387)]},
            {"p": [x(595.776), y(720.0)]},
            {"p": [x(372.305), y(720.0)]},
            {"p": [x(372.305), y(458.0)], "out": [x(330.0), y(446.0)]},
            {"p": [x(254.093), y(422.959)], "in": [x(300.0), y(436.0)], "out": [x(235.193), y(418.433)]},
            {"p": [x(194.405), y(415.999)], "in": [x(215.156), y(415.999)], "out": [x(105.107), y(415.999)]},
            {"p": [0.0, y(524.159)], "in": [x(29.018), y(461.043)], "out": [0.0, y(300.0)]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 5. Top-right metadata over the photograph (Position Lock: catalogue, then ISSN/ISBN)
    add_cover_text_frame(app, 0, [x(330.0), y(17.0), x(560.0), y(35.0)], cat_text, dark, "Arial", "Bold", 8.5 * s_avg, None, None, "right")?;
    add_cover_text_frame(app, 0, [x(330.0), y(35.0), x(560.0), y(53.0)], code_text, dark, "Arial", "Bold", 8.5 * s_avg, None, None, "right")?;

    // 6. Headline inside the dark block: the title (accent), then the standard suffix.
    //    The frames stop short of the accent block, whose left edge cuts into the dark block,
    //    so an accent-coloured title never runs out over accent-coloured paper. One word per
    //    line in the title, as on the reference cover, so a long title never hyphenates.
    let formatted_title = if title_text.contains(' ') { title_text.replace(' ', "\n") } else { title_text.to_string() };
    add_cover_text_frame(
        app,
        0,
        [x(36.0), y(578.0), x(368.0), y(686.0)],
        &formatted_title,
        accent,
        "Arial Black",
        "Regular",
        34.0 * s_avg,
        Some(39.0 * s_avg),
        Some(10.0),
        "left",
    )?;
    add_cover_text_frame(
        app,
        0,
        [x(36.0), y(690.0), x(380.0), y(730.0)],
        title2_text,
        "#ffffff",
        "Arial Black",
        "Regular",
        26.0 * s_avg,
        None,
        Some(20.0),
        "left",
    )?;

    // 7. Year with its volume directly underneath, centred in the accent block
    add_cover_text_frame(
        app,
        0,
        [x(390.0), y(540.0), x(580.0), y(620.0)],
        year_text,
        "#ffffff",
        "Arial Black",
        "Regular",
        56.0 * s_avg,
        None,
        Some(20.0),
        "center",
    )?;
    add_cover_text_frame(
        app,
        0,
        [x(390.0), y(624.0), x(580.0), y(648.0)],
        volume_text,
        "#ffffff",
        "Arial",
        "Bold",
        11.0 * s_avg,
        None,
        Some(40.0),
        "center",
    )?;

    // 8. Bottom-Left Logo & Agency Text (Position Lock)
    app.run(
        "frame.create",
        json!({
            "spread": 0,
            "rect": [x(36.0), y(752.0), x(84.0), y(804.0)],
            "content": "graphic",
            "shape": "rectangle"
        }),
    )?;
    fx(app, "file.place", json!({"path": logo}));
    fx(app, "object.fit", json!({"mode": "fitProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(
        app,
        0,
        [x(92.0), y(754.0), x(380.0), y(806.0)],
        agency_text,
        "#ffffff",
        "Arial",
        "Bold Italic",
        10.0 * s_avg,
        Some(13.5 * s_avg),
        None,
        "left",
    )?;
    Ok(())
}

/// Procedural cover style 8 — Business Report. The hand-drawn fallback when no template matches.
pub(super) fn cover_style_8(app: &mut DesignApp, v: CoverArtVals) -> Result<(), String> {
    let CoverArtVals {
        pw, ph, accent, dark, title_text, title2_text, year_text, volume_text, cat_text, code_text, agency_text, logo, skyline, ..
    } = v;
    // Style 8: Business Report — traced from design-10.designcraft (the navy/orange
    // business-report cover). White base; an orange corner tucked behind the navy plane in
    // the bottom-right; an orange band along the top; the metadata, the headline stack and
    // the agency block on the navy plane; then the arrow-shaped skyline photograph with
    // its chevron sliver across a thin white gap, and the white circle carrying the
    // two-tone year on top. Every text frame maps to a Publication Cover option.
    let sx = pw / 595.275591;
    let sy = ph / 841.889764;
    let s_avg = (sx + sy) / 2.0;
    let x = |v: f64| v * sx;
    let y = |v: f64| v * sy;

    // 1. Base white background
    app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Orange corner tucked into the bottom-right, behind the navy plane (traced item 15).
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [x(423.8), y(738.4)], "in": [x(479.5), y(770.8)], "out": [x(448.2), y(724.1)]},
            {"p": [pw, y(637.6)], "in": [x(563.4), y(656.3)], "out": [pw, y(702.0)]},
            {"p": [pw, y(838.1)], "in": [pw, y(772.9)], "out": [x(537.9), y(804.7)]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 3. Navy plane crossing in front of the photograph from the left (traced item 16).
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [0.0, y(295.6)], "in": [0.0, y(455.1)], "out": [x(71.4), y(336.0)]},
            {"p": [x(216.5), y(418.3)], "in": [x(144.6), y(377.5)], "out": [x(216.5), y(519.3)]},
            {"p": [x(216.5), y(621.2)]},
            {"p": [pw, y(835.8)], "in": [x(409.3), y(730.4)], "out": [pw, y(839.8)]},
            {"p": [pw, ph]},
            {"p": [0.0, ph]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": dark, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 4. Orange band along the top edge (traced item 17), behind the metadata corner.
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [x(247.4), 0.0], "in": [x(304.8), y(33.9)], "out": [x(344.1), 0.0]},
            {"p": [pw, 0.0]},
            {"p": [pw, y(81.1)], "in": [pw, y(55.6)], "out": [x(539.9), y(114.1)]},
            {"p": [x(490.3), y(143.4)]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 5. Top-right publication metadata over the orange band (Position Lock: catalogue,
    //    then ISSN/ISBN).
    add_cover_text_frame(app, 0, [x(320.0), y(14.0), x(580.0), y(32.0)], cat_text, "#ffffff", "Arial", "Bold", 8.5 * s_avg, None, None, "right")?;
    add_cover_text_frame(app, 0, [x(320.0), y(32.0), x(580.0), y(50.0)], code_text, "#ffffff", "Arial", "Bold", 8.5 * s_avg, None, None, "right")?;

    // 6. Headline stack top-left: the title (orange, one line), then DALAM ANGKA (navy) —
    //    the navy / orange / navy rhythm of the reference.
    add_cover_text_frame(
        app,
        0,
        [x(36.0), y(128.0), x(370.0), y(174.0)],
        title_text,
        accent,
        "Arial Black",
        "Regular",
        26.0 * s_avg,
        Some(30.0 * s_avg),
        Some(10.0),
        "left",
    )?;
    add_cover_text_frame(
        app,
        0,
        [x(36.0), y(176.0), x(360.0), y(206.0)],
        title2_text,
        dark,
        "Arial Black",
        "Regular",
        20.0 * s_avg,
        None,
        Some(26.0),
        "left",
    )?;

    // 7. Agency logo & name on the navy plane, bottom-left (Position Lock).
    app.run("frame.create", json!({"spread": 0, "rect": [x(46.0), y(772.0), x(86.0), y(812.0)], "content": "graphic", "shape": "rectangle"}))?;
    fx(app, "file.place", json!({"path": logo}));
    fx(app, "object.fit", json!({"mode": "fitProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(
        app,
        0,
        [x(94.0), y(774.0), x(310.0), y(816.0)],
        agency_text,
        "#ffffff",
        "Arial",
        "Bold Italic",
        10.0 * s_avg,
        Some(13.5 * s_avg),
        None,
        "left",
    )?;

    // 8. The photograph fills both traced subpaths of design-10's photo frame: the arrow
    //    itself, and — across a thin white gap — the chevron sliver below it. One placed
    //    image goes into both frames at the same world position, so the buildings continue
    //    across the gap.
    let arrow_id = app
        .run(
            "path.create",
            json!({"spread": 0, "closed": true, "anchors": [
                {"p": [x(202.3), y(316.7)], "in": [x(202.3), y(345.5)], "out": [x(324.4), y(246.6)]},
                {"p": [x(594.8), y(91.2)], "in": [x(469.1), y(163.4)], "out": [x(594.8), y(189.2)]},
                {"p": [x(594.8), y(627.1)], "in": [x(594.8), y(491.9)], "out": [x(483.5), y(564.0)]},
                {"p": [x(202.3), y(404.3)], "in": [x(317.5), y(469.7)], "out": [x(202.3), y(375.1)]}
            ]}),
        )?
        .get("id")
        .and_then(Value::as_u64);
    let chev_id = app
        .run(
            "path.create",
            json!({"spread": 0, "closed": true, "anchors": [
                {"p": [x(220.8), y(423.2)], "in": [x(220.8), y(487.8)], "out": [x(343.5), y(492.7)]},
                {"p": [x(590.0), y(632.4)], "in": [x(477.8), y(568.9)], "out": [x(535.4), y(664.1)]},
                {"p": [x(423.3), y(729.1)], "in": [x(479.3), y(696.6)], "out": [x(355.0), y(690.4)]},
                {"p": [x(220.8), y(614.3)], "in": [x(286.2), y(651.3)], "out": [x(220.8), y(553.3)]}
            ]}),
        )?
        .get("id")
        .and_then(Value::as_u64);
    if let (Some(arrow_id), Some(chev_id)) = (arrow_id, chev_id) {
        if let Some(img_id) = app.run("file.place", json!({"path": skyline, "x": x(130.6), "y": y(91.2)}))?.get("id").and_then(Value::as_u64) {
            // Stretch the placed image over the cover box both shapes sit on, then paste
            // copies into each frame — the world position carries over, so the two read
            // as one photograph.
            if let Some(from) = item_bounds(app, img_id) {
                fx(app, "transform.resize", json!({"from": from, "to": [x(130.6), y(91.2), x(809.7), y(742.4)], "content": true, "ids": [img_id]}));
            }
            fx(app, "edit.copy", json!({}));
            fx(app, "edit.pasteInto", json!({"id": arrow_id}));
            fx(app, "edit.pasteInto", json!({"id": chev_id}));
            fx(app, "edit.clear", json!({"ids": [img_id]}));
        }
        fx(app, "object.dropShadow", json!({"on": true, "distance": 7.0, "size": 5.0, "opacity": 0.3, "ids": [arrow_id, chev_id]}));
        fx(app, "object.stroke", json!({"swatch": "[None]", "ids": [arrow_id, chev_id]}));
    }

    // 9. White circle on the navy plane carrying the two-tone year — the medallion of the
    //    reference.
    app.run("frame.create", json!({"spread": 0, "rect": [x(55.0), y(436.0), x(245.0), y(626.0)], "content": "unassigned", "shape": "ellipse"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.dropShadow", json!({"on": true, "distance": 6.0, "size": 12.0, "opacity": 0.25}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    let year_chars: Vec<char> = year_text.chars().collect();
    let (year_first, year_second) = if year_chars.len() >= 2 {
        let mid = year_chars.len().div_ceil(2);
        (year_chars[..mid].iter().collect::<String>(), year_chars[mid..].iter().collect::<String>())
    } else {
        (year_text.to_string(), String::new())
    };
    add_cover_text_frame(
        app,
        0,
        [x(78.0), y(512.0), x(160.0), y(558.0)],
        &year_first,
        dark,
        "Arial Black",
        "Regular",
        54.0 * s_avg,
        None,
        Some(10.0),
        "left",
    )?;
    add_cover_text_frame(
        app,
        0,
        [x(140.0), y(512.0), x(222.0), y(558.0)],
        &year_second,
        accent,
        "Arial Black",
        "Regular",
        54.0 * s_avg,
        None,
        Some(10.0),
        "left",
    )?;
    add_cover_text_frame(
        app,
        0,
        [x(78.0), y(578.0), x(242.0), y(604.0)],
        volume_text,
        dark,
        "Arial",
        "Bold",
        10.0 * s_avg,
        None,
        Some(30.0),
        "left",
    )?;
    Ok(())
}

/// Procedural cover style 9 — Urban Angle. The hand-drawn fallback when no template matches.
pub(super) fn cover_style_9(app: &mut DesignApp, v: CoverArtVals) -> Result<(), String> {
    let CoverArtVals {
        pw, ph, accent, dark, title_text, title2_text, year_text, volume_text, cat_text, code_text, agency_text, logo, skyline, ..
    } = v;
    // Style 9: Urban Angle (from design-6.designcraft). Percentage-based geometry so the
    // traced shapes scale with the page: orange top-left wedge, navy diagonal band behind a
    // right-hand cityscape panel, white negative-X, navy bottom wedge.
    let sx = pw / 595.275591;
    let sy = ph / 841.889764;
    let s_avg = (sx + sy) / 2.0;
    let x = |v: f64| v * sx;
    let y = |v: f64| v * sy;

    // 1. Base white background.
    app.run("frame.create", json!({"spread": 0, "rect": [0, 0, pw, ph], "content": "unassigned", "shape": "rectangle"}))?;
    fx(app, "object.color", json!({"color": "#ffffff", "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 2. Navy bottom wedge.
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [x(521.13), y(564.5)]},
            {"p": [x(595.28), y(631.26)], "in": [x(552.24), y(592.51)], "out": [x(595.28), y(701.88)]},
            {"p": [x(595.28), y(841.89)], "in": [x(595.28), y(780.19)], "out": [x(482.55), y(841.89)]},
            {"p": [x(271.37), y(841.89)], "in": [x(364.38), y(841.89)], "out": [x(333.69), y(772.67)]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": dark, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 3. Navy diagonal band behind the cityscape.
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [x(71.61), y(75.34)]},
            {"p": [x(162.59), y(-1.0)], "in": [x(108.52), y(44.36)], "out": [x(292.29), y(-1.0)]},
            {"p": [x(595.28), y(-1.0)], "in": [x(444.42), y(-1.0)], "out": [x(595.28), y(152.97)]},
            {"p": [x(595.28), y(699.43)], "in": [x(595.28), y(525.07)], "out": [x(503.88), y(590.51)]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": dark, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 4. Cityscape photograph across the right-hand panels (two traced subpaths).
    let photo_res = app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [x(231.97), y(363.57)]},
            {"p": [x(537.04), y(0.0)], "in": [x(382.64), y(184.0)], "out": [x(556.2), y(0.0)]},
            {"p": [x(594.78), y(0.0)], "in": [x(575.46), y(0.0)], "out": [x(594.78), y(57.76)]},
            {"p": [x(594.78), y(340.84)], "in": [x(594.78), y(192.02)], "out": [x(487.89), y(468.21)]},
            {"p": [x(403.84), y(568.39)]}
        ]}),
    )?;
    let photo_id = photo_res.get("id").and_then(Value::as_u64);
    fx(app, "object.content", json!({"type": "graphic", "ids": [photo_id]}));
    fx(app, "file.place", json!({"path": skyline, "frame": photo_id}));
    fx(app, "object.fit", json!({"mode": "fillProportionally", "ids": [photo_id]}));
    fx(app, "object.fit", json!({"mode": "centerContent", "ids": [photo_id]}));
    fx(app, "object.stroke", json!({"swatch": "[None]", "ids": [photo_id]}));
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [x(344.83), y(652.62)]},
            {"p": [x(594.78), y(373.1)], "in": [x(456.73), y(527.48)], "out": [x(594.78), y(544.64)]},
            {"p": [x(594.78), y(835.54)], "in": [x(594.78), y(729.3)], "out": [x(567.52), y(866.02)]},
            {"p": [x(551.59), y(883.83)]}
        ]}),
    )?;
    fx(app, "object.content", json!({"type": "graphic"}));
    fx(app, "file.place", json!({"path": skyline}));
    fx(app, "object.fit", json!({"mode": "fillProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 5. Orange top-left wedge, drawn last so it cuts cleanly over the band.
    app.run(
        "path.create",
        json!({"spread": 0, "closed": true, "anchors": [
            {"p": [x(299.84), y(-0.5)], "in": [x(203.05), y(-0.5)], "out": [x(208.94), y(107.83)]},
            {"p": [x(0.0), y(356.84)], "in": [x(64.17), y(280.36)], "out": [x(0.0), y(243.84)]},
            {"p": [x(0.0), y(-0.5)], "in": [x(0.0), y(96.13)], "out": [x(99.02), y(-0.5)]}
        ]}),
    )?;
    fx(app, "object.color", json!({"color": accent, "target": "fill"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    // 6. Rotated year in the navy band, split into two tones like the reference.
    let (y1, y2) = if year_text.len() >= 4 { (&year_text[..2], &year_text[2..]) } else { (year_text, "") };
    let year1 = add_cover_text_frame(
        app,
        0,
        [x(150.0), y(73.0), x(232.0), y(153.0)],
        y1,
        "#ffffff",
        "Arial Black",
        "Regular",
        44.0 * s_avg,
        None,
        Some(10.0),
        "center",
    )?;
    let year2 = add_cover_text_frame(
        app,
        0,
        [x(205.0), y(61.0), x(290.0), y(141.0)],
        y2,
        accent,
        "Arial Black",
        "Regular",
        44.0 * s_avg,
        None,
        Some(10.0),
        "center",
    )?;
    fx(app, "transform.rotate", json!({"ids": [year1, year2], "angle": -50.0}));

    // 7. Volume caption along the band, under the year.
    let vol_id = add_cover_text_frame(
        app,
        0,
        [x(252.0), y(151.0), x(356.0), y(185.0)],
        volume_text,
        "#ffffff",
        "Arial",
        "Bold",
        8.0 * s_avg,
        None,
        Some(20.0),
        "left",
    )?;
    fx(app, "transform.rotate", json!({"ids": [vol_id], "angle": -50.0}));

    // 8. Headline stack on the white left panel (title in accent, then DALAM ANGKA).
    add_cover_text_frame(
        app,
        0,
        [x(36.0), y(650.0), x(340.0), y(724.0)],
        &title_text.replace(' ', "\n"),
        accent,
        "Arial Black",
        "Regular",
        30.0 * s_avg,
        Some(34.0 * s_avg),
        Some(8.0),
        "left",
    )?;
    add_cover_text_frame(
        app,
        0,
        [x(36.0), y(728.0), x(330.0), y(776.0)],
        title2_text,
        dark,
        "Arial Black",
        "Regular",
        22.0 * s_avg,
        None,
        Some(15.0),
        "left",
    )?;

    // 9. Top-left agency logo & text over the orange wedge (Position Lock).
    app.run("frame.create", json!({"spread": 0, "rect": [x(36.0), y(24.0), x(74.0), y(64.0)], "content": "graphic", "shape": "rectangle"}))?;
    fx(app, "file.place", json!({"path": logo}));
    fx(app, "object.fit", json!({"mode": "fitProportionally"}));
    fx(app, "object.fit", json!({"mode": "centerContent"}));
    fx(app, "object.stroke", json!({"swatch": "[None]"}));

    add_cover_text_frame(
        app,
        0,
        [x(82.0), y(26.0), x(300.0), y(68.0)],
        agency_text,
        dark,
        "Arial",
        "Bold Italic",
        10.0 * s_avg,
        Some(13.5 * s_avg),
        None,
        "left",
    )?;

    // 10. Top-right publication metadata over the photograph (Position Lock).
    add_cover_text_frame(app, 0, [x(330.0), y(17.0), x(560.0), y(35.0)], cat_text, "#ffffff", "Arial", "Bold", 8.5 * s_avg, None, None, "right")?;
    add_cover_text_frame(app, 0, [x(330.0), y(35.0), x(560.0), y(53.0)], code_text, "#ffffff", "Arial", "Bold", 8.5 * s_avg, None, None, "right")?;
    Ok(())
}
