//! Pubgen preflight audit ("Audit Publication"): the statistical-publication rules on top of
//! the generic preflight — missing `Sumber:`/`Catatan:` note lines and the table-number
//! sequence — merged with `preflight.run` into one checklist.

use designcraft_doc::{Content, Intent, ItemId, PageSide, SpreadRef};
use designcraft_geom::Affine;
use serde_json::{Value, json};

use super::preflight::{Issue, check, page_of};
use super::{CommandSpec, cmd, has_doc};
use crate::{Result, Session};

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(query "preflight.pubgen", "Pubgen Preflight Audit", ["Window", "Output", "Pubgen Audit"], None,
        "{profile?: \"print\"|\"digital\", minPpi?: 300|150} → {profile, errors, warnings, issues}: the generic preflight \
         plus the Pubgen rules (missing Sumber:/Catatan: lines, table-number sequence, dual-lang parity; print adds \
         spine/bleed guards and a 300 ppi floor)", has_doc, run)]
}

/// The keywords a table note line starts with
/// (twin: `panels/pubgen/excel.rs` `matches_table_note_keyword` — keep the lists in sync).
fn matches_note_keyword(field: &str) -> bool {
    let lower = field.trim().to_lowercase();
    lower.starts_with("keterangan")
        || lower.starts_with("ket.")
        || lower.starts_with("ket ")
        || lower == "ket"
        || lower.starts_with("catatan")
        || lower.starts_with("note")
        || lower.starts_with("sumber")
        || lower.starts_with("source")
}

/// The content a note line carries after its label and colon: Pubgen's note frames use three
/// tab fields (`Sumber ␉ : ␉ BPS, 2025`), hand-typed lines put it in one run (`Sumber: BPS,
/// 2025`). `None` = not a note line. An empty string = a note line with nothing after the
/// colon — the `PRF-03` defect.
fn note_content(line: &str) -> Option<String> {
    let fields: Vec<&str> = line.trim().split('\t').collect();
    if !matches_note_keyword(fields[0].trim()) {
        return None;
    }
    match fields.len() {
        0..=1 => {
            let label = fields.first().copied().unwrap_or("").trim();
            let idx = label.find(':')?;
            Some(label[idx + 1..].trim().to_string())
        }
        2 => {
            // `label ␉ : BPS, 2025`. A second field without a colon is free text, not a note.
            fields[1].trim().strip_prefix(':').map(str::trim).map(str::to_string)
        }
        _ => Some(fields[2..].join(" ").trim().to_string()),
    }
}

/// `Tabel 02.04 …` → ("table", 2, 4). Bilingual titles put both languages in one paragraph
/// joined by a forced break; the caller splits on it, so each half is seen on its own.
fn title_number(line: &str) -> Option<(&'static str, u32, u32)> {
    let (word, rest) = line.trim().split_once(' ')?;
    let kind = match word.trim_end_matches(':').to_lowercase().as_str() {
        "tabel" | "table" => "table",
        "grafik" | "chart" => "chart",
        "gambar" | "figure" => "figure",
        _ => return None,
    };
    let num = rest.split([' ', '\t']).next().unwrap_or("");
    let (a, b) = num.split_once('.')?;
    if a.is_empty() || b.is_empty() || a.len() > 2 || b.len() > 2 {
        return None;
    }
    if !a.chars().all(|c| c.is_ascii_digit()) || !b.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((kind, a.parse().ok()?, b.parse().ok()?))
}

/// The page a story's first placed frame sits on.
fn story_page(s: &Session, sid: designcraft_doc::StoryId) -> Option<usize> {
    let st = s.active()?;
    let d = &st.doc;
    let fid = *d.story(sid)?.frames.first()?;
    d.find(fid).zip(d.item(fid)).and_then(|(loc, it)| page_of(d, loc.spread, it, d.parent_xf(&loc)))
}

