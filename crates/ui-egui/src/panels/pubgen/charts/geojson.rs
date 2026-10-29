//! GeoJSON parser and vector coordinate projection engine for Pubgen.
//!
//! Parses GeoJSON files (`FeatureCollection`, `Polygon`, `MultiPolygon`) and
//! projects geographic coordinates (longitude/latitude) into pixel-precise
//! coordinates within a target bounding rectangle on the spread.
//!
//! Never panics; all parsing and projection returns `Result<T, String>`.

use designcraft_geom::Rect;
use serde_json::Value;

/// A single geographic feature with its properties and polygon rings.
#[derive(Debug, Clone)]
pub struct GeoFeature {
    /// Arbitrary GeoJSON feature properties (e.g., `nmdesa`, `nmkec`, `highlight`).
    pub properties: serde_json::Map<String, Value>,
    /// List of polygon rings: each outer ring is `Vec<(f64, f64)>` in [lon, lat] order.
    pub polygons: Vec<Vec<(f64, f64)>>,
}

impl GeoFeature {
    /// Returns the trimmed `nmdesa` property if present and string-valued.
    pub fn nmdesa(&self) -> Option<&str> {
        self.properties
            .get("nmdesa")
            .or_else(|| self.properties.get("NMDESA"))
            .or_else(|| self.properties.get("desa"))
            .and_then(Value::as_str)
            .map(str::trim)
    }

    /// Returns the trimmed `nmkec` property if present and string-valued.
    pub fn nmkec(&self) -> Option<&str> {
        self.properties
            .get("nmkec")
            .or_else(|| self.properties.get("NMKEC"))
            .or_else(|| self.properties.get("kecamatan"))
            .and_then(Value::as_str)
            .map(str::trim)
    }

    /// Returns `true` if `highlight` property is 1 (integer, float, or string `"1"`).
    pub fn is_highlighted(&self) -> bool {
        let val = self.properties.get("highlight").or_else(|| self.properties.get("HIGHLIGHT")).or_else(|| self.properties.get("Highlight"));

        match val {
            Some(Value::Number(n)) => n.as_i64() == Some(1) || (n.as_f64().unwrap_or(0.0) - 1.0).abs() < 1e-6,
            Some(Value::String(s)) => s.trim() == "1" || s.trim().eq_ignore_ascii_case("true"),
            Some(Value::Bool(b)) => *b,
            _ => false,
        }
    }
}

/// Parsed GeoJSON dataset containing all features and global geographic bounds.
#[derive(Debug, Clone)]
pub struct GeoMap {
    /// Name or identifier of the map.
    pub name: String,
    /// Geographic bounding box: `[min_lon, min_lat, max_lon, max_lat]`.
    pub bbox: [f64; 4],
    /// All parsed features in the map.
    pub features: Vec<GeoFeature>,
}

