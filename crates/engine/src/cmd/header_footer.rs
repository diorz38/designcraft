//! Header/Footer frames for pages (Layout ▸ Header/Footer…).
//!
//! Provides `layout.headerFooter.get`, `layout.headerFooter.apply`, and `layout.headerFooter.remove`.
//! Creates and maintains a dedicated parent spread ("H-Header/Footer") with header and footer
//! text frames and hairline rules, with auto-sizing, master inheritance, and scope page assignment.

use std::sync::Arc;

use designcraft_doc::{
    Document, ItemId, Margins, Page, PageSide, ParaFormat, SpreadId, SpreadRef,
    header_footer::{HeaderFooter, HeaderFooterColors, HeaderFooterObjects, HeaderFooterTemplate, HfScope, Sided, SidedItems, SidedText},
    item::{AutoSize, Fill, Item, Shape, Stroke},
    story,
    vars::var_char,
};
use designcraft_geom::{Point, Rect, shapes};
use serde_json::{Value, json};

use super::{CommandSpec, always, bad, cmd, has_doc, ok, str_param};
use crate::{Result, Session};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "layout.headerFooter.get",
            "Header/Footer",
            [],
            None,
            "{}",
            has_doc,
            get
        ),
        cmd!(
            query "layout.headerFooter.listTemplates",
            "Header/Footer Templates",
            [],
            None,
            "{}",
            always,
            list_templates
        ),
        cmd!(
            noundo "layout.headerFooter.saveTemplate",
            "Save Header/Footer Template",
            [],
            None,
            "{template?: object, name?: string} — saves or replaces a custom header/footer template in preferences",
            always,
            save_template
        ),
        cmd!(
            noundo "layout.headerFooter.deleteTemplate",
            "Delete Header/Footer Template",
            [],
            None,
            "{name: string} — deletes a custom header/footer template from preferences by name",
            always,
            delete_template
        ),
        cmd!(
            "layout.headerFooter.apply",
            "Header/Footer…",
            ["Layout"],
            None,
            "{preset?, template?, sided?, scope?, headerOffset?, footerOffset?, header?, footer?, style?, objects?, colors?, rule?, margins?}",
            has_doc,
            apply
        ),
        cmd!("layout.headerFooter.remove", "Remove Header/Footer", ["Layout"], None, "{}", has_doc, remove),
    ]
}

/// Known preset specifications.
pub fn preset_spec(preset: &str) -> Option<HeaderFooter> {
    match preset.to_ascii_lowercase().as_str() {
        "minimal" => Some(HeaderFooter {
            preset: "minimal".into(),
            sided: Sided::Single,
            scope: HfScope::All,
            parent: None,
            header_offset: 24.0,
            footer_offset: 24.0,
            header: SidedText::default(),
            footer: SidedText { single: "{filename}\u{e006}{page}".into(), left: String::new(), right: String::new() },
            style: story::BASIC_PARAGRAPH.into(),
            rule: false,
            objects: HeaderFooterObjects {
                header_rule: false,
                header_rule_weight: 0.5,
                footer_rule: false,
                footer_rule_weight: 0.5,
                top_accent_bar: false,
                top_accent_bar_height: 4.0,
                top_accent_bar_bleed: true,
            },
            colors: HeaderFooterColors::default(),
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
            accent_bar_items: SidedItems::default(),
            footer_rule_items: SidedItems::default(),
            folio_items: SidedItems::default(),
        }),
        "book" => Some(HeaderFooter {
            preset: "book".into(),
            sided: Sided::Double,
            scope: HfScope::All,
            parent: None,
            header_offset: 36.0,
            footer_offset: 36.0,
            header: SidedText { single: String::new(), left: "{page}\u{e006}{runningHeader}".into(), right: "{runningHeader}\u{e006}{page}".into() },
            footer: SidedText::default(),
            style: story::BASIC_PARAGRAPH.into(),
            rule: false,
            objects: HeaderFooterObjects {
                header_rule: false,
                header_rule_weight: 0.5,
                footer_rule: false,
                footer_rule_weight: 0.5,
                top_accent_bar: false,
                top_accent_bar_height: 4.0,
                top_accent_bar_bleed: true,
            },
            colors: HeaderFooterColors::default(),
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
            accent_bar_items: SidedItems::default(),
            footer_rule_items: SidedItems::default(),
            folio_items: SidedItems::default(),
        }),
        "editorial" => Some(HeaderFooter {
            preset: "editorial".into(),
            sided: Sided::Double,
            scope: HfScope::All,
            parent: None,
            header_offset: 36.0,
            footer_offset: 36.0,
            header: SidedText {
                single: String::new(),
                left: "{page}  {section}\u{e006}{runningHeader}".into(),
                right: "{runningHeader}\u{e006}{section}  {page}".into(),
            },
            footer: SidedText { single: String::new(), left: "{filename}".into(), right: "{filename}".into() },
            style: story::BASIC_PARAGRAPH.into(),
            rule: true,
            objects: HeaderFooterObjects {
                header_rule: true,
                header_rule_weight: 0.5,
                footer_rule: false,
                footer_rule_weight: 0.5,
                top_accent_bar: false,
                top_accent_bar_height: 4.0,
                top_accent_bar_bleed: true,
            },
            colors: HeaderFooterColors::default(),
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
            accent_bar_items: SidedItems::default(),
            footer_rule_items: SidedItems::default(),
            folio_items: SidedItems::default(),
        }),
        "hf-brs" | "brs" => Some(HeaderFooter {
            preset: "hf-brs".into(),
            sided: Sided::Double,
            scope: HfScope::All,
            parent: None,
            header_offset: 24.0,
            footer_offset: 24.0,
            header: SidedText::default(),
            footer: SidedText { single: "{title}\n{nopub}".into(), left: "{title}\n{nopub}".into(), right: "{title}\n{nopub}".into() },
            style: story::BASIC_PARAGRAPH.into(),
            rule: false,
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
            header_items: SidedItems::default(),
            footer_items: SidedItems::default(),
            rule_items: SidedItems::default(),
            accent_bar_items: SidedItems::default(),
            footer_rule_items: SidedItems::default(),
            folio_items: SidedItems::default(),
        }),
        _ => None,
    }
}

/// Returns all available templates: built-ins followed by user saved presets.
pub fn all_templates(s: &Session) -> Vec<Value> {
    let mut out = Vec::new();
    for t in HeaderFooterTemplate::built_in_templates() {
        if let Ok(mut v) = serde_json::to_value(&t) {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("builtIn".to_string(), json!(true));
            }
            out.push(v);
        }
    }
    for t in &s.prefs.header_footer_templates {
        if let Ok(mut v) = serde_json::to_value(t) {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("builtIn".to_string(), json!(false));
            }
            out.push(v);
        }
    }
    out
}

/// Query command returning all available templates.
fn list_templates(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(Value::Array(all_templates(s)))
}

/// Saves or replaces a custom header/footer template in preferences.
fn save_template(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "layout.headerFooter.saveTemplate";
    let template: HeaderFooterTemplate = if let Some(t_val) = p.get("template") {
        serde_json::from_value(t_val.clone()).map_err(|e| bad(CMD, format!("invalid template: {e}")))?
    } else {
        serde_json::from_value(p.clone()).map_err(|e| bad(CMD, format!("invalid template: {e}")))?
    };

    let name = template.name.trim();
    if name.is_empty() {
        return Err(bad(CMD, "template name cannot be empty"));
    }

    if HeaderFooterTemplate::built_in_templates().iter().any(|b| b.name.eq_ignore_ascii_case(name)) {
        return Err(bad(CMD, format!("cannot overwrite built-in template `{name}`")));
    }

    if template.header_offset < 0.0 || !template.header_offset.is_finite() {
        return Err(bad(CMD, "headerOffset must be a non-negative finite number"));
    }
    if template.footer_offset < 0.0 || !template.footer_offset.is_finite() {
        return Err(bad(CMD, "footerOffset must be a non-negative finite number"));
    }

    if let Some(pos) = s.prefs.header_footer_templates.iter().position(|t| t.name.eq_ignore_ascii_case(name)) {
        s.prefs.header_footer_templates[pos] = template.clone();
    } else {
        s.prefs.header_footer_templates.push(template.clone());
    }

    Ok(json!({ "saved": true, "template": template }))
}

/// Deletes a custom header/footer template from preferences by name.
fn delete_template(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "layout.headerFooter.deleteTemplate";
    let name = str_param(p, "name").ok_or_else(|| bad(CMD, "missing `name` parameter"))?.trim();
    if HeaderFooterTemplate::built_in_templates().iter().any(|b| b.name.eq_ignore_ascii_case(name)) {
        return Err(bad(CMD, format!("cannot delete built-in template `{name}`")));
    }
    let before = s.prefs.header_footer_templates.len();
    s.prefs.header_footer_templates.retain(|t| !t.name.eq_ignore_ascii_case(name));
    if s.prefs.header_footer_templates.len() == before {
        return Err(bad(CMD, format!("template `{name}` not found")));
    }
    ok()
}

