//! Chapter-review summarizer: deterministic statistics from the catalog's table CSVs, the
//! narrative prompt and the RTF the document receives. The model call lives in `translate`
//! under the `groq` feature; nothing here touches the network.

use std::sync::atomic::{AtomicBool, Ordering};

use designcraft_translate as translate;

use super::*;

/// One numeric column of a table.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnStat {
    pub name: String,
    pub sum: f64,
    pub max: f64,
    pub min: f64,
}

/// What the narrative is built from: one table's numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct TableSummary {
    pub table_title: String,
    pub column_stats: Vec<ColumnStat>,
    /// Label and value of the row holding the first numeric column's maximum/minimum.
    pub top_row: Option<(String, f64)>,
    pub bottom_row: Option<(String, f64)>,
}

/// Parse one table cell. Indonesian tables write thousands with `.` and decimals with `,`
/// (`1.234,56`); English exports do the opposite (`1,234.56`), so when both appear the last
/// one is the decimal separator. A lone `.` grouping exactly three digits is thousands
/// (`1.234` = 1234) unless the whole part is `0` (`0.125` = 0.125); a lone `,` is decimal,
/// as Indonesian reads it. `(1.234)` is negative, `Rp` is dropped, `45%` becomes 0.45, and
/// a dash or a word (e.g. the `-` of an empty cell) is not a number.
pub fn parse_number(text: &str) -> Option<f64> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let (mut negative, body) = match t.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        Some(inner) => (true, inner.trim()),
        None => (false, t),
    };
    let body = match body.strip_prefix('-') {
        Some(rest) => {
            negative = true;
            rest.trim_start()
        }
        None => body.strip_prefix('+').map(str::trim_start).unwrap_or(body),
    };
    let body = body.strip_prefix("Rp").or_else(|| body.strip_prefix("rp")).map(str::trim_start).unwrap_or(body);
    let (percent, body) = match body.strip_suffix('%') {
        Some(rest) => (true, rest.trim_end()),
        None => (false, body),
    };
    let body: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    if body.is_empty() {
        return None;
    }

    let (decimal, thousands) = match (body.rfind('.'), body.rfind(',')) {
        (Some(d), Some(c)) if d > c => (Some('.'), Some(',')),
        (Some(_), Some(_)) => (Some(','), Some('.')),
        (Some(_), None) if groups_of_three(&body, '.') => (None, Some('.')),
        (Some(_), None) => (Some('.'), None),
        (None, Some(_)) => (Some(','), None),
        (None, None) => (None, None),
    };

    let (whole, fraction) = match decimal.and_then(|sep| body.rsplit_once(sep)) {
        Some((whole, fraction)) => (whole, fraction),
        None => (body.as_str(), ""),
    };
    let whole = match thousands {
        Some(sep) => {
            let groups: Vec<&str> = whole.split(sep).collect();
            let valid = groups.first().is_some_and(|g| !g.is_empty() && g.len() <= 3)
                && groups.iter().all(|g| !g.is_empty() && g.chars().all(|c| c.is_ascii_digit()))
                && groups.iter().skip(1).all(|g| g.len() == 3);
            if !valid {
                return None;
            }
            groups.concat()
        }
        None => whole.to_string(),
    };
    if whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut value: f64 = if fraction.is_empty() { whole.parse().ok()? } else { format!("{whole}.{fraction}").parse().ok()? };
    if percent {
        value /= 100.0;
    }
    if negative {
        value = -value;
    }
    Some(value)
}

/// `1.234` / `1.234.567`: every group after the first has exactly three digits.
fn groups_of_three(body: &str, sep: char) -> bool {
    let groups: Vec<&str> = body.split(sep).collect();
    groups.len() >= 2
        && !groups[0].is_empty()
        && groups[0].len() <= 3
        && groups[0] != "0"
        && groups.iter().skip(1).all(|g| g.len() == 3 && g.chars().all(|c| c.is_ascii_digit()))
}