/// A collected title number: (kind, chapter, number, story).
type Title = (&'static str, u32, u32, designcraft_doc::StoryId);

pub fn pubgen_issues(s: &Session, print: bool) -> Vec<Issue> {
    let Some(st) = s.active() else { return vec![] };
    let d = &st.doc;
    let mut out = Vec::new();
    // PRF-03: note lines with nothing after the colon; PRF-04: collected title numbers.
    let mut titles: Vec<Title> = Vec::new();
    for story in d.stories.values() {
        for line in story.text.split(['\n', '\u{2028}']) {
            if let Some(content) = note_content(line)
                && content.is_empty()
            {
                out.push(Issue {
                    severity: "warning",
                    kind: "missingSourceNote",
                    message: format!("Empty `{}` note line", line.trim().split('\t').next().unwrap_or("").trim_end_matches([':', ' '])),
                    item: story.frames.first().map(|f| f.0),
                    page: story_page(s, story.id),
                });
            }
            if let Some((kind, chapter, number)) = title_number(line) {
                titles.push((kind, chapter, number, story.id));
            }
        }
    }
    sequence_issues(s, &mut titles, &mut out);
    parity_issues(s, &mut out);
    if print {
        cover_guards(s, &mut out);
    }
    out
}

// --- PRF-05: dual-language parity ---------------------------------------------------------

/// A review page's body column is at least this wide and tall on the page (pt): what
/// separates the two review columns from caption or header frames.
const BODY_MIN_SIDE: f64 = 100.0;
/// The Indonesian column must carry at least this many characters before a page counts as a
/// review spread (twin: `panels/pubgen/ai.rs` `detect_page_target` — keep in sync).
const REVIEW_BODY_CHARS: usize = 120;

/// A detected dual-lang review page.
struct ReviewPage {
    id_text: String,
    en_frame: ItemId,
    en_text: String,
}

/// Engine-side twin of the panel's `detect_page_target` reduced to what the parity rule
/// needs: an EN "Chapter NN" header, a two-word `BAB NN` line, a short Indonesian title above
/// the header, and two side-by-side body columns below it. Unlike the panel twin it keeps
/// empty frames — an empty English column is exactly the defect this rule reports.
fn detect_review_page(d: &designcraft_doc::Document, si: usize) -> Option<ReviewPage> {
    let sp = d.spreads.get(si)?;
    // id, x0, y0, width, height, text.
    let mut frames: Vec<(ItemId, f64, f64, f64, f64, String)> = Vec::new();
    for it in &sp.items {
        if let Some(tf) = it.text_frame()
            && let Some(story) = d.story(tf.story)
        {
            let b = it.bounds();
            frames.push((it.id, b.x0, b.y0, b.x1 - b.x0, b.y1 - b.y0, story.text.clone()));
        }
    }
    let en = frames.iter().find(|(_, _, _, _, _, t)| {
        let t = t.trim();
        t.len() < 80 && t.to_lowercase().starts_with("chapter")
    })?;
    let (en_id, en_y) = (en.0, en.2);
    let bab = frames
        .iter()
        .find(|(id, _, _, _, _, t)| {
            *id != en_id && {
                let up = t.trim().to_uppercase();
                (up.starts_with("BAB ") || up.starts_with("CHAPTER ")) && up.split_whitespace().count() == 2
            }
        })?
        .0;
    let title = frames
        .iter()
        .filter(|(id, _, y0, _, _, t)| *id != en_id && *id != bab && *y0 < en_y && !t.trim().is_empty() && t.trim().len() < 80)
        .max_by_key(|(_, _, y0, _, _, _)| y0.to_bits())?
        .0;
    let mut bodies: Vec<_> = frames
        .iter()
        .filter(|(id, _, y0, w, h, _)| *id != en_id && *id != bab && *id != title && *y0 > en_y && *w >= BODY_MIN_SIDE && *h >= BODY_MIN_SIDE)
        .collect();
    if bodies.len() < 2 {
        return None;
    }
    bodies.sort_by_key(|(_, x0, _, _, _, _)| x0.to_bits());
    let id = bodies[0];
    let en = bodies[bodies.len() - 1];
    if id.5.trim().chars().count() <= REVIEW_BODY_CHARS {
        return None;
    }
    Some(ReviewPage { id_text: id.5.clone(), en_frame: en.0, en_text: en.5.clone() })
}

fn is_placeholder(t: &str) -> bool {
    matches!(t.trim(), "…" | "..." | "-" | "–" | "—" | "TODO" | "todo")
}

fn parity_issues(s: &Session, out: &mut Vec<Issue>) {
    let Some(st) = s.active() else { return };
    let d = &st.doc;
    for si in 0..d.spreads.len() {
        let Some(rp) = detect_review_page(d, si) else { continue };
        let en = rp.en_text.trim();
        let reason = if en.is_empty() {
            "empty"
        } else if is_placeholder(en) {
            "a placeholder"
        } else if en == rp.id_text.trim() {
            "untranslated"
        } else {
            continue;
        };
        out.push(Issue {
            severity: "warning",
            kind: "dualLangParity",
            message: format!("Dual-lang review page: the English body is {reason}"),
            item: Some(rp.en_frame.0),
            page: d.item(rp.en_frame).and_then(|it| page_of(d, SpreadRef::Doc(si), it, Affine::IDENTITY)),
        });
    }
}

// --- PRF-07: spine and bleed guards --------------------------------------------------------

/// Millimetres in points.
const MM: f64 = 72.0 / 25.4;
/// A full-bleed element must pass the trim edge by this much.
const BLEED_MM: f64 = 3.0;
/// No text may sit within this distance of the spine.
const SPINE_MM: f64 = 5.0;

fn cover_guards(s: &Session, out: &mut Vec<Issue>) {
    let Some(st) = s.active() else { return };
    let d = &st.doc;
    if d.settings.intent != Intent::Print {
        return;
    }
    let has_bleed = d.settings.bleed.iter().any(|&v| v > 0.0);
    for (si, sp) in d.spreads.iter().enumerate() {
        let trim = sp.bounds();
        let facing = sp.pages.iter().any(|p| p.side == PageSide::Left) && sp.pages.iter().any(|p| p.side == PageSide::Right);
        let spine_x = if facing { Some(sp.spine_x()) } else { None };
        let mut pending: Vec<_> = sp.items.iter().rev().map(|it| (it.as_ref(), Affine::IDENTITY)).collect();
        while let Some((it, parent)) = pending.pop() {
            let xf = parent * it.xf;
            pending.extend(it.children().iter().rev().map(|child| (child.as_ref(), xf)));
            let b = parent.transform_rect_bbox(it.bounds());
            let page = page_of(d, SpreadRef::Doc(si), it, parent);
            match &it.content {
                Content::Graphic(_) if has_bleed => {
                    let edges = [(trim.x0 - b.x0, "left"), (b.x1 - trim.x1, "right"), (trim.y0 - b.y0, "top"), (b.y1 - trim.y1, "bottom")];
                    if let Some((overshoot, edge)) = edges.into_iter().find(|(overshoot, _)| *overshoot > 0.01 && *overshoot < BLEED_MM * MM) {
                        out.push(Issue {
                            severity: "error",
                            kind: "bleedGuard",
                            message: format!(
                                "Bleed too short: the graphic passes the {edge} trim edge by {overshoot:.1} pt (< {:.1} pt)",
                                BLEED_MM * MM
                            ),
                            item: Some(it.id.0),
                            page,
                        });
                    }
                }
                Content::Text(_) if facing => {
                    let spine = spine_x.unwrap_or_default();
                    let dist = if b.x1 <= spine {
                        spine - b.x1
                    } else if b.x0 >= spine {
                        b.x0 - spine
                    } else {
                        0.0 // across the spine: certainly too close
                    };
                    if dist < SPINE_MM * MM {
                        out.push(Issue {
                            severity: "error",
                            kind: "spineGuard",
                            message: format!("Text too close to the spine: {dist:.1} pt (< {:.1} pt)", SPINE_MM * MM),
                            item: Some(it.id.0),
                            page,
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

/// `PRF-04`: within one (kind, chapter) the numbers must run 01, 02, 03… A bilingual title
/// pair is two occurrences of one number on one page — not a duplicate.
fn sequence_issues(s: &Session, titles: &mut Vec<Title>, out: &mut Vec<Issue>) {
    titles.sort_by(|a, b| (a.0, a.1, a.2, a.3).cmp(&(b.0, b.1, b.2, b.3)));
    for group in titles.chunk_by(|a, b| a.0 == b.0 && a.1 == b.1) {
        if group.len() < 2 {
            continue;
        }
        let (kind, chapter) = (group[0].0, group[0].1);
        let mut prev: Option<u32> = None;
        let mut same: Vec<(u32, Option<usize>)> = Vec::new(); // one number, consecutive
        for &(kind, chapter, n, sid) in group {
            if let Some(prev_n) = prev
                && n > prev_n + 1
            {
                out.push(Issue {
                    severity: "warning",
                    kind: "tableNumberSequence",
                    message: format!(
                        "{w} number gap: {chapter:02}.{miss:02} skipped (between {chapter:02}.{prev_n:02} and {chapter:02}.{n:02})",
                        w = word(kind),
                        miss = prev_n + 1
                    ),
                    item: None,
                    page: story_page(s, sid),
                });
            }
            if same.last().is_some_and(|(last, _)| *last == n) {
                same.push((n, story_page(s, sid)));
            } else {
                flush_run(kind, chapter, &mut same, out);
                same.push((n, story_page(s, sid)));
            }
            prev = Some(n);
        }
        flush_run(kind, chapter, &mut same, out);
    }
}

fn word(kind: &str) -> &'static str {
    match kind {
        "chart" => "Chart",
        "figure" => "Figure",
        _ => "Table",
    }
}

/// Three occurrences of one number, or two on different pages, are a duplicate; a bilingual
/// title pair (two occurrences on one page) is not.
fn flush_run(kind: &str, chapter: u32, same: &mut Vec<(u32, Option<usize>)>, out: &mut Vec<Issue>) {
    if same.len() >= 3 || (same.len() == 2 && same[0].1 != same[1].1) {
        let (n, page) = same[0];
        let pages: Vec<String> = same.iter().map(|(_, p)| p.map_or_else(|| "?".to_string(), |p| (p + 1).to_string())).collect();
        out.push(Issue {
            severity: "warning",
            kind: "tableNumberSequence",
            message: format!("{w} number duplicated: {chapter:02}.{n:02} appears on page(s) {p}", w = word(kind), p = pages.join(", ")),
            item: None,
            page,
        });
    }
    same.clear();
}

fn run(s: &mut Session, p: &Value) -> Result<Value> {
    // The distribution profile picks the rules: the print profile adds the spine and bleed
    // guards and a stricter DPI floor. Absent, the document's intent decides.
    let print = match p.get("profile").and_then(Value::as_str) {
        Some("print") => true,
        Some("digital") => false,
        _ => s.active().is_none_or(|st| st.doc.settings.intent == Intent::Print),
    };
    let min = p.get("minPpi").and_then(Value::as_f64).unwrap_or(if print { 300.0 } else { 150.0 });
    let mut issues = check(s, min);
    issues.extend(pubgen_issues(s, print));
    let errors = issues.iter().filter(|i| i.severity == "error").count();
    Ok(json!({"profile": if print { "print" } else { "digital" }, "errors": errors, "warnings": issues.len() - errors, "issues": issues}))
}

#[cfg(test)]
mod note_line_tests {
    use super::*;

    #[test]
    fn note_content_reads_both_pubgen_and_typed_layouts() {
        // Pubgen's note frame: label ␉ colon ␉ content.
        assert_eq!(note_content("Sumber\t:\tBPS Kabupaten Karawang, 2025").as_deref(), Some("BPS Kabupaten Karawang, 2025"));
        // Hand-typed single run.
        assert_eq!(note_content("Sumber: BPS, 2025").as_deref(), Some("BPS, 2025"));
        // Empty content is the defect: empty after the colon in either layout.
        assert_eq!(note_content("Sumber\t:\t").as_deref(), Some(""));
        assert_eq!(note_content("Sumber:").as_deref(), Some(""));
        assert_eq!(note_content("Catatan : ").as_deref(), Some(""));
        // Present content passes; a dash is a legitimate "no notes".
        assert_eq!(note_content("Catatan: -").as_deref(), Some("-"));
        // Not note lines at all.
        assert_eq!(note_content("Ket\tfree text"), None, "a second field without a colon is not a note");
        assert_eq!(note_content("Keterangan umum bab ini"), None, "no colon, no note");
        assert_eq!(note_content("Sumber data tahun 2024"), None);
        assert_eq!(note_content("Percentage of households"), None);
    }
}

#[cfg(test)]
mod sequence_tests {
    use super::*;

    fn titles(list: &[(u32, u32)]) -> Vec<Title> {
        list.iter().map(|&(c, n)| ("table", c, n, designcraft_doc::StoryId(n.into()))).collect()
    }

    fn messages(out: &[Issue]) -> Vec<String> {
        out.iter().filter(|i| i.kind == "tableNumberSequence").map(|i| i.message.clone()).collect()
    }

    #[test]
    fn gaps_and_duplicates_are_named() {
        let mut out = Vec::new();
        let mut t = titles(&[(2, 1), (2, 2), (2, 4)]);
        sequence_issues(&Session::new(), &mut t, &mut out);
        assert_eq!(messages(&out), ["Table number gap: 02.03 skipped (between 02.02 and 02.04)"]);

        // Same number twice on different pages is a duplicate; a bilingual pair (same page) is not.
        let mut s = Session::new();
        s.execute("file.new", &serde_json::json!({"pages": 2})).unwrap();
        let a = new_frame_story(&mut s, 0);
        let b = new_frame_story(&mut s, 1);
        let mut out = Vec::new();
        let mut t: Vec<Title> = vec![("table", 2, 2, a), ("table", 2, 2, b)];
        sequence_issues(&s, &mut t, &mut out);
        assert_eq!(messages(&out).len(), 1, "{:?}", messages(&out));

        let mut out = Vec::new();
        let mut t: Vec<Title> = vec![("table", 2, 2, a), ("table", 2, 2, a)];
        sequence_issues(&s, &mut t, &mut out);
        assert!(messages(&out).is_empty(), "a bilingual title pair is not a duplicate: {:?}", messages(&out));
    }

    #[test]
    fn kinds_and_chapters_never_cross_compare() {
        let mut out = Vec::new();
        let mut t = titles(&[(1, 1), (2, 1)]);
        sequence_issues(&Session::new(), &mut t, &mut out);
        assert!(messages(&out).is_empty());
        let mut t: Vec<Title> = vec![("chart", 2, 1, designcraft_doc::StoryId(1)), ("table", 2, 1, designcraft_doc::StoryId(2))];
        sequence_issues(&Session::new(), &mut t, &mut out);
        assert!(messages(&out).is_empty());
    }

    /// A story with one placed frame, so duplicate detection can see two different pages.
    fn new_frame_story(s: &mut Session, spread: usize) -> designcraft_doc::StoryId {
        let r = s
            .execute("frame.create", &serde_json::json!({"spread": spread, "rect": [0, 0, 100, 50], "content": "text", "text": "Tabel 02.02 X"}))
            .unwrap();
        designcraft_doc::StoryId(r["story"].as_u64().unwrap())
    }
}

#[cfg(test)]
mod command_tests {
    use serde_json::json;

    use crate::Session;

    /// The merged checklist: an empty `Sumber:` note and a skipped table number, plus the
    /// generic rules riding along.
    #[test]
    fn the_command_merges_generic_and_pubgen_issues() {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        s.execute("frame.create", &json!({"rect": [20, 20, 200, 30], "content": "text", "text": "Sumber\t:\t"})).unwrap();
        s.execute("frame.create", &json!({"rect": [20, 80, 200, 30], "content": "text", "text": "Tabel 02.01 Jumlah Desa"})).unwrap();
        s.execute("frame.create", &json!({"rect": [20, 140, 200, 30], "content": "text", "text": "Tabel 02.03 Luas Wilayah"})).unwrap();
        let r = s.execute("preflight.pubgen", &json!({})).unwrap();
        let issues = r["issues"].as_array().unwrap();
        assert!(issues.iter().any(|i| i["kind"] == "missingSourceNote"), "{issues:?}");
        assert!(issues.iter().any(|i| i["kind"] == "tableNumberSequence"), "{issues:?}");
        assert_eq!(r["errors"].as_u64().unwrap() + r["warnings"].as_u64().unwrap(), issues.len() as u64);
    }

    /// Nothing in a plain document matches a Pubgen pattern: the Pubgen rules stay silent.
    #[test]
    fn a_plain_document_raises_no_pubgen_issues() {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        s.execute("frame.create", &json!({"rect": [20, 20, 300, 400], "content": "text", "text": "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."})).unwrap();
        assert!(super::pubgen_issues(&s, false).is_empty());
    }
}

#[cfg(test)]
mod parity_tests {
    use serde_json::json;

    use crate::Session;

    /// A generated dual-lang review page: BAB line, Indonesian title, EN "Chapter" header and
    /// two body columns; the English column carries `en_text`.
    fn review_page(en_text: &str) -> Session {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        let id_body = "Penduduk kecamatan ini meningkat ".repeat(8);
        let frames: [(&str, [f64; 4]); 5] = [
            ("BAB 01", [20.0, 20.0, 300.0, 50.0]),
            ("Geografi", [20.0, 55.0, 300.0, 85.0]),
            ("Chapter 01 Geography", [20.0, 90.0, 300.0, 120.0]),
            (id_body.as_str(), [20.0, 130.0, 290.0, 400.0]),
            (en_text, [300.0, 130.0, 570.0, 400.0]),
        ];
        for (text, rect) in frames {
            s.execute("frame.create", &json!({"rect": rect, "content": "text", "text": text})).unwrap();
        }
        s
    }

    fn parity(s: &mut Session) -> Vec<String> {
        let r = s.execute("preflight.pubgen", &json!({})).unwrap();
        r["issues"].as_array().unwrap().iter().filter(|i| i["kind"] == "dualLangParity").map(|i| i["message"].as_str().unwrap().to_string()).collect()
    }

    #[test]
    fn empty_placeholder_and_untranslated_english_columns_are_flagged() {
        let translated = "Population grew by three percent ".repeat(8);
        for (en, word) in [("", "empty"), ("…", "a placeholder"), ("TODO", "a placeholder")] {
            let mut s = review_page(en);
            let msgs = parity(&mut s);
            assert_eq!(msgs.len(), 1, "{en:?}: {msgs:?}");
            assert!(msgs[0].contains(word), "{en:?}: {msgs:?}");
        }
        let mut s = review_page(&"Penduduk kecamatan ini meningkat ".repeat(8));
        let msgs = parity(&mut s);
        assert_eq!(msgs.len(), 1, "{msgs:?}");
        assert!(msgs[0].contains("untranslated"), "{msgs:?}");

        let mut s = review_page(&translated);
        assert!(parity(&mut s).is_empty());
    }

    /// A page without the review-page shape is never flagged, whatever its frames hold.
    #[test]
    fn a_page_without_a_review_shape_is_not_flagged() {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        s.execute("frame.create", &json!({"rect": [20, 20, 400, 500], "content": "text", "text": "An ordinary page of prose. ".repeat(8)})).unwrap();
        assert!(parity(&mut s).is_empty());
    }
}

#[cfg(test)]
mod cover_guard_tests {
    use serde_json::json;

    use crate::Session;
    use crate::cmd::base64_encode;

    fn kinds(s: &mut Session, kind: &str) -> Vec<String> {
        let r = s.execute("preflight.pubgen", &json!({})).unwrap();
        r["issues"].as_array().unwrap().iter().filter(|i| i["kind"] == kind).map(|i| i["message"].as_str().unwrap().to_string()).collect()
    }

    /// A print document with bleed: a graphic crossing a trim edge by less than 3 mm is an
    /// error; one crossing it by more is not.
    #[test]
    fn a_short_bleed_is_an_error() {
        let mut s = Session::new();
        s.execute("file.new", &json!({"intent": "print", "bleed": 12})).unwrap();
        let png = designcraft_render::Rendered { width: 400, height: 200, pixels: vec![200; 400 * 200 * 4] }.to_png();
        // 600 + 16 = 616: 4 pt past the right trim edge (612) — less than 3 mm (8.5 pt).
        s.execute("file.place", &json!({"base64": base64_encode(&png), "name": "short.png", "x": 600, "y": 100, "width": 16})).unwrap();
        s.execute("selection.set", &json!({"ids": []})).unwrap();
        // 600 + 100 = 700: 88 pt past the trim edge — a proper bleed.
        s.execute("file.place", &json!({"base64": base64_encode(&png), "name": "full.png", "x": 600, "y": 300, "width": 100})).unwrap();
        let msgs = kinds(&mut s, "bleedGuard");
        assert_eq!(msgs.len(), 1, "{msgs:?}");
        assert!(msgs[0].contains("right trim edge"), "{msgs:?}");
    }

    /// Without bleed in the document settings nothing is expected to bleed.
    #[test]
    fn no_bleed_settings_means_no_bleed_guard() {
        let mut s = Session::new();
        s.execute("file.new", &json!({"intent": "print"})).unwrap();
        let png = designcraft_render::Rendered { width: 400, height: 200, pixels: vec![200; 400 * 200 * 4] }.to_png();
        s.execute("file.place", &json!({"base64": base64_encode(&png), "name": "edge.png", "x": 600, "y": 100, "width": 16})).unwrap();
        assert!(kinds(&mut s, "bleedGuard").is_empty());
    }

    /// Text on a facing-page spread keeps 5 mm (14.2 pt) from the spine.
    #[test]
    fn text_within_5mm_of_the_spine_is_an_error() {
        let mut s = Session::new();
        s.execute("file.new", &json!({"facingPages": true, "pages": 4})).unwrap();
        // Right page: the spine sits at 612; 616 is 4 pt from it.
        s.execute("frame.create", &json!({"spread": 1, "rect": [616, 100, 700, 300], "content": "text", "text": "Too close"})).unwrap();
        let msgs = kinds(&mut s, "spineGuard");
        assert_eq!(msgs.len(), 1, "{msgs:?}");

        // 18 pt from the spine passes; a frame straddling the spine is certainly too close.
        let mut s = Session::new();
        s.execute("file.new", &json!({"facingPages": true, "pages": 4})).unwrap();
        s.execute("frame.create", &json!({"spread": 1, "rect": [630, 100, 700, 300], "content": "text", "text": "Far enough"})).unwrap();
        s.execute("frame.create", &json!({"spread": 1, "rect": [592, 320, 640, 360], "content": "text", "text": "Across"})).unwrap();
        let msgs = kinds(&mut s, "spineGuard");
        assert_eq!(msgs.len(), 1, "{msgs:?}");
        assert!(msgs[0].contains("0.0 pt"), "{msgs:?}");
    }

    /// A single-sided document has no spine; nothing is flagged however close a frame sits to
    /// the page centre.
    #[test]
    fn single_sided_documents_have_no_spine_guard() {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        s.execute("frame.create", &json!({"rect": [300, 100, 400, 300], "content": "text", "text": "Centre"})).unwrap();
        assert!(kinds(&mut s, "spineGuard").is_empty());
    }

    /// The digital (online PDF) distribution profile drops the print-only guards even for a
    /// print-intent document; `profile: "print"` brings them back.
    #[test]
    fn the_profile_picks_the_print_only_guards() {
        let png = designcraft_render::Rendered { width: 400, height: 200, pixels: vec![200; 400 * 200 * 4] }.to_png();
        let audit = |s: &mut Session, profile: &str| {
            let r = s.execute("preflight.pubgen", &json!({"profile": profile})).unwrap();
            (r["profile"].as_str().unwrap().to_string(), r["issues"].as_array().unwrap().iter().filter(|i| i["kind"] == "bleedGuard").count())
        };
        let mut s = Session::new();
        s.execute("file.new", &json!({"intent": "print", "bleed": 12})).unwrap();
        s.execute("file.place", &json!({"base64": base64_encode(&png), "name": "short.png", "x": 600, "y": 100, "width": 16})).unwrap();
        assert_eq!(audit(&mut s, "digital"), ("digital".into(), 0));
        assert_eq!(audit(&mut s, "print"), ("print".into(), 1));
        // No profile: the document's intent (print) decides.
        assert_eq!(audit(&mut s, ""), ("print".into(), 1));
    }
}