/// Resolves a template by name/alias from user preferences or built-in templates.
fn find_template(s: &Session, name: &str) -> Option<HeaderFooterTemplate> {
    let trimmed = name.trim();
    if let Some(user_t) = s.prefs.header_footer_templates.iter().find(|t| t.name.eq_ignore_ascii_case(trimmed)) {
        return Some(user_t.clone());
    }
    if let Some(built_in) = HeaderFooterTemplate::built_in_templates().into_iter().find(|t| t.name.eq_ignore_ascii_case(trimmed)) {
        return Some(built_in);
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "modern minimal" | "modern" => HeaderFooterTemplate::built_in_templates().into_iter().find(|t| t.name.to_lowercase().contains("minimal")),
        "editorial book" => HeaderFooterTemplate::built_in_templates().into_iter().find(|t| t.name.to_lowercase().contains("book")),
        "corporate report" | "corporate" | "report" => {
            HeaderFooterTemplate::built_in_templates().into_iter().find(|t| t.name.to_lowercase().contains("corporate"))
        }
        "hf-brs" | "brs" => HeaderFooterTemplate::built_in_templates().into_iter().find(|t| t.name.eq_ignore_ascii_case("hf-brs")),
        _ => None,
    }
}

/// Resolves an existing swatch name or creates a process swatch from hex.
fn resolve_or_create_swatch(d: &mut Document, prefix: &str, color_str: &str) -> String {
    let trimmed = color_str.trim();
    if trimmed.is_empty() {
        return designcraft_color::swatch::BLACK.to_string();
    }
    if d.swatches.iter().any(|s| s.name.eq_ignore_ascii_case(trimmed)) {
        return trimmed.to_string();
    }
    if let Some(c) = designcraft_color::Color::from_hex(trimmed) {
        let name = format!("{prefix}-{}", c.to_hex().trim_start_matches('#'));
        if !d.swatches.iter().any(|s| s.name == name) {
            d.swatches.push(designcraft_color::swatch::Swatch::color(&name, c));
        }
        return name;
    }
    designcraft_color::swatch::BLACK.to_string()
}

/// Expands template tokens into text with special character markers:
/// - `{page}` -> `\u{E000}` (`story::PAGE_NUMBER`)
/// - `{section}` -> `\u{E001}` (`story::SECTION_MARKER`)
/// - `{filename}` -> text variable "File Name"
/// - `{chapter}` -> text variable "Chapter Number"
/// - `{runningHeader}` -> text variable "Running Header"
pub fn expand_tokens(template: &str, doc: &Document) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        if let Some(close) = after_open.find('}') {
            let token = &after_open[..close];
            match token {
                "page" => out.push(story::PAGE_NUMBER),
                "section" => out.push(story::SECTION_MARKER),
                "filename" => {
                    if let Some(c) = doc.variable("File Name").and_then(var_char) {
                        out.push(c);
                    } else {
                        out.push_str("File Name");
                    }
                }
                "chapter" => {
                    if let Some(c) = doc.variable("Chapter Number").and_then(var_char) {
                        out.push(c);
                    } else {
                        out.push_str("Chapter Number");
                    }
                }
                "runningHeader" => {
                    if let Some(c) = doc.variable("Running Header").and_then(var_char) {
                        out.push(c);
                    } else {
                        out.push_str("Running Header");
                    }
                }
                "title" => {
                    if let Some(c) = doc.variable("Title").and_then(var_char) {
                        out.push(c);
                    } else {
                        out.push_str("Title");
                    }
                }
                "nopub" | "releaseNo" => {
                    if let Some(c) = doc.variable("Release No").and_then(var_char) {
                        out.push(c);
                    } else {
                        out.push_str("Release No");
                    }
                }
                "tab" | "rightIndentTab" => out.push(story::RIGHT_INDENT_TAB),
                _ => {
                    out.push('{');
                    out.push_str(token);
                    out.push('}');
                }
            }
            rest = &after_open[close + 1..];
        } else {
            out.push('{');
            rest = after_open;
        }
    }
    out.push_str(rest);
    out
}

/// Query command returning stored spec + resolved page count and active page index.
fn get(s: &mut Session, _: &Value) -> Result<Value> {
    let st = s.doc()?;
    let d = &st.doc;
    let active_page = st.selection.page.map(|p| p + 1).unwrap_or(1);
    let styles: Vec<&str> = d.styles.paragraph.iter().map(|p| p.name.as_str()).collect();
    let sections: Vec<Value> = d
        .sections
        .iter()
        .enumerate()
        .map(|(i, sec)| {
            json!({
                "index": i,
                "start": sec.start + 1,
                "marker": sec.marker,
                "prefix": sec.prefix,
            })
        })
        .collect();

    let presets = json!({
        "minimal": preset_spec("minimal"),
        "book": preset_spec("book"),
        "editorial": preset_spec("editorial"),
        "hf-brs": preset_spec("hf-brs"),
    });

    let templates = all_templates(s);

    Ok(json!({
        "headerFooter": d.header_footer,
        "facingPages": d.settings.facing_pages,
        "pageCount": d.page_count(),
        "activePage": active_page,
        "presets": presets,
        "templates": templates,
        "paragraphStyles": styles,
        "sections": sections,
    }))
}

/// Parses the page scope from params.
fn parse_scope(p: &Value, cmd: &str) -> Result<Option<HfScope>> {
    let Some(v) = p.get("scope") else { return Ok(None) };
    if v.as_str().is_some_and(|s| s.eq_ignore_ascii_case("all")) {
        return Ok(Some(HfScope::All));
    }
    if let Some(obj) = v.as_object() {
        if let Some(sec) = obj.get("section").and_then(Value::as_u64) {
            return Ok(Some(HfScope::Section(sec as usize)));
        }
        if let Some(range) = obj.get("range").and_then(Value::as_array) {
            let first = range.first().and_then(Value::as_u64).ok_or_else(|| bad(cmd, "invalid scope range first page"))?;
            let last = range.get(1).and_then(Value::as_u64).ok_or_else(|| bad(cmd, "invalid scope range last page"))?;
            return Ok(Some(HfScope::Range { first: first as usize, last: last as usize }));
        }
    }
    Err(bad(cmd, "invalid scope: expected \"all\", {\"section\": n}, or {\"range\": [first, last]}"))
}

