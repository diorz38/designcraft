use designcraft_color::{Color, Swatch};
use designcraft_doc::{Content, Document, ParaFormat, SpreadRef, build::NewDocument};
use designcraft_geom::Rect;
use serde_json::json;

use super::types::{SlotTag, SlotType, parse_slot_tag};
use crate::Session;

#[test]
fn test_parse_slot_tag_explicit_types() {
    assert_eq!(parse_slot_tag("#slot:text:title"), Some(SlotTag { kind: SlotType::Text, key: "title" }));
    assert_eq!(parse_slot_tag("#slot:image:hero"), Some(SlotTag { kind: SlotType::Image, key: "hero" }));
    assert_eq!(parse_slot_tag("#slot:graphic:logo"), Some(SlotTag { kind: SlotType::Image, key: "logo" }));
    assert_eq!(parse_slot_tag("#slot:photo:medallion"), Some(SlotTag { kind: SlotType::Image, key: "medallion" }));
    assert_eq!(parse_slot_tag("#slot:table:main"), Some(SlotTag { kind: SlotType::Table, key: "main" }));
    assert_eq!(parse_slot_tag("#slot:story:notes"), Some(SlotTag { kind: SlotType::Story, key: "notes" }));
    assert_eq!(parse_slot_tag("#slot:rtf:chapter_body"), Some(SlotTag { kind: SlotType::Story, key: "chapter_body" }));
}

#[test]
fn test_parse_slot_tag_untyped_fallback() {
    // When kind is omitted, defaults to Text
    assert_eq!(parse_slot_tag("#slot:year"), Some(SlotTag { kind: SlotType::Text, key: "year" }));
    assert_eq!(parse_slot_tag("  #slot:agency  "), Some(SlotTag { kind: SlotType::Text, key: "agency" }));
}

#[test]
fn test_parse_slot_tag_non_slot_or_invalid() {
    assert_eq!(parse_slot_tag("regular_frame_label"), None);
    assert_eq!(parse_slot_tag("#slot:"), None);
    assert_eq!(parse_slot_tag("#slot:text:"), None);
    assert_eq!(parse_slot_tag(""), None);
}

#[test]
fn test_template_page_apply_inserts_spread_and_binds_slots() {
    // 1. Build a synthetic template document
    let mut tpl_doc = Document::new(&NewDocument { pages: 1, facing_pages: false, width: 420.0, height: 595.0, ..Default::default() });

    // Add a text frame tagged with #slot:text:title
    let frame_rect = Rect::new(36.0, 36.0, 384.0, 100.0);
    let (item_id, _) =
        tpl_doc.add_text_frame(SpreadRef::Doc(0), frame_rect, tpl_doc.default_layer(), "Original Template Title", ParaFormat::default()).unwrap();

    // Tag the frame with slot tag
    if let Some(it) = tpl_doc.item_mut(item_id) {
        it.label = "#slot:text:title".to_string();
    }

    // Add swatch "Primary"
    tpl_doc.swatches.push(Swatch::color("Primary", Color::from_hex("#0072bc").unwrap()));

    let tpl_bytes = designcraft_format::save(&tpl_doc).unwrap();

    // 2. Open an active session with an empty document
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 595.28, "height": 841.89, "pages": 1, "facingPages": false})).unwrap();

    // 3. Execute template.page.apply
    let res = s.execute(
        "template.page.apply",
        &json!({
            "bytes": tpl_bytes,
            "position": "after",
            "slots": {
                "text": {
                    "title": "New Statistical Publication 2026"
                }
            },
            "palette": {
                "Primary": "#122647"
            }
        }),
    );

    assert!(res.is_ok(), "template.page.apply should succeed: {:?}", res);
    let val = res.unwrap();
    assert_eq!(val["slots_bound"], 1);

    // 4. Verify document state
    let active = s.active().unwrap();
    assert_eq!(active.doc.page_count(), 2);

    // Verify swatch color was re-tinted
    let primary_swatch = active.doc.swatches.iter().find(|sw| sw.name == "Primary").unwrap();
    if let designcraft_color::SwatchValue::Color { color, .. } = &primary_swatch.value {
        assert_eq!(color.to_hex(), "#122647");
    } else {
        panic!("Primary swatch should be color");
    }

    // Verify text replacement
    let new_item_id = val["items"].as_array().unwrap().first().unwrap().as_u64().unwrap();
    let item = active.doc.item(designcraft_doc::ItemId(new_item_id)).unwrap();
    assert_eq!(item.label, "#slot:text:title");
    if let Content::Text(tf) = &item.content {
        let story = active.doc.story(tf.story).unwrap();
        assert_eq!(story.text.trim(), "New Statistical Publication 2026");
    } else {
        panic!("Item should be text frame");
    }
}

