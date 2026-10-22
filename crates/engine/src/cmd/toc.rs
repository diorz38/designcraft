//! Layout → Table of Contents: list the paragraphs of chosen styles with their page names, as a
//! story with "TOC Title" / "TOC Level n" styles (right tab with dot leader) or the entry styles
//! chosen per style. Update Table of Contents regenerates the same story after edits.
//!
//! A document holds several tables of contents (Contents, Tables, Charts): `toc.generate` adds one
//! per page (replacing the one already there), `toc.update` regenerates them all. An entry with
//! `pageTitle` lists only paragraphs whose page begins with the given words: the first text box on
//! the page starts with "Tabel" or "Table" for a list of tables, "Grafik" or "Chart" for charts.

use designcraft_compose::{ComposeOptions, caret, compose_story};
use designcraft_doc::{
    Align, Document, NO_CHAR_STYLE, ParaAttrs, ParaFormat, ParagraphStyle, SpreadRef, StoryId, TabAlign, TabStop, Toc, TocEntry, TocPageNumber,
};
use designcraft_geom::Rect;
use serde_json::{Value, json};

use super::{CommandSpec, bad, cmd, has_doc, str_param};
use crate::{Result, Session};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "toc.generate",
            "Table of Contents…",
            ["Layout"],
            None,
            "{entries: [{style, level?: 1 (1-9), entryStyle?: paragraph style of its entries (default \"TOC Level n\"), pageNumber?: \"after\"|\"before\"|\"none\", separator?: text between entry and number (default a tab), pageNumberStyle?: character style of the page number, separatorStyle?: character style of the separator, sort?: bool (Sort Entries in Alphabetical Order: among their siblings, each with the entries under it), pageTitle?: [words] (list only paragraphs whose page's first text box starts with one of these, e.g. [\"Tabel\", \"Table\"]), chapterNumber?: bool (prefix the entry with the page's first text box: the chapter line \"BAB 01\" before \"Geografi\")}] | styles: [names] (levels 1, 2, …), title?: \"Contents\", titleStyle?: paragraph style of the title (default \"TOC Title\"), pageNumbers?: true, replace?: true (regenerate the table of contents on the target page in place; false makes a new frame), page?: 1-based page for a new frame (its margins), rect?: [x0,y0,x1,y1] (spread coords)} → {story, entries, tocs}",
            has_doc,
            generate
        ),
        cmd!(
            "toc.update",
            "Update Table of Contents",
            ["Layout"],
            None,
            "{} — regenerate every table of contents in the document",
            has_toc,
            |s, _| {
                s.edit(|d, _| {
                    let tocs = d.tocs.clone();
                    if tocs.is_empty() {
                        return Err(bad("toc.update", "no table of contents"));
                    }
                    let mut entries = 0;
                    for toc in &tocs {
                        entries += fill(d, toc, None)?;
                    }
                    Ok(json!({"entries": entries, "tocs": tocs.len()}))
                })
            }
        ),
        cmd!(query "toc.entries", "Table of Contents Entries", [], None, "{entries | styles} → [{level, text, page}] without changing the document", has_doc, |s, p| {
            let entries = entries_param(p)?;
            let d = &s.doc()?.doc;
            let skip = toc_stories(d);
            let found = sort_entries(collect(d, &entries, &skip), &entries);
            Ok(json!(found.into_iter().map(|e| json!({"level": e.level, "text": e.text, "page": e.page})).collect::<Vec<_>>()))
        }),
    ]
}

fn has_toc(s: &Session) -> std::result::Result<(), String> {
    match s.doc() {
        Ok(st) if st.doc.tocs.iter().any(|t| st.doc.story(t.story).is_some()) => Ok(()),
        _ => Err("the document has no table of contents".into()),
    }
}

/// The stories the tables of contents generate: never listed as entries themselves.
fn toc_stories(d: &Document) -> Vec<StoryId> {
    d.tocs.iter().map(|t| t.story).collect()
}

const MAX_ENTRIES: usize = 64;
const MAX_NAME: usize = 256;
const MAX_TITLE: usize = 1024;
const MAX_SEPARATOR: usize = 64;
/// Words a `pageTitle` filter may test (each up to `MAX_TITLE` characters).
pub const MAX_PAGE_TITLE_WORDS: usize = 8;

fn entries_param(p: &Value) -> Result<Vec<TocEntry>> {
    let mut entries: Vec<TocEntry> = if let Some(v) = p.get("entries") {
        if v.as_array().is_some_and(|a| a.len() > MAX_ENTRIES) {
            return Err(bad("toc.generate", format!("at most {MAX_ENTRIES} entries")));
        }
        serde_json::from_value(v.clone()).map_err(|e| bad("toc.generate", e.to_string()))?
    } else if let Some(a) = p.get("styles").and_then(Value::as_array) {
        if a.len() > MAX_ENTRIES {
            return Err(bad("toc.generate", format!("at most {MAX_ENTRIES} styles")));
        }
        a.iter().enumerate().filter_map(|(i, v)| v.as_str().map(|s| TocEntry::new(s, (i + 1).min(9) as u8))).collect()
    } else {
        return Err(bad("toc.generate", "give `entries` or `styles`"));
    };
    for e in &mut entries {
        if !(1..=9).contains(&e.level) {
            return Err(bad("toc.generate", format!("level {} of `{}` is not 1–9", e.level, e.style)));
        }
        let names = [&e.entry_style, &e.page_number_style, &e.separator_style];
        if e.style.chars().count() > MAX_NAME || names.iter().any(|n| n.as_ref().is_some_and(|n| n.chars().count() > MAX_NAME)) {
            return Err(bad("toc.generate", format!("style names are at most {MAX_NAME} characters")));
        }
        if let Some(sep) = &mut e.separator {
            if sep.chars().count() > MAX_SEPARATOR {
                return Err(bad("toc.generate", format!("the separator is at most {MAX_SEPARATOR} characters")));
            }
            *sep = one_line(sep);
        }
        if e.page_title.len() > MAX_PAGE_TITLE_WORDS {
            return Err(bad("toc.generate", format!("at most {MAX_PAGE_TITLE_WORDS} page titles per entry")));
        }
        for w in &mut e.page_title {
            if w.chars().count() > MAX_NAME {
                return Err(bad("toc.generate", format!("page titles are at most {MAX_NAME} characters")));
            }
            *w = one_line(w).trim().to_string();
        }
        e.page_title.retain(|w| !w.is_empty());
        e.page_title.dedup();
        // The defaults are stored as `None`, so a definition reads the same however it was made.
        if e.entry_style.as_deref() == Some("") {
            e.entry_style = None;
        }
        if e.separator.as_deref() == Some("\t") {
            e.separator = None;
        }
        for n in [&mut e.page_number_style, &mut e.separator_style] {
            if n.as_deref().is_some_and(|n| n.is_empty() || n == NO_CHAR_STYLE) {
                *n = None;
            }
        }
    }
    Ok(entries)
}