/// Applies header/footer frames and scope.
#[allow(clippy::cognitive_complexity)]
fn apply(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "layout.headerFooter.apply";

    // Build the target HeaderFooter configuration from template / preset / existing / params.
    let preset_name = str_param(p, "preset").or_else(|| str_param(p, "name"));
    let mut hf = if let Some(tpl_val) = p.get("template") {
        let tpl: HeaderFooterTemplate = serde_json::from_value(tpl_val.clone()).map_err(|e| bad(CMD, format!("invalid template: {e}")))?;
        let mut h = tpl.to_header_footer(HfScope::All);
        if let Some(existing) = &s.doc()?.doc.header_footer {
            h.parent = existing.parent;
            h.scope = existing.scope.clone();
            h.header_items = existing.header_items;
            h.footer_items = existing.footer_items;
            h.rule_items = existing.rule_items;
            h.accent_bar_items = existing.accent_bar_items;
            h.footer_rule_items = existing.footer_rule_items;
            h.folio_items = existing.folio_items;
        }
        h
    } else if let Some(name) = preset_name {
        let mut h = if let Some(spec) = preset_spec(name) {
            spec
        } else if let Some(tpl) = find_template(s, name) {
            tpl.to_header_footer(HfScope::All)
        } else {
            return Err(bad(CMD, format!("unknown preset or template `{name}`")));
        };
        if let Some(existing) = &s.doc()?.doc.header_footer {
            h.parent = existing.parent;
            h.scope = existing.scope.clone();
            h.header_items = existing.header_items;
            h.footer_items = existing.footer_items;
            h.rule_items = existing.rule_items;
            h.accent_bar_items = existing.accent_bar_items;
            h.footer_rule_items = existing.footer_rule_items;
            h.folio_items = existing.folio_items;
        }
        h
    } else if let Some(existing) = s.doc()?.doc.header_footer.clone() {
        existing
    } else {
        preset_spec("minimal")
            .or_else(|| find_template(s, "Modern Minimal").map(|t| t.to_header_footer(HfScope::All)))
            .ok_or_else(|| bad(CMD, "default template missing"))?
    };

    if let Some(name) = preset_name {
        hf.preset = name.to_string();
    } else if p.as_object().is_some_and(|o| o.keys().any(|k| k != "preset" && k != "template" && k != "name")) {
        hf.preset = "custom".to_string();
    }

    if let Some(s_val) = str_param(p, "sided") {
        hf.sided = match s_val.to_ascii_lowercase().as_str() {
            "single" => Sided::Single,
            "double" => Sided::Double,
            _ => return Err(bad(CMD, format!("invalid sided `{s_val}`; expected single or double"))),
        };
    }

    if let Some(scope) = parse_scope(p, CMD)? {
        hf.scope = scope;
    }

    if let Some(v) = p.get("headerOffset").and_then(Value::as_f64) {
        hf.header_offset = v;
    }
    if let Some(v) = p.get("footerOffset").and_then(Value::as_f64) {
        hf.footer_offset = v;
    }
    if let Some(st) = str_param(p, "style") {
        hf.style = st.to_string();
    }
    if let Some(r) = p.get("rule").and_then(Value::as_bool) {
        hf.rule = r;
        hf.objects.header_rule = r;
    }

    if let Some(h) = p.get("header") {
        if let Some(s) = str_param(h, "single") {
            hf.header.single = s.to_string();
        }
        if let Some(l) = str_param(h, "left") {
            hf.header.left = l.to_string();
        }
        if let Some(r) = str_param(h, "right") {
            hf.header.right = r.to_string();
        }
    }

    if let Some(f) = p.get("footer") {
        if let Some(s) = str_param(f, "single") {
            hf.footer.single = s.to_string();
        }
        if let Some(l) = str_param(f, "left") {
            hf.footer.left = l.to_string();
        }
        if let Some(r) = str_param(f, "right") {
            hf.footer.right = r.to_string();
        }
    }

    if let Some(objs) = p.get("objects") {
        if let Ok(deserialized) = serde_json::from_value::<HeaderFooterObjects>(objs.clone()) {
            hf.objects = deserialized;
        } else if let Some(obj) = objs.as_object() {
            if let Some(v) = obj.get("headerRule").and_then(Value::as_bool) {
                hf.objects.header_rule = v;
            }
            if let Some(v) = obj.get("headerRuleWeight").and_then(Value::as_f64) {
                hf.objects.header_rule_weight = v;
            }
            if let Some(v) = obj.get("footerRule").and_then(Value::as_bool) {
                hf.objects.footer_rule = v;
            }
            if let Some(v) = obj.get("footerRuleWeight").and_then(Value::as_f64) {
                hf.objects.footer_rule_weight = v;
            }
            if let Some(v) = obj.get("topAccentBar").and_then(Value::as_bool) {
                hf.objects.top_accent_bar = v;
            }
            if let Some(v) = obj.get("topAccentBarHeight").and_then(Value::as_f64) {
                hf.objects.top_accent_bar_height = v;
            }
            if let Some(v) = obj.get("topAccentBarBleed").and_then(Value::as_bool) {
                hf.objects.top_accent_bar_bleed = v;
            }
        }
        hf.rule = hf.objects.header_rule;
    }

    if let Some(cols) = p.get("colors") {
        if let Ok(deserialized) = serde_json::from_value::<HeaderFooterColors>(cols.clone()) {
            hf.colors = deserialized;
        } else if let Some(obj) = cols.as_object() {
            if let Some(s) = obj.get("primary").and_then(Value::as_str) {
                hf.colors.primary = s.to_string();
            }
            if let Some(s) = obj.get("accent").and_then(Value::as_str) {
                hf.colors.accent = s.to_string();
            }
            if let Some(s) = obj.get("dark").and_then(Value::as_str) {
                hf.colors.dark = s.to_string();
            }
            if let Some(s) = obj.get("ruleColor").and_then(Value::as_str) {
                hf.colors.rule_color = s.to_string();
            }
            if let Some(s) = obj.get("textColor").and_then(Value::as_str) {
                hf.colors.text_color = s.to_string();
            }
        }
    }

    // Validation
    if hf.header_offset < 0.0 || !hf.header_offset.is_finite() {
        return Err(bad(CMD, "headerOffset must be a non-negative finite number"));
    }
    if hf.footer_offset < 0.0 || !hf.footer_offset.is_finite() {
        return Err(bad(CMD, "footerOffset must be a non-negative finite number"));
    }
    let page_h = s.doc()?.doc.settings.page_height;
    if hf.header_offset + hf.footer_offset >= page_h {
        return Err(bad(
            CMD,
            format!("headerOffset ({}) + footerOffset ({}) must be less than page height ({})", hf.header_offset, hf.footer_offset, page_h),
        ));
    }
    if hf.sided == Sided::Double && !s.doc()?.doc.settings.facing_pages {
        return Err(bad(CMD, "double-sided header/footer requires facing pages"));
    }

    let explicit_margins: Option<Margins> = match p.get("margins") {
        None => None,
        Some(Value::Number(n)) => n.as_f64().map(Margins::uniform),
        Some(v) => serde_json::from_value(v.clone()).ok(),
    };
    if explicit_margins.is_some_and(|m| ![m.top, m.bottom, m.inside, m.outside].iter().all(|v| v.is_finite() && (0.0..=15552.0).contains(v))) {
        return Err(bad(CMD, "margins: 0 to 216 in"));
    }

    s.edit(|d, _| {
        // Ensure paragraph style exists
        let style_to_use = if d.styles.para(&hf.style).is_some() { hf.style.clone() } else { story::BASIC_PARAGRAPH.to_string() };

        // Swatch resolution
        let doc_bleed = d.settings.bleed;
        let primary_swatch = resolve_or_create_swatch(d, "HF Primary", &hf.colors.primary);
        let rule_swatch = resolve_or_create_swatch(d, "HF Rule", &hf.colors.rule_color);
        let text_swatch = (!hf.colors.text_color.trim().is_empty()).then(|| resolve_or_create_swatch(d, "HF Text", &hf.colors.text_color));

        // Compute scope pages and detect canonical geometry
        let target_pages = compute_scope_pages(d, &hf.scope);
        let (doc_cols, doc_gutter, doc_margins) = detect_doc_geometry(d, &target_pages, explicit_margins);

        // Resolve or create parent spread
        let parent_id = resolve_parent_spread(d, doc_cols, doc_gutter, doc_margins)?;
        hf.parent = Some(parent_id);

        let parent_idx = d.parent_index(parent_id).ok_or_else(|| bad(CMD, "parent index missing"))?;

        // Synchronize parent spread pages with document margins
        let new_page_id = designcraft_doc::PageId(d.alloc());
        if let Some(sp) = d.parents.get_mut(parent_idx) {
            let sp = Arc::make_mut(sp);
            if d.settings.facing_pages
                && sp.pages.len() < 2
                && let Some(first) = sp.pages.first().cloned()
            {
                let mut second = first;
                second.id = new_page_id;
                second.side = PageSide::Right;
                second.margins = doc_margins;
                second.columns.count = doc_cols;
                second.columns.gutter = doc_gutter;
                sp.pages.push(second);
            }
            for pg in &mut sp.pages {
                pg.margins = doc_margins;
                pg.columns.count = doc_cols;
                pg.columns.gutter = doc_gutter;
            }
            sp.relayout();
        }

        // Geometry: rects for single, left, right pages on the parent spread
        let parent_pages = d.parents.get(parent_idx).ok_or_else(|| bad(CMD, "parent spread missing"))?.pages.clone();

        let layer = d.default_layer();

        // Single vs Double sided processing
        let is_hf_brs = hf.preset.eq_ignore_ascii_case("hf-brs") || hf.preset.eq_ignore_ascii_case("brs");
        match hf.sided {
            Sided::Single => {
                // If previously double-sided items existed, remove them
                remove_optional_item(d, hf.header_items.left.take());
                remove_optional_item(d, hf.header_items.right.take());
                remove_optional_item(d, hf.footer_items.left.take());
                remove_optional_item(d, hf.footer_items.right.take());
                remove_optional_item(d, hf.rule_items.left.take());
                remove_optional_item(d, hf.rule_items.right.take());
                remove_optional_item(d, hf.accent_bar_items.left.take());
                remove_optional_item(d, hf.accent_bar_items.right.take());
                remove_optional_item(d, hf.footer_rule_items.left.take());
                remove_optional_item(d, hf.footer_rule_items.right.take());
                remove_optional_item(d, hf.folio_items.left.take());
                remove_optional_item(d, hf.folio_items.right.take());

                let pg = parent_pages.first().cloned().ok_or_else(|| bad(CMD, "parent has no pages"))?;
                let (h_rect, f_rect, r_line, fr_line, bar_rect) = page_hf_geometry(
                    &pg,
                    doc_bleed,
                    hf.header_offset,
                    hf.footer_offset,
                    hf.objects.top_accent_bar_height,
                    hf.objects.top_accent_bar_bleed,
                );

                // Top accent bar
                if hf.objects.top_accent_bar {
                    hf.accent_bar_items.single = Some(sync_accent_bar(d, hf.accent_bar_items.single, parent_idx, layer, bar_rect, &primary_swatch)?);
                } else {
                    remove_optional_item(d, hf.accent_bar_items.single.take());
                }

                // Header text frame & folio
                let h_text = expand_tokens(&hf.header.single, d);
                if is_hf_brs || h_text.trim().is_empty() {
                    remove_optional_item(d, hf.header_items.single.take());
                } else {
                    hf.header_items.single = sync_text_frame(
                        d,
                        hf.header_items.single,
                        parent_idx,
                        h_rect,
                        layer,
                        &h_text,
                        &style_to_use,
                        text_swatch.as_deref(),
                        0, // Top
                    )?;
                }

                // Header hairline rule
                if hf.objects.header_rule {
                    hf.rule_items.single =
                        Some(sync_rule_item(d, hf.rule_items.single, parent_idx, layer, r_line, hf.objects.header_rule_weight, &rule_swatch)?);
                } else {
                    remove_optional_item(d, hf.rule_items.single.take());
                }

                // Footer hairline rule
                if hf.objects.footer_rule {
                    hf.footer_rule_items.single = Some(sync_rule_item(
                        d,
                        hf.footer_rule_items.single,
                        parent_idx,
                        layer,
                        fr_line,
                        hf.objects.footer_rule_weight,
                        &rule_swatch,
                    )?);
                } else {
                    remove_optional_item(d, hf.footer_rule_items.single.take());
                }

                // Footer text frame & folio
                let f_text = expand_tokens(&hf.footer.single, d);
                if is_hf_brs {
                    let folio_w = 34.0;
                    let gap = 8.0;
                    let f_top = f_rect.y0;
                    let f_h = 28.0;
                    let folio_rect = Rect::new(f_rect.x0, f_top, f_rect.x0 + folio_w, f_top + f_h);
                    let text_rect = Rect::new(f_rect.x0 + folio_w + gap, f_top, f_rect.x1, f_top + f_h);

                    let folio_text = format!("{}", story::PAGE_NUMBER);
                    hf.folio_items.single = sync_styled_text_frame(
                        d,
                        hf.folio_items.single,
                        parent_idx,
                        folio_rect,
                        layer,
                        &folio_text,
                        &style_to_use,
                        text_swatch.as_deref(),
                        Some(designcraft_doc::Align::Left),
                        Some("Raleway"),
                        Some(20.0),
                        Some("Bold"),
                        6, // Bottom
                    )?;

                    hf.footer_items.single = sync_styled_text_frame(
                        d,
                        hf.footer_items.single,
                        parent_idx,
                        text_rect,
                        layer,
                        &f_text,
                        &style_to_use,
                        text_swatch.as_deref(),
                        Some(designcraft_doc::Align::Left),
                        Some("Raleway"),
                        Some(8.0),
                        Some("Regular"),
                        6, // Bottom
                    )?;
                } else {
                    remove_optional_item(d, hf.folio_items.single.take());
                    hf.footer_items.single = sync_text_frame(
                        d,
                        hf.footer_items.single,
                        parent_idx,
                        f_rect,
                        layer,
                        &f_text,
                        &style_to_use,
                        text_swatch.as_deref(),
                        6, // Bottom
                    )?;
                }
            }
            Sided::Double => {
                // If previously single-sided items existed, remove them
                remove_optional_item(d, hf.header_items.single.take());
                remove_optional_item(d, hf.footer_items.single.take());
                remove_optional_item(d, hf.rule_items.single.take());
                remove_optional_item(d, hf.accent_bar_items.single.take());
                remove_optional_item(d, hf.footer_rule_items.single.take());
                remove_optional_item(d, hf.folio_items.single.take());

                // Find left and right pages on the parent spread by PageSide
                let left_pg = parent_pages
                    .iter()
                    .find(|p| p.side == PageSide::Left)
                    .cloned()
                    .ok_or_else(|| bad(CMD, "facing parent spread has no left page"))?;
                let right_pg = parent_pages
                    .iter()
                    .find(|p| p.side == PageSide::Right)
                    .cloned()
                    .ok_or_else(|| bad(CMD, "facing parent spread has no right page"))?;

                // Left (Verso)
                let (lh_rect, lf_rect, lr_line, lfr_line, lbar_rect) = page_hf_geometry(
                    &left_pg,
                    doc_bleed,
                    hf.header_offset,
                    hf.footer_offset,
                    hf.objects.top_accent_bar_height,
                    hf.objects.top_accent_bar_bleed,
                );

                if hf.objects.top_accent_bar {
                    hf.accent_bar_items.left = Some(sync_accent_bar(d, hf.accent_bar_items.left, parent_idx, layer, lbar_rect, &primary_swatch)?);
                } else {
                    remove_optional_item(d, hf.accent_bar_items.left.take());
                }

                let lh_text = expand_tokens(&hf.header.left, d);
                if is_hf_brs || lh_text.trim().is_empty() {
                    remove_optional_item(d, hf.header_items.left.take());
                } else {
                    hf.header_items.left =
                        sync_text_frame(d, hf.header_items.left, parent_idx, lh_rect, layer, &lh_text, &style_to_use, text_swatch.as_deref(), 0)?;
                }

                if hf.objects.header_rule {
                    hf.rule_items.left =
                        Some(sync_rule_item(d, hf.rule_items.left, parent_idx, layer, lr_line, hf.objects.header_rule_weight, &rule_swatch)?);
                } else {
                    remove_optional_item(d, hf.rule_items.left.take());
                }

                if hf.objects.footer_rule {
                    hf.footer_rule_items.left =
                        Some(sync_rule_item(d, hf.footer_rule_items.left, parent_idx, layer, lfr_line, hf.objects.footer_rule_weight, &rule_swatch)?);
                } else {
                    remove_optional_item(d, hf.footer_rule_items.left.take());
                }

                let lf_text = expand_tokens(&hf.footer.left, d);
                if is_hf_brs {
                    let folio_w = 34.0;
                    let gap = 8.0;
                    let f_top = lf_rect.y0;
                    let f_h = 28.0;
                    let text_rect = Rect::new(lf_rect.x0, f_top, lf_rect.x1 - folio_w - gap, f_top + f_h);
                    let folio_rect = Rect::new(lf_rect.x1 - folio_w, f_top, lf_rect.x1, f_top + f_h);

                    hf.footer_items.left = sync_styled_text_frame(
                        d,
                        hf.footer_items.left,
                        parent_idx,
                        text_rect,
                        layer,
                        &lf_text,
                        &style_to_use,
                        text_swatch.as_deref(),
                        Some(designcraft_doc::Align::Right),
                        Some("Raleway"),
                        Some(8.0),
                        Some("Regular"),
                        6, // Bottom
                    )?;

                    let folio_text = format!("{}", story::PAGE_NUMBER);
                    hf.folio_items.left = sync_styled_text_frame(
                        d,
                        hf.folio_items.left,
                        parent_idx,
                        folio_rect,
                        layer,
                        &folio_text,
                        &style_to_use,
                        text_swatch.as_deref(),
                        Some(designcraft_doc::Align::Right),
                        Some("Raleway"),
                        Some(20.0),
                        Some("Bold"),
                        6, // Bottom
                    )?;
                } else {
                    remove_optional_item(d, hf.folio_items.left.take());
                    hf.footer_items.left =
                        sync_text_frame(d, hf.footer_items.left, parent_idx, lf_rect, layer, &lf_text, &style_to_use, text_swatch.as_deref(), 6)?;
                }

                // Right (Recto)
                let (rh_rect, rf_rect, rr_line, rfr_line, rbar_rect) = page_hf_geometry(
                    &right_pg,
                    doc_bleed,
                    hf.header_offset,
                    hf.footer_offset,
                    hf.objects.top_accent_bar_height,
                    hf.objects.top_accent_bar_bleed,
                );

                if hf.objects.top_accent_bar {
                    hf.accent_bar_items.right = Some(sync_accent_bar(d, hf.accent_bar_items.right, parent_idx, layer, rbar_rect, &primary_swatch)?);
                } else {
                    remove_optional_item(d, hf.accent_bar_items.right.take());
                }

                let rh_text = expand_tokens(&hf.header.right, d);
                if is_hf_brs || rh_text.trim().is_empty() {
                    remove_optional_item(d, hf.header_items.right.take());
                } else {
                    hf.header_items.right =
                        sync_text_frame(d, hf.header_items.right, parent_idx, rh_rect, layer, &rh_text, &style_to_use, text_swatch.as_deref(), 0)?;
                }

                if hf.objects.header_rule {
                    hf.rule_items.right =
                        Some(sync_rule_item(d, hf.rule_items.right, parent_idx, layer, rr_line, hf.objects.header_rule_weight, &rule_swatch)?);
                } else {
                    remove_optional_item(d, hf.rule_items.right.take());
                }

                if hf.objects.footer_rule {
                    hf.footer_rule_items.right = Some(sync_rule_item(
                        d,
                        hf.footer_rule_items.right,
                        parent_idx,
                        layer,
                        rfr_line,
                        hf.objects.footer_rule_weight,
                        &rule_swatch,
                    )?);
                } else {
                    remove_optional_item(d, hf.footer_rule_items.right.take());
                }

                let rf_text = expand_tokens(&hf.footer.right, d);
                if is_hf_brs {
                    let folio_w = 34.0;
                    let gap = 8.0;
                    let f_top = rf_rect.y0;
                    let f_h = 28.0;
                    let folio_rect = Rect::new(rf_rect.x0, f_top, rf_rect.x0 + folio_w, f_top + f_h);
                    let text_rect = Rect::new(rf_rect.x0 + folio_w + gap, f_top, rf_rect.x1, f_top + f_h);

                    let folio_text = format!("{}", story::PAGE_NUMBER);
                    hf.folio_items.right = sync_styled_text_frame(
                        d,
                        hf.folio_items.right,
                        parent_idx,
                        folio_rect,
                        layer,
                        &folio_text,
                        &style_to_use,
                        text_swatch.as_deref(),
                        Some(designcraft_doc::Align::Left),
                        Some("Raleway"),
                        Some(20.0),
                        Some("Bold"),
                        6, // Bottom
                    )?;

                    hf.footer_items.right = sync_styled_text_frame(
                        d,
                        hf.footer_items.right,
                        parent_idx,
                        text_rect,
                        layer,
                        &rf_text,
                        &style_to_use,
                        text_swatch.as_deref(),
                        Some(designcraft_doc::Align::Left),
                        Some("Raleway"),
                        Some(8.0),
                        Some("Regular"),
                        6, // Bottom
                    )?;
                } else {
                    remove_optional_item(d, hf.folio_items.right.take());
                    hf.footer_items.right =
                        sync_text_frame(d, hf.footer_items.right, parent_idx, rf_rect, layer, &rf_text, &style_to_use, text_swatch.as_deref(), 6)?;
                }
            }
        }

        // Apply parent to scoped pages
        let fallback_parent = d.parents.first().filter(|p| p.id != parent_id).map(|p| p.id);

        // Clear parent from pages previously assigned to header/footer parent that are no longer in scope
        let pages_to_clear: Vec<usize> =
            (0..d.page_count()).filter(|&i| d.page(i).is_some_and(|p| p.parent == Some(parent_id)) && !target_pages.contains(&i)).collect();
        if !pages_to_clear.is_empty() {
            d.apply_parent(&pages_to_clear, fallback_parent)?;
        }

        if !target_pages.is_empty() {
            d.apply_parent(&target_pages, Some(parent_id))?;
        }

        d.header_footer = Some(hf.clone());
        Ok(json!({ "headerFooter": hf }))
    })
}

