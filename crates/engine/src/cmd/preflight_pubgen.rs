//! Pubgen preflight audit ("Audit Publication"): the statistical-publication rules on top of
//! the generic preflight — missing `Sumber:`/`Catatan:` note lines and the table-number
//! sequence — merged with `preflight.run` into one checklist.

use serde_json::{Value, json};

use super::preflight::{Issue, check, page_of};
use super::{CommandSpec, cmd, has_doc};
use crate::{Result, Session};

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(query "preflight.pubgen", "Pubgen Preflight Audit", ["Window", "Output", "Pubgen Audit"], None,
        "{minPpi?: 150} → {errors, warnings, issues}: the generic preflight plus the Pubgen rules \
         (missing Sumber:/Catatan: lines, table-number sequence)", has_doc, run)]
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

pub fn pubgen_issues(s: &Session) -> Vec<Issue> {
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
    out
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
    let min = p.get("minPpi").and_then(Value::as_f64).unwrap_or(150.0);
    let mut issues = check(s, min);
    issues.extend(pubgen_issues(s));
    let errors = issues.iter().filter(|i| i.severity == "error").count();
    Ok(json!({"errors": errors, "warnings": issues.len() - errors, "issues": issues}))
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
}
