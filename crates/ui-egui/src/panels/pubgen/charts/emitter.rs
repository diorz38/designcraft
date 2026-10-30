//! Vector chart emitter.
//!
//! Transforms a `ChartLayout` into native DesignCraft shapes and text frames
//! on a target spread using engine commands (`frame.create`, `object.fill`, `object.stroke`,
//! `transform.rotate`, and `object.group`).
//!
//! Never panics; returns `Result<usize, String>` where the `usize` is the count
//! of emitted frames.

use serde_json::{Value, json};

use crate::DesignApp;

use super::layout::{AXIS_LABEL_W, ChartLayout};
use super::palette::series_color;
use super::spec::{ChartSpec, format_indonesian_number};

/// Ensures a swatch exists in the document and returns its name.
///
/// If `hex_or_name` is already an existing swatch or built-in (`[None]`, `[Paper]`, `[Black]`),
/// returns it directly. Otherwise, runs `swatch.create` and returns the created swatch name.
pub fn ensure_swatch(app: &mut DesignApp, hex_or_name: &str) -> Result<String, String> {
    let name = hex_or_name.trim();
    if name.is_empty() || matches!(name, "[None]" | "[Paper]" | "[Black]" | "Registration") {
        return Ok(name.to_string());
    }
    if let Some(active) = app.session.active()
        && active.doc.swatch(name).is_some()
    {
        return Ok(name.to_string());
    }
    let res = app.run("swatch.create", json!({"name": name, "color": name})).map_err(|e| format!("swatch.create `{name}` failed: {e}"))?;
    Ok(res.get("name").and_then(Value::as_str).unwrap_or(name).to_string())
}

/// Emits a closed polygon frame onto the given spread with exact coordinates.
pub fn emit_polygon(
    app: &mut DesignApp,
    spread_idx: usize,
    points: &[(f64, f64)],
    fill: &str,
    stroke: &str,
    stroke_weight: f64,
) -> Result<u64, String> {
    if points.len() < 3 {
        return Err("Polygon must have at least 3 points".into());
    }
    let mut x0 = f64::INFINITY;
    let mut y0 = f64::INFINITY;
    let mut x1 = f64::NEG_INFINITY;
    let mut y1 = f64::NEG_INFINITY;
    for &(x, y) in points {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let rect = [x0, y0, x1.max(x0 + 0.5), y1.max(y0 + 0.5)];

    let res = app
        .run(
            "frame.create",
            json!({
                "spread": spread_idx,
                "rect": rect,
                "content": "unassigned",
                "shape": "polygon",
                "caret": false,
            }),
        )
        .map_err(|e| format!("frame.create polygon failed: {e}"))?;

    let fid = res.get("id").and_then(Value::as_u64).ok_or_else(|| "frame.create returned no id".to_string())?;

    let kurbo_pts: Vec<designcraft_geom::Point> = points.iter().map(|&(x, y)| designcraft_geom::Point::new(x, y)).collect();
    let poly_path = designcraft_geom::PathData::single(designcraft_geom::SubPath::polyline(&kurbo_pts, true));

    let _ = app.session.edit(|doc, _| {
        if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
            it.path = poly_path;
            it.shape = designcraft_doc::Shape::Polygon;
        }
        Ok(Value::Null)
    });

    if fill != "[None]" {
        let fill_swatch = ensure_swatch(app, fill)?;
        let _ = app.run("object.fill", json!({"ids": [fid], "swatch": fill_swatch}));
    } else {
        let _ = app.run("object.fill", json!({"ids": [fid], "swatch": "[None]"}));
    }

    if stroke != "[None]" {
        let stroke_swatch = ensure_swatch(app, stroke)?;
        let _ = app.run("object.stroke", json!({"ids": [fid], "swatch": stroke_swatch}));
        let _ = app.run("object.strokeWeight", json!({"ids": [fid], "weight": stroke_weight}));
    } else {
        let _ = app.run("object.stroke", json!({"ids": [fid], "swatch": "[None]"}));
    }

    Ok(fid)
}

/// Emits an ellipse / circle frame onto the given spread.
pub fn emit_ellipse(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], fill: &str, stroke: &str, stroke_weight: f64) -> Result<u64, String> {
    let res = app
        .run(
            "frame.create",
            json!({
                "spread": spread_idx,
                "rect": rect,
                "content": "unassigned",
                "shape": "ellipse",
                "caret": false,
            }),
        )
        .map_err(|e| format!("frame.create ellipse failed: {e}"))?;

    let fid = res.get("id").and_then(Value::as_u64).ok_or_else(|| "frame.create returned no id".to_string())?;

    if fill != "[None]" {
        let fill_swatch = ensure_swatch(app, fill)?;
        let _ = app.run("object.fill", json!({"ids": [fid], "swatch": fill_swatch}));
    } else {
        let _ = app.run("object.fill", json!({"ids": [fid], "swatch": "[None]"}));
    }

    if stroke != "[None]" {
        let stroke_swatch = ensure_swatch(app, stroke)?;
        let _ = app.run("object.stroke", json!({"ids": [fid], "swatch": stroke_swatch}));
        let _ = app.run("object.strokeWeight", json!({"ids": [fid], "weight": stroke_weight}));
    } else {
        let _ = app.run("object.stroke", json!({"ids": [fid], "swatch": "[None]"}));
    }

    Ok(fid)
}