/// The total/subtotal row of an Indonesian table repeats the column figures; leaving it in
/// would double every sum.
fn is_summary_row(label: &str) -> bool {
    let l = label.trim().to_ascii_lowercase();
    l.starts_with("jumlah") || l.starts_with("total") || l.contains("subjumlah") || l.starts_with("sub total")
}

/// Statistics for one parsed table. The first row holding a non-numeric cell is the header;
/// rows with no numeric cell and the total rows are not data. `None` when the table carries
/// no numbers at all (a layout artifact, a division title, …).
pub fn analyze_table(matrix: &[Vec<String>], title: &str) -> Option<TableSummary> {
    let rows: Vec<&Vec<String>> = matrix.iter().filter(|r| r.iter().any(|c| !c.trim().is_empty())).collect();
    let header_idx = rows.iter().position(|r| {
        r.iter().any(|c| {
            let t = c.trim();
            !t.is_empty() && parse_number(t).is_none()
        })
    })?;
    let header = rows.get(header_idx)?;

    let mut data: Vec<(String, Vec<Option<f64>>)> = Vec::new();
    for row in rows.iter().skip(header_idx + 1) {
        let label = row.first().map(|c| c.trim().to_string()).unwrap_or_default();
        if is_summary_row(&label) {
            continue;
        }
        let values: Vec<Option<f64>> = row.iter().map(|c| parse_number(c)).collect();
        if values.iter().any(Option::is_some) {
            data.push((label, values));
        }
    }
    if data.is_empty() {
        return None;
    }

    let columns = data.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
    let mut column_stats = Vec::new();
    for ci in 1..columns {
        let values: Vec<f64> = data.iter().filter_map(|(_, v)| v.get(ci).copied().flatten()).collect();
        if values.is_empty() {
            continue;
        }
        let name = header.get(ci).map(|c| c.trim()).filter(|c| !c.is_empty()).map(str::to_string).unwrap_or_else(|| format!("Column {}", ci + 1));
        column_stats.push(ColumnStat {
            name,
            sum: values.iter().sum(),
            max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            min: values.iter().copied().fold(f64::INFINITY, f64::min),
        });
    }
    if column_stats.is_empty() {
        return None;
    }

    let key_col = (1..columns).find(|&ci| data.iter().any(|(_, v)| v.get(ci).is_some_and(Option::is_some)))?;
    let mut top_row: Option<(String, f64)> = None;
    let mut bottom_row: Option<(String, f64)> = None;
    for (label, values) in &data {
        let Some(value) = values.get(key_col).copied().flatten() else { continue };
        if top_row.as_ref().is_none_or(|(_, t)| value > *t) {
            top_row = Some((label.clone(), value));
        }
        if bottom_row.as_ref().is_none_or(|(_, b)| value < *b) {
            bottom_row = Some((label.clone(), value));
        }
    }

    Some(TableSummary { table_title: title.trim().to_string(), column_stats, top_row, bottom_row })
}

/// Every catalog row that resolves to a table CSV, deduplicated by path, in catalog order.
pub fn collect_table_summaries(st: &PubgenState) -> Vec<TableSummary> {
    let file_col = effective_col(st.card1_file_col, &st.card1_headers, &["file", "filename"], 3);
    let title_col = effective_col(st.card1_title_col, &st.card1_headers, &["title", "name", "headline", "chapter"], 2);
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for row in &st.card1_records {
        let title = row.get(title_col).map(|c| c.trim()).unwrap_or("");
        let file_val = row.get(file_col).map(|c| c.trim()).unwrap_or("");
        if file_val.is_empty() || file_val.to_lowercase().ends_with(".rtf") {
            continue;
        }
        let Some(path) = resolve_catalog_related_file(&st.card1_path, file_val, "csv") else { continue };
        if !seen.insert(path.clone()) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else { continue };
        let matrix = parse_csv_content_with_delim(&content, detect_delimiter(&content));
        let title = if title.is_empty() { file_val } else { title };
        if let Some(summary) = analyze_table(&matrix, title) {
            out.push(summary);
        }
    }
    out
}

