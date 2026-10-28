//! Layout calculation and coordinate mapping for vector charts.
//!
//! Transforms a `ChartSpec` and extracted data matrix into pixel-precise
//! coordinates for bars, grid lines, axes, tick labels, value callouts,
//! line markers, donut segments, and legend swatches.
//!
//! All calculations are pure geometry with zero engine dependencies or side effects.

use designcraft_geom::Rect;

use super::spec::{ChartSpec, ChartType};

/// Width reserved for Y-axis tick labels (points).
pub const AXIS_LABEL_W: f64 = 44.0;
/// Height reserved for X-axis tick labels (points).
pub const AXIS_LABEL_H: f64 = 20.0;
/// Width reserved for category labels on horizontal bar charts (points).
pub const HBAR_CATEGORY_W: f64 = 72.0;
/// Height per row in the legend area (points).
pub const LEGEND_ROW_H: f64 = 18.0;
/// Dimensions of legend color swatch (points).
pub const LEGEND_SWATCH_W: f64 = 12.0;
pub const LEGEND_SWATCH_H: f64 = 10.0;
/// Gap between category bar groups (fraction of group slot width).
pub const BAR_GROUP_GAP: f64 = 0.20;
/// Gap between series bars inside a group (fraction of bar width).
pub const BAR_SERIES_GAP: f64 = 0.05;
/// Number of horizontal grid subdivisions.
pub const GRID_LINES: usize = 5;
/// Maximum character count for X-axis label before rotation is triggered.
pub const MAX_LABEL_CHARS: usize = 12;

/// A single bar rectangle on the chart.
#[derive(Debug, Clone, PartialEq)]
pub struct BarRect {
    /// Bounding rectangle `[x0, y0, x1, y1]`.
    pub rect: [f64; 4],
    /// Original numeric value.
    pub value: f64,
    /// Index into `spec.series`.
    pub series_idx: usize,
    /// Index into `categories`.
    pub category_idx: usize,
    /// Formatted value string for callout label.
    pub label: String,
}

/// A horizontal or vertical grid line across the plot area.
#[derive(Debug, Clone, PartialEq)]
pub struct GridLine {
    /// Bounding rectangle `[x0, y0, x1, y1]`.
    pub rect: [f64; 4],
    /// Numeric value corresponding to this grid level.
    pub value: f64,
    /// Formatted tick label (e.g., `"200"` or `"50%"`).
    pub label: String,
}

/// Tick label placed along the category axis.
#[derive(Debug, Clone, PartialEq)]
pub struct XTickLabel {
    /// Bounding rectangle for the text frame `[x0, y0, x1, y1]`.
    pub rect: [f64; 4],
    /// Label text.
    pub text: String,
    /// Whether the label should be angled/rotated (e.g. 45°).
    pub rotated: bool,
}

/// A legend entry with swatch and text label.
#[derive(Debug, Clone, PartialEq)]
pub struct LegendItem {
    pub swatch_rect: [f64; 4],
    pub label_rect: [f64; 4],
    pub label: String,
    pub series_idx: usize,
}

/// An arc/wedge segment of a donut chart.
#[derive(Debug, Clone, PartialEq)]
pub struct DonutSlice {
    /// Polygon points [x, y] tracing the annular wedge (outer arc, then inner arc in reverse, closed).
    pub points: Vec<(f64, f64)>,
    /// Frame for the slice label (category / percentage).
    pub label_rect: [f64; 4],
    /// Category / slice label.
    pub label: String,
    /// Formatted percentage or value string.
    pub value_label: String,
    /// Series / color index.
    pub series_idx: usize,
    /// Sweep angle in degrees (0.0 .. 360.0).
    pub sweep_degrees: f64,
}

/// Complete geometry layout for a vector chart.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartLayout {
    /// Bars for `BarClustered`, `BarStacked`, and `HorizontalBar`.
    pub bars: Vec<BarRect>,
    /// Points/markers per series for `Line` charts (4x4 pt square rects).
    pub line_markers: Vec<Vec<[f64; 4]>>,
    /// Connecting line segments per series.
    pub line_segments: Vec<Vec<[f64; 4]>>,
    /// Slices for `Donut` chart.
    pub donut_slices: Vec<DonutSlice>,
    /// Reference grid lines.
    pub grid_lines: Vec<GridLine>,
    /// Category axis tick labels.
    pub x_tick_labels: Vec<XTickLabel>,
    /// Direct value labels: `(series_idx, rect, text)`.
    pub value_labels: Vec<(usize, [f64; 4], String)>,
    /// Legend items (swatch + label).
    pub legend_items: Vec<LegendItem>,
    /// Solid baseline (X-axis).
    pub axis_baseline: [f64; 4],
    /// Solid left axis line (Y-axis).
    pub axis_left: [f64; 4],
    /// Primary plotting rectangle `[x0, y0, x1, y1]`.
    pub plot_rect: [f64; 4],
    /// Axis ceiling value used for scaling.
    pub ceiling: f64,
}

