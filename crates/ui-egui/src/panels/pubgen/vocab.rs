//! Vocabulary collection: the Indonesian words and phrases worth an official translation,
//! gathered from the panel's generated pages into the glossary with blank English columns.
//! Collection is structural (whole header cells, row labels, titles, note and source
//! lines — where BPS terms actually live) plus stopword-delimited phrase runs from prose;
//! single common words are never collected.

use super::ai::{self, BpsGlossary};
use crate::DesignApp;

/// Indonesian function words — never glossary material. Content words (`jumlah`, `tahun`,
/// `angka`, `kecamatan`…) are deliberately absent: those are terms. `menurut` is here
/// because it opens almost every BPS title without carrying meaning.
const STOPWORDS: &[&str] = &[
    "ada",
    "adalah",
    "agar",
    "akan",
    "antara",
    "atau",
    "bahwa",
    "banyak",
    "beberapa",
    "bila",
    "bisa",
    "boleh",
    "bukan",
    "dalam",
    "dan",
    "dari",
    "demi",
    "dengan",
    "di",
    "dia",
    "harus",
    "hanya",
    "hingga",
    "ia",
    "ini",
    "itu",
    "jika",
    "juga",
    "kami",
    "kita",
    "lagi",
    "lalu",
    "maka",
    "masih",
    "maupun",
    "melalui",
    "memiliki",
    "mereka",
    "menurut",
    "meski",
    "oleh",
    "pada",
    "para",
    "per",
    "pun",
    "saja",
    "sampai",
    "sangat",
    "sebagai",
    "sebab",
    "sedangkan",
    "sejak",
    "selama",
    "selain",
    "semua",
    "sendiri",
    "seperti",
    "serta",
    "sehingga",
    "tanpa",
    "tentang",
    "tersebut",
    "untuk",
    "yaitu",
    "yakni",
    "data",
];

/// Words that anchor an organisation phrase: a single capitalized token is only collected
/// when it is one of these (`BPS`, `Dinas`), otherwise proper-noun runs need 2+ words.
const ORG_KEYWORDS: &[&str] = &["bps", "badan", "dinas", "kantor", "statistik"];

/// A general (function) word, a number, a unit like `km2`, or too short to be a term.
fn is_general(word: &str) -> bool {
    if word.chars().count() < 3 {
        return true;
    }
    if STOPWORDS.contains(&word) {
        return true;
    }
    let letters = word.chars().filter(|c| c.is_alphabetic()).count();
    letters == 0 || (word.chars().any(|c| c.is_ascii_digit()) && letters < 3)
}

/// A candidate is worth an entry: it trims clean, carries letters, is not a bare
/// number/year/percent, and stays cell-sized.
fn normalize(s: &str) -> Option<String> {
    let t = s.trim_matches(|c: char| !c.is_alphanumeric()).trim();
    let n = t.chars().count();
    if !(3..=60).contains(&n) || !t.chars().any(|c| c.is_alphabetic()) {
        return None;
    }
    Some(t.to_string())
}

/// The phrases worth glossary entries from one line of Indonesian text, deduplicated:
/// stopword-delimited runs of content words (`"kepadatan penduduk"`, `"kecamatan"`) and
/// proper-noun runs of 2+ Capitalized words (`"Badan Pusat Statistik Kabupaten Karawang"`)
/// — organisation and place names. A single capitalized word is collected only when it is
/// an organisation keyword (`BPS`, `Dinas`).
pub fn candidate_phrases(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Proper-noun runs from the original casing first: a name keeps its display casing
    // and the case-insensitive dedupe then keeps the lowercase content-run from re-adding
    // the same phrase in prose casing.
    let mut run: Vec<&str> = Vec::new();
    for tok in text.split_whitespace() {
        let bare = tok.trim_matches(|c: char| !c.is_alphanumeric());
        let capitalized = bare.chars().next().is_some_and(|c| c.is_uppercase());
        if capitalized && !bare.is_empty() && !is_general(&bare.to_lowercase()) {
            run.push(bare);
        } else {
            flush_name_run(&mut out, &mut seen, &mut run);
        }
    }
    flush_name_run(&mut out, &mut seen, &mut run);

    // Content-word runs, split at stopwords, numbers and units; collected in lowercase —
    // prose casing is incidental, the glossary matches case-insensitively anyway.
    let mut run: Vec<String> = Vec::new();
    for tok in text.split_whitespace() {
        let bare = tok.trim_matches(|c: char| !c.is_alphanumeric());
        if !bare.is_empty() && !is_general(&bare.to_lowercase()) {
            run.push(bare.to_lowercase());
        } else {
            flush_content_run(&mut out, &mut seen, &mut run);
        }
    }
    flush_content_run(&mut out, &mut seen, &mut run);
    out
}

fn flush_content_run(out: &mut Vec<String>, seen: &mut std::collections::HashSet<String>, run: &mut Vec<String>) {
    if !run.is_empty() {
        let phrase = run.join(" ");
        if let Some(n) = normalize(&phrase)
            && seen.insert(n.to_lowercase())
        {
            out.push(n);
        }
    }
    run.clear();
}

fn flush_name_run(out: &mut Vec<String>, seen: &mut std::collections::HashSet<String>, run: &mut Vec<&str>) {
    let is_name = run.len() >= 2 || (run.len() == 1 && ORG_KEYWORDS.contains(&run[0].to_lowercase().as_str()));
    if is_name {
        let phrase = run.join(" ");
        if let Some(n) = normalize(&phrase)
            && seen.insert(n.to_lowercase())
        {
            out.push(n);
        }
    }
    run.clear();
}

