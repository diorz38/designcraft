//! The table-of-contents engines: entry, table-list, chart-list and paragraph TOCs.

use super::*;

/// The Quick Actions "TOC style" choices: what the table of contents lists.
pub const TOC_KINDS: [&str; 4] = ["TOC Entry", "TOC Table", "TOC Chart", "TOC Paragraph"];

/// The paragraph style of a narrative page's title: the pages the `GXX` and `RXX` sheets produce.
/// The paragraph table of contents lists these pages and nothing else, so a table, chart, divider
/// or publication-note page never shows up in it.
pub const TOC_NARRATIVE_STYLE: &str = "TOC Chapter";

// The English twin title of a Dual Lang narrative page ("Chapter 01 Geography"): tagged with
// this style at generation, so the paragraph TOC pairs it with the Indonesian chapter row
// ("BAB 01 Geografi / Geography") instead of listing it as a page of its own. Only tagged when
// the pairing checkbox is on with the Dual Lang toggle.

/// The words the first text box on a page must start with for each TOC style; none lists every
/// paragraph of the tagged style (the original behaviour). A paragraph list takes chapter pages
/// only: `RXX` and `BXX` pages open with their "BAB 01" line, table and chart pages don't.
pub fn toc_kind_words(kind: usize) -> &'static [&'static str] {
    match kind {
        1 => &["Tabel", "Table"],
        2 => &["Grafik", "Chart"],
        3 => &["BAB"],
        _ => &[],
    }
}

/// The title a TOC style fills in when it is picked; `None` leaves the typed title alone.
pub fn toc_kind_title(kind: usize, bilingual: bool) -> Option<&'static str> {
    match (kind, bilingual) {
        (1, false) => Some("Daftar Tabel"),
        (1, true) => Some("Daftar Tabel / List of Tables"),
        (2, false) => Some("Daftar Grafik"),
        (2, true) => Some("Daftar Grafik / List of Charts"),
        (3, false) => Some("Daftar Isi"),
        (3, true) => Some("Daftar Isi / Table of Contents"),
        _ => None,
    }
}

/// The TOC style a stored table of contents was generated with: its entry's `pageTitle` filter.
pub fn toc_kind_of_words(words: &[String]) -> usize {
    let has = |w: &str| words.iter().any(|x| x.eq_ignore_ascii_case(w));
    if has("bab") {
        3
    } else if has("grafik") || has("chart") {
        2
    } else if has("tabel") || has("table") {
        1
    } else {
        0
    }
}

/// Strip type and number prefixes from a title for TablePage layout (which puts the type in the column header
/// and the number in column 1, leaving only the title text for column 2).
pub(crate) fn strip_tablepage_title_prefix<'a>(title: &'a str, num: &str, typ: &str) -> &'a str {
    let mut t = title.trim();
    if !typ.is_empty() {
        if let Some(rest) = t.strip_prefix(typ) {
            t = rest.trim();
        } else if let Some(rest) = t.to_lowercase().strip_prefix(&typ.to_lowercase()) {
            t = title[title.len() - rest.len()..].trim();
        }
    } else {
        for prefix in &["Tabel", "Table", "Grafik", "Chart"] {
            if let Some(rest) = t.strip_prefix(prefix) {
                t = rest.trim();
                break;
            } else if let Some(rest) = t.to_lowercase().strip_prefix(&prefix.to_lowercase()) {
                t = title[title.len() - rest.len()..].trim();
                break;
            }
        }
    }

    let clean_num = num.trim_end_matches('.');
    if !clean_num.is_empty() {
        if let Some(rest) = t.strip_prefix(clean_num) {
            t = rest.trim();
        } else if let Some(rest) = t.strip_prefix(num) {
            t = rest.trim();
        }
    } else if let Some((first, rest)) = t.split_once(' ') {
        let f_clean = first.trim_end_matches('.');
        if f_clean.chars().any(|c| c.is_ascii_digit()) && f_clean.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
            t = rest.trim();
        }
    }

    t.trim_start_matches(['.', '-', ':', ';', ' '])
}

/// Wrap a TOC entry title using forced line breaks (`\u{2028}`) so:
/// 1. The first line wraps early (leaving ample breathing room for leader dots and page number).
/// 2. Continuation lines fit comfortably within the remaining column width after hanging indent.
pub(crate) fn wrap_toc_title(title: &str, width: f64, hanging_indent: f64, page_str: Option<&str>) -> String {
    let clean_title = title.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
    let clean_title = clean_title.trim();
    if clean_title.is_empty() {
        return String::new();
    }

    let page_w = page_str.map(estimate_text_advance).unwrap_or(20.0);
    // Reserve breathing room for tab leader dots before the page number: at least 48pt
    let min_leader_gap = 48.0;
    let right_reserve = page_w + min_leader_gap;

    let max_line0 = (width - right_reserve).max(80.0);
    let max_line_cont = (width - hanging_indent - right_reserve).max(60.0);

    // If the full title already fits on the first line with plenty of leader dots, keep it on one line.
    if estimate_text_advance(clean_title) <= max_line0 {
        return clean_title.to_string();
    }

    let words: Vec<&str> = clean_title.split_whitespace().collect();
    if words.len() <= 1 {
        return clean_title.to_string();
    }

    let mut lines: Vec<String> = Vec::new();
    let mut cur_line = String::new();
    let mut is_first = true;

    for word in words {
        let max_w = if is_first { max_line0 } else { max_line_cont };
        let candidate = if cur_line.is_empty() { word.to_string() } else { format!("{cur_line} {word}") };

        if estimate_text_advance(&candidate) <= max_w || cur_line.is_empty() {
            cur_line = candidate;
        } else {
            lines.push(cur_line);
            cur_line = word.to_string();
            is_first = false;
        }
    }

    if !cur_line.is_empty() {
        lines.push(cur_line);
    }

    lines.join(&designcraft_doc::story::FORCED_LINE_BREAK.to_string())
}

/// Wrap a title specifically for TablePage layout, where every line of the title starts
/// after `hanging_indent`, and the page number reserve sits at the right margin.
pub(crate) fn wrap_tablepage_title(title: &str, width: f64, hanging_indent: f64, page_str: Option<&str>) -> String {
    let clean_title = title.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
    let clean_title = clean_title.trim();
    if clean_title.is_empty() {
        return String::new();
    }

    let page_w = page_str.map(estimate_text_advance).unwrap_or(20.0);
    let min_leader_gap = 48.0;
    let right_reserve = page_w + min_leader_gap;
    let max_line_w = (width - hanging_indent - right_reserve).max(60.0);

    if estimate_text_advance(clean_title) <= max_line_w {
        return clean_title.to_string();
    }

    let words: Vec<&str> = clean_title.split_whitespace().collect();
    if words.len() <= 1 {
        return clean_title.to_string();
    }

    let mut lines: Vec<String> = Vec::new();
    let mut cur_line = String::new();

    for word in words {
        let candidate = if cur_line.is_empty() { word.to_string() } else { format!("{cur_line} {word}") };

        if estimate_text_advance(&candidate) <= max_line_w || cur_line.is_empty() {
            cur_line = candidate;
        } else {
            lines.push(cur_line);
            cur_line = word.to_string();
        }
    }

    if !cur_line.is_empty() {
        lines.push(cur_line);
    }

    lines.join(&designcraft_doc::story::FORCED_LINE_BREAK.to_string())
}

/// The first column header a TablePage table of contents writes: the type word of its own kind
/// ("Grafik"/"Chart" for a list of charts, "Tabel"/"Table" otherwise) in the language the source
/// records use. In dual lang mode, it writes two lines (Indonesian on line 1, English on line 2).
pub(crate) fn toc_header_line(plan: &TocPlan, st: &PubgenState) -> String {
    if st.bilingual_output {
        let (id_type, en_type) = if plan.kind() == 2 { ("Grafik", "Chart") } else { ("Tabel", "Table") };
        return format!("{id_type}\tHalaman{}\u{2028}{en_type}\tPage", "");
    }
    let english = st.card1_records.iter().any(|r| {
        let t = r.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("");
        t.eq_ignore_ascii_case("table") || t.eq_ignore_ascii_case("chart")
    });
    let w = if english { locale::words(Locale::En) } else { locale::words(st.locale) };
    let type_word = if plan.kind() == 2 { w.figure } else { w.table };
    format!("{type_word}\t{}", w.page_column)
}

/// Whether a line is the column header of a TablePage table of contents ("Tabel\thalaman",
/// "Grafik\t…": the word a list of tables or a list of charts writes).
pub(crate) fn is_toc_header_line(line: &str) -> bool {
    ["Tabel\t", "Table\t", "Grafik\t", "Chart\t"].iter().any(|p| line.starts_with(p))
}

/// Detect whether a record represents a Section header (e.g. "Bab 4", "4.3 Perumahan", or "Section 2")
/// which in TablePage layout does not receive a page number or dot leaders.
/// Lists of tables (kind 1) or charts (kind 2) never contain section headers.
pub(crate) fn is_section_header(toc_kind: usize, r_type: &str, num_str: &str, title: &str) -> bool {
    if toc_kind != 0 {
        return false;
    }
    let t_lower = r_type.trim().to_lowercase();
    if t_lower == "bab" || t_lower == "section" || t_lower == "subbab" || t_lower == "kategori" || t_lower == "chapter" {
        return true;
    }
    let num_clean = num_str.trim().trim_end_matches('.');
    let segs: Vec<&str> = num_clean.split('.').filter(|s| !s.is_empty()).collect();
    if (t_lower.is_empty() || t_lower == "section" || t_lower == "bab") && segs.len() == 2 && !segs.iter().any(|s| s.len() > 3) {
        let title_lower = title.trim().to_lowercase();
        if !title_lower.starts_with("tabel ") && !title_lower.starts_with("table ") && !title_lower.starts_with("grafik ") {
            return true;
        }
    }
    false
}

/// What a table of contents lists and how it reads. "Add TOC" builds it from the panel's controls;
/// a rebuild builds it from the stored table so it keeps the TOC style, title and filter the user
/// chose when it was created.
#[derive(Clone, Debug)]
/// One listed paragraph style of a table of contents and the level it nests at.
pub(crate) struct TocLevel {
    pub(crate) style: String,
    pub(crate) level: u8,
    /// `pageTitle` filter: list a page only when its first text box starts with one of these.
    pub(crate) words: Vec<String>,
    /// Prefix the entry with the page's first text box (the chapter line): a narrative page's
    /// title reads "BAB 01 Geografi" instead of "Geografi".
    pub(crate) chapter_number: bool,
    /// Pair bilingual headings (ID / EN) of this style into one row on Dual Lang pages.
    pub(crate) pair: bool,
}

#[derive(Clone, Debug)]
pub struct TocPlan {
    /// The styles the table of contents lists, outermost level first.
    pub(crate) levels: Vec<TocLevel>,
    /// Title line of the table of contents.
    title: String,
    /// 0 Standard, 1 TablePage (BPS catalog layout).
    layout: usize,
    hanging: bool,
    include_self: bool,
}

impl TocPlan {
    /// The plan the panel's TOC controls describe. A paragraph list nests chapter titles over
    /// the headings the imported RTF files carry.
    pub(crate) fn from_state(st: &PubgenState) -> Self {
        // Pairing only makes sense when the Dual Lang toggle is on: without it the pages carry no
        // second language at all, so the checkbox stays inert.
        let pair = st.toc_pair_bilingual && st.bilingual_output;
        let level = |style: String, words: Vec<String>, chapter_number: bool| TocLevel { style, level: 1, words, chapter_number, pair };
        let levels = if st.toc_kind == 3 {
            // The narrative pages (the G and Ulasan sheets) and the headings they carry.
            vec![
                level(TOC_NARRATIVE_STYLE.to_string(), Vec::new(), true),
                TocLevel { style: "Heading 1".into(), level: 2, words: Vec::new(), chapter_number: false, pair },
                TocLevel { style: "Heading 2".into(), level: 3, words: Vec::new(), chapter_number: false, pair },
            ]
        } else if st.toc_kind == 0 {
            // The plain list: every tagged page title plus the narrative pages.
            vec![
                level(st.toc_style.clone(), toc_kind_words(0).iter().map(|w| (*w).to_string()).collect(), false),
                level(TOC_NARRATIVE_STYLE.to_string(), Vec::new(), false),
            ]
        } else {
            vec![level(st.toc_style.clone(), toc_kind_words(st.toc_kind).iter().map(|w| (*w).to_string()).collect(), false)]
        };
        let is_para = st.toc_kind == 3;
        Self {
            levels,
            title: st.toc_title.trim().to_string(),
            layout: st.toc_layout_style,
            hanging: st.toc_hanging_indent && !is_para,
            include_self: st.toc_include_self,
        }
    }

    /// The plan a stored table of contents rebuilds with: its own styles, levels, title and
    /// filters, with the panel's layout and indentation options.
    fn from_toc(st: &PubgenState, toc: &designcraft_doc::Toc) -> Self {
        let levels: Vec<TocLevel> = toc
            .entries
            .iter()
            .map(|e| TocLevel {
                style: e.style.clone(),
                level: e.level,
                words: e.page_title.clone(),
                chapter_number: e.chapter_number,
                pair: e.pair_columns,
            })
            .collect();
        let levels = if levels.is_empty() {
            vec![TocLevel { style: st.toc_style.clone(), level: 1, words: Vec::new(), chapter_number: false, pair: false }]
        } else {
            levels
        };
        let is_para = levels.first().is_some_and(|l| l.style == TOC_NARRATIVE_STYLE);
        Self {
            levels,
            title: toc.title.trim().to_string(),
            layout: st.toc_layout_style,
            hanging: st.toc_hanging_indent && !is_para,
            include_self: st.toc_include_self,
        }
    }

    /// The Quick Actions TOC style this plan came from (for messages).
    pub(crate) fn kind(&self) -> usize {
        if self.levels.iter().any(|l| l.level > 1) { 3 } else { toc_kind_of_words(&self.levels.first().map(|l| l.words.clone()).unwrap_or_default()) }
    }

    /// A list of chapter titles over their headings, regenerated with each entry's own level.
    /// The narrative list leads with the narrative pages; the plain list leads with the tagged
    /// page titles and adds the narrative pages as a second level of the same depth.
    pub(crate) fn is_paragraph_toc(&self) -> bool {
        self.levels.first().is_some_and(|l| l.style == TOC_NARRATIVE_STYLE)
    }

