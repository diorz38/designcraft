//! RTF → story: text, paragraphs, line breaks, tabs, bold/italic/underline/strikethrough, size,
//! Unicode (`\uN`) and hex (`\'hh`) characters. Destinations that aren't body text (font and
//! colour tables, stylesheet, info, pictures, `\*` groups) are skipped.

use designcraft_doc::{Align, CharAttrs, CharFormat, ParaAttrs, Story, StoryId, TabAlign, TabStop};

use crate::{ImportError, Imported};

#[derive(Clone, Default)]
struct State {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    size: Option<f64>,
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
                ..Default::default()
            },
        }
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

pub fn import(bytes: &[u8]) -> Result<Imported, ImportError> {
    let src = String::from_utf8_lossy(bytes);
    if !src.trim_start().starts_with("{\\rtf") {
        return Err(ImportError::Corrupt("not an RTF file".into()));
    }
    let mut st = Story::new(StoryId(0));
    let mut stack: Vec<State> = vec![State { uc: 1, ..Default::default() }];
    let mut cur_para = ParaAttrs::default();
    let mut next_tab_align = TabAlign::Left;
    let mut pending = 0usize; // characters still to skip after \uN
    let mut units = crate::Utf16Units::default();
    let mut buf = String::new();
    let mut fmt = stack[0].format();
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
                }
                first = false;
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
                        let ch = if b == 0xB7 && stack.last().is_some_and(|s| s.in_listtext) {
                            '\u{2022}'
                        } else {
                            cp1252(b)
                        };
                        buf.push(ch);
                    }
                    continue;
                }
                if !next.is_ascii_alphabetic() {
                    // Control symbols: \\ \{ \} \~ (nbsp) \- (soft hyphen) \_ (nb hyphen).
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
                                if let Some(p) = st.paras.get_mut(pi) {
                                    p.para = cur_para.clone();
                                }
                                let len = st.len();
                                st.insert_with(len, "\n", fmt.clone());
                            }
                            _ => {}
                        }
                    }
                    i += 2;
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
                        if let Some(p) = st.paras.get_mut(pi) {
                            p.para = cur_para.clone();
                        }
                        let len = st.len();
                        st.insert_with(len, "\n", fmt.clone());
                    }
                    "pard" => {
                        if !stack.last().is_some_and(|s| s.in_listtext) {
                            cur_para = ParaAttrs::default();
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
                                tabs.push(TabStop {
                                    position: pos_pt,
                                    align: next_tab_align,
                                    leader: String::new(),
                                    align_on: String::new(),
                                });
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
                    "fs" => restyle(&|s| s.size = param.map(|p| p as f64 / 2.0)),
                    "plain" => restyle(&|s| {
                        s.bold = false;
                        s.italic = false;
                        s.underline = false;
                        s.strike = false;
                        s.size = None;
                    }),
                    "uc" => {
                        if let (Some(s), Some(n)) = (stack.last_mut(), param) {
                            s.uc = n.max(0) as usize;
                        }
                    }
                    "u" => {
                        if let Some(n) = param {
                            let unit = if n < 0 { (n + 65536) as u32 } else { n as u32 };
                            if let Some(ch) = units.push(unit) {
                                buf.push(ch);
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
    if let Some(p) = st.paras.get_mut(pi) {
        p.para = cur_para.clone();
    }
    // A page or column break ends its paragraph.
    st.end_paragraphs_at_breaks();
    Ok(Imported { story: st, para_styles: vec![], char_styles: vec![], warnings })
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

    /// A '-' right after a control word is text unless a digit follows it (then it is a negative
    /// parameter). It used to be swallowed.
    #[test]
    fn hyphen_after_control_word_without_digits_is_text() {
        let i = import(br"{\rtf1\ansi Rock\emdash-roll, x\tab-y, a\li-720 b}").unwrap();
        assert_eq!(i.story.text, "Rock\u{2014}-roll, x\t-y, ab");
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

    #[test]
    fn listtext_hex_bullet_becomes_unicode_bullet() {
        let rtf = br"{\rtf1\ansi {\listtext\pard\plain \'b7\tab}Bullet item\par}";
        let imp = import(rtf).unwrap();
        assert_eq!(imp.story.text, "\u{2022}\tBullet item");
    }
}

