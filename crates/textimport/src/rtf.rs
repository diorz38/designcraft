//! RTF → story: text, paragraphs, line breaks, tabs, bold/italic/underline/strikethrough, size,
//! Unicode (`\uN`) and hex (`\'hh`) characters, and the named paragraph styles of the
//! `\stylesheet` table. Destinations that aren't body text (font and colour tables, info,
//! pictures, `\*` groups) are skipped.

use designcraft_doc::{Align, CharAttrs, CharFormat, ParaAttrs, Story, StoryId, TabAlign, TabStop};

use crate::{ImportError, Imported, ImportedStyle};

#[derive(Clone, Default)]
struct State {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    size: Option<f64>,
    language: Option<String>,
    /// Inside a destination whose text is ignored.
    skip: bool,
    /// Inside a `\listtext` group (list numbers/bullets).
    in_listtext: bool,
    /// Characters to skip after `\uN` (`\ucN`).
    uc: usize,
}

impl State {
    fn format(&self) -> CharFormat {
        let font_style = match (self.bold, self.italic) {
            (true, true) => Some("Bold Italic".to_string()),
            (true, false) => Some("Bold".into()),
            (false, true) => Some("Italic".into()),
            _ => None,
        };
        CharFormat {
            style: designcraft_doc::story::NO_CHAR_STYLE.into(),
            over: CharAttrs {
                font_style,
                underline: self.underline.then_some(true),
                strikethrough: self.strike.then_some(true),
                size: self.size,
                language: self.language.clone(),
                ..Default::default()
            },
        }
    }
}

/// Map a Microsoft Locale ID (LCID) to the canonical language name.
fn lcid_to_language(lcid: i64) -> Option<&'static str> {
    match lcid {
        1057 | 14345 => Some("Indonesian"),
        1033 => Some("English: USA"),
        3081 => Some("English: Australian"),
        2057 => Some("English: UK"),
        1025 => Some("Arabic"),
        1031 => Some("German: Standard"),
        1036 => Some("French: Standard"),
        1034 => Some("Spanish: Castilian"),
        1040 => Some("Italian: Standard"),
        1041 => Some("Japanese"),
        2052 => Some("Chinese: Simplified"),
        _ => None,
    }
}

const SKIP: &[&str] = &[
    "fonttbl",
    "colortbl",
    "stylesheet",
    "info",
    "pict",
    "header",
    "footer",
    "footnote",
    "listtable",
    "listoverridetable",
    "rsidtbl",
    "generator",
    "themedata",
    "colorschememapping",
    "latentstyles",
    "datastore",
    "xmlnstbl",
    "fldinst",
    "object",
];

/// A style of the `\stylesheet` table: its `\sN` index, the index it is based on, and its
/// attributes.
struct StyleDef {
    idx: u32,
    based_on_idx: Option<u32>,
    style: ImportedStyle,
}

/// The control word at `i` (`chars[i] == '\\'`), its optional numeric parameter, and the index
/// after the delimiting space.
fn control_word(chars: &[char], i: usize) -> (String, Option<i64>, usize) {
    let mut j = i + 1;
    while j < chars.len() && chars[j].is_ascii_alphabetic() {
        j += 1;
    }
    let word: String = chars[i + 1..j].iter().collect();
    let mut k = j;
    if chars.get(k) == Some(&'-') && chars.get(k + 1).is_some_and(char::is_ascii_digit) || chars.get(k).is_some_and(char::is_ascii_digit) {
        k += 1;
        while k < chars.len() && chars[k].is_ascii_digit() {
            k += 1;
        }
    }
    let param: Option<i64> = chars[j..k].iter().collect::<String>().parse().ok();
    if k < chars.len() && chars[k] == ' ' {
        k += 1;
    }
    (word, param, k)
}