/// The role a narrative block can carry (kept in sync with `role_style`).
pub const NARRATIVE_ROLES: [&str; 4] = ["heading1", "heading2", "lead", "body"];

/// Role → (DesignCraft paragraph style, `\sN` index). Unknown roles fall back to body text.
fn role_style(role: &str) -> (&'static str, u32) {
    match role.trim().to_ascii_lowercase().as_str() {
        "heading1" | "h1" => ("Heading 1", 1),
        "heading2" | "h2" => ("Heading 2", 2),
        "lead" => ("Lead Paragraph", 3),
        _ => ("Normal", 0),
    }
}

/// `(system, user)` for the drafting call: the chapter title plus every table's statistics,
/// and the JSON block contract. The model never writes RTF — Rust builds it.
pub fn build_narrative_prompt(summaries: &[TableSummary], chapter: &str) -> (String, String) {
    let system = "You are the editor of an English BPS statistical publication. Write the review \
(narrative) for one chapter from the table statistics given. Use only the numbers provided: never \
invent, round or recompute a figure. Write a short lead paragraph that summarises the chapter, then \
one section per table with a `heading2` block naming the table in English. Use official BPS \
terminology. Return only JSON of the form \
{\"blocks\":[{\"role\":\"lead\"|\"heading2\"|\"body\",\"text\":\"...\"}]}, with the lead block first, \
no markdown and no commentary."
        .to_string();
    let tables: Vec<Value> = summaries
        .iter()
        .map(|s| {
            let top = s.top_row.as_ref().map(|(label, value)| json!({"label": label, "value": value}));
            let bottom = s.bottom_row.as_ref().map(|(label, value)| json!({"label": label, "value": value}));
            json!({
                "title": s.table_title,
                "columns": s.column_stats.iter().map(|c| json!({"name": c.name, "sum": c.sum, "max": c.max, "min": c.min})).collect::<Vec<_>>(),
                "top_row": top,
                "bottom_row": bottom,
            })
        })
        .collect();
    (system, json!({"chapter": chapter.trim(), "tables": tables}).to_string())
}

/// RTF for the placed narrative: one paragraph per block line, each with its style index in
/// the stylesheet table so the importer (and `file.place`) carries the styles in.
pub fn build_narrative_rtf(blocks: &[(String, String)]) -> String {
    let mut body = String::new();
    for (role, text) in blocks {
        let (_, mask) = role_style(role);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            body.push_str(&format!("\\pard\\plain\\s{mask} {}\\par\n", ai::escape_rtf_text(line)));
        }
    }
    format!(
        "{{\\rtf1\\ansi\\deff0{{\\fonttbl{{\\f0 Arial;}}}}\
         {{\\stylesheet{{\\s0 Normal;}}{{\\s1 Heading 1;}}{{\\s2 Heading 2;}}{{\\s3 Lead Paragraph;}}}}{body}}}"
    )
}

/// Place the narrative into the review page's English body frame: writes the RTF, then lets
/// `file.place` import it with its paragraph styles (one undo step).
#[cfg(not(target_arch = "wasm32"))]
pub fn apply_narrative(app: &mut DesignApp, target: &ai::PageTarget, blocks: &[(String, String)]) -> Result<usize, String> {
    let path = ai::temp_dir().join(format!("narrative-{}.rtf", target.page_idx));
    std::fs::write(&path, build_narrative_rtf(blocks)).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    app.run("file.place", json!({"path": path.to_string_lossy(), "frame": target.en_body_fid, "autoflow": true}))
        .map_err(|e| format!("could not place the narrative: {e}"))?;
    Ok(blocks.len())
}

/// Terms appended to the drafting prompt (bounded like the translation prompt).
const TERMS_IN_PROMPT: usize = 200;