/// Computes the complete pixel-precise layout for a chart.
pub fn compute_layout(spec: &ChartSpec, categories: &[String], series_values: &[Vec<f64>], margin: Rect, bilingual: bool) -> ChartLayout {
    let title_h = if bilingual && !spec.title_en.is_empty() {
        56.0
    } else if !spec.title_id.is_empty() {
        44.0
    } else {
        0.0
    };
    let top_offset = if title_h > 0.0 { title_h + 8.0 } else { 0.0 };

    let is_donut = matches!(spec.chart_type, ChartType::Donut);
    let is_hbar = matches!(spec.chart_type, ChartType::HorizontalBar);
    let any_rotated = !is_hbar && !is_donut && categories.iter().any(|c| c.chars().count() > MAX_LABEL_CHARS);
    let left_reserve = if is_donut {
        0.0
    } else if is_hbar {
        HBAR_CATEGORY_W
    } else {
        AXIS_LABEL_W
    };
    let bottom_axis_h = if is_donut {
        0.0
    } else if is_hbar {
        16.0
    } else if any_rotated {
        64.0
    } else {
        AXIS_LABEL_H + 4.0
    };
    let bottom_gap = if any_rotated { 16.0 } else { 4.0 };

    let use_category_legend = is_donut || spec.series.len() <= 1;

    let legend_rows = if spec.show_legend {
        if use_category_legend && !categories.is_empty() {
            categories.len().div_ceil(4)
        } else if !spec.series.is_empty() {
            spec.series.len().div_ceil(3)
        } else {
            0
        }
    } else {
        0
    };
    let legend_h = if legend_rows > 0 { legend_rows as f64 * LEGEND_ROW_H + 6.0 } else { 0.0 };

    let plot_x0 = margin.x0 + left_reserve;
    let plot_x1 = (margin.x1 - 12.0).max(plot_x0 + 10.0);
    let plot_y0 = margin.y0 + top_offset + 10.0;
    let plot_y1 = (margin.y1 - bottom_axis_h - legend_h - bottom_gap).max(plot_y0 + 10.0);
    let plot_rect = [plot_x0, plot_y0, plot_x1, plot_y1];

    // Compute axis ceiling
    let raw_max = match spec.chart_type {
        ChartType::BarStacked => {
            let mut max_sum = 0.0f64;
            for c_idx in 0..categories.len() {
                let sum: f64 = series_values.iter().filter_map(|s| s.get(c_idx).copied()).sum();
                if sum > max_sum {
                    max_sum = sum;
                }
            }
            max_sum
        }
        ChartType::Donut => {
            let mut total = 0.0f64;
            if let Some(first_series) = series_values.first() {
                for &v in first_series {
                    if v > 0.0 {
                        total += v;
                    }
                }
            }
            total
        }
        _ => {
            let mut max_v = 0.0f64;
            for s in series_values {
                for &v in s {
                    if v > max_v {
                        max_v = v;
                    }
                }
            }
            max_v
        }
    };

    let ceiling = if let Some(override_max) = spec.max_value_override { override_max.max(raw_max).max(1.0) } else { compute_nice_ceiling(raw_max) };

    // Axes
    let axis_baseline = [plot_x0, plot_y1, plot_x1, plot_y1 + 1.0];
    let axis_left = [plot_x0 - 1.0, plot_y0, plot_x0, plot_y1];

    // Grid lines
    let mut grid_lines = Vec::new();
    if spec.show_grid && ceiling > 0.0 {
        if is_hbar {
            // Vertical grid lines for horizontal bar chart
            let plot_w = plot_x1 - plot_x0;
            for step in 1..=GRID_LINES {
                let frac = step as f64 / GRID_LINES as f64;
                let x = plot_x0 + frac * plot_w;
                let val = frac * ceiling;
                let label = spec.value_format.format(val, bilingual);
                grid_lines.push(GridLine { rect: [x, plot_y0, x + 0.5, plot_y1], value: val, label });
            }
        } else {
            // Horizontal grid lines for standard charts
            let plot_h = plot_y1 - plot_y0;
            for step in 1..=GRID_LINES {
                let frac = step as f64 / GRID_LINES as f64;
                let y = plot_y1 - frac * plot_h;
                let val = frac * ceiling;
                let label = spec.value_format.format(val, bilingual);
                grid_lines.push(GridLine { rect: [plot_x0, y, plot_x1, y + 0.5], value: val, label });
            }
        }
    }

    // Legend items
    let mut legend_items = Vec::new();
    if spec.show_legend {
        if use_category_legend && !categories.is_empty() {
            // For Donut charts or any chart with only 1 series data, legend always takes category items
            let leg_y_start = (margin.y1 - legend_h + 4.0).max(plot_y1 + bottom_axis_h + bottom_gap);
            let total_cats = categories.len();
            let items_per_row = total_cats.clamp(1, 4);
            let col_w = (margin.x1 - margin.x0) / items_per_row as f64;

            for (cat_idx, cat_name) in categories.iter().enumerate() {
                let row = cat_idx / items_per_row;
                let col = cat_idx % items_per_row;
                let item_x = margin.x0 + col as f64 * col_w;
                let item_y = leg_y_start + row as f64 * LEGEND_ROW_H;

                let swatch_y0 = item_y + (LEGEND_ROW_H - LEGEND_SWATCH_H) / 2.0;
                let swatch_rect = [item_x, swatch_y0, item_x + LEGEND_SWATCH_W, swatch_y0 + LEGEND_SWATCH_H];
                let label_rect = [item_x + LEGEND_SWATCH_W + 6.0, item_y, item_x + col_w - 4.0, item_y + LEGEND_ROW_H];
                legend_items.push(LegendItem { swatch_rect, label_rect, label: cat_name.clone(), series_idx: cat_idx });
            }
        } else if !spec.series.is_empty() {
            let leg_y_start = (margin.y1 - legend_h + 4.0).max(plot_y1 + bottom_axis_h + bottom_gap);
            let total_series = spec.series.len();
            let items_per_row = total_series.clamp(1, 3);
            let col_w = (margin.x1 - margin.x0) / items_per_row as f64;

            for (s_idx, series) in spec.series.iter().enumerate() {
                let row = s_idx / items_per_row;
                let col = s_idx % items_per_row;
                let item_x = margin.x0 + col as f64 * col_w;
                let item_y = leg_y_start + row as f64 * LEGEND_ROW_H;

                let swatch_y0 = item_y + (LEGEND_ROW_H - LEGEND_SWATCH_H) / 2.0;
                let swatch_rect = [item_x, swatch_y0, item_x + LEGEND_SWATCH_W, swatch_y0 + LEGEND_SWATCH_H];
                let label_rect = [item_x + LEGEND_SWATCH_W + 6.0, item_y, item_x + col_w - 4.0, item_y + LEGEND_ROW_H];
                let label = series.display_label(bilingual);
                legend_items.push(LegendItem { swatch_rect, label_rect, label, series_idx: s_idx });
            }
        }
    }

    // Specific chart type layouts
    match spec.chart_type {
        ChartType::BarClustered => {
            layout_bar_clustered(spec, categories, series_values, plot_rect, ceiling, bilingual, grid_lines, legend_items, axis_baseline, axis_left)
        }
        ChartType::BarStacked => {
            layout_bar_stacked(spec, categories, series_values, plot_rect, ceiling, bilingual, grid_lines, legend_items, axis_baseline, axis_left)
        }
        ChartType::HorizontalBar => {
            layout_horizontal_bar(spec, categories, series_values, plot_rect, ceiling, bilingual, grid_lines, legend_items, axis_baseline, axis_left)
        }
        ChartType::Line | ChartType::Area => {
            layout_line_and_area(spec, categories, series_values, plot_rect, ceiling, bilingual, grid_lines, legend_items, axis_baseline, axis_left)
        }
        ChartType::Donut => layout_donut(spec, categories, series_values, plot_rect, ceiling, bilingual, legend_items, axis_baseline, axis_left),
    }
}

