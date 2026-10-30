//! Discovery and management of stock GeoJSON maps for Pubgen charts and covers.
//!
//! Stock maps are compiled directly into the binary from `assets/pubgen/maps/*.geojson`:
//! - `{AAAA}_DESA.geojson` → Village-level maps for Choropleth charts.
//! - `{AAAA}.geojson` → Sub-district/regional context maps with `highlight == 1` for Publication Covers.
//!
//! Custom maps can also be loaded directly from disk or user-uploaded file paths.

use std::path::{Path, PathBuf};

pub struct EmbeddedMap {
    pub id: &'static str,
    pub display_name: &'static str,
    pub filename: &'static str,
    pub geojson: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/embedded_maps.rs"));
use super::geojson::GeoMap;

/// An entry describing an available stock GeoJSON map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockMapEntry {
    /// Short identifier, e.g., `"PANGKALAN"`.
    pub id: String,
    /// Display name shown in UI dropdowns, e.g., `"Pangkalan (Desa)"` or `"Pangkalan (Kecamatan)"`.
    pub display_name: String,
    /// Identifier or path to the GeoJSON file.
    pub path: PathBuf,
}

/// Candidate paths to the `assets/pubgen/maps` directory on disk (for external/custom maps).
pub fn maps_dir_candidates() -> Vec<PathBuf> {
    let rel = Path::new("assets").join("pubgen").join("maps");
    let from_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(&rel);

    let mut candidates = vec![rel.clone(), from_manifest];
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        candidates.push(exe_dir.join(&rel));
    }
    candidates
}

/// Locates the first existing `assets/pubgen/maps` directory on disk, if present.
pub fn find_maps_dir() -> Option<PathBuf> {
    maps_dir_candidates().into_iter().find(|p| p.is_dir())
}

/// Format an uppercase identifier like `"PANGKALAN"` into title-case `"Pangkalan"`.
fn to_title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str().to_lowercase().as_str(),
    }
}

/// Lists available stock maps for **Choropleth Map Charts**.
/// Uses compile-time embedded maps first, augmented by any disk-based maps.
pub fn list_stock_map_charts() -> Vec<StockMapEntry> {
    let mut out: Vec<StockMapEntry> = EMBEDDED_CHART_MAPS
        .iter()
        .map(|m| StockMapEntry { id: m.id.to_string(), display_name: m.display_name.to_string(), path: PathBuf::from(m.filename) })
        .collect();

    // Augment with any additional custom maps found in assets/pubgen/maps on disk
    if let Some(dir) = find_maps_dir() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

                if let Some(prefix) = fname.strip_suffix("_DESA.geojson").or_else(|| fname.strip_suffix("_desa.geojson")) {
                    let id = prefix.to_uppercase();
                    if !out.iter().any(|e| e.id == id) {
                        out.push(StockMapEntry { id, display_name: format!("{} (Desa)", to_title_case(prefix)), path });
                    }
                }
            }
        }
    }

    out.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    out
}

/// Lists available stock maps for **Publication Covers**.
/// Uses compile-time embedded maps first, augmented by any disk-based maps.
pub fn list_stock_cover_maps() -> Vec<StockMapEntry> {
    let mut out: Vec<StockMapEntry> = EMBEDDED_COVER_MAPS
        .iter()
        .map(|m| StockMapEntry { id: m.id.to_string(), display_name: m.display_name.to_string(), path: PathBuf::from(m.filename) })
        .collect();

    // Augment with any additional custom maps found in assets/pubgen/maps on disk
    if let Some(dir) = find_maps_dir() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();

                if (fname.ends_with(".geojson") || fname.ends_with(".GEOJSON")) && !fname.to_lowercase().ends_with("_desa.geojson") {
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(&fname).to_string();

                    let id = stem.to_uppercase();
                    if !out.iter().any(|e| e.id == id) {
                        out.push(StockMapEntry { id, display_name: format!("{} (Kecamatan)", to_title_case(&stem)), path });
                    }
                }
            }
        }
    }

    out.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    out
}

/// Loads a `GeoMap` for **Choropleth Map Charts** (expects village-level `*_DESA.geojson`).
/// Reads the real map from disk first, then falls back to compile-time embedded data.
pub fn load_chart_geo_map(path_or_id: &str) -> Result<GeoMap, String> {
    let trimmed = path_or_id.trim();
    if trimmed.is_empty() {
        return Err("No GeoJSON file or stock map specified for chart".to_string());
    }

    // 1. Direct path on disk (custom user upload / absolute or relative path)
    let direct_path = Path::new(trimmed);
    if direct_path.is_file() {
        let text = std::fs::read_to_string(direct_path).map_err(|e| format!("Failed to read GeoJSON file at '{}': {e}", direct_path.display()))?;
        return GeoMap::from_json_str(&text);
    }

    let upper = trimmed.to_uppercase();
    let upper_no_suffix = upper.trim_end_matches("_DESA");
    let upper_norm = upper_no_suffix.replace(' ', "_").replace('-', "_");

    // 2. Disk assets first: the real maps on disk must not be shadowed by a
    //    stale embedded copy baked into an older binary.
    if let Some(dir) = find_maps_dir() {
        let candidate = dir.join(format!("{upper_norm}_DESA.geojson"));
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate).map_err(|e| format!("Failed to read stock chart map '{}': {e}", candidate.display()))?;
            return GeoMap::from_json_str(&text);
        }
        let cand_plain = dir.join(format!("{upper_no_suffix}_DESA.geojson"));
        if cand_plain.is_file() {
            let text = std::fs::read_to_string(&cand_plain).map_err(|e| format!("Failed to read stock chart map '{}': {e}", cand_plain.display()))?;
            return GeoMap::from_json_str(&text);
        }
    }

    // 3. Check compile-time embedded stock maps
    for m in EMBEDDED_CHART_MAPS {
        if m.id == upper
            || m.id == upper_no_suffix
            || m.id == upper_norm
            || m.filename.eq_ignore_ascii_case(trimmed)
            || m.filename.strip_suffix(".geojson").is_some_and(|s| s.eq_ignore_ascii_case(trimmed))
            || m.display_name.eq_ignore_ascii_case(trimmed)
        {
            return GeoMap::from_json_str(m.geojson);
        }
    }

    Err(format!("Stock chart GeoJSON map '{trimmed}' (*_DESA.geojson) not found in embedded maps or on disk"))
}