/// Paragraph breaks in a title or separator would shift the paragraph styles after them.
fn one_line(s: &str) -> String {
    s.chars().map(|c| if matches!(c, '\n' | '\r' | '\u{2029}') { ' ' } else { c }).collect()
}

fn para_style_exists(d: &Document, name: &str) -> Result<()> {
    match d.styles.para(name) {
        Some(_) => Ok(()),
        None => Err(bad("toc.generate", format!("no paragraph style `{name}`"))),
    }
}

fn char_style_exists(d: &Document, name: &str) -> Result<()> {
    match d.styles.char_style(name) {
        Some(_) => Ok(()),
        None => Err(bad("toc.generate", format!("no character style `{name}`"))),
    }
}

/// Sort Entries in Alphabetical Order: the entries of sorted styles change places with each
/// other among their siblings (entries under the same entry of a higher level), each taking the
/// lower-level entries under it along. Other entries keep their places.
fn sort_entries(found: Vec<Found>, entries: &[TocEntry]) -> Vec<Found> {
    if !entries.iter().any(|e| e.sort) {
        return found;
    }
    // Siblings: each entry with the deeper entries that follow it.
    let mut blocks: Vec<Vec<Found>> = Vec::new();
    for f in found {
        match blocks.last_mut() {
            Some(b) if b.first().is_some_and(|head| f.level > head.level) => b.push(f),
            _ => blocks.push(vec![f]),
        }
    }
    // Children first; a child is always deeper than its head, so this stops after nine levels.
    let mut blocks: Vec<Vec<Found>> = blocks
        .into_iter()
        .map(|mut b| {
            let children = b.split_off(1.min(b.len()));
            b.extend(sort_entries(children, entries));
            b
        })
        .collect();
    let sorted = |b: &Vec<Found>| b.first().and_then(|h| entries.get(h.entry)).is_some_and(|e| e.sort);
    let slots: Vec<usize> = (0..blocks.len()).filter(|&i| sorted(&blocks[i])).collect();
    let mut moved: Vec<Vec<Found>> = slots.iter().map(|&i| std::mem::take(&mut blocks[i])).collect();
    let key = |b: &Vec<Found>| b.first().map(|h| (h.text.to_lowercase(), h.text.clone())).unwrap_or_default();
    moved.sort_by_cached_key(key);
    for (i, b) in slots.into_iter().zip(moved) {
        blocks[i] = b;
    }
    blocks.into_iter().flatten().collect()
}

pub struct Found {
    pub level: u8,
    pub text: String,
    pub page: String,
    entry: usize,
    key: (usize, i64, i64),
}

/// Paragraphs in `entries` styles across the document, in page order. `skip` holds the stories the
/// tables of contents generate (never listed as entries).
pub fn collect(d: &Document, entries: &[TocEntry], skip: &[StoryId]) -> Vec<Found> {
    let mut out = Vec::new();
    // Page → its first text box's text, for `page_title` filters (one lookup per page).
    let mut page_titles: std::collections::HashMap<usize, Option<String>> = std::collections::HashMap::new();
    for (sid, st) in &d.stories {
        if skip.contains(sid) || !st.paras.iter().any(|f| entries.iter().any(|e| e.style == f.style)) {
            continue;
        }
        let cs = compose_story(d, *sid, &ComposeOptions::default());
        for (pi, r) in st.para_ranges().into_iter().enumerate() {
            let Some((ei, e)) = entries.iter().enumerate().find(|(_, e)| e.style == st.paras[pi].style) else { continue };
            let text: String = st.text[r.clone()]
                .chars()
                .filter(|c| !('\u{E000}'..='\u{E1FF}').contains(c))
                .map(|c| if c == '\t' || c == '\u{2028}' { ' ' } else { c })
                .collect();
            let text = text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            // Overset paragraphs have no page and are left out (as InDesign does).
            let Some((fi, x, baseline, ..)) = caret(&cs, r.start) else { continue };
            let Some(ft) = cs.frames.get(fi) else { continue };
            if r.start >= ft.range.end && r.start != ft.range.start {
                continue;
            }
            let Some(loc) = d.find(ft.frame) else { continue };
            let SpreadRef::Doc(si) = loc.spread else { continue };
            let Some(item) = d.item(ft.frame) else { continue };
            let b = item.bounds();
            let in_spread = d.spreads[si].page_at_x(b.center().x).unwrap_or(0);
            let abs = d.first_page_of_spread(si) + in_spread;
            // "Tabel"/"Table" and "Grafik"/"Chart" lists test the first text box of the page.
            if !e.page_title.is_empty() {
                let title = page_titles.entry(abs).or_insert_with(|| page_first_text(d, abs));
                if !title.as_deref().is_some_and(|t| title_matches(t, &e.page_title)) {
                    continue;
                }
            }
            // The chapter line: the page's first text box ("BAB 01") before the entry's own
            // text ("Geografi" → "BAB 01 Geografi"). A page whose first text box is the entry
            // itself keeps its own text.
            let text = if e.chapter_number {
                let prefix = page_titles.entry(abs).or_insert_with(|| page_first_text(d, abs));
                match prefix.as_deref() {
                    Some(p) if !p.trim().is_empty() && !p.trim().eq_ignore_ascii_case(&text) => format!("{} {}", p.trim(), text),
                    _ => text,
                }
            } else {
                text
            };
            out.push(Found {
                level: e.level,
                text,
                entry: ei,
                page: d.page_name(abs),
                key: (abs, ((b.y0 + baseline) * 10.0) as i64, ((b.x0 + x) * 10.0) as i64),
            });
        }
    }
    out.sort_by_key(|f| f.key);
    out
}