/// The collectable pieces of a note or source line: the label word (`Sumber`,
/// `Keterangan`) plus the content split on `/` and commas — organisation names live
/// there. Year and page tails (`2025`, `2025-2027`) are dropped.
pub fn split_source_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line.trim();
    if let Some((label, tail)) = rest.split_once(':') {
        if let Some(l) = normalize(label) {
            out.push(l);
        }
        rest = tail;
    }
    for part in rest.split(['/', ',']) {
        let part = part.trim();
        if part.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue; // a year or page range, not a term
        }
        if let Some(p) = normalize(part) {
            out.push(p);
        }
    }
    out
}

/// Feed one text unit: the whole unit (a cell or title is exactly a glossary term) plus
/// its phrases and, for note/source lines, the split parts. Returns new terms added.
fn feed_unit(glossary: &mut BpsGlossary, unit: &str) -> usize {
    let mut added = 0;
    if let Some(w) = normalize(unit)
        && glossary.collect(&w)
    {
        added += 1;
    }
    for p in candidate_phrases(unit) {
        if glossary.collect(&p) {
            added += 1;
        }
    }
    if unit.contains(':') || unit.contains('/') || unit.contains(',') {
        for p in split_source_line(unit) {
            if glossary.collect(&p) {
                added += 1;
            }
        }
    }
    added
}

/// Collect the vocabulary of the whole open document into `st.glossary`: every generated
/// review or table page contributes its units. Returns the number of new terms.
pub fn collect_document(app: &DesignApp, st: &mut super::PubgenState) -> usize {
    let pages = app.session.active().map(|s| s.doc.page_count()).unwrap_or(0);
    let mut added = 0;
    for page_idx in 0..pages {
        if let Some(target) = ai::detect_table_page_target(app, page_idx) {
            let units = ai::collect_table_page_units(app, &target, &st.glossary);
            for (unit, _) in units {
                added += feed_unit(&mut st.glossary, &unit);
            }
        } else if let Some(target) = ai::detect_page_target(app, page_idx) {
            let units = ai::collect_page_units(app, &target);
            for (unit, _) in units {
                added += feed_unit(&mut st.glossary, &unit);
            }
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_runs_are_split_at_stopwords_and_numbers() {
        let phrases = candidate_phrases("Data kepadatan penduduk menurut kecamatan tahun 2024");
        assert!(phrases.contains(&"kepadatan penduduk".to_string()), "{phrases:?}");
        assert!(phrases.iter().any(|p| p.contains("kecamatan")), "{phrases:?}");
        assert!(!phrases.iter().any(|p| p.to_lowercase().split(' ').any(|w| w == "menurut" || w == "data")), "{phrases:?}");
        assert!(!phrases.iter().any(|p| p.contains("2024")), "{phrases:?}");
    }

    #[test]
    fn proper_noun_runs_are_collected_whole_and_single_words_only_for_org_keywords() {
        let phrases = candidate_phrases("data dari Badan Pusat Statistik Kabupaten Karawang");
        assert!(phrases.contains(&"Badan Pusat Statistik Kabupaten Karawang".to_string()), "{phrases:?}");
        let single = candidate_phrases("keterangan oleh Dinas");
        assert!(single.contains(&"Dinas".to_string()), "{single:?}");
        let plain = candidate_phrases("hasil Warungbambu baik");
        assert!(!plain.iter().any(|p| p == "Warungbambu"), "one capitalized word is not a term: {plain:?}");
    }

    #[test]
    fn source_lines_split_into_label_and_parts_dropping_years() {
        let parts = split_source_line("Sumber: BPS Kabupaten Karawang/Dinas terkait, 2025");
        assert!(parts.contains(&"Sumber".to_string()), "{parts:?}");
        assert!(parts.contains(&"BPS Kabupaten Karawang".to_string()), "{parts:?}");
        assert!(parts.contains(&"Dinas terkait".to_string()), "{parts:?}");
        assert!(!parts.iter().any(|p| p.contains("2025")), "{parts:?}");
    }

    #[test]
    fn collection_deduplicates_and_counts() {
        let mut g = BpsGlossary::default();
        assert!(g.collect("Kecamatan"));
        assert!(!g.collect("kecamatan"), "same term, different case");
        assert!(g.collect("Kepadatan Penduduk"));
        assert_eq!(g.terms.len(), 2);
        assert_eq!(g.terms[0].count, 1);
        assert_eq!(g.terms[1].count, 0);
        // Only official English is ever applied.
        g.set_ai("Kecamatan", "District");
        assert_eq!(g.lookup("Kecamatan"), None, "an AI draft is not an official translation");
        g.set_official("Kecamatan", "Subdistrict");
        assert_eq!(g.lookup(" KECAMATAN "), Some("Subdistrict"));
        assert!(g.remove("KECAMATAN"));
        assert_eq!(g.lookup("Kecamatan"), None);
    }

    #[test]
    fn units_feed_whole_terms_phrases_and_source_parts() {
        let mut g = BpsGlossary::default();
        assert_eq!(feed_unit(&mut g, "Persentase"), 1, "a cell is one term");
        feed_unit(&mut g, "Data kepadatan penduduk menurut kecamatan");
        assert!(g.terms.iter().any(|t| t.matches("kepadatan penduduk")));
        feed_unit(&mut g, "Sumber: BPS Kabupaten Karawang");
        assert!(g.terms.iter().any(|t| t.matches("BPS Kabupaten Karawang")));
        // Re-feeding counts without duplicating.
        let before = g.terms.len();
        feed_unit(&mut g, "Persentase");
        assert_eq!(g.terms.len(), before);
    }
}