impl GeoMap {
    /// Parse a GeoJSON string into a `GeoMap`.
    pub fn from_json_str(json_str: &str) -> Result<Self, String> {
        let root: Value = serde_json::from_str(json_str).map_err(|e| format!("Invalid GeoJSON: {e}"))?;

        let mut features = Vec::new();
        let mut min_lon = f64::INFINITY;
        let mut min_lat = f64::INFINITY;
        let mut max_lon = f64::NEG_INFINITY;
        let mut max_lat = f64::NEG_INFINITY;

        let raw_features = match root.get("features").and_then(Value::as_array) {
            Some(arr) => arr.as_slice(),
            None => {
                // Check if root is a single Feature
                if root.get("type").and_then(Value::as_str) == Some("Feature") {
                    std::slice::from_ref(&root)
                } else {
                    return Err("GeoJSON has no 'features' array and is not a Feature".to_string());
                }
            }
        };

        for feat_val in raw_features {
            let props = feat_val.get("properties").and_then(Value::as_object).cloned().unwrap_or_default();

            let geom = match feat_val.get("geometry") {
                Some(g) if !g.is_null() => g,
                _ => continue,
            };

            let geom_type = geom.get("type").and_then(Value::as_str).unwrap_or("");
            let coords = match geom.get("coordinates") {
                Some(c) => c,
                None => continue,
            };

            let mut polygons = Vec::new();

            match geom_type {
                "Polygon" => {
                    // coordinates: [[[lon, lat], ...], [hole...]]
                    if let Some(rings) = coords.as_array() {
                        for ring in rings {
                            if let Some(pts) = parse_ring(ring, &mut min_lon, &mut min_lat, &mut max_lon, &mut max_lat) {
                                if pts.len() >= 3 {
                                    polygons.push(pts);
                                }
                            }
                        }
                    }
                }
                "MultiPolygon" => {
                    // coordinates: [[[[lon, lat], ...]], ...]
                    if let Some(poly_list) = coords.as_array() {
                        for poly in poly_list {
                            if let Some(rings) = poly.as_array() {
                                for ring in rings {
                                    if let Some(pts) = parse_ring(ring, &mut min_lon, &mut min_lat, &mut max_lon, &mut max_lat) {
                                        if pts.len() >= 3 {
                                            polygons.push(pts);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Non-polygon geometries (Points, LineStrings) are skipped for area maps
                }
            }

            if !polygons.is_empty() {
                features.push(GeoFeature { properties: props, polygons });
            }
        }

        if features.is_empty() {
            return Err("GeoJSON contains no valid polygon features".to_string());
        }

        let name = root.get("name").and_then(Value::as_str).unwrap_or("Map").to_string();

        let bbox = [
            if min_lon.is_finite() { min_lon } else { 0.0 },
            if min_lat.is_finite() { min_lat } else { 0.0 },
            if max_lon.is_finite() { max_lon } else { 1.0 },
            if max_lat.is_finite() { max_lat } else { 1.0 },
        ];

        Ok(GeoMap { name, bbox, features })
    }

    /// Project all polygons of all features into pixel coordinates fitting inside `target_rect`.
    ///
    /// Preserves geographic aspect ratio using equirectangular projection centered at the map's mean latitude.
    /// Returns a list of projected features: `(feature_index, Vec<Vec<(f64, f64)>>)`.
    pub fn project_features(&self, target_rect: Rect) -> Vec<ProjectedFeature> {
        let [min_lon, min_lat, max_lon, max_lat] = self.bbox;
        let d_lon = (max_lon - min_lon).max(1e-9);
        let d_lat = (max_lat - min_lat).max(1e-9);

        // Aspect ratio correction: 1 degree lon is cos(lat) * 1 degree lat
        let mean_lat_rad = ((min_lat + max_lat) * 0.5).to_radians();
        let cos_lat = mean_lat_rad.cos().abs().clamp(0.05, 1.0);
        let geo_w = d_lon * cos_lat;
        let geo_h = d_lat;
        let geo_aspect = geo_w / geo_h;

        let target_w = (target_rect.x1 - target_rect.x0).max(10.0);
        let target_h = (target_rect.y1 - target_rect.y0).max(10.0);
        let target_aspect = target_w / target_h;

        let (draw_w, draw_h) = if target_aspect > geo_aspect {
            // Target is wider than geo bounds: height-constrained
            (target_h * geo_aspect, target_h)
        } else {
            // Target is taller than geo bounds: width-constrained
            (target_w, target_w / geo_aspect)
        };

        // Center within target_rect
        let off_x = target_rect.x0 + (target_w - draw_w) * 0.5;
        let off_y = target_rect.y0 + (target_h - draw_h) * 0.5;

        self.features
            .iter()
            .enumerate()
            .map(|(idx, feat)| {
                let mut projected_polygons = Vec::with_capacity(feat.polygons.len());
                let mut centroid_x = 0.0;
                let mut centroid_y = 0.0;
                let mut total_pts = 0usize;

                for poly in &feat.polygons {
                    let mut proj_pts = Vec::with_capacity(poly.len());
                    for &(lon, lat) in poly {
                        // Project lon: 0.0 at min_lon, 1.0 at max_lon
                        let norm_x = (lon - min_lon) / d_lon;
                        // Project lat: in screen coords, Y increases downwards! North (max_lat) is top (0.0).
                        let norm_y = (max_lat - lat) / d_lat;

                        let px = off_x + norm_x * draw_w;
                        let py = off_y + norm_y * draw_h;

                        centroid_x += px;
                        centroid_y += py;
                        total_pts += 1;

                        proj_pts.push((px, py));
                    }
                    projected_polygons.push(proj_pts);
                }

                let centroid = if total_pts > 0 {
                    (centroid_x / total_pts as f64, centroid_y / total_pts as f64)
                } else {
                    (off_x + draw_w * 0.5, off_y + draw_h * 0.5)
                };

                ProjectedFeature { feature_idx: idx, polygons: projected_polygons, centroid }
            })
            .collect()
    }
}

/// A projected geographic feature with coordinates mapped to the document page.
#[derive(Debug, Clone)]
pub struct ProjectedFeature {
    /// Index in original `GeoMap.features`.
    pub feature_idx: usize,
    /// Projected polygon point rings ready for `emit_polygon`.
    pub polygons: Vec<Vec<(f64, f64)>>,
    /// Geographic centroid in page coordinates (ideal for label placement).
    pub centroid: (f64, f64),
}

/// Helper to parse a single ring of `[lon, lat]` points and update bounding box.
fn parse_ring(ring: &Value, min_lon: &mut f64, min_lat: &mut f64, max_lon: &mut f64, max_lat: &mut f64) -> Option<Vec<(f64, f64)>> {
    let pts_arr = ring.as_array()?;
    let mut out = Vec::with_capacity(pts_arr.len());

    for pt in pts_arr {
        let coords = pt.as_array()?;
        if coords.len() < 2 {
            continue;
        }
        let lon = coords[0].as_f64()?;
        let lat = coords[1].as_f64()?;

        *min_lon = min_lon.min(lon);
        *min_lat = min_lat.min(lat);
        *max_lon = max_lon.max(lon);
        *max_lat = max_lat.max(lat);

        out.push((lon, lat));
    }

    if out.len() >= 3 { Some(out) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_GEOJSON: &str = r#"{
        "type": "FeatureCollection",
        "name": "TEST_MAP",
        "features": [
            {
                "type": "Feature",
                "properties": {
                    "nmdesa": "DESA_A",
                    "highlight": 1
                },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [
                        [[107.0, -6.0], [107.5, -6.0], [107.5, -6.5], [107.0, -6.5], [107.0, -6.0]]
                    ]
                }
            },
            {
                "type": "Feature",
                "properties": {
                    "nmdesa": "DESA_B",
                    "highlight": 0
                },
                "geometry": {
                    "type": "MultiPolygon",
                    "coordinates": [
                        [
                            [[107.5, -6.0], [108.0, -6.0], [108.0, -6.5], [107.5, -6.5], [107.5, -6.0]]
                        ]
                    ]
                }
            }
        ]
    }"#;

    #[test]
    fn test_parse_sample_geojson() {
        let map = GeoMap::from_json_str(SAMPLE_GEOJSON).expect("should parse");
        assert_eq!(map.name, "TEST_MAP");
        assert_eq!(map.features.len(), 2);

        // Feature 0
        assert_eq!(map.features[0].nmdesa(), Some("DESA_A"));
        assert!(map.features[0].is_highlighted());
        assert_eq!(map.features[0].polygons.len(), 1);
        assert_eq!(map.features[0].polygons[0].len(), 5);

        // Feature 1
        assert_eq!(map.features[1].nmdesa(), Some("DESA_B"));
        assert!(!map.features[1].is_highlighted());
        assert_eq!(map.features[1].polygons.len(), 1);

        // Bounding box
        assert_eq!(map.bbox[0], 107.0);
        assert_eq!(map.bbox[1], -6.5);
        assert_eq!(map.bbox[2], 108.0);
        assert_eq!(map.bbox[3], -6.0);
    }

    #[test]
    fn test_project_features_to_rect() {
        let map = GeoMap::from_json_str(SAMPLE_GEOJSON).expect("should parse");
        let target = Rect { x0: 50.0, y0: 100.0, x1: 450.0, y1: 500.0 };

        let projected = map.project_features(target);
        assert_eq!(projected.len(), 2);

        for pf in &projected {
            for ring in &pf.polygons {
                for &(px, py) in ring {
                    assert!(px >= target.x0 - 0.1 && px <= target.x1 + 0.1, "px {px} out of bounds");
                    assert!(py >= target.y0 - 0.1 && py <= target.y1 + 0.1, "py {py} out of bounds");
                }
            }
        }

        // DESA_A is west (left), DESA_B is east (right)
        assert!(projected[0].centroid.0 < projected[1].centroid.0);
    }

    #[test]
    fn test_highlight_parsing_variations() {
        let feat_num = GeoFeature { properties: serde_json::json!({"highlight": 1}).as_object().unwrap().clone(), polygons: vec![] };
        assert!(feat_num.is_highlighted());

        let feat_str = GeoFeature { properties: serde_json::json!({"highlight": "1"}).as_object().unwrap().clone(), polygons: vec![] };
        assert!(feat_str.is_highlighted());

        let feat_null = GeoFeature { properties: serde_json::json!({"highlight": null}).as_object().unwrap().clone(), polygons: vec![] };
        assert!(!feat_null.is_highlighted());

        let feat_missing = GeoFeature { properties: serde_json::Map::new(), polygons: vec![] };
        assert!(!feat_missing.is_highlighted());
    }
}