/// The text of the first text box on absolute page `abs`: the topmost, then leftmost, text frame
/// whose center sits on that page. `None` when the page has no text box.
fn page_first_text(d: &Document, abs: usize) -> Option<String> {
    let (si, _) = d.page_loc(abs)?;
    let spread = d.spreads.get(si)?;
    let mut best: Option<(i64, i64, StoryId)> = None;
    for it in &spread.items {
        let Some(tf) = it.text_frame() else { continue };
        if d.page_of_item(it.id) != Some(abs) {
            continue;
        }
        let b = it.bounds();
        if !b.x0.is_finite() || !b.y0.is_finite() {
            continue;
        }
        let key = ((b.y0 * 10.0) as i64, (b.x0 * 10.0) as i64);
        if best.is_none_or(|(y, x, _)| key < (y, x)) {
            best = Some((key.0, key.1, tf.story));
        }
    }
    let story = d.story(best?.2)?;
    Some(story.text.lines().next().unwrap_or("").trim().to_string())
}

/// Whether `text` starts with one of `words` (case-insensitive, whole-word-ish: the text box reads
/// "Tabel 01.01. Jarak …", which starts with "Tabel").
fn title_matches(text: &str, words: &[String]) -> bool {
    let text = text.trim_start();
    words.iter().any(|w| {
        let w = w.trim();
        !w.is_empty() && text.get(..w.len()).is_some_and(|head| head.eq_ignore_ascii_case(w))
    })
}

fn ensure_styles(d: &mut Document, levels: u8, width: f64) {
    let mut want = vec![("TOC Title".to_string(), ParaAttrs { space_after: Some(12.0), ..Default::default() }, Some(16.0))];
    for l in 1..=levels.max(1) {
        let indent = (l as f64 - 1.0) * 12.0;
        let hanging = 68.0;
        want.push((
            format!("TOC Level {l}"),
            ParaAttrs {
                left_indent: Some(indent + hanging),
                first_line_indent: Some(-hanging),
                space_before: Some(if l == 1 { 4.0 } else { 0.0 }),
                align: Some(Align::Left),
                tabs: Some(vec![TabStop { position: width.max(36.0), align: TabAlign::Right, leader: ". ".into(), align_on: String::new() }]),
                ..Default::default()
            },
            None,
        ));
    }
    let missing: Vec<_> = want.into_iter().filter(|(n, ..)| d.styles.para(n).is_none()).collect();
    if missing.is_empty() {
        return;
    }
    let st = d.styles_mut();
    for (name, para, size) in missing {
        let chars = designcraft_doc::CharAttrs { size, ..Default::default() };
        st.paragraph.push(ParagraphStyle {
            name,
            based_on: Some(designcraft_doc::story::BASIC_PARAGRAPH.into()),
            next_style: None,
            para,
            chars,
            shortcut: String::new(),
        });
    }
}

/// Rewrite the TOC story; returns the number of entries.
fn fill(d: &mut Document, toc: &Toc, width: Option<f64>) -> Result<usize> {
    let skip = toc_stories(d);
    let found = collect(d, &toc.entries, &skip);
    let levels = toc.entries.iter().map(|e| e.level).max().unwrap_or(1);
    let width = width.unwrap_or_else(|| {
        d.story(toc.story).and_then(|st| st.frames.first()).and_then(|f| d.item(*f)).map(|i| i.text_area().width()).unwrap_or(300.0)
    });
    ensure_styles(d, levels, width);
    for name in toc.entries.iter().filter_map(|e| e.entry_style.as_deref()).chain(toc.title_style.as_deref()) {
        para_style_exists(d, name)?;
    }
    for name in toc.entries.iter().flat_map(|e| [e.page_number_style.as_deref(), e.separator_style.as_deref()]).flatten() {
        char_style_exists(d, name)?;
    }
    let found = sort_entries(found, &toc.entries);
    let mut text = String::new();
    let mut styles = Vec::new();
    // Character styles of page numbers and separators: (byte range, style).
    let mut spans: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    if !toc.title.is_empty() {
        text.push_str(&toc.title);
        styles.push(toc.title_style.clone().unwrap_or_else(|| "TOC Title".into()));
    }
    for f in &found {
        let Some(e) = toc.entries.get(f.entry) else { continue };
        if !text.is_empty() {
            text.push('\n');
        }
        let sep = e.separator.as_deref().unwrap_or("\t");
        let mut push = |text: &mut String, s: &str, style: Option<&str>| {
            let start = text.len();
            text.push_str(s);
            if let Some(style) = style {
                spans.push((start..text.len(), style.to_string()));
            }
        };
        let (number, between) = (e.page_number_style.as_deref(), e.separator_style.as_deref());
        match e.page_number {
            TocPageNumber::After if toc.page_numbers => {
                text.push_str(&f.text);
                push(&mut text, sep, between);
                push(&mut text, &f.page, number);
            }
            TocPageNumber::Before if toc.page_numbers => {
                push(&mut text, &f.page, number);
                push(&mut text, sep, between);
                text.push_str(&f.text);
            }
            _ => text.push_str(&f.text),
        }
        styles.push(e.entry_style.clone().unwrap_or_else(|| format!("TOC Level {}", f.level.max(1))));
    }
    let st = d.story_mut(toc.story).ok_or(designcraft_doc::DocError::NoStory(toc.story))?;
    let len = st.len();
    st.replace(0..len, &text);
    // The text takes the format of the old first character: its character style would spread
    // over the whole table (a page number before the first entry, without a title).
    st.format_chars(0..st.len(), |f| f.style = NO_CHAR_STYLE.into());
    for (r, style) in spans {
        st.format_chars(r, |f| f.style.clone_from(&style));
    }
    for (pi, r) in st.para_ranges().into_iter().enumerate() {
        if let Some(name) = styles.get(pi) {
            st.format_paras(r.start..r.start, |f| *f = ParaFormat { style: name.clone(), ..Default::default() });
        }
    }
    Ok(found.len())
}

