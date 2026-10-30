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

/// Summary metrics box displayed at the bottom of a Population Pyramid.
#[derive(Debug, Clone, PartialEq)]
pub struct PyramidSummary {
    pub rect: [f64; 4],
    pub total_male: f64,
    pub total_female: f64,
    pub total_combined: f64,
    pub male_label: String,
    pub female_label: String,
    pub combined_label: String,
    pub axis_y: f64,
    pub ceiling: f64,
    pub max_bar_w: f64,
    pub center_left: f64,
    pub center_right: f64,
}

/// Complete geometry layout for a Gauge / Speedometer chart.
#[derive(Debug, Clone, PartialEq)]
pub struct GaugeLayout {
    pub track_points: Vec<(f64, f64)>,
    pub value_points: Vec<(f64, f64)>,
    pub needle_points: Vec<(f64, f64)>,
    pub pivot_rect: [f64; 4],
    pub readout_rect: [f64; 4],
    pub readout_text: String,
    pub subtitle_rect: [f64; 4],
    pub subtitle_text: String,
    pub min_rect: [f64; 4],
    pub min_text: String,
    pub max_rect: [f64; 4],
    pub max_text: String,
}

/// A single mapped region on a Choropleth Map chart.
#[derive(Debug, Clone, PartialEq)]
pub struct MapRegionLayout {
    /// Bounding/coordinate polygon rings in page points.
    pub polygons: Vec<Vec<(f64, f64)>>,
    /// Fill color hex string (e.g. `"#0072BC"`).
    pub color: String,
    /// Stroke color hex string (e.g. `"#FFFFFF"`).
    pub stroke_color: String,
    /// Stroke weight in points.
    pub stroke_weight: f64,
    /// Region name (e.g. village `nmdesa`).
    pub name: String,
    /// Formatted value string.
    pub value_str: String,
    /// Numeric value.
    pub value: f64,
    /// Geographic centroid in page coordinates.
    pub centroid: (f64, f64),
}

/// Legend item for Choropleth maps.
#[derive(Debug, Clone, PartialEq)]
pub struct MapLegendEntry {
    pub swatch_rect: [f64; 4],
    pub color: String,
    pub label: String,
    pub label_rect: [f64; 4],
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
    /// Gauge layout data if archetype is `Gauge`.
    pub gauge: Option<GaugeLayout>,
    /// Summary cards if archetype is `PopulationPyramid`.
    pub pyramid_summary: Option<PyramidSummary>,
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
    /// Mapped regions for `Map` (Choropleth) charts.
    pub map_regions: Vec<MapRegionLayout>,
    /// Map legend entries for `Map` charts.
    pub map_legend: Vec<MapLegendEntry>,
}

