use super::types::{SlotTag, SlotType, parse_slot_tag};

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