fn generate(s: &mut Session, p: &Value) -> Result<Value> {
    let entries = entries_param(p)?;
    if entries.is_empty() {
        return Err(bad("toc.generate", "no entries"));
    }
    let title = str_param(p, "title").unwrap_or("Contents");
    if title.chars().count() > MAX_TITLE {
        return Err(bad("toc.generate", format!("the title is at most {MAX_TITLE} characters")));
    }
    let title = one_line(title);
    let title_style = str_param(p, "titleStyle").filter(|n| !n.is_empty()).map(str::to_string);
    if title_style.as_ref().is_some_and(|n| n.chars().count() > MAX_NAME) {
        return Err(bad("toc.generate", format!("style names are at most {MAX_NAME} characters")));
    }
    let page_numbers = p.get("pageNumbers").and_then(Value::as_bool).unwrap_or(true);
    let replace = p.get("replace").and_then(Value::as_bool).unwrap_or(true);
    let rect = super::rect_param(p, "rect");
    if rect.is_some_and(|r| ![r.x0, r.y0, r.x1, r.y1].iter().all(|v| v.is_finite()) || r.width() <= 0.0 || r.height() <= 0.0) {
        return Err(bad("toc.generate", "`rect` must be finite with a positive size"));
    }
    let page = p.get("page").and_then(Value::as_u64).map_or(Some(0), |n| usize::try_from(n.max(1) - 1).ok());
    let page = page.ok_or_else(|| bad("toc.generate", "no such page"))?;
    s.edit(|d, sel| {
        for e in &entries {
            para_style_exists(d, &e.style)?;
        }
        // Replace: regenerate the table of contents already on the target page in place;
        // otherwise make a frame. A document can hold several (Contents, Tables, Charts).
        let existing = d.toc_on_page(page).map(|t| t.story).filter(|sid| replace && d.story(*sid).is_some());
        if existing.is_none() && d.tocs.len() >= designcraft_doc::MAX_TOCS {
            return Err(bad("toc.generate", format!("a document holds at most {} tables of contents", designcraft_doc::MAX_TOCS)));
        }
        let (story, width) = match existing {
            Some(sid) => (sid, None),
            None => {
                let (si, pi) = d.page_loc(page).ok_or_else(|| bad("toc.generate", format!("no page {}", page + 1)))?;
                let r: Rect = rect.unwrap_or_else(|| d.spreads[si].pages[pi].margin_rect());
                let layer = d.default_layer();
                let (fid, sid) = d.add_text_frame(SpreadRef::Doc(si), r, layer, "", ParaFormat::default())?;
                *sel = designcraft_doc::Selection::items(vec![fid]);
                let w = d.item(fid).map(|i| i.text_area().width());
                (sid, w)
            }
        };
        let toc = Toc { story, title: title.clone(), entries: entries.clone(), page_numbers, title_style: title_style.clone() };
        let n = fill(d, &toc, width)?;
        // The list keeps one entry per story, most recently generated last; tables whose story is
        // gone are dropped.
        let dead: Vec<StoryId> = d.tocs.iter().map(|t| t.story).filter(|sid| d.story(*sid).is_none()).collect();
        d.tocs.retain(|t| t.story != story && !dead.contains(&t.story));
        d.tocs.push(toc.clone());
        let autoflow = p.get("autoflow").and_then(Value::as_bool).unwrap_or(true);
        let mut pages_added = 0;
        if autoflow {
            let added = super::place_text_autoflow(d, story, 50, "toc.generate", false)?;
            pages_added += added;
            if added > 0 {
                let _ = fill(d, &toc, width);
                let added2 = super::place_text_autoflow(d, story, 50, "toc.generate", false)?;
                pages_added += added2;
            }
        }
        Ok(json!({"story": story.0, "entries": n, "pagesAdded": pages_added, "tocs": d.tocs.len()}))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toc_lists_headings_in_page_order_and_updates() {
        let mut s = Session::new();
        s.execute("file.newSample", &json!({})).unwrap();
        let pages = s.doc().unwrap().doc.page_count();
        s.execute("layout.pages.insert", &json!({"count": 1})).unwrap();
        let r = s.execute("toc.generate", &json!({"styles": ["Headline", "Cover Title"], "page": pages + 1})).unwrap();
        assert!(r["entries"].as_u64().unwrap() >= 2, "{r}");
        let d = &s.doc().unwrap().doc;
        let sid = StoryId(r["story"].as_u64().unwrap());
        let text = d.story(sid).unwrap().text.clone();
        let lines: Vec<&str> = text.split('\n').collect();
        assert_eq!(lines[0], "Contents");
        // Cover title (page 1) comes before the feature headline (page 2+).
        let cover = lines.iter().position(|l| l.starts_with("The Quiet")).unwrap();
        let head = lines.iter().position(|l| l.starts_with("Notes on the Grid")).unwrap();
        assert!(cover < head);
        assert!(lines[head].ends_with("\t2") || lines[head].contains('\t'), "{}", lines[head]);
        assert_eq!(d.story(sid).unwrap().paras[head].style, "TOC Level 1");
        assert!(d.styles.para("TOC Level 2").is_some());
        // Edit a heading, then update.
        let hid = d.stories.iter().find(|(_, st)| st.text.starts_with("Notes on the Grid")).map(|(id, _)| *id).unwrap();
        s.execute("story.replaceRange", &json!({"story": hid.0, "start": 0, "end": 5, "text": "Thoughts"})).unwrap();
        s.execute("toc.update", &json!({})).unwrap();
        let text = s.doc().unwrap().doc.story(sid).unwrap().text.clone();
        assert!(text.contains("Thoughts on the Grid"), "{text}");
        s.doc().unwrap().doc.check().unwrap();
    }

    fn sample_with_toc_page() -> (Session, usize) {
        let mut s = Session::new();
        s.execute("file.newSample", &json!({})).unwrap();
        let pages = s.doc().unwrap().doc.page_count();
        s.execute("layout.pages.insert", &json!({"count": 1})).unwrap();
        (s, pages + 1)
    }

    fn toc_lines(s: &Session) -> Vec<(String, String)> {
        let d = &s.doc().unwrap().doc;
        let st = d.story(d.tocs.last().unwrap().story).unwrap();
        st.para_ranges().into_iter().zip(&st.paras).map(|(r, f)| (st.text[r].trim_end_matches('\n').to_string(), f.style.clone())).collect()
    }

    #[test]
    fn toc_entry_style_page_number_position_and_separator_survive_update() {
        let (mut s, page) = sample_with_toc_page();
        let basic = designcraft_doc::story::BASIC_PARAGRAPH;
        s.execute(
            "toc.generate",
            &json!({"page": page, "title": "In This Issue", "titleStyle": basic, "entries": [
                {"style": "Cover Title", "level": 1, "pageNumber": "before", "separator": " — "},
                {"style": "Headline", "level": 2, "entryStyle": basic, "pageNumber": "none"},
            ]}),
        )
        .unwrap();
        let check = |s: &Session, headline: &str| {
            let lines = toc_lines(s);
            assert_eq!(lines[0], ("In This Issue".to_string(), basic.to_string()));
            let cover = lines.iter().find(|(t, _)| t.contains("The Quiet")).unwrap();
            assert!(cover.0.starts_with("1 — The Quiet"), "{cover:?}");
            assert_eq!(cover.1, "TOC Level 1");
            let head = lines.iter().find(|(t, _)| t.contains("on the Grid")).unwrap();
            assert!(head.0.starts_with(headline) && !head.0.contains('\t'), "{head:?}");
            assert_eq!(head.1, basic);
        };
        check(&s, "Notes on the Grid");
        let d = &s.doc().unwrap().doc;
        let hid = d.stories.iter().find(|(_, st)| st.text.starts_with("Notes on the Grid")).map(|(id, _)| *id).unwrap();
        s.execute("story.replaceRange", &json!({"story": hid.0, "start": 0, "end": 5, "text": "Thoughts"})).unwrap();
        s.execute("toc.update", &json!({})).unwrap();
        check(&s, "Thoughts on the Grid");
    }

    #[test]
    fn replace_regenerates_the_toc_in_place_and_off_makes_a_new_frame() {
        let (mut s, page) = sample_with_toc_page();
        let stories = |s: &Session| s.doc().unwrap().doc.stories.len();
        let first = s.execute("toc.generate", &json!({"styles": ["Headline"], "page": page})).unwrap()["story"].clone();
        let n = stories(&s);
        let undo = s.doc().unwrap().history.undo.len();
        let r = s.execute("toc.generate", &json!({"styles": ["Headline"], "title": "Inside", "page": page, "replace": true})).unwrap();
        assert_eq!(r["story"], first);
        assert_eq!(stories(&s), n, "one table of contents");
        assert_eq!(s.doc().unwrap().history.undo.len(), undo + 1, "one undo step");
        assert_eq!(toc_lines(&s)[0].0, "Inside");
        let r = s.execute("toc.generate", &json!({"styles": ["Headline"], "page": page, "replace": false})).unwrap();
        assert_ne!(r["story"], first);
        assert_eq!(stories(&s), n + 1);
        assert_eq!(s.doc().unwrap().doc.tocs.last().unwrap().story.0, r["story"].as_u64().unwrap());
    }

    /// A document of single-page spreads with one frame per page: `pages` is (text, style) per
    /// page. Returns the session and the number of pages (a table of contents goes on a new one).
    fn doc_of_pages(pages: &[(&str, &str)]) -> (Session, usize) {
        let mut s = Session::new();
        s.execute("file.new", &json!({"pages": pages.len() + 2, "facingPages": false})).unwrap();
        let mut names: Vec<&str> = pages.iter().map(|(_, st)| *st).collect();
        names.dedup();
        for n in names {
            if s.doc().unwrap().doc.styles.para(n).is_none() {
                s.execute("style.paragraph.create", &json!({"name": n})).unwrap();
            }
        }
        for (i, (text, style)) in pages.iter().enumerate() {
            let r = s.execute("frame.create", &json!({"spread": i, "rect": [36, 36, 576, 200], "content": "text", "text": text})).unwrap();
            s.execute("text.select", &json!({"story": r["story"], "anchor": 0, "focus": 0})).unwrap();
            s.execute("style.paragraph.apply", &json!({"name": style})).unwrap();
        }
        (s, pages.len() + 1)
    }

    #[test]
    fn chapter_number_prefixes_the_entry_with_the_page_s_first_text_box() {
        // A narrative page: the chapter line in its own text box, the title below it.
        let mut s = Session::new();
        s.execute("file.new", &json!({"pages": 2, "facingPages": false})).unwrap();
        for n in ["TOC Entry", "TOC Chapter"] {
            s.execute("style.paragraph.create", &json!({"name": n})).unwrap();
        }
        for (text, style, top) in [("BAB 01", "TOC Entry", 36.0_f64), ("Geografi", "TOC Chapter", 120.0_f64)] {
            let r =
                s.execute("frame.create", &json!({"spread": 0, "rect": [36.0, top, 576.0, top + 60.0], "content": "text", "text": text})).unwrap();
            s.execute("text.select", &json!({"story": r["story"], "anchor": 0, "focus": 0})).unwrap();
            s.execute("style.paragraph.apply", &json!({"name": style})).unwrap();
        }
        let entries = json!({"entries": [{"style": "TOC Chapter", "chapterNumber": true}]});
        assert_eq!(s.execute("toc.entries", &entries).unwrap(), json!([{"level": 1, "text": "BAB 01 Geografi", "page": "1"}]));
        // Without the flag the entry is the title on its own.
        assert_eq!(
            s.execute("toc.entries", &json!({"entries": [{"style": "TOC Chapter"}]})).unwrap(),
            json!([{"level": 1, "text": "Geografi", "page": "1"}])
        );
        // A page whose first text box is the entry itself keeps its own text.
        let mut s = Session::new();
        s.execute("file.new", &json!({"pages": 2, "facingPages": false})).unwrap();
        s.execute("style.paragraph.create", &json!({"name": "TOC Chapter"})).unwrap();
        let r =
            s.execute("frame.create", &json!({"spread": 0, "rect": [36, 36, 576, 96], "content": "text", "text": "Keterangan Publikasi"})).unwrap();
        s.execute("text.select", &json!({"story": r["story"], "anchor": 0, "focus": 0})).unwrap();
        s.execute("style.paragraph.apply", &json!({"name": "TOC Chapter"})).unwrap();
        assert_eq!(
            s.execute("toc.entries", &json!({"entries": [{"style": "TOC Chapter", "chapterNumber": true}]})).unwrap(),
            json!([{"level": 1, "text": "Keterangan Publikasi", "page": "1"}])
        );
    }

    #[test]
    fn page_title_lists_only_the_pages_whose_first_text_box_starts_with_a_word() {
        let (mut s, page) = doc_of_pages(&[
            ("Tabel 1.1. Jarak Antar Kecamatan", "TOC Entry"),
            ("Grafik 1.1. Curah Hujan Tiap Bulan", "TOC Entry"),
            ("Bab 1. Pendahuluan", "TOC Entry"),
        ]);
        let entry = |words: &[&str]| {
            let mut e = json!({"style": "TOC Entry"});
            if !words.is_empty() {
                e["pageTitle"] = json!(words);
            }
            json!({"entries": [e]})
        };
        // Every tagged page, then the tables and the charts.
        let all = s.execute("toc.entries", &entry(&[])).unwrap();
        assert_eq!(all.as_array().unwrap().len(), 3, "{all}");
        assert_eq!(
            s.execute("toc.entries", &entry(&["Tabel", "Table"])).unwrap(),
            json!([{"level": 1, "text": "Tabel 1.1. Jarak Antar Kecamatan", "page": "1"}])
        );
        // "Chart" is not a prefix of the page text "Grafik …", and a page that starts with
        // neither word is in no list.
        assert_eq!(
            s.execute("toc.entries", &entry(&["Grafik", "Chart"])).unwrap(),
            json!([{"level": 1, "text": "Grafik 1.1. Curah Hujan Tiap Bulan", "page": "2"}])
        );

        // A table of contents on the page after the content, listing the tables only.
        let r = s
            .execute(
                "toc.generate",
                &json!({"page": page, "title": "Daftar Table", "entries": [{"style": "TOC Entry", "pageTitle": ["Tabel", "Table"]}]}),
            )
            .unwrap();
        let tables = StoryId(r["story"].as_u64().unwrap());
        let text = s.doc().unwrap().doc.story(tables).unwrap().text.clone();
        assert!(text.contains("Tabel 1.1. Jarak Antar Kecamatan"), "{text}");
        assert!(!text.contains("Grafik 1.1.") && !text.contains("Bab 1."), "{text}");
        // The filter survives the save/load round trip of the definition.
        let e = &s.doc().unwrap().doc.tocs.last().unwrap().entries[0];
        assert_eq!(e.page_title, ["Tabel", "Table"]);
    }

    #[test]
    fn a_document_holds_several_tables_of_contents_and_updates_them_all() {
        let (mut s, page) = doc_of_pages(&[("Tabel 1.1. Jarak Antar Kecamatan", "TOC Entry"), ("Grafik 1.1. Curah Hujan Tiap Bulan", "TOC Entry")]);
        let make_toc = |s: &mut Session, page: usize, title: &str, words: &[&str]| {
            let r =
                s.execute("toc.generate", &json!({"page": page, "title": title, "entries": [{"style": "TOC Entry", "pageTitle": words}]})).unwrap();
            StoryId(r["story"].as_u64().unwrap())
        };
        let tables = make_toc(&mut s, page, "Daftar Table", &["Tabel"]);
        let charts = make_toc(&mut s, page + 1, "Daftar Grafik", &["Grafik"]);
        assert_ne!(tables, charts);
        let d = &s.doc().unwrap().doc;
        assert_eq!(d.tocs.len(), 2, "{:?}", d.tocs);
        assert_eq!(d.toc_of_story(tables).map(|t| t.title.as_str()), Some("Daftar Table"));
        assert_eq!(d.toc_on_page(page - 1).map(|t| t.story), Some(tables));
        assert_eq!(d.toc_on_page(page).map(|t| t.story), Some(charts));
        assert!(!d.story(charts).unwrap().text.contains("Tabel 1.1."), "{:?}", d.story(charts).unwrap().text);

        // Regenerating the first one replaces it in place and leaves the second alone.
        let again = make_toc(&mut s, page, "Daftar Table", &["Tabel"]);
        assert_eq!(again, tables);
        let d = &s.doc().unwrap().doc;
        assert_eq!(d.tocs.len(), 2);
        assert!(d.story(charts).is_some());

        // Update Table of Contents regenerates both, keeping each one's filter.
        let r = s.execute("toc.update", &json!({})).unwrap();
        assert_eq!(r["tocs"], 2, "{r}");
        let d = &s.doc().unwrap().doc;
        let t = d.story(tables).unwrap().text.clone();
        assert!(t.contains("Tabel 1.1.") && !t.contains("Grafik 1.1."), "{t}");
        let c = d.story(charts).unwrap().text.clone();
        assert!(c.contains("Grafik 1.1.") && !c.contains("Tabel 1.1."), "{c}");
        for toc in &d.tocs {
            assert_eq!(toc.entries[0].page_title.len(), 1);
        }
        d.check().unwrap();
    }

    #[test]
    fn hostile_toc_params_are_errors_that_leave_the_document_alone() {
        let (mut s, page) = sample_with_toc_page();
        let long = "x".repeat(5000);
        let many: Vec<Value> = (0..65).map(|_| json!({"style": "Headline"})).collect();
        let words: Vec<Value> = (0..9).map(|i| json!(format!("w{i}"))).collect();
        for p in [
            json!({"entries": [{"style": "Headline", "level": 0}]}),
            json!({"entries": [{"style": "Headline", "level": 10}]}),
            json!({"entries": [{"style": "Headline", "level": 300}]}),
            json!({"entries": [{"style": "Headline", "level": -1}]}),
            json!({"entries": [{"style": "Headline", "pageNumber": "sideways"}]}),
            json!({"entries": [{"style": "No Such Style"}]}),
            json!({"entries": [{"style": "Headline", "entryStyle": "No Such Style"}]}),
            json!({"entries": [{"style": "Headline", "pageNumberStyle": "No Such Style"}]}),
            json!({"entries": [{"style": "Headline", "separatorStyle": "No Such Style"}]}),
            json!({"entries": [{"style": "Headline", "pageNumberStyle": long}]}),
            json!({"entries": [{"style": "Headline", "sort": "yes"}]}),
            json!({"entries": [{"style": "Headline", "separator": long}]}),
            json!({"entries": [{"style": long}]}),
            json!({"entries": many}),
            json!({"entries": [{"style": "Headline", "pageTitle": words}]}),
            json!({"entries": [{"style": "Headline", "pageTitle": [long]}]}),
            json!({"entries": [{"style": "Headline", "pageTitle": "Tabel"}]}),
            json!({"entries": "Headline"}),
            json!({"entries": []}),
            json!({"styles": ["Headline"], "titleStyle": "No Such Style"}),
            json!({"styles": ["Headline"], "title": long}),
            json!({"styles": ["Headline"], "rect": [100, 100, 50, 50]}),
            json!({"styles": ["Headline"], "page": u64::MAX}),
            json!([1, 2]),
        ] {
            let rev = s.doc().unwrap().revision;
            assert!(s.execute("toc.generate", &merge(&p, page)).is_err(), "{p}");
            assert_eq!(s.doc().unwrap().revision, rev, "{p}");
            assert!(s.doc().unwrap().doc.tocs.is_empty());
        }
    }

    /// A one-page document whose one story has `paras` (text, paragraph style; the styles are
    /// made here), and an empty second page for the table of contents.
    fn doc_with(paras: &[(&str, &str)]) -> (Session, usize) {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        let mut names: Vec<&str> = paras.iter().map(|(_, st)| *st).collect();
        names.dedup();
        for n in names {
            if s.doc().unwrap().doc.styles.para(n).is_none() {
                s.execute("style.paragraph.create", &json!({"name": n})).unwrap();
            }
        }
        let text = paras.iter().map(|(t, _)| *t).collect::<Vec<_>>().join("\n");
        let r = s.execute("frame.create", &json!({"rect": [36, 36, 576, 756], "content": "text", "text": text})).unwrap();
        let mut at = 0;
        for (t, st) in paras {
            s.execute("text.select", &json!({"story": r["story"], "anchor": at, "focus": at})).unwrap();
            s.execute("style.paragraph.apply", &json!({"name": st})).unwrap();
            at += t.len() + 1;
        }
        s.execute("layout.pages.insert", &json!({"count": 1})).unwrap();
        (s, 2)
    }

    /// The table of contents' character runs as (text, character style), merged by style.
    fn toc_runs(s: &Session) -> Vec<(String, String)> {
        let d = &s.doc().unwrap().doc;
        let st = d.story(d.tocs.last().unwrap().story).unwrap();
        let mut out: Vec<(String, String)> = Vec::new();
        for (r, f) in st.runs() {
            match out.last_mut() {
                Some(last) if last.1 == f.style => last.0.push_str(&st.text[r]),
                _ => out.push((st.text[r].to_string(), f.style.clone())),
            }
        }
        out
    }

    #[test]
    fn page_number_and_separator_character_styles_are_applied_and_survive_update() {
        let (mut s, page) = doc_with(&[("Apples", "Head"), ("Pears", "Head")]);
        s.execute("style.character.create", &json!({"name": "Number", "chars": {"fontStyle": "Bold"}})).unwrap();
        s.execute("style.character.create", &json!({"name": "Dots", "chars": {"fill": "[Registration]"}})).unwrap();
        let none = designcraft_doc::NO_CHAR_STYLE.to_string();
        s.execute(
            "toc.generate",
            &json!({"page": page, "title": "", "entries": [
                {"style": "Head", "pageNumberStyle": "Number", "separatorStyle": "Dots", "separator": " … "},
            ]}),
        )
        .unwrap();
        let want = |n: &str| -> Vec<(String, String)> {
            vec![
                ("Apples".into(), none.clone()),
                (" … ".into(), "Dots".into()),
                ("1".into(), "Number".into()),
                (format!("\n{n}"), none.clone()),
                (" … ".into(), "Dots".into()),
                ("1".into(), "Number".into()),
            ]
        };
        assert_eq!(toc_runs(&s), want("Pears"));
        let e = &s.doc().unwrap().doc.tocs.last().unwrap().entries[0];
        assert_eq!((e.page_number_style.as_deref(), e.separator_style.as_deref()), (Some("Number"), Some("Dots")));
        s.execute("toc.update", &json!({})).unwrap();
        assert_eq!(toc_runs(&s), want("Pears"));

        // Before the entry, without a title: the first character is a styled page number, and
        // regenerating doesn't spread its style over the rest of the story.
        s.execute(
            "toc.generate",
            &json!({"page": page, "title": "", "entries": [{"style": "Head", "pageNumber": "before", "pageNumberStyle": "Number"}]}),
        )
        .unwrap();
        s.execute("toc.update", &json!({})).unwrap();
        assert_eq!(
            toc_runs(&s),
            vec![
                ("1".to_string(), "Number".to_string()),
                ("\tApples\n".into(), none.clone()),
                ("1".into(), "Number".into()),
                ("\tPears".into(), none.clone()),
            ]
        );
        // Without styles everything is [None].
        s.execute("toc.generate", &json!({"page": page, "entries": [{"style": "Head"}]})).unwrap();
        assert!(toc_runs(&s).iter().all(|(_, st)| *st == none), "{:?}", toc_runs(&s));
    }

    #[test]
    fn sorted_styles_list_their_entries_alphabetically_and_keep_their_children() {
        let (mut s, page) = doc_with(&[
            ("Zucchini", "Chapter"),
            ("Zest", "Section"),
            ("Bake", "Section"),
            ("apricot", "Chapter"),
            ("Mix", "Section"),
            ("Bread", "Chapter"),
        ]);
        let lines = |s: &Session| toc_lines(s).into_iter().skip(1).map(|(t, _)| t.split('\t').next().unwrap().to_string()).collect::<Vec<_>>();
        // Chapters sorted (case-insensitively), sections stay in page order under their chapter.
        s.execute(
            "toc.generate",
            &json!({"page": page, "entries": [{"style": "Chapter", "level": 1, "sort": true}, {"style": "Section", "level": 2}]}),
        )
        .unwrap();
        assert_eq!(lines(&s), ["apricot", "Mix", "Bread", "Zucchini", "Zest", "Bake"]);
        assert!(s.doc().unwrap().doc.tocs.last().unwrap().entries[0].sort);
        // Sections sorted within each chapter only.
        s.execute(
            "toc.generate",
            &json!({"page": page, "entries": [{"style": "Chapter", "level": 1}, {"style": "Section", "level": 2, "sort": true}]}),
        )
        .unwrap();
        assert_eq!(lines(&s), ["Zucchini", "Bake", "Zest", "apricot", "Mix", "Bread"]);
        s.execute("toc.update", &json!({})).unwrap();
        assert_eq!(lines(&s), ["Zucchini", "Bake", "Zest", "apricot", "Mix", "Bread"]);
        // The preview lists them as they will be set.
        let preview = s.execute("toc.entries", &json!({"entries": [{"style": "Chapter", "level": 1, "sort": true}]})).unwrap();
        let texts: Vec<&str> = preview.as_array().unwrap().iter().map(|e| e["text"].as_str().unwrap()).collect();
        assert_eq!(texts, ["apricot", "Bread", "Zucchini"]);
        // Unsorted: page order.
        s.execute("toc.generate", &json!({"page": page, "styles": ["Chapter", "Section"]})).unwrap();
        assert_eq!(lines(&s), ["Zucchini", "Zest", "Bake", "apricot", "Mix", "Bread"]);
    }

    #[test]
    fn renaming_or_deleting_a_character_style_updates_the_toc_definition() {
        let (mut s, page) = doc_with(&[("Apples", "Head")]);
        s.execute("style.character.create", &json!({"name": "Number"})).unwrap();
        s.execute("style.character.create", &json!({"name": "Dots"})).unwrap();
        s.execute("toc.generate", &json!({"page": page, "entries": [{"style": "Head", "pageNumberStyle": "Number", "separatorStyle": "Dots"}]}))
            .unwrap();
        s.execute("style.character.edit", &json!({"name": "Number", "rename": "Folio"})).unwrap();
        s.execute("style.character.delete", &json!({"name": "Dots"})).unwrap();
        let e = s.doc().unwrap().doc.tocs.last().unwrap().entries[0].clone();
        assert_eq!((e.page_number_style.as_deref(), e.separator_style), (Some("Folio"), None));
        s.execute("toc.update", &json!({})).unwrap();
        assert!(toc_runs(&s).iter().any(|(t, st)| t == "1" && st == "Folio"), "{:?}", toc_runs(&s));
    }

    #[test]
    fn toc_entry_options_round_trip_through_the_native_format() {
        let mut e = TocEntry::new("Head", 2);
        e.page_number_style = Some("Number".into());
        e.separator_style = Some("Dots".into());
        e.sort = true;
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!((&v["pageNumberStyle"], &v["separatorStyle"], &v["sort"]), (&json!("Number"), &json!("Dots"), &json!(true)));
        assert_eq!(serde_json::from_value::<TocEntry>(v).unwrap(), e);
        // Files written before these options read with none of them.
        let old: TocEntry = serde_json::from_value(json!({"style": "Head", "level": 1, "pageNumber": "after"})).unwrap();
        assert_eq!(old, TocEntry::new("Head", 1));
        assert!(serde_json::to_value(&old).unwrap().get("sort").is_none());
    }

    /// `p` with the TOC page added when it is an object and has none.
    fn merge(p: &Value, page: usize) -> Value {
        let mut p = p.clone();
        if let Some(o) = p.as_object_mut() {
            o.entry("page").or_insert(json!(page));
        }
        p
    }
}