/// The index of the `}` closing the group opened at `open`, or the end of the input.
fn group_end(chars: &[char], open: usize) -> usize {
    let mut depth = 0usize;
    let mut i = open;
    while i < chars.len() {
        // `\{` and `\}` are text, not group boundaries.
        let escaped = i > 0 && chars[i - 1] == '\\';
        match chars[i] {
            '{' if !escaped => depth += 1,
            '}' if !escaped => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    chars.len()
}

/// The index just past the `\name` control word, if the input has one.
fn find_control(chars: &[char], name: &str) -> Option<usize> {
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && chars.get(i + 1).is_some_and(char::is_ascii_alphabetic) {
            let (word, _, next) = control_word(chars, i);
            if word == name {
                return Some(next);
            }
            i = next;
        } else {
            i += 1;
        }
    }
    None
}

/// One `{\sN … Name;}` definition of the stylesheet table.
fn parse_style_def(chars: &[char], start: usize, end: usize, idx: u32) -> StyleDef {
    let mut style = ImportedStyle::default();
    let mut based_on_idx = None;
    let (mut bold, mut italic) = (false, false);
    let mut name = String::new();
    let mut i = start;
    while i < end {
        match chars[i] {
            // Nested groups (`{\*\…}`, `\snext` etc. are control words, not groups).
            '{' => i = group_end(chars, i).saturating_add(1),
            ';' => break,
            '\\' => {
                let next = chars.get(i + 1).copied().unwrap_or(' ');
                if next == '\'' {
                    let hex: String = chars.iter().skip(i + 2).take(2).collect();
                    if let Ok(b) = u8::from_str_radix(&hex, 16) {
                        name.push(cp1252(b));
                    }
                    i += 4;
                    continue;
                }
                if !next.is_ascii_alphabetic() {
                    match next {
                        '\\' | '{' | '}' => name.push(next),
                        _ => {}
                    }
                    i += 2;
                    continue;
                }
                let (word, param, next_i) = control_word(chars, i);
                i = next_i;
                let on = param != Some(0);
                match word.as_str() {
                    "sbasedon" => based_on_idx = param.and_then(|p| u32::try_from(p).ok()),
                    "b" => bold = on,
                    "i" => italic = on,
                    "fs" => {
                        style.chars.size = param.map(|p| p as f64 / 2.0).filter(|pt| (0.1..=1296.0).contains(pt));
                    }
                    "ql" => style.para.align = Some(Align::Left),
                    "qr" => style.para.align = Some(Align::Right),
                    "qc" => style.para.align = Some(Align::Center),
                    "qj" => style.para.align = Some(Align::LeftJustified),
                    "qd" => style.para.align = Some(Align::FullyJustified),
                    "li" => style.para.left_indent = param.map(|v| (v as f64 / 20.0).clamp(-10_000.0, 10_000.0)),
                    "ri" => style.para.right_indent = param.map(|v| (v as f64 / 20.0).clamp(-10_000.0, 10_000.0)),
                    "fi" => style.para.first_line_indent = param.map(|v| (v as f64 / 20.0).clamp(-10_000.0, 10_000.0)),
                    "sa" => style.para.space_after = param.map(|v| (v as f64 / 20.0).clamp(0.0, 10_000.0)),
                    "sb" => style.para.space_before = param.map(|v| (v as f64 / 20.0).clamp(0.0, 10_000.0)),
                    _ => {}
                }
            }
            c => {
                name.push(c);
                i += 1;
            }
        }
    }
    style.name = normalize_style_name(&name);
    style.chars.font_style = match (bold, italic) {
        (true, true) => Some("Bold Italic".into()),
        (true, false) => Some("Bold".into()),
        (false, true) => Some("Italic".into()),
        _ => None,
    };
    StyleDef { idx, based_on_idx, style }
}

/// The DesignCraft name of a stylesheet style. Word writes `heading 1` in lower case, and style
/// names are matched exactly, so the heading/title names the table of contents asks for are
/// canonicalized.
fn normalize_style_name(raw: &str) -> String {
    let name = raw.trim();
    let lower = name.to_ascii_lowercase();
    if lower == "normal" {
        return designcraft_doc::story::BASIC_PARAGRAPH.into();
    }
    if let Some(n) = lower.strip_prefix("heading ") {
        return format!("Heading {}", n.trim());
    }
    if lower == "title" {
        return "Title".into();
    }
    if lower == "subtitle" {
        return "Subtitle".into();
    }
    let mut cs = name.chars();
    match cs.next() {
        Some(f) => f.to_uppercase().collect::<String>() + cs.as_str(),
        None => String::new(),
    }
}

/// The `\stylesheet` table's paragraph styles, by `\sN` index. The body loop keeps skipping the
/// destination; this reads it separately.
fn parse_stylesheet(chars: &[char]) -> Vec<StyleDef> {
    let Some(mut i) = find_control(chars, "stylesheet") else {
        return Vec::new();
    };
    let mut defs = Vec::new();
    // The destination's own group is already open.
    let mut depth = 1usize;
    while i < chars.len() {
        match chars[i] {
            '{' => {
                if let Some(&'\\') = chars.get(i + 1) {
                    let (word, param, next) = control_word(chars, i + 1);
                    if word == "s"
                        && let Some(n) = param.and_then(|p| u32::try_from(p).ok())
                    {
                        let end = group_end(chars, i);
                        defs.push(parse_style_def(chars, next, end, n));
                        i = end.saturating_add(1);
                        continue;
                    }
                }
                depth += 1;
                i += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    let names: std::collections::HashMap<u32, String> = defs.iter().map(|d| (d.idx, d.style.name.clone())).collect();
    for d in &mut defs {
        d.style.based_on = d.based_on_idx.and_then(|b| names.get(&b).cloned());
    }
    defs
}

/// The styles the story uses, plus the ones they are based on. `[Basic Paragraph]` is always in
/// the document, so it is never imported.
fn used_styles(defs: Vec<StyleDef>, st: &Story) -> Vec<ImportedStyle> {
    let mut want: std::collections::HashSet<String> = st.paras.iter().map(|p| p.style.clone()).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for d in &defs {
            if want.contains(&d.style.name)
                && let Some(b) = &d.style.based_on
                && want.insert(b.clone())
            {
                changed = true;
            }
        }
    }
    defs.into_iter().map(|d| d.style).filter(|s| s.name != designcraft_doc::story::BASIC_PARAGRAPH && want.contains(&s.name)).collect()
}

#[allow(clippy::cognitive_complexity)]
pub fn import(bytes: &[u8]) -> Result<Imported, ImportError> {
    let src = String::from_utf8_lossy(bytes);
    if !src.trim_start().starts_with("{\\rtf") {
        return Err(ImportError::Corrupt("not an RTF file".into()));
    }
    let mut st = Story::new(StoryId(0));
    let mut stack: Vec<State> = vec![State { uc: 1, ..Default::default() }];
    let mut cur_para = ParaAttrs::default();
    // Paragraph styles of the `\stylesheet` table and the `\sN` in force.
    let styles = parse_stylesheet(&src.chars().collect::<Vec<char>>());
    let style_names: std::collections::HashMap<u32, String> = styles.iter().map(|d| (d.idx, d.style.name.clone())).collect();
    let mut cur_style: Option<u32> = None;
    let mut next_tab_align = TabAlign::Left;
    // Paragraph properties are scoped to their group: a floating shape's `{\shp…\pard\ql\li0…}`
    // must not reset the indents of the paragraphs that follow it.
    let mut para_stack: Vec<(ParaAttrs, Option<u32>, TabAlign)> = Vec::new();
    let mut pending = 0usize; // characters still to skip after \uN
    let mut units = crate::Utf16Units::default();
    let mut buf = String::new();
    let mut fmt = stack[0].format();
    let mut default_doc_lang: Option<String> = None;
    let mut warnings = Vec::new();
    let flush = |st: &mut Story, buf: &mut String, fmt: &CharFormat| {
        if !buf.is_empty() {
            let len = st.len();
            st.insert_with(len, buf, fmt.clone());
            buf.clear();
        }
    };
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    // The first `{` opens the document group.
    let mut first = true;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '{' => {
                flush(&mut st, &mut buf, &fmt);
                let top = stack.last().cloned().unwrap_or_default();
                if !first {
                    stack.push(top);
                    para_stack.push((cur_para.clone(), cur_style, next_tab_align));
                }
                first = false;
                // A group is not a character: the `\uN` fallback skip ends here too.
                pending = 0;
                // `{\*\dest …}`: an ignorable destination.
                if chars.get(i + 1) == Some(&'\\')
                    && chars.get(i + 2) == Some(&'*')
                    && let Some(s) = stack.last_mut()
                {
                    s.skip = true;
                }
                i += 1;
            }
            '}' => {
                flush(&mut st, &mut buf, &fmt);
                if stack.len() > 1 {
                    stack.pop();
                    if let Some((para, style, tab)) = para_stack.pop() {
                        cur_para = para;
                        cur_style = style;
                        next_tab_align = tab;
                    }
                }
                // The `\uN` fallback skip ends with the group.
                pending = 0;
                fmt = stack.last().map(State::format).unwrap_or_default();
                i += 1;
            }
            '\\' => {
                let next = chars.get(i + 1).copied().unwrap_or(' ');
                if next == '\'' {
                    // \'hh: a Windows-1252 byte.
                    let hex: String = chars.iter().skip(i + 2).take(2).collect();
                    i += 4;
                    if pending > 0 {
                        pending -= 1;
                        continue;
                    }
                    if let Ok(b) = u8::from_str_radix(&hex, 16)
                        && !stack.last().is_some_and(|s| s.skip)
                    {
                        units.reset();
                        let ch = if b == 0xB7 && stack.last().is_some_and(|s| s.in_listtext) { '\u{2022}' } else { cp1252(b) };
                        buf.push(ch);
                    }
                    continue;
                }
                if !next.is_ascii_alphabetic() {
                    // Control symbols: \\ \{ \} \~ (nbsp) \- (soft hyphen) \_ (nb hyphen).
                    i += 2;
                    if pending > 0 {
                        pending -= 1;
                        continue;
                    }
                    let skip = stack.last().is_some_and(|s| s.skip);
                    if !skip {
                        match next {
                            '\\' | '{' | '}' => buf.push(next),
                            '~' => buf.push('\u{A0}'),
                            '-' => buf.push('\u{AD}'),
                            '_' => buf.push('\u{2011}'),
                            '\n' | '\r' => {
                                flush(&mut st, &mut buf, &fmt);
                                let pi = st.para_at(st.len());
                                let style = style_of(cur_style, &style_names);
                                if let Some(p) = st.paras.get_mut(pi) {
                                    p.para = cur_para.clone();
                                    p.style = style;
                                }
                                let len = st.len();
                                st.insert_with(len, "\n", fmt.clone());
                            }
                            _ => {}
                        }
                    }
                    continue;
                }
                // A control word with an optional numeric parameter and one delimiting space.
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_alphabetic() {
                    j += 1;
                }
                let word: String = chars[i + 1..j].iter().collect();
                let mut k = j;
                // A '-' only starts the parameter when a digit follows; otherwise it is text.
                let negative = chars.get(k) == Some(&'-') && chars.get(k + 1).is_some_and(char::is_ascii_digit);
                if negative || chars.get(k).is_some_and(char::is_ascii_digit) {
                    k += 1;
                    while k < chars.len() && chars[k].is_ascii_digit() {
                        k += 1;
                    }
                }
                let param: Option<i64> = chars[j..k].iter().collect::<String>().parse().ok();
                if k < chars.len() && chars[k] == ' ' {
                    k += 1;
                }
                i = k;
                // A control word is one character of the `\uN` fallback (a following `\u` is not).
                if pending > 0 && word != "u" {
                    pending -= 1;
                    continue;
                }
                let skip = stack.last().is_some_and(|s| s.skip);
                if SKIP.contains(&word.as_str()) {
                    flush(&mut st, &mut buf, &fmt);
                    if let Some(s) = stack.last_mut() {
                        s.skip = true;
                    }
                    if word == "pict" && !warnings.iter().any(|w: &String| w.starts_with("Pictures")) {
                        warnings.push("Pictures in the RTF file were not imported.".into());
                    }
                    continue;
                }
                if skip {
                    continue;
                }
                let on = param != Some(0);
                let mut restyle = |f: &dyn Fn(&mut State)| {
                    flush(&mut st, &mut buf, &fmt);
                    if let Some(s) = stack.last_mut() {
                        f(s);
                        fmt = s.format();
                    }
                };
                match word.as_str() {
                    "par" | "sect" => {
                        flush(&mut st, &mut buf, &fmt);
                        let pi = st.para_at(st.len());
                        let style = style_of(cur_style, &style_names);
                        if let Some(p) = st.paras.get_mut(pi) {
                            p.para = cur_para.clone();
                            p.style = style;
                        }
                        let len = st.len();
                        st.insert_with(len, "\n", fmt.clone());
                    }
                    "s" => {
                        // `\s0` is the default paragraph style.
                        cur_style = param.and_then(|p| u32::try_from(p).ok()).filter(|&n| n != 0);
                    }
                    "pard" => {
                        if !stack.last().is_some_and(|s| s.in_listtext) {
                            cur_para = ParaAttrs::default();
                            cur_style = None;
                            next_tab_align = TabAlign::Left;
                        }
                    }
                    "listtext" => {
                        if let Some(s) = stack.last_mut() {
                            s.in_listtext = true;
                        }
                    }
                    "li" | "lin" => {
                        cur_para.left_indent = param.map(|v| (v as f64 / 20.0).clamp(-10_000.0, 10_000.0));
                    }
                    "ri" | "rin" => {
                        cur_para.right_indent = param.map(|v| (v as f64 / 20.0).clamp(-10_000.0, 10_000.0));
                    }
                    "fi" => {
                        cur_para.first_line_indent = param.map(|v| (v as f64 / 20.0).clamp(-10_000.0, 10_000.0));
                    }
                    "sa" => {
                        cur_para.space_after = param.map(|v| (v as f64 / 20.0).clamp(0.0, 10_000.0));
                    }
                    "sb" => {
                        cur_para.space_before = param.map(|v| (v as f64 / 20.0).clamp(0.0, 10_000.0));
                    }
                    "ql" => cur_para.align = Some(Align::Left),
                    "qr" => cur_para.align = Some(Align::Right),
                    "qc" => cur_para.align = Some(Align::Center),
                    "qj" => cur_para.align = Some(Align::LeftJustified),
                    "qd" => cur_para.align = Some(Align::FullyJustified),
                    "tqr" => next_tab_align = TabAlign::Right,
                    "tqc" => next_tab_align = TabAlign::Center,
                    "tqdec" => next_tab_align = TabAlign::Char,
                    "tx" => {
                        if let Some(pos_twips) = param {
                            let pos_pt = (pos_twips as f64 / 20.0).clamp(0.0, designcraft_doc::MAX_TAB_POSITION);
                            let mut tabs = cur_para.tabs.take().unwrap_or_default();
                            if let Some(existing) = tabs.iter_mut().find(|t| (t.position - pos_pt).abs() < 0.01) {
                                existing.align = next_tab_align;
                            } else if tabs.len() < designcraft_doc::MAX_TAB_STOPS {
                                tabs.push(TabStop { position: pos_pt, align: next_tab_align, leader: String::new(), align_on: String::new() });
                                tabs.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap_or(std::cmp::Ordering::Equal));
                            }
                            cur_para.tabs = Some(tabs);
                            next_tab_align = TabAlign::Left;
                        }
                    }
                    "line" => buf.push(designcraft_doc::FORCED_LINE_BREAK),
                    "tab" => buf.push('\t'),
                    "page" => buf.push(designcraft_doc::PAGE_BREAK),
                    "column" => buf.push(designcraft_doc::COLUMN_BREAK),
                    "emdash" => buf.push('\u{2014}'),
                    "endash" => buf.push('\u{2013}'),
                    "bullet" => buf.push('\u{2022}'),
                    "lquote" => buf.push('\u{2018}'),
                    "rquote" => buf.push('\u{2019}'),
                    "ldblquote" => buf.push('\u{201C}'),
                    "rdblquote" => buf.push('\u{201D}'),
                    "b" => restyle(&|s| s.bold = on),
                    "i" => restyle(&|s| s.italic = on),
                    "ul" => restyle(&|s| s.underline = on),
                    "ulnone" => restyle(&|s| s.underline = false),
                    "strike" => restyle(&|s| s.strike = on),
                    // Half-points; sizes outside the engine's 0.1–1296 pt range fall back to the default.
                    "fs" => restyle(&|s| {
                        s.size = param.map(|p| p as f64 / 2.0).and_then(designcraft_doc::attrs::imported_type_size);
                    }),
                    "plain" => restyle(&|s| {
                        s.bold = false;
                        s.italic = false;
                        s.underline = false;
                        s.strike = false;
                        s.size = None;
                        s.language = default_doc_lang.clone();
                    }),
                    "deflang" | "deflangfe" => {
                        if let Some(p) = param {
                            let lang = lcid_to_language(p).map(str::to_string);
                            default_doc_lang = lang.clone();
                            restyle(&|s| {
                                if s.language.is_none() {
                                    s.language = lang.clone();
                                }
                            });
                        }
                    }
                    "lang" | "langfe" => {
                        let lang = param.and_then(lcid_to_language).map(str::to_string);
                        restyle(&|s| {
                            s.language = lang.clone();
                        });
                    }
                    "uc" => {
                        if let (Some(s), Some(n)) = (stack.last_mut(), param) {
                            s.uc = n.max(0) as usize;
                        }
                    }
                    "u" => {
                        if let Some(n) = param {
                            // RTF allows -32768..=65535; anything else is dropped.
                            if (-32768..=65535).contains(&n) {
                                let unit = if n < 0 { (n + 65536) as u32 } else { n as u32 };
                                if let Some(ch) = units.push(unit) {
                                    buf.push(ch);
                                }
                            }
                            pending = stack.last().map_or(1, |s| s.uc);
                        }
                    }
                    _ => {}
                }
            }
            '\r' | '\n' => i += 1,
            _ => {
                i += 1;
                if pending > 0 {
                    pending -= 1;
                    continue;
                }
                if !stack.last().is_some_and(|s| s.skip) {
                    units.reset();
                    buf.push(c);
                }
            }
        }
    }
    flush(&mut st, &mut buf, &fmt);
    // A trailing \par leaves an empty last paragraph.
    if st.text.ends_with('\n') {
        let n = st.len();
        st.delete(n - 1..n);
    }
    let pi = st.para_at(st.len());
    let style = style_of(cur_style, &style_names);
    if let Some(p) = st.paras.get_mut(pi) {
        p.para = cur_para.clone();
        p.style = style;
    }
    // A page or column break ends its paragraph.
    st.end_paragraphs_at_breaks();
    let para_styles = used_styles(styles, &st);
    Ok(Imported { story: st, para_styles, char_styles: vec![], warnings })
}

