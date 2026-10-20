use designcraft_color::{Color, Swatch};
use designcraft_doc::{Content, Document, ParaFormat, SpreadRef, build::NewDocument};
use designcraft_geom::Rect;
use serde_json::json;

use super::types::{SlotTag, SlotType, parse_slot_tag};
use crate::Session;

#[test]
fn test_parse_slot_tag_explicit_types() {
    assert_eq!(
        parse_slot_tag("#slot:text:title"),
        Some(SlotTag {
            kind: SlotType::Text,
            key: "title",
        })
    );
    assert_eq!(
        parse_slot_tag("#slot:image:hero"),
        Some(SlotTag {
            kind: SlotType::Image,
            key: "hero",
        })
    );
    assert_eq!(
        parse_slot_tag("#slot:graphic:logo"),
        Some(SlotTag {
            kind: SlotType::Image,
            key: "logo",
        })
    );
    assert_eq!(
        parse_slot_tag("#slot:photo:medallion"),
        Some(SlotTag {
            kind: SlotType::Image,
            key: "medallion",
        })
    );
    assert_eq!(
        parse_slot_tag("#slot:table:main"),
        Some(SlotTag {
            kind: SlotType::Table,
            key: "main",
        })
    );
    assert_eq!(
        parse_slot_tag("#slot:story:notes"),
        Some(SlotTag {
            kind: SlotType::Story,
            key: "notes",
        })
    );
    assert_eq!(
        parse_slot_tag("#slot:rtf:chapter_body"),
        Some(SlotTag {
            kind: SlotType::Story,
            key: "chapter_body",
        })
    );
}

#[test]
fn test_parse_slot_tag_untyped_fallback() {
    // When kind is omitted, defaults to Text
    assert_eq!(
        parse_slot_tag("#slot:year"),
        Some(SlotTag {
            kind: SlotType::Text,
            key: "year",
        })
    );
    assert_eq!(
        parse_slot_tag("  #slot:agency  "),
        Some(SlotTag {
            kind: SlotType::Text,
            key: "agency",
        })
    );
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
    let mut tpl_doc = Document::new(&NewDocument {
        pages: 1,
        facing_pages: false,
        width: 420.0,
        height: 595.0,
        ..Default::default()
    });

    // Add a text frame tagged with #slot:text:title
    let frame_rect = Rect::new(36.0, 36.0, 384.0, 100.0);
    let (item_id, _) = tpl_doc
        .add_text_frame(
            SpreadRef::Doc(0),
            frame_rect,
            tpl_doc.default_layer(),
            "Original Template Title",
            ParaFormat::default(),
        )
        .unwrap();

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