/// Loads a `GeoMap` for **Publication Covers** (expects district overview `*.geojson` with `highlight`).
/// Reads the real map from disk first, then falls back to compile-time embedded data.
pub fn load_cover_geo_map(path_or_id: &str) -> Result<GeoMap, String> {
    let trimmed = path_or_id.trim();
    if trimmed.is_empty() {
        return Err("No GeoJSON file or stock map specified for cover".to_string());
    }

    // 1. Direct path on disk (custom user upload / absolute or relative path)
    let direct_path = Path::new(trimmed);
    if direct_path.is_file() {
        let text = std::fs::read_to_string(direct_path).map_err(|e| format!("Failed to read GeoJSON file at '{}': {e}", direct_path.display()))?;
        return GeoMap::from_json_str(&text);
    }

    let upper = trimmed.to_uppercase();
    let upper_clean = upper.trim_end_matches("_DESA");
    let upper_norm = upper_clean.replace(' ', "_").replace('-', "_");

    // 2. Disk assets first: the real maps on disk must not be shadowed by a
    //    stale embedded copy baked into an older binary.
    if let Some(dir) = find_maps_dir() {
        let candidate = dir.join(format!("{upper_norm}.geojson"));
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate).map_err(|e| format!("Failed to read stock cover map '{}': {e}", candidate.display()))?;
            return GeoMap::from_json_str(&text);
        }
        let cand_plain = dir.join(format!("{upper_clean}.geojson"));
        if cand_plain.is_file() {
            let text = std::fs::read_to_string(&cand_plain).map_err(|e| format!("Failed to read stock cover map '{}': {e}", cand_plain.display()))?;
            return GeoMap::from_json_str(&text);
        }
    }

    // 3. Check compile-time embedded stock maps
    for m in EMBEDDED_COVER_MAPS {
        if m.id == upper
            || m.id == upper_clean
            || m.id == upper_norm
            || m.filename.eq_ignore_ascii_case(trimmed)
            || m.filename.strip_suffix(".geojson").is_some_and(|s| s.eq_ignore_ascii_case(trimmed))
            || m.display_name.eq_ignore_ascii_case(trimmed)
        {
            return GeoMap::from_json_str(m.geojson);
        }
    }

    Err(format!("Stock cover GeoJSON map '{trimmed}' (*.geojson) not found in embedded maps or on disk"))
}

/// Generic loader falling back between chart and cover maps.
pub fn load_geo_map(path_or_id: &str) -> Result<GeoMap, String> {
    load_chart_geo_map(path_or_id).or_else(|_| load_cover_geo_map(path_or_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_stock_maps_finds_pangkalan() {
        let chart_maps = list_stock_map_charts();
        assert!(!chart_maps.is_empty(), "Expected at least one stock chart map (e.g. PANGKALAN_DESA)");
        assert!(
            chart_maps.iter().any(|m| m.id == "PANGKALAN"),
            "Expected PANGKALAN in chart stock maps, got: {:?}",
            chart_maps.iter().map(|m| &m.id).collect::<Vec<_>>()
        );

        let cover_maps = list_stock_cover_maps();
        assert!(!cover_maps.is_empty(), "Expected at least one stock cover map (e.g. PANGKALAN)");
        assert!(
            cover_maps.iter().any(|m| m.id == "PANGKALAN"),
            "Expected PANGKALAN in cover stock maps, got: {:?}",
            cover_maps.iter().map(|m| &m.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn embedded_chart_maps_parse_without_reading_disk() {
        let entry = EMBEDDED_CHART_MAPS.iter().find(|m| m.id == "PANGKALAN").expect("PANGKALAN must be embedded");
        let map = GeoMap::from_json_str(entry.geojson).expect("embedded GeoJSON must parse");
        assert!(map.features.len() >= 8, "expected all Pangkalan villages, got {}", map.features.len());
    }

    #[test]
    fn test_embedded_maps_load_without_disk() {
        let map_res = load_chart_geo_map("PANGKALAN");
        assert!(map_res.is_ok(), "Expected embedded PANGKALAN_DESA chart map to load: {map_res:?}");
        let map = map_res.unwrap();
        assert!(!map.features.is_empty(), "Expected features in PANGKALAN map");

        let cover_res = load_cover_geo_map("PANGKALAN");
        assert!(cover_res.is_ok(), "Expected embedded PANGKALAN cover map to load: {cover_res:?}");
        let cover_map = cover_res.unwrap();
        assert!(!cover_map.features.is_empty(), "Expected features in PANGKALAN cover map");
        assert!(cover_map.features.iter().any(|f| f.is_highlighted()), "Expected highlighted feature in cover map");
    }
}
