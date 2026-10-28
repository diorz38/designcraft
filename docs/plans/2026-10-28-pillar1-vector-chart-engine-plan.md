# Implementation Plan: Pillar 1 — Native Statistical Infographics & Vector Chart Engine

- **Branch:** `feature/pillar1-vector-chart-engine`
- **Date:** 2026-10-28
- **Authors:** diorz38 & Antigravity Agent
- **Milestone:** M4 (from the extension roadmap)
- **Effort estimate:** 5–7 days across 5 phases
- **Status:** Planning — implementation not started

---

## 0. Context & Current State

### What exists today (partial)

| Component | File | What it does |
|---|---|---|
| `is_chart_sheet(name)` | `excel.rs:882` | Recognises `C01`, `C02_01`, `c03_02` sheet names |
| `parse_chart_sheet_with_template(sheet)` | `excel.rs:1055` | Reads `title;…` / `file;…` / `template;…` rows → `(type, num, title, file, tpl)` |
| `parse_chart_sheet(sheet)` | `excel.rs:1081` | Backward-compat 4-tuple wrapper |
| Chart page layout | `pages.rs:1489` | Bilingual title box + raster image frame from `file;` row |
| Generation summary | `pages.rs:2012` | Reports `N chart image(s) placed` |
| Tests | `pages.rs:3270`, `excel.rs:2317` | Bilingual title parsing, chart page creation |

### What does NOT exist

- Vector chart schema rows (`chart_type`, `category_col`, `series_*_col`, `data_start_row`, `data_end_row`)
- `ChartSpec` data structure
- Coordinate mapper (data → page points)
- Vector emitters for bar / horizontal-bar / line / donut charts
- Axis baseline, grid ticks, tick labels
- Legend boxes
- DIV palette → chart series color binding
- `charts/` module in `crates/ui-egui/src/panels/pubgen/`

### How pubgen draws shapes (the pattern to follow)

All geometry is driven by engine commands through `app.run(…)`:

```rust
// Rectangle frame
app.run("frame.create", json!({
    "spread": spread_idx,
    "rect": [x0, y0, x1, y1],
    "content": "unassigned",
    "shape": "rectangle"
}))?;
app.run("object.fill", json!({"swatch": "#0088D2"}))?;
app.run("object.stroke", json!({"swatch": "[None]"}))?;
```

The chart engine will use this same API. No direct `Content::Shape` mutation is needed — `frame.create` with `shape: "rectangle"` | `"ellipse"` covers bars, grid lines, donut arcs (approximated as stacked arc frames), and legend swatches.

---

## 1. Phase 1 — Extended Sheet Schema & `ChartSpec` Parser

**Goal:** Parse a fully-specified `CXX_XX` sheet into a typed `ChartSpec` struct that carries everything needed for vector rendering. This phase is **pure data parsing** — no rendering.

### 1.1 New Sheet Schema

Extend the `CXX_XX` sheet format. Rows are key-value pairs (`col 0 = key`, `col 1 = value`). All existing keys (`title`, `file`, `template`) remain valid for backward compatibility — a sheet with only those keys continues to work exactly as before (raster image fallback).

New optional rows:

| Row key | Values | Required for vector |
|---|---|---|
| `chart_type` | `bar_clustered`, `bar_stacked`, `horizontal_bar`, `line`, `area`, `donut` | ✓ |
| `title` | `"Grafik 02.01 Judul"` | existing |
| `title_en` | `"Chart 02.01 Title"` | optional |
| `category_col` | `A` — column letter of X-axis labels | ✓ |
| `series_1_col` | `B` | ✓ |
| `series_1_label` | `Laki-laki` | ✓ |
| `series_1_label_en` | `Male` | optional |
| `series_2_col` | `C` | optional |
| `series_2_label` | `Perempuan` | optional |
| `series_2_label_en` | `Female` | optional |
| `series_3_col` | `D` | optional |
| `series_3_label` | `Total` | optional |
| `series_3_label_en` | `Total` | optional |
| `data_start_row` | `2` (1-based row number in the sheet) | ✓ |
| `data_end_row` | `31` | ✓ |
| `value_format` | `number` \| `percent` \| `currency_idr` | optional, default `number` |
| `show_value_labels` | `true` \| `false` | optional, default `true` |
| `show_legend` | `true` \| `false` | optional, default `true` |
| `show_grid` | `true` \| `false` | optional, default `true` |
| `max_value` | `1000.0` — override Y-axis ceiling | optional, auto if absent |

