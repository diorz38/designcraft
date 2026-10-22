use designcraft_color::{Color, Swatch};
use designcraft_doc::{Content, Document, ParaFormat, SpreadRef, build::NewDocument};
use designcraft_geom::Rect;
use serde_json::json;

use super::types::{SlotTag, SlotType, parse_slot_tag, split_slot_value};
use crate::Session;

#[test]
fn test_parse_slot_tag_explicit_types() {
    assert_eq!(parse_slot_tag("#slot:text:title"), Some(SlotTag { kind: SlotType::Text, key: "title", part: 1 }));
    assert_eq!(parse_slot_tag("#slot:image:hero"), Some(SlotTag { kind: SlotType::Image, key: "hero", part: 1 }));
    assert_eq!(parse_slot_tag("#slot:graphic:logo"), Some(SlotTag { kind: SlotType::Image, key: "logo", part: 1 }));
    assert_eq!(parse_slot_tag("#slot:photo:medallion"), Some(SlotTag { kind: SlotType::Image, key: "medallion", part: 1 }));
    assert_eq!(parse_slot_tag("#slot:table:main"), Some(SlotTag { kind: SlotType::Table, key: "main", part: 1 }));
    assert_eq!(parse_slot_tag("#slot:story:notes"), Some(SlotTag { kind: SlotType::Story, key: "notes", part: 1 }));
    assert_eq!(parse_slot_tag("#slot:rtf:chapter_body"), Some(SlotTag { kind: SlotType::Story, key: "chapter_body", part: 1 }));
}

#[test]
fn test_parse_slot_tag_untyped_fallback() {
    // When kind is omitted, defaults to Text
    assert_eq!(parse_slot_tag("#slot:year"), Some(SlotTag { kind: SlotType::Text, key: "year", part: 1 }));
    assert_eq!(parse_slot_tag("  #slot:agency  "), Some(SlotTag { kind: SlotType::Text, key: "agency", part: 1 }));
}

#[test]
fn test_parse_slot_tag_part_suffix() {
    // A headline split over frames: `title` and `title#2`.
    assert_eq!(parse_slot_tag("#slot:text:title#2"), Some(SlotTag { kind: SlotType::Text, key: "title", part: 2 }));
    assert_eq!(parse_slot_tag("#slot:text:year#3"), Some(SlotTag { kind: SlotType::Text, key: "year", part: 3 }));
    assert_eq!(parse_slot_tag("#slot:title#2"), Some(SlotTag { kind: SlotType::Text, key: "title", part: 2 }));
    // `#1` and a `#` without a number are part of the key, not a suffix.
    assert_eq!(parse_slot_tag("#slot:text:title#1"), Some(SlotTag { kind: SlotType::Text, key: "title#1", part: 1 }));
    assert_eq!(parse_slot_tag("#slot:text:title#x"), Some(SlotTag { kind: SlotType::Text, key: "title#x", part: 1 }));
    assert_eq!(parse_slot_tag("#slot:text:#2"), None);
}

#[test]
fn test_split_slot_value() {
    // One part is the value untouched, line break and all.
    assert_eq!(split_slot_value("KARAWANG TIMUR\nDALAM ANGKA", 1), vec!["KARAWANG TIMUR\nDALAM ANGKA"]);
    // Written lines win.
    assert_eq!(split_slot_value("KARAWANG TIMUR\nDALAM ANGKA", 2), vec!["KARAWANG TIMUR", "DALAM ANGKA"]);
    // Three lines over two frames: only the grouping is even, no line is lost.
    assert_eq!(split_slot_value("A\nB\nC", 2), vec!["A", "B\nC"]);
    // No line break: whole words, so one line still lands on a word boundary.
    assert_eq!(split_slot_value("KARAWANG TIMUR DALAM ANGKA", 2), vec!["KARAWANG TIMUR", "DALAM ANGKA"]);
    // A single word is cut at character boundaries: the two-tone year.
    assert_eq!(split_slot_value("2026", 2), vec!["20", "26"]);
    // More parts than the value can divide into: character grouping stays even, nothing repeats.
    assert_eq!(split_slot_value("2026", 3), vec!["2", "0", "26"]);
}

#[test]
fn test_template_page_apply_binds_text_parts() {
    // A headline and a year each split over two frames, as the cover styles design them.
    let mut tpl_doc = Document::new(&NewDocument { pages: 1, facing_pages: false, width: 420.0, height: 595.0, ..Default::default() });
    let layer = tpl_doc.default_layer();
    let mut frame = |y: f64, label: &str| {
        let rect = Rect::new(36.0, y, 384.0, 40.0);
        let (id, _) = tpl_doc.add_text_frame(SpreadRef::Doc(0), rect, layer, "demo", ParaFormat::default()).unwrap();
        if let Some(it) = tpl_doc.item_mut(id) {
            it.label = label.to_string();
        }
    };
    frame(36.0, "#slot:text:title");
    frame(80.0, "#slot:text:title#2");
    frame(124.0, "#slot:text:year");
    frame(168.0, "#slot:text:year#2");

    let tpl_bytes = designcraft_format::save(&tpl_doc).unwrap();

    let mut s = Session::new();
    s.execute("file.new", &json!({"pages": 1})).unwrap();
    let res = s
        .execute(
            "template.page.apply",
            &json!({
                "bytes": tpl_bytes,
                "slots": {"text": {"title": "KARAWANG TIMUR\nDALAM ANGKA", "year": "2026"}}
            }),
        )
        .expect("apply should succeed");
    assert_eq!(res["slots_bound"], 4, "both parts of both slots bind: {res}");

    let active = s.active().unwrap();
    let mut texts: Vec<String> = active
        .doc
        .spreads
        .iter()
        .flat_map(|sp| sp.items.iter())
        .filter_map(|it| match &it.content {
            Content::Text(tf) => active.doc.story(tf.story).map(|st| st.text.clone()),
            _ => None,
        })
        .collect();
    texts.sort();
    assert_eq!(texts, vec!["20", "26", "DALAM ANGKA", "KARAWANG TIMUR"]);
}

