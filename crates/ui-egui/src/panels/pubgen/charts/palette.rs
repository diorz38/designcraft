//! Color palette resolution for vector charts.
//!
//! Maps chart series to color swatches from the active chapter DIV theme,
//! or falls back to standard BPS publication swatches.

/// Standard BPS publication palette for statistical charts.
pub const BPS_CHART_PALETTE: &[&str] = &[
    "#0072BC", // BPS Primary Blue
    "#F5871F", // BPS Accent Orange
    "#00A859", // Leaf Green
    "#0088D2", // Sky Blue
    "#E53935", // Crimson Red
    "#8E24AA", // Purple
    "#3949AB", // Indigo
    "#FDD835", // Amber
    "#00897B", // Teal
    "#6D4C41", // Brown
];

/// Returns the hex color string for a series index.
///
/// If `theme_colors` contains a valid hex at `series_idx`, that is used.
/// Otherwise, cycles through `BPS_CHART_PALETTE`.
pub fn series_color(theme_colors: &[String], series_idx: usize) -> &str {
    if let Some(color) = theme_colors.get(series_idx)
        && !color.trim().is_empty()
    {
        return color.as_str();
    }
    BPS_CHART_PALETTE[series_idx % BPS_CHART_PALETTE.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_colors_are_preferred_when_present() {
        let theme = vec!["#112233".to_string(), "#445566".to_string()];
        assert_eq!(series_color(&theme, 0), "#112233");
        assert_eq!(series_color(&theme, 1), "#445566");
    }

    #[test]
    fn falls_back_to_bps_palette_when_theme_is_empty_or_exhausted() {
        let theme = vec!["#112233".to_string()];
        assert_eq!(series_color(&theme, 0), "#112233");
        assert_eq!(series_color(&theme, 1), BPS_CHART_PALETTE[1]);
        assert_eq!(series_color(&[], 0), BPS_CHART_PALETTE[0]);
    }

    #[test]
    fn palette_cycles_when_series_index_exceeds_palette_length() {
        let idx = BPS_CHART_PALETTE.len() + 2;
        assert_eq!(series_color(&[], idx), BPS_CHART_PALETTE[2]);
    }
}