    /// The paragraph styles this list's entries take: the contents list owns `TOC Level n`, and
    /// the Tables and Charts lists get a family of their own. The styles carry each list's tab
    /// stop and indent, so sharing them would let regenerating one list restyle the pages of
    /// another (a table list's rows moved when the contents list was rebuilt).
    fn entry_prefix(&self) -> &'static str {
        match self.kind() {
            1 => "TOC Table Level",
            2 => "TOC Chart Level",
            _ => "TOC Level",
        }
    }

    /// Documents whose pages predate the narrative style keep their chapters: the card's TOC style
    /// on pages that open with "BAB" still lists them (with the chapter line, as a new page's),
    /// and a `pageTitle` filter keeps table and chart pages out. Only used while the document has
    /// no narrative-styled paragraph at all, so regenerated pages never take this path.
    fn with_legacy_chapters(&self, st: &PubgenState) -> Self {
        let mut plan = self.clone();
        plan.levels.push(TocLevel {
            style: st.toc_style.clone(),
            level: 1,
            words: toc_kind_words(3).iter().map(|w| (*w).to_string()).collect(),
            chapter_number: true,
            pair: st.toc_pair_bilingual && st.bilingual_output,
        });
        plan
    }

    /// Whether the table of contents writes a title line (an empty field writes none, which the
    /// table/page layout needs to start straight at its header).
    fn has_title(&self) -> bool {
        !self.title.trim().is_empty()
    }

    /// The title the definition carries: "Contents" when the field is empty.
    fn title_or_contents(&self) -> &str {
        if self.has_title() { self.title.trim() } else { "Contents" }
    }

    /// The `toc.generate` entries that list this plan's pages.
    pub(crate) fn entries_json(&self) -> Value {
        let prefix = self.entry_prefix();
        Value::Array(
            self.levels
                .iter()
                .map(|l| {
                    let mut e = json!({
                        "style": l.style,
                        "level": l.level,
                        "entryStyle": format!("{prefix} {}", l.level),
                        "pageNumber": "after",
                        "separator": "\t"
                    });
                    if !l.words.is_empty() {
                        e["pageTitle"] = json!(l.words);
                    }
                    if l.chapter_number {
                        e["chapterNumber"] = json!(true);
                    }
                    if l.pair {
                        e["pairColumns"] = json!(true);
                    }
                    e
                })
                .collect(),
        )
    }
}

pub(crate) fn enrich_toc_story(
    app: &mut DesignApp,
    sid: designcraft_doc::StoryId,
    st: &PubgenState,
    plan: &TocPlan,
    width: f64,
    hanging_indent: f64,
    toc_page: usize,
) {
    // A paragraph list is generated straight from the document's styles: rewriting its text
    // would desynchronize the entries from the "TOC Level n" styles the engine gave them.
    if plan.is_paragraph_toc() {
        return;
    }
    let _ = app.session.edit(|doc, _| {
        let full_text = doc.story(sid).map(|s| s.text.clone()).ok_or(designcraft_doc::DocError::NoStory(sid))?;
        let toc_page_label = doc.page_name(toc_page);
        let self_title = plan.title_or_contents();

        let is_table_page = plan.layout == 1;

        // The header follows the table of contents' own kind: a list of charts reads "Grafik".
        let header_line = toc_header_line(plan, st);

        let mut new_lines = Vec::new();
        let mut changed = false;
        let mut used_records: Vec<bool> = vec![false; st.card1_records.len()];

        let lines: Vec<&str> = full_text.split('\n').collect();
        let orig_title_line = lines.first().copied().unwrap_or("");

        if is_table_page {
            if plan.has_title() {
                new_lines.push(plan.title.trim().to_string());
                new_lines.push(header_line.clone());
            } else {
                new_lines.push(header_line.clone());
            }
        } else {
            new_lines.push(orig_title_line.to_string());
        }

        // 1. Pre-process raw TOC lines: extract (title, page) and coalesce multi-line entries on the same page
        let mut raw_entries: Vec<(String, Option<String>)> = Vec::new();
        for (idx, line) in lines.into_iter().enumerate() {
            if idx == 0 || line.trim().is_empty() {
                continue;
            }

            let line_clean = line.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
            if is_table_page && (line_clean == header_line || is_toc_header_line(&line_clean)) {
                continue;
            }

            let (raw_title, page_part) = match line_clean.split_once('\t') {
                Some((t, p)) => (t.trim().to_string(), Some(p.trim().to_string())),
                None => (line_clean.trim().to_string(), None),
            };

            if !raw_title.is_empty() {
                raw_entries.push((raw_title, page_part));
            }
        }

        // Coalesce entries from the same page where one is type+number and the other is the title
        let mut coalesced: Vec<(String, Option<String>)> = Vec::new();
        let mut c_idx = 0;
        while c_idx < raw_entries.len() {
            let (cur_t, cur_p) = &raw_entries[c_idx];
            if c_idx + 1 < raw_entries.len() {
                let (next_t, next_p) = &raw_entries[c_idx + 1];
                if cur_p.is_some() && cur_p == next_p {
                    if is_type_and_number_only(cur_t) && !is_type_and_number_only(next_t) {
                        coalesced.push((format!("{cur_t} {next_t}"), cur_p.clone()));
                        c_idx += 2;
                        changed = true;
                        continue;
                    } else if !is_type_and_number_only(cur_t) && is_type_and_number_only(next_t) {
                        coalesced.push((format!("{next_t} {cur_t}"), cur_p.clone()));
                        c_idx += 2;
                        changed = true;
                        continue;
                    }
                }
            }
            coalesced.push((cur_t.clone(), cur_p.clone()));
            c_idx += 1;
        }

        for (raw_title, page_part) in coalesced {
            // Check if this line is an existing TOC self-entry
            let is_self_entry = raw_title.eq_ignore_ascii_case("contents")
                || raw_title.eq_ignore_ascii_case("table of contents")
                || raw_title.eq_ignore_ascii_case("daftar isi")
                || raw_title.eq_ignore_ascii_case("daftar tabel")
                || raw_title.eq_ignore_ascii_case("daftar grafik")
                || raw_title.eq_ignore_ascii_case(self_title);

            if is_self_entry {
                if plan.include_self {
                    let wrapped =
                        if plan.hanging { wrap_toc_title(self_title, width, hanging_indent, Some(&toc_page_label)) } else { self_title.to_string() };
                    new_lines.push(format!("{wrapped}\t{toc_page_label}"));
                } else {
                    changed = true;
                }
                continue;
            }

            let mut matched_type = String::new();
            let mut matched_num = String::new();
            let mut clean_title = raw_title.clone();
            let mut matched_record = false;

            // 1. Try matching against records in st.card1_records
            for (r_idx, row) in st.card1_records.iter().enumerate() {
                if used_records.get(r_idx).copied().unwrap_or(false) {
                    continue;
                }
                let r_type = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let r_num = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let r_title = row.get(st.card1_title_col.min(2)).map(String::as_str).unwrap_or("").trim();

                let titles_match = !r_title.is_empty()
                    && (raw_title == r_title || raw_title.contains(r_title) || raw_title.ends_with(r_title) || r_title.ends_with(&raw_title) || {
                        let norm_r = strip_tablepage_title_prefix(r_title, r_num, r_type);
                        let norm_raw = strip_tablepage_title_prefix(&raw_title, r_num, r_type);
                        !norm_r.is_empty() && norm_r.eq_ignore_ascii_case(norm_raw)
                    });

                if titles_match {
                    matched_record = true;
                    matched_type = r_type.to_string();
                    matched_num = r_num.to_string();
                    clean_title = strip_tablepage_title_prefix(r_title, r_num, r_type).to_string();
                    if clean_title.is_empty() {
                        clean_title = strip_tablepage_title_prefix(&raw_title, r_num, r_type).to_string();
                    }
                    if let Some(used) = used_records.get_mut(r_idx) {
                        *used = true;
                    }
                    break;
                }
            }

            // 2. Strip prefix if in raw_title (e.g. "Tabel 4.2.1 Banyaknya..." or "Grafik 02.01 Panjang Jalan...")
            if !matched_record {
                let trimmed_title = raw_title.trim();
                let (det_type, after_type) = if let Some(rest) = trimmed_title.strip_prefix("Tabel ") {
                    ("Tabel", rest.trim())
                } else if let Some(rest) = trimmed_title.strip_prefix("Table ") {
                    ("Table", rest.trim())
                } else if let Some(rest) = trimmed_title.strip_prefix("Grafik ") {
                    ("Grafik", rest.trim())
                } else if let Some(rest) = trimmed_title.strip_prefix("Chart ") {
                    ("Chart", rest.trim())
                } else {
                    let default_type = if plan.kind() == 2 {
                        "Grafik"
                    } else if plan.kind() == 1 {
                        "Tabel"
                    } else {
                        ""
                    };
                    (default_type, trimmed_title)
                };

                if let Some((first_word, rest_of_title)) = after_type.split_once(' ')
                    && first_word.chars().any(|c| c.is_ascii_digit())
                    && first_word.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-')
                {
                    matched_type = det_type.to_string();
                    matched_num = first_word.to_string();
                    clean_title = rest_of_title.trim().to_string();
                } else if after_type.chars().any(|c| c.is_ascii_digit()) && after_type.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
                    matched_type = det_type.to_string();
                    matched_num = after_type.to_string();
                    clean_title = String::new();
                }
            }

            // 3. Fallback: inspect page frames ONLY if no record in st.card1_records matched!
            if matched_num.is_empty()
                && !matched_record
                && let Some(p_str) = &page_part
                && let Some((t_val, n_val)) = find_type_and_number_on_page(doc, p_str, &raw_title)
            {
                matched_type = t_val;
                matched_num = n_val;
            }

            if matched_type.is_empty() {
                if plan.kind() == 2 {
                    matched_type = "Grafik".to_string();
                } else if plan.kind() == 1 {
                    matched_type = "Tabel".to_string();
                }
            }

            if is_table_page {
                clean_title = strip_tablepage_title_prefix(&clean_title, &matched_num, &matched_type).to_string();
                let clean_num = matched_num.trim_end_matches('.');
                let is_sec = is_section_header(plan.kind(), &matched_type, clean_num, &clean_title);
                if is_sec {
                    let entry = if !clean_num.is_empty() { format!("{clean_num}\t{clean_title}") } else { clean_title };
                    new_lines.push(entry);
                } else {
                    let wrapped =
                        if plan.hanging { wrap_tablepage_title(&clean_title, width, hanging_indent, page_part.as_deref()) } else { clean_title };
                    let entry = if let Some(p_str) = &page_part {
                        if !clean_num.is_empty() { format!("{clean_num}\t{wrapped}\t{p_str}") } else { format!("{wrapped}\t{p_str}") }
                    } else {
                        wrapped
                    };
                    new_lines.push(entry);
                }
                changed = true;
            } else {
                // Standard mode
                let mut prefix_opt = None;
                let expected_prefix =
                    [&matched_type, &matched_num].into_iter().filter(|s| !s.is_empty()).map(|s| s.as_str()).collect::<Vec<_>>().join(" ");

                let starts_with_type = !matched_type.is_empty() && raw_title.to_lowercase().starts_with(&matched_type.to_lowercase());
                if !expected_prefix.is_empty() && !raw_title.starts_with(&expected_prefix) && !starts_with_type {
                    prefix_opt = Some(expected_prefix);
                }

                let title_to_use = if let Some(prefix) = prefix_opt { format!("{prefix} {raw_title}") } else { raw_title.to_string() };

                let wrapped_title =
                    if plan.hanging { wrap_toc_title(&title_to_use, width, hanging_indent, page_part.as_deref()) } else { title_to_use };

                if let Some(p_str) = &page_part {
                    new_lines.push(format!("{wrapped_title}\t{p_str}"));
                } else {
                    new_lines.push(wrapped_title);
                }
                changed = true;
            }
        }

        // If toc_include_self is requested and not yet added, insert right after title/header
        if plan.include_self && !new_lines.is_empty() {
            let already_has = new_lines.iter().skip(1).any(|l| {
                let t = l.split_once('\t').map(|(t, _)| t.trim()).unwrap_or(l.trim());
                t.eq_ignore_ascii_case(self_title)
                    || t.eq_ignore_ascii_case("contents")
                    || t.eq_ignore_ascii_case("table of contents")
                    || t.eq_ignore_ascii_case("daftar isi")
                    || t.eq_ignore_ascii_case("daftar tabel")
                    || t.eq_ignore_ascii_case("daftar grafik")
            });
            if !already_has {
                let wrapped =
                    if plan.hanging { wrap_toc_title(self_title, width, hanging_indent, Some(&toc_page_label)) } else { self_title.to_string() };
                let insert_pos = if is_table_page && new_lines.len() > 1 { 2 } else { 1 };
                let insert_pos = insert_pos.min(new_lines.len());
                new_lines.insert(insert_pos, format!("{wrapped}\t{toc_page_label}"));
                changed = true;
            }
        }

        if changed || is_table_page {
            let new_text = new_lines.join("\n");
            let story = doc.story_mut(sid).ok_or(designcraft_doc::DocError::NoStory(sid))?;
            let len = story.len();
            story.replace(0..len, &new_text);
        }

        Ok(serde_json::Value::Null)
    });
}

