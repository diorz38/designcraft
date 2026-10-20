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
    /// Which part of a multi-frame value this frame takes (1-based). `1` for a whole slot.
    pub part: u32,
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
///
/// A text slot split across several frames, each with its own styling (a two-tone
/// headline, a two-tone year), names the part after `#`: `#slot:text:title` and
/// `#slot:text:title#2` take the first and second part of the `title` value. See
/// [`split_slot_value`].
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
        let (key, part) = split_part(key);
        if !valid_key(key) {
            return None;
        }
        Some(SlotTag { kind, key, part })
    } else {
        let (key, part) = split_part(rest.trim());
        if !valid_key(key) {
            return None;
        }
        Some(SlotTag { kind: SlotType::Text, key, part })
    }
}

/// A slot key is a plain identifier: no leading `#` left over from an empty base.
fn valid_key(key: &str) -> bool {
    !key.is_empty() && !key.starts_with('#')
}

/// `key#2` → (`key`, 2). A `#` without a positive number stays part of the key.
fn split_part(key: &str) -> (&str, u32) {
    let Some((base, suffix)) = key.rsplit_once('#') else { return (key, 1) };
    match suffix.trim().parse::<u32>() {
        Ok(n) if n >= 2 && !base.trim().is_empty() => (base.trim(), n),
        _ => (key, 1),
    }
}

/// Split one slot value over the frames of a multi-part text slot.
///
/// A multi-part slot (see [`parse_slot_tag`]) lets a headline keep per-frame styling: the frames
/// carry `title`, `title#2`, … and the value is divided between them.
///
/// 1. Line breaks divide the value when it has at least `parts` lines, so a title typed as
///    `KARAWANG TIMUR`⏎`DALAM ANGKA` fills two frames exactly as written.
/// 2. Otherwise whole words divide it evenly, so one line still lands on word boundaries.
/// 3. A single word is cut evenly at character boundaries: the year `2026` gives `20` and `26`.
pub fn split_slot_value(value: &str, parts: usize) -> Vec<String> {
    let parts = parts.max(1);
    if parts == 1 {
        return vec![value.to_string()];
    }
    let lines: Vec<&str> = value.lines().collect();
    if lines.len() >= parts {
        return group_evenly(&lines, parts, "\n");
    }
    let words: Vec<&str> = value.split_whitespace().collect();
    if words.len() >= parts {
        return group_evenly(&words, parts, " ");
    }
    let chars: Vec<String> = value.chars().map(|c| c.to_string()).collect();
    let refs: Vec<&str> = chars.iter().map(String::as_str).collect();
    group_evenly(&refs, parts, "")
}

/// `items` split into `parts` consecutive groups as evenly as the count allows, each joined by
/// `sep`. With at least one item per part, no group is empty.
fn group_evenly(items: &[&str], parts: usize, sep: &str) -> Vec<String> {
    let n = items.len();
    (0..parts)
        .map(|i| {
            let start = i * n / parts;
            let end = (i + 1) * n / parts;
            items.get(start..end).map(|s| s.join(sep)).unwrap_or_default()
        })
        .collect()
}