> **Backward compatibility:** A sheet with `file;…` but no `chart_type` key continues to produce a raster image page. A sheet with `chart_type` but also `file` triggers vector rendering; the `file` key is silently ignored with a debug note.

### 1.2 `ChartSpec` Struct

New file: `crates/ui-egui/src/panels/pubgen/charts/spec.rs`

```rust
#[derive(Debug, Clone)]
pub struct SeriesSpec {
    /// Column letter in the data sheet (e.g. "B").
    pub col: String,
    /// Indonesian label.
    pub label_id: String,
    /// English label (empty = same as Indonesian).
    pub label_en: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChartType {
    BarClustered,
    BarStacked,
    HorizontalBar,
    Line,
    Area,
    Donut,
}

#[derive(Debug, Clone)]
pub enum ValueFormat {
    Number,
    Percent,
    CurrencyIdr,
}

#[derive(Debug, Clone)]
pub struct ChartSpec {
    pub chart_type: ChartType,
    pub title_id: String,
    pub title_en: String,
    /// Column letter for category labels (X-axis or pie labels).
    pub category_col: String,
    pub series: Vec<SeriesSpec>,      // 1–3 series
    pub data_start_row: usize,        // 1-based
    pub data_end_row: usize,          // 1-based, inclusive
    pub value_format: ValueFormat,
    pub show_value_labels: bool,
    pub show_legend: bool,
    pub show_grid: bool,
    pub max_value_override: Option<f64>,
}
```

### 1.3 `parse_chart_spec()` Function

New function in `charts/spec.rs`:

```rust
/// Parse a `CXX_XX` sheet into a `ChartSpec` when `chart_type` is present.
/// Returns `None` when the sheet has no `chart_type` row (raster image fallback).
pub fn parse_chart_spec(sheet: &ExcelSheetData) -> Option<ChartSpec>
```

- Scans all rows for key-value pairs (case-insensitive keys, trimmed values).
- Returns `None` (not an error) when `chart_type` is absent.
- Returns `None` with a `tracing::warn!` when `chart_type` is present but required fields are missing.
- Column letters are normalised to uppercase.

### 1.4 Data Extraction Helper

```rust
/// Extract the data matrix from `sheet` for the given spec.
/// Returns `(categories, series_values)` — both `Vec` of the same length.
pub fn extract_chart_data(
    sheet: &ExcelSheetData,
    spec: &ChartSpec,
) -> Result<(Vec<String>, Vec<Vec<f64>>), String>
```

- Reads rows `data_start_row..=data_end_row` (1-based).
- Skips rows where the category cell is empty or equals the column header.
- Parses values with the existing Indonesian/English number parser (`parse_indonesian_number` in `helpers.rs`).
- Returns an `Err(String)` if no data rows could be parsed (surfaces as a generation warning, not a crash).

### 1.5 Tests for Phase 1

File: `charts/spec.rs` (inline `#[cfg(test)]`)

```rust
#[test] fn bar_clustered_spec_parses_correctly()
#[test] fn missing_chart_type_returns_none()
#[test] fn missing_required_field_returns_none_with_warning()
#[test] fn data_extraction_skips_empty_rows()
#[test] fn data_extraction_handles_indonesian_number_format()
#[test] fn data_extraction_handles_percent_values()
```

**Deliverables:** `charts/mod.rs`, `charts/spec.rs` — compiles, tests green, no rendering yet.

---

## 2. Phase 2 — Coordinate Mapper & Color Palette Binding

**Goal:** Given a `ChartSpec`, data, page margin rect, and the active DIV theme palette, compute all pixel-precise coordinates for every chart element. Pure geometry — no engine calls yet.

### 2.1 Layout Model