pub(crate) fn ensure_toc_entry_style(app: &mut DesignApp, width: f64, hanging: bool, hanging_indent: f64, layout_style: usize, prefix: &str) {
    let level_one = format!("{prefix} 1");
    let _ = app.session.edit(|d, _| {
        if d.styles.char_style("TOC English").is_none() {
            d.styles_mut().character.push(designcraft_doc::CharacterStyle {
                name: "TOC English".into(),
                based_on: None,
                chars: designcraft_doc::CharAttrs { font_style: Some("Italic".into()), ..Default::default() },
                shortcut: String::new(),
            });
        }

        let tab_stop = designcraft_doc::TabStop {
            position: width.max(36.0),
            align: designcraft_doc::TabAlign::Right,
            leader: ". ".into(),
            align_on: String::new(),
        };

        if layout_style == 1 {
            // TablePage layout styles
            let tab_stop_num = designcraft_doc::TabStop {
                position: hanging_indent,
                align: designcraft_doc::TabAlign::Left,
                leader: String::new(),
                align_on: String::new(),
            };
            let (li, fi) = if hanging { (Some(hanging_indent), Some(-hanging_indent)) } else { (Some(0.0), Some(0.0)) };

            let mut found_level1 = false;
            let mut found_sec = false;
            let mut found_hdr = false;

            for style in &mut d.styles_mut().paragraph {
                if style.name == level_one {
                    found_level1 = true;
                    style.para.left_indent = li;
                    style.para.first_line_indent = fi;
                    style.para.tabs = Some(vec![tab_stop_num.clone(), tab_stop.clone()]);
                    style.para.space_before = Some(3.0);
                } else if style.name == "TOC Section Header" {
                    found_sec = true;
                    style.para.left_indent = li;
                    style.para.first_line_indent = fi;
                    style.para.tabs = Some(vec![tab_stop_num.clone()]);
                    style.para.space_before = Some(8.0);
                    style.para.space_after = Some(2.0);
                } else if style.name == "TOC TablePage Header" {
                    found_hdr = true;
                    style.para.tabs = Some(vec![designcraft_doc::TabStop {
                        position: width.max(36.0),
                        align: designcraft_doc::TabAlign::Right,
                        leader: String::new(),
                        align_on: String::new(),
                    }]);
                    style.para.space_after = Some(8.0);
                    if style.chars.size.is_none() {
                        style.chars.size = Some(10.0);
                    }
                    if style.chars.fill_tint.is_none() {
                        style.chars.fill_tint = Some(0.6);
                    }
                }
            }

            let st = d.styles_mut();
            if !found_level1 {
                st.paragraph.push(designcraft_doc::ParagraphStyle {
                    name: level_one.clone(),
                    based_on: Some(designcraft_doc::story::BASIC_PARAGRAPH.into()),
                    next_style: None,
                    para: designcraft_doc::ParaAttrs {
                        left_indent: li,
                        first_line_indent: fi,
                        space_before: Some(3.0),
                        align: Some(designcraft_doc::Align::Left),
                        tabs: Some(vec![tab_stop_num.clone(), tab_stop]),
                        ..Default::default()
                    },
                    chars: designcraft_doc::CharAttrs::default(),
                    shortcut: String::new(),
                });
            }
            if !found_sec {
                st.paragraph.push(designcraft_doc::ParagraphStyle {
                    name: "TOC Section Header".to_string(),
                    based_on: Some(designcraft_doc::story::BASIC_PARAGRAPH.into()),
                    next_style: None,
                    para: designcraft_doc::ParaAttrs {
                        left_indent: li,
                        first_line_indent: fi,
                        space_before: Some(8.0),
                        space_after: Some(2.0),
                        align: Some(designcraft_doc::Align::Left),
                        tabs: Some(vec![tab_stop_num]),
                        ..Default::default()
                    },
                    chars: designcraft_doc::CharAttrs { font_style: Some("Bold".into()), size: Some(10.5), ..Default::default() },
                    shortcut: String::new(),
                });
            }
            if !found_hdr {
                st.paragraph.push(designcraft_doc::ParagraphStyle {
                    name: "TOC TablePage Header".to_string(),
                    based_on: Some(designcraft_doc::story::BASIC_PARAGRAPH.into()),
                    next_style: None,
                    para: designcraft_doc::ParaAttrs {
                        space_after: Some(8.0),
                        align: Some(designcraft_doc::Align::Left),
                        tabs: Some(vec![designcraft_doc::TabStop {
                            position: width.max(36.0),
                            align: designcraft_doc::TabAlign::Right,
                            leader: String::new(),
                            align_on: String::new(),
                        }]),
                        ..Default::default()
                    },
                    chars: designcraft_doc::CharAttrs { size: Some(10.0), fill_tint: Some(0.6), ..Default::default() },
                    shortcut: String::new(),
                });
            }
        } else {
            let mut found = false;
            for style in &mut d.styles_mut().paragraph {
                let Some(idx) = style.name.strip_prefix(prefix).and_then(|rest| rest.strip_prefix(' ')).and_then(|n| n.parse::<usize>().ok()) else {
                    continue;
                };
                found = true;
                let base_indent = (idx.saturating_sub(1) as f64) * 12.0;
                let (li, fi) = if hanging { (Some(base_indent + hanging_indent), Some(-hanging_indent)) } else { (Some(base_indent), Some(0.0)) };
                style.para.left_indent = li;
                style.para.first_line_indent = fi;
                style.para.tabs = Some(vec![tab_stop.clone()]);
            }

            if !found {
                let (left_ind, first_ind) = if hanging { (Some(hanging_indent), Some(-hanging_indent)) } else { (Some(0.0), Some(0.0)) };
                let st = d.styles_mut();
                st.paragraph.push(designcraft_doc::ParagraphStyle {
                    name: level_one.clone(),
                    based_on: Some(designcraft_doc::story::BASIC_PARAGRAPH.into()),
                    next_style: None,
                    para: designcraft_doc::ParaAttrs {
                        left_indent: left_ind,
                        first_line_indent: first_ind,
                        space_before: Some(4.0),
                        align: Some(designcraft_doc::Align::Left),
                        tabs: Some(vec![tab_stop]),
                        ..Default::default()
                    },
                    chars: designcraft_doc::CharAttrs::default(),
                    shortcut: String::new(),
                });
            }
        }
        Ok(serde_json::Value::Null)
    });
}

pub(crate) fn compute_detected_prefix_width(st: &PubgenState) -> f64 {
    if st.toc_layout_style == 1 {
        // TablePage: measure width of number column only (e.g. "4.2.1", "04.01", "4.10.12")
        let max_w = st
            .card1_records
            .iter()
            .filter_map(|row| {
                let r_num = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let clean_num = r_num.trim_end_matches('.');
                if clean_num.is_empty() { None } else { Some(estimate_text_advance(&format!("{clean_num}  "))) }
            })
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(36.0);
        max_w.clamp(28.0, 54.0)
    } else {
        st.card1_records
            .iter()
            .filter_map(|row| {
                let r_type = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let r_num = row.get(st.card1_num_col.min(2)).map(String::as_str).unwrap_or("").trim();
                let prefix = [r_type, r_num].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
                if prefix.is_empty() { None } else { Some(estimate_text_advance(&format!("{prefix} "))) }
            })
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(68.0)
    }
}

/// The entry that lists the table of contents itself ("Daftar Isi .... 5").
///
/// The flat lists get it from `enrich_toc_story`, which rewrites their text; a paragraph list
/// keeps the engine's own level styles, so its entry is added as a paragraph of its own.
pub(crate) fn add_toc_self_entry(app: &mut DesignApp, sid: designcraft_doc::StoryId, plan: &TocPlan, toc_page: usize) {
    if !plan.include_self {
        return;
    }
    let title = plan.title_or_contents().to_string();
    let _ = app.session.edit(|doc, _| {
        let page_label = doc.page_name(toc_page);
        let Some(story) = doc.story_mut(sid) else {
            return Ok(serde_json::Value::Null);
        };
        // Already listed (a rebuild starts from the engine's fresh story, but a second call on the
        // same story must not add the entry twice). The title line is not an entry.
        let ranges = story.para_ranges();
        let listed =
            ranges.iter().skip(1).any(|r| story.text[r.clone()].split('\t').next().unwrap_or_default().trim().eq_ignore_ascii_case(title.trim()));
        if listed {
            return Ok(serde_json::Value::Null);
        }
        // Where it belongs in page order, like the entries around it: before the first entry whose
        // page number is higher. A label that isn't plain digits (roman front matter) or a table
        // without numbered entries keeps it at the top, right after the title.
        let page_no = page_label.trim().parse::<u64>().ok();
        let mut at = ranges.last().map_or_else(|| story.len(), |r| r.end);
        for r in ranges.iter().skip(1) {
            let label = story.text[r.clone()].rsplit('\t').next().unwrap_or_default().trim().to_string();
            let higher = match (page_no, label.parse::<u64>().ok()) {
                (Some(page), Some(other)) => other > page,
                (None, Some(_)) => true,
                _ => false,
            };
            if higher {
                at = r.start;
                break;
            }
        }
        let entry = if at >= story.len() { format!("\n{title}\t{page_label}") } else { format!("{title}\t{page_label}\n") };
        let start = if at >= story.len() { at + 1 } else { at };
        story.insert(at, &entry);
        story.format_paras(start..start, |f| *f = designcraft_doc::ParaFormat { style: "TOC Level 1".to_string(), ..Default::default() });
        Ok(serde_json::Value::Null)
    });
}

/// Wrap a paragraph TOC title line so that continuation lines do not crowd the right margin
/// and page numbers, and maintain ample dot leader space before the page number.
pub(crate) fn wrap_paragraph_title(clean_title: &str, max_line_w: f64) -> String {
    let raw_words: Vec<&str> = clean_title.split_whitespace().collect();
    if raw_words.len() <= 1 {
        return clean_title.to_string();
    }

    // Attach any standalone "/" to the preceding word so that lines never begin with a slash
    let mut words: Vec<String> = Vec::new();
    for w in raw_words {
        if w == "/" && !words.is_empty() {
            if let Some(prev) = words.last_mut() {
                prev.push_str(" /");
            }
        } else {
            words.push(w.to_string());
        }
    }

    let mut lines: Vec<String> = Vec::new();
    let mut cur_line = String::new();

    for word in words {
        let candidate = if cur_line.is_empty() { word.clone() } else { format!("{cur_line} {word}") };

        if estimate_text_advance(&candidate) <= max_line_w || cur_line.is_empty() {
            cur_line = candidate;
        } else {
            lines.push(cur_line);
            cur_line = word;
        }
    }

    if !cur_line.is_empty() {
        lines.push(cur_line);
    }

    lines.join(&designcraft_doc::story::FORCED_LINE_BREAK.to_string())
}

/// Early wrapping for paragraph TOC entries so that long headings wrap cleanly before the
/// page number column, while maintaining character styling (such as italic for English halves
/// in bilingual paired headings).
pub(crate) fn enrich_paragraph_toc_story(app: &mut DesignApp, sid: designcraft_doc::StoryId, width: f64) {
    let _ = app.session.edit(|doc, _| {
        let story = doc.story_mut(sid).ok_or(designcraft_doc::DocError::NoStory(sid))?;
        let ranges = story.para_ranges();
        if ranges.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        let mut new_lines = Vec::new();
        let mut para_styles = Vec::new();
        let mut en_spans: Vec<(std::ops::Range<usize>, String)> = Vec::new();
        let mut current_offset = 0;

        for (pi, r) in ranges.into_iter().enumerate() {
            let para_style = story.paras.get(pi).map(|p| p.style.clone()).unwrap_or_else(|| "TOC Level 1".into());
            let para_text = story.text[r].trim_end_matches('\r').to_string();

            if pi == 0 || !para_text.contains('\t') {
                let line_len = para_text.len();
                new_lines.push(para_text);
                para_styles.push(para_style);
                current_offset += line_len + 1;
                continue;
            }

            let (title, page_part) = match para_text.split_once('\t') {
                Some((t, p)) => (t, p),
                None => {
                    let line_len = para_text.len();
                    new_lines.push(para_text);
                    para_styles.push(para_style);
                    current_offset += line_len + 1;
                    continue;
                }
            };

            let clean_title = title.replace(designcraft_doc::story::FORCED_LINE_BREAK, " ");
            let clean_title = clean_title.trim();

            let level = para_style.strip_prefix("TOC Level ").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
            let base_indent = (level.saturating_sub(1) as f64) * 12.0;

            let page_trim = page_part.trim();
            let page_w = estimate_text_advance(page_trim);
            let min_leader_gap = 48.0;
            let right_reserve = page_w + min_leader_gap;
            let max_line_w = (width - base_indent - right_reserve).max(80.0);

            let wrapped_title = if clean_title.is_empty() {
                String::new()
            } else if estimate_text_advance(clean_title) <= max_line_w {
                clean_title.to_string()
            } else {
                wrap_paragraph_title(clean_title, max_line_w)
            };

            let new_line = format!("{wrapped_title}\t{page_part}");
            let line_len = new_line.len();

            // Detect English half in bilingual paired heading ("Indonesian / English")
            if let Some(slash_idx) = wrapped_title.find(" / ") {
                let en_start = current_offset + slash_idx + 3;
                let en_end = current_offset + wrapped_title.len();
                if en_start < en_end {
                    en_spans.push((en_start..en_end, "TOC English".to_string()));
                }
            } else if let Some(slash_idx) = wrapped_title.find(" /") {
                let after_slash = slash_idx + 2;
                let mut en_start_rel = after_slash;
                for c in wrapped_title[after_slash..].chars() {
                    if c.is_whitespace() || c == designcraft_doc::story::FORCED_LINE_BREAK {
                        en_start_rel += c.len_utf8();
                    } else {
                        break;
                    }
                }
                let en_start = current_offset + en_start_rel;
                let en_end = current_offset + wrapped_title.len();
                if en_start < en_end {
                    en_spans.push((en_start..en_end, "TOC English".to_string()));
                }
            }

            new_lines.push(new_line);
            para_styles.push(para_style);
            current_offset += line_len + 1;
        }

        let full_text = new_lines.join("\n");
        let len = story.len();
        story.replace(0..len, &full_text);

        story.format_chars(0..story.len(), |f| f.style = designcraft_doc::NO_CHAR_STYLE.into());
        for (span, style) in en_spans {
            story.format_chars(span, |f| f.style = style.clone());
        }

        for (pi, r) in story.para_ranges().into_iter().enumerate() {
            if let Some(st_name) = para_styles.get(pi) {
                story.format_paras(r.start..r.start, |f| {
                    *f = designcraft_doc::ParaFormat { style: st_name.clone(), ..Default::default() };
                });
            }
        }

        Ok(serde_json::Value::Null)
    });
}

pub(crate) fn apply_toc_post_process(
    app: &mut DesignApp,
    st: &PubgenState,
    plan: &TocPlan,
    sid: designcraft_doc::StoryId,
    width: f64,
    hanging_indent: f64,
    target_page: usize,
    mut pages_added: u64,
) -> u64 {
    enrich_toc_story(app, sid, st, plan, width, hanging_indent, target_page);

    // A paragraph list keeps the engine's own level styles, so the entry that lists the table of
    // contents itself is added here: `enrich_toc_story` rewrites the flat lists and skips them.
    if plan.is_paragraph_toc() {
        add_toc_self_entry(app, sid, plan, target_page);
        enrich_paragraph_toc_story(app, sid, width);
    }

    let entry_style = format!("{} 1", plan.entry_prefix());
    let _ = app.session.edit(|doc, _| {
        if let Some(story) = doc.story_mut(sid) {
            let ranges = story.para_ranges();
            for (pi, r) in ranges.into_iter().enumerate() {
                let para_text = story.text[r.clone()].trim();
                let style_name: String = if plan.layout == 1 {
                    if pi == 0
                        && !plan.title.trim().is_empty()
                        && (para_text == plan.title.trim()
                            || para_text.eq_ignore_ascii_case("contents")
                            || para_text.eq_ignore_ascii_case("daftar isi")
                            || para_text.eq_ignore_ascii_case("daftar tabel")
                            || para_text.eq_ignore_ascii_case("daftar grafik")
                            || para_text.eq_ignore_ascii_case("daftar chart")
                            || para_text.eq_ignore_ascii_case("daftar tabel / list of tables")
                            || para_text.eq_ignore_ascii_case("daftar grafik / list of charts")
                            || para_text.eq_ignore_ascii_case("daftar isi / table of contents"))
                    {
                        "TOC Title".to_string()
                    } else if is_toc_header_line(para_text) {
                        "TOC TablePage Header".to_string()
                    } else if para_text.split('\t').count() == 2 {
                        "TOC Section Header".to_string()
                    } else {
                        entry_style.clone()
                    }
                } else if pi == 0 {
                    "TOC Title".to_string()
                } else {
                    entry_style.clone()
                };

                // A paragraph list keeps each entry's own "TOC Level n" from `toc.generate`;
                // flattening it to level 1 would lose the heading nesting.
                if plan.is_paragraph_toc() && pi > 0 {
                    continue;
                }

                // The Tables and Charts lists center their title line.
                let center = pi == 0 && style_name == "TOC Title" && plan.kind() != 0;
                story.format_paras(r.start..r.start, |f| {
                    *f = designcraft_doc::ParaFormat { style: style_name.clone(), ..Default::default() };
                    if center {
                        f.para.align = Some(designcraft_doc::Align::Center);
                    }
                });

                if style_name == "TOC TablePage Header"
                    && let Some(pos) = story.text[r.clone()].find(designcraft_doc::story::FORCED_LINE_BREAK)
                {
                    let second_line_start = r.start + pos + designcraft_doc::story::FORCED_LINE_BREAK.len_utf8();
                    let second_line_end = r.end;
                    if second_line_start < second_line_end {
                        story.format_chars(second_line_start..second_line_end, |f| {
                            f.style = "TOC English".to_string();
                            f.over.font_style = Some("Italic".to_string());
                        });
                    }
                }
            }
        }
        Ok(serde_json::Value::Null)
    });

    ensure_toc_entry_style(app, width, plan.hanging, hanging_indent, plan.layout, plan.entry_prefix());

    if let Ok(af_res) = app.run("text.autoflow", json!({"story": sid.0, "maxPages": 50}))
        && let Some(extra) = af_res.get("pagesAdded").and_then(Value::as_u64)
    {
        pages_added += extra;
    }

    pages_added = sync_toc_continuation_headers(app, st, sid, plan, pages_added);

    pages_added
}