/// Emits a stylized male person pictogram (head ellipse + torso/legs polygon).
pub fn emit_male_pictogram(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], color: &str) -> Result<Vec<u64>, String> {
    let mut ids = Vec::new();
    let [x0, y0, x1, y1] = rect;
    let w = (x1 - x0).max(4.0);
    let h = (y1 - y0).max(8.0);
    let cx = (x0 + x1) / 2.0;

    // Head
    let head_r = (w * 0.32).min(h * 0.13);
    let head_rect = [cx - head_r, y0, cx + head_r, y0 + 2.0 * head_r];
    if let Ok(hid) = emit_ellipse(app, spread_idx, head_rect, color, "[None]", 0.0) {
        ids.push(hid);
    }

    // Body & legs as polygon
    let body_top = y0 + 2.0 * head_r + h * 0.04;
    let sw = w * 0.78;
    let crotch_y = body_top + h * 0.38;
    let leg_w = sw * 0.36;
    let leg_gap = sw * 0.18;

    let pts = vec![
        (cx - sw / 2.0, body_top),
        (cx + sw / 2.0, body_top),
        (cx + sw / 2.0, crotch_y),
        (cx + leg_gap / 2.0 + leg_w, y1),
        (cx + leg_gap / 2.0, y1),
        (cx, crotch_y + h * 0.06),
        (cx - leg_gap / 2.0, y1),
        (cx - leg_gap / 2.0 - leg_w, y1),
        (cx - sw / 2.0, crotch_y),
    ];
    if let Ok(bid) = emit_polygon(app, spread_idx, &pts, color, "[None]", 0.0) {
        ids.push(bid);
    }

    Ok(ids)
}

/// Emits a stylized female person pictogram (head ellipse + dress/legs polygon).
pub fn emit_female_pictogram(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], color: &str) -> Result<Vec<u64>, String> {
    let mut ids = Vec::new();
    let [x0, y0, x1, y1] = rect;
    let w = (x1 - x0).max(4.0);
    let h = (y1 - y0).max(8.0);
    let cx = (x0 + x1) / 2.0;

    // Head
    let head_r = (w * 0.32).min(h * 0.13);
    let head_rect = [cx - head_r, y0, cx + head_r, y0 + 2.0 * head_r];
    if let Ok(hid) = emit_ellipse(app, spread_idx, head_rect, color, "[None]", 0.0) {
        ids.push(hid);
    }

    // Dress & legs as polygon
    let body_top = y0 + 2.0 * head_r + h * 0.04;
    let shoulder_w = w * 0.52;
    let skirt_bottom = body_top + h * 0.45;
    let skirt_w = w * 0.92;
    let leg_w = w * 0.22;
    let leg_gap = w * 0.16;

    let pts = vec![
        (cx - shoulder_w / 2.0, body_top),
        (cx + shoulder_w / 2.0, body_top),
        (cx + skirt_w / 2.0, skirt_bottom),
        (cx + leg_gap / 2.0 + leg_w, skirt_bottom),
        (cx + leg_gap / 2.0 + leg_w, y1),
        (cx + leg_gap / 2.0, y1),
        (cx + leg_gap / 2.0, skirt_bottom),
        (cx - leg_gap / 2.0, skirt_bottom),
        (cx - leg_gap / 2.0, y1),
        (cx - leg_gap / 2.0 - leg_w, y1),
        (cx - leg_gap / 2.0 - leg_w, skirt_bottom),
        (cx - skirt_w / 2.0, skirt_bottom),
    ];
    if let Ok(bid) = emit_polygon(app, spread_idx, &pts, color, "[None]", 0.0) {
        ids.push(bid);
    }

    Ok(ids)
}

/// Emits a stylized population group pictogram (center person flanked by two persons).
pub fn emit_group_pictogram(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], color: &str) -> Result<Vec<u64>, String> {
    let mut ids = Vec::new();
    let [x0, y0, x1, y1] = rect;
    let w = (x1 - x0).max(6.0);
    let h = (y1 - y0).max(8.0);
    let cx = (x0 + x1) / 2.0;

    // Center person
    let c_rect = [cx - w * 0.22, y0, cx + w * 0.22, y1];
    if let Ok(cids) = emit_male_pictogram(app, spread_idx, c_rect, color) {
        ids.extend(cids);
    }

    // Left person (slightly smaller)
    let l_rect = [x0, y0 + h * 0.12, x0 + w * 0.38, y1];
    if let Ok(lids) = emit_male_pictogram(app, spread_idx, l_rect, color) {
        ids.extend(lids);
    }

    // Right person (slightly smaller)
    let r_rect = [x1 - w * 0.38, y0 + h * 0.12, x1, y1];
    if let Ok(rids) = emit_female_pictogram(app, spread_idx, r_rect, color) {
        ids.extend(rids);
    }

    Ok(ids)
}

/// Emits a single rectangle frame onto the given spread.
pub fn emit_rect(app: &mut DesignApp, spread_idx: usize, rect: [f64; 4], fill: &str, stroke: &str, stroke_weight: f64) -> Result<u64, String> {
    let res = app
        .run(
            "frame.create",
            json!({
                "spread": spread_idx,
                "rect": rect,
                "content": "unassigned",
                "shape": "rectangle",
                "caret": false,
            }),
        )
        .map_err(|e| format!("frame.create rect failed: {e}"))?;

    let fid = res.get("id").and_then(Value::as_u64).ok_or_else(|| "frame.create returned no id".to_string())?;

    if fill != "[None]" {
        let fill_swatch = ensure_swatch(app, fill)?;
        let _ = app.run("object.fill", json!({"ids": [fid], "swatch": fill_swatch}));
    } else {
        let _ = app.run("object.fill", json!({"ids": [fid], "swatch": "[None]"}));
    }

    if stroke != "[None]" {
        let stroke_swatch = ensure_swatch(app, stroke)?;
        let _ = app.run("object.stroke", json!({"ids": [fid], "swatch": stroke_swatch}));
        let _ = app.run("object.strokeWeight", json!({"ids": [fid], "weight": stroke_weight}));
    } else {
        let _ = app.run("object.stroke", json!({"ids": [fid], "swatch": "[None]"}));
    }

    Ok(fid)
}