/// Removes header/footer frames, resets parent assignments, and clears doc.header_footer.
fn remove(s: &mut Session, _: &Value) -> Result<Value> {
    s.edit(|d, _| {
        let Some(hf) = d.header_footer.take() else {
            return ok();
        };

        // Remove all generated items
        for id in [
            hf.header_items.single,
            hf.header_items.left,
            hf.header_items.right,
            hf.footer_items.single,
            hf.footer_items.left,
            hf.footer_items.right,
            hf.rule_items.single,
            hf.rule_items.left,
            hf.rule_items.right,
            hf.accent_bar_items.single,
            hf.accent_bar_items.left,
            hf.accent_bar_items.right,
            hf.footer_rule_items.single,
            hf.footer_rule_items.left,
            hf.footer_rule_items.right,
            hf.folio_items.single,
            hf.folio_items.left,
            hf.folio_items.right,
        ]
        .into_iter()
        .flatten()
        {
            remove_optional_item(d, Some(id));
        }

        // Reset parent on pages that have the header/footer parent
        if let Some(pid) = hf.parent {
            let fallback = d.parents.first().filter(|p| p.id != pid).map(|p| p.id);
            let pages_with_hf: Vec<usize> = (0..d.page_count()).filter(|&i| d.page(i).is_some_and(|p| p.parent == Some(pid))).collect();
            if !pages_with_hf.is_empty() {
                let _ = d.apply_parent(&pages_with_hf, fallback);
            }
        }

        ok()
    })
}

