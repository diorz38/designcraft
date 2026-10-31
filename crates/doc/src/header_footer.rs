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

/// Decorative objects and geometric rules for header/footer templates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HeaderFooterObjects {
    pub header_rule: bool,
    pub header_rule_weight: f64,
    pub footer_rule: bool,
    pub footer_rule_weight: f64,
    pub top_accent_bar: bool,
    pub top_accent_bar_height: f64,
    pub top_accent_bar_bleed: bool,
}

impl Default for HeaderFooterObjects {
    fn default() -> Self {
        Self {
            header_rule: true,
            header_rule_weight: 0.5,
            footer_rule: false,
            footer_rule_weight: 0.5,
            top_accent_bar: false,
            top_accent_bar_height: 4.0,
            top_accent_bar_bleed: true,
        }
    }
}

/// Color theme palette for header/footer templates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HeaderFooterColors {
    pub primary: String,
    pub accent: String,
    pub dark: String,
    pub rule_color: String,
    pub text_color: String,
}

impl Default for HeaderFooterColors {
    fn default() -> Self {
        Self {
            primary: "#0072bc".into(),
            accent: "#f5ab19".into(),
            dark: "#122647".into(),
            rule_color: "[Black]".into(),
            text_color: "[Black]".into(),
        }
    }
}

/// Unified Header & Footer Template bundling header, footer, decorative objects, and colors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HeaderFooterTemplate {
    pub name: String,
    pub description: String,
    pub sided: Sided,
    pub header_offset: f64,
    pub footer_offset: f64,
    pub header: SidedText,
    pub footer: SidedText,
    pub style: String,
    pub objects: HeaderFooterObjects,
    pub colors: HeaderFooterColors,
}

impl Default for HeaderFooterTemplate {
    fn default() -> Self {
        Self {
            name: "Modern Minimal".into(),
            description: "Clean single-line header with hairline rule and subtle folio footer.".into(),
            sided: Sided::Double,
            header_offset: 36.0,
            footer_offset: 36.0,
            header: SidedText {
                single: "{section}\u{e006}{page}".into(),
                left: "{page}\u{e006}{section}".into(),
                right: "{section}\u{e006}{page}".into(),
            },
            footer: SidedText { single: "{filename}".into(), left: "{filename}".into(), right: "{filename}".into() },
            style: "[Basic Paragraph]".into(),
            objects: HeaderFooterObjects {
                header_rule: true,
                header_rule_weight: 0.5,
                footer_rule: false,
                footer_rule_weight: 0.5,
                top_accent_bar: false,
                top_accent_bar_height: 4.0,
                top_accent_bar_bleed: true,
            },
            colors: HeaderFooterColors {
                primary: "#0072bc".into(),
                accent: "#f5ab19".into(),
                dark: "#122647".into(),
                rule_color: "[Black]".into(),
                text_color: "[Black]".into(),
            },
        }
    }
}

impl HeaderFooterTemplate {
    /// Returns the 3 predefined templates: Modern Minimal, Editorial Book, and Corporate Report.
    pub fn built_in_templates() -> Vec<Self> {
        vec![
            Self::default(),
            Self {
                name: "Editorial Book".into(),
                description: "Literary spread layout with running headers, chapter numbers, and elegant rules.".into(),
                sided: Sided::Double,
                header_offset: 32.0,
                footer_offset: 32.0,
                header: SidedText {
                    single: "{page}\u{e006}{runningHeader}".into(),
                    left: "{page}\u{e006}{section}".into(),
                    right: "{runningHeader}\u{e006}{page}".into(),
                },
                footer: SidedText { single: "{filename}".into(), left: "{filename}".into(), right: "".into() },
                style: "[Basic Paragraph]".into(),
                objects: HeaderFooterObjects {
                    header_rule: true,
                    header_rule_weight: 0.5,
                    footer_rule: true,
                    footer_rule_weight: 0.25,
                    top_accent_bar: false,
                    top_accent_bar_height: 4.0,
                    top_accent_bar_bleed: true,
                },
                colors: HeaderFooterColors {
                    primary: "#1e293b".into(),
                    accent: "#b45309".into(),
                    dark: "#0f172a".into(),
                    rule_color: "[Black]".into(),
                    text_color: "[Black]".into(),
                },
            },
            Self {
                name: "Corporate Report".into(),
                description: "Institutional publication styling with colored top accent bar and agency branding footer.".into(),
                sided: Sided::Double,
                header_offset: 40.0,
                footer_offset: 36.0,
                header: SidedText {
                    single: "{section}  |  {runningHeader}\u{e006}{page}".into(),
                    left: "{page}\u{e006}{runningHeader}  |  {section}".into(),
                    right: "{section}  |  {runningHeader}\u{e006}{page}".into(),
                },
                footer: SidedText {
                    single: "{filename}\u{e006}Badan Pusat Statistik".into(),
                    left: "Badan Pusat Statistik\u{e006}{filename}".into(),
                    right: "{filename}\u{e006}Badan Pusat Statistik".into(),
                },
                style: "[Basic Paragraph]".into(),
                objects: HeaderFooterObjects {
                    header_rule: true,
                    header_rule_weight: 0.75,
                    footer_rule: true,
                    footer_rule_weight: 0.5,
                    top_accent_bar: true,
                    top_accent_bar_height: 4.0,
                    top_accent_bar_bleed: true,
                },
                colors: HeaderFooterColors {
                    primary: "#0072bc".into(),
                    accent: "#f5ab19".into(),
                    dark: "#122647".into(),
                    rule_color: "[Black]".into(),
                    text_color: "[Black]".into(),
                },
            },
            Self {
                name: "hf-brs".into(),
                description: "BPS Berita Resmi Statistik running footer with publication title, release number, and prominent folio.".into(),
                sided: Sided::Double,
                header_offset: 24.0,
                footer_offset: 24.0,
                header: SidedText::default(),
                footer: SidedText { single: "{title}\n{nopub}".into(), left: "{title}\n{nopub}".into(), right: "{title}\n{nopub}".into() },
                style: "[Basic Paragraph]".into(),
                objects: HeaderFooterObjects {
                    header_rule: false,
                    header_rule_weight: 0.5,
                    footer_rule: false,
                    footer_rule_weight: 0.5,
                    top_accent_bar: false,
                    top_accent_bar_height: 4.0,
                    top_accent_bar_bleed: true,
                },
                colors: HeaderFooterColors {
                    primary: "#f68839".into(),
                    accent: "#fbbf91".into(),
                    dark: "#122647".into(),
                    rule_color: "#f68839".into(),
                    text_color: "#f68839".into(),
                },
            },
        ]
    }