#[test]
fn test_template_page_apply_binds_a_part_given_its_own_value() {
    // The cover's headline frame and its second line: the second part carries its own value
    // (`title#2`), so the headline above it is placed as written.
    let mut tpl_doc = Document::new(&NewDocument { pages: 1, facing_pages: false, width: 420.0, height: 595.0, ..Default::default() });
    let layer = tpl_doc.default_layer();
    let mut frame = |y: f64, label: &str| {
        let rect = Rect::new(36.0, y, 384.0, 40.0);
        let (id, _) = tpl_doc.add_text_frame(SpreadRef::Doc(0), rect, layer, "demo", ParaFormat::default()).unwrap();
        if let Some(it) = tpl_doc.item_mut(id) {
            it.label = label.to_string();
        }
    };
    frame(36.0, "#slot:text:title");
    frame(80.0, "#slot:text:title#2");

    let tpl_bytes = designcraft_format::save(&tpl_doc).unwrap();

    let mut s = Session::new();
    s.execute("file.new", &json!({"pages": 1})).unwrap();
    let res = s
        .execute(
            "template.page.apply",
            &json!({
                "bytes": tpl_bytes,
                "slots": {"text": {"title": "KARAWANG TIMUR\nDALAM ANGKA", "title#2": "Title 2"}}
            }),
        )
        .expect("apply should succeed");
    assert_eq!(res["slots_bound"], 2, "both headline frames bind: {res}");

    let active = s.active().unwrap();
    let mut texts: Vec<String> = active
        .doc
        .spreads
        .iter()
        .flat_map(|sp| sp.items.iter())
        .filter_map(|it| match &it.content {
            Content::Text(tf) => active.doc.story(tf.story).map(|st| st.text.clone()),
            _ => None,
        })
        .collect();
    texts.sort();
    // The headline keeps both of its lines; the second frame carries its own text.
    assert_eq!(texts, vec!["KARAWANG TIMUR\nDALAM ANGKA", "Title 2"]);
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

    let path = assets_dir().join("templates");
    let catalog = scan_template_catalog(&path);
    assert!(!catalog.is_empty(), "Canonical templates should be discovered in {path:?}");
    let covers: Vec<_> = catalog.iter().filter(|t| t.category == "covers").collect();
    assert_eq!(covers.len(), 10, "Should discover 10 canonical cover templates");
}

/// The repository root, from the crate directory a test runs in.
fn assets_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets")
}

#[test]
fn test_canonical_cover_templates_carry_their_slots() {
    // What the Pubgen cover card fills: every cover must offer all of these, or a silently
    // unfilled cover ships (the panel reports success whatever comes back).
    const TEXT_KEYS: [&str; 7] = ["title", "level", "year", "volume", "catalogue", "code", "agency"];

    let dir = assets_dir().join("templates").join("covers");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cover templates at {dir:?}: {e}"))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "designcraft"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 10, "ten canonical covers: {files:?}");

    for path in &files {
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let doc = designcraft_format::load(&bytes).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let page = doc.spreads.first().and_then(|s| s.pages.first()).unwrap_or_else(|| panic!("{path:?}: no page"));
        assert!(
            (page.width - 595.2755905511812).abs() < 0.5 && (page.height - 841.8897637795277).abs() < 0.5,
            "{path:?}: an A4 artboard, got {}x{}",
            page.width,
            page.height
        );

        // Tags may sit on items nested in a group (008 puts its photographs there).
        fn tags(items: &[std::sync::Arc<designcraft_doc::Item>], out: &mut Vec<String>) {
            for it in items {
                out.push(it.label.clone());
                if let Content::Group { items } = &it.content {
                    tags(items, out);
                }
            }
        }
        let mut labels = Vec::new();
        for spread in &doc.spreads {
            tags(&spread.items, &mut labels);
        }
        let keys: Vec<(SlotType, String)> = labels.iter().filter_map(|l| parse_slot_tag(l)).map(|t| (t.kind, t.key.to_string())).collect();
        let parts: Vec<(SlotType, String, u32)> =
            labels.iter().filter_map(|l| parse_slot_tag(l)).map(|t| (t.kind, t.key.to_string(), t.part)).collect();

        for key in TEXT_KEYS {
            assert!(keys.iter().any(|(k, s)| *k == SlotType::Text && s == key), "{path:?}: no `#slot:text:{key}` frame (tags: {labels:?})");
        }
        // The card's second headline line has its own frame in every cover: it is styled smaller
        // and in another colour than the headline above it, so a cover without it loses that line.
        assert!(
            parts.iter().any(|(k, s, p)| *k == SlotType::Text && s == "title" && *p == 2),
            "{path:?}: no `#slot:text:title#2` frame (tags: {labels:?})"
        );
        assert!(
            keys.iter().any(|(k, s)| *k == SlotType::Image && (s == "hero" || s == "landscape")),
            "{path:?}: no `#slot:image:hero` (or landscape) frame (tags: {labels:?})"
        );
        assert!(keys.iter().any(|(k, s)| *k == SlotType::Image && s == "logo"), "{path:?}: no `#slot:image:logo` frame (tags: {labels:?})");
    }
}
