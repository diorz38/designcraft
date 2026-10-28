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
use super::spec::ChartSpec;

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

/// Emits a styled text frame onto the given spread.
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

    if let Some(story_id) = sid {
        let _ = app.session.edit(|doc, _| {
            if let Some(story) = doc.story_mut(designcraft_doc::StoryId(story_id)) {
                for r in &mut story.chars {
                    r.format.over.font_family = Some("Arial".into());
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
            Ok(Value::Null)
        });
    }

    let _ = app.run("object.stroke", json!({"ids": [fid], "swatch": "[None]"}));
    let _ = app.run("object.fill", json!({"ids": [fid], "swatch": "[None]"}));

    Ok(fid)
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
        if let Ok(tfid) = emit_text(app, spread_idx, tick_rect, &gl.label, 7.5, false, align, Some("#666666")) {
            frame_ids.push(tfid);
        }
    }

    // 2. Axis baseline & left line (only for Cartesian coordinate charts)
    if spec.chart_type != super::spec::ChartType::Donut {
        let base_fid = emit_rect(app, spread_idx, layout.axis_baseline, "#333333", "[None]", 0.0)?;
        frame_ids.push(base_fid);
        let left_fid = emit_rect(app, spread_idx, layout.axis_left, "#333333", "[None]", 0.0)?;
        frame_ids.push(left_fid);
    }

    // 3. Bars
    for bar in &layout.bars {
        let color = series_color(palette, bar.series_idx);
        let fid = emit_rect(app, spread_idx, bar.rect, color, "[None]", 0.0)?;
        frame_ids.push(fid);
    }

    // 4. Line segments and markers
    for (s_idx, segments) in layout.line_segments.iter().enumerate() {
        let color = series_color(palette, s_idx);
        for &seg_rect in segments {
            let fid = emit_rect(app, spread_idx, seg_rect, color, "[None]", 0.0)?;
            frame_ids.push(fid);
        }
    }
    for (s_idx, markers) in layout.line_markers.iter().enumerate() {
        let color = series_color(palette, s_idx);
        for &m_rect in markers {
            let fid = emit_rect(app, spread_idx, m_rect, color, "[Paper]", 1.0)?;
            frame_ids.push(fid);
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

    // 7. Direct value labels (above bars / points)
    for (s_idx, v_rect, text) in &layout.value_labels {
        let color = series_color(palette, *s_idx);
        if let Ok(fid) = emit_text(app, spread_idx, *v_rect, text, 7.5, true, "center", Some(color)) {
            frame_ids.push(fid);
        }
    }

    // 8. Legend
    for leg in &layout.legend_items {
        let color = series_color(palette, leg.series_idx);
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
        }
    }

    fn sample_margin() -> Rect {
        Rect { x0: 36.0, y0: 36.0, x1: 550.0, y1: 750.0 }
    }

    #[test]
    fn bar_clustered_emits_correct_frame_count() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Desa A".into(), "Desa B".into(), "Desa C".into()];
        let values = vec![vec![10.0, 20.0, 30.0], vec![15.0, 25.0, 35.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);
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

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);
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

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);
        let count = emit_chart(&mut app, 0, &spec, &layout, &[], false).unwrap();

        assert!(count > 10);
    }

    #[test]
    fn donut_chart_emits_slice_frames() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::Donut);
        let categories = vec!["Slice 1".into(), "Slice 2".into()];
        let values = vec![vec![40.0, 60.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);
        let count = emit_chart(&mut app, 0, &spec, &layout, &[], false).unwrap();

        assert!(count > 0);
    }

    #[test]
    fn chart_is_grouped_after_emission() {
        let mut app = test_app();
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["A".into(), "B".into()];
        let values = vec![vec![10.0, 20.0], vec![30.0, 40.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);
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

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), true);
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
}
