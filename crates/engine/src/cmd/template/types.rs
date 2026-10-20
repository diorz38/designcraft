//! Slot contract types and parsing for the template engine.

use serde::{Deserialize, Serialize};

/// Type of content expected by a template placeholder slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotType {
    /// Injects plain text into a text frame story, preserving existing styling.
    Text,
    /// Injects an image into a graphic frame, applying object fitting.
    Image,
    /// Injects a structured table matrix into a frame.
    Table,
    /// Injects a rich story (RTF, Markdown, or multi-paragraph file) into a text frame.
    Story,
}

/// Parsed slot placeholder tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotTag<'a> {
    pub kind: SlotType,
    pub key: &'a str,
}

/// Parse a slot tag from an item's label or name.
///
/// Accepted formats:
/// - `#slot:text:<key>` (e.g. `#slot:text:title`, `#slot:text:year`)
/// - `#slot:image:<key>` (e.g. `#slot:image:hero`, `#slot:image:logo`)
/// - `#slot:graphic:<key>` (alias for image)
/// - `#slot:photo:<key>` (alias for image)
/// - `#slot:table:<key>` (e.g. `#slot:table:main`)
/// - `#slot:story:<key>` (e.g. `#slot:story:body`, `#slot:story:notes`)
/// - `#slot:rtf:<key>` (alias for story)
/// - `#slot:<key>` (defaults to `SlotType::Text`)
pub fn parse_slot_tag(label: &str) -> Option<SlotTag<'_>> {
    let trimmed = label.trim();
    let rest = trimmed.strip_prefix("#slot:")?;
    if rest.is_empty() {
        return None;
    }

    if let Some((kind_str, key)) = rest.split_once(':') {
        let key = key.trim();
        if key.is_empty() {
            return None;
        }
        let kind = match kind_str.trim().to_ascii_lowercase().as_str() {
            "text" => SlotType::Text,
            "image" | "graphic" | "photo" => SlotType::Image,
            "table" => SlotType::Table,
            "story" | "rtf" => SlotType::Story,
            _ => SlotType::Text,
        };
        Some(SlotTag { kind, key })
    } else {
        Some(SlotTag {
            kind: SlotType::Text,
            key: rest.trim(),
        })
    }
}