/// The drafting system prompt plus the terminology pairs the model must use.
pub fn narrative_system_prompt(system: &str, glossary: &[(String, String)]) -> String {
    if glossary.is_empty() {
        return system.to_string();
    }
    let pairs: Vec<String> = glossary.iter().take(TERMS_IN_PROMPT).map(|(id, en)| format!("{id} = {en}")).collect();
    format!("{system}\n\nOfficial terminology you MUST use: {}.", pairs.join("; "))
}

/// Parse the model's `{"blocks":[{"role","text"}]}` reply. One triple-backtick fence is
/// stripped; anything else — another shape, an unknown role, empty text, prose around the
/// object — is an error. Never salvage prose into a document.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn parse_blocks(content: &str) -> Result<Vec<(String, String)>, String> {
    let fence = "`".repeat(3);
    let mut s = content.trim();
    if let Some(rest) = s.strip_prefix(&fence) {
        let body = rest.split_once('\n').map(|(_, body)| body).unwrap_or(rest);
        s = body.trim_end();
        if let Some(without) = s.strip_suffix(&fence) {
            s = without.trim_end();
        }
    }
    let v: Value = serde_json::from_str(s).map_err(|e| format!("the model reply is not JSON: {e}"))?;
    let arr = v.get("blocks").and_then(Value::as_array).ok_or("the model reply has no `blocks` array")?;
    if arr.is_empty() {
        return Err("the model returned no blocks".into());
    }
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let role = item.get("role").and_then(Value::as_str).map(str::trim).unwrap_or("").to_ascii_lowercase();
        if !NARRATIVE_ROLES.contains(&role.as_str()) {
            return Err(format!("unknown block role `{role}`"));
        }
        let text = item.get("text").and_then(Value::as_str).map(str::trim).unwrap_or("");
        if text.is_empty() {
            return Err("a block has no text".into());
        }
        out.push((role, text.to_string()));
    }
    Ok(out)
}

/// Draft the review blocks from the statistics. One parse failure retries with a terser
/// prompt; a second failure is an error the dialog shows (no block is ever guessed).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn draft_blocks(
    t: &translate::OnlineTranslator,
    summaries: &[TableSummary],
    chapter: &str,
    glossary: &[(String, String)],
    cancel: &AtomicBool,
) -> Result<Vec<(String, String)>, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    let (system, user) = build_narrative_prompt(summaries, chapter);
    let system = narrative_system_prompt(&system, glossary);
    let raw = t.complete(&system, &user).map_err(|e| e.to_string())?;
    let first = match parse_blocks(&raw) {
        Ok(blocks) => return Ok(blocks),
        Err(e) => e,
    };
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    let retry_system =
        format!("{system}\n\nReturn ONLY the JSON object {{\"blocks\":[{{\"role\":\"...\",\"text\":\"...\"}}]}} — no prose, no markdown fences.");
    let raw = t.complete(&retry_system, &user).map_err(|e| format!("{first}; retry failed: {e}"))?;
    parse_blocks(&raw).map_err(|e| format!("{first}; retry: {e}"))
}

/// Whether any catalog row names a table CSV — the cheap enable test (no disk I/O).
pub fn has_catalog_tables(st: &PubgenState) -> bool {
    let file_col = effective_col(st.card1_file_col, &st.card1_headers, &["file", "filename"], 3);
    st.card1_records
        .iter()
        .any(|row| row.get(file_col).map(|c| c.trim()).filter(|c| !c.is_empty()).is_some_and(|c| !c.to_lowercase().ends_with(".rtf")))
}

// ---------------------------------------------------------------------------
// Background drafting job
// ---------------------------------------------------------------------------