#[test]
fn test_template_palette_override_multiple_swatches() {
    let mut tpl_doc = Document::new(&NewDocument { pages: 1, facing_pages: false, width: 420.0, height: 595.0, ..Default::default() });
    tpl_doc.swatches.push(Swatch::color("Primary", Color::from_hex("#0072bc").unwrap()));
    tpl_doc.swatches.push(Swatch::color("Accent", Color::from_hex("#f5ab19").unwrap()));

    let tpl_bytes = designcraft_format::save(&tpl_doc).unwrap();

    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 420.0, "height": 595.0})).unwrap();

    let res = s.execute(
        "template.page.apply",
        &json!({
            "bytes": tpl_bytes,
            "palette": {
                "Primary": "#082641",
                "Accent": "#f5871f"
            }
        }),
    );
    assert!(res.is_ok());

    let active = s.active().unwrap();
    let prim = active.doc.swatches.iter().find(|sw| sw.name == "Primary").unwrap();
    if let designcraft_color::SwatchValue::Color { color, .. } = &prim.value {
        assert_eq!(color.to_hex(), "#082641");
    } else {
        panic!("Primary swatch should be color");
    }

    let acc = active.doc.swatches.iter().find(|sw| sw.name == "Accent").unwrap();
    if let designcraft_color::SwatchValue::Color { color, .. } = &acc.value {
        assert_eq!(color.to_hex(), "#f5871f");
    } else {
        panic!("Accent swatch should be color");
    }
}

#[test]
fn test_template_catalog_discovery() {
    use super::catalog::{read_template_preview, scan_template_catalog};

    let dir = tempfile::tempdir().unwrap();
    let covers_dir = dir.path().join("covers");
    let dividers_dir = dir.path().join("dividers");
    std::fs::create_dir_all(&covers_dir).unwrap();
    std::fs::create_dir_all(&dividers_dir).unwrap();

    let doc = Document::new(&NewDocument::default());
    let dummy_png = b"\x89PNG\r\n\x1a\nfake_preview".to_vec();
    let bytes_with_preview = designcraft_format::save_with_preview(&doc, Some(&dummy_png)).unwrap();
    let bytes_no_preview = designcraft_format::save(&doc).unwrap();

    std::fs::write(covers_dir.join("000_wave_curves.designcraft"), &bytes_with_preview).unwrap();
    std::fs::write(dividers_dir.join("001_minimal_divider.designcraft"), &bytes_no_preview).unwrap();

    let catalog = scan_template_catalog(dir.path());
    assert_eq!(catalog.len(), 2);

    let wave = &catalog[0];
    assert_eq!(wave.id, "000_wave_curves");
    assert_eq!(wave.name, "Wave Curves");
    assert_eq!(wave.category, "covers");
    assert!(wave.has_preview);

    let preview_read = read_template_preview(&wave.path);
    assert_eq!(preview_read, Some(dummy_png));

    let divider = &catalog[1];
    assert_eq!(divider.id, "001_minimal_divider");
    assert_eq!(divider.name, "Minimal Divider");
    assert_eq!(divider.category, "dividers");
    assert!(!divider.has_preview);

    // Test template.catalog.list command via session
    let mut s = Session::new();
    let dir_str = dir.path().to_str().unwrap();
    let list_res = s.execute("template.catalog.list", &json!({"dir": dir_str, "category": "covers"})).unwrap();
    let list_arr = list_res.as_array().unwrap();
    assert_eq!(list_arr.len(), 1);
    assert_eq!(list_arr[0]["id"], "000_wave_curves");
}

#[test]
fn test_canonical_assets_catalog_discovery() {
    use super::catalog::scan_template_catalog;

    let path = std::path::Path::new("assets/templates");
    if !path.exists() {
        return;
    }
    let catalog = scan_template_catalog(path);
    assert!(!catalog.is_empty(), "Canonical templates should be discovered in assets/templates");
    let covers: Vec<_> = catalog.iter().filter(|t| t.category == "covers").collect();
    assert_eq!(covers.len(), 10, "Should discover 10 canonical cover templates");
}