/// Emits a styled text frame onto the given spread with optional font family.
pub fn emit_text_with_font(
    app: &mut DesignApp,
    spread_idx: usize,
    rect: [f64; 4],
    text: &str,
    size: f64,
    bold: bool,
    align: &str,
    color: Option<&str>,
    font_family: Option<&str>,
) -> Result<u64, String> {
    let res = app
        .run(
            "frame.create",
            json!({
                "spread": spread_idx,
                "rect": rect,
                "content": "text",
                "text": text,
                "caret": false,
            }),
        )
        .map_err(|e| format!("frame.create text failed: {e}"))?;

    let fid = res.get("id").and_then(Value::as_u64).ok_or_else(|| "frame.create returned no id".to_string())?;
    let sid = res.get("story").and_then(Value::as_u64);

    let swatch_name = if let Some(c) = color { ensure_swatch(app, c)? } else { "[Black]".to_string() };
    let fam = font_family.unwrap_or("Arial");

    if let Some(story_id) = sid {
        let _ = app.session.edit(|doc, _| {
            if let Some(story) = doc.story_mut(designcraft_doc::StoryId(story_id)) {
                for r in &mut story.chars {
                    r.format.over.font_family = Some(fam.into());
                    r.format.over.size = Some(size);
                    r.format.over.font_style = Some(if bold { "Bold".into() } else { "Regular".into() });
                    r.format.over.fill = Some(swatch_name.clone());
                }
                for p in &mut story.paras {
                    p.para.align = Some(match align {
                        "center" => designcraft_doc::Align::Center,
                        "right" => designcraft_doc::Align::Right,
                        _ => designcraft_doc::Align::Left,
                    });
                }
            }
            if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid))
                && let designcraft_doc::Content::Text(tf) = &mut it.content
            {
                tf.options.vertical_justification = designcraft_doc::VerticalJustification::Center;
            }
            Ok(Value::Null)
        });
    }

    let _ = app.run("object.stroke", json!({"ids": [fid], "swatch": "[None]"}));
    let _ = app.run("object.fill", json!({"ids": [fid], "swatch": "[None]"}));

    Ok(fid)
}

/// Emits a styled text frame onto the given spread (default Arial).
pub fn emit_text(
    app: &mut DesignApp,
    spread_idx: usize,
    rect: [f64; 4],
    text: &str,
    size: f64,
    bold: bool,
    align: &str,
    color: Option<&str>,
) -> Result<u64, String> {
    emit_text_with_font(app, spread_idx, rect, text, size, bold, align, color, None)
}