```
margin_rect
├── title_area      height = title_h (48 or 60 pt — existing logic)
├── chart_area      = margin_rect.inset(top=title_h+8, bottom=legend_h+8, left=axis_label_w, right=0)
│   ├── plot_rect   = chart_area (bars/lines render here)
│   ├── y_axis      left edge of chart_area
│   └── x_axis      bottom edge of chart_area
└── legend_area     height = legend_h (24 pt per row of series)
```

Constants:
```rust
const AXIS_LABEL_W: f64 = 48.0;   // width reserved for Y-axis tick labels
const AXIS_LABEL_H: f64 = 20.0;   // height reserved for X-axis tick labels
const LEGEND_ROW_H: f64 = 18.0;   // height per legend row
const LEGEND_SWATCH_W: f64 = 12.0;
const BAR_GROUP_GAP: f64 = 0.20;  // fraction of group width that is gap
const BAR_SERIES_GAP: f64 = 0.05; // fraction of bar width between bars in group
const GRID_LINES: usize = 5;       // number of horizontal grid lines
const MAX_LABEL_CHARS: usize = 12; // chars before X-label is rotated 45°
```

### 2.2 `ChartLayout` Struct

New file: `charts/layout.rs`

```rust
pub struct BarRect {
    pub rect: [f64; 4],   // [x0, y0, x1, y1]
    pub value: f64,
    pub series_idx: usize,
    pub category_idx: usize,
    pub label: String,    // formatted value string for label frame
}

pub struct GridLine {
    pub rect: [f64; 4],   // 1 pt tall horizontal rect spanning plot width
    pub value: f64,
    pub label: String,    // Y-axis tick label
}

pub struct XTickLabel {
    pub rect: [f64; 4],   // text frame below X-axis
    pub text: String,
    pub rotated: bool,    // true when text > MAX_LABEL_CHARS
}

pub struct LegendItem {
    pub swatch_rect: [f64; 4],
    pub label_rect: [f64; 4],
    pub label: String,
    pub series_idx: usize,
}

pub struct DonutSlice {
    /// Donut slices are rendered as a series of thin rectangle frames arranged
    /// around a centre point to approximate arcs. Each `rect` covers one 1-degree step.
    pub rects: Vec<[f64; 4]>,
    pub label_rect: [f64; 4],
    pub label: String,
    pub value_label: String,
    pub series_idx: usize,  // used for color selection
}

pub struct ChartLayout {
    pub bars: Vec<BarRect>,           // BarClustered / BarStacked / HorizontalBar
    pub lines: Vec<Vec<[f64; 4]>>,   // Line: per-series list of 4-pt square marker rects
    pub line_segments: Vec<Vec<[f64; 4]>>, // 1-pt tall rects connecting markers
    pub donut_slices: Vec<DonutSlice>,
    pub grid_lines: Vec<GridLine>,
    pub x_tick_labels: Vec<XTickLabel>,
    pub value_label_rects: Vec<(usize, [f64; 4], String)>, // (series_idx, rect, text)
    pub legend_items: Vec<LegendItem>,
    pub axis_baseline: [f64; 4],      // bottom horizontal line
    pub axis_left: [f64; 4],          // left vertical line
}
```

### 2.3 `compute_layout()` Function

```rust
pub fn compute_layout(
    spec: &ChartSpec,
    categories: &[String],
    series_values: &[Vec<f64>],
    margin: designcraft_geom::Rect,
    bilingual: bool,
) -> ChartLayout
```

Internal helpers:
- `fn bar_clustered_layout(...)` — groups of bars, one per category, N bars per group
- `fn bar_stacked_layout(...)` — bars stacked; Y-ceiling = max sum
- `fn horizontal_bar_layout(...)` — axes swapped; category labels on Y-axis
- `fn line_layout(...)` — marker dots + connecting segments as thin rects
- `fn donut_layout(...)` — percentage slices using 360 thin-rect approximation
- `fn grid_lines(max_v, plot_rect)` — 5 evenly spaced horizontal lines

### 2.4 Color Palette Binding

New file: `charts/palette.rs`

```rust
/// Resolve the chart palette from the active DIV theme.
/// Series 0 → primary, 1 → secondary, 2 → accent.
/// Falls back to the standard BPS palette when no DIV theme is loaded.
pub fn series_color(theme_colors: &[String], series_idx: usize) -> &str
```

