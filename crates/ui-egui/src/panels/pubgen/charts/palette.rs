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

/// Preset map color gradients: (name, min_hex, max_hex).
pub const MAP_COLOR_PRESETS: &[(&str, &str, &str)] = &[
    ("BPS Blue (Default)", "#E1F5FE", "#0072BC"),
    ("Flame Amber", "#FFF8E1", "#F5871F"),
    ("Emerald Green", "#E8F5E9", "#00A859"),
    ("Crimson Red", "#FFEBEE", "#E53935"),
    ("Purple Orchid", "#F3E5F5", "#8E24AA"),
    ("Teal Cyan", "#E0F2F1", "#00897B"),
];

/// Match a theme name or color keyword to a preset palette index in `MAP_COLOR_PRESETS`.
pub fn find_map_palette_idx(name: &str) -> usize {
    let lower = name.trim().to_lowercase();
    if lower.is_empty() {
        return 0;
    }
    for (i, (preset_name, _, _)) in MAP_COLOR_PRESETS.iter().enumerate() {
        let p_lower = preset_name.to_lowercase();
        if p_lower.contains(&lower) || lower.contains(&p_lower) {
            return i;
        }
    }
    if lower.contains("amber") || lower.contains("orange") || lower.contains("flame") || lower.contains("yellow") {
        1
    } else if lower.contains("green") || lower.contains("emerald") || lower.contains("hijau") {
        2
    } else if lower.contains("red") || lower.contains("crimson") || lower.contains("merah") {
        3
    } else if lower.contains("purple") || lower.contains("orchid") || lower.contains("violet") || lower.contains("ungu") {
        4
    } else if lower.contains("teal") || lower.contains("cyan") {
        5
    } else {
        0
    }
}

/// Parse a hex string `#RRGGBB` or `#RGB` into `(r, g, b)`.
pub fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
        let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
        let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
        Some((r, g, b))
    } else if clean.len() == 3 {
        let r = u8::from_str_radix(&clean[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&clean[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&clean[2..3], 16).ok()? * 17;
        Some((r, g, b))
    } else {
        None
    }
}

/// Interpolates between two hex colors `c0` and `c1` with factor `t` in `[0.0, 1.0]`.
pub fn interpolate_color(c0: &str, c1: &str, t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let (r0, g0, b0) = parse_hex_rgb(c0).unwrap_or((225, 245, 254));
    let (r1, g1, b1) = parse_hex_rgb(c1).unwrap_or((0, 114, 188));

    let r = (r0 as f64 + t * (r1 as f64 - r0 as f64)).round() as u8;
    let g = (g0 as f64 + t * (g1 as f64 - g0 as f64)).round() as u8;
    let b = (b0 as f64 + t * (b1 as f64 - b0 as f64)).round() as u8;

    format!("#{r:02X}{g:02X}{b:02X}")
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
    fn test_interpolate_color_endpoints_and_midpoint() {
        let white = "#FFFFFF";
        let black = "#000000";
        assert_eq!(interpolate_color(white, black, 0.0), "#FFFFFF");
        assert_eq!(interpolate_color(white, black, 1.0), "#000000");
        assert_eq!(interpolate_color(white, black, 0.5), "#808080");
    }

    #[test]
    fn palette_cycles_when_series_index_exceeds_palette_length() {
        let idx = BPS_CHART_PALETTE.len() + 2;
        assert_eq!(series_color(&[], idx), BPS_CHART_PALETTE[2]);
    }
}