/// Compute nice rounded ceiling for chart axis.
fn compute_nice_ceiling(raw: f64) -> f64 {
    if raw <= 0.0 {
        return 10.0;
    }
    let magnitude = 10.0f64.powf(raw.log10().floor());
    let normalized = raw / magnitude;
    let nice_norm = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 2.5 {
        2.5
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice_norm * magnitude
}

fn layout_bar_clustered(
    spec: &ChartSpec,
    categories: &[String],
    series_values: &[Vec<f64>],
    plot_rect: [f64; 4],
    ceiling: f64,
    bilingual: bool,
    grid_lines: Vec<GridLine>,
    legend_items: Vec<LegendItem>,
    axis_baseline: [f64; 4],
    axis_left: [f64; 4],
) -> ChartLayout {
    let mut bars = Vec::new();
    let mut x_tick_labels = Vec::new();
    let mut value_labels = Vec::new();

    let plot_w = plot_rect[2] - plot_rect[0];
    let plot_h = plot_rect[3] - plot_rect[1];
    let cat_count = categories.len().max(1);
    let group_w = plot_w / cat_count as f64;
    let group_gap = group_w * BAR_GROUP_GAP;
    let content_w = (group_w - group_gap).max(1.0);

    let series_count = spec.series.len().max(1);
    let series_gap = (content_w * BAR_SERIES_GAP).min(4.0);
    let total_series_gaps = series_gap * (series_count - 1) as f64;
    let bar_w = ((content_w - total_series_gaps) / series_count as f64).max(1.0);

    for (c_idx, cat) in categories.iter().enumerate() {
        let group_x0 = plot_rect[0] + c_idx as f64 * group_w + group_gap / 2.0;

        for (s_idx, s_vals) in series_values.iter().enumerate() {
            let val = s_vals.get(c_idx).copied().unwrap_or(0.0);
            let bar_h = if ceiling > 0.0 { ((val / ceiling) * plot_h).max(0.0) } else { 0.0 };

            let x0 = group_x0 + s_idx as f64 * (bar_w + series_gap);
            let x1 = x0 + bar_w;
            let y0 = (plot_rect[3] - bar_h).max(plot_rect[1]);
            let y1 = plot_rect[3];

            let label_str = spec.value_format.format(val, bilingual);
            let color_idx = if spec.series.len() <= 1 { c_idx } else { s_idx };
            bars.push(BarRect { rect: [x0, y0, x1, y1], value: val, series_idx: color_idx, category_idx: c_idx, label: label_str.clone() });

            if spec.show_value_labels && val > 0.0 {
                let lbl_w = bar_w.max(28.0);
                let lbl_x0 = (x0 + x1) / 2.0 - lbl_w / 2.0;
                let lbl_y1 = (y0 - 2.0).max(plot_rect[1]);
                let lbl_y0 = lbl_y1 - 10.0;
                value_labels.push((color_idx, [lbl_x0, lbl_y0, lbl_x0 + lbl_w, lbl_y1], label_str));
            }
        }

        // X tick label centered below group
        let center_x = group_x0 + content_w / 2.0;
        let rotated = cat.chars().count() > MAX_LABEL_CHARS;
        let lbl_w = group_w.max(40.0);
        let lbl_rect = [center_x - lbl_w / 2.0, plot_rect[3] + 4.0, center_x + lbl_w / 2.0, plot_rect[3] + AXIS_LABEL_H];
        x_tick_labels.push(XTickLabel { rect: lbl_rect, text: cat.clone(), rotated });
    }

    ChartLayout {
        bars,
        line_markers: Vec::new(),
        line_segments: Vec::new(),
        donut_slices: Vec::new(),
        grid_lines,
        x_tick_labels,
        value_labels,
        legend_items,
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
    }
}

fn layout_bar_stacked(
    spec: &ChartSpec,
    categories: &[String],
    series_values: &[Vec<f64>],
    plot_rect: [f64; 4],
    ceiling: f64,
    bilingual: bool,
    grid_lines: Vec<GridLine>,
    legend_items: Vec<LegendItem>,
    axis_baseline: [f64; 4],
    axis_left: [f64; 4],
) -> ChartLayout {
    let mut bars = Vec::new();
    let mut x_tick_labels = Vec::new();
    let mut value_labels = Vec::new();

    let plot_w = plot_rect[2] - plot_rect[0];
    let plot_h = plot_rect[3] - plot_rect[1];
    let cat_count = categories.len().max(1);
    let group_w = plot_w / cat_count as f64;
    let group_gap = group_w * BAR_GROUP_GAP;
    let bar_w = (group_w - group_gap).max(1.0);

    for (c_idx, cat) in categories.iter().enumerate() {
        let x0 = plot_rect[0] + c_idx as f64 * group_w + group_gap / 2.0;
        let x1 = x0 + bar_w;
        let mut cur_y = plot_rect[3];

        for (s_idx, s_vals) in series_values.iter().enumerate() {
            let val = s_vals.get(c_idx).copied().unwrap_or(0.0);
            let bar_h = if ceiling > 0.0 { ((val / ceiling) * plot_h).max(0.0) } else { 0.0 };

            let y1 = cur_y;
            let y0 = (cur_y - bar_h).max(plot_rect[1]);
            cur_y = y0;

            let label_str = spec.value_format.format(val, bilingual);
            let color_idx = if spec.series.len() <= 1 { c_idx } else { s_idx };
            bars.push(BarRect { rect: [x0, y0, x1, y1], value: val, series_idx: color_idx, category_idx: c_idx, label: label_str.clone() });

            if spec.show_value_labels && bar_h > 12.0 {
                let lbl_y = (y0 + y1) / 2.0 - 5.0;
                value_labels.push((color_idx, [x0, lbl_y, x1, lbl_y + 10.0], label_str));
            }
        }

        let center_x = x0 + bar_w / 2.0;
        let rotated = cat.chars().count() > MAX_LABEL_CHARS;
        let lbl_w = group_w.max(40.0);
        let lbl_rect = [center_x - lbl_w / 2.0, plot_rect[3] + 4.0, center_x + lbl_w / 2.0, plot_rect[3] + AXIS_LABEL_H];
        x_tick_labels.push(XTickLabel { rect: lbl_rect, text: cat.clone(), rotated });
    }

    ChartLayout {
        bars,
        line_markers: Vec::new(),
        line_segments: Vec::new(),
        donut_slices: Vec::new(),
        grid_lines,
        x_tick_labels,
        value_labels,
        legend_items,
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
    }
}

fn layout_horizontal_bar(
    spec: &ChartSpec,
    categories: &[String],
    series_values: &[Vec<f64>],
    plot_rect: [f64; 4],
    ceiling: f64,
    bilingual: bool,
    grid_lines: Vec<GridLine>,
    legend_items: Vec<LegendItem>,
    axis_baseline: [f64; 4],
    axis_left: [f64; 4],
) -> ChartLayout {
    let mut bars = Vec::new();
    let mut x_tick_labels = Vec::new();
    let mut value_labels = Vec::new();

    let plot_w = plot_rect[2] - plot_rect[0];
    let plot_h = plot_rect[3] - plot_rect[1];
    let cat_count = categories.len().max(1);
    let slot_h = plot_h / cat_count as f64;
    let slot_gap = slot_h * BAR_GROUP_GAP;
    let content_h = (slot_h - slot_gap).max(1.0);

    let series_count = spec.series.len().max(1);
    let series_gap = (content_h * BAR_SERIES_GAP).min(4.0);
    let total_series_gaps = series_gap * (series_count - 1) as f64;
    let bar_h = ((content_h - total_series_gaps) / series_count as f64).max(1.0);

    for (c_idx, cat) in categories.iter().enumerate() {
        let slot_y0 = plot_rect[1] + c_idx as f64 * slot_h + slot_gap / 2.0;

        for (s_idx, s_vals) in series_values.iter().enumerate() {
            let val = s_vals.get(c_idx).copied().unwrap_or(0.0);
            let bar_w = if ceiling > 0.0 { ((val / ceiling) * plot_w).max(0.0) } else { 0.0 };

            let x0 = plot_rect[0];
            let x1 = (x0 + bar_w).min(plot_rect[2]);
            let y0 = slot_y0 + s_idx as f64 * (bar_h + series_gap);
            let y1 = y0 + bar_h;

            let label_str = spec.value_format.format(val, bilingual);
            let color_idx = if spec.series.len() <= 1 { c_idx } else { s_idx };
            bars.push(BarRect { rect: [x0, y0, x1, y1], value: val, series_idx: color_idx, category_idx: c_idx, label: label_str.clone() });

            if spec.show_value_labels && val > 0.0 {
                let lbl_x = x1 + 4.0;
                value_labels.push((color_idx, [lbl_x, y0, lbl_x + 36.0, y1], label_str));
            }
        }

        // Category label on left axis
        let center_y = slot_y0 + content_h / 2.0;
        let lbl_rect = [plot_rect[0] - HBAR_CATEGORY_W, center_y - 6.0, plot_rect[0] - 4.0, center_y + 6.0];
        x_tick_labels.push(XTickLabel { rect: lbl_rect, text: cat.clone(), rotated: false });
    }

    ChartLayout {
        bars,
        line_markers: Vec::new(),
        line_segments: Vec::new(),
        donut_slices: Vec::new(),
        grid_lines,
        x_tick_labels,
        value_labels,
        legend_items,
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
    }
}

fn layout_line_and_area(
    spec: &ChartSpec,
    categories: &[String],
    series_values: &[Vec<f64>],
    plot_rect: [f64; 4],
    ceiling: f64,
    bilingual: bool,
    grid_lines: Vec<GridLine>,
    legend_items: Vec<LegendItem>,
    axis_baseline: [f64; 4],
    axis_left: [f64; 4],
) -> ChartLayout {
    let mut line_markers = Vec::new();
    let mut line_segments = Vec::new();
    let mut x_tick_labels = Vec::new();
    let mut value_labels = Vec::new();

    let plot_w = plot_rect[2] - plot_rect[0];
    let plot_h = plot_rect[3] - plot_rect[1];
    let cat_count = categories.len().max(1);
    let step_x = if cat_count > 1 { plot_w / (cat_count - 1) as f64 } else { plot_w / 2.0 };

    for (s_idx, s_vals) in series_values.iter().enumerate() {
        let mut markers = Vec::new();
        let mut segments = Vec::new();
        let mut prev_pt: Option<(f64, f64)> = None;

        for (c_idx, &val) in s_vals.iter().enumerate().take(cat_count) {
            let cx = if cat_count > 1 { plot_rect[0] + c_idx as f64 * step_x } else { plot_rect[0] + plot_w / 2.0 };
            let cy = if ceiling > 0.0 { plot_rect[3] - (val / ceiling) * plot_h } else { plot_rect[3] };

            // 4x4 pt square marker
            markers.push([cx - 2.0, cy - 2.0, cx + 2.0, cy + 2.0]);

            if let Some((px, py)) = prev_pt {
                let seg_x0 = px.min(cx);
                let seg_x1 = px.max(cx);
                let seg_y0 = py.min(cy);
                let seg_y1 = (py.max(cy)).max(seg_y0 + 1.0);
                segments.push([seg_x0, seg_y0, seg_x1, seg_y1]);
            }
            prev_pt = Some((cx, cy));

            if spec.show_value_labels {
                let label_str = spec.value_format.format(val, bilingual);
                value_labels.push((s_idx, [cx - 16.0, cy - 14.0, cx + 16.0, cy - 2.0], label_str));
            }
        }

        line_markers.push(markers);
        line_segments.push(segments);
    }

    for (c_idx, cat) in categories.iter().enumerate() {
        let cx = if cat_count > 1 { plot_rect[0] + c_idx as f64 * step_x } else { plot_rect[0] + plot_w / 2.0 };
        let rotated = cat.chars().count() > MAX_LABEL_CHARS;
        let lbl_w = 40.0;
        let lbl_rect = [cx - lbl_w / 2.0, plot_rect[3] + 4.0, cx + lbl_w / 2.0, plot_rect[3] + AXIS_LABEL_H];
        x_tick_labels.push(XTickLabel { rect: lbl_rect, text: cat.clone(), rotated });
    }

    ChartLayout {
        bars: Vec::new(),
        line_markers,
        line_segments,
        donut_slices: Vec::new(),
        grid_lines,
        x_tick_labels,
        value_labels,
        legend_items,
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
    }
}

fn layout_donut(
    _spec: &ChartSpec,
    categories: &[String],
    series_values: &[Vec<f64>],
    plot_rect: [f64; 4],
    ceiling: f64,
    _bilingual: bool,
    legend_items: Vec<LegendItem>,
    axis_baseline: [f64; 4],
    axis_left: [f64; 4],
) -> ChartLayout {
    let mut donut_slices = Vec::new();
    let s_vals = series_values.first().map(Vec::as_slice).unwrap_or(&[]);
    let sum_vals: f64 = s_vals.iter().copied().sum();
    let total = if sum_vals > 0.0 { sum_vals } else { 1.0 };

    let plot_w = plot_rect[2] - plot_rect[0];
    let plot_h = plot_rect[3] - plot_rect[1];
    let cx = (plot_rect[0] + plot_rect[2]) / 2.0;
    let cy = (plot_rect[1] + plot_rect[3]) / 2.0;
    let outer_r = (plot_w.min(plot_h) / 2.0 * 0.70).max(20.0);
    let inner_r = outer_r * 0.52;

    let mut cur_angle = 0.0f64;

    for (idx, &val) in s_vals.iter().enumerate() {
        let cat = categories.get(idx).cloned().unwrap_or_else(|| format!("Item {}", idx + 1));
        let sweep = if total > 0.0 { (val / total) * 360.0 } else { 0.0 };
        let pct = if total > 0.0 { (val / total) * 100.0 } else { 0.0 };
        let val_lbl = format!("{:.1}%", pct);

        // Generate curved annular sector polygon points starting from 12 o'clock (-90 degrees)
        let a0 = (cur_angle - 90.0).to_radians();
        let a1 = (cur_angle + sweep - 90.0).to_radians();
        let steps = (sweep.ceil() as usize / 3).clamp(8, 64);

        let mut points = Vec::with_capacity(steps * 2 + 2);
        // Outer arc (forward)
        for step in 0..=steps {
            let t = a0 + (step as f64 / steps as f64) * (a1 - a0);
            points.push((cx + outer_r * t.cos(), cy + outer_r * t.sin()));
        }
        // Inner arc (reverse)
        for step in (0..=steps).rev() {
            let t = a0 + (step as f64 / steps as f64) * (a1 - a0);
            points.push((cx + inner_r * t.cos(), cy + inner_r * t.sin()));
        }

        // Callout label positioned outside the slice arc
        let mid_angle = (cur_angle + sweep / 2.0 - 90.0).to_radians();
        let callout_r = outer_r + 20.0;
        let lx = cx + callout_r * mid_angle.cos();
        let ly = cy + callout_r * mid_angle.sin();
        let label_rect = [lx - 35.0, ly - 9.0, lx + 35.0, ly + 9.0];

        donut_slices.push(DonutSlice { points, label_rect, label: cat, value_label: val_lbl, series_idx: idx, sweep_degrees: sweep });

        cur_angle += sweep;
    }

    ChartLayout {
        bars: Vec::new(),
        line_markers: Vec::new(),
        line_segments: Vec::new(),
        donut_slices,
        grid_lines: Vec::new(),
        x_tick_labels: Vec::new(),
        value_labels: Vec::new(),
        legend_items,
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
    }
}

#[cfg(test)]
mod tests {
    use super::super::spec::{SeriesSpec, ValueFormat};
    use super::*;

    fn sample_spec(chart_type: ChartType) -> ChartSpec {
        ChartSpec {
            chart_type,
            title_id: "Sample Chart".into(),
            title_en: "Sample Chart EN".into(),
            category_col: "A".into(),
            series: vec![
                SeriesSpec { col: "B".into(), label_id: "Series A".into(), label_en: "Series A EN".into() },
                SeriesSpec { col: "C".into(), label_id: "Series B".into(), label_en: "Series B EN".into() },
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
        Rect { x0: 36.0, y0: 36.0, x1: 500.0, y1: 700.0 }
    }

    #[test]
    fn bar_layout_widths_sum_to_plot_width() {
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Cat 1".into(), "Cat 2".into(), "Cat 3".into()];
        let values = vec![vec![10.0, 20.0, 30.0], vec![15.0, 25.0, 35.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);

        assert_eq!(layout.bars.len(), 6);
        let plot_w = layout.plot_rect[2] - layout.plot_rect[0];
        let cat_w = plot_w / 3.0;

        // Verify group slot widths
        for c in 0..3 {
            let s0 = &layout.bars[c * 2];
            let s1 = &layout.bars[c * 2 + 1];
            assert!(s0.rect[0] >= layout.plot_rect[0] + c as f64 * cat_w);
            assert!(s1.rect[2] <= layout.plot_rect[0] + (c + 1) as f64 * cat_w);
        }
    }

    #[test]
    fn bar_layout_heights_proportional_to_values() {
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["A".into(), "B".into()];
        let values = vec![vec![100.0, 200.0], vec![50.0, 100.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);

        let h_a0 = layout.bars[0].rect[3] - layout.bars[0].rect[1]; // 100
        let h_b0 = layout.bars[2].rect[3] - layout.bars[2].rect[1]; // 200
        assert!((h_b0 - h_a0 * 2.0).abs() < 1e-4, "Height should be proportional to value: {h_b0} vs 2 * {h_a0}");
    }

    #[test]
    fn stacked_layout_does_not_exceed_ceiling() {
        let spec = sample_spec(ChartType::BarStacked);
        let categories = vec!["X".into(), "Y".into()];
        let values = vec![vec![40.0, 80.0], vec![60.0, 120.0]]; // max sum is 200

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);

        assert!(layout.ceiling >= 200.0);
        for bar in &layout.bars {
            assert!(bar.rect[1] >= layout.plot_rect[1] - 1e-4);
            assert!(bar.rect[3] <= layout.plot_rect[3] + 1e-4);
        }
    }

    #[test]
    fn horizontal_bar_swaps_axes() {
        let spec = sample_spec(ChartType::HorizontalBar);
        let categories = vec!["Cat A".into(), "Cat B".into()];
        let values = vec![vec![50.0, 100.0], vec![25.0, 75.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);

        assert_eq!(layout.bars.len(), 4);
        for bar in &layout.bars {
            assert_eq!(bar.rect[0], layout.plot_rect[0], "Horizontal bars must start at left axis");
            assert!(bar.rect[2] > bar.rect[0], "Horizontal bars extend rightwards");
        }
    }

    #[test]
    fn donut_slices_sum_to_360_degrees() {
        let spec = sample_spec(ChartType::Donut);
        let categories = vec!["Slice 1".into(), "Slice 2".into(), "Slice 3".into()];
        let values = vec![vec![25.0, 50.0, 25.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);

        assert_eq!(layout.donut_slices.len(), 3);
        let total_deg: f64 = layout.donut_slices.iter().map(|s| s.sweep_degrees).sum();
        assert!((total_deg - 360.0).abs() < 1e-4, "Donut slices must sum to 360 degrees: {total_deg}");
    }

    #[test]
    fn grid_lines_span_plot_width_and_are_evenly_spaced() {
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Cat".into()];
        let values = vec![vec![100.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);

        assert_eq!(layout.grid_lines.len(), GRID_LINES);
        let plot_w = layout.plot_rect[2] - layout.plot_rect[0];
        for gl in &layout.grid_lines {
            let gl_w = gl.rect[2] - gl.rect[0];
            assert!((gl_w - plot_w).abs() < 1e-4, "Grid line must span plot width");
        }
    }

    #[test]
    fn legend_items_count_matches_series_count() {
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Cat".into()];
        let values = vec![vec![10.0], vec![20.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);

        assert_eq!(layout.legend_items.len(), 2);
        assert_eq!(layout.legend_items[0].label, "Series A");
        assert_eq!(layout.legend_items[1].label, "Series B");
    }

    #[test]
    fn bilingual_legend_uses_display_label() {
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Cat".into()];
        let values = vec![vec![10.0], vec![20.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), true);

        assert_eq!(layout.legend_items[0].label, "Series A / Series A EN");
    }

    #[test]
    fn long_labels_are_marked_for_rotation() {
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Short".into(), "A Very Long Category Name".into()];
        let values = vec![vec![10.0, 20.0], vec![30.0, 40.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false);

        assert_eq!(layout.x_tick_labels.len(), 2);
        assert!(!layout.x_tick_labels[0].rotated);
        assert!(layout.x_tick_labels[1].rotated);
    }

    #[test]
    fn rotated_labels_increase_clearance_to_legend() {
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Short".into(), "A Very Long Category Name".into()];
        let values = vec![vec![10.0, 20.0]];
        let margin = sample_margin();

        let layout = compute_layout(&spec, &categories, &values, margin, false);

        assert!(!layout.legend_items.is_empty());
        let first_leg = &layout.legend_items[0];
        assert!(
            first_leg.swatch_rect[1] >= layout.plot_rect[3] + 80.0,
            "Legend item top ({}) must be at least 80pt below plot baseline ({}) for rotated labels",
            first_leg.swatch_rect[1],
            layout.plot_rect[3]
        );

        let swatch_cy = (first_leg.swatch_rect[1] + first_leg.swatch_rect[3]) / 2.0;
        let label_cy = (first_leg.label_rect[1] + first_leg.label_rect[3]) / 2.0;
        assert!((swatch_cy - label_cy).abs() < 1e-4, "Legend swatch center y ({swatch_cy}) must match label center y ({label_cy})");
    }

    #[test]
    fn single_series_chart_legend_takes_category_items() {
        let mut spec = sample_spec(ChartType::BarClustered);
        spec.series.truncate(1);
        let categories = vec!["Cat Alpha".into(), "Cat Beta".into(), "Cat Gamma".into()];
        let values = vec![vec![10.0, 20.0, 30.0]];
        let margin = sample_margin();

        let layout = compute_layout(&spec, &categories, &values, margin, false);

        assert_eq!(layout.legend_items.len(), 3, "Legend must have 3 items for the 3 categories");
        assert_eq!(layout.legend_items[0].label, "Cat Alpha");
        assert_eq!(layout.legend_items[1].label, "Cat Beta");
        assert_eq!(layout.legend_items[2].label, "Cat Gamma");
        assert_eq!(layout.legend_items[0].series_idx, 0);
        assert_eq!(layout.legend_items[1].series_idx, 1);
        assert_eq!(layout.legend_items[2].series_idx, 2);

        // Bars should also have category color indexes matching the legend
        assert_eq!(layout.bars.len(), 3);
        assert_eq!(layout.bars[0].series_idx, 0);
        assert_eq!(layout.bars[1].series_idx, 1);
        assert_eq!(layout.bars[2].series_idx, 2);
    }
}