/// One background narrative draft, polled frame by frame like `TranslateJob`.
#[derive(Clone, Debug)]
pub struct NarrativeJob {
    #[cfg(not(target_arch = "wasm32"))]
    rx: std::sync::Arc<std::sync::Mutex<std::sync::mpsc::Receiver<NarrativeMsg>>>,
    #[cfg(not(target_arch = "wasm32"))]
    cancel: std::sync::Arc<AtomicBool>,
    pub finished: bool,
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
enum NarrativeMsg {
    Done(Vec<(String, String)>),
    Failed(String),
}

impl NarrativeJob {
    /// Spawn the drafting worker. Native only: the button stays disabled on wasm.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn(
        provider: &str,
        key: &str,
        model: &str,
        summaries: Vec<TableSummary>,
        chapter: String,
        glossary: Vec<(String, String)>,
        ctx: egui::Context,
    ) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<NarrativeMsg>();
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let cancel_thread = cancel.clone();
        let provider = provider.to_string();
        let key = key.to_string();
        let model = model.to_string();
        let tx_thread = tx.clone();
        let spawned = std::thread::Builder::new().name("designcraft-narrative".into()).spawn(move || {
            let outcome = if provider == "groq" {
                let t = translate::OnlineTranslator::new(&key, &model);
                draft_blocks(&t, &summaries, &chapter, &glossary, &cancel_thread)
            } else {
                Err("the chapter review needs the Groq backend — configure it in AI Translator".to_string())
            };
            let _ = match outcome {
                Ok(blocks) => tx_thread.send(NarrativeMsg::Done(blocks)),
                Err(e) => tx_thread.send(NarrativeMsg::Failed(e)),
            };
            ctx.request_repaint();
        });
        if spawned.is_err() {
            let _ = tx.send(NarrativeMsg::Failed("could not start the drafting thread".into()));
        }
        Self { rx: std::sync::Arc::new(std::sync::Mutex::new(rx)), cancel, finished: false }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn(
        _provider: &str,
        _key: &str,
        _model: &str,
        _summaries: Vec<TableSummary>,
        _chapter: String,
        _glossary: Vec<(String, String)>,
        _ctx: egui::Context,
    ) -> Self {
        Self { finished: true }
    }

    pub fn cancel(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Drain the worker's result into `st.narrative_preview`; true when drafting finished OK.
    pub fn poll(&mut self, st: &mut PubgenState) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = st;
            false
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let rx = self.rx.clone();
            let mut finished_ok = false;
            while let Ok(msg) = rx.lock().unwrap_or_else(std::sync::PoisonError::into_inner).try_recv() {
                match msg {
                    NarrativeMsg::Done(blocks) => {
                        st.narrative_preview = Some(blocks);
                        st.header_status = Some(("Chapter review drafted — review the blocks, then Apply.".into(), true));
                        self.finished = true;
                        finished_ok = true;
                    }
                    NarrativeMsg::Failed(e) => {
                        self.finished = true;
                        st.header_status = Some((format!("Chapter review failed: {e}"), false));
                    }
                }
            }
            finished_ok
        }
    }
}