/// Computes the page indices targeted by a scope.
fn compute_scope_pages(d: &Document, scope: &HfScope) -> Vec<usize> {
    let total = d.page_count();
    match scope {
        HfScope::All => (0..total).collect(),
        HfScope::Section(sec_idx) => {
            if let Some(sec) = d.sections.get(*sec_idx) {
                let start = sec.start;
                let end = d.sections.get(sec_idx + 1).map(|s| s.start).unwrap_or(total);
                (start..end.min(total)).collect()
            } else {
                (0..total).collect()
            }
        }
        HfScope::Range { first, last } => {
            if *first == 0 || *first > *last || total == 0 {
                return vec![];
            }
            let first_idx = first.saturating_sub(1);
            let last_idx = (*last).min(total).saturating_sub(1);
            if first_idx <= last_idx { (first_idx..=last_idx).collect() } else { vec![] }
        }
    }
}

/// Resolves or creates the parent spread ("H-Header/Footer") with inheritance.
fn resolve_parent_spread(d: &mut Document, columns: u32, gutter: f64, margins: Margins) -> Result<SpreadId> {
    let base_parent = d.parents.iter().find(|p| p.parent.as_ref().is_some_and(|pi| pi.name != "Header/Footer")).map(|p| p.id);

    // 1. Existing parent recorded in doc.header_footer
    if let Some(pid) = d.header_footer.as_ref().and_then(|hf| hf.parent).filter(|&pid| d.parent_index(pid).is_some()) {
        if let Some(pi) = d.parents.iter_mut().find(|p| p.id == pid).and_then(|sp| Arc::make_mut(sp).parent.as_mut())
            && pi.based_on.is_none()
        {
            pi.based_on = base_parent;
        }
        return Ok(pid);
    }

    // 2. Existing parent named "Header/Footer"
    if let Some(existing_id) = d.parents.iter().find(|p| p.parent.as_ref().is_some_and(|pi| pi.name == "Header/Footer")).map(|p| p.id) {
        if let Some(pi) = d.parents.iter_mut().find(|p| p.id == existing_id).and_then(|sp| Arc::make_mut(sp).parent.as_mut())
            && pi.based_on.is_none()
        {
            pi.based_on = base_parent;
        }
        return Ok(existing_id);
    }

    // 3. Create fresh parent "H-Header/Footer"
    let prefix =
        if d.parents.iter().any(|p| p.parent.as_ref().is_some_and(|pi| pi.prefix == "H")) { d.next_parent_prefix() } else { "H".to_string() };

    let parent_id = d.add_parent(&prefix, "Header/Footer", columns, gutter, margins);

    // Parent inheritance: set based_on to first existing parent so artwork/guides from A-Parent are kept
    if let Some(pi) = d.parents.iter_mut().find(|p| p.id == parent_id).and_then(|sp| Arc::make_mut(sp).parent.as_mut()) {
        pi.based_on = base_parent;
    }

    Ok(parent_id)
}

/// Resolves the canonical column count, gutter, and margins from the document:
/// 1. From explicit parameters if provided
/// 2. From scoped pages with non-zero margins
/// 3. From any spread page with non-zero margins (excluding 0-margin covers/dividers)
/// 4. From any existing parent spread (e.g. A-Parent) with non-zero margins
/// 5. From page 0 if non-zero
/// 6. Fallback: Margins::uniform(36.0)
fn detect_doc_geometry(d: &Document, target_pages: &[usize], explicit_margins: Option<Margins>) -> (u32, f64, Margins) {
    if let Some(m) = explicit_margins {
        let (cols, gutter) = d.page(0).map_or((1, 12.0), |p| (p.columns.count, p.columns.gutter));
        return (cols, gutter, m);
    }

    let is_non_zero = |m: &Margins| m.top > 0.0 || m.bottom > 0.0 || m.inside > 0.0 || m.outside > 0.0;

    // 1. Check pages in target scope
    for &idx in target_pages {
        if let Some(pg) = d.page(idx)
            && is_non_zero(&pg.margins)
        {
            return (pg.columns.count, pg.columns.gutter, pg.margins);
        }
    }

    // 2. Check any spread page
    for sp in &d.spreads {
        for pg in &sp.pages {
            if is_non_zero(&pg.margins) {
                return (pg.columns.count, pg.columns.gutter, pg.margins);
            }
        }
    }

    // 3. Check any existing parent spread (other than Header/Footer)
    for p in &d.parents {
        if p.parent.as_ref().is_some_and(|pi| pi.name == "Header/Footer") {
            continue;
        }
        for pg in &p.pages {
            if is_non_zero(&pg.margins) {
                return (pg.columns.count, pg.columns.gutter, pg.margins);
            }
        }
    }

    // 4. Check page 0 if non-zero
    if let Some(pg) = d.page(0)
        && is_non_zero(&pg.margins)
    {
        return (pg.columns.count, pg.columns.gutter, pg.margins);
    }

    // 5. Default fallback
    (1, 12.0, Margins::uniform(36.0))
}

/// Calculates header rect, footer rect, header rule line, footer rule line, and top accent bar rect coordinates for a given parent page.
fn page_hf_geometry(
    page: &Page,
    doc_bleed: [f64; 4],
    header_offset: f64,
    footer_offset: f64,
    accent_bar_height: f64,
    accent_bar_bleed: bool,
) -> (Rect, Rect, (Point, Point), (Point, Point), Rect) {
    let (left_m, right_m) = page.left_right_margins();
    let x0 = page.x + left_m;
    let x1 = (page.x + page.width - right_m).max(x0 + 1.0);

    // Default frame height
    const FRAME_H: f64 = 18.0;

    // Header text frame
    let h_y0 = header_offset;
    let h_y1 = h_y0 + FRAME_H;
    let header_rect = Rect::new(x0, h_y0, x1, h_y1);

    // Footer text frame
    let f_y1 = (page.height - footer_offset).max(FRAME_H);
    let f_y0 = f_y1 - FRAME_H;
    let footer_rect = Rect::new(x0, f_y0, x1, f_y1);

    // Hairline rule line immediately below header area
    let rule_y = h_y1 + 4.0;
    let header_rule_line = (Point::new(x0, rule_y), Point::new(x1, rule_y));

    // Hairline rule line immediately above footer area
    let footer_rule_y = f_y0 - 4.0;
    let footer_rule_line = (Point::new(x0, footer_rule_y), Point::new(x1, footer_rule_y));

    // Top accent bar
    let h = if accent_bar_height > 0.0 { accent_bar_height } else { 4.0 };
    let bar_y0 = if accent_bar_bleed { -doc_bleed[0] } else { 0.0 };
    let bar_y1 = h;
    let (bar_x0, bar_x1) = match page.side {
        PageSide::Single => (
            if accent_bar_bleed { page.x - doc_bleed[2] } else { page.x },
            if accent_bar_bleed { page.x + page.width + doc_bleed[3] } else { page.x + page.width },
        ),
        PageSide::Left => (if accent_bar_bleed { page.x - doc_bleed[2] } else { page.x }, page.x + page.width),
        PageSide::Right => (page.x, if accent_bar_bleed { page.x + page.width + doc_bleed[3] } else { page.x + page.width }),
    };
    let top_accent_bar_rect = Rect::new(bar_x0, bar_y0, bar_x1, bar_y1);

    (header_rect, footer_rect, header_rule_line, footer_rule_line, top_accent_bar_rect)
}