The DIV theme already lives in `PubgenState` as `state.theme_colors: Vec<String>` (populated by `themes.rs`). Pass it through to `compute_layout`.

### 2.5 Tests for Phase 2

```rust
#[test] fn bar_layout_widths_sum_to_plot_width()
#[test] fn bar_layout_heights_proportional_to_values()
#[test] fn stacked_layout_does_not_exceed_ceiling()
#[test] fn horizontal_bar_swaps_axes()
#[test] fn donut_slices_sum_to_360_degrees()
#[test] fn grid_lines_span_plot_width_and_are_evenly_spaced()
#[test] fn legend_items_count_matches_series_count()
```

**Deliverables:** `charts/layout.rs`, `charts/palette.rs` — all coordinate math tested, no engine calls.

---

## 3. Phase 3 — Vector Emitter (Engine Command Layer)

**Goal:** Take a `ChartLayout` + `ChartSpec` + `ChartPalette` and emit it onto a page spread via `app.run(…)` commands. This is the rendering heart of the engine.

### 3.1 Emitter API

New file: `charts/emitter.rs`

```rust
/// Emit a complete vector chart onto the given spread.
/// Returns the count of frames created, or an error string (shown as a
/// generation warning — never panics).
pub fn emit_chart(
    app: &mut DesignApp,
    spread_idx: usize,
    spec: &ChartSpec,
    layout: &ChartLayout,
    palette: &[String],
    bilingual: bool,
) -> Result<usize, String>
```

### 3.2 Emission Order (Back-to-Front)

All elements are emitted in painter's order so later frames appear on top:

```
1. Background fill rect (optional, [Paper] white)
2. Grid lines          → thin rect frames, 0.5 pt stroke, light gray
3. Axis baseline       → 1 pt rect frame, dark gray fill
4. Axis left line      → 1 pt rect frame, dark gray fill
5. Bar / slice frames  → rect frames, filled with series palette color
6. Area fills          → rect frames stacked below line
7. Line segments       → 1 pt tall rect frames
8. Line markers        → 4×4 pt rect frames (series color fill)
9. X-tick labels       → text frames below X-axis
10. Y-axis tick labels → text frames left of Y-axis
11. Value labels        → text frames above/inside bars
12. Legend swatches    → 12×10 pt rect frames
13. Legend text labels → text frames beside swatches
```

### 3.3 Frame Creation Pattern

Each element is a `frame.create` + optional `object.fill` + `object.stroke`:

```rust
fn emit_rect(
    app: &mut DesignApp,
    spread_idx: usize,
    rect: [f64; 4],
    fill: &str,     // swatch name or hex "#RRGGBB" or "[None]"
    stroke: &str,   // swatch name or "[None]"
    stroke_weight: f64,
) -> Result<u64, String> {
    let res = app.run("frame.create", json!({
        "spread": spread_idx,
        "rect": rect,
        "content": "unassigned",
        "shape": "rectangle",
        "caret": false,
    })).map_err(|e| e.to_string())?;
    let fid = res.get("id").and_then(Value::as_u64)
        .ok_or_else(|| "frame.create returned no id".to_string())?;
    app.run("object.fill", json!({"ids": [fid], "swatch": fill}))
        .map_err(|e| e.to_string())?;
    app.run("object.stroke", json!({"ids": [fid], "swatch": stroke}))
        .map_err(|e| e.to_string())?;
    if stroke != "[None]" {
        app.run("object.strokeWeight", json!({"ids": [fid], "weight": stroke_weight}))
            .map_err(|e| e.to_string())?;
    }
    Ok(fid)
}
```

```rust
fn emit_text(
    app: &mut DesignApp,
    spread_idx: usize,
    rect: [f64; 4],
    text: &str,
    size: f64,
    bold: bool,
    align: &str,   // "left" | "center" | "right"
) -> Result<u64, String>
```

### 3.4 Donut Arc Approximation

Donut slices are approximated by emitting a fan of thin (1 pt wide) rect frames rotated around the donut center using `object.rotate`. Each 5-degree step is one frame:

```rust
for deg in (start_deg..end_deg).step_by(5) {
    let (cx, cy) = donut_center;
    let r = donut_outer_r;
    let x = cx + r * deg_cos - 0.5;
    let y = cy - r * deg_sin - inner_r;
    emit_rect(app, spread_idx, [x, y, x + 1.0, y + (r - inner_r)], color, "[None]", 0.0)?;
    app.run("object.rotate", json!({"ids": [fid], "angle": deg, "cx": cx, "cy": cy}))?;
}
```

> **Alternative (Phase 4+ upgrade):** Once `frame.create` supports `shape: "polygon"` with arbitrary path points, replace the fan approximation with a true arc path. For now the fan approach is fast to implement and visually sufficient.

### 3.5 Swatches

Before emitting any colored element, ensure the swatch exists:

```rust
fn ensure_swatch(app: &mut DesignApp, hex: &str) -> Result<String, String> {
    // Try swatch.create; if it already exists the engine returns the existing name.
    let res = app.run("swatch.create", json!({"value": hex}))?;
    Ok(res["name"].as_str().unwrap_or(hex).to_string())
}
```

### 3.6 Grouping

After emitting all frames for a chart, group them:

```rust
app.run("object.group", json!({"ids": all_frame_ids}))?;
```

This makes the chart a single selectable unit on the canvas.

### 3.7 Tests for Phase 3

These are integration tests that run with a live `Session`:

```rust
#[test] fn bar_clustered_emits_correct_frame_count()
#[test] fn bar_stacked_frames_do_not_overlap_ceiling()
#[test] fn line_chart_emits_markers_and_segments()
#[test] fn donut_chart_emits_slice_frames()
#[test] fn chart_is_grouped_after_emission()
#[test] fn legend_frames_match_series_count()
#[test] fn value_labels_appear_above_bars()
```

**Deliverables:** `charts/emitter.rs` — all chart types render on a live document spread.

---

## 4. Phase 4 — Integration into `pages.rs` & `excel.rs`

**Goal:** Wire the new vector engine into the existing chart page generation path with a clean fallback to the raster image path.

### 4.1 Decision Logic in `pages.rs`

In the `} else if is_chart_type(type_val) {` branch (line ~1489), add:

```rust
} else if is_chart_type(type_val) {
    // --- Try vector chart engine first ---
    let vector_rendered = if let Some(spec) = charts::spec::parse_chart_spec(sheet) {
        match charts::spec::extract_chart_data(sheet, &spec) {
            Ok((categories, series_values)) => {
                let layout = charts::layout::compute_layout(
                    &spec, &categories, &series_values, margin, st.bilingual_output,
                );
                let palette = charts::palette::series_colors(&st.theme_colors);
                match charts::emitter::emit_chart(app, spread_idx, &spec, &layout, &palette, st.bilingual_output) {
                    Ok(n) => {
                        vector_charts_count = vector_charts_count.saturating_add(1);
                        true
                    }
                    Err(e) => {
                        tracing::warn!("Vector chart failed for {sheet_name}: {e}; falling back to raster");
                        false
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Chart data extraction failed for {sheet_name}: {e}; falling back to raster");
                false
            }
        }
    } else {
        false  // no chart_type row → raster path
    };

    if !vector_rendered {
        // Existing raster image placement path (unchanged)
        if let Some(file_str) = Some(file_val).filter(|s| !s.is_empty()) {
            // ... existing code ...
        }
    }
```

### 4.2 Generation Summary

Update the summary string in `pages.rs:2012`:

```rust
// Before:
details.push(format!("{inserted_images_count} chart image(s) placed"));

// After:
if vector_charts_count > 0 {
    details.push(format!("{vector_charts_count} vector chart(s) rendered"));
}
if inserted_images_count > 0 {
    details.push(format!("{inserted_images_count} chart image(s) placed"));
}
```

### 4.3 `excel.rs` — Expose Sheet Data to Parser

`parse_chart_spec()` needs the full `ExcelSheetData` including its data rows (not just the metadata rows). Verify that `ExcelSheetData::grid` already carries all rows when a `CXX_XX` sheet is ingested:

- If `grid` is already complete → no change needed.
- If only metadata rows are kept → extend `ingest_chart_sheet()` to preserve the full grid.

The `chart_type` row tells the parser this is a vector chart; it then reads data from the same grid using `data_start_row`/`data_end_row` row indices.

### 4.4 `excel.rs` — Catalog Entry Update

When a vector chart spec is present, set the catalog entry's `file` field to `""` (empty) so the raster path does not attempt to resolve a missing image file. Add a `vector: true` field to the catalog entry for the generation summary:

```rust
// In the CXX catalog entry builder:
let is_vector = parse_chart_spec(sheet).is_some();
catalog.push(CatalogEntry {
    entry_type: "Chart".into(),
    number: num_val,
    title: title_val,
    file: if is_vector { String::new() } else { file_val },
    template: tpl_val,
    vector_chart: is_vector,
});
```

### 4.5 Tests for Phase 4

```rust
#[test] fn vector_chart_sheet_does_not_place_raster_image()
#[test] fn raster_fallback_used_when_no_chart_type_row()
#[test] fn raster_fallback_used_when_data_extraction_fails()
#[test] fn generation_summary_distinguishes_vector_and_raster()
#[test] fn existing_raster_chart_tests_still_pass()  // regression guard
```

**Deliverables:** Vector charts generated from compliant `CXX_XX` sheets; raster path unchanged.

---

## 5. Phase 5 — Polish, Bilingual Labels, TOC Integration & Tests

**Goal:** Finish the user-facing quality: label rotation, bilingual legend text, TOC list-of-charts entries, and the complete test suite.

### 5.1 Bilingual Legend Labels

When `st.bilingual_output && !spec.series[i].label_en.is_empty()`:

```
Legend text: "Laki-laki / Male"
```

Emit with `locale::italicize_ranges(app, story, ...)` applying Italic to the English portion — exactly as chart titles already do.

### 5.2 X-Axis Label Rotation

When `layout.x_tick_labels[i].rotated`:
- Emit the text frame 45° rotated using `object.rotate`.
- Allocate extra bottom padding (`AXIS_LABEL_H * 1.8`) in the layout.

For `horizontal_bar` charts, labels are on the Y-axis — no rotation needed.

### 5.3 Bilingual Value Format

```rust
fn format_value(v: f64, fmt: &ValueFormat, bilingual: bool) -> String {
    match fmt {
        ValueFormat::Percent => format!("{:.1}%", v),
        ValueFormat::CurrencyIdr => {
            // "Rp 1.234.567" (ID) or "IDR 1,234,567" (EN)
            if bilingual { format!("IDR {:,.0}", v) } else { format!("Rp {:.*}", v) }
        }
        ValueFormat::Number => format_indonesian_number(v),
    }
}
```

### 5.4 TOC / List of Charts Integration

The chart page already registers a `"Chart"` catalog entry in the TOC builder (in `toc.rs`). After this phase:
- Vector chart entries carry the same catalog type → they appear in *Daftar Grafik* automatically.
- No change needed in `toc.rs`.

Verify with a regression test:

```rust
#[test] fn vector_chart_appears_in_list_of_charts_toc()
```

### 5.5 Batch / Compiler Integration

`pubgen.compileProject` (Pillar 2) calls `pages.rs` per chapter workbook. Since the vector engine is integrated into `pages.rs`, it works in batch mode automatically. Add one batch-mode test:

```rust
#[test] fn compile_project_renders_vector_charts_in_chapter()
```

### 5.6 Error Surface

