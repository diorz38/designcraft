//! Regression tests for stock GeoJSON map loading.
//!
//! A stale build artifact once embedded only a 1-polygon fallback stub while the
//! real 8-village `PANGKALAN_DESA.geojson` sat on disk; the embedded stub
//! shadowed it and choropleth maps rendered as a single tiny polygon.

#[test]
fn test_stock_map_loader_returns_real_pangkalan_map() {
    let geo = super::stock_maps::load_chart_geo_map("PANGKALAN").expect("PANGKALAN stock map must load");
    assert_eq!(geo.name, "PANGKALAN_DESA", "loaded the real file, not a fallback stub");
    assert!(geo.features.len() >= 8, "expected all 8 villages, got {}", geo.features.len());
    let names: Vec<&str> = geo.features.iter().filter_map(|f| f.nmdesa()).collect();
    for expect in ["CINTAASIH", "MEDALSARI", "TAMANSARI"] {
        assert!(names.contains(&expect), "village {expect} missing from {names:?}");
    }
}
