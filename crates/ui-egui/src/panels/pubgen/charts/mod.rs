//! Native statistical infographic & vector chart engine for Pubgen.
//!
//! # Architecture
//!
//! ```text
//! ExcelSheetData (CXX_XX)
//!   └─ spec::parse_chart_spec()     → ChartSpec   (Phase 1)
//!   └─ spec::extract_chart_data()   → (categories, series_values)
//!        └─ layout::compute_layout() → ChartLayout  (Phase 2)
//!             └─ emitter::emit_chart() → frames on spread (Phase 3)
//! ```
//!
//! # Sheet schema (new optional rows in CXX_XX sheets)
//!
//! A sheet with **no** `chart_type` row continues to produce a raster-image page (zero regression).
//! A sheet with `chart_type` present triggers vector rendering; `file;…` is silently ignored.
//!
//! | Row key          | Example value       | Required for vector |
//! |------------------|---------------------|---------------------|
//! | `chart_type`     | `bar_clustered`     | ✓                   |
//! | `title`          | `Grafik 02.01 …`    | existing            |
//! | `title_en`       | `Chart 02.01 …`     | optional            |
//! | `category_col`   | `A`                 | ✓                   |
//! | `series_1_col`   | `B`                 | ✓                   |
//! | `series_1_label` | `Laki-laki`         | ✓                   |
//! | `series_1_label_en` | `Male`           | optional            |
//! | `series_2_col`   | `C`                 | optional            |
//! | `series_2_label` | `Perempuan`         | optional            |
//! | `data_start_row` | `10`                | ✓                   |
//! | `data_end_row`   | `30`                | ✓                   |
//! | `value_format`   | `number`            | optional            |
//! | `show_value_labels` | `true`           | optional            |
//! | `show_legend`    | `true`              | optional            |
//! | `show_grid`      | `true`              | optional            |

pub mod emitter;
pub mod layout;
pub mod palette;
pub mod spec;

pub use emitter::emit_chart;
pub use layout::{
    AXIS_LABEL_H, AXIS_LABEL_W, BAR_GROUP_GAP, BAR_SERIES_GAP, BarRect, ChartLayout, DonutSlice, GridLine, LegendItem, XTickLabel,
    compute_layout,
};
pub use palette::{BPS_CHART_PALETTE, series_color};
pub use spec::{ChartSpec, ChartType, SeriesSpec, ValueFormat, extract_chart_data, parse_chart_spec};