All errors surface as generation warnings (shown in the panel's status row), never as panics:

| Condition | Behaviour |
|---|---|
| `chart_type` row absent | silent raster fallback |
| Required field missing | `tracing::warn!` + raster fallback |
| Data extraction fails | `tracing::warn!` + raster fallback |
| `app.run` error during emit | `tracing::warn!` + returns `Err(…)` → raster fallback |
| Empty data (0 rows) | `tracing::warn!("no data rows")` + raster fallback |

### 5.7 Full Test Suite for Phase 5

```rust
#[test] fn bilingual_legend_labels_have_italic_english()
#[test] fn long_x_labels_are_rotated()
#[test] fn percent_format_shows_percent_sign()
#[test] fn idr_format_uses_indonesian_separator_in_monolingual()
#[test] fn idr_format_uses_english_separator_in_bilingual()
#[test] fn vector_chart_appears_in_list_of_charts_toc()
#[test] fn compile_project_renders_vector_charts_in_chapter()
#[test] fn all_error_paths_fall_back_without_panic()
```

---

## 6. File Layout

```
crates/ui-egui/src/panels/pubgen/
├── charts/
│   ├── mod.rs          Phase 1 — re-exports, module wiring
│   ├── spec.rs         Phase 1 — ChartSpec, ChartType, parse_chart_spec(), extract_chart_data()
│   ├── layout.rs       Phase 2 — ChartLayout, compute_layout(), per-type layout fns
│   ├── palette.rs      Phase 2 — series_colors(), DIV theme binding
│   └── emitter.rs      Phase 3 — emit_chart(), emit_rect(), emit_text(), ensure_swatch()
├── excel.rs            Phase 4 — catalog entry update (vector_chart flag)
└── pages.rs            Phase 4 — vector/raster decision branch + summary update
```

---

## 7. Excel Workbook Sheet Example

A fully-specified `C02_01` sheet for a clustered bar chart:

| A | B |
|---|---|
| `chart_type` | `bar_clustered` |
| `title` | `Grafik 02.01 Penduduk Menurut Desa dan Jenis Kelamin, 2026` |
| `title_en` | `Chart 02.01 Population by Village and Sex, 2026` |
| `category_col` | `A` |
| `series_1_col` | `B` |
| `series_1_label` | `Laki-laki` |
| `series_1_label_en` | `Male` |
| `series_2_col` | `C` |
| `series_2_label` | `Perempuan` |
| `series_2_label_en` | `Female` |
| `data_start_row` | `10` |
| `data_end_row` | `30` |
| `show_value_labels` | `true` |
| `show_legend` | `true` |

Rows 10–30 in the same sheet:

| A (Desa/Village) | B (Laki-laki) | C (Perempuan) |
|---|---|---|
| Desa Karang Mukti | 1.234 | 1.190 |
| Desa Cikampek Selatan | 987 | 1.034 |
| … | … | … |

---

## 8. Acceptance Criteria

- [ ] A `CXX_XX` sheet with `chart_type = bar_clustered` renders a grouped-bar vector chart — no raster image needed.
- [ ] A `CXX_XX` sheet with `chart_type = donut` renders a donut breakdown.
- [ ] A `CXX_XX` sheet with `chart_type = line` renders a line chart with markers.
- [ ] A `CXX_XX` sheet with `chart_type = horizontal_bar` renders a ranked horizontal bar chart.
- [ ] A `CXX_XX` sheet with **no** `chart_type` row continues to place a raster image (zero regression).
- [ ] Chart colors match the chapter's DIV theme palette.
- [ ] Bilingual legend labels show `"Laki-laki / Male"` with the English part in Italic.
- [ ] Long category labels are rotated 45° on the X-axis.
- [ ] The chart appears as a single grouped object on the canvas.
- [ ] The chart appears in the *Daftar Grafik* TOC.
- [ ] `pubgen.compileProject` batch mode renders vector charts correctly.
- [ ] `cargo xtask ci` passes (fmt, clippy -D warnings, all tests).
- [ ] All error paths (missing fields, bad data, engine errors) produce a warning, not a panic.

---

## 9. Milestone Summary

| Phase | Key deliverable | Days |
|---|---|---|
| **1** | `ChartSpec` parser, schema, data extractor, 6 tests | 1 |
| **2** | Coordinate mapper, all 4 chart-type layouts, color palette binding, 7 tests | 1.5 |
| **3** | Vector emitter, all chart types draw on live doc, 7 integration tests | 2 |
| **4** | Wire into `pages.rs` / `excel.rs`, summary update, 5 tests | 1 |
| **5** | Bilingual labels, rotation, TOC, batch, error polish, 8 tests | 1 |
| **Total** | Full vector chart engine | **6.5 days** |