    /// Converts this template into an active `HeaderFooter` specification.
    pub fn to_header_footer(&self, scope: HfScope) -> HeaderFooter {
        HeaderFooter {
            preset: self.name.clone(),
            sided: self.sided,
            scope,
            parent: None,
            header_offset: self.header_offset,
            footer_offset: self.footer_offset,
            header: self.header.clone(),
            footer: self.footer.clone(),
            style: self.style.clone(),
            rule: self.objects.header_rule,
            objects: self.objects.clone(),
            colors: self.colors.clone(),
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
            accent_bar_items: SidedItems::default(),
            footer_rule_items: SidedItems::default(),
            folio_items: SidedItems::default(),
        }
    }
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
    /// Decorative objects (hairline rules, accent bars).
    #[serde(default)]
    pub objects: HeaderFooterObjects,
    /// Color theme and swatches.
    #[serde(default)]
    pub colors: HeaderFooterColors,
    /// Generated header text frame ItemIds.
    #[serde(default)]
    pub header_items: SidedItems,
    /// Generated footer text frame ItemIds.
    #[serde(default)]
    pub footer_items: SidedItems,
    /// Generated hairline rule ItemIds.
    #[serde(default)]
    pub rule_items: SidedItems,
    /// Generated top accent bar ItemIds.
    #[serde(default)]
    pub accent_bar_items: SidedItems,
    /// Generated footer rule ItemIds.
    #[serde(default)]
    pub footer_rule_items: SidedItems,
    /// Generated folio frame ItemIds (e.g. for hf-brs template).
    #[serde(default)]
    pub folio_items: SidedItems,
}

impl HeaderFooter {
    /// Extracts a template definition from this `HeaderFooter` specification.
    pub fn to_template(&self, name: impl Into<String>, description: impl Into<String>) -> HeaderFooterTemplate {
        HeaderFooterTemplate {
            name: name.into(),
            description: description.into(),
            sided: self.sided,
            header_offset: self.header_offset,
            footer_offset: self.footer_offset,
            header: self.header.clone(),
            footer: self.footer.clone(),
            style: self.style.clone(),
            objects: self.objects.clone(),
            colors: self.colors.clone(),
        }
    }
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
            objects: HeaderFooterObjects::default(),
            colors: HeaderFooterColors::default(),
            header_items: SidedItems { single: None, left: Some(ItemId(101)), right: Some(ItemId(102)) },
            footer_items: SidedItems { single: None, left: Some(ItemId(103)), right: None },
            rule_items: SidedItems { single: None, left: Some(ItemId(104)), right: Some(ItemId(105)) },
            accent_bar_items: SidedItems::default(),
            footer_rule_items: SidedItems::default(),
            folio_items: SidedItems::default(),
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
            objects: HeaderFooterObjects::default(),
            colors: HeaderFooterColors::default(),
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
            accent_bar_items: SidedItems::default(),
            footer_rule_items: SidedItems::default(),
            folio_items: SidedItems::default(),
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
            objects: HeaderFooterObjects::default(),
            colors: HeaderFooterColors::default(),
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
            accent_bar_items: SidedItems::default(),
            footer_rule_items: SidedItems::default(),
            folio_items: SidedItems::default(),
        });
        assert!(doc.check().is_ok());
    }

    #[test]
    fn built_in_templates_serde_roundtrip() {
        let templates = HeaderFooterTemplate::built_in_templates();
        assert_eq!(templates.len(), 4);
        assert_eq!(templates[0].name, "Modern Minimal");
        assert_eq!(templates[1].name, "Editorial Book");
        assert_eq!(templates[2].name, "Corporate Report");
        assert!(templates[2].objects.top_accent_bar);
        assert_eq!(templates[3].name, "hf-brs");
        assert_eq!(templates[3].sided, Sided::Double);
        assert!(!templates[3].objects.header_rule);

        let json = serde_json::to_string(&templates).unwrap();
        let de: Vec<HeaderFooterTemplate> = serde_json::from_str(&json).unwrap();
        assert_eq!(templates, de);
    }
}
