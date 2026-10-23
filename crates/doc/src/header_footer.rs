//! Header/Footer frames specification (Layout ▸ Header/Footer…).
//!
//! Stores the document-level header/footer configuration, presets, page scope,
//! offsets, paragraph style, and the IDs of generated text and graphic line items
//! on the assigned parent spread.

use serde::{Deserialize, Serialize};

use crate::ids::{ItemId, SpreadId};

/// Single-sided vs double-sided (facing pages with left/right page variants).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sided {
    Single,
    Double,
}

/// Scope of pages to which the parent spread is applied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HfScope {
    All,
    Section(usize),
    Range { first: usize, last: usize },
}

/// Header or footer template text per side.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidedText {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub single: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub left: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub right: String,
}

/// Generated item IDs per side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidedItems {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub single: Option<ItemId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<ItemId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<ItemId>,
}

/// Stored Header/Footer specification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderFooter {
    /// Preset name: "minimal" | "book" | "editorial" | "custom".
    pub preset: String,
    pub sided: Sided,
    pub scope: HfScope,
    /// The parent spread created/owned for headers/footers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<SpreadId>,
    /// Distance in points from top page edge to header frame top.
    pub header_offset: f64,
    /// Distance in points from footer frame bottom to bottom page edge.
    pub footer_offset: f64,
    /// Header template text.
    #[serde(default)]
    pub header: SidedText,
    /// Footer template text.
    #[serde(default)]
    pub footer: SidedText,
    /// Paragraph style name applied to header/footer text.
    #[serde(default)]
    pub style: String,
    /// Hairline separator rule flag.
    #[serde(default)]
    pub rule: bool,
    /// Generated header text frame ItemIds.
    #[serde(default)]
    pub header_items: SidedItems,
    /// Generated footer text frame ItemIds.
    #[serde(default)]
    pub footer_items: SidedItems,
    /// Generated hairline rule ItemIds.
    #[serde(default)]
    pub rule_items: SidedItems,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_footer_serde_roundtrip() {
        let hf = HeaderFooter {
            preset: "editorial".into(),
            sided: Sided::Double,
            scope: HfScope::Range { first: 1, last: 10 },
            parent: Some(SpreadId(42)),
            header_offset: 36.0,
            footer_offset: 36.0,
            header: SidedText {
                single: String::new(),
                left: "{page} {section}\u{e006}{runningHeader}".into(),
                right: "{runningHeader}\u{e006}{section} {page}".into(),
            },
            footer: SidedText { single: String::new(), left: "{filename}".into(), right: String::new() },
            style: "Folio".into(),
            rule: true,
            header_items: SidedItems { single: None, left: Some(ItemId(101)), right: Some(ItemId(102)) },
            footer_items: SidedItems { single: None, left: Some(ItemId(103)), right: None },
            rule_items: SidedItems { single: None, left: Some(ItemId(104)), right: Some(ItemId(105)) },
        };

        let json = serde_json::to_string(&hf).unwrap();
        assert!(json.contains("\"headerOffset\":36.0"));
        assert!(json.contains("\"footerOffset\":36.0"));
        assert!(json.contains("\"sided\":\"double\""));
        assert!(json.contains("\"preset\":\"editorial\""));
        let de: HeaderFooter = serde_json::from_str(&json).unwrap();
        assert_eq!(hf, de);
    }

    #[test]
    fn header_footer_default_omits_none_items() {
        let hf = HeaderFooter {
            preset: "minimal".into(),
            sided: Sided::Single,
            scope: HfScope::All,
            parent: None,
            header_offset: 24.0,
            footer_offset: 24.0,
            header: SidedText { single: "{filename}\u{e006}{page}".into(), left: String::new(), right: String::new() },
            footer: SidedText::default(),
            style: "[Basic Paragraph]".into(),
            rule: false,
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
        };

        let json = serde_json::to_string(&hf).unwrap();
        assert!(!json.contains("\"parent\""));
        let de: HeaderFooter = serde_json::from_str(&json).unwrap();
        assert_eq!(hf, de);
    }

    #[test]
    fn older_document_json_without_header_footer_deserializes() {
        let d = crate::Document::new(&crate::build::NewDocument::default());
        let mut val = serde_json::to_value(&d).unwrap();
        val.as_object_mut().unwrap().remove("headerFooter");
        let back: crate::Document = serde_json::from_value(val).unwrap();
        assert_eq!(back.header_footer, None);
    }

    #[test]
    fn document_with_header_footer_passes_check() {
        let mut doc = crate::Document::new(&crate::build::NewDocument::default());
        doc.header_footer = Some(HeaderFooter {
            preset: "minimal".into(),
            sided: Sided::Single,
            scope: HfScope::All,
            parent: None,
            header_offset: 24.0,
            footer_offset: 24.0,
            header: SidedText { single: "{page}".into(), left: String::new(), right: String::new() },
            footer: SidedText::default(),
            style: "[Basic Paragraph]".into(),
            rule: false,
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
        });
        assert!(doc.check().is_ok());
    }
}