/// Computes the complete pixel-precise layout for a chart.
///
/// Fails only for `Map` charts whose GeoJSON source cannot be loaded, so callers can
/// report the problem instead of silently drawing a different map.
pub fn compute_layout(
    spec: &ChartSpec,
    categories: &[String],
    series_values: &[Vec<f64>],
    margin: Rect,
    bilingual: bool,
) -> Result<ChartLayout, String> {
    let title_h = if bilingual && !spec.title_en.is_empty() {
        56.0
    } else if !spec.title_id.is_empty() {
        44.0
    } else {
        0.0
    };
    let top_offset = if title_h > 0.0 { title_h + 8.0 } else { 0.0 };

    if spec.chart_type == ChartType::Map {
        let plot_x0 = margin.x0;
        let plot_x1 = margin.x1;
        let plot_y0 = margin.y0 + top_offset + 10.0;
        let plot_y1 = margin.y1;
        let plot_rect = [plot_x0, plot_y0, plot_x1, plot_y1];
        return compute_map_layout(spec, categories, series_values, plot_rect, bilingual);
    }

    let is_donut = matches!(spec.chart_type, ChartType::Donut);
    let is_gauge = matches!(spec.chart_type, ChartType::Gauge);
    let is_pyramid = matches!(spec.chart_type, ChartType::PopulationPyramid);
    let is_hbar = matches!(spec.chart_type, ChartType::HorizontalBar);
    let any_rotated = !is_hbar && !is_donut && !is_gauge && !is_pyramid && categories.iter().any(|c| c.chars().count() > MAX_LABEL_CHARS);
    let left_reserve = if is_donut || is_gauge || is_pyramid {
        0.0
    } else if is_hbar {
        HBAR_CATEGORY_W
    } else {
        AXIS_LABEL_W
    };
    let bottom_axis_h = if is_donut || is_gauge || is_pyramid {
        0.0
    } else if is_hbar {
        16.0
    } else if any_rotated {
        64.0
    } else {
        AXIS_LABEL_H + 4.0
    };
    let bottom_gap = if is_pyramid {
        0.0
    } else if any_rotated {
        16.0
    } else {
        4.0
    };

    let use_category_legend = is_donut || (spec.series.len() <= 1 && !is_gauge && !is_pyramid);

    let legend_rows = if is_gauge || is_pyramid {
        0
    } else if spec.show_legend {
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
    let plot_x1 = if is_pyramid { margin.x1 } else { (margin.x1 - 12.0).max(plot_x0 + 10.0) };
    let plot_y0 = if is_pyramid { margin.y0 + top_offset + 32.0 } else { margin.y0 + top_offset + 10.0 };
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
        ChartType::Gauge => series_values.first().and_then(|s| s.first()).copied().unwrap_or(0.0),
        ChartType::PopulationPyramid => {
            let mut max_v = 0.0f64;
            for s in series_values.iter().take(2) {
                for &v in s {
                    if v > max_v {
                        max_v = v;
                    }
                }
            }
            max_v
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

    let ceiling = if let Some(override_max) = spec.max_value_override {
        override_max.max(raw_max).max(1.0)
    } else if matches!(spec.chart_type, ChartType::Gauge) {
        if matches!(spec.value_format, super::spec::ValueFormat::Percent) { 100.0 } else { compute_nice_ceiling(raw_max.max(1.0)) }
    } else if matches!(spec.chart_type, ChartType::PopulationPyramid) {
        compute_pyramid_ceiling(raw_max)
    } else {
        compute_nice_ceiling(raw_max)
    };

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
        if is_pyramid {
            let leg_y_start = (margin.y1 - legend_h + 4.0).max(plot_y1 + bottom_axis_h + bottom_gap);
            let items_per_row = 2;
            let col_w = (margin.x1 - margin.x0) / items_per_row as f64;
            let male_label = spec.series.first().map(|s| s.display_label(bilingual)).unwrap_or_else(|| "Laki-laki (Male)".into());
            let female_label = spec.series.get(1).map(|s| s.display_label(bilingual)).unwrap_or_else(|| "Perempuan (Female)".into());

            for (s_idx, label) in [male_label, female_label].into_iter().enumerate() {
                let item_x = margin.x0 + s_idx as f64 * col_w + (col_w - 140.0).max(0.0) / 2.0;
                let item_y = leg_y_start;
                let swatch_y0 = item_y + (LEGEND_ROW_H - LEGEND_SWATCH_H) / 2.0;
                let swatch_rect = [item_x, swatch_y0, item_x + LEGEND_SWATCH_W, swatch_y0 + LEGEND_SWATCH_H];
                let label_rect = [item_x + LEGEND_SWATCH_W + 6.0, item_y, item_x + 140.0, item_y + LEGEND_ROW_H];
                legend_items.push(LegendItem { swatch_rect, label_rect, label, series_idx: s_idx });
            }
        } else if use_category_legend && !categories.is_empty() {
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
    Ok(match spec.chart_type {
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
        ChartType::PopulationPyramid => {
            layout_population_pyramid(spec, categories, series_values, plot_rect, ceiling, bilingual, legend_items, axis_baseline, axis_left)
        }
        ChartType::Gauge => layout_gauge(spec, categories, series_values, plot_rect, ceiling, bilingual, legend_items, axis_baseline, axis_left),
        // Map charts return above; this arm keeps the match exhaustive.
        ChartType::Map => return compute_map_layout(spec, categories, series_values, plot_rect, bilingual),
    })
}

/// Compute nice rounded ceiling specifically for 4-interval population pyramids.
/// E.g. raw_max 7940 -> ceiling 8000 (step 2000: 0, 2k, 4k, 6k, 8k);
///      raw_max 11240 -> ceiling 12000 (step 3000: 0, 3k, 6k, 9k, 12k).
pub fn compute_pyramid_ceiling(raw: f64) -> f64 {
    if raw <= 0.0 {
        return 4000.0;
    }
    let raw_step = raw / 4.0;
    let magnitude = 10.0f64.powf(raw_step.log10().floor());
    let normalized = raw_step / magnitude;

    // Clean step multipliers for 4-interval axis:
    // (1.0 -> 4k, 1.25 -> 5k, 1.5 -> 6k, 2.0 -> 8k, 2.5 -> 10k, 3.0 -> 12k, 4.0 -> 16k, 5.0 -> 20k, 6.0 -> 24k, 7.5 -> 30k, 8.0 -> 32k, 10.0 -> 40k)
    const NICE_STEPS: [f64; 12] = [1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 7.5, 8.0, 10.0];
    let step_mult = NICE_STEPS.iter().copied().find(|&s| s >= normalized - 1e-9).unwrap_or(10.0);
    let step = step_mult * magnitude;
    step * 4.0
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
        gauge: None,
        pyramid_summary: None,
        grid_lines,
        x_tick_labels,
        value_labels,
        legend_items,
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
        map_regions: Vec::new(),
        map_legend: Vec::new(),
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
        gauge: None,
        pyramid_summary: None,
        grid_lines,
        x_tick_labels,
        value_labels,
        legend_items,
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
        map_regions: Vec::new(),
        map_legend: Vec::new(),
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
        gauge: None,
        pyramid_summary: None,
        grid_lines,
        x_tick_labels,
        value_labels,
        legend_items,
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
        map_regions: Vec::new(),
        map_legend: Vec::new(),
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
        gauge: None,
        pyramid_summary: None,
        grid_lines,
        x_tick_labels,
        value_labels,
        legend_items,
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
        map_regions: Vec::new(),
        map_legend: Vec::new(),
    }
}

fn layout_population_pyramid(
    spec: &ChartSpec,
    categories: &[String],
    series_values: &[Vec<f64>],
    plot_rect: [f64; 4],
    ceiling: f64,
    bilingual: bool,
    _legend_items: Vec<LegendItem>,
    axis_baseline: [f64; 4],
    axis_left: [f64; 4],
) -> ChartLayout {
    let mut bars = Vec::new();
    let mut x_tick_labels = Vec::new();
    let mut value_labels = Vec::new();
    let mut grid_lines = Vec::new();

    let cx = (plot_rect[0] + plot_rect[2]) / 2.0;
    let center_col_w = 48.0f64;
    let val_lbl_w = 38.0f64;
    let center_left = cx - center_col_w / 2.0;
    let center_right = cx + center_col_w / 2.0;

    let max_bar_w = (center_left - (plot_rect[0] + val_lbl_w + 4.0)).max(30.0);

    // Reserve bottom space for baseline axis, ticks, title and 3-column Summary Box
    let summary_h = 50.0f64;
    let axis_gap = 14.0f64;
    let title_gap = 14.0f64;
    let box_gap = 8.0f64;
    let bottom_reserve = axis_gap + title_gap + box_gap + summary_h;
    let pyramid_bottom = (plot_rect[3] - bottom_reserve).max(plot_rect[1] + 50.0);

    let cat_count = categories.len().max(1);
    let avail_bars_h = (pyramid_bottom - plot_rect[1]).max(40.0);
    // Natural compact row height: 14pt min up to 26pt max
    let row_h = (avail_bars_h / cat_count as f64).clamp(14.0, 26.0);
    let bar_gap = 2.0f64;
    let bar_h = (row_h - bar_gap).max(4.0);

    // Position pyramid bottom right after the bars
    let actual_bars_h = cat_count as f64 * row_h;
    let pyramid_bottom = plot_rect[1] + actual_bars_h;

    let male_vals = series_values.first().map(Vec::as_slice).unwrap_or(&[]);
    let female_vals = series_values.get(1).map(Vec::as_slice).unwrap_or(&[]);

    let mut total_male = 0.0f64;
    let mut total_female = 0.0f64;

    // Check if table starts with youngest age group (e.g. 0-4 or 00-04)
    // In population pyramids, oldest (75+) sits at top, youngest at bottom.
    let reverse_order = categories.first().is_some_and(|c| {
        let t = c.trim();
        t.starts_with('0') || t.starts_with("1-") || t.starts_with('<')
    });

    for r in 0..cat_count {
        let c_idx = if reverse_order { cat_count - 1 - r } else { r };
        let cat = categories.get(c_idx).cloned().unwrap_or_default();
        let row_y = plot_rect[1] + r as f64 * row_h;
        let y0 = row_y + (row_h - bar_h) / 2.0;
        let y1 = y0 + bar_h;

        // Subtle horizontal grid line every 2 age groups (10-year cohort markers)
        if r % 2 == 1 {
            grid_lines.push(GridLine { rect: [plot_rect[0], row_y + row_h, plot_rect[2], row_y + row_h + 0.5], value: 0.0, label: String::new() });
        }

        // 1. Male bar (Left side extending from center_left towards left)
        let v_m = male_vals.get(c_idx).copied().unwrap_or(0.0);
        total_male += v_m;
        let w_m = if ceiling > 0.0 { ((v_m / ceiling) * max_bar_w).max(0.0) } else { 0.0 };
        let x0_m = center_left - w_m;
        let x1_m = center_left;
        let lbl_m = spec.value_format.format(v_m, bilingual);
        bars.push(BarRect { rect: [x0_m, y0, x1_m, y1], value: v_m, series_idx: 0, category_idx: c_idx, label: lbl_m.clone() });
        if spec.show_value_labels && v_m > 0.0 {
            value_labels.push((0, [x0_m - val_lbl_w - 3.0, y0 - 1.0, x0_m - 3.0, y1 + 1.0], lbl_m));
        }

        // 2. Female bar (Right side extending from center_right towards right)
        let v_f = female_vals.get(c_idx).copied().unwrap_or(0.0);
        total_female += v_f;
        let w_f = if ceiling > 0.0 { ((v_f / ceiling) * max_bar_w).max(0.0) } else { 0.0 };
        let x0_f = center_right;
        let x1_f = center_right + w_f;
        let lbl_f = spec.value_format.format(v_f, bilingual);
        bars.push(BarRect { rect: [x0_f, y0, x1_f, y1], value: v_f, series_idx: 1, category_idx: c_idx, label: lbl_f.clone() });
        if spec.show_value_labels && v_f > 0.0 {
            value_labels.push((1, [x1_f + 3.0, y0 - 1.0, x1_f + val_lbl_w + 3.0, y1 + 1.0], lbl_f));
        }

        // 3. Center Category Label (Age Group)
        let cat_rect = [center_left + 1.0, y0 - 1.0, center_right - 1.0, y1 + 1.0];
        x_tick_labels.push(XTickLabel { rect: cat_rect, text: cat, rotated: false });
    }

    // Summary Card at bottom
    let total_combined = total_male + total_female;
    let summary_y0 = pyramid_bottom + axis_gap + title_gap + box_gap;
    let summary_rect = [plot_rect[0], summary_y0, plot_rect[2], summary_y0 + summary_h];
    let male_label = if bilingual { "TOTAL LAKI-LAKI (MALE)" } else { "TOTAL LAKI-LAKI" }.to_string();
    let female_label = if bilingual { "TOTAL PEREMPUAN (FEMALE)" } else { "TOTAL PEREMPUAN" }.to_string();
    let combined_label = if bilingual { "TOTAL PENDUDUK (TOTAL POPULATION)" } else { "TOTAL PENDUDUK" }.to_string();

    let pyramid_summary = Some(PyramidSummary {
        rect: summary_rect,
        total_male,
        total_female,
        total_combined,
        male_label,
        female_label,
        combined_label,
        axis_y: pyramid_bottom,
        ceiling,
        max_bar_w,
        center_left,
        center_right,
    });

    ChartLayout {
        bars,
        line_markers: Vec::new(),
        line_segments: Vec::new(),
        donut_slices: Vec::new(),
        gauge: None,
        pyramid_summary,
        grid_lines,
        x_tick_labels,
        value_labels,
        legend_items: Vec::new(), // No redundant bottom legend
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
        map_regions: Vec::new(),
        map_legend: Vec::new(),
    }
}

fn layout_gauge(
    spec: &ChartSpec,
    categories: &[String],
    series_values: &[Vec<f64>],
    plot_rect: [f64; 4],
    ceiling: f64,
    bilingual: bool,
    _legend_items: Vec<LegendItem>,
    axis_baseline: [f64; 4],
    axis_left: [f64; 4],
) -> ChartLayout {
    let plot_w = plot_rect[2] - plot_rect[0];
    let plot_h = plot_rect[3] - plot_rect[1];
    let cx = (plot_rect[0] + plot_rect[2]) / 2.0;

    let outer_r = (plot_w.min(plot_h * 1.6) / 2.0 * 0.76).clamp(50.0, 150.0);
    let inner_r = outer_r * 0.70;
    let cy = plot_rect[1] + outer_r + 20.0;

    let val = series_values.first().and_then(|s| s.first()).copied().unwrap_or(0.0);
    let max_val = if ceiling > 0.0 { ceiling } else { 100.0 };
    let frac = (val / max_val).clamp(0.0, 1.0);

    // 1. Background track: 180° semi-circle from 180° (pi) to 0° (0)
    let track_steps = 48;
    let mut track_points = Vec::with_capacity(track_steps * 2 + 2);
    for s in 0..=track_steps {
        let t = std::f64::consts::PI - (s as f64 / track_steps as f64) * std::f64::consts::PI;
        track_points.push((cx + outer_r * t.cos(), cy - outer_r * t.sin()));
    }
    for s in (0..=track_steps).rev() {
        let t = std::f64::consts::PI - (s as f64 / track_steps as f64) * std::f64::consts::PI;
        track_points.push((cx + inner_r * t.cos(), cy - inner_r * t.sin()));
    }

    // 2. Value arc: sweeps clockwise from 180° to (180 - frac * 180)
    let val_steps = ((frac * track_steps as f64).ceil() as usize).clamp(4, track_steps);
    let mut value_points = Vec::with_capacity(val_steps * 2 + 2);
    let end_angle = std::f64::consts::PI - frac * std::f64::consts::PI;
    for s in 0..=val_steps {
        let t = std::f64::consts::PI - (s as f64 / val_steps as f64) * (std::f64::consts::PI - end_angle);
        value_points.push((cx + outer_r * t.cos(), cy - outer_r * t.sin()));
    }
    for s in (0..=val_steps).rev() {
        let t = std::f64::consts::PI - (s as f64 / val_steps as f64) * (std::f64::consts::PI - end_angle);
        value_points.push((cx + inner_r * t.cos(), cy - inner_r * t.sin()));
    }

    // 3. Needle pointer: triangular needle from center pivot pointing to value angle
    let needle_len = outer_r * 0.88;
    let needle_tip = (cx + needle_len * end_angle.cos(), cy - needle_len * end_angle.sin());
    let perp_angle = end_angle + std::f64::consts::FRAC_PI_2;
    let base_w = 4.0;
    let needle_b1 = (cx + base_w * perp_angle.cos(), cy - base_w * perp_angle.sin());
    let needle_b2 = (cx - base_w * perp_angle.cos(), cy + base_w * perp_angle.sin());
    let needle_points = vec![needle_b1, needle_tip, needle_b2];

    // 4. Pivot cap (circle radius 7pt)
    let pivot_rect = [cx - 7.0, cy - 7.0, cx + 7.0, cy + 7.0];

    // 5. Readout text
    let readout_text = spec.value_format.format(val, bilingual);
    let readout_rect = [cx - 60.0, cy + 10.0, cx + 60.0, cy + 32.0];

    // 6. Subtitle text (Category name or Series label)
    let subtitle_text = categories.first().cloned().unwrap_or_else(|| spec.series.first().map(|s| s.display_label(bilingual)).unwrap_or_default());
    let subtitle_rect = [cx - 80.0, cy + 33.0, cx + 80.0, cy + 47.0];

    // 7. Min / Max labels
    let min_text = spec.value_format.format(0.0, bilingual);
    let max_text = spec.value_format.format(max_val, bilingual);
    let min_rect = [cx - outer_r - 20.0, cy + 4.0, cx - inner_r + 4.0, cy + 18.0];
    let max_rect = [cx + inner_r - 4.0, cy + 4.0, cx + outer_r + 20.0, cy + 18.0];

    let gauge = Some(GaugeLayout {
        track_points,
        value_points,
        needle_points,
        pivot_rect,
        readout_rect,
        readout_text,
        subtitle_rect,
        subtitle_text,
        min_rect,
        min_text,
        max_rect,
        max_text,
    });

    ChartLayout {
        bars: Vec::new(),
        line_markers: Vec::new(),
        line_segments: Vec::new(),
        donut_slices: Vec::new(),
        gauge,
        pyramid_summary: None,
        grid_lines: Vec::new(),
        x_tick_labels: Vec::new(),
        value_labels: Vec::new(),
        legend_items: Vec::new(),
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
        map_regions: Vec::new(),
        map_legend: Vec::new(),
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
        gauge: None,
        pyramid_summary: None,
        grid_lines: Vec::new(),
        x_tick_labels: Vec::new(),
        value_labels: Vec::new(),
        legend_items,
        axis_baseline,
        axis_left,
        plot_rect,
        ceiling,
        map_regions: Vec::new(),
        map_legend: Vec::new(),
    }
}

/// Loads the GeoJSON source selected for a Map chart, reporting why it cannot be used.
fn load_chart_map(spec: &ChartSpec) -> Result<super::geojson::GeoMap, String> {
    let path = spec.geojson_path.as_deref().map(str::trim).unwrap_or("");
    if path.is_empty() {
        return Err("Map chart has no GeoJSON stock map or file selected".into());
    }
    super::stock_maps::load_chart_geo_map(path)
}

fn compute_map_layout(
    spec: &ChartSpec,
    categories: &[String],
    series_values: &[Vec<f64>],
    plot_rect: [f64; 4],
    bilingual: bool,
) -> Result<ChartLayout, String> {
    let geo_map = load_chart_map(spec)?;

    let s_vals = series_values.first().map(Vec::as_slice).unwrap_or(&[]);
    let mut min_val = f64::INFINITY;
    let mut max_val = f64::NEG_INFINITY;

    let mut val_map = std::collections::HashMap::new();
    for (idx, &val) in s_vals.iter().enumerate() {
        if let Some(cat) = categories.get(idx) {
            let key = cat.trim().to_uppercase();
            val_map.insert(key, val);
            min_val = min_val.min(val);
            max_val = max_val.max(val);
        }
    }

    if !min_val.is_finite() || !max_val.is_finite() {
        min_val = 0.0;
        max_val = 100.0;
    } else if (max_val - min_val).abs() < 1e-6 {
        max_val = min_val + 1.0;
    }

    if let Some(override_max) = spec.max_value_override.filter(|v| v.is_finite()) {
        max_val = max_val.max(override_max);
    }

    let (map_rect, legend_rect) = if spec.show_legend {
        (
            designcraft_geom::Rect { x0: plot_rect[0], y0: plot_rect[1], x1: plot_rect[2], y1: (plot_rect[3] - 32.0).max(plot_rect[1] + 20.0) },
            Some([plot_rect[0], plot_rect[3] - 28.0, plot_rect[2], plot_rect[3]]),
        )
    } else {
        (designcraft_geom::Rect { x0: plot_rect[0], y0: plot_rect[1], x1: plot_rect[2], y1: plot_rect[3] }, None)
    };

    let projected = geo_map.project_features(map_rect);
    let mut map_regions = Vec::new();

    for pf in projected {
        let feat = match geo_map.features.get(pf.feature_idx) {
            Some(f) => f,
            None => continue,
        };

        let feat_name = feat.nmdesa().or_else(|| feat.nmkec()).unwrap_or("");
        let norm_name = feat_name.trim().to_uppercase();

        let val_opt = val_map.get(&norm_name).copied();
        let (min_hex, max_hex) =
            super::palette::MAP_COLOR_PRESETS.get(spec.map_palette_idx).map(|(_, min_c, max_c)| (*min_c, *max_c)).unwrap_or(("#E1F5FE", "#0072BC"));

        let (val, fill_color) = if let Some(v) = val_opt {
            let t = ((v - min_val) / (max_val - min_val)).clamp(0.0, 1.0);
            let c = super::palette::interpolate_color(min_hex, max_hex, t);
            (v, c)
        } else {
            (0.0, "#F1F5F9".to_string())
        };

        let value_str = if val_opt.is_some() { spec.value_format.format(val, bilingual) } else { String::new() };

        map_regions.push(MapRegionLayout {
            polygons: pf.polygons,
            color: fill_color,
            stroke_color: "#FFFFFF".to_string(),
            stroke_weight: 0.75,
            name: feat_name.to_string(),
            value_str,
            value: val,
            centroid: pf.centroid,
        });
    }

    let mut map_legend = Vec::new();
    if let Some(l_rect) = legend_rect {
        let steps = 5;
        let total_w = l_rect[2] - l_rect[0];
        let swatch_w = 40.0f64;
        let swatch_h = 10.0f64;
        let gap = 12.0f64;
        let full_legend_w = steps as f64 * swatch_w + (steps - 1) as f64 * gap;
        let start_x = l_rect[0] + (total_w - full_legend_w).max(0.0) * 0.5;
        let swatch_y = l_rect[1] + 2.0;

        for i in 0..steps {
            let frac = i as f64 / (steps - 1) as f64;
            let val = min_val + frac * (max_val - min_val);
            let (min_hex, max_hex) = super::palette::MAP_COLOR_PRESETS
                .get(spec.map_palette_idx)
                .map(|(_, min_c, max_c)| (*min_c, *max_c))
                .unwrap_or(("#E1F5FE", "#0072BC"));
            let color = super::palette::interpolate_color(min_hex, max_hex, frac);
            let sx0 = start_x + i as f64 * (swatch_w + gap);
            let s_rect = [sx0, swatch_y, sx0 + swatch_w, swatch_y + swatch_h];
            let lbl_rect = [sx0 - 4.0, swatch_y + swatch_h + 2.0, sx0 + swatch_w + 4.0, swatch_y + swatch_h + 12.0];
            let label = spec.value_format.format(val, bilingual);

            map_legend.push(MapLegendEntry { swatch_rect: s_rect, color, label, label_rect: lbl_rect });
        }
    }

    Ok(ChartLayout {
        bars: Vec::new(),
        line_markers: Vec::new(),
        line_segments: Vec::new(),
        donut_slices: Vec::new(),
        gauge: None,
        pyramid_summary: None,
        grid_lines: Vec::new(),
        x_tick_labels: Vec::new(),
        value_labels: Vec::new(),
        legend_items: Vec::new(),
        axis_baseline: [plot_rect[0], plot_rect[3], plot_rect[2], plot_rect[3]],
        axis_left: [plot_rect[0], plot_rect[1], plot_rect[0], plot_rect[3]],
        plot_rect,
        ceiling: max_val,
        map_regions,
        map_legend,
    })
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
            geojson_path: None,
            map_palette_idx: 0,
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

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();

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

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();

        let h_a0 = layout.bars[0].rect[3] - layout.bars[0].rect[1]; // 100
        let h_b0 = layout.bars[2].rect[3] - layout.bars[2].rect[1]; // 200
        assert!((h_b0 - h_a0 * 2.0).abs() < 1e-4, "Height should be proportional to value: {h_b0} vs 2 * {h_a0}");
    }

    #[test]
    fn stacked_layout_does_not_exceed_ceiling() {
        let spec = sample_spec(ChartType::BarStacked);
        let categories = vec!["X".into(), "Y".into()];
        let values = vec![vec![40.0, 80.0], vec![60.0, 120.0]]; // max sum is 200

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();

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

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();

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

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();

        assert_eq!(layout.donut_slices.len(), 3);
        let total_deg: f64 = layout.donut_slices.iter().map(|s| s.sweep_degrees).sum();
        assert!((total_deg - 360.0).abs() < 1e-4, "Donut slices must sum to 360 degrees: {total_deg}");
    }

    #[test]
    fn grid_lines_span_plot_width_and_are_evenly_spaced() {
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Cat".into()];
        let values = vec![vec![100.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();

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

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();

        assert_eq!(layout.legend_items.len(), 2);
        assert_eq!(layout.legend_items[0].label, "Series A");
        assert_eq!(layout.legend_items[1].label, "Series B");
    }

    #[test]
    fn bilingual_legend_uses_display_label() {
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Cat".into()];
        let values = vec![vec![10.0], vec![20.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), true).unwrap();

        assert_eq!(layout.legend_items[0].label, "Series A / Series A EN");
    }

    #[test]
    fn long_labels_are_marked_for_rotation() {
        let spec = sample_spec(ChartType::BarClustered);
        let categories = vec!["Short".into(), "A Very Long Category Name".into()];
        let values = vec![vec![10.0, 20.0], vec![30.0, 40.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();

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

        let layout = compute_layout(&spec, &categories, &values, margin, false).unwrap();

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

        let layout = compute_layout(&spec, &categories, &values, margin, false).unwrap();

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

    #[test]
    fn population_pyramid_creates_left_and_right_bars_and_summary() {
        let spec = sample_spec(ChartType::PopulationPyramid);
        let categories = vec!["00-04".into(), "05-09".into(), "10-14".into()];
        let values = vec![
            vec![1000.0, 1500.0, 2000.0], // Male
            vec![950.0, 1450.0, 1950.0],  // Female
        ];
        let margin = sample_margin();

        let layout = compute_layout(&spec, &categories, &values, margin, false).unwrap();

        assert_eq!(layout.bars.len(), 6, "Must emit 3 male and 3 female bars");
        let male_bars: Vec<_> = layout.bars.iter().filter(|b| b.series_idx == 0).collect();
        let female_bars: Vec<_> = layout.bars.iter().filter(|b| b.series_idx == 1).collect();
        assert_eq!(male_bars.len(), 3);
        assert_eq!(female_bars.len(), 3);

        // Male bars must be to the left of the center column, female bars to the right
        for b in &male_bars {
            assert!(b.rect[2] <= female_bars[0].rect[0]);
        }

        // Summary must compute total population accurately
        assert!(layout.pyramid_summary.is_some());
        let sum = layout.pyramid_summary.as_ref().unwrap();
        assert_eq!(sum.total_male, 4500.0);
        assert_eq!(sum.total_female, 4350.0);
        assert_eq!(sum.total_combined, 8850.0);

        // Verify fatter bars and closer spacing
        let gap = male_bars[1].rect[1] - male_bars[0].rect[3];
        assert!(gap <= 2.5, "Inter-bar vertical gap must be close (<= 2.5pt), got {gap}");
        assert!(male_bars[0].rect[3] - male_bars[0].rect[1] >= 20.0, "Bars must be thick/fat");

        // Population pyramid must not have redundant bottom legend
        assert!(layout.legend_items.is_empty(), "Population pyramid must not have redundant bottom legend");
    }

    #[test]
    fn gauge_chart_creates_semicircular_geometry_and_needle() {
        let mut spec = sample_spec(ChartType::Gauge);
        spec.value_format = ValueFormat::Percent;
        let categories = vec!["Pencapaian Target".into()];
        let values = vec![vec![75.0]];
        let margin = sample_margin();

        let layout = compute_layout(&spec, &categories, &values, margin, false).unwrap();

        assert!(layout.gauge.is_some());
        let g = layout.gauge.as_ref().unwrap();
        assert!(!g.track_points.is_empty(), "Track points must form an arc");
        assert!(!g.value_points.is_empty(), "Value points must form an arc");
        assert_eq!(g.needle_points.len(), 3, "Needle must be a triangular polygon");
        assert_eq!(g.readout_text, "75.0%");
        assert_eq!(g.subtitle_text, "Pencapaian Target");
    }

    #[test]
    fn test_compute_pyramid_ceiling_rounds_to_tight_4_step_intervals() {
        // User example 1: max 7940 -> ceiling 8000 (0, 2000, 4000, 6000, 8000)
        assert_eq!(compute_pyramid_ceiling(7940.0), 8000.0);

        // User example 2: max 11240 -> ceiling 12000 (0, 3000, 6000, 9000, 12000)
        assert_eq!(compute_pyramid_ceiling(11240.0), 12000.0);

        // Other common BPS scale thresholds
        assert_eq!(compute_pyramid_ceiling(8000.0), 8000.0);
        assert_eq!(compute_pyramid_ceiling(8001.0), 10000.0);
        assert_eq!(compute_pyramid_ceiling(4500.0), 5000.0);
        assert_eq!(compute_pyramid_ceiling(5500.0), 6000.0);
        assert_eq!(compute_pyramid_ceiling(15500.0), 16000.0);
        assert_eq!(compute_pyramid_ceiling(19500.0), 20000.0);
    }

    #[test]
    fn map_layout_matches_village_values_and_honours_max_override() {
        let mut spec = sample_spec(ChartType::Map);
        spec.geojson_path = Some("PANGKALAN".into());
        spec.max_value_override = Some(200.0);
        let categories = vec!["CINTAASIH".into(), "MULANGSARI".into()];
        let values = vec![vec![10.0, 20.0]];

        let layout = compute_layout(&spec, &categories, &values, sample_margin(), false).unwrap();

        assert!(!layout.map_regions.is_empty(), "Pangkalan map must produce regions");
        assert!((layout.ceiling - 200.0).abs() < 1e-9, "max override must raise the map legend ceiling");
        assert!(
            layout.map_regions.iter().any(|r| r.value_str == "10"),
            "village values must be matched and formatted: {:?}",
            layout.map_regions.iter().map(|r| (&r.name, &r.value_str)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn map_layout_errors_for_unknown_geojson() {
        let mut spec = sample_spec(ChartType::Map);
        spec.geojson_path = Some("NO_SUCH_STOCK_MAP".into());

        let res = compute_layout(&spec, &["A".into()], &[vec![1.0]], sample_margin(), false);

        assert!(res.is_err(), "an unknown map must fail instead of silently drawing Pangkalan");
    }

    #[test]
    fn map_layout_requires_a_map_source() {
        let spec = sample_spec(ChartType::Map);

        let res = compute_layout(&spec, &["A".into()], &[vec![1.0]], sample_margin(), false);

        assert!(res.is_err(), "a Map chart without a GeoJSON source must fail");
    }
}
