//! Shared test helpers for the pubgen domain files.

use super::*;

#[cfg(test)]
pub(crate) fn toc_entry_count(app: &mut DesignApp, style: &str) -> usize {
    app.session.execute("toc.entries", &json!({"entries": [{"style": style, "level": 1}]})).ok().and_then(|v| v.as_array().map(Vec::len)).unwrap_or(0)
}

/// Path of a small synthetic image used by the page-from-image tests.
pub(crate) fn sample_image_path() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/app-icon/designcraft-1024.png").to_string()
}