/// The paragraph style of a `\sN`, or `[Basic Paragraph]` for a style the table doesn't define.
fn style_of(idx: Option<u32>, names: &std::collections::HashMap<u32, String>) -> String {
    idx.and_then(|i| names.get(&i).cloned()).unwrap_or_else(|| designcraft_doc::story::BASIC_PARAGRAPH.into())
}

/// Windows-1252 byte → char.
fn cp1252(b: u8) -> char {
    const HI: [u16; 32] = [
        0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x8D, 0x017D, 0x8F, 0x90, 0x2018,
        0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x9D, 0x017E, 0x0178,
    ];
    if (0x80..0xA0).contains(&b) { char::from_u32(HI[(b - 0x80) as usize] as u32).unwrap_or('?') } else { b as char }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_of_range_unicode_escape_is_dropped() {
        assert_eq!(import(br"{\rtf1 a\u4294967361?b}").unwrap().story.text, "ab");
        assert_eq!(import(br"{\rtf1 a\u-4294967231?b}").unwrap().story.text, "ab");
    }

    #[test]
    fn imports_rtf_text_and_formatting() {
        let rtf = br"{\rtf1\ansi{\fonttbl{\f0 Times;}}{\colortbl;\red0\green0\blue0;}{\*\generator Test;}
\f0\fs24 Hello {\b bold} and {\i italic\i0  plain}\par
Caf\'e9 \u8364? price\tab tab\line next\par
}";
        let i = import(rtf).unwrap();
        let st = &i.story;
        st.check().unwrap();
        assert_eq!(st.text, "Hello bold and italic plain\nCafé € price\ttab\u{2028}next");
        let at = st.text.find("bold").unwrap();
        assert_eq!(st.format_after(at).over.font_style.as_deref(), Some("Bold"));
        assert_eq!(st.format_after(st.text.find("italic").unwrap()).over.font_style.as_deref(), Some("Italic"));
        assert_eq!(st.format_after(st.text.find("plain").unwrap()).over.font_style, None);
        assert_eq!(st.format_after(0).over.size, Some(12.0));
        assert!(!st.text.contains("Times"), "font table skipped");
    }

    /// `\fs` sizes outside the engine's 0.1–1296 pt character size range are not stored.
    #[test]
    fn font_size_out_of_range_is_not_stored() {
        let i = import(br"{\rtf1\ansi\fs0 A\fs-8 B\fs999999999 C\fs24 D}").unwrap();
        let st = &i.story;
        for ch in ["A", "B", "C"] {
            let size = st.format_after(st.text.find(ch).unwrap()).over.size;
            assert!(size.is_none_or(|s| (0.1..=1296.0).contains(&s)), "{ch}: {size:?}");
        }
        assert_eq!(st.format_after(st.text.find('D').unwrap()).over.size, Some(12.0));
    }

    /// A '-' right after a control word is text unless a digit follows it (then it is a negative
    /// parameter). It used to be swallowed.
    #[test]
    fn hyphen_after_control_word_without_digits_is_text() {
        let i = import(br"{\rtf1\ansi Rock\emdash-roll, x\tab-y, a\li-720 b}").unwrap();
        assert_eq!(i.story.text, "Rock\u{2014}-roll, x\t-y, ab");
    }

    /// A control word or control symbol is one character of the `\uN` fallback, so it is skipped
    /// like a plain character and does not eat the text after it.
    #[test]
    fn control_fallback_after_unicode_is_skipped() {
        let i = import(br"{\rtf1 \u8220\ldblquote Hi\u8221\rdblquote  x\u8212\emdash y}").unwrap();
        assert_eq!(i.story.text, "\u{201C}Hi\u{201D} x\u{2014}y");
        let i = import(br"{\rtf1 \u8220\{Hi}").unwrap();
        assert_eq!(i.story.text, "\u{201C}Hi");
        let i = import(br"{\rtf1 \u8220{\b Hi}}").unwrap();
        assert_eq!(i.story.text, "\u{201C}Hi");
    }

    /// Characters past U+FFFF are written as two `\uN` words, a UTF-16 surrogate pair (the form
    /// our own exporter and Word write). Each half was converted alone and dropped.
    #[test]
    fn surrogate_pairs_become_one_character() {
        let i = import(br"{\rtf1\ansi\uc1 A\u-10179?\u-8704?B \u-10174?\u-8265?\u30000?}").unwrap();
        assert_eq!(i.story.text, "A\u{1F600}B \u{20BB7}\u{7530}");
        // What the exporter writes comes back.
        let mut st = Story::new(StoryId(1));
        st.insert(0, "x \u{1F600} \u{20BB7}\u{7530} y");
        let doc = designcraft_doc::Document::new(&designcraft_doc::build::NewDocument::default());
        assert_eq!(import(crate::export::rtf(&doc, &st).as_bytes()).unwrap().story.text, st.text);
        // Half a pair is not a character: it is still left out.
        assert_eq!(import(br"{\rtf1 A\u-10179?B\u-8704?C}").unwrap().story.text, "ABC");
    }

    /// A `\uN` at the end of a group has no fallback character inside it. The skip count must
    /// end with the group, not eat the text after it.
    #[test]
    fn unicode_fallback_skip_ends_with_group() {
        assert_eq!(import(br"{\rtf1 {\b \u8364} AB}").unwrap().story.text, "\u{20AC} AB");
        assert_eq!(import(br"{\rtf1 {\uc2\b \u8364} ABC}").unwrap().story.text, "\u{20AC} ABC");
    }

    #[test]
    fn imports_rtf_paragraph_formatting_and_listtext() {
        let rtf = br"{\rtf1\ansi
\pard\ql\li360\ri180\fi-180\sb100\sa200\tx720 First paragraph\par
\pard\qc Centered paragraph\par
{\listtext\pard\plain 1.\tab}\pard\qj\fi-360\li720\sa160\tx720 Item one\par
{\listtext\pard\plain 2.\tab}Item two\par
\pard\ql Outside list\par
}";
        let imp = import(rtf).unwrap();
        let st = &imp.story;
        st.check().unwrap();
        assert_eq!(st.paras.len(), 5);

        // First paragraph
        let p0 = &st.paras[0].para;
        assert_eq!(p0.align, Some(Align::Left));
        assert_eq!(p0.left_indent, Some(18.0));
        assert_eq!(p0.right_indent, Some(9.0));
        assert_eq!(p0.first_line_indent, Some(-9.0));
        assert_eq!(p0.space_before, Some(5.0));
        assert_eq!(p0.space_after, Some(10.0));
        assert_eq!(p0.tabs.as_ref().map(|t| t.len()), Some(1));
        assert_eq!(p0.tabs.as_ref().unwrap()[0].position, 36.0);

        // Centered paragraph
        let p1 = &st.paras[1].para;
        assert_eq!(p1.align, Some(Align::Center));
        assert_eq!(p1.left_indent, None);

        // Item 1 (list item with hanging indent)
        let p2 = &st.paras[2].para;
        assert_eq!(p2.align, Some(Align::LeftJustified));
        assert_eq!(p2.left_indent, Some(36.0));
        assert_eq!(p2.first_line_indent, Some(-18.0));
        assert_eq!(p2.space_after, Some(8.0));
        assert_eq!(p2.tabs.as_ref().unwrap()[0].position, 36.0);

        // Item 2 (inherits list formatting despite inner \pard inside listtext)
        let p3 = &st.paras[3].para;
        assert_eq!(p3.align, Some(Align::LeftJustified));
        assert_eq!(p3.left_indent, Some(36.0));
        assert_eq!(p3.first_line_indent, Some(-18.0));
        assert_eq!(p3.space_after, Some(8.0));

        // Outside list (reset with \pard)
        let p4 = &st.paras[4].para;
        assert_eq!(p4.align, Some(Align::Left));
        assert_eq!(p4.left_indent, None);
        assert_eq!(p4.first_line_indent, None);
    }

    /// The `\stylesheet` table names paragraphs, and Word's lower-case names come out as the
    /// DesignCraft names the table of contents asks for.
    #[test]
    fn stylesheet_names_reach_paragraphs() {
        let rtf = br"{\rtf1\ansi{\stylesheet{\s1\b\fs40 heading 1;}{\s2\fs32 heading 2;}}
\pard\plain \s1 Heading one\par
\pard\plain \s2 Heading two\par
}";
        let i = import(rtf).unwrap();
        i.story.check().unwrap();
        assert_eq!(i.para_styles.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Heading 1", "Heading 2"]);
        let h1 = &i.para_styles[0];
        assert_eq!(h1.chars.font_style.as_deref(), Some("Bold"));
        assert_eq!(h1.chars.size, Some(20.0));
        assert_eq!(i.para_styles[1].chars.size, Some(16.0));
        assert_eq!(i.story.paras[0].style, "Heading 1");
        assert_eq!(i.story.paras[1].style, "Heading 2");
    }

    /// `\pard` resets the paragraph style; without it every later paragraph would keep the
    /// heading's style.
    #[test]
    fn pard_resets_paragraph_style() {
        let rtf = br"{\rtf1\ansi{\stylesheet{\s1 heading 1;}}
\pard\plain \s1 Heading\par
\pard\plain Body\par
}";
        let i = import(rtf).unwrap();
        assert_eq!(i.story.paras[0].style, "Heading 1");
        assert_eq!(i.story.paras[1].style, designcraft_doc::story::BASIC_PARAGRAPH);
    }

    /// The stylesheet is still a skipped destination: none of its words become body text.
    #[test]
    fn stylesheet_text_never_becomes_body_text() {
        let i = import(br"{\rtf1\ansi{\stylesheet{\s1\b\fs40 heading 1;\sbasedon0\snext0 Normal;}}\pard\plain Body\par}").unwrap();
        assert_eq!(i.story.text, "Body");
        for word in ["heading", "sbasedon", "Normal", "snext"] {
            assert!(!i.story.text.contains(word), "{word} leaked into the text");
        }
    }

    #[test]
    fn only_used_styles_are_returned() {
        let rtf = br"{\rtf1\ansi{\stylesheet{\s1\b heading 1;}{\s2 heading 2;}{\s3 heading 3;}}
\pard\plain \s2 Only this one\par
}";
        let i = import(rtf).unwrap();
        assert_eq!(i.para_styles.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Heading 2"]);
    }

    /// A style only used as a base is imported too, so its formatting survives.
    #[test]
    fn based_on_chain_is_kept() {
        let rtf = br"{\rtf1\ansi{\stylesheet{\s1\fs40 Base style;}{\s2\sbasedon1\b heading 2;}}
\pard\plain \s2 Both\par
}";
        let i = import(rtf).unwrap();
        assert_eq!(i.para_styles.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Base style", "Heading 2"]);
        assert_eq!(i.para_styles[1].based_on.as_deref(), Some("Base style"));
    }

    #[test]
    fn listtext_hex_bullet_becomes_unicode_bullet() {
        let rtf = br"{\rtf1\ansi {\listtext\pard\plain \'b7\tab}Bullet item\par}";
        let imp = import(rtf).unwrap();
        assert_eq!(imp.story.text, "\u{2022}\tBullet item");
    }

    /// Paragraph properties are group-scoped: Word wraps a floating picture in its own group whose
    /// `\pard\ql\li0` must not reset the indents of the paragraphs that follow (the `B01.rtf`
    /// signature block lost its 170 pt centre indent this way).
    #[test]
    fn a_group_pard_does_not_leak_into_following_paragraphs() {
        let rtf = br"{\rtf1\ansi
\pard\qc\li3402 Date\par
{\shp\pard\ql\li0 Shape\par}
Kepala BPS\par
}";
        let imp = import(rtf).unwrap();
        let st = &imp.story;
        st.check().unwrap();
        assert_eq!(st.paras[0].para.align, Some(Align::Center));
        assert_eq!(st.paras[0].para.left_indent, Some(170.1));
        assert_eq!(st.paras[1].para.align, Some(Align::Left), "the shape's own \\pard applies inside its group");
        assert_eq!(st.paras[2].para.align, Some(Align::Center), "the shape's \\pard must not leak into the following paragraph");
        assert_eq!(st.paras[2].para.left_indent, Some(170.1));
    }

    #[test]
    fn imports_rtf_language_tags() {
        // Document with deflang14345 (Indonesian) and a span with lang1033 (English)
        let rtf = br"{\rtf1\ansi\deflang14345
\pard Indonesian text {\lang1033 English text} back to default\par
}";
        let imp = import(rtf).unwrap();
        let st = &imp.story;
        st.check().unwrap();

        // 1. Indonesian runs carry language "Indonesian"
        let f_start = st.format_after(0);
        assert_eq!(f_start.over.language.as_deref(), Some("Indonesian"));

        // 2. English span carries "English: USA"
        let en_pos = st.text.find("English").unwrap();
        let f_en = st.format_after(en_pos);
        assert_eq!(f_en.over.language.as_deref(), Some("English: USA"));

        // 3. Post-group text reverts to document default "Indonesian"
        let back_pos = st.text.find("back to").unwrap();
        let f_back = st.format_after(back_pos);
        assert_eq!(f_back.over.language.as_deref(), Some("Indonesian"));

        // Also test LCID 1057 (canonical Indonesian) and 3081 (Australian)
        let rtf2 = br"{\rtf1\ansi\deflang1057
\pard Halo {\lang3081 G'day} Dunia\par
}";
        let imp2 = import(rtf2).unwrap();
        let st2 = &imp2.story;
        assert_eq!(st2.format_after(0).over.language.as_deref(), Some("Indonesian"));
        let gday_pos = st2.text.find("G'day").unwrap();
        assert_eq!(st2.format_after(gday_pos).over.language.as_deref(), Some("English: Australian"));
        let dunia_pos = st2.text.find("Dunia").unwrap();
        assert_eq!(st2.format_after(dunia_pos).over.language.as_deref(), Some("Indonesian"));
    }
}