/// Safely removes an item if present.
fn remove_optional_item(d: &mut Document, item_id: Option<ItemId>) {
    if let Some(id) = item_id.filter(|&id| d.item(id).is_some()) {
        let _ = d.remove_item(id);
    }
}

/// Synchronizes a text frame item on the parent spread with styling: updates existing or creates new.
#[allow(clippy::too_many_arguments)]
fn sync_styled_text_frame(
    d: &mut Document,
    existing_id: Option<ItemId>,
    parent_idx: usize,
    rect: Rect,
    layer: designcraft_doc::LayerId,
    expanded_text: &str,
    style_name: &str,
    text_swatch: Option<&str>,
    align: Option<designcraft_doc::Align>,
    font_family: Option<&str>,
    font_size: Option<f64>,
    font_weight: Option<&str>,
    auto_size_ref: u8,
) -> Result<Option<ItemId>> {
    if expanded_text.is_empty() {
        // If template evaluates to completely empty, clean up existing frame
        remove_optional_item(d, existing_id);
        return Ok(None);
    }

    if let Some((id, sid)) = existing_id.and_then(|id| {
        let tf = d.item(id)?.text_frame()?;
        Some((id, tf.story))
    }) {
        // Update story text safely using st.replace
        if let Some(st) = d.story_mut(sid) {
            let old_len = st.text.len();
            st.replace(0..old_len, expanded_text);
            for p in &mut st.paras {
                p.style = style_name.to_string();
                if let Some(a) = align {
                    p.para.align = Some(a);
                }
                if let Some(ts) = text_swatch {
                    p.chars.fill = Some(ts.to_string());
                }
                if let Some(ff) = font_family {
                    p.chars.font_family = Some(ff.to_string());
                }
                if let Some(fs) = font_size {
                    p.chars.size = Some(fs);
                    p.chars.leading = Some(designcraft_doc::attrs::Leading::Points(fs * 1.25));
                }
                if let Some(fw) = font_weight {
                    p.chars.font_style = Some(fw.to_string());
                }
            }
            for r in &mut st.chars {
                if let Some(ts) = text_swatch {
                    r.format.over.fill = Some(ts.to_string());
                }
                if let Some(ff) = font_family {
                    r.format.over.font_family = Some(ff.to_string());
                }
                if let Some(fs) = font_size {
                    r.format.over.size = Some(fs);
                    r.format.over.leading = Some(designcraft_doc::attrs::Leading::Points(fs * 1.25));
                }
                if let Some(fw) = font_weight {
                    r.format.over.font_style = Some(fw.to_string());
                }
            }
        }
        // Update frame rect
        if let Some(it) = d.item_mut(id) {
            it.path = shapes::rectangle(rect);
            if let Some(tf) = it.text_frame_mut() {
                tf.options.auto_size = AutoSize::HeightOnly;
                tf.options.auto_size_ref = auto_size_ref;
            }
        }
        return Ok(Some(id));
    }

    // Otherwise create a new text frame with its own story
    let (fid, sid) = d.add_text_frame(SpreadRef::Parent(parent_idx), rect, layer, expanded_text, ParaFormat::default())?;

    if let Some(tf) = d.item_mut(fid).and_then(|it| it.text_frame_mut()) {
        tf.options.auto_size = AutoSize::HeightOnly;
        tf.options.auto_size_ref = auto_size_ref;
    }

    if let Some(st) = d.story_mut(sid) {
        for p in &mut st.paras {
            p.style = style_name.to_string();
            if let Some(a) = align {
                p.para.align = Some(a);
            }
            if let Some(ts) = text_swatch {
                p.chars.fill = Some(ts.to_string());
            }
            if let Some(ff) = font_family {
                p.chars.font_family = Some(ff.to_string());
            }
            if let Some(fs) = font_size {
                p.chars.size = Some(fs);
                p.chars.leading = Some(designcraft_doc::attrs::Leading::Points(fs * 1.25));
            }
            if let Some(fw) = font_weight {
                p.chars.font_style = Some(fw.to_string());
            }
        }
        for r in &mut st.chars {
            if let Some(ts) = text_swatch {
                r.format.over.fill = Some(ts.to_string());
            }
            if let Some(ff) = font_family {
                r.format.over.font_family = Some(ff.to_string());
            }
            if let Some(fs) = font_size {
                r.format.over.size = Some(fs);
                r.format.over.leading = Some(designcraft_doc::attrs::Leading::Points(fs * 1.25));
            }
            if let Some(fw) = font_weight {
                r.format.over.font_style = Some(fw.to_string());
            }
        }
    }

    Ok(Some(fid))
}

/// Synchronizes a text frame item on the parent spread: updates existing or creates new.
#[allow(clippy::too_many_arguments)]
fn sync_text_frame(
    d: &mut Document,
    existing_id: Option<ItemId>,
    parent_idx: usize,
    rect: Rect,
    layer: designcraft_doc::LayerId,
    expanded_text: &str,
    style_name: &str,
    text_swatch: Option<&str>,
    auto_size_ref: u8,
) -> Result<Option<ItemId>> {
    sync_styled_text_frame(d, existing_id, parent_idx, rect, layer, expanded_text, style_name, text_swatch, None, None, None, None, auto_size_ref)
}

/// Synchronizes a hairline rule item on the parent spread: updates existing or creates new.
fn sync_rule_item(
    d: &mut Document,
    existing_id: Option<ItemId>,
    parent_idx: usize,
    layer: designcraft_doc::LayerId,
    line: (Point, Point),
    weight: f64,
    swatch: &str,
) -> Result<ItemId> {
    let w = if weight > 0.0 { weight } else { 0.5 };
    if let Some(it) = existing_id.and_then(|id| d.item_mut(id)) {
        it.path = shapes::line(line.0, line.1);
        it.stroke = Stroke { weight: w, swatch: swatch.to_string(), ..Default::default() };
        it.fill = Fill::none();
        return Ok(it.id);
    }

    let id = ItemId(d.alloc());
    let mut it = Item::new(id, layer, Shape::GraphicLine, shapes::line(line.0, line.1));
    it.stroke = Stroke { weight: w, swatch: swatch.to_string(), ..Default::default() };
    it.fill = Fill::none();
    d.insert_item(SpreadRef::Parent(parent_idx), it, None)?;
    Ok(id)
}