/// The preview modal: every block in order, Apply (one undo step via `file.place`) or Discard.
pub fn preview_ui(app: &mut DesignApp, st: &mut PubgenState, ui: &mut egui::Ui) {
    if st.narrative_preview.is_none() {
        return;
    }
    let mut open = true;
    let mut apply = false;
    let mut discard = false;
    egui::Window::new(egui::RichText::new("AI Chapter Review").size(15.0))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .show(ui.ctx(), |ui| {
            if let Some(blocks) = &st.narrative_preview {
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for (role, text) in blocks {
                        ui.label(egui::RichText::new(role.to_uppercase()).size(9.0).strong());
                        ui.label(egui::RichText::new(text).size(11.5));
                        ui.add_space(5.0);
                    }
                });
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Apply to page").clicked() {
                    apply = true;
                }
                if ui.button("Discard").clicked() {
                    discard = true;
                }
                if st.narrative_job.as_ref().is_some_and(|j| !j.finished)
                    && ui.button("Cancel").clicked()
                    && let Some(job) = &st.narrative_job
                {
                    job.cancel();
                }
            });
        });
    if !open || discard {
        st.narrative_preview = None;
        st.narrative_target = None;
        return;
    }
    if apply {
        let blocks = st.narrative_preview.clone().unwrap_or_default();
        let Some(target) = st.narrative_target.clone() else {
            st.header_status = Some(("No page selected — select the review page first.".into(), false));
            return;
        };
        #[cfg(not(target_arch = "wasm32"))]
        match apply_narrative(app, &target, &blocks) {
            Ok(n) => st.header_status = Some((format!("{n} block(s) applied to the page."), true)),
            Err(e) => st.header_status = Some((e, false)),
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (app, target, blocks);
            st.header_status = Some(("Applying a narrative is not available in the web build.".into(), false));
        }
        st.narrative_preview = None;
        st.narrative_target = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_indonesian_and_english_number_formats() {
        assert_eq!(parse_number("1.234,56"), Some(1234.56));
        assert_eq!(parse_number("1,234.56"), Some(1234.56));
        assert_eq!(parse_number("1.234"), Some(1234.0));
        assert_eq!(parse_number("1.234.567"), Some(1234567.0));
        assert_eq!(parse_number("3.14"), Some(3.14));
        assert_eq!(parse_number("0.125"), Some(0.125));
        assert_eq!(parse_number("1,5"), Some(1.5));
        assert_eq!(parse_number("(1.234)"), Some(-1234.0));
        assert_eq!(parse_number("-12,5"), Some(-12.5));
        assert_eq!(parse_number("Rp1.234"), Some(1234.0));
        assert_eq!(parse_number("45%"), Some(0.45));
        assert_eq!(parse_number(" 2024 "), Some(2024.0));
    }

    #[test]
    fn rejects_what_is_not_a_number() {
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("-"), None);
        assert_eq!(parse_number("–"), None);
        assert_eq!(parse_number("n/a"), None);
        assert_eq!(parse_number("1.2.3"), None);
        assert_eq!(parse_number("1..234"), None);
        assert_eq!(parse_number("(1.234"), None);
        assert_eq!(parse_number("12a"), None);
    }

    fn table() -> Vec<Vec<String>> {
        vec![
            vec!["Desa/Kelurahan".into(), "Luas (km2)".into(), "Penduduk".into()],
            vec!["Warungbambu".into(), "1.234,5".into(), "12.000".into()],
            vec!["Karawang Kulon".into(), "3.500,25".into(), "8.500".into()],
            vec!["Jumlah".into(), "4.734,75".into(), "20.500".into()],
        ]
    }

    #[test]
    fn analyses_columns_and_finds_the_extreme_rows() {
        let summary = analyze_table(&table(), "Luas Wilayah menurut Desa").expect("table has numbers");
        assert_eq!(summary.table_title, "Luas Wilayah menurut Desa");
        assert_eq!(summary.column_stats.len(), 2);
        assert_eq!(summary.column_stats[0].name, "Luas (km2)");
        assert!((summary.column_stats[0].sum - 4734.75).abs() < 1e-9, "{}", summary.column_stats[0].sum);
        assert_eq!(summary.column_stats[0].max, 3500.25);
        assert_eq!(summary.column_stats[0].min, 1234.5);
        assert_eq!(summary.top_row, Some(("Karawang Kulon".to_string(), 3500.25)));
        assert_eq!(summary.bottom_row, Some(("Warungbambu".to_string(), 1234.5)));
        assert_eq!(summary.column_stats[1].sum, 20500.0, "the total row is not counted twice");
    }

    #[test]
    fn malformed_or_empty_tables_have_no_summary() {
        assert_eq!(analyze_table(&[], "x"), None);
        assert_eq!(analyze_table(&[vec!["Hanya Judul".into()]], "x"), None);
        assert_eq!(analyze_table(&[vec!["A".into(), "B".into()], vec!["x".into(), "-".into()]], "x"), None);
    }

    #[test]
    fn prompt_carries_every_table_and_stat() {
        let summary = analyze_table(&table(), "Luas Wilayah menurut Desa").expect("summary");
        let (system, user) = build_narrative_prompt(&[summary], "Geografi");
        assert!(system.contains("blocks"), "{system}");
        assert!(system.contains("heading2"), "{system}");
        assert!(user.contains("Luas Wilayah menurut Desa"), "{user}");
        assert!(user.contains("Luas (km2)"), "{user}");
        assert!(user.contains("4734.75"), "{user}");
        assert!(user.contains("Warungbambu"), "{user}");
        assert!(user.contains("\"chapter\":\"Geografi\""), "{user}");
    }

    #[test]
    fn narrative_rtf_imports_with_the_named_styles() {
        let blocks = vec![
            ("lead".to_string(), "The chapter opens with the land area.".to_string()),
            ("heading2".to_string(), "Land area by village".to_string()),
            ("body".to_string(), "Warungbambu is the smallest village {with braces} \\ and a slash.".to_string()),
            ("heading1".to_string(), "Closing remarks".to_string()),
        ];
        let rtf = build_narrative_rtf(&blocks);
        let imported = designcraft_textimport::import("narrative.rtf", rtf.as_bytes()).expect("valid RTF");
        let styles: Vec<&str> = imported.story.paras.iter().map(|p| p.style.as_str()).collect();
        assert_eq!(styles, ["Lead Paragraph", "Heading 2", designcraft_doc::story::BASIC_PARAGRAPH, "Heading 1"]);
        assert!(imported.story.text.contains("{with braces} \\ and a slash."), "{}", imported.story.text);
        assert!(imported.para_styles.iter().any(|s| s.name == "Lead Paragraph"), "style reaches the import");
    }

    #[test]
    fn parses_the_block_contract_including_a_json_fence() {
        let reply = "```json\n{\"blocks\":[{\"role\":\"lead\",\"text\":\" One. \"},{\"role\":\"heading2\",\"text\":\"Two\"}]}\n```";
        assert_eq!(parse_blocks(reply), Ok(vec![("lead".to_string(), "One.".to_string()), ("heading2".to_string(), "Two".to_string())]));
    }

    #[test]
    fn rejects_prose_unknown_roles_and_empty_text() {
        assert!(parse_blocks("Sure! Here is the review: {\"blocks\":[]}").is_err());
        assert!(parse_blocks("{\"blocks\":[]}").is_err());
        assert!(parse_blocks("{\"paragraphs\":[]}").is_err());
        assert!(parse_blocks("{\"blocks\":[{\"role\":\"title\",\"text\":\"x\"}]}").is_err());
        assert!(parse_blocks("{\"blocks\":[{\"role\":\"body\",\"text\":\"   \"}]}").is_err());
    }

    #[test]
    fn glossary_terms_join_the_narrative_prompt() {
        let terms = vec![("Kecamatan".to_string(), "Subdistrict".to_string())];
        let prompt = narrative_system_prompt("base", &terms);
        assert!(prompt.contains("Kecamatan = Subdistrict"), "{prompt}");
        assert_eq!(narrative_system_prompt("base", &[]), "base");
        let many: Vec<(String, String)> = (0..TERMS_IN_PROMPT + 1).map(|i| (format!("id{i}"), format!("en{i}"))).collect();
        let capped = narrative_system_prompt("base", &many);
        assert!(capped.contains("id199 = en199"));
        assert!(!capped.contains("id200 = en200"), "the 201st term is dropped");
    }

    #[test]
    fn parsing_a_cancelled_draft_stops_before_the_request() {
        let cancel = AtomicBool::new(true);
        let t = translate::OnlineTranslator::new("", "");
        let err = draft_blocks(&t, &[], "Geografi", &[], &cancel).err().expect("cancelled");
        assert_eq!(err, "cancelled");
    }
}
