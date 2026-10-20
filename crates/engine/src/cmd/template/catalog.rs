use std::fs::File;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Summary metadata describing an available document template.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateInfo {
    /// Template unique identifier (file stem, e.g. "000_wave_curves").
    pub id: String,
    /// Human-friendly display title (e.g. "Wave Curves").
    pub name: String,
    /// Category folder (e.g. "covers", "dividers", "tables", "articles").
    pub category: String,
    /// Path to the .designcraft template file.
    pub path: PathBuf,
    /// Whether the template bundle contains an embedded fast preview thumbnail.
    pub has_preview: bool,
}

/// Convert a kebab-case or snake_case identifier to a human-friendly title.
fn humanize_name(id: &str) -> String {
    // Strip leading numeric ordering prefixes like "000_", "01_", etc.
    let stripped = if let Some(pos) = id.find('_') {
        let prefix = &id[..pos];
        if prefix.chars().all(|c| c.is_ascii_digit()) {
            &id[pos + 1..]
        } else {
            id
        }
    } else {
        id
    };

    let mut words = Vec::new();
    for part in stripped.split(['_', '-']) {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            let capitalized = first.to_uppercase().chain(chars).collect::<String>();
            words.push(capitalized);
        }
    }

    if words.is_empty() {
        id.to_string()
    } else {
        words.join(" ")
    }
}

/// Recursively scans a root directory for all `*.designcraft` templates.
pub fn scan_template_catalog(root: &Path) -> Vec<TemplateInfo> {
    let mut results = Vec::new();
    scan_dir_recursive(root, root, &mut results);
    results.sort_by(|a, b| a.id.cmp(&b.id));
    results
}

fn scan_dir_recursive(root: &Path, current: &Path, results: &mut Vec<TemplateInfo>) {
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir_recursive(root, &path, results);
        } else if path.extension().is_some_and(|ext| ext == "designcraft") {
            let id = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if id.is_empty() {
                continue;
            }

            let category = path
                .parent()
                .and_then(|p| p.strip_prefix(root).ok())
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();

            let has_preview = File::open(&path)
                .ok()
                .and_then(designcraft_format::read_preview)
                .is_some();

            results.push(TemplateInfo {
                name: humanize_name(&id),
                id,
                category,
                path,
                has_preview,
            });
        }
    }
}

/// Read the embedded PNG thumbnail for a template, if present.
pub fn read_template_preview(path: &Path) -> Option<Vec<u8>> {
    let file = File::open(path).ok()?;
    designcraft_format::read_preview(file)
}