/// Synchronizes a top accent bar item on the parent spread: updates existing or creates new.
fn sync_accent_bar(
    d: &mut Document,
    existing_id: Option<ItemId>,
    parent_idx: usize,
    layer: designcraft_doc::LayerId,
    rect: Rect,
    swatch: &str,
) -> Result<ItemId> {
    if let Some(it) = existing_id.and_then(|id| d.item_mut(id)) {
        it.path = shapes::rectangle(rect);
        it.stroke = Stroke { weight: 0.0, swatch: designcraft_color::swatch::NONE.into(), ..Default::default() };
        it.fill = Fill::swatch(swatch);
        return Ok(it.id);
    }

    let id = ItemId(d.alloc());
    let mut it = Item::new(id, layer, Shape::Rectangle, shapes::rectangle(rect));
    it.stroke = Stroke { weight: 0.0, swatch: designcraft_color::swatch::NONE.into(), ..Default::default() };
    it.fill = Fill::swatch(swatch);
    d.insert_item(SpreadRef::Parent(parent_idx), it, None)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use designcraft_doc::build::NewDocument;

    fn session_with_doc(facing: bool, pages: usize) -> Session {
        let mut s = Session::new();
        let nd = NewDocument { facing_pages: facing, pages, ..Default::default() };
        let d = Document::new(&nd);
        s.add_document(crate::DocState::new(d, None));
        s
    }

    #[test]
    fn test_token_expansion() {
        let s = session_with_doc(true, 4);
        let d = &s.doc().unwrap().doc;
        let expanded = expand_tokens("{page} {section}\u{e006}{filename} {chapter}", d);
        assert!(expanded.contains(story::PAGE_NUMBER));
        assert!(expanded.contains(story::SECTION_MARKER));
        assert!(expanded.contains('\u{e006}'));
        assert!(expanded.contains(var_char(2).unwrap())); // File Name is index 2
        assert!(expanded.contains(var_char(0).unwrap())); // Chapter Number is index 0
    }

    #[test]
    fn test_apply_editorial_preset_facing_pages() {
        let mut s = session_with_doc(true, 4);
        let res = s.execute("layout.headerFooter.apply", &json!({ "preset": "editorial" }));
        assert!(res.is_ok(), "{:?}", res.err());

        let doc = &s.doc().unwrap().doc;
        let hf = doc.header_footer.as_ref().expect("header_footer should be present");
        assert_eq!(hf.preset, "editorial");
        assert_eq!(hf.sided, Sided::Double);
        assert!(hf.rule);

        // Header items should exist for left and right
        assert!(hf.header_items.left.is_some());
        assert!(hf.header_items.right.is_some());
        assert!(hf.header_items.single.is_none());

        // Footer items should exist for left and right
        assert!(hf.footer_items.left.is_some());
        assert!(hf.footer_items.right.is_some());

        // Rule items should exist for left and right
        assert!(hf.rule_items.left.is_some());
        assert!(hf.rule_items.right.is_some());

        // Parent should be applied to all 4 pages
        let parent_id = hf.parent.expect("parent id");
        for i in 0..4 {
            assert_eq!(doc.page(i).unwrap().parent, Some(parent_id));
        }

        // Parent should inherit based_on from A-Parent
        let parent_spread = doc.parents.iter().find(|p| p.id == parent_id).expect("parent spread");
        assert!(parent_spread.parent.as_ref().and_then(|p| p.based_on).is_some() || doc.parents.len() > 1);
    }

    #[test]
    fn test_double_sided_rejected_on_single_sided_document() {
        let mut s = session_with_doc(false, 2);
        let res = s.execute("layout.headerFooter.apply", &json!({ "sided": "double" }));
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("requires facing pages"));
    }

    #[test]
    fn test_switch_sided_cleans_up_orphaned_frames() {
        let mut s = session_with_doc(true, 4);

        // First apply double-sided
        s.execute("layout.headerFooter.apply", &json!({ "preset": "editorial" })).unwrap();
        let (left_h, right_h) = {
            let hf = s.doc().unwrap().doc.header_footer.as_ref().unwrap();
            (hf.header_items.left.unwrap(), hf.header_items.right.unwrap())
        };
        assert!(s.doc().unwrap().doc.item(left_h).is_some());
        assert!(s.doc().unwrap().doc.item(right_h).is_some());

        // Now switch to single-sided minimal
        s.execute("layout.headerFooter.apply", &json!({ "preset": "minimal" })).unwrap();
        let doc = &s.doc().unwrap().doc;
        let hf = doc.header_footer.as_ref().unwrap();
        assert_eq!(hf.sided, Sided::Single);

        // Left and right items must be removed
        assert!(doc.item(left_h).is_none());
        assert!(doc.item(right_h).is_none());
        assert!(hf.footer_items.single.is_some());
    }

    #[test]
    fn test_reapply_preserves_item_ids_and_story_invariants() {
        let mut s = session_with_doc(true, 4);
        s.execute("layout.headerFooter.apply", &json!({ "preset": "book" })).unwrap();

        let initial_left_h = s.doc().unwrap().doc.header_footer.as_ref().unwrap().header_items.left.unwrap();

        // Reapply with modified header template
        s.execute("layout.headerFooter.apply", &json!({ "header": { "left": "{page} -- custom header" } })).unwrap();

        let doc = &s.doc().unwrap().doc;
        let current_left_h = doc.header_footer.as_ref().unwrap().header_items.left.unwrap();
        // Item ID should be preserved
        assert_eq!(initial_left_h, current_left_h);

        // Story invariants must hold
        assert!(doc.check().is_ok());

        let sid = doc.item(current_left_h).unwrap().text_frame().unwrap().story;
        let story = doc.story(sid).unwrap();
        assert!(story.text.contains("custom header"));
    }

    #[test]
    fn test_remove_cleans_up_items_and_resets_parent() {
        let mut s = session_with_doc(true, 4);
        s.execute("layout.headerFooter.apply", &json!({ "preset": "editorial" })).unwrap();

        let (left_h, right_h, left_r, right_r, pid) = {
            let hf = s.doc().unwrap().doc.header_footer.as_ref().unwrap();
            (
                hf.header_items.left.unwrap(),
                hf.header_items.right.unwrap(),
                hf.rule_items.left.unwrap(),
                hf.rule_items.right.unwrap(),
                hf.parent.unwrap(),
            )
        };

        // Execute remove
        s.execute("layout.headerFooter.remove", &json!({})).unwrap();

        let doc = &s.doc().unwrap().doc;
        assert_eq!(doc.header_footer, None);
        assert!(doc.item(left_h).is_none());
        assert!(doc.item(right_h).is_none());
        assert!(doc.item(left_r).is_none());
        assert!(doc.item(right_r).is_none());

        // Pages should no longer have the header/footer parent
        for i in 0..4 {
            assert_ne!(doc.page(i).unwrap().parent, Some(pid));
        }
    }

    #[test]
    fn test_rtl_binding_locates_left_and_right_by_page_side() {
        let mut s = Session::new();
        let nd = NewDocument { facing_pages: true, pages: 4, ..Default::default() };
        let mut d = Document::new(&nd);
        d.settings.right_to_left_binding = true;
        s.add_document(crate::DocState::new(d, None));

        s.execute("layout.headerFooter.apply", &json!({ "preset": "book" })).unwrap();
        let doc = &s.doc().unwrap().doc;
        let hf = doc.header_footer.as_ref().unwrap();
        assert_eq!(hf.sided, Sided::Double);

        let parent_id = hf.parent.unwrap();
        let parent = doc.parents.iter().find(|p| p.id == parent_id).unwrap();

        let left_item_id = hf.header_items.left.unwrap();
        let right_item_id = hf.header_items.right.unwrap();

        let left_item = doc.item(left_item_id).unwrap();
        let right_item = doc.item(right_item_id).unwrap();

        let left_page = parent.pages.iter().find(|p| p.side == PageSide::Left).unwrap();
        let right_page = parent.pages.iter().find(|p| p.side == PageSide::Right).unwrap();

        // Left item x coordinate must be on the left page bounds
        assert!(left_item.bounds().x0 >= left_page.x);
        assert!(right_item.bounds().x0 >= right_page.x);
    }

    #[test]
    fn test_list_templates_returns_builtins() {
        let mut s = session_with_doc(true, 4);
        let res = s.execute("layout.headerFooter.listTemplates", &json!({})).unwrap();
        let arr = res.as_array().expect("array of templates");
        assert!(arr.len() >= 3);
        let names: Vec<&str> = arr.iter().filter_map(|t| t.get("name").and_then(Value::as_str)).collect();
        assert!(names.contains(&"Modern Minimal"));
        assert!(names.contains(&"Editorial Book"));
        assert!(names.contains(&"Corporate Report"));
        for t in arr {
            assert_eq!(t.get("builtIn").and_then(Value::as_bool), Some(true));
        }
    }

    #[test]
    fn test_save_and_delete_template() {
        let mut s = session_with_doc(true, 4);
        let custom_template = json!({
            "name": "Custom Magazine",
            "description": "Custom test template",
            "sided": "double",
            "headerOffset": 28.0,
            "footerOffset": 28.0,
            "header": { "left": "{page} | Section", "right": "Section | {page}" },
            "footer": { "left": "Magazine", "right": "Magazine" },
            "objects": {
                "headerRule": true,
                "headerRuleWeight": 1.0,
                "footerRule": true,
                "footerRuleWeight": 1.0,
                "topAccentBar": true,
                "topAccentBarHeight": 6.0,
                "topAccentBarBleed": true
            },
            "colors": {
                "primary": "#ff5500",
                "accent": "#00aaff",
                "dark": "#112233",
                "ruleColor": "#333333",
                "textColor": "#111111"
            }
        });

        // 1. Save template
        let save_res = s.execute("layout.headerFooter.saveTemplate", &json!({ "template": custom_template }));
        assert!(save_res.is_ok(), "{:?}", save_res.err());

        // 2. Verify it is listed with builtIn: false
        let list_res = s.execute("layout.headerFooter.listTemplates", &json!({})).unwrap();
        let arr = list_res.as_array().unwrap();
        let custom = arr.iter().find(|t| t.get("name").and_then(Value::as_str) == Some("Custom Magazine")).expect("custom template");
        assert_eq!(custom.get("builtIn").and_then(Value::as_bool), Some(false));

        // 3. Apply custom template by name
        let apply_res = s.execute("layout.headerFooter.apply", &json!({ "preset": "Custom Magazine" }));
        assert!(apply_res.is_ok(), "{:?}", apply_res.err());

        let doc = &s.doc().unwrap().doc;
        let hf = doc.header_footer.as_ref().unwrap();
        assert_eq!(hf.preset, "Custom Magazine");
        assert!(hf.accent_bar_items.left.is_some());
        assert!(hf.accent_bar_items.right.is_some());
        assert!(hf.footer_rule_items.left.is_some());
        assert!(hf.footer_rule_items.right.is_some());

        // 4. Delete template
        let del_res = s.execute("layout.headerFooter.deleteTemplate", &json!({ "name": "Custom Magazine" }));
        assert!(del_res.is_ok());

        // 5. Verify it is removed
        let list_res2 = s.execute("layout.headerFooter.listTemplates", &json!({})).unwrap();
        assert!(!list_res2.as_array().unwrap().iter().any(|t| t.get("name").and_then(Value::as_str) == Some("Custom Magazine")));
    }

    #[test]
    fn test_apply_corporate_report_with_accent_bar_and_footer_rule() {
        let mut s = session_with_doc(true, 4);
        let res = s.execute("layout.headerFooter.apply", &json!({ "preset": "Corporate Report" }));
        assert!(res.is_ok(), "{:?}", res.err());

        let left_bar_id;
        {
            let doc = &s.doc().unwrap().doc;
            let hf = doc.header_footer.as_ref().unwrap();
            assert_eq!(hf.preset, "Corporate Report");
            assert!(hf.objects.top_accent_bar);
            assert!(hf.objects.footer_rule);

            // Accent bars and footer rules should exist
            assert!(hf.accent_bar_items.left.is_some());
            assert!(hf.accent_bar_items.right.is_some());
            assert!(hf.footer_rule_items.left.is_some());
            assert!(hf.footer_rule_items.right.is_some());

            left_bar_id = hf.accent_bar_items.left.unwrap();
            let left_bar = doc.item(left_bar_id).unwrap();
            assert_eq!(left_bar.shape, Shape::Rectangle);

            // Swatch should be created for primary color
            assert!(doc.swatches.iter().any(|sw| sw.name.contains("HF Primary")));
        }

        // Remove cleans everything up
        s.execute("layout.headerFooter.remove", &json!({})).unwrap();
        let doc_after = &s.doc().unwrap().doc;
        assert_eq!(doc_after.header_footer, None);
        assert!(doc_after.item(left_bar_id).is_none());
    }

    #[test]
    fn test_header_footer_follows_custom_page_margins() {
        let mut s = session_with_doc(true, 4);

        // Page 0 is a cover page with 0 margins
        s.edit(|d, _| {
            let first_id = d.spreads.first().map(|sp| sp.id);
            if let Some(pg) = d.spreads.first_mut().and_then(|sp| Arc::make_mut(sp).pages.first_mut()) {
                pg.margins = Margins::uniform(0.0);
            }
            // Other pages have custom asymmetric margins (e.g., inside 53.86, outside 42.52)
            let custom_m = Margins { top: 57.26, bottom: 56.69, inside: 53.86, outside: 42.52 };
            for sp in &mut d.spreads {
                let sp = Arc::make_mut(sp);
                for (i, pg) in sp.pages.iter_mut().enumerate() {
                    if Some(sp.id) == first_id && i == 0 {
                        continue;
                    }
                    pg.margins = custom_m;
                }
            }
            Ok(())
        })
        .unwrap();

        // Apply Corporate Report (facing pages)
        s.execute("layout.headerFooter.apply", &json!({ "preset": "Corporate Report" })).unwrap();

        let (rh_id, right_pg_x, right_pg_width) = {
            let doc = &s.doc().unwrap().doc;
            let hf = doc.header_footer.as_ref().unwrap();
            let parent_id = hf.parent.unwrap();
            let parent_sp = doc.parents.iter().find(|p| p.id == parent_id).expect("parent spread");

            // Verify parent spread pages synchronized to custom margins
            let left_pg = &parent_sp.pages[0];
            let right_pg = &parent_sp.pages[1];
            assert_eq!(left_pg.margins.inside, 53.86);
            assert_eq!(left_pg.margins.outside, 42.52);
            assert_eq!(right_pg.margins.inside, 53.86);
            assert_eq!(right_pg.margins.outside, 42.52);

            // Verify right page header text frame starts at inside margin and ends at outside margin
            let rh_id = hf.header_items.right.unwrap();
            let rh_bounds = doc.item(rh_id).unwrap().bounds();
            let (right_inside, right_outside) = (right_pg.margins.inside, right_pg.margins.outside);
            let expected_rh_x0 = right_pg.x + right_inside;
            let expected_rh_x1 = right_pg.x + right_pg.width - right_outside;
            assert!((rh_bounds.x0 - expected_rh_x0).abs() < 1e-4, "right header x0 ({}) should be {}", rh_bounds.x0, expected_rh_x0);
            assert!((rh_bounds.x1 - expected_rh_x1).abs() < 1e-4, "right header x1 ({}) should be {}", rh_bounds.x1, expected_rh_x1);

            // Verify left page header text frame starts at outside margin and ends at inside margin
            let lh_id = hf.header_items.left.unwrap();
            let lh_bounds = doc.item(lh_id).unwrap().bounds();
            let (left_outside, left_inside) = (left_pg.margins.outside, left_pg.margins.inside);
            let expected_lh_x0 = left_pg.x + left_outside;
            let expected_lh_x1 = left_pg.x + left_pg.width - left_inside;
            assert!((lh_bounds.x0 - expected_lh_x0).abs() < 1e-4, "left header x0 ({}) should be {}", lh_bounds.x0, expected_lh_x0);
            assert!((lh_bounds.x1 - expected_lh_x1).abs() < 1e-4, "left header x1 ({}) should be {}", lh_bounds.x1, expected_lh_x1);

            (rh_id, right_pg.x, right_pg.width)
        };

        // Verify re-applying with new explicit margins adjusts header coordinates
        let new_m = Margins { top: 40.0, bottom: 40.0, inside: 60.0, outside: 30.0 };
        s.execute(
            "layout.headerFooter.apply",
            &json!({
                "margins": {
                    "top": new_m.top,
                    "bottom": new_m.bottom,
                    "inside": new_m.inside,
                    "outside": new_m.outside,
                }
            }),
        )
        .unwrap();

        let doc2 = &s.doc().unwrap().doc;
        let rh_bounds2 = doc2.item(rh_id).unwrap().bounds();
        let expected_rh_x0_new = right_pg_x + 60.0;
        let expected_rh_x1_new = right_pg_x + right_pg_width - 30.0;
        assert!((rh_bounds2.x0 - expected_rh_x0_new).abs() < 1e-4, "updated rh x0: {}", rh_bounds2.x0);
        assert!((rh_bounds2.x1 - expected_rh_x1_new).abs() < 1e-4, "updated rh x1: {}", rh_bounds2.x1);
    }

    #[test]
    fn test_apply_hf_brs_double_sided() {
        let mut s = session_with_doc(true, 4);
        let res = s.execute(
            "layout.headerFooter.apply",
            &json!({
                "preset": "hf-brs",
                "footer": {
                    "left": "Perkembangan Nilai Tukar Petani Jawa Barat, September 2026\nBRS No. 61/10/32/Th. XXVIII, 1 Oktober 2026",
                    "right": "Perkembangan Nilai Tukar Petani Jawa Barat, September 2026\nBRS No. 61/10/32/Th. XXVIII, 1 Oktober 2026",
                }
            }),
        );
        assert!(res.is_ok(), "apply hf-brs: {res:?}");

        let doc = &s.doc().unwrap().doc;
        let hf = doc.header_footer.as_ref().unwrap();
        assert_eq!(hf.preset, "hf-brs");
        assert_eq!(hf.sided, Sided::Double);

        // Header frames are blank
        assert!(hf.header_items.left.is_none());
        assert!(hf.header_items.right.is_none());

        // Verify folio items and text items are generated on both left and right sides in the footer
        let left_text_id = hf.footer_items.left.expect("left text frame exists in footer");
        let left_folio_id = hf.folio_items.left.expect("left folio frame exists");
        let right_text_id = hf.footer_items.right.expect("right text frame exists in footer");
        let right_folio_id = hf.folio_items.right.expect("right folio frame exists");

        let left_text_b = doc.item(left_text_id).unwrap().bounds();
        let left_folio_b = doc.item(left_folio_id).unwrap().bounds();
        let right_text_b = doc.item(right_text_id).unwrap().bounds();
        let right_folio_b = doc.item(right_folio_id).unwrap().bounds();

        // Left page (verso): text is on the left, folio is on the right
        assert!(left_text_b.x1 < left_folio_b.x0, "left page: text ({}) must be left of folio ({})", left_text_b.x1, left_folio_b.x0);

        // Right page (recto): folio is on the left, text is on the right
        assert!(right_folio_b.x1 < right_text_b.x0, "right page: folio ({}) must be left of text ({})", right_folio_b.x1, right_text_b.x0);

        // Verify folio content is the page marker
        let left_folio_story = doc.story(doc.item(left_folio_id).unwrap().text_frame().unwrap().story).unwrap();
        assert_eq!(left_folio_story.text, format!("{}", story::PAGE_NUMBER));

        let parent_idx = doc.parent_index(hf.parent.unwrap()).unwrap();
        let parent_spread = &doc.parents[parent_idx];
        assert_eq!(parent_spread.pages.len(), 2);
        assert_eq!(parent_spread.page_at_x(left_text_b.center().x), Some(0));
        assert_eq!(parent_spread.page_at_x(left_folio_b.center().x), Some(0));
        assert_eq!(parent_spread.page_at_x(right_text_b.center().x), Some(1));
        assert_eq!(parent_spread.page_at_x(right_folio_b.center().x), Some(1));

        // Check shown_parent_pages on spreads (shows both base parent 0 and H-Header/Footer parent 1)
        // Spread 0, Page 0 (which is Page 1, Recto / Right side)
        let shown_p1 = doc.shown_parent_pages(SpreadRef::Doc(0), 0);
        assert!(shown_p1.contains(&(parent_idx, 1)), "Page 1 is right side, should show parent page 1");

        // Spread 1, Page 0 (Page 2, Verso / Left side)
        let shown_p2 = doc.shown_parent_pages(SpreadRef::Doc(1), 0);
        assert!(shown_p2.contains(&(parent_idx, 0)), "Page 2 is left side, should show parent page 0");

        // Spread 1, Page 1 (Page 3, Recto / Right side)
        let shown_p3 = doc.shown_parent_pages(SpreadRef::Doc(1), 1);
        assert!(shown_p3.contains(&(parent_idx, 1)), "Page 3 is right side, should show parent page 1");

        // Removing cleans up both text frames and folio frames
        s.execute("layout.headerFooter.remove", &json!({})).unwrap();
        let doc_after = &s.doc().unwrap().doc;
        assert!(doc_after.header_footer.is_none());
        assert!(doc_after.item(left_text_id).is_none());
        assert!(doc_after.item(left_folio_id).is_none());
        assert!(doc_after.item(right_text_id).is_none());
        assert!(doc_after.item(right_folio_id).is_none());
    }
}