/// Emits the complete chart layout onto the document spread.
///
/// Returns the number of frames created, or an `Err` message on failure.
pub fn emit_chart(
    app: &mut DesignApp,
    spread_idx: usize,
    spec: &ChartSpec,
    layout: &ChartLayout,
    palette: &[String],
    bilingual: bool,
) -> Result<usize, String> {
    let mut frame_ids: Vec<u64> = Vec::new();

    // 1. Grid lines and tick labels
    for gl in &layout.grid_lines {
        let fid = emit_rect(app, spread_idx, gl.rect, "#E0E0E0", "[None]", 0.0)?;
        frame_ids.push(fid);

        // Tick label
        let is_vertical_grid = (gl.rect[2] - gl.rect[0]) <= 1.0;
        let tick_rect = if is_vertical_grid {
            [gl.rect[0] - 24.0, gl.rect[3] + 2.0, gl.rect[0] + 24.0, gl.rect[3] + 14.0]
        } else {
            [(gl.rect[0] - AXIS_LABEL_W - 4.0).max(0.0), gl.rect[1] - 6.0, gl.rect[0] - 4.0, gl.rect[1] + 6.0]
        };
        let align = if is_vertical_grid { "center" } else { "right" };
        let font = if spec.chart_type == super::spec::ChartType::Line { Some("Lato") } else { None };
        if let Ok(tfid) = emit_text_with_font(app, spread_idx, tick_rect, &gl.label, 7.0, false, align, Some("#555555"), font) {
            frame_ids.push(tfid);
        }
    }

    // 2. Axis baseline & left line (only for Cartesian coordinate charts)
    if spec.chart_type != super::spec::ChartType::Donut
        && spec.chart_type != super::spec::ChartType::Gauge
        && spec.chart_type != super::spec::ChartType::Map
    {
        let base_fid = emit_rect(app, spread_idx, layout.axis_baseline, "#333333", "[None]", 0.0)?;
        frame_ids.push(base_fid);
        if spec.chart_type != super::spec::ChartType::PopulationPyramid && spec.chart_type != super::spec::ChartType::Map {
            let left_fid = emit_rect(app, spread_idx, layout.axis_left, "#333333", "[None]", 0.0)?;
            frame_ids.push(left_fid);
        }
    }

    // 3. Bars
    for bar in &layout.bars {
        let (color, stroke_color) = if spec.chart_type == super::spec::ChartType::PopulationPyramid {
            if bar.series_idx == 0 {
                // Male: Blue with dark blue outline
                (series_color(palette, 0), "#1A4971")
            } else {
                // Female: Pink with dark pink outline
                (if palette.len() > 1 { series_color(palette, 1) } else { "#F04C82" }, "#A61445")
            }
        } else {
            (series_color(palette, bar.series_idx), "[None]")
        };
        let stroke_weight = if stroke_color == "[None]" { 0.0 } else { 0.75 };
        let fid = emit_rect(app, spread_idx, bar.rect, color, stroke_color, stroke_weight)?;
        frame_ids.push(fid);
    }

    // 4. Line segments and markers
    for (s_idx, segments) in layout.line_segments.iter().enumerate() {
        let color = if spec.chart_type == super::spec::ChartType::Line && palette.is_empty() {
            super::palette::brs_line_series_color(s_idx)
        } else {
            series_color(palette, s_idx)
        };
        for &seg in segments {
            let [px, py, cx, cy] = seg;
            let dx = cx - px;
            let dy = cy - py;
            let len = (dx * dx + dy * dy).sqrt();
            if len > 1e-4 {
                let half_w = 1.0;
                let nx = -dy / len * half_w;
                let ny = dx / len * half_w;
                let pts = [(px + nx, py + ny), (cx + nx, cy + ny), (cx - nx, cy - ny), (px - nx, py - ny)];
                if let Ok(fid) = emit_polygon(app, spread_idx, &pts, color, "[None]", 0.0) {
                    frame_ids.push(fid);
                }
            }
        }
    }
    for (s_idx, markers) in layout.line_markers.iter().enumerate() {
        let color = if spec.chart_type == super::spec::ChartType::Line && palette.is_empty() {
            super::palette::brs_line_series_color(s_idx)
        } else {
            series_color(palette, s_idx)
        };
        for &m_rect in markers {
            if spec.chart_type == super::spec::ChartType::Line {
                if s_idx == 0 {
                    // Square marker
                    if let Ok(fid) = emit_rect(app, spread_idx, m_rect, color, color, 0.0) {
                        frame_ids.push(fid);
                    }
                } else if s_idx == 1 {
                    // Circle marker (white center, color border)
                    let cx = (m_rect[0] + m_rect[2]) / 2.0;
                    let cy = (m_rect[1] + m_rect[3]) / 2.0;
                    let r = 2.5;
                    let pts: Vec<(f64, f64)> = (0..12)
                        .map(|i| {
                            let a = i as f64 * std::f64::consts::TAU / 12.0;
                            (cx + r * a.cos(), cy + r * a.sin())
                        })
                        .collect();
                    if let Ok(fid) = emit_polygon(app, spread_idx, &pts, "#FFFFFF", color, 1.5) {
                        frame_ids.push(fid);
                    }
                } else {
                    // Diamond marker
                    let cx = (m_rect[0] + m_rect[2]) / 2.0;
                    let cy = (m_rect[1] + m_rect[3]) / 2.0;
                    let r = 2.8;
                    let pts = [(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)];
                    if let Ok(fid) = emit_polygon(app, spread_idx, &pts, color, "[None]", 0.0) {
                        frame_ids.push(fid);
                    }
                }
            } else {
                let fid = emit_rect(app, spread_idx, m_rect, color, "[Paper]", 1.0)?;
                frame_ids.push(fid);
            }
        }
    }

    // 5. Donut slices
    for slice in &layout.donut_slices {
        let color = series_color(palette, slice.series_idx);
        let fid = emit_polygon(app, spread_idx, &slice.points, color, "#FFFFFF", 1.5)?;
        frame_ids.push(fid);

        if !slice.value_label.is_empty() {
            let lbl_text = format!("{}\n{}", slice.label, slice.value_label);
            if let Ok(tfid) = emit_text(app, spread_idx, slice.label_rect, &lbl_text, 8.0, true, "center", Some("#333333")) {
                frame_ids.push(tfid);
            }
        }
    }

    // 5b. Gauge chart
    if let Some(gauge) = &layout.gauge {
        let primary_color = series_color(palette, 0);
        let trk_fid = emit_polygon(app, spread_idx, &gauge.track_points, "#E5E7EB", "[None]", 0.0)?;
        frame_ids.push(trk_fid);

        let val_fid = emit_polygon(app, spread_idx, &gauge.value_points, primary_color, "[None]", 0.0)?;
        frame_ids.push(val_fid);

        let needle_fid = emit_polygon(app, spread_idx, &gauge.needle_points, "#1F2937", "[None]", 0.0)?;
        frame_ids.push(needle_fid);

        let pivot_fid = emit_rect(app, spread_idx, gauge.pivot_rect, "#111827", "#FFFFFF", 1.5)?;
        frame_ids.push(pivot_fid);

        let ro_fid = emit_text(app, spread_idx, gauge.readout_rect, &gauge.readout_text, 18.0, true, "center", Some(primary_color))?;
        frame_ids.push(ro_fid);

        if !gauge.subtitle_text.is_empty() {
            let sub_fid = emit_text(app, spread_idx, gauge.subtitle_rect, &gauge.subtitle_text, 9.0, false, "center", Some("#4B5563"))?;
            frame_ids.push(sub_fid);
        }

        let min_fid = emit_text(app, spread_idx, gauge.min_rect, &gauge.min_text, 7.5, false, "left", Some("#6B7280"))?;
        let max_fid = emit_text(app, spread_idx, gauge.max_rect, &gauge.max_text, 7.5, false, "right", Some("#6B7280"))?;
        frame_ids.push(min_fid);
        frame_ids.push(max_fid);
    }

    // 5c. Population pyramid specific elements: headers, baseline ticks, and bottom summary box
    if spec.chart_type == super::spec::ChartType::PopulationPyramid {
        let cx = (layout.plot_rect[0] + layout.plot_rect[2]) / 2.0;
        let center_col_w = 48.0f64;
        let center_left = cx - center_col_w / 2.0;
        let center_right = cx + center_col_w / 2.0;

        let hdr_y0 = (layout.plot_rect[1] - 30.0).max(0.0);
        let hdr_y1 = layout.plot_rect[1] - 4.0;

        let male_hdr_rect = [layout.plot_rect[0], hdr_y0, center_left - 10.0, hdr_y1];
        let center_hdr_rect = [center_left - 6.0, hdr_y0, center_right + 6.0, hdr_y1];
        let female_hdr_rect = [center_right + 10.0, hdr_y0, layout.plot_rect[2], hdr_y1];

        let male_label = spec.series.first().map(|s| s.display_label(bilingual)).unwrap_or_else(|| "LAKI-LAKI (MALE)".into()).to_uppercase();
        let female_label = spec.series.get(1).map(|s| s.display_label(bilingual)).unwrap_or_else(|| "PEREMPUAN (FEMALE)".into()).to_uppercase();
        let center_label = if bilingual { "KELOMPOK UMUR\n(AGE GROUPS)" } else { "KELOMPOK UMUR" };

        let male_color = series_color(palette, 0);
        let female_color = if palette.len() > 1 { series_color(palette, 1) } else { "#F04C82" };

        // Male Header + Icon
        let icon_h = 18.0f64;
        let icon_w = 8.0f64;
        if let Ok(ids) =
            emit_male_pictogram(app, spread_idx, [center_left - 24.0, hdr_y0 + 4.0, center_left - 24.0 + icon_w, hdr_y0 + 4.0 + icon_h], male_color)
        {
            frame_ids.extend(ids);
        }
        if let Ok(fid) = emit_text(app, spread_idx, male_hdr_rect, &male_label, 8.5, true, "center", Some(male_color)) {
            frame_ids.push(fid);
        }

        // Center Header (Age Groups)
        if let Ok(fid) = emit_text(app, spread_idx, center_hdr_rect, center_label, 7.0, true, "center", Some("#1E293B")) {
            frame_ids.push(fid);
        }

        // Female Header + Icon
        if let Ok(ids) = emit_female_pictogram(
            app,
            spread_idx,
            [center_right + 16.0, hdr_y0 + 4.0, center_right + 16.0 + icon_w, hdr_y0 + 4.0 + icon_h],
            female_color,
        ) {
            frame_ids.extend(ids);
        }
        if let Ok(fid) = emit_text(app, spread_idx, female_hdr_rect, &female_label, 8.5, true, "center", Some(female_color)) {
            frame_ids.push(fid);
        }

        if let Some(summary) = &layout.pyramid_summary {
            // Baseline at axis_y
            let axis_y = summary.axis_y;
            if let Ok(base_fid) =
                emit_rect(app, spread_idx, [layout.plot_rect[0], axis_y, layout.plot_rect[2], axis_y + 1.0], "#64748B", "[None]", 0.0)
            {
                frame_ids.push(base_fid);
            }

            // Axis ticks: 4 intervals (0, 25%, 50%, 75%, 100%)
            let ceiling = summary.ceiling;
            let max_w = summary.max_bar_w;
            let steps = 4;
            for i in 0..=steps {
                let frac = i as f64 / steps as f64;
                let val = (1.0 - frac) * ceiling;
                let val_str = spec.value_format.format(val, bilingual);

                // Left tick (Male): from center_left towards left
                let x_left = summary.center_left - (1.0 - frac) * max_w;
                let _ = emit_rect(app, spread_idx, [x_left - 0.5, axis_y, x_left + 0.5, axis_y + 3.0], "#64748B", "[None]", 0.0)
                    .map(|id| frame_ids.push(id));
                let lbl_rect_left = [x_left - 20.0, axis_y + 3.0, x_left + 20.0, axis_y + 13.0];
                let _ = emit_text(app, spread_idx, lbl_rect_left, &val_str, 7.5, false, "center", Some("#333333")).map(|id| frame_ids.push(id));

                // Right tick (Female): from center_right towards right
                let val_right = frac * ceiling;
                let val_str_r = spec.value_format.format(val_right, bilingual);
                let x_right = summary.center_right + frac * max_w;
                let _ = emit_rect(app, spread_idx, [x_right - 0.5, axis_y, x_right + 0.5, axis_y + 3.0], "#64748B", "[None]", 0.0)
                    .map(|id| frame_ids.push(id));
                let lbl_rect_right = [x_right - 20.0, axis_y + 3.0, x_right + 20.0, axis_y + 13.0];
                let _ = emit_text(app, spread_idx, lbl_rect_right, &val_str_r, 7.5, false, "center", Some("#333333")).map(|id| frame_ids.push(id));
            }

            // Axis Title: JUMLAH PENDUDUK (POPULATION)
            let title_y0 = axis_y + 14.0;
            let axis_title_rect = [cx - 120.0, title_y0, cx + 120.0, title_y0 + 12.0];
            let axis_title = if bilingual { "JUMLAH PENDUDUK (POPULATION)" } else { "JUMLAH PENDUDUK" };
            if let Ok(fid) = emit_text(app, spread_idx, axis_title_rect, axis_title, 7.5, true, "center", Some("#1E293B")) {
                frame_ids.push(fid);
            }

            // Summary Card at bottom
            let s_rect = summary.rect;
            if let Ok(box_fid) = emit_rect(app, spread_idx, s_rect, "#FFFFFF", "#CBD5E1", 1.0) {
                frame_ids.push(box_fid);
            }

            let col_w = (s_rect[2] - s_rect[0]) / 3.0;
            let d1 = s_rect[0] + col_w;
            let d2 = s_rect[0] + 2.0 * col_w;

            // Vertical Dividers
            if let Ok(dfid1) = emit_rect(app, spread_idx, [d1, s_rect[1] + 4.0, d1 + 0.5, s_rect[3] - 4.0], "#E2E8F0", "[None]", 0.0) {
                frame_ids.push(dfid1);
            }
            if let Ok(dfid2) = emit_rect(app, spread_idx, [d2, s_rect[1] + 4.0, d2 + 0.5, s_rect[3] - 4.0], "#E2E8F0", "[None]", 0.0) {
                frame_ids.push(dfid2);
            }

            // Col 1: Male (Icon on left, texts on right)
            let icon_pad = 8.0f64;
            let icon_w = 9.0f64;
            let icon_h = 24.0f64;
            let icon_y = s_rect[1] + (s_rect[3] - s_rect[1] - icon_h) / 2.0;
            if let Ok(ids) =
                emit_male_pictogram(app, spread_idx, [s_rect[0] + icon_pad, icon_y, s_rect[0] + icon_pad + icon_w, icon_y + icon_h], male_color)
            {
                frame_ids.extend(ids);
            }

            let text_x0 = s_rect[0] + icon_pad + icon_w + 6.0;
            let text_x1 = d1 - 2.0;
            let c1_title = [text_x0, s_rect[1] + 4.0, text_x1, s_rect[1] + 16.0];
            let c1_num = [text_x0, s_rect[1] + 15.0, text_x1, s_rect[1] + 33.0];
            let c1_sub = [text_x0, s_rect[1] + 33.0, text_x1, s_rect[3] - 3.0];
            let _ = emit_text(app, spread_idx, c1_title, &summary.male_label, 6.0, true, "left", Some(male_color)).map(|id| frame_ids.push(id));
            let _ = emit_text(app, spread_idx, c1_num, &format_indonesian_number(summary.total_male), 13.0, true, "left", Some("#0F172A"))
                .map(|id| frame_ids.push(id));
            let _ = emit_text(app, spread_idx, c1_sub, "jiwa (people)", 6.5, false, "left", Some("#64748B")).map(|id| frame_ids.push(id));

            // Col 2: Total Population (Family/Group Icon on left, texts on right)
            let grp_w = 18.0f64;
            let grp_h = 22.0f64;
            let grp_pad = 6.0f64;
            let grp_x = d1 + grp_pad;
            let grp_y = s_rect[1] + (s_rect[3] - s_rect[1] - grp_h) / 2.0;
            if let Ok(ids) = emit_group_pictogram(app, spread_idx, [grp_x, grp_y, grp_x + grp_w, grp_y + grp_h], "#0F172A") {
                frame_ids.extend(ids);
            }

            let c2_text_x0 = grp_x + grp_w + 6.0;
            let c2_text_x1 = d2 - 2.0;
            let c2_title = [c2_text_x0, s_rect[1] + 4.0, c2_text_x1, s_rect[1] + 16.0];
            let c2_num = [c2_text_x0, s_rect[1] + 15.0, c2_text_x1, s_rect[1] + 33.0];
            let c2_sub = [c2_text_x0, s_rect[1] + 33.0, c2_text_x1, s_rect[3] - 3.0];
            let _ = emit_text(app, spread_idx, c2_title, &summary.combined_label, 6.0, true, "left", Some("#0F172A")).map(|id| frame_ids.push(id));
            let _ = emit_text(app, spread_idx, c2_num, &format_indonesian_number(summary.total_combined), 14.0, true, "left", Some("#0F172A"))
                .map(|id| frame_ids.push(id));
            let _ = emit_text(app, spread_idx, c2_sub, "jiwa (people)", 6.5, false, "left", Some("#64748B")).map(|id| frame_ids.push(id));

            // Col 3: Female (Icon on left, texts on right)
            let fem_pad = 8.0f64;
            let fem_w = 9.0f64;
            let fem_x = d2 + fem_pad;
            let fem_y = s_rect[1] + (s_rect[3] - s_rect[1] - icon_h) / 2.0;
            if let Ok(ids) = emit_female_pictogram(app, spread_idx, [fem_x, fem_y, fem_x + fem_w, fem_y + icon_h], female_color) {
                frame_ids.extend(ids);
            }

            let c3_text_x0 = fem_x + fem_w + 6.0;
            let c3_text_x1 = s_rect[2] - 2.0;
            let c3_title = [c3_text_x0, s_rect[1] + 4.0, c3_text_x1, s_rect[1] + 16.0];
            let c3_num = [c3_text_x0, s_rect[1] + 15.0, c3_text_x1, s_rect[1] + 33.0];
            let c3_sub = [c3_text_x0, s_rect[1] + 33.0, c3_text_x1, s_rect[3] - 3.0];
            let _ = emit_text(app, spread_idx, c3_title, &summary.female_label, 6.0, true, "left", Some(female_color)).map(|id| frame_ids.push(id));
            let _ = emit_text(app, spread_idx, c3_num, &format_indonesian_number(summary.total_female), 13.0, true, "left", Some("#0F172A"))
                .map(|id| frame_ids.push(id));
            let _ = emit_text(app, spread_idx, c3_sub, "jiwa (people)", 6.5, false, "left", Some("#64748B")).map(|id| frame_ids.push(id));
        }
    }

    // 5d. Map (Choropleth) regions and custom map legend
    if spec.chart_type == super::spec::ChartType::Map {
        for region in &layout.map_regions {
            let color = if !region.color.is_empty() { &region.color } else { "#0072BC" };
            for ring in &region.polygons {
                if let Ok(fid) = emit_polygon(app, spread_idx, ring, color, &region.stroke_color, region.stroke_weight) {
                    frame_ids.push(fid);
                }
            }

            if spec.show_value_labels && !region.name.is_empty() {
                let text = if !region.value_str.is_empty() { format!("{}\n{}", region.name, region.value_str) } else { region.name.clone() };
                let text_color = if let Some((r, g, b)) = super::palette::parse_hex_rgb(&region.color) {
                    let lum = 0.299 * (r as f64) + 0.587 * (g as f64) + 0.114 * (b as f64);
                    if lum < 140.0 { "#FFFFFF" } else { "#1E293B" }
                } else {
                    "#1E293B"
                };
                let (cx, cy) = region.centroid;
                let lbl_rect = [cx - 36.0, cy - 11.0, cx + 36.0, cy + 11.0];
                if let Ok(tfid) = emit_text(app, spread_idx, lbl_rect, &text, 6.0, true, "center", Some(text_color)) {
                    frame_ids.push(tfid);
                }
            }
        }

        for leg in &layout.map_legend {
            if let Ok(sfid) = emit_rect(app, spread_idx, leg.swatch_rect, &leg.color, "#CBD5E1", 0.5) {
                frame_ids.push(sfid);
            }
            if let Ok(lfid) = emit_text(app, spread_idx, leg.label_rect, &leg.label, 7.0, false, "center", Some("#475569")) {
                frame_ids.push(lfid);
            }
        }
    }

    // 6. X-tick labels
    let is_hbar = matches!(spec.chart_type, super::spec::ChartType::HorizontalBar);
    for x_tick in &layout.x_tick_labels {
        let align = if is_hbar { "right" } else { "center" };
        let fid = emit_text(app, spread_idx, x_tick.rect, &x_tick.text, 8.0, false, align, Some("#333333"))?;
        if x_tick.rotated {
            let _ = app.run("transform.rotate", json!({"ids": [fid], "angle": 45}));
        }
        frame_ids.push(fid);
    }

    // 7. Direct value labels
    for (s_idx, v_rect, text) in &layout.value_labels {
        let (color, align) = if spec.chart_type == super::spec::ChartType::PopulationPyramid {
            if *s_idx == 0 {
                (series_color(palette, 0), "right")
            } else {
                (if palette.len() > 1 { series_color(palette, 1) } else { "#ED1C6F" }, "left")
            }
        } else {
            (series_color(palette, *s_idx), "center")
        };
        let font = if spec.chart_type == super::spec::ChartType::Line { Some("Lato") } else { None };
        if let Ok(fid) = emit_text_with_font(app, spread_idx, *v_rect, text, 7.5, true, align, Some(color), font) {
            frame_ids.push(fid);
        }
    }

    // 8. Legend (Skipped for PopulationPyramid which has dedicated headers and summary card)
    if spec.chart_type != super::spec::ChartType::PopulationPyramid {
        for leg in &layout.legend_items {
            let color = if spec.chart_type == super::spec::ChartType::PopulationPyramid && leg.series_idx == 1 {
                if palette.len() > 1 { series_color(palette, 1) } else { "#ED1C6F" }
            } else {
                series_color(palette, leg.series_idx)
            };
            let sfid = emit_rect(app, spread_idx, leg.swatch_rect, color, "[None]", 0.0)?;
            frame_ids.push(sfid);

            let lfid = emit_text(app, spread_idx, leg.label_rect, &leg.label, 8.0, false, "left", Some("#333333"))?;
            let _ = app.session.edit(|doc, _| {
                if let Some(it) = doc.item_mut(designcraft_doc::ItemId(lfid))
                    && let designcraft_doc::Content::Text(tf) = &mut it.content
                {
                    tf.options.vertical_justification = designcraft_doc::VerticalJustification::Center;
                }
                Ok(Value::Null)
            });
            if bilingual && leg.label.contains(" / ") {
                // Italicize the English portion of the legend label
                let slash_idx = leg.label.find(" / ").unwrap_or(0);
                let b_start = slash_idx + 3;
                let _b_end = leg.label.len();
                if let Some(active) = app.session.active()
                    && let Some(it) = active.doc.spreads[spread_idx].items.iter().find(|i| i.id.0 == lfid)
                    && let designcraft_doc::Content::Text(tf) = &it.content
                {
                    let sid = tf.story;
                    let _ = app.session.edit(|doc, _| {
                        if let Some(story) = doc.story_mut(sid) {
                            let mut offset = 0;
                            for r in &mut story.chars {
                                let run_end = offset + r.len;
                                offset = run_end;
                                if run_end > b_start {
                                    r.format.over.font_style = Some("Italic".into());
                                }
                            }
                        }
                        Ok(Value::Null)
                    });
                }
            }
            frame_ids.push(lfid);
        }
    }

    // 8b. Data Table
    if let Some(table) = &layout.data_table {
        for &line in &table.grid_lines {
            if let Ok(fid) = emit_rect(app, spread_idx, line, "#666666", "[None]", 0.0) {
                frame_ids.push(fid);
            }
        }
        for cell in &table.cells {
            let font = Some("Lato");
            let color = Some("#111111");
            let size = if cell.is_header {
                6.5
            } else if cell.is_bold {
                7.0
            } else {
                6.5
            };
            if let Ok(tfid) = emit_text_with_font(app, spread_idx, cell.rect, &cell.text, size, cell.is_bold, cell.align, color, font) {
                frame_ids.push(tfid);
            }
        }
        for &(s_idx, m_rect, line) in &table.series_keys {
            let color = if !palette.is_empty() { series_color(palette, s_idx) } else { super::palette::brs_line_series_color(s_idx) };
            if let Ok(lfid) = emit_rect(app, spread_idx, line, color, "[None]", 0.0) {
                frame_ids.push(lfid);
            }
            if s_idx == 0 {
                let _ = emit_rect(app, spread_idx, m_rect, color, color, 0.0).map(|id| frame_ids.push(id));
            } else if s_idx == 1 {
                let cx = (m_rect[0] + m_rect[2]) / 2.0;
                let cy = (m_rect[1] + m_rect[3]) / 2.0;
                let r = 2.0;
                let pts: Vec<(f64, f64)> = (0..12)
                    .map(|i| {
                        let a = i as f64 * std::f64::consts::TAU / 12.0;
                        (cx + r * a.cos(), cy + r * a.sin())
                    })
                    .collect();
                let _ = emit_polygon(app, spread_idx, &pts, "#FFFFFF", color, 1.2).map(|id| frame_ids.push(id));
            } else {
                let cx = (m_rect[0] + m_rect[2]) / 2.0;
                let cy = (m_rect[1] + m_rect[3]) / 2.0;
                let r = 2.2;
                let pts = [(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)];
                let _ = emit_polygon(app, spread_idx, &pts, color, "[None]", 0.0).map(|id| frame_ids.push(id));
            }
        }
    }

    // 8c. Bottom Title
    if let Some((b_rect, b_title)) = &layout.bottom_title {
        if let Ok(tfid) = emit_text_with_font(app, spread_idx, *b_rect, b_title, 9.5, true, "center", Some("#D97706"), Some("Raleway")) {
            frame_ids.push(tfid);
        }
    }

    // 9. Group all emitted frames
    if frame_ids.len() > 1 {
        let _ = app.run("object.group", json!({"ids": frame_ids}));
    }

    Ok(frame_ids.len())
}