pub const TOC_CONTINUATION_HEADER_LABEL: &str = "pubgen_toc_continuation_header";
pub(crate) const TOC_CONT_HEADER_H: f64 = 18.0;
pub(crate) const TOC_CONT_HEADER_GAP: f64 = 6.0;

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn remove_toc_continuation_headers(app: &mut DesignApp) {
    let _ = app.session.edit(|doc, _| {
        let mut to_remove = Vec::new();
        for spread in &doc.spreads {
            for it in &spread.items {
                if it.label == TOC_CONTINUATION_HEADER_LABEL {
                    to_remove.push(it.id);
                }
            }
        }
        for id in to_remove {
            let _ = doc.remove_item(id);
        }
        Ok(serde_json::Value::Null)
    });
}

/// Remove the continuation-page headers of the table of contents `sid` — the ones on the pages its
/// continuation frames sit on — plus any orphan (a header on a page no table of contents occupies),
/// leaving the other tables of contents alone.
pub(crate) fn remove_toc_continuation_headers_of(app: &mut DesignApp, sid: designcraft_doc::StoryId) {
    let _ = app.session.edit(|doc, _| {
        let pages_of = |story: designcraft_doc::StoryId| -> Vec<usize> {
            doc.story(story).map(|s| s.frames.iter().skip(1).filter_map(|f| doc.page_of_item(*f)).collect()).unwrap_or_default()
        };
        let mine = pages_of(sid);
        let others: Vec<usize> = doc.tocs.iter().filter(|t| t.story != sid).flat_map(|t| pages_of(t.story)).collect();
        let mut to_remove = Vec::new();
        for spread in &doc.spreads {
            for it in &spread.items {
                if it.label != TOC_CONTINUATION_HEADER_LABEL {
                    continue;
                }
                if let Some(pg) = doc.page_of_item(it.id)
                    && (mine.contains(&pg) || !others.contains(&pg))
                {
                    to_remove.push(it.id);
                }
            }
        }
        for id in to_remove {
            let _ = doc.remove_item(id);
        }
        Ok(serde_json::Value::Null)
    });
}

pub(crate) fn sync_toc_continuation_headers(
    app: &mut DesignApp,
    st: &PubgenState,
    sid: designcraft_doc::StoryId,
    plan: &TocPlan,
    mut pages_added: u64,
) -> u64 {
    remove_toc_continuation_headers_of(app, sid);

    let (header_text, style_name) = if plan.layout == 1 {
        let detected = app
            .session
            .active()
            .and_then(|a| a.doc.story(sid).and_then(|s| s.text.lines().find(|l| is_toc_header_line(l.trim())).map(|l| l.trim().to_string())));

        let text = detected.unwrap_or_else(|| toc_header_line(plan, st));

        (text, "TOC TablePage Header")
    } else {
        let title = plan.title_or_contents();
        let is_id = title.to_lowercase().contains("daftar") || title.to_lowercase().contains("tabel") || title.to_lowercase().contains("grafik");
        let cont_suffix = if is_id { "(Lanjutan)" } else { "(Continued)" };
        (format!("{title} {cont_suffix}"), "TOC Title")
    };

    for _iter in 0..5 {
        let active = match app.session.active() {
            Some(a) => a,
            None => break,
        };

        let frames = match active.doc.story(sid) {
            Some(s) if s.frames.len() > 1 => s.frames.clone(),
            _ => break,
        };

        let first_pg = frames.first().and_then(|f| active.doc.page_of_item(*f));

        let cs = designcraft_compose::compose_story(&active.doc, sid, &Default::default());

        struct FramePlan {
            fid: designcraft_doc::ItemId,
            spread_ref: designcraft_doc::SpreadRef,
            layer: designcraft_doc::LayerId,
            header_rect: designcraft_geom::Rect,
            flow_rect: designcraft_geom::Rect,
            needs_header: bool,
            needs_resize: bool,
        }

        let mut plans = Vec::new();

        for &fid in &frames[1..] {
            let has_text = cs.frame(fid).is_some_and(|ft| !ft.lines.is_empty());
            if !has_text && cs.overset_at.is_none() {
                continue;
            }

            let Some(pg) = active.doc.page_of_item(fid) else { continue };
            if Some(pg) == first_pg {
                continue;
            }

            let Some((si, pi)) = active.doc.page_loc(pg) else { continue };
            let Some(spread) = active.doc.spreads.get(si) else { continue };
            let Some(page) = spread.pages.get(pi) else { continue };
            let m = page.margin_rect();

            let cont_h = if header_text.contains(designcraft_doc::story::FORCED_LINE_BREAK) { 28.0 } else { TOC_CONT_HEADER_H };
            let header_rect = designcraft_geom::Rect::new(m.x0, m.y0, m.x1, m.y0 + cont_h);
            let flow_top = (m.y0 + cont_h + TOC_CONT_HEADER_GAP).min(m.y1 - 10.0);
            let flow_rect = designcraft_geom::Rect::new(m.x0, flow_top, m.x1, m.y1);

            let layer = active.doc.item(fid).map(|it| it.layer).unwrap_or_else(|| active.doc.default_layer());

            let has_hdr = spread.items.iter().any(|it| it.label == TOC_CONTINUATION_HEADER_LABEL && active.doc.page_of_item(it.id) == Some(pg));

            let current_b = active.doc.item(fid).map(|it| it.bounds()).unwrap_or(designcraft_geom::Rect::ZERO);
            let needs_resize = (current_b.y0 - flow_top).abs() > 1.0;

            if !has_hdr || needs_resize {
                plans.push(FramePlan {
                    fid,
                    spread_ref: designcraft_doc::SpreadRef::Doc(si),
                    layer,
                    header_rect,
                    flow_rect,
                    needs_header: !has_hdr,
                    needs_resize,
                });
            }
        }

        if plans.is_empty() {
            break;
        }

        let mut resized_any = false;
        let _ = app.session.edit(|doc, _| {
            for p in &plans {
                if p.needs_header {
                    let para = designcraft_doc::ParaFormat { style: style_name.to_string(), ..Default::default() };
                    if let Ok((hdr_id, hdr_sid)) = doc.add_text_frame(p.spread_ref, p.header_rect, p.layer, &header_text, para) {
                        if let Some(it) = doc.item_mut(hdr_id) {
                            it.label = TOC_CONTINUATION_HEADER_LABEL.to_string();
                            it.name = "TOC Continuation Header".to_string();
                        }
                        if let Some(hdr_story) = doc.story_mut(hdr_sid)
                            && let Some(pos) = hdr_story.text.find(designcraft_doc::story::FORCED_LINE_BREAK)
                        {
                            let start = pos + designcraft_doc::story::FORCED_LINE_BREAK.len_utf8();
                            let end = hdr_story.len();
                            if start < end {
                                hdr_story.format_chars(start..end, |f| {
                                    f.style = "TOC English".to_string();
                                    f.over.font_style = Some("Italic".to_string());
                                });
                            }
                        }
                    }
                }
                if p.needs_resize
                    && let Some(it) = doc.item_mut(p.fid)
                {
                    it.path = designcraft_geom::shapes::rectangle(p.flow_rect);
                    resized_any = true;
                }
            }
            Ok(serde_json::Value::Null)
        });

        if resized_any {
            if let Ok(af_res) = app.run("text.autoflow", json!({"story": sid.0, "maxPages": 50}))
                && let Some(extra) = af_res.get("pagesAdded").and_then(Value::as_u64)
            {
                pages_added += extra;
            }
        } else {
            break;
        }
    }

    pages_added
}

/// The story of the table of contents whose first frame is on page `page`, if any.
pub(crate) fn toc_story_on_page(app: &DesignApp, page: usize) -> Option<designcraft_doc::StoryId> {
    app.session.active()?.doc.toc_on_page(page).map(|t| t.story)
}

/// The page a table of contents' first frame sits on.
pub(crate) fn toc_story_page(app: &DesignApp, sid: designcraft_doc::StoryId) -> Option<usize> {
    let active = app.session.active()?;
    let story = active.doc.story(sid)?;
    story.frames.first().and_then(|f| active.doc.page_of_item(*f))
}

/// Text width of a page's margins (the TOC's column).
pub(crate) fn page_margin_width(app: &DesignApp, page: usize) -> Result<f64, String> {
    let active = app.session.active().ok_or("No active document")?;
    let (si, pi) = active.doc.page_loc(page).ok_or("Page location not found")?;
    let spread = active.doc.spreads.get(si).ok_or("Spread missing")?;
    let p = spread.pages.get(pi).ok_or("Page missing")?;
    let m = p.margin_rect();
    Ok((m.x1 - m.x0).max(100.0))
}

/// Hanging indent of wrapped TOC entries for a layout.
pub(crate) fn toc_hanging_indent(st: &PubgenState, layout: usize) -> f64 {
    let detected = compute_detected_prefix_width(st);
    if layout == 1 { detected.round().clamp(28.0, 60.0) } else { detected.round().clamp(48.0, 140.0) }
}

/// Generate (or regenerate) one table of contents on `page` and run the panel's post-process
/// (type/number merging, title styles, continuation pages and headers).
/// Whether any paragraph of the document carries `style` (the panel's own marker styles are only
/// present once a page has been generated with them).
pub(crate) fn document_uses_style(app: &DesignApp, style: &str) -> bool {
    app.session.active().is_some_and(|a| a.doc.stories.values().any(|s| s.paras.iter().any(|p| p.style == style)))
}

pub(crate) fn generate_toc(app: &mut DesignApp, st: &PubgenState, page: usize, plan: &TocPlan, replace: bool) -> Result<(u64, u64), String> {
    let width = page_margin_width(app, page)?;
    let hanging_indent = toc_hanging_indent(st, plan.layout);

    // A document whose pages were made before the narrative style existed still lists its
    // chapters, rather than coming up empty until its pages are generated again.
    let legacy = plan.clone();
    let plan = if plan.is_paragraph_toc() && !document_uses_style(app, TOC_NARRATIVE_STYLE) { legacy.with_legacy_chapters(st) } else { legacy };
    let plan = &plan;

    for level in &plan.levels {
        ensure_para_style(app, &level.style);
    }
    ensure_toc_entry_style(app, width, plan.hanging, hanging_indent, plan.layout, plan.entry_prefix());

    let res = app.run(
        "toc.generate",
        json!({
            "page": page + 1,
            "title": plan.title_or_contents(),
            "entries": plan.entries_json(),
            "replace": replace,
            "autoflow": true
        }),
    )?;

    let count = res.get("entries").and_then(Value::as_u64).unwrap_or(0);
    let pages_added = res.get("pagesAdded").and_then(Value::as_u64).unwrap_or(0);
    let sid_u64 = res.get("story").and_then(Value::as_u64).ok_or("TOC story missing")?;
    let sid = designcraft_doc::StoryId(sid_u64);

    let final_pages_added = apply_toc_post_process(app, st, plan, sid, width, hanging_indent, page, pages_added);
    Ok((count, final_pages_added))
}

pub(crate) fn execute_add_toc(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    if app.session.active().is_none() {
        return Err("No active document".to_string());
    }
    let selected_page =
        app.session.active().and_then(|s| s.selection.page).or_else(|| crate::canvas::current_page(app)).unwrap_or_else(|| target_page_index(app));
    let selected_label = app.session.page_label(selected_page);

    let (target_page, replace) = if st.toc_insert_after {
        let _ = app.run("layout.pages.insert", json!({"after": selected_page, "count": 1}))?;
        let new_page = selected_page + 1;
        if let Some(active) = app.session.active_mut() {
            active.selection.page = Some(new_page);
        }
        crate::canvas::go_to_page(app, new_page);
        (new_page, false)
    } else {
        (selected_page, toc_story_on_page(app, selected_page).is_some())
    };

    let target_label = app.session.page_label(target_page);
    let plan = TocPlan::from_state(st);
    let (count, final_pages_added) = generate_toc(app, st, target_page, &plan, replace)?;

    let cont_msg = if final_pages_added > 0 { format!(" across {} pages", final_pages_added + 1) } else { String::new() };

    let kind = TOC_KINDS.get(plan.kind()).copied().unwrap_or(TOC_KINDS[0]);
    if st.toc_insert_after {
        Ok(format!("Inserted blank page and added {kind} on Page {target_label} after Page {selected_label} ({count} entries{cont_msg})"))
    } else {
        Ok(format!("Added {kind} on Page {target_label} ({count} entries{cont_msg})"))
    }
}

/// Rebuild the table of contents on the page in view, keeping the TOC style, title and filter it
/// was created with. With none on this page it is added, as when the document had none.
pub(crate) fn execute_rebuild_toc(app: &mut DesignApp, st: &mut PubgenState) -> Result<String, String> {
    let cur_page = target_page_index(app);
    let Some(sid) = toc_story_on_page(app, cur_page) else {
        return execute_add_toc(app, st);
    };

    let plan = {
        let active = app.session.active().ok_or("No active document")?;
        let toc = active.doc.toc_of_story(sid).ok_or("No table of contents exists")?;
        TocPlan::from_toc(st, toc)
    };

    let toc_label = app.session.page_label(cur_page);
    let (count, final_pages_added) = generate_toc(app, st, cur_page, &plan, true)?;

    let cont_msg = if final_pages_added > 0 { format!(" across {} pages", final_pages_added + 1) } else { String::new() };

    Ok(format!("Rebuilt TOC on Page {toc_label} ({count} entries{cont_msg})"))
}