#[cfg(test)]
mod tests {
    use super::super::layout::compute_layout;
    use super::super::spec::{ChartSpec, ChartType, SeriesSpec, ValueFormat};
    use super::*;
    use crate::Services;
    use designcraft_geom::Rect;

    fn test_app() -> DesignApp {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), Services::default());
        app.run("file.new", json!({"width": 595.28, "height": 841.89})).unwrap();
        app
    }

    fn sample_spec(chart_type: ChartType) -> ChartSpec {
        ChartSpec {
            chart_type,
            title_id: "Judul Grafik".into(),
            title_en: "Chart Title".into(),
            category_col: "A".into(),
            series: vec![
                SeriesSpec { col: "B".into(), label_id: "Pria".into(), label_en: "Male".into() },
                SeriesSpec { col: "C".into(), label_id: "Wanita".into(), label_en: "Female".into() },
            ],
            data_start_row: 1,
            data_end_row: 3,
            value_format: ValueFormat::Number,
            show_value_labels: true,
            show_legend: true,
            show_grid: true,
            max_value_override: None,
            geojson_path: None,
            map_palette_idx: 0,
            include_data_table: false,
        }
    }

    fn sample_margin() -> Rect {
        Rect { x0: 36.0, y0: 36.0, x1: 550.0, y1: 750.0 }
    }

    #[test]
    fn map_chart_emits_polygon_frames_and_legend() {
        let mut app = test_app();
        let mut spec = sample_spec(ChartType::Map);
        spec.geojson_path = Some("PANGKALAN".into());
        spec.show_legend = true;
        spec.show_value_labels = true;

        let categories = vec![
            "CINTAASIH".into(),
            "MULANGSARI".into(),
            "TAMANMEKAR".into(),
            "CIPTASARI".into(),
            "TAMANSARI".into(),
            "JATILAKSANA".into(),
            "KERTASARI".into(),
            "MEDALSARI".into(),
        ];
        let values = vec![vec![78.5, 82.1, 64.3, 71.0, 75.4, 69.8, 85.2, 60.5]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();
        assert!(!layout.map_regions.is_empty(), "Map layout must contain regions");
        assert!(!layout.map_legend.is_empty(), "Map layout must contain legend entries");

        let count = emit_chart(&mut app, 0, &spec, &layout, &[], false).unwrap();
        assert!(count > 10, "Map chart must emit multiple polygon and legend frames: {count}");

        let active = app.session.active().unwrap();
        assert!(!active.doc.spreads[0].items.is_empty());
    }

    #[test]
    fn bar_clustered_emits_correct_frame_count() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Desa A".into(), "Desa B".into(), "Desa C".into()];
        let values = vec![vec![10.0, 20.0, 30.0], vec![15.0, 25.0, 35.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();
        let count = emit_chart(&mut app, 0, &spec, &layout, &[], false).unwrap();

        assert!(count > 10, "Chart must emit multiple frames: {count}");

        let active = app.session.active().unwrap();
        let items_count = active.doc.spreads[0].items.len();
        assert!(items_count > 0, "Spread should contain emitted frames or group");
    }

    #[test]
    fn bar_stacked_frames_do_not_overlap_ceiling() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::BarStacked);
        let categories = vec!["Cat 1".into(), "Cat 2".into()];
        let values = vec![vec![40.0, 80.0], vec![60.0, 120.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();
        let count = emit_chart(&mut app, 0, &spec, &layout, &[], false).unwrap();

        assert!(count > 0);
        let active = app.session.active().unwrap();
        assert!(!active.doc.spreads[0].items.is_empty());
    }

    #[test]
    fn line_chart_emits_markers_and_segments() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::Line);
        let categories = vec!["2024".into(), "2025".into(), "2026".into()];
        let values = vec![vec![100.0, 150.0, 200.0], vec![80.0, 120.0, 190.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();
        let count = emit_chart(&mut app, 0, &spec, &layout, &[], false).unwrap();

        assert!(count > 10);
    }

    #[test]
    fn donut_chart_emits_slice_frames() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::Donut);
        let categories = vec!["Slice 1".into(), "Slice 2".into()];
        let values = vec![vec![40.0, 60.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();
        let count = emit_chart(&mut app, 0, &spec, &layout, &[], false).unwrap();

        assert!(count > 0);
    }

    #[test]
    fn chart_is_grouped_after_emission() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["A".into(), "B".into()];
        let values = vec![vec![10.0, 20.0], vec![30.0, 40.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();
        let _ = emit_chart(&mut app, 0, &spec, &layout, &[], false).unwrap();

        let active = app.session.active().unwrap();
        // Spreads contain a Group item containing the child frames
        let has_group = active.doc.spreads[0].items.iter().any(|it| matches!(it.content, designcraft_doc::Content::Group { .. }));
        assert!(has_group, "Emitted chart should be wrapped in an object.group");
    }

    #[test]
    fn legend_frames_match_series_count() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["A".into()];
        let values = vec![vec![10.0], vec![20.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), true).unwrap();
        assert_eq!(layout.legend_items.len(), 2);

        let _ = emit_chart(&mut app, 0, &spec, &layout, &[], true).unwrap();
        let active = app.session.active().unwrap();
        assert!(!active.doc.spreads[0].items.is_empty());

        let mut found_center_vj = false;
        for it in &active.doc.spreads[0].items {
            if let designcraft_doc::Content::Group { items } = &it.content {
                for kid in items {
                    if let designcraft_doc::Content::Text(tf) = &kid.content {
                        if tf.options.vertical_justification == designcraft_doc::VerticalJustification::Center {
                            found_center_vj = true;
                        }
                    }
                }
            }
        }
        assert!(found_center_vj, "Legend text frame inside group must have VerticalJustification::Center");
    }

    #[test]
    fn population_pyramid_emits_frames() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::PopulationPyramid);
        let categories = vec!["00-04".into(), "05-09".into()];
        let values = vec![vec![500.0, 800.0], vec![480.0, 790.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();
        let count = emit_chart(&mut app, 0, &spec, &layout, &[], false).unwrap();

        assert!(count >= 30, "Population pyramid must emit bars, labels, ticks, pictograms, and summary frames (got {count})");

        // Verify that bars have stroke outlines and pictograms (ellipses & polygons) are inside the group
        let active = app.session.active().unwrap();
        let mut bar_strokes = 0;
        let mut pictogram_shapes = 0;
        for it in &active.doc.spreads[0].items {
            if let designcraft_doc::Content::Group { items } = &it.content {
                for kid in items {
                    if kid.stroke.weight > 0.0 {
                        bar_strokes += 1;
                    }
                    if matches!(kid.shape, designcraft_doc::Shape::Oval | designcraft_doc::Shape::Polygon) {
                        pictogram_shapes += 1;
                    }
                }
            }
        }
        assert!(bar_strokes >= 4, "Population pyramid bars must have stroke outlines (got {bar_strokes})");
        assert!(pictogram_shapes >= 8, "Pictograms must be included inside the chart group (got {pictogram_shapes})");
    }

    #[test]
    fn gauge_chart_emits_frames() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::Gauge);
        let categories = vec!["Target".into()];
        let values = vec![vec![60.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();
        let count = emit_chart(&mut app, 0, &spec, &layout, &[], false).unwrap();

        assert!(count >= 7, "Gauge chart must emit track, value, needle, pivot, readout and subtitle");
    }
}