/// Rebuild every table of contents in the document (after renumbering), each with its own TOC
/// style, title and filter.
pub(crate) fn rebuild_all_tocs(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    let story_ids: Vec<designcraft_doc::StoryId> = {
        let active = app.session.active().ok_or("No active document")?;
        active.doc.tocs.iter().map(|t| t.story).collect()
    };
    if story_ids.is_empty() {
        return Ok(String::new());
    }

    let mut rebuilt = 0usize;
    let mut entries = 0u64;
    for sid in story_ids {
        // Look the page up again each time: regenerating one table can add or remove pages.
        let Some(page) = toc_story_page(app, sid) else { continue };
        let plan = {
            let active = match app.session.active() {
                Some(a) => a,
                None => break,
            };
            match active.doc.toc_of_story(sid) {
                Some(toc) => TocPlan::from_toc(st, toc),
                None => continue,
            }
        };
        let (n, _) = generate_toc(app, st, page, &plan, true)?;
        entries += n;
        rebuilt += 1;
    }

    if rebuilt == 0 {
        return Ok(String::new());
    }
    Ok(format!("{rebuilt} table(s) of contents rebuilt ({entries} entries)"))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::testutil::*;
    use super::*;

    #[test]
    fn test_toc_line_preset_writes_one_styled_paragraph_per_record() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Grafik".into(), "3.1.".into(), "Piramida Penduduk".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 5,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        let res = execute_generate_pages(&mut app, &st);
        assert!(res.is_ok(), "the TOC Line preset should succeed: {res:?}");

        // One combined paragraph per record, not three separate frames.
        let active = app.session.active().unwrap();
        let mut lines: Vec<&str> =
            active.doc.stories.values().filter(|s| s.paras.first().is_some_and(|p| p.style == "TOC Entry")).map(|s| s.text.trim()).collect();
        lines.sort_unstable();
        assert_eq!(lines, ["Grafik 3.1. Piramida Penduduk", "Tabel 01.01. Jarak Antar Kecamatan"], "one styled paragraph per record");
    }

    #[test]
    fn test_add_toc_on_selected_page_collects_toc_line_entries() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Tabel".into(), "01.02.".into(), "Curah Hujan".into()],
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 5,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        // The same calls the Quick Actions "Add TOC (selected page)" button makes.
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        ensure_para_style(&mut app, &st.toc_style);
        let page = app.session.active().unwrap().doc.page_count();
        let res = app.run(
            "toc.generate",
            json!({"page": page, "title": "Contents",
                   "entries": [{"style": "TOC Entry", "level": 1, "pageNumber": "after"}],
                   "replace": true}),
        );
        assert!(res.is_ok(), "toc.generate should succeed: {res:?}");

        let sid = designcraft_doc::StoryId(res.unwrap()["story"].as_u64().unwrap());
        let text = app.session.active().unwrap().doc.story(sid).unwrap().text.clone();
        assert!(text.contains("Contents"), "{text}");
        assert!(text.contains("Jarak Antar Kecamatan"), "one line per record: {text}");
        assert!(text.contains("Curah Hujan"), "{text}");
    }

    #[test]
    fn test_default_preset_title_is_tagged_and_listed_in_toc() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        // The default preset still writes three frames, but only the title carries the TOC style.
        assert_eq!(toc_entry_count(&mut app, "TOC Entry"), 1, "the title paragraph is a TOC entry");

        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let page = app.session.active().unwrap().doc.page_count();
        let res = app.run(
            "toc.generate",
            json!({"page": page, "title": "Contents",
                   "entries": [{"style": "TOC Entry", "level": 1, "pageNumber": "after"}],
                   "replace": true}),
        );
        assert!(res.is_ok(), "toc.generate should succeed: {res:?}");

        let sid = designcraft_doc::StoryId(res.unwrap()["story"].as_u64().unwrap());
        let text = app.session.active().unwrap().doc.story(sid).unwrap().text.clone();
        assert!(text.contains("Jarak Antar Kecamatan"), "the generated page title is listed: {text}");
    }

    #[test]
    fn test_execute_add_toc_kbda_row_merges_type_number_title() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Row #2 from KBDA2026_chapters.csv:
        // Tabel;01.01.;Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006
        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into()],
                vec!["Tabel".into(), "01.02.".into(), "Keadaan Curah Hujan Tiap Bulan menurut Kecamatan (mm), 2006".into()],
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            card1_toc_prefix: false, // Even when false in page generation, execute_add_toc merges them!
            toc_style: "TOC Entry".into(),
            toc_hanging_indent: true,
            ..Default::default()
        };

        // Generate pages
        execute_generate_pages(&mut app, &st).unwrap();

        // Now insert TOC page and run execute_add_toc
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("Document TOC should be populated");
        let story = active.doc.story(toc.story).expect("TOC story should exist");
        let text = &story.text;

        assert!(
            text.contains("Tabel 01.01. Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"),
            "TOC should contain merged type + number + title: {text}"
        );
        assert!(
            text.contains("Tabel 01.02. Keadaan Curah Hujan Tiap Bulan menurut Kecamatan (mm), 2006"),
            "TOC should contain second merged record: {text}"
        );
    }

    #[test]
    fn test_execute_add_toc_hanging_indent_applied() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            card1_toc_prefix: false,
            toc_style: "TOC Entry".into(),
            toc_hanging_indent: true,
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        execute_add_toc(&mut app, &mut st).unwrap();

        let active = app.session.active().unwrap();
        let style = active.doc.styles.para("TOC Level 1").expect("TOC Level 1 style should exist");
        assert_eq!(style.para.left_indent, Some(78.0), "Hanging indent left_indent");
        assert_eq!(style.para.first_line_indent, Some(-78.0), "Hanging indent first_line_indent");

        let tab = style.para.tabs.as_ref().and_then(|t| t.first()).expect("Tab stop should exist");
        assert_eq!(tab.align, designcraft_doc::TabAlign::Right);
        assert_eq!(tab.leader, ". ");
    }

    #[test]
    fn test_execute_add_toc_fallback_from_page_frames() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            card1_toc_prefix: false,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();

        // Now simulate empty card1_records (e.g. session without CSV in memory)
        let mut st_empty = PubgenState { card1_records: Vec::new(), toc_style: "TOC Entry".into(), ..Default::default() };

        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let res = execute_add_toc(&mut app, &mut st_empty);
        assert!(res.is_ok(), "execute_add_toc with fallback should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = &active.doc.story(toc.story).unwrap().text;

        assert!(
            text.contains("Tabel 01.01 Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"),
            "Fallback should extract type and number from page frames: {text}"
        );
    }

    #[test]
    fn test_execute_add_toc_ignores_footnote_and_source_text_frames() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            card1_toc_prefix: true, // Page generated with prefix
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();

        // Simulate Table from CSV creating a footnote text frame on that page
        let spread_idx = 0;
        let _ = app.run(
            "frame.create",
            json!({
                "spread": spread_idx,
                "rect": [72.0, 500.0, 400.0, 530.0],
                "content": "text",
                "text": "Sources: Karawang Region Government Office",
                "caret": false
            }),
        );

        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = &active.doc.story(toc.story).unwrap().text;

        // Must NOT contain footnote text
        assert!(!text.to_lowercase().contains("sources"), "TOC must not incorporate footnote/source text: {text}");
        assert!(text.contains("Tabel 01.01. Jarak Antar Kecamatan Di Kabupaten Karawang (km), 2006"), "TOC must contain clean entry: {text}");
    }

    #[test]
    fn test_execute_add_toc_generates_continuation_pages_when_overflows() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 80 entries - definitely cannot fit in 1 page (typical page holds ~35 entries)
        let records: Vec<Vec<String>> =
            (1..=80).map(|i| vec!["Tabel".into(), format!("01.{i:02}."), format!("Dataset Observation Record Number {i} in Region")]).collect();

        let mut st = PubgenState {
            card1_records: records,
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 5, // TOC Line preset
            toc_style: "TOC Entry".into(),
            toc_hanging_indent: true,
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();

        let initial_pages = app.session.active().unwrap().doc.page_count();
        // Insert TOC starting page
        app.run("layout.pages.insert", json!({"after": 0, "count": 1})).unwrap();
        crate::canvas::go_to_page(&mut app, 1);

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story_id = toc.story;

        // Verify that the TOC story has threaded across multiple frames/pages
        let frames_count = active.doc.story(story_id).map(|s| s.frames.len()).unwrap_or(0);
        assert!(frames_count > 1, "TOC should span more than 1 frame: {frames_count}");

        let current_pages = active.doc.page_count();
        assert!(
            current_pages > initial_pages + 1,
            "Continuation pages should be added to hold overflowed TOC (initial: {initial_pages}, current: {current_pages})"
        );

        // Verify the story is NOT overset
        let cs = designcraft_compose::compose_story(&active.doc, story_id, &Default::default());
        assert!(cs.overset_at.is_none(), "TOC should not be overset after autoflow");
    }

    #[test]
    fn test_wrap_toc_title_wraps_early_before_page_number() {
        let width = 440.0;
        let hanging = 68.0;

        // 1. Short title fits comfortably on line 0
        let short = "Tabel 01.01. Short Title, 2006";
        let wrapped_short = wrap_toc_title(short, width, hanging, Some("12"));
        assert_eq!(wrapped_short, short);

        // 2. Long title (Grafik 4.6. from user request) wraps early with \u{2028}
        let long = "Grafik 4.6. Banyaknya Tenaga Medis/Dokter Menurut Keahlian di Kabupaten Karawang, 2006";
        let wrapped_long = wrap_toc_title(long, width, hanging, Some("44"));
        assert!(wrapped_long.contains(designcraft_doc::story::FORCED_LINE_BREAK), "Title should be wrapped with FORCED_LINE_BREAK");
        let parts: Vec<&str> = wrapped_long.split(designcraft_doc::story::FORCED_LINE_BREAK).collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].starts_with("Grafik 4.6. Banyaknya"));
        // First line should wrap early (less than width - 48 - page_w)
        assert!(estimate_text_advance(parts[0]) <= width - 48.0 - 15.0);
        // Second line contains the remaining words
        assert!(parts[1].ends_with("2006"));
    }

    #[test]
    fn test_execute_add_toc_warps_early_and_indents_second_line() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let records = vec![
            vec!["Grafik".into(), "4.6.".into(), "Banyaknya Tenaga Medis/Dokter Menurut Keahlian di Kabupaten Karawang, 2006".into()],
            vec![
                "Tabel".into(),
                "04.01.".into(),
                "Banyaknya Sekolah, Murid dan Guru Taman Kanak-Kanak Menurut Status Sekolah Tiap Kecamatan, 2006".into(),
            ],
        ];

        let mut st = PubgenState {
            card1_records: records,
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            toc_hanging_indent: true,
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        crate::canvas::go_to_page(&mut app, 1);

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story_id = toc.story;
        let story = active.doc.story(story_id).unwrap();

        // Both long titles should have wrapped lines via \u{2028}
        assert!(
            story.text.contains(designcraft_doc::story::FORCED_LINE_BREAK),
            "Story text should contain soft line breaks for early wrapping: {}",
            story.text
        );

        // Compose the TOC story to verify layout geometry
        let cs = designcraft_compose::compose_story(&active.doc, story_id, &Default::default());
        assert!(!cs.frames.is_empty(), "Frames should exist");
        let lines = &cs.frames[0].lines;
        assert!(lines.len() >= 4, "Should compose into multiple lines: {}", lines.len());

        // Check style indentation
        let style = active.doc.styles.para("TOC Level 1").unwrap();
        let left_ind = style.para.left_indent.unwrap();
        let first_ind = style.para.first_line_indent.unwrap();
        assert!(left_ind >= 68.0, "Second line indent should be deep (closer to first char of title): {left_ind}");
        assert_eq!(first_ind, -left_ind, "First line indent must balance left indent");
    }

    #[test]
    fn test_execute_rebuild_toc_updates_entries_and_page_numbers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Tabel".into(), "01.02.".into(), "Keadaan Curah Hujan".into()],
            ],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };

        // Page 1 is blank. Generate pages: Page 2 is 01.01, Page 3 is 01.02.
        execute_generate_pages(&mut app, &st).unwrap();

        // Put TOC on Page 1
        crate::canvas::go_to_page(&mut app, 0);
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().unwrap();
            let text = active.doc.story(toc.story).unwrap().text.clone();
            assert!(text.contains("Jarak Antar Kecamatan\t2") || text.contains("2"), "TOC text: {text}");
        }

        // Now insert a blank page after Page 1 (between TOC and Chapter 1), shifting Chapter 1 to Page 3 -> 4!
        app.run("layout.pages.insert", json!({"after": 0, "count": 1})).unwrap();

        // Rebuild TOC
        let res_rebuild = execute_rebuild_toc(&mut app, &mut st);
        assert!(res_rebuild.is_ok(), "execute_rebuild_toc should succeed: {res_rebuild:?}");

        {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().unwrap();
            let text = active.doc.story(toc.story).unwrap().text.clone();
            // Now Chapter 1 is on Page 3 and Chapter 2 is on Page 4!
            assert!(text.contains("\t3"), "Chapter 1 should be updated to page 3: {text}");
            assert!(text.contains("\t4"), "Chapter 2 should be updated to page 4: {text}");
        }
    }

    #[test]
    fn test_toc_include_self_option() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Bab".into(), "1".into(), "Pendahuluan".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Isi".into(),
            toc_include_self: true,
            ..Default::default()
        };

        // Generate chapter page (Page 2)
        execute_generate_pages(&mut app, &st).unwrap();

        // Generate TOC on Page 1 with toc_include_self = true
        crate::canvas::go_to_page(&mut app, 0);
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc with toc_include_self should succeed: {res:?}");

        let (toc_label, text) = {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().unwrap();
            let first_frame = active.doc.story(toc.story).unwrap().frames.first().copied().unwrap();
            let toc_pg = active.doc.page_of_item(first_frame).unwrap();
            let label = app.session.page_label(toc_pg);
            let text = active.doc.story(toc.story).unwrap().text.clone();
            (label, text)
        };
        // Should contain both the title and self entry:
        assert!(text.contains("Daftar Isi"), "Title should be Daftar Isi: {text}");
        assert!(text.contains(&format!("Daftar Isi\t{toc_label}")), "Self entry should be present: {text}");
        assert!(text.contains("Bab 1 Pendahuluan"), "Chapter entry should be present: {text}");

        // Now disable toc_include_self and rebuild TOC
        st.toc_include_self = false;
        let res_rebuild = execute_rebuild_toc(&mut app, &mut st);
        assert!(res_rebuild.is_ok(), "execute_rebuild_toc should succeed: {res_rebuild:?}");

        {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().unwrap();
            let text = active.doc.story(toc.story).unwrap().text.clone();
            assert!(!text.contains(&format!("Daftar Isi\t{toc_label}")), "Self entry should be removed when unchecked: {text}");
            assert!(text.contains("Bab 1 Pendahuluan"), "Chapter entry should remain: {text}");
        }
    }

    #[test]
    fn test_toc_paragraph_lists_a_generated_review_page_with_its_chapter_line() {
        // The whole way: an RXX row becomes a page with two text boxes (the chapter line and the
        // title), and the paragraph list names it "BAB 01 Geografi".
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Ulasan".into(), "01".into(), "BAB 01 Geografi".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 0,
            toc_kind: 3,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Isi".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        crate::canvas::go_to_page(&mut app, 0);
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains("BAB 01 Geografi"), "the review page reads chapter line and title: {text}");
    }

    #[test]
    fn test_toc_paragraph_pairs_bilingual_column_headings_on_a_dual_lang_page() {
        // A Dual Lang RXX page: the Indonesian RTF flows into the left column frame, the English
        // into the right one; both carry `Heading 1`. With the pairing checkbox on, the paragraph
        // list shows one row per heading pair.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        ensure_para_style(&mut app, "Heading 1");
        ensure_para_style(&mut app, TOC_NARRATIVE_STYLE);
        let (si, _) = app.session.active().unwrap().doc.page_loc(0).unwrap();
        for (x0, x1, text) in [(36.0, 300.0, "Proporsi fisik daratan"), (312.0, 576.0, "Physical proportions of the landmass")] {
            let res = app
                .run("frame.create", json!({"spread": si, "rect": [x0, 120.0, x1, 240.0], "content": "text", "text": text, "caret": false}))
                .expect("frame.create");
            let sid = res["story"].as_u64().expect("story");
            app.run("text.select", json!({"story": sid, "anchor": 0, "focus": 0})).expect("text.select");
            app.run("style.paragraph.apply", json!({"name": "Heading 1"})).expect("style.paragraph.apply");
        }
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let pages = app.session.active().unwrap().doc.page_count();
        crate::canvas::go_to_page(&mut app, pages - 1);

        let mut st = PubgenState {
            toc_kind: 3,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Isi".into(),
            toc_pair_bilingual: true,
            ..Default::default()
        };
        // The checkbox alone does nothing: pairing fires only with the Dual Lang toggle on.
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");
        {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().expect("table of contents");
            let text = active.doc.story(toc.story).expect("TOC story").text.clone();
            assert!(!text.contains(" / "), "single lang mode never pairs: {text}");
        }

        st.bilingual_output = true;
        let res = execute_rebuild_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");
        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains("Proporsi fisik daratan / Physical proportions of the landmass"), "the pair merges into one row: {text}");
        assert!(!text.lines().filter(|l| l.contains("Proporsi")).any(|l| !l.contains(" / ")), "no unpaired Indonesian row: {text}");
        // The stored definition keeps the option, so Rebuild TOC pairs again.
        assert!(toc.entries.iter().any(|e| e.pair_columns), "{:?}", toc.entries);
        let res = execute_rebuild_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");
        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains("Proporsi fisik daratan / Physical proportions of the landmass"), "the rebuild keeps the pairing: {text}");
    }

    #[test]
    fn test_toc_paragraph_long_heading2_wraps_early_without_hanging_indent() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        ensure_para_style(&mut app, "Heading 2");
        ensure_para_style(&mut app, TOC_NARRATIVE_STYLE);

        let active = app.session.active().unwrap();
        let si = 0;
        let p = &active.doc.spreads[si].pages[0];
        let bounds = p.bounds();
        let mid = bounds.center().x;
        let (left_col, right_col) = ([bounds.x0 + 36.0, mid - 18.0], [mid + 18.0, bounds.x1 - 36.0]);

        // Place chapter title
        let _ = app.run("frame.create", json!({"spread": si, "rect": [bounds.x0 + 36.0, 36.0, bounds.x1 - 36.0, 72.0], "content": "text", "text": "BAB 02 Pemerintahan", "caret": false})).unwrap();

        // Place long heading 2 in left column (ID) and right column (EN)
        let id_heading = "Analisis Struktur Wilayah Administrasi dan Kapasitas Sumber Daya Manusia Kecamatan Karawang Timur";
        let en_heading = "Administrative Structure and Human Resource Capacity in Karawang Timur District";

        for (text, [x0, x1]) in [(id_heading, left_col), (en_heading, right_col)] {
            let res = app
                .run("frame.create", json!({"spread": si, "rect": [x0, 120.0, x1, 240.0], "content": "text", "text": text, "caret": false}))
                .expect("frame.create");
            let sid = res["story"].as_u64().expect("story");
            app.run("text.select", json!({"story": sid, "anchor": 0, "focus": 0})).expect("text.select");
            app.run("style.paragraph.apply", json!({"name": "Heading 2"})).expect("style.paragraph.apply");
        }

        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let pages = app.session.active().unwrap().doc.page_count();
        crate::canvas::go_to_page(&mut app, pages - 1);

        let mut st = PubgenState {
            toc_kind: 3, // TOC Paragraph
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Isi".into(),
            toc_pair_bilingual: true,
            bilingual_output: true,
            toc_hanging_indent: true, // User had this checked by default
            ..Default::default()
        };

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let story = active.doc.story(toc.story).expect("TOC story");

        // Verify style indentation for TOC Level 3 (Heading 2)
        let style = active.doc.styles.para("TOC Level 3").expect("TOC Level 3 style");
        assert_eq!(style.para.left_indent, Some(24.0), "TOC Level 3 left_indent must equal base_indent (24.0) without unnecessary hanging indent");
        assert_eq!(style.para.first_line_indent, Some(0.0), "TOC Level 3 first_line_indent must be 0.0 so continuation lines align with first line");

        // Verify the long paired heading was wrapped early using FORCED_LINE_BREAK
        assert!(
            story.text.contains(designcraft_doc::story::FORCED_LINE_BREAK),
            "Long Heading 2 should be early-wrapped with soft line breaks: {}",
            story.text
        );

        // Verify English half retains TOC English (italic) styling across the wrapped lines
        let en_pos = story.text.find("Administrative Structure").expect("English text start");
        let fmt = story.char_format_of(en_pos);
        assert_eq!(fmt.style, "TOC English", "English portion must be styled with TOC English italic");

        let en_end_pos = story.text.find("District").expect("English text end");
        let fmt_end = story.char_format_of(en_end_pos);
        assert_eq!(fmt_end.style, "TOC English", "End of English portion across line breaks must remain TOC English");

        // Verify Indonesian half retains normal style
        let id_pos = story.text.find("Analisis Struktur").expect("ID text start");
        let fmt_id = story.char_format_of(id_pos);
        assert_eq!(fmt_id.style, designcraft_doc::NO_CHAR_STYLE, "ID portion must have default char style");
    }

    #[test]
    fn test_toc_paragraph_lists_a_legacy_chapter_page_until_its_pages_are_regenerated() {
        // A page made before the narrative style existed: the chapter line in a plain box, the
        // title below it tagged with the card's TOC style.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        ensure_para_style(&mut app, "TOC Entry");
        let (si, pi) = app.session.active().unwrap().doc.page_loc(0).unwrap();
        let b = app.session.active().unwrap().doc.spreads[si].pages[pi].bounds();
        for (text, style, dy) in [("BAB 01", None, 0.0_f64), ("Geografi", Some("TOC Entry"), 40.0_f64)] {
            let rect = [b.x0 + 36.0, b.y0 + 36.0 + dy, b.x1 - 36.0, b.y0 + 76.0 + dy];
            let res =
                app.run("frame.create", json!({"spread": si, "rect": rect, "content": "text", "text": text, "caret": false})).expect("frame.create");
            if let Some(style) = style {
                let sid = res["story"].as_u64().expect("story");
                app.run("text.select", json!({"story": sid, "anchor": 0, "focus": text.chars().count() as u64})).expect("text.select");
                app.run("style.paragraph.apply", json!({"name": style})).expect("style.paragraph.apply");
            }
        }
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        let mut st = PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), toc_title: "Daftar Isi".into(), ..Default::default() };
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains("BAB 01 Geografi"), "an older document still lists its chapter: {text}");
    }

    #[test]
    fn test_toc_table_list_names_itself_when_include_self_is_on() {
        // The Tables list gets the self entry like the plain list does.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![vec!["Tabel".into(), "02.01.".into(), "Jumlah Rukun Warga (RW)".into()]],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 5,
            toc_kind: 1,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Tabel".into(),
            toc_include_self: true,
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();
        crate::canvas::go_to_page(&mut app, 0);
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let first_frame = active.doc.story(toc.story).expect("TOC story").frames.first().copied().expect("frame");
        let toc_pg = active.doc.page_of_item(first_frame).expect("page");
        let label = app.session.page_label(toc_pg);
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains(&format!("Daftar Tabel\t{label}")), "the tables list names itself: {text}");
    }

    #[test]
    fn test_tablepage_toc_layout_style_generates_header_and_standalone_number_column() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "02.01.".into(), "Jumlah Rukun Warga (RW)".into()],
                vec!["Tabel".into(), "02.02.".into(), "Jumlah Pegawai Negeri Sipil".into()],
            ],
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Tabel".into(),
            toc_layout_style: 1, // TablePage mode
            ..Default::default()
        };

        // Generate pages
        execute_generate_pages(&mut app, &st).unwrap();

        // Insert blank page at beginning for TOC
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc in TablePage mode should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story = active.doc.story(toc.story).unwrap();
        let text = &story.text;

        // 1. Must contain "Daftar Tabel" title and "Tabel\thalaman" column header
        assert!(text.contains("Daftar Tabel"), "Should contain title: {text}");
        assert!(text.contains("Tabel\thalaman"), "Should contain TablePage header line: {text}");

        // 2. Entries must use standalone number column: {num}\t{title}\t{page}
        assert!(text.contains("02.01\tJumlah Rukun Warga (RW)\t2"), "Entry 1 must have standalone number column: {text}");
        assert!(text.contains("02.02\tJumlah Pegawai Negeri Sipil\t3"), "Entry 2 must have standalone number column: {text}");

        // 3. Verify paragraph styles applied
        assert!(active.doc.styles.para("TOC TablePage Header").is_some());
        assert!(active.doc.styles.para("TOC Level 1").is_some());
    }

    #[test]
    fn test_tablepage_toc_layout_section_headers_without_page_numbers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Bab".into(), "4.3".into(), "Perumahan dan Lingkungan".into()],
                vec!["Tabel".into(), "4.3.1".into(), "Banyaknya Desa Menurut Penerangan Jalan".into()],
            ],
            toc_style: "TOC Entry".into(),
            toc_title: "".into(), // Blank title: starts directly with Tabel\thalaman
            toc_layout_style: 1,  // TablePage mode
            ..Default::default()
        };

        // Generate pages
        execute_generate_pages(&mut app, &st).unwrap();

        // Insert blank page for TOC
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc in TablePage mode should succeed: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story = active.doc.story(toc.story).unwrap();
        let text = &story.text;

        // 1. Starts directly with Tabel\thalaman header
        assert!(text.starts_with("Tabel\thalaman"), "Should start with header when title is empty: {text}");

        // 2. Section header 4.3 must NOT have page number or leader dots
        assert!(text.contains("4.3\tPerumahan dan Lingkungan\n"), "Section header must not have a page number: {text}");

        // 3. Table entry 4.3.1 must have page number
        assert!(text.contains("4.3.1\tBanyaknya Desa Menurut Penerangan Jalan\t3"), "Table entry must have page number: {text}");

        // 4. Styles
        assert!(active.doc.styles.para("TOC Section Header").is_some());
    }

    #[test]
    fn test_tablepage_toc_continuation_headers_on_overflow_pages() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 80 records so TOC spans across multiple pages (at least 2 pages)
        let records: Vec<Vec<String>> =
            (1..=80).map(|i| vec!["Tabel".into(), format!("01.{i:02}."), format!("Dataset Observation Record Number {i} in Region")]).collect();

        let mut st = PubgenState {
            card1_records: records,
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_preset: 5, // TOC Line preset
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Tabel".into(),
            toc_layout_style: 1, // TablePage mode
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();

        // Insert TOC starting page at beginning
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        crate::canvas::go_to_page(&mut app, 0);

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        let (cont_count, expected_flow_top, second_frame_y0) = {
            let active = app.session.active().unwrap();
            let toc = active.doc.tocs.last().unwrap();
            let story_id = toc.story;
            let story = active.doc.story(story_id).unwrap();

            // Must have more than 1 frame because 80 entries overflow page 1
            assert!(story.frames.len() > 1, "TOC should span across multiple frames: {}", story.frames.len());

            // Find continuation header items across spreads
            let mut cont_headers = Vec::new();
            for spread in &active.doc.spreads {
                for it in &spread.items {
                    if it.label == TOC_CONTINUATION_HEADER_LABEL {
                        cont_headers.push(it.clone());
                    }
                }
            }

            assert!(!cont_headers.is_empty(), "Continuation header items must be created for overflowed pages");

            // Inspect the continuation header text
            let first_hdr = &cont_headers[0];
            let tf = first_hdr.text_frame().unwrap();
            let hdr_story = active.doc.story(tf.story).unwrap();
            assert_eq!(hdr_story.text.trim(), "Tabel\thalaman");

            // Verify the continuation text frame on page 2 starts below the header
            let second_frame_id = story.frames[1];
            let second_frame_item = active.doc.item(second_frame_id).unwrap();
            let second_pg = active.doc.page_of_item(second_frame_id).unwrap();
            let (si, pi) = active.doc.page_loc(second_pg).unwrap();
            let m = active.doc.spreads[si].pages[pi].margin_rect();

            let expected_flow_top = m.y0 + TOC_CONT_HEADER_H + TOC_CONT_HEADER_GAP;
            (cont_headers.len(), expected_flow_top, second_frame_item.bounds().y0)
        };

        assert!(
            (second_frame_y0 - expected_flow_top).abs() < 1.0,
            "Continuation text frame must start below header at {expected_flow_top}, got {second_frame_y0}"
        );

        // Test Rebuild TOC: ensure rebuild cleanly preserves single continuation header per page
        let rebuild_res = execute_rebuild_toc(&mut app, &mut st);
        assert!(rebuild_res.is_ok(), "execute_rebuild_toc should succeed: {rebuild_res:?}");

        let active2 = app.session.active().unwrap();
        let mut cont_headers_after = Vec::new();
        for spread in &active2.doc.spreads {
            for it in &spread.items {
                if it.label == TOC_CONTINUATION_HEADER_LABEL {
                    cont_headers_after.push(it.clone());
                }
            }
        }
        assert_eq!(cont_count, cont_headers_after.len(), "Rebuild should not duplicate continuation headers");
    }

    #[test]
    fn test_generating_the_paragraph_toc_leaves_the_table_toc_alone() {
        // Each list keeps its own entry styles now: the Tables list hangs its rows differently
        // from the Contents list, and the styles they used to share carried the tab stop and
        // indent of whichever list was generated last — regenerating the Contents moved the
        // Tables page's rows.
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        for name in ["TOC Entry", TOC_NARRATIVE_STYLE] {
            ensure_para_style(&mut app, name);
        }
        add_page_text_at(&mut app, 0, "BAB 01 Geografi", TOC_NARRATIVE_STYLE, 0.0);
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        add_page_text_at(&mut app, 1, "Tabel 01.01 Jarak Antar Kecamatan", "TOC Entry", 0.0);
        app.run("layout.pages.insert", json!({"count": 2})).unwrap();

        let table_state = PubgenState {
            toc_kind: 1,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Tabel".into(),
            toc_hanging_indent: false,
            ..Default::default()
        };
        let para_state = PubgenState {
            toc_kind: 3,
            toc_style: "TOC Entry".into(),
            toc_title: "Daftar Isi".into(),
            toc_hanging_indent: true,
            ..Default::default()
        };
        let table_plan = TocPlan::from_state(&table_state);
        let para_plan = TocPlan::from_state(&para_state);
        generate_toc(&mut app, &table_state, 2, &table_plan, false).expect("table toc");
        let table_text = toc_text_on_page(&app, 2).expect("the Tables list on page 2");
        assert!(table_text.contains("Tabel 01.01"), "the Tables list names the table page: {table_text:?}");

        generate_toc(&mut app, &para_state, 3, &para_plan, false).expect("paragraph toc");

        assert_eq!(toc_text_on_page(&app, 2).as_deref(), Some(table_text.as_str()), "the Tables list is untouched");
        let active = app.session.active().unwrap();
        let table_style = active.doc.styles.para("TOC Table Level 1").expect("the Tables list's own style");
        assert_eq!(table_style.para.left_indent, Some(0.0), "the Tables list keeps its (unhung) indent");
        assert_eq!(table_style.para.first_line_indent, Some(0.0));
        let contents_style = active.doc.styles.para("TOC Level 1").expect("the Contents list's style");
        assert_eq!(contents_style.para.left_indent, Some(0.0), "the Contents list keeps its unhung indent for paragraph TOC");
        assert_eq!(contents_style.para.first_line_indent, Some(0.0));
        let table_entry = active
            .doc
            .tocs
            .iter()
            .find(|t| toc_story_page(&app, t.story) == Some(2))
            .and_then(|t| active.doc.story(t.story))
            .and_then(|s| s.paras.get(1).map(|p| p.style.clone()))
            .unwrap_or_default();
        assert_eq!(table_entry, "TOC Table Level 1", "the Tables list's entry takes its own style");
    }

    #[test]
    fn test_toc_paragraph_kind_tables_and_plan() {
        assert_eq!(toc_kind_words(3), ["BAB"]);
        assert_eq!(toc_kind_title(3, false), Some("Daftar Isi"));
        assert_eq!(toc_kind_title(3, true), Some("Daftar Isi / Table of Contents"));
        assert_eq!(toc_kind_of_words(&["BAB".into()]), 3);
        // The older kinds keep their detection.
        assert_eq!(toc_kind_of_words(&["Tabel".into(), "Table".into()]), 1);
        assert_eq!(toc_kind_of_words(&["Grafik".into(), "Chart".into()]), 2);
        assert_eq!(toc_kind_of_words(&[]), 0);

        let st = PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() };
        let plan = TocPlan::from_state(&st);
        assert!(plan.is_paragraph_toc());
        let levels: Vec<(&str, u8)> = plan.levels.iter().map(|l| (l.style.as_str(), l.level)).collect();
        assert_eq!(levels, [(TOC_NARRATIVE_STYLE, 1), ("Heading 1", 2), ("Heading 2", 3)]);
        let json = plan.entries_json();
        let arr = json.as_array().expect("entries array");
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["chapterNumber"], json!(true), "the narrative entry carries the chapter line");
        assert!(arr[0].get("pageTitle").is_none(), "the narrative style alone picks the narrative pages");
        assert!(arr[1].get("pageTitle").is_none(), "heading levels list every paragraph of the style");

        // The plain list keeps the tagged page titles and adds the narrative pages.
        let st = PubgenState { toc_kind: 0, toc_style: "TOC Entry".into(), ..Default::default() };
        let plan = TocPlan::from_state(&st);
        assert_eq!(plan.entries_json().as_array().map(Vec::len), Some(2));

        // A single-level kind still emits one entry.
        let st = PubgenState { toc_kind: 1, toc_style: "TOC Entry".into(), ..Default::default() };
        let plan = TocPlan::from_state(&st);
        assert!(!plan.is_paragraph_toc());
        assert_eq!(plan.entries_json().as_array().map(Vec::len), Some(1));
        assert_eq!(plan.kind(), 1);
    }

    /// Add a text frame on absolute `page` and tag its paragraphs with `style`.
    fn add_page_text(app: &mut DesignApp, page: usize, text: &str, style: &str) {
        add_page_text_at(app, page, text, style, 0.0);
    }

    /// The same, `dy` points below the top margin: a page with two text boxes (the chapter line
    /// and the title below it).
    fn add_page_text_at(app: &mut DesignApp, page: usize, text: &str, style: &str, dy: f64) {
        let (si, pi) = app.session.active().expect("document").doc.page_loc(page).expect("page");
        let b = app.session.active().expect("document").doc.spreads[si].pages[pi].bounds();
        let rect = [b.x0 + 36.0, b.y0 + 36.0 + dy, b.x1 - 36.0, b.y0 + 140.0 + dy];
        let res =
            app.run("frame.create", json!({"spread": si, "rect": rect, "content": "text", "text": text, "caret": false})).expect("frame.create");
        let sid = res["story"].as_u64().expect("story");
        let end = text.chars().count().max(1) as u64;
        app.run("text.select", json!({"story": sid, "anchor": 0, "focus": end})).expect("text.select");
        app.run("style.paragraph.apply", json!({"name": style})).expect("style.paragraph.apply");
    }

    /// The text of the table of contents whose first frame sits on `page`.
    fn toc_text_on_page(app: &DesignApp, page: usize) -> Option<String> {
        let active = app.session.active()?;
        let toc = active.doc.tocs.iter().find(|t| toc_story_page(app, t.story) == Some(page))?;
        active.doc.story(toc.story).map(|s| s.text.clone())
    }

    /// Four content pages (chapter, two headings, a table) plus a blank page for the TOC.
    fn paragraph_toc_doc(app: &mut DesignApp) {
        app.run("file.new", json!({})).unwrap();
        for name in ["TOC Entry", TOC_NARRATIVE_STYLE, "Heading 1", "Heading 2"] {
            ensure_para_style(app, name);
        }
        // A narrative page: the chapter line in its own text box, the title below it.
        add_page_text_at(app, 0, "BAB 01", "TOC Entry", 0.0);
        add_page_text_at(app, 0, "Geografi", TOC_NARRATIVE_STYLE, 40.0);
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        add_page_text(app, 1, "Pendahuluan", "Heading 1");
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        add_page_text(app, 2, "Latar Belakang", "Heading 2");
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        add_page_text(app, 3, "Tabel 01.01 Jarak Antar Kecamatan", "TOC Entry");
        // The table of contents lands on the last page (no canvas in tests).
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
    }

    #[test]
    fn test_toc_paragraph_scan_finds_styled_paragraphs() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        paragraph_toc_doc(&mut app);

        let st = PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), ..Default::default() };
        let plan = TocPlan::from_state(&st);
        let found = app.session.execute("toc.entries", &json!({"entries": plan.entries_json()})).expect("toc.entries");
        let rows: Vec<(u8, String)> = found
            .as_array()
            .expect("entries")
            .iter()
            .map(|e| (e["level"].as_u64().expect("level") as u8, e["text"].as_str().expect("text").to_string()))
            .collect();

        assert_eq!(
            rows.iter().map(|(l, t)| (*l, t.as_str())).collect::<Vec<_>>(),
            [(1, "BAB 01 Geografi"), (2, "Pendahuluan"), (3, "Latar Belakang")],
            "the narrative page at level 1 with its chapter line, the headings nested under it, the table page left out: {rows:?}"
        );
    }

    #[test]
    fn test_toc_paragraph_keeps_toc_level_styles_and_daftar_isi_title() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        paragraph_toc_doc(&mut app);

        let mut st = PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), toc_title: "Daftar Isi".into(), ..Default::default() };
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");
        assert!(res.as_deref().unwrap_or_default().contains("TOC Paragraph"), "{}", res.unwrap_or_default());

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let story = active.doc.story(toc.story).expect("TOC story");
        assert!(story.text.starts_with("Daftar Isi"), "the title line: {}", story.text);
        assert!(story.text.contains("BAB 01 Geografi"), "the narrative entry reads chapter line and title: {}", story.text);
        assert!(story.text.contains("Pendahuluan") && story.text.contains("Latar Belakang"), "{}", story.text);
        assert!(!story.text.contains("Tabel 01.01"), "table pages stay out of a paragraph list: {}", story.text);

        let styles: Vec<&str> = story.paras.iter().map(|p| p.style.as_str()).collect();
        assert_eq!(
            styles,
            ["TOC Title", "TOC Level 1", "TOC Level 2", "TOC Level 3"],
            "each entry keeps its own level style instead of being flattened: {styles:?}"
        );
    }

    #[test]
    fn test_toc_include_self_lists_the_toc_page_in_a_paragraph_list() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        paragraph_toc_doc(&mut app);

        let mut st =
            PubgenState { toc_kind: 3, toc_style: "TOC Entry".into(), toc_title: "Daftar Isi".into(), toc_include_self: true, ..Default::default() };
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let first_frame = active.doc.story(toc.story).expect("TOC story").frames.first().copied().expect("frame");
        let toc_page = active.doc.page_of_item(first_frame).expect("page");
        let label = app.session.page_label(toc_page);
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(text.contains(&format!("Daftar Isi\t{label}")), "the list names the table of contents itself: {text}");

        // Without the option the entry is gone again.
        st.toc_include_self = false;
        assert!(execute_rebuild_toc(&mut app, &mut st).is_ok());
        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().expect("table of contents");
        let text = active.doc.story(toc.story).expect("TOC story").text.clone();
        assert!(!text.contains(&format!("Daftar Isi\t{label}")), "the entry is gone when the option is off: {text}");
    }

    #[test]
    fn test_toc_self_entry_sits_in_page_order() {
        // The entry that lists the table of contents itself falls where its page falls among the
        // entries: first when the table opens the document, last when it closes it.
        let cases = [(0_usize, "Daftar Isi\t1\nBAB 01 Geografi\t3"), (4, "Pendahuluan\t4\nDaftar Isi\t5")];
        for (toc_page, expected) in cases {
            let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
            app.run("file.new", json!({})).unwrap();
            ensure_para_style(&mut app, "TOC Entry");
            app.run("layout.pages.insert", json!({"count": 4})).unwrap();
            let (si, pi) = app.session.active().unwrap().doc.page_loc(0).unwrap();
            let b = app.session.active().unwrap().doc.spreads[si].pages[pi].bounds();
            let rect = [b.x0 + 36.0, b.y0 + 36.0, b.x1 - 36.0, b.y0 + 200.0];
            let res = app
                .run(
                    "frame.create",
                    json!({"spread": si, "rect": rect, "content": "text", "caret": false, "text": "Daftar Isi\nBAB 01 Geografi\t3\nPendahuluan\t4"}),
                )
                .expect("frame.create");
            let sid = designcraft_doc::StoryId(res["story"].as_u64().expect("story"));
            let st = PubgenState {
                toc_kind: 3,
                toc_style: "TOC Entry".into(),
                toc_title: "Daftar Isi".into(),
                toc_include_self: true,
                ..Default::default()
            };
            let plan = TocPlan::from_state(&st);
            add_toc_self_entry(&mut app, sid, &plan, toc_page);

            let active = app.session.active().unwrap();
            let text = active.doc.story(sid).expect("story").text.clone();
            assert!(text.contains(expected), "on page {}: {text}", toc_page + 1);
            assert_eq!(text.split('\n').count(), 4, "one paragraph per entry: {text}");
        }
    }

    #[test]
    fn test_toc_table_style_lists_only_table_pages_and_centers_its_title() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Grafik".into(), "1.1".into(), "Curah Hujan Tiap Bulan".into()],
                vec!["Tabel".into(), "01.02.".into(), "Keadaan Curah Hujan".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_preset: 5, // "TOC Line": each page's text box reads "Tabel 01.01. Jarak …"
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        // Quick Actions TOC style = "TOC Table" (its title was filled in when it was picked).
        st.toc_kind = 1;
        st.toc_title = toc_kind_title(1, false).unwrap().to_string();
        assert_eq!(st.toc_title, "Daftar Tabel");
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story = active.doc.story(toc.story).unwrap();
        let text = story.text.clone();
        assert!(text.starts_with("Daftar Tabel"), "{text}");
        assert!(text.contains("Tabel 01.01. Jarak Antar Kecamatan"), "{text}");
        assert!(text.contains("Tabel 01.02. Keadaan Curah Hujan"), "{text}");
        assert!(!text.contains("Grafik 1.1."), "a list of tables lists no charts: {text}");
        assert_eq!(story.paras[0].style, "TOC Title");
        assert_eq!(story.paras[0].para.align, Some(designcraft_doc::Align::Center), "the title is centered");
        // The filter lives in the document, so the table stays a list of tables.
        assert_eq!(toc.entries[0].page_title, ["Tabel", "Table"]);
        assert_eq!(toc_kind_of_words(&toc.entries[0].page_title), 1);
    }

    #[test]
    fn test_toc_chart_page_layout_names_charts_in_its_column_header() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Grafik".into(), "1.1".into(), "Curah Hujan Tiap Bulan".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_preset: 5,
            toc_style: "TOC Entry".into(),
            toc_layout_style: 1, // TablePage (BPS catalog) writes a Tabel/Grafik column header
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        st.toc_kind = 2;
        st.toc_title = toc_kind_title(2, false).unwrap().to_string();
        assert_eq!(st.toc_title, "Daftar Grafik");
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = active.doc.story(toc.story).unwrap().text.clone();
        assert!(text.contains("Grafik\thalaman"), "a list of charts heads its column with Grafik: {text}");
        assert!(!text.contains("Tabel\thalaman"), "{text}");
        assert!(text.contains("Curah Hujan Tiap Bulan"), "{text}");
        assert!(!text.contains("Jarak Antar Kecamatan"), "a list of charts lists no tables: {text}");

        // The header follows the kind and the language of the source records: an English chart.
        st.card1_records = vec![vec!["Chart".into(), "1.1".into(), "Population Pyramid".into()]];
        assert_eq!(toc_header_line(&TocPlan::from_state(&st), &st), "Chart\tpage");
        st.card1_records = vec![vec!["Tabel".into(), "1.1".into(), "Luas Wilayah".into()]];
        assert_eq!(toc_header_line(&TocPlan::from_state(&st), &st), "Grafik\thalaman");
    }

    #[test]
    fn test_toc_chart_tablepage_layout_format() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let img_path = sample_image_path();
        let mut st = PubgenState {
            card1_records: vec![vec!["Grafik".into(), "02.01".into(), "Grafik 02.01 Panjang Jalan Kecamatan Karawang Timur 2026".into(), img_path]],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            toc_layout_style: 1, // TablePage
            ..Default::default()
        };

        execute_generate_pages(&mut app, &st).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        st.toc_kind = 2; // TOC Chart
        st.toc_title = "Daftar Grafik".into();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = active.doc.story(toc.story).unwrap().text.clone();
        assert!(text.contains("Grafik\thalaman"), "header must be present: {text}");
        // Must contain single line for the chart matching TablePage layout:
        // "02.01\tPanjang Jalan Kecamatan Karawang Timur 2026\t<page>"
        assert!(
            text.contains("02.01\tPanjang Jalan Kecamatan Karawang Timur 2026"),
            "Should format chart number and title into TablePage columns: {text}"
        );
        assert!(!text.contains("Grafik 02.01\t"), "Should not contain raw 'Grafik 02.01' line: {text}");
    }

    #[test]
    fn test_toc_chart_tablepage_layout_with_existing_newline_paragraphs() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Create page 1 with 2 separate paragraphs in a single frame, both with style "TOC Entry"
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        ensure_para_style(&mut app, "TOC Entry");
        let (spread_idx, page_idx) = app.session.active().unwrap().doc.page_loc(1).unwrap();
        let margin = app.session.active().unwrap().doc.spreads[spread_idx].pages[page_idx].margin_rect();
        frame_with_centered_bold_title(
            &mut app,
            spread_idx,
            [margin.x0, margin.y0, margin.x1, margin.y0 + 48.0],
            "Grafik 02.01\nPanjang Jalan Kecamatan Karawang Timur 2026",
            Some("TOC Entry"),
        );

        let mut st = PubgenState {
            toc_kind: 2,
            toc_title: "Daftar Grafik".into(),
            toc_style: "TOC Entry".into(),
            toc_layout_style: 1, // TablePage
            ..Default::default()
        };

        crate::canvas::go_to_page(&mut app, 0);
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "{res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let text = active.doc.story(toc.story).unwrap().text.clone();
        assert!(text.contains("Grafik\thalaman"), "header must be present: {text}");
        assert!(
            text.contains("02.01\tPanjang Jalan Kecamatan Karawang Timur 2026\t2"),
            "Should coalesce the 2 paragraphs on page 2 into one TablePage entry: {text}"
        );
        assert!(!text.contains("Grafik 02.01\t"), "Should not contain raw 'Grafik 02.01' line: {text}");
    }

    #[test]
    fn test_rebuild_toc_rebuilds_only_the_table_of_contents_in_view() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Jarak Antar Kecamatan".into()],
                vec!["Grafik".into(), "1.1".into(), "Curah Hujan Tiap Bulan".into()],
            ],
            card1_preset: 5,
            card1_range_all: true,
            toc_style: "TOC Entry".into(),
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        // A list of tables on one page, a list of charts on the next.
        app.run("layout.pages.insert", json!({"count": 2})).unwrap();
        let pages = app.session.active().unwrap().doc.page_count();
        app.canvas_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0)));
        crate::canvas::go_to_page(&mut app, pages - 2);
        st.toc_kind = 1;
        st.toc_title = "Daftar Table".into();
        execute_add_toc(&mut app, &mut st).unwrap();
        crate::canvas::go_to_page(&mut app, pages - 1);
        st.toc_kind = 2;
        st.toc_title = "Daftar Grafik".into();
        execute_add_toc(&mut app, &mut st).unwrap();

        let (tables_sid, charts_sid) = {
            let doc = &app.session.active().unwrap().doc;
            assert_eq!(doc.tocs.len(), 2, "{:?}", doc.tocs);
            (doc.tocs[0].story, doc.tocs[1].story)
        };
        let before = app.session.active().unwrap().doc.story(tables_sid).unwrap().text.clone();

        // Pressing Rebuild with the panel back on "TOC Entry"/"Contents" rebuilds the table on the
        // page in view with the TOC style it was created with, and leaves the other one alone.
        st.toc_kind = 0;
        st.toc_title = "Contents".into();
        let msg = execute_rebuild_toc(&mut app, &mut st).unwrap();
        assert!(msg.contains("Rebuilt TOC"), "{msg}");

        let active = app.session.active().unwrap();
        assert_eq!(active.doc.tocs.len(), 2, "a rebuild adds no third table");
        let charts = active.doc.tocs.iter().find(|t| t.story == charts_sid).unwrap();
        assert_eq!(charts.title, "Daftar Grafik", "the title it was made with");
        assert_eq!(charts.entries[0].page_title, ["Grafik", "Chart"], "still a list of charts");
        let text = active.doc.story(charts_sid).unwrap().text.clone();
        assert!(text.starts_with("Daftar Grafik"), "{text}");
        assert!(!text.contains("Tabel 01.01."), "{text}");
        assert_eq!(active.doc.story(tables_sid).unwrap().text, before, "the other table is untouched");
    }

    #[test]
    fn test_split_chapter_title() {
        assert_eq!(split_chapter_title("BAB 01 Pemerintahan", "01"), ("BAB 01".into(), "Pemerintahan".into()));
        assert_eq!(split_chapter_title("BAB 01. Pemerintahan Daerah", "01"), ("BAB 01".into(), "Pemerintahan Daerah".into()));
        assert_eq!(split_chapter_title("BAB 2 Keadaan Alam", "02"), ("BAB 2".into(), "Keadaan Alam".into()));
        assert_eq!(split_chapter_title("Pemerintahan", "01"), ("BAB 01".into(), "Pemerintahan".into()));
    }

    #[test]
    fn test_split_chart_title() {
        assert_eq!(
            split_chart_title("Grafik 02.01 Panjang Jalan Kecamatan Karawang Timur 2026", "02.01", "Grafik"),
            ("Grafik 02.01".into(), "Panjang Jalan Kecamatan Karawang Timur 2026".into())
        );
        assert_eq!(split_chart_title("Grafik 02.01", "02.01", "Grafik"), ("Grafik 02.01".into(), "".into()));
        assert_eq!(split_chart_title("02.01 Panjang Jalan", "02.01", "Grafik"), ("Grafik 02.01".into(), "Panjang Jalan".into()));
        assert_eq!(split_chart_title("Panjang Jalan Kecamatan", "02.01", "Grafik"), ("Grafik 02.01".into(), "Panjang Jalan Kecamatan".into()));
    }

    #[test]
    fn test_execute_add_toc_insert_after_selected_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();

        // Place a styled title on Page 2 (0-based index 1)
        let para = designcraft_doc::ParaFormat { style: "TOC Chapter".to_string(), ..Default::default() };
        let _ = app.session.edit(|doc, _| {
            doc.add_text_frame(
                designcraft_doc::SpreadRef::Doc(1),
                designcraft_geom::Rect::new(36.0, 36.0, 200.0, 100.0),
                designcraft_doc::LayerId(1),
                "Chapter 1 Overview",
                para,
            )?;
            Ok(())
        });

        let mut st = PubgenState::default();
        st.toc_style = "TOC Chapter".to_string();
        st.toc_insert_after = true;

        // Select Page 1 (0-based index 0)
        app.session.active_mut().unwrap().selection.page = Some(0);

        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc should succeed: {res:?}");

        // The document should now have 3 pages:
        // Page 1: original Page 1
        // Page 2: newly inserted blank page with the TOC
        // Page 3: the chapter page (shifted from index 1 to index 2)
        let doc_pages = app.session.active().unwrap().doc.page_count();
        assert_eq!(doc_pages, 3);

        // Verify the TOC is on Page 2 (0-based index 1)
        let toc_story_id = toc_story_on_page(&app, 1).unwrap();
        assert_eq!(toc_story_page(&app, toc_story_id), Some(1));
    }

    #[test]
    fn test_toc_style_default_titles_single_and_dual_lang() {
        // 1. TOC Table (kind 1)
        assert_eq!(toc_kind_title(1, false), Some("Daftar Tabel"));
        assert_eq!(toc_kind_title(1, true), Some("Daftar Tabel / List of Tables"));

        // 2. TOC Chart (kind 2)
        assert_eq!(toc_kind_title(2, false), Some("Daftar Grafik"));
        assert_eq!(toc_kind_title(2, true), Some("Daftar Grafik / List of Charts"));

        // 3. TOC Paragraph (kind 3)
        assert_eq!(toc_kind_title(3, false), Some("Daftar Isi"));
        assert_eq!(toc_kind_title(3, true), Some("Daftar Isi / Table of Contents"));

        // Verify switching modes updates default title
        let mut st = PubgenState {
            toc_kind: 1, // TOC Table
            toc_title: "Daftar Tabel".into(),
            bilingual_output: false,
            ..Default::default()
        };

        // Simulate toggling to Dual Lang mode
        let was_bilingual = st.bilingual_output;
        st.bilingual_output = !st.bilingual_output;
        if let Some(old_title) = toc_kind_title(st.toc_kind, was_bilingual) {
            if st.toc_title.trim().is_empty() || st.toc_title.trim() == old_title {
                if let Some(new_title) = toc_kind_title(st.toc_kind, st.bilingual_output) {
                    st.toc_title = new_title.to_string();
                }
            }
        }
        assert_eq!(st.toc_title, "Daftar Tabel / List of Tables");

        // Simulate toggling back to Single Lang mode
        let was_bilingual = st.bilingual_output;
        st.bilingual_output = !st.bilingual_output;
        if let Some(old_title) = toc_kind_title(st.toc_kind, was_bilingual) {
            if st.toc_title.trim().is_empty() || st.toc_title.trim() == old_title {
                if let Some(new_title) = toc_kind_title(st.toc_kind, st.bilingual_output) {
                    st.toc_title = new_title.to_string();
                }
            }
        }
        assert_eq!(st.toc_title, "Daftar Tabel");

        // Custom titles are preserved and not overwritten on mode toggle
        st.toc_title = "Custom Table List".into();
        let was_bilingual = st.bilingual_output;
        st.bilingual_output = !st.bilingual_output;
        if let Some(old_title) = toc_kind_title(st.toc_kind, was_bilingual) {
            if st.toc_title.trim().is_empty() || st.toc_title.trim() == old_title {
                if let Some(new_title) = toc_kind_title(st.toc_kind, st.bilingual_output) {
                    st.toc_title = new_title.to_string();
                }
            }
        }
        assert_eq!(st.toc_title, "Custom Table List");
    }

    #[test]
    fn test_toc_header_line_uses_locale_words() {
        let mut st =
            PubgenState { card1_records: vec![vec!["Tabel".into(), "01.01".into(), "Penduduk".into()]], locale: Locale::Id, ..Default::default() };
        let plan_table = TocPlan::from_state(&st);
        assert_eq!(toc_header_line(&plan_table, &st), "Tabel\thalaman");

        st.locale = Locale::En;
        assert_eq!(toc_header_line(&plan_table, &st), "Table\tpage");

        // Dual Lang mode produces two-line header: Tabel / Table, Halaman / Page
        st.bilingual_output = true;
        assert_eq!(toc_header_line(&plan_table, &st), format!("Tabel\tHalaman{}\u{2028}Table\tPage", ""));

        // Chart list
        st.toc_kind = 2;
        let plan_chart = TocPlan::from_state(&st);
        st.bilingual_output = false;
        st.locale = Locale::Id;
        assert_eq!(toc_header_line(&plan_chart, &st), "Grafik\thalaman");

        st.locale = Locale::En;
        assert_eq!(toc_header_line(&plan_chart, &st), "Chart\tpage");

        // Dual Lang mode produces two-line header: Grafik / Chart, Halaman / Page
        st.bilingual_output = true;
        assert_eq!(toc_header_line(&plan_chart, &st), format!("Grafik\tHalaman{}\u{2028}Chart\tPage", ""));
    }

    #[test]
    fn test_toc_table_and_chart_dual_lang_two_line_header() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01.".into(), "Luas Daerah".into()],
                vec!["Grafik".into(), "01.01.".into(), "Pertumbuhan Penduduk".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_preset: 5,
            toc_style: "TOC Entry".into(),
            toc_layout_style: 1, // TablePage layout
            bilingual_output: true,
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st).unwrap();

        // 1. Generate TOC Table in Dual Lang mode
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        st.toc_kind = 1;
        st.toc_title = toc_kind_title(1, true).unwrap().to_string();
        let res = execute_add_toc(&mut app, &mut st);
        assert!(res.is_ok(), "execute_add_toc for TOC Table: {res:?}");

        let active = app.session.active().unwrap();
        let toc = active.doc.tocs.last().unwrap();
        let story = active.doc.story(toc.story).unwrap();
        assert!(story.text.contains("Tabel\tHalaman\u{2028}Table\tPage"), "TOC Table story text: {}", story.text);

        let table_pos = story.text.find("Table\tPage").expect("Table position");
        let fmt_table = story.char_format_of(table_pos);
        assert_eq!(fmt_table.style, "TOC English");
        assert_eq!(fmt_table.over.font_style.as_deref(), Some("Italic"));

        let page_pos = story.text.find("Page").expect("Page position");
        let fmt_page = story.char_format_of(page_pos);
        assert_eq!(fmt_page.style, "TOC English");
        assert_eq!(fmt_page.over.font_style.as_deref(), Some("Italic"));

        let id_table_pos = story.text.find("Tabel\t").expect("Tabel position");
        let fmt_id_table = story.char_format_of(id_table_pos);
        assert_eq!(fmt_id_table.style, designcraft_doc::NO_CHAR_STYLE);

        // 2. Generate TOC Chart in Dual Lang mode
        app.run("layout.pages.insert", json!({"count": 1})).unwrap();
        let pages = app.session.active().unwrap().doc.page_count();
        crate::canvas::go_to_page(&mut app, pages - 1);
        st.toc_kind = 2;
        st.toc_title = toc_kind_title(2, true).unwrap().to_string();
        let res_chart = execute_add_toc(&mut app, &mut st);
        assert!(res_chart.is_ok(), "execute_add_toc for TOC Chart: {res_chart:?}");

        let active = app.session.active().unwrap();
        let toc_chart = active.doc.tocs.last().unwrap();
        let story_chart = active.doc.story(toc_chart.story).unwrap();
        assert!(story_chart.text.contains("Grafik\tHalaman\u{2028}Chart\tPage"), "TOC Chart story text: {}", story_chart.text);

        let chart_pos = story_chart.text.find("Chart\tPage").expect("Chart position");
        let fmt_chart = story_chart.char_format_of(chart_pos);
        assert_eq!(fmt_chart.style, "TOC English");
        assert_eq!(fmt_chart.over.font_style.as_deref(), Some("Italic"));
    }

    #[test]
    fn test_split_chapter_title_locale() {
        let (ch1, t1) = split_chapter_title_locale("BAB 01 PEMERINTAHAN", "01", false);
        assert_eq!(ch1, "BAB 01");
        assert_eq!(t1, "PEMERINTAHAN");

        let (ch2, t2) = split_chapter_title_locale("CHAPTER 01 GEOGRAPHY AND CLIMATE", "01", true);
        assert_eq!(ch2, "CHAPTER 01");
        assert_eq!(t2, "GEOGRAPHY AND CLIMATE");

        let (ch3, t3) = split_chapter_title_locale("GEOGRAPHY AND CLIMATE", "01", true);
        assert_eq!(ch3, "CHAPTER 01");
        assert_eq!(t3, "GEOGRAPHY AND CLIMATE");

        let (ch4, t4) = split_chapter_title_locale("Kata Pengantar", "", false);
        assert_eq!(ch4, "Kata Pengantar");
        assert_eq!(t4, "");
    }
}
