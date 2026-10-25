//! AI translation for Pubgen (plan docs/plans/2026-10-22-offline-ai-translation-pubgen.md):
//! the setup dialog, the background translate job, the collect pass, EN-lookup /
//! table-pairing helpers, and page-targeted in-place translation of generated RXX pages.
//! Provider-blind: an online Groq backend or the offline translator behind one \`translate\`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use egui::{Color32, RichText};
use serde_json::json;

use super::{PubgenState, detect_delimiter, effective_col, parse_csv_content_with_delim, resolve_catalog_related_file};
use crate::DesignApp;
use designcraft_translate as translate;

pub const PROVIDER_OFFLINE: usize = 0;
pub const PROVIDER_GROQ: usize = 1;

// ---------------------------------------------------------------------------
// Setup dialog
// ---------------------------------------------------------------------------

/// The "Translate with AI" setup modal owned by the panel.
#[derive(Clone, Debug)]
pub struct AiSetupDialog {
    pub provider: usize,
    pub groq_key: String,
    pub groq_model: String,
    pub groq_models: Vec<String>,
    pub groq_models_error: Option<String>,
    pub offline_available: bool,
    pub mem: Option<translate::system::MemoryInfo>,
    pub cores: usize,
    pub error: Option<String>,
    /// Set by a successful Use AI: the dialog closes and stays closed.
    pub close_requested: bool,
}

impl AiSetupDialog {
    /// Prefill from the persisted preferences; probe the hardware synchronously (a
    /// few microseconds — no thread needed).
    pub fn open(app: &DesignApp) -> Self {
        let prefs = &app.session.prefs;
        let groq_model =
            if prefs.translation_groq_model.trim().is_empty() { "llama-3.3-70b-versatile".to_string() } else { prefs.translation_groq_model.clone() };
        Self {
            provider: if prefs.translation_provider == "groq" { PROVIDER_GROQ } else { PROVIDER_OFFLINE },
            groq_key: prefs.translation_groq_api_key.clone(),
            groq_model,
            groq_models: Vec::new(),
            groq_models_error: None,
            offline_available: translate::is_available(),
            mem: translate::system::memory_info(),
            cores: translate::system::cpu_cores(),
            error: None,
            close_requested: false,
        }
    }

    /// Fetch the Groq model list synchronously (bounded by the client's 30 s timeout).
    pub fn refresh_models(&mut self) {
        self.groq_models_error = None;
        let t = translate::OnlineTranslator::new(&self.groq_key, &self.groq_model);
        match t.list_models() {
            Ok(models) => {
                // Keep translation-capable models only (drop audio/safety/tts helpers),
                // preselect llama-3.3 when the account has it, else the first capable.
                let capable: Vec<String> = models.into_iter().filter(|m| translation_capable(m)).collect();
                if self.groq_model.is_empty() || !capable.iter().any(|m| m == &self.groq_model) {
                    self.groq_model = capable.iter().find(|m| m.contains("llama-3.3")).or_else(|| capable.first()).cloned().unwrap_or_default();
                }
                self.groq_models = capable;
            }
            Err(e) => {
                // Never echo the key: TranslateError strings name the endpoint problem only.
                self.groq_models_error = Some(e.to_string());
            }
        }
    }

    /// Prove the setup against the provider with one tiny real request (validates the
    /// key AND the model id), then persist through prefs.set and arm the panel.
    /// Returns false (with self.error set) when the check fails — the dialog stays
    /// open so the user can fix the setup.
    pub fn verify_and_accept_groq(&mut self, app: &mut DesignApp, st: &mut PubgenState) -> bool {
        self.error = None;
        let mut t = translate::OnlineTranslator::new(self.groq_key.trim(), self.groq_model.trim());
        let cancel = AtomicBool::new(false);
        match t.translate(&["tes".into()], translate::Lang::Indonesian, translate::Lang::English, &cancel, &mut |_, _tot| {}) {
            Ok(_) => {
                let _ = app.run(
                    "prefs.set",
                    json!({
                        "translationProvider": "groq",
                        "translationGroqApiKey": self.groq_key.trim(),
                        "translationGroqModel": self.groq_model.trim(),
                    }),
                );
                st.ai_setup_done = true;
                self.close_requested = true;
                true
            }
            Err(e) => {
                self.error = Some(format!("Setup check failed: {e} — fix the key or model and try again."));
                false
            }
        }
    }
}

/// Groq accounts list non-translation endpoints too (speech-to-text, TTS, safety
/// classifiers). Hide anything that cannot run a chat translation.
fn translation_capable(id: &str) -> bool {
    let l = id.to_lowercase();
    !["whisper", "guard", "orpheus", "tts", "embed", "moderation", "distil"].iter().any(|x| l.contains(x))
}

fn groq_pane(app: &mut DesignApp, st: &mut PubgenState, dialog: &mut AiSetupDialog, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("API key:");
        let key_edit = egui::TextEdit::singleline(&mut dialog.groq_key).password(true).hint_text("gsk_…").desired_width(220.0);
        ui.add(key_edit);
        if ui.button("Refresh models").clicked() {
            dialog.refresh_models();
        }
    });
    ui.hyperlink_to("Get a free key", "https://console.groq.com/keys");
    ui.label(RichText::new("Key is stored locally in DesignCraft settings.").size(10.0));

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Model:");
        if dialog.groq_models.is_empty() {
            ui.add(egui::TextEdit::singleline(&mut dialog.groq_model).hint_text("e.g. openai/gpt-oss-20b").desired_width(220.0));
        } else {
            egui::ComboBox::from_id_salt("pubgen_ai_groq_model")
                .selected_text(if dialog.groq_model.is_empty() { "Pick a model" } else { dialog.groq_model.as_str() })
                .width(280.0)
                .show_ui(ui, |ui| {
                    for m in &dialog.groq_models {
                        ui.selectable_value(&mut dialog.groq_model, m.clone(), m);
                    }
                });
        }
    });
    if let Some(err) = &dialog.groq_models_error {
        ui.label(RichText::new(err).size(10.0).color(Color32::from_rgb(220, 120, 60)));
    }
    ui.add_space(4.0);
    // Honest lines (plan §3.1.0).
    ui.label(RichText::new("Catalog text is sent to Groq's cloud.").size(10.0));
    ui.label(RichText::new("Free-tier limits: big jobs run slower, not blocked.").size(10.0));

    ui.add_space(6.0);
    let can_use = !dialog.groq_key.trim().is_empty() && !dialog.groq_model.trim().is_empty();
    // The click runs one tiny live request against Groq (a second, at most) and either
    // arms AI or shows the provider error in the dialog.
    if ui.add_enabled(can_use, egui::Button::new("Use AI")).clicked() {
        dialog.verify_and_accept_groq(app, st);
    }
    if let Some(err) = &dialog.error {
        ui.add_space(4.0);
        ui.label(RichText::new(err).size(10.5).color(Color32::from_rgb(220, 80, 80)));
    }
}

fn offline_pane(app: &mut DesignApp, st: &mut PubgenState, dialog: &mut AiSetupDialog, ui: &mut egui::Ui) {
    match dialog.mem {
        Some(mem) => {
            ui.label(format!("RAM      {:.1} GB available of {:.1} GB", mem.available as f64 / 1e9, mem.total as f64 / 1e9));
            ui.label(format!("CPU      {} logical cores", dialog.cores));
            ui.label("GPU      none detected (CPU inference)");
        }
        None => {
            ui.label("Hardware check unavailable.");
        }
    }
    ui.add_space(4.0);
    if !dialog.offline_available {
        ui.label(
            RichText::new("This build has no offline model compiled in. Use the Groq tab, or run DesignCraft with --features ai-translate.")
                .size(10.5)
                .color(Color32::from_rgb(220, 120, 60)),
        );
    } else {
        if ui.button("Use Offline Model").clicked() {
            let _ = app.run(
                "prefs.set",
                json!({
                    "translationProvider": "offline",
                }),
            );
            st.ai_setup_done = true;
            dialog.close_requested = true;
        }
    }

    for opt in translate::models::MODEL_OPTIONS {
        ui.add_space(2.0);
        ui.label(format!("{} · {:.1} GB download · {:.1} GB peak", opt.label, opt.approx_bytes as f64 / 1e9, opt.peak_ram_bytes as f64 / 1e9));
        let fit = dialog.mem.map(|mem| translate::models::fit(opt, &mem));
        if let (Some(fit), Some(mem)) = (fit, dialog.mem) {
            let est = translate::models::estimate_speed(opt, dialog.cores);
            let badge = match fit {
                translate::models::Fit::Fits => "fits",
                translate::models::Fit::Tight => "tight",
                translate::models::Fit::TooSmall => "needs more RAM",
            };
            ui.label(
                RichText::new(format!(
                    "Est. {:.1} tokens/s — about {:.0} s per paragraph, {:.0} s per page   [{}]",
                    est.tokens_per_sec, est.per_paragraph_secs, est.per_page_secs, badge
                ))
                .size(10.0),
            );
            let _ = mem;
        }
        ui.label(RichText::new("Licence: CC-BY-NC-4.0 (non-commercial)").size(10.0));
    }
}

/// Render the modal, when open. Called from the panel's `show`.
pub fn dialog_ui(app: &mut DesignApp, st: &mut PubgenState, ui: &mut egui::Ui) {
    let Some(mut dialog) = st.ai_dialog.take() else { return };
    let mut open = true;
    let mut closed_by_x = false;
    egui::Window::new(egui::RichText::new("Translate with AI — setup").size(15.0))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(460.0)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut dialog.provider, PROVIDER_GROQ, "Online — Groq (free account)");
                ui.radio_value(&mut dialog.provider, PROVIDER_OFFLINE, "Offline on this computer");
            });
            ui.separator();

            if dialog.provider == PROVIDER_GROQ {
                groq_pane(app, st, &mut dialog, ui);
            } else {
                offline_pane(app, st, &mut dialog, ui);
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    closed_by_x = true;
                }
            });
        });
    open &= !closed_by_x;
    if dialog.close_requested || !open {
        st.ai_dialog = None;
    } else {
        st.ai_dialog = Some(dialog);
    }
}

// ---------------------------------------------------------------------------
// Background job
// ---------------------------------------------------------------------------

/// Units per worker batch (mirrors the provider client's chunk size; small batches
/// keep free-tier per-minute token limits from tripping).
const BATCH_UNITS: usize = 8;

/// One background translation run over the collected units (ID source text → EN).
/// The receiver sits in an Arc<Mutex<_>> so the job can live in the panel's egui
/// temp state (Clone + Send + Sync) and be polled frame by frame.
#[derive(Clone, Debug)]
pub struct TranslateJob {
    #[cfg(not(target_arch = "wasm32"))]
    rx: Arc<std::sync::Mutex<std::sync::mpsc::Receiver<JobMsg>>>,
    #[cfg(not(target_arch = "wasm32"))]
    cancel: Arc<AtomicBool>,
    pub done: usize,
    pub total: usize,
    pub finished: bool,
}

enum JobMsg {
    Progress(usize, usize),
    Done(Vec<(String, String)>),
    /// Error message + everything already translated before the failure.
    Failed(String, Vec<(String, String)>),
}

impl TranslateJob {
    /// Spawn the worker for \`units\` (key = Indonesian source text). Native only: on wasm
    /// the AI features are off and the header button stays disabled.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn(provider: &str, key: &str, model: &str, units: Vec<(String, String)>, ctx: egui::Context) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<JobMsg>();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_thread = cancel.clone();
        let total = units.len();
        let provider = provider.to_string();
        let key = key.to_string();
        let model = model.to_string();
        let tx_thread = tx.clone();
        // Batch-by-batch from the worker: whatever finished before a Cancel or an
        // error is kept and reported, so a re-run only translates the rest.
        let spawned = std::thread::Builder::new().name("designcraft-translate".into()).spawn(move || {
            let mut done_pairs: Vec<(String, String)> = Vec::new();
            let outcome: Result<(), translate::TranslateError> = if provider == "groq" {
                let mut t = translate::OnlineTranslator::new(&key, &model);
                let mut err = None;
                for chunk in units.chunks(BATCH_UNITS) {
                    if cancel_thread.load(std::sync::atomic::Ordering::Relaxed) {
                        err = Some(translate::TranslateError::Cancelled);
                        break;
                    }
                    let sources: Vec<String> = chunk.iter().map(|(_, s)| s.clone()).collect();
                    match t.translate(&sources, translate::Lang::Indonesian, translate::Lang::English, &cancel_thread, &mut |_, _| {}) {
                        Ok(out) => {
                            done_pairs.extend(chunk.iter().map(|(k, _)| k.clone()).zip(out));
                            let _ = tx_thread.send(JobMsg::Progress(done_pairs.len(), units.len()));
                        }
                        Err(e) => {
                            err = Some(e);
                            break;
                        }
                    }
                }
                err.map_or(Ok(()), Err)
            } else {
                Err(translate::TranslateError::Unavailable)
            };
            let _ = match outcome {
                Ok(()) => tx_thread.send(JobMsg::Done(done_pairs)),
                Err(e) => tx_thread.send(JobMsg::Failed(e.to_string(), done_pairs)),
            };
            ctx.request_repaint();
        });
        if spawned.is_err() {
            // The worker could not start: surface it as a normal job failure.
            let _ = tx.send(JobMsg::Failed("could not start the translation thread".into(), Vec::new()));
        }
        Self { rx: Arc::new(std::sync::Mutex::new(rx)), cancel, done: 0, total, finished: false }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn(_provider: &str, _key: &str, _model: &str, _units: Vec<(String, String)>, _ctx: egui::Context) -> Self {
        Self { done: 0, total: 0, finished: true }
    }

    /// Drain progress messages; on Done, store the pairs into \`st.translations\` and
    /// report through the panel status lines. Returns true when the job finished OK.
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
                    JobMsg::Progress(done, total) => {
                        self.done = done;
                        self.total = total;
                        let msg = format!("Translating with AI… {done}/{total}");
                        st.card1_status = Some((msg.clone(), true));
                        st.card3_status = Some((msg.clone(), true));
                        st.header_status = Some((msg, true));
                    }
                    JobMsg::Done(pairs) => {
                        for (k, en) in pairs {
                            st.translations.insert(k, en);
                        }
                        self.finished = true;
                        finished_ok = true;
                    }
                    JobMsg::Failed(err, partial) => {
                        // Keep whatever finished before the failure/cancel; the caller
                        // clears the slice signature so the next run translates only the
                        // rest.
                        for (k, en) in partial {
                            st.translations.insert(k, en);
                        }
                        self.finished = true;
                        self.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                        let msg = format!(
                            "AI translation stopped at {}/{}: {err} — run the same action again to translate the remaining texts",
                            self.done, self.total
                        );
                        eprintln!("pubgen ai: {msg}");
                        // Both cards can start the job — both must show the failure.
                        st.card1_status = Some((msg.clone(), false));
                        st.card3_status = Some((msg.clone(), false));
                        st.header_status = Some((msg, false));
                    }
                }
            }
            finished_ok
        }
    }

    pub fn cancel(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Collect pass
// ---------------------------------------------------------------------------

/// Walk the same records the generator will generate and gather every Indonesian text,
/// deduped and order-stable: row titles, the cover headline lines (unless
/// \`ai_only_reviews\`), table cells from the rows' CSV files, and RTF narrative
/// paragraphs for rows without a \`file_en\`. Returns (source) pairs — the key IS the
/// source text.
pub fn collect_translation_units(st: &PubgenState) -> Vec<(String, String)> {
    let total = st.card1_records.len();
    let (start, end) = if st.card1_range_all {
        (0, total)
    } else {
        let from = st.card1_range_from.clamp(1, total).saturating_sub(1);
        let to = st.card1_range_to.clamp(from + 1, total);
        (from, to)
    };
    let file_col = effective_col(st.card1_file_col, &st.card1_headers, &["file", "filename"], 3);
    let file_en_col = effective_col(st.card1_file_en_col, &st.card1_headers, &["file_en", "fileen"], 6);

    let mut seen: HashMap<String, ()> = HashMap::new();
    let mut out: Vec<(String, String)> = Vec::new();
    let push = |out: &mut Vec<(String, String)>, seen: &mut HashMap<String, ()>, text: &str| {
        let t = text.trim();
        if t.is_empty() || !translatable_cell(t) || seen.contains_key(t) {
            return;
        }
        seen.insert(t.to_string(), ());
        out.push((t.to_string(), t.to_string()));
    };

    for row in st.card1_records.get(start..end).unwrap_or(&[]) {
        let type_val = row.get(st.card1_type_col.min(2)).map(String::as_str).unwrap_or("").trim();
        let title_val = row.get(st.card1_title_col.min(2)).map(String::as_str).unwrap_or("").trim();
        if st.ai_only_reviews && !super::is_ulasan_type(type_val) {
            continue;
        }
        push(&mut out, &mut seen, title_val);
        let file_val = row.get(file_col).map(String::as_str).unwrap_or("").trim();
        let file_en_val = row.get(file_en_col).map(String::as_str).unwrap_or("").trim();
        let lower = file_val.to_ascii_lowercase();
        if lower.ends_with(".rtf") && file_en_val.is_empty() {
            for para in rtf_paragraphs(st, file_val) {
                push(&mut out, &mut seen, &para);
            }
        } else if lower.ends_with(".csv") || (!lower.is_empty() && !lower.ends_with(".rtf")) {
            for cell in table_cells(st, file_val) {
                push(&mut out, &mut seen, &cell);
            }
        }
    }

    if !st.ai_only_reviews {
        push(&mut out, &mut seen, st.cover_title.trim());
        push(&mut out, &mut seen, st.cover_title2.trim());
    }
    out
}

fn rtf_paragraphs(st: &PubgenState, file_val: &str) -> Vec<String> {
    let Some(path) = resolve_catalog_related_file(&st.card1_path, file_val, "rtf") else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    match designcraft_textimport::import("translate.rtf", &bytes) {
        Ok(imp) => imp.story.text.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}

fn table_cells(st: &PubgenState, file_val: &str) -> Vec<String> {
    let Some(path) = resolve_catalog_related_file(&st.card1_path, file_val, "csv") else {
        return Vec::new();
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let delim = detect_delimiter(&content);
    let matrix = parse_csv_content_with_delim(&content, delim);
    matrix.iter().flat_map(|row| row.iter()).filter(|c| translatable_cell(c)).cloned().collect()
}

// ---------------------------------------------------------------------------
// Cell rules and table pairing
// ---------------------------------------------------------------------------

/// Cells never sent to the model (a guard, not a prompt instruction — MT invents digits):
/// numeric/decimal/thousands, currency and %, dates and bare years, one short token,
/// the summary-word aliases (mapped deterministically instead), and empty cells.
pub fn translatable_cell(cell: &str) -> bool {
    let t = cell.trim();
    if t.is_empty() {
        return false;
    }
    if t.chars().count() <= 2 && !t.contains(' ') {
        return false;
    }
    if alias_en(t).is_some() {
        return false;
    }
    let Some(first) = t.chars().next() else { return false };
    if first.is_ascii_digit() || matches!(first, '-' | '+' | '(' | '%' | 'R') && t.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    if t.ends_with('%') {
        return false;
    }
    true
}

/// Deterministic summary-word aliases (\`total_words\` and friends): English directly, no
/// MT. ponytail: extend the list from real catalogs when a run misses one.
pub fn alias_en(cell: &str) -> Option<&'static str> {
    match cell.trim().to_ascii_lowercase().as_str() {
        "jumlah" | "total" => Some("Total"),
        "tahun" => Some("Year"),
        "kabupaten" => Some("Regency"),
        "kecamatan" | "kec" => Some("Subdistrict"),
        "provinsi" | "propinsi" => Some("Province"),
        "keterangan" | "ket." | "ket" => Some("Note"),
        "catatan" => Some("Note"),
        "sumber" => Some("Source"),
        "no" => Some("No"),
        _ => None,
    }
}

/// One table, two lines per cell: \`id\u{2028}en\` for translatable cells with a
/// translation; every skipped cell stays single-line. Same rows × cols by construction.
pub fn pair_cells(matrix: &[Vec<String>], translations: &HashMap<String, String>) -> Vec<Vec<String>> {
    matrix
        .iter()
        .map(|row| {
            if super::excel::is_table_note_or_source_row(row) {
                return row.clone();
            }
            row.iter()
                .map(|cell| {
                    let t = cell.trim();
                    if t.is_empty() || t.contains('\u{2028}') || t.contains('\n') || t.contains(" / ") {
                        return cell.clone();
                    }
                    if let Some(en) = alias_en(t) {
                        if t.eq_ignore_ascii_case(en) {
                            return cell.clone();
                        }
                        return format!("{t}\u{2028}{en}");
                    }
                    if !translatable_cell(t) {
                        return cell.clone();
                    }
                    match translations.get(t).map(String::as_str).filter(|s| !s.is_empty()) {
                        Some(en) if !t.eq_ignore_ascii_case(en) => format!("{t}\u{2028}{en}"),
                        _ => cell.clone(),
                    }
                })
                .collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// EN lookups and temp RTF bodies
// ---------------------------------------------------------------------------

/// The EN half for a source text: explicit \`existing\` always wins, else the AI map.
pub fn en_text<'a>(st: &'a PubgenState, source: &str, existing: &'a str) -> &'a str {
    if !existing.is_empty() { existing } else { st.translations.get(source.trim()).map(String::as_str).unwrap_or("") }
}

/// Temp RTF with the translated paragraphs of an Indonesian source RTF (per run temp
/// dir). Missing translations are skipped; generation tolerates the empty result.
pub fn translated_body_rtf(st: &PubgenState, source_rtf: &str) -> Option<PathBuf> {
    if let Some(p) = st.translated_bodies.get(source_rtf) {
        return Some(p.clone());
    }
    let paras = rtf_paragraphs(st, source_rtf);
    let mut en_paras: Vec<String> = Vec::new();
    for p in paras {
        let en = st.translations.get(p.trim()).map(String::as_str).unwrap_or("").trim().to_string();
        if !en.is_empty() {
            en_paras.push(en);
        }
    }
    if en_paras.is_empty() {
        return None;
    }
    let dir = temp_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}_en.rtf", super::excel::sanitize_filename(source_rtf.trim_end_matches(".rtf"))));
    write_plain_rtf(&en_paras.join("\n"), &path).ok()?;
    Some(path)
}

fn temp_dir() -> PathBuf {
    std::env::temp_dir().join(format!("designcraft-pubgen-{}", std::process::id()))
}

/// Minimal RTF writer: escape backslash, braces; non-ASCII to \`u<dd>?\`; newlines
/// become \`par\`.
pub fn write_plain_rtf(text: &str, path: &std::path::Path) -> Result<(), String> {
    let mut body = String::with_capacity(text.len() + 64);
    for c in text.chars() {
        match c {
            '\\' => body.push_str("\\\\"),
            '{' => body.push_str("\\{"),
            '}' => body.push_str("\\}"),
            '\n' => body.push_str("\\par\n"),
            c if (c as u32) > 127 => body.push_str(&format!("\\u{}?", c as u32)),
            c => body.push(c),
        }
    }
    let rtf = format!("{{\\rtf1\\ansi\\deff0{{\\fonttbl{{\\f0 Arial;}}}}\\f0\\fs22 {body}}}");
    std::fs::write(path, rtf).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// A stable signature of what the current Generate run would translate: when it matches
/// the finished job's, a second Generate runs without touching the model again.
pub fn slice_signature(st: &PubgenState) -> String {
    let total = st.card1_records.len();
    let (start, end) = if st.card1_range_all {
        (0, total)
    } else {
        let from = st.card1_range_from.clamp(1, total).saturating_sub(1);
        let to = st.card1_range_to.clamp(from + 1, total);
        (from, to)
    };
    let slice = st.card1_records.get(start..end).unwrap_or(&[]);
    format!("ai|{start}..{end}|{slice:?}|{}|{}", st.cover_title.trim(), st.cover_title2.trim())
}

/// Build the backend the panel should use right now from the persisted prefs.
pub fn backend_from_prefs(app: &DesignApp) -> Option<(String, String, String)> {
    let prefs = &app.session.prefs;
    if prefs.translation_provider == "groq" && !prefs.translation_groq_api_key.trim().is_empty() {
        let model = if prefs.translation_groq_model.trim().is_empty() {
            "llama-3.3-70b-versatile".to_string()
        } else {
            prefs.translation_groq_model.trim().to_string()
        };
        return Some((String::from("groq"), prefs.translation_groq_api_key.trim().to_string(), model));
    } else if prefs.translation_provider == "offline" && translate::is_available() {
        return Some((String::from("offline"), prefs.translation_model_dir.clone(), prefs.translation_model_id.clone()));
    }
    None
}

/// Compact status row (armed state): provider, Configure…, progress, Cancel.
pub fn header_status_row(app: &DesignApp, st: &mut PubgenState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let provider = if app.session.prefs.translation_provider == "groq" { "Groq" } else { "offline" };
        ui.add(egui::Label::new(RichText::new(format!("Translate with AI ({provider}) — ready")).size(10.5)).wrap());
        if let Some(job) = &st.translate_job
            && !job.finished
        {
            if job.total > 0 {
                let frac = job.done as f32 / job.total.max(1) as f32;
                ui.add(egui::ProgressBar::new(frac).show_percentage().desired_width(120.0));
            }
            if ui.button("Cancel").clicked() {
                job.cancel();
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Page-targeted translation (select a generated RXX page, translate in place)
// ---------------------------------------------------------------------------

/// A detected translatable RXX page: the frames the AI result will be written into.
#[derive(Debug, Clone)]
pub struct PageTarget {
    pub page_idx: usize,
    /// Frame with the italic "Chapter 01 ..." header line (textbox #3).
    pub en_header_fid: u64,
    /// Existing header prefix, e.g. "Chapter 01".
    pub header_prefix: String,
    /// The Indonesian chapter title text (textbox #2, e.g. "Geografi").
    pub id_title: String,
    /// The frame holding that title.
    pub title_fid: u64,
    /// The Indonesian body frame (left column).
    pub id_body_fid: u64,
    /// The English body frame (right column) — its paragraphs are replaced.
    pub en_body_fid: u64,
}

fn frame_text(app: &DesignApp, fid: u64) -> Option<String> {
    let active = app.session.active()?;
    let item = active.doc.item(designcraft_doc::ItemId(fid))?;
    let tf = item.text_frame()?;
    let story = active.doc.story(tf.story)?;
    Some(story.text.clone())
}

/// Detect the selected page as a translatable RXX review page: an EN header line
/// ("Chapter NN ..."), a "BAB NN" line, an Indonesian title line, and two side-by-side
/// body columns. Returns None for anything else.
pub fn detect_page_target(app: &DesignApp, page_idx: usize) -> Option<PageTarget> {
    let active = app.session.active()?;
    let (si, _pi) = active.doc.page_loc(page_idx)?;
    let spread = active.doc.spreads.get(si)?;

    let mut frames: Vec<(u64, f64, f64, String)> = Vec::new(); // id, y0, x0, text
    for it in &spread.items {
        if let Some(tf) = it.text_frame() {
            let text = active.doc.story(tf.story)?.text.clone();
            if !text.trim().is_empty() {
                frames.push((it.id.0, it.bounds().y0, it.bounds().x0, text));
            }
        }
    }

    // EN header: a short line starting with "Chapter".
    let (en_header_fid, en_header_text, en_header_y) = frames
        .iter()
        .find(|(_, _, _, t)| {
            let t = t.trim();
            t.len() < 80 && t.to_lowercase().starts_with("chapter")
        })
        .map(|(id, y, _, t)| (*id, t.trim().to_string(), *y))?;

    // BAB line: a two-word "BAB 01" / "CHAPTER 01" line.
    let bab_fid = frames
        .iter()
        .find(|(id, _, _, t)| {
            *id != en_header_fid && {
                let up = t.trim().to_uppercase();
                (up.starts_with("BAB ") || up.starts_with("CHAPTER ")) && up.split_whitespace().count() == 2
            }
        })
        .map(|(id, _, _, _)| *id)?;

    // ID title: the short line above the EN header that is not the BAB line.
    let (title_fid, id_title) = frames
        .iter()
        .filter(|(id, y, _, t)| *id != en_header_fid && *id != bab_fid && *y < en_header_y && t.trim().len() < 80)
        .max_by_key(|(_, y, _, _)| y.to_bits())
        .map(|(id, _, _, t)| (*id, t.trim().to_string()))?;

    // Body columns: text frames below the header, plenty of text, two side by side.
    let mut bodies: Vec<(u64, f64)> = frames
        .iter()
        .filter(|(id, y, _, t)| {
            *id != en_header_fid && *id != bab_fid && Some(*id) != Some(title_fid) && *y > en_header_y && t.trim().chars().count() > 120
        })
        .map(|(id, _, x, _)| (*id, *x))
        .collect();
    if bodies.len() < 2 {
        return None;
    }
    bodies.sort_by(|a, b| a.1.total_cmp(&b.1));
    let id_body_fid = bodies[0].0;
    let en_body_fid = bodies[bodies.len() - 1].0;

    let header_prefix = {
        let words: Vec<&str> = en_header_text.split_whitespace().collect();
        if words.len() >= 2 && words[0].to_lowercase() == "chapter" {
            format!("{} {}", words[0], words[1])
        } else {
            words.first().map(|w| (*w).to_string()).unwrap_or_default()
        }
    };

    Some(PageTarget { page_idx, en_header_fid, header_prefix, id_title, title_fid, id_body_fid, en_body_fid })
}

/// The page's Indonesian units: the title line plus each body paragraph.
pub fn collect_page_units(app: &DesignApp, target: &PageTarget) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut push = |t: &str| {
        let t = t.trim();
        if !t.is_empty() {
            out.push((t.to_string(), t.to_string()));
        }
    };
    if let Some(t) = frame_text(app, target.title_fid) {
        push(&t);
    }
    if let Some(body) = frame_text(app, target.id_body_fid) {
        for para in body.split('\n') {
            push(para);
        }
    }
    out
}

/// Write the AI results into the page: EN header line and right-column body —
/// paragraph by paragraph, preserving each paragraph's own style (heading2, body, …):
/// the paragraph table of contents is generated from those styles. A paragraph without
/// a translation keeps its Indonesian text.
pub fn apply_page_result(app: &mut DesignApp, st: &PubgenState, target: &PageTarget) -> usize {
    let map = &st.translations;

    // 1. EN header line (single paragraph): "Chapter 01 <translated title>".
    let en_title = map.get(target.id_title.trim()).map(String::as_str).filter(|s| !s.is_empty());
    let mut replaced = 0usize;
    if let Some(en) = en_title {
        replace_frame_text(app, target.en_header_fid, &format!("{} {en}", target.header_prefix));
        replaced += 1;
    }

    // 2. Right-column body: replace paragraph-by-paragraph so every paragraph keeps its
    //    own paragraph style (the paragraph TOC is generated from those styles).
    replaced += replace_frame_paragraphs(app, target.en_body_fid, map);
    replaced
}

/// Replace each paragraph of the frame whose trimmed text has a translation, keeping
/// every paragraph's own paragraph style. Returns the number of paragraphs replaced.
fn replace_frame_paragraphs(app: &mut DesignApp, fid: u64, map: &HashMap<String, String>) -> usize {
    let snapshot = (|| -> Option<(designcraft_doc::StoryId, Vec<(usize, String, String)>)> {
        let active = app.session.active()?;
        let item = active.doc.item(designcraft_doc::ItemId(fid))?;
        let tf = item.text_frame()?;
        let sid = tf.story;
        let story = active.doc.story(sid)?;
        let mut v = Vec::new();
        for (i, r) in story.para_ranges().iter().enumerate() {
            let mut content = story.text.get(r.start..r.end)?.to_string();
            if content.ends_with('\n') {
                content.pop();
            }
            let style = story.paras.get(i).map(|p| p.style.clone()).unwrap_or_default();
            v.push((r.start, content, style));
        }
        Some((sid, v))
    })();
    let Some((sid, paras)) = snapshot else { return 0 };

    // Apply from the last paragraph backwards so earlier byte offsets stay valid.
    let mut replaced = 0usize;
    for (start, content, _style) in paras.iter().rev() {
        let key = content.trim();
        let Some(en) = map.get(key).map(String::as_str).filter(|s| !s.is_empty()) else {
            continue;
        };
        let _ = app.run("text.select", json!({"story": sid, "anchor": *start, "focus": start + content.len()}));
        let _ = app.run("text.insert", json!({"text": en}));
        replaced += 1;
    }
    replaced
}

/// Replace the whole story of a frame with text (paragraphs on \n). Single-paragraph
/// frames only — the paragraph style is kept because the frame has one paragraph.
fn replace_frame_text(app: &mut DesignApp, fid: u64, text: &str) {
    let Some(active) = app.session.active() else { return };
    let Some(item) = active.doc.item(designcraft_doc::ItemId(fid)) else { return };
    let Some(tf) = item.text_frame() else { return };
    let sid = tf.story;
    let len = active.doc.story(sid).map(|s| s.text.len()).unwrap_or(0);
    let _ = active;
    if len == 0 {
        return;
    }
    let _ = app.run("text.select", json!({"story": sid, "anchor": 0, "focus": len}));
    let _ = app.run("text.insert", json!({"text": text}));
}

// ---------------------------------------------------------------------------
// Table page-targeted translation (select a generated Table page, translate in place)
// ---------------------------------------------------------------------------

/// A detected translatable Table page: title frame, table frame, and notes/source frame.
#[derive(Debug, Clone)]
pub struct TablePageTarget {
    pub page_idx: usize,
    /// Frame holding the table title (above the table), if any.
    pub title_fid: Option<u64>,
    /// Frame holding the Table.
    pub table_fid: u64,
    /// Story ID of the Table.
    pub table_sid: designcraft_doc::StoryId,
    /// Table ID within that story.
    pub table_id: u64,
    /// Frame holding the notes / sources footnotes (below the table), if any.
    pub notes_fid: Option<u64>,
}

/// Returns true if the text only represents a table kicker ("Tabel", "Table", "Tabel\nTable") or a number ("02.01", "1.1").
pub fn is_pure_kicker_or_num(text: &str) -> bool {
    let t = text.trim();
    let lower = t.to_lowercase();
    let kicker_words = ["tabel", "table"];
    let lines: Vec<&str> = lower.lines().flat_map(|l| l.split(['\u{2028}', '/'])).map(str::trim).filter(|s| !s.is_empty()).collect();
    if !lines.is_empty() && lines.iter().all(|l| kicker_words.contains(l)) {
        return true;
    }
    if t.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | ' ' | '-' | '(' | ')')) {
        return true;
    }
    let words: Vec<&str> = t.split_whitespace().collect();
    if words.len() == 2
        && kicker_words.contains(&words[0].to_lowercase().as_str())
        && words[1].chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | '-'))
    {
        return true;
    }
    false
}

/// Detect the selected page as a translatable Table page.
pub fn detect_table_page_target(app: &DesignApp, page_idx: usize) -> Option<TablePageTarget> {
    let active = app.session.active()?;
    let (si, pi) = active.doc.page_loc(page_idx)?;
    let spread = active.doc.spreads.get(si)?;
    let page = spread.pages.get(pi)?;
    let page_bounds = page.bounds();

    let is_on_this_page = |b: designcraft_geom::Rect| -> bool {
        if spread.pages.len() <= 1 {
            return true;
        }
        let mid_x = (b.x0 + b.x1) / 2.0;
        mid_x >= page_bounds.x0 - 5.0 && mid_x <= page_bounds.x1 + 5.0
    };

    // 1. Locate the table item on this page
    let mut table_item = None;
    for it in &spread.items {
        let b = it.bounds();
        if !is_on_this_page(b) {
            continue;
        }
        if let Some(tf) = it.text_frame()
            && let Some(story) = active.doc.story(tf.story)
            && let Some((&tid, _)) = story.tables.iter().next()
        {
            table_item = Some((it.id.0, tf.story, tid, b));
            break;
        }
    }
    let (table_fid, table_sid, table_id, table_bounds) = table_item?;

    // 2. Locate title frame on this page (located above the table)
    let mut title_fid = None;
    let mut best_score = -1.0;
    for it in &spread.items {
        if it.id.0 == table_fid {
            continue;
        }
        let b = it.bounds();
        if !is_on_this_page(b) {
            continue;
        }
        if b.y0 >= table_bounds.y0 {
            continue;
        }
        if let Some(tf) = it.text_frame()
            && let Some(story) = active.doc.story(tf.story)
        {
            let text = story.text.trim();
            if text.is_empty() || is_pure_kicker_or_num(text) {
                continue;
            }
            let lower_lbl = it.label.to_lowercase();
            let mut score = b.width();
            if lower_lbl.contains("title") || lower_lbl.contains("judul") {
                score += 1000.0;
            }
            let dist = (table_bounds.y0 - b.y1).abs();
            score -= (dist / 10.0).min(50.0);
            if score > best_score {
                best_score = score;
                title_fid = Some(it.id.0);
            }
        }
    }

    // 3. Locate notes/source frame on this page (located below the table)
    let mut notes_fid = None;
    for it in &spread.items {
        if it.id.0 == table_fid || Some(it.id.0) == title_fid {
            continue;
        }
        let b = it.bounds();
        if !is_on_this_page(b) {
            continue;
        }
        if b.y0 >= table_bounds.y0 - 5.0
            && let Some(tf) = it.text_frame()
            && let Some(story) = active.doc.story(tf.story)
        {
            let text = story.text.trim();
            let lower = text.to_lowercase();
            let lower_lbl = it.label.to_lowercase();
            if lower.starts_with("sumber")
                || lower.starts_with("source")
                || lower.starts_with("keterangan")
                || lower.starts_with("ket.")
                || lower.starts_with("ket ")
                || lower.starts_with("ket:")
                || lower == "ket"
                || lower.starts_with("catatan")
                || lower.starts_with("note")
                || lower_lbl.contains("note")
                || lower_lbl.contains("footnote")
            {
                notes_fid = Some(it.id.0);
                break;
            }
        }
    }

    Some(TablePageTarget { page_idx, title_fid, table_fid, table_sid, table_id, notes_fid })
}

/// Helper to parse catalog or table title into (clean_title, Option<prefix>).
/// e.g. "Tabel 01.01 Luas Daerah..." -> ("Luas Daerah...", Some("Tabel 01.01"))
pub fn extract_table_title_parts(title: &str) -> (String, Option<String>) {
    let trimmed = title.trim();
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() >= 3 && (words[0].eq_ignore_ascii_case("tabel") || words[0].eq_ignore_ascii_case("table")) {
        let prefix = format!("{} {}", words[0], words[1]);
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            return (rest.trim().to_string(), Some(prefix));
        }
    }
    (trimmed.to_string(), None)
}

/// Helper to parse a single line from a table note or source textbox into (label, colon, content).
pub fn parse_note_line(line: &str) -> (String, String, String) {
    if line.contains('\t') {
        let parts: Vec<&str> = line.split('\t').collect();
        let label = parts.first().unwrap_or(&"").trim().to_string();
        let colon = parts.get(1).unwrap_or(&":").trim().to_string();
        let content = parts.get(2..).map(|p| p.join("\t")).unwrap_or_default().trim().to_string();
        (label, colon, content)
    } else if let Some((l, c)) = line.split_once(':') {
        (l.trim().to_string(), ":".to_string(), c.trim().to_string())
    } else {
        (line.trim().to_string(), ":".to_string(), String::new())
    }
}

/// Helper to get translated note elements for a given (label, colon, content).
pub fn translate_note_item(label: &str, colon: &str, content: &str, map: &HashMap<String, String>) -> (String, String, String) {
    let lower = label.to_lowercase();
    let label_en = if lower.starts_with("keterangan")
        || lower.starts_with("ket.")
        || lower.starts_with("ket ")
        || lower.starts_with("ket:")
        || lower == "ket"
        || lower.starts_with("catatan")
    {
        "Note".to_string()
    } else if lower.starts_with("sumber") {
        "Source".to_string()
    } else {
        map.get(label.trim()).cloned().unwrap_or_else(|| label.to_string())
    };

    let colon_en = colon.to_string();

    let content_trimmed = content.trim();
    let content_en = if content_trimmed == "-" || content_trimmed.is_empty() {
        content.to_string()
    } else {
        map.get(content_trimmed).cloned().unwrap_or_else(|| content.to_string())
    };

    (label_en, colon_en, content_en)
}

/// Collect translatable units from a table page.
pub fn collect_table_page_units(app: &DesignApp, target: &TablePageTarget) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut seen: HashMap<String, ()> = HashMap::new();
    let push = |out: &mut Vec<(String, String)>, seen: &mut HashMap<String, ()>, t: &str| {
        let t = t.trim();
        if t.is_empty() || seen.contains_key(t) {
            return;
        }
        seen.insert(t.to_string(), ());
        out.push((t.to_string(), t.to_string()));
    };

    // 1. Title
    if let Some(fid) = target.title_fid
        && let Some(text) = frame_text(app, fid)
    {
        let lines: Vec<&str> = text.split(['\n', '\u{2028}']).map(str::trim).filter(|s| !s.is_empty()).collect();
        if let Some(first) = lines.first() {
            let (clean_title, _) = extract_table_title_parts(first);
            if translatable_cell(&clean_title) {
                push(&mut out, &mut seen, &clean_title);
            }
            if clean_title != *first && translatable_cell(first) {
                push(&mut out, &mut seen, first);
            }
        }
    }

    // 2. Table cells
    if let Some(active) = app.session.active()
        && let Some(story) = active.doc.story(target.table_sid)
        && let Some(table) = story.tables.get(&target.table_id)
    {
        let (nr, nc) = (table.nrows(), table.ncols());
        for r in 0..nr {
            for c in 0..nc {
                if let Some(cell) = table.cell(r, c) {
                    let text = cell.text.text.trim();
                    if text.is_empty() || text.contains('\n') || text.contains('\u{2028}') || text.contains(" / ") {
                        continue;
                    }
                    if translatable_cell(text) {
                        push(&mut out, &mut seen, text);
                    }
                }
            }
        }
    }

    // 3. Notes / footnotes content
    if let Some(fid) = target.notes_fid
        && let Some(text) = frame_text(app, fid)
    {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (label, _, content) = parse_note_line(line);
            let lower_lbl = label.to_lowercase();
            if lower_lbl.starts_with("source") || lower_lbl.starts_with("note") {
                continue;
            }
            let c_trimmed = content.trim();
            if !c_trimmed.is_empty() && c_trimmed != "-" && translatable_cell(c_trimmed) {
                push(&mut out, &mut seen, c_trimmed);
            }
        }
    }

    out
}

fn story_is_bold(story: &designcraft_doc::Story) -> bool {
    story.chars.iter().any(|r| r.format.over.font_style.as_deref().is_some_and(|s| s.to_lowercase().contains("bold")))
}

/// Apply AI translation result in place onto a detected Table page:
/// - Title: new line Italic
/// - Table Header, Footer, and Category cells: in same cell new line [italic]
/// - Footnotes / Notes: next row [italic] for each line
pub fn apply_table_page_result(app: &mut DesignApp, st: &PubgenState, target: &TablePageTarget) -> usize {
    let map = &st.translations;
    let mut total_replaced = 0usize;

    // 1. Table Title (New line Italic)
    if let Some(fid) = target.title_fid
        && let Some(text) = frame_text(app, fid)
    {
        let lines: Vec<&str> = text.split(['\n', '\u{2028}']).collect();
        let first_line = lines.first().copied().unwrap_or(text.as_str()).trim();
        let (clean_title, prefix_opt) = extract_table_title_parts(first_line);
        let en_opt = map.get(&clean_title).or_else(|| map.get(first_line)).map(String::as_str).filter(|s| !s.is_empty());
        if let Some(en) = en_opt {
            let en_line = match prefix_opt {
                Some(p) => {
                    let p_en = if p.to_lowercase().starts_with("tabel") { p.replacen("Tabel", "Table", 1).replacen("tabel", "Table", 1) } else { p };
                    format!("{p_en} {en}")
                }
                None => en.to_string(),
            };
            let combined = format!("{first_line}\n{en_line}");
            let len = text.len();
            if let Some(active) = app.session.active()
                && let Some(item) = active.doc.item(designcraft_doc::ItemId(fid))
                && let Some(tf) = item.text_frame()
            {
                let sid = tf.story;
                let is_bold = active.doc.story(sid).map(story_is_bold).unwrap_or(false);

                let _ = app.run("text.select", json!({"story": sid.0, "anchor": 0, "focus": len}));
                let _ = app.run("text.insert", json!({"text": combined}));

                // Format line 2 as Italic
                let _ = app.session.edit(|doc, _| {
                    if let Some(story) = doc.story_mut(sid)
                        && let Some(pos) = story.text.find('\n').or_else(|| story.text.find('\u{2028}'))
                    {
                        let start = pos + 1;
                        let end = story.text.len();
                        if start < end {
                            story.format_chars(start..end, |f| {
                                f.over.font_style = Some(if is_bold { "Bold Italic".to_string() } else { "Italic".to_string() });
                            });
                            story.rev += 1;
                        }
                    }
                    Ok(serde_json::Value::Null)
                });

                // Auto-fit title frame height
                if let Some(active) = app.session.active() {
                    let cs = designcraft_compose::compose_story(&active.doc, sid, &Default::default());
                    let h = cs.frames.first().map(|f| f.content_height).unwrap_or(40.0);
                    let fit_h = (h + 4.0).max(24.0);
                    let _ = app.session.edit(|doc, _| {
                        if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
                            let rect = it.bounds();
                            it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(rect.x0, rect.y0, rect.x1, rect.y0 + fit_h));
                        }
                        Ok(serde_json::Value::Null)
                    });
                }

                total_replaced += 1;
            }
        }
    }

    // 2. Table Cells (in same cell new line [italic])
    let mut replaced_cells = 0usize;
    let _ = app.session.edit(|doc, _| {
        let Some(story) = doc.story_mut(target.table_sid) else {
            return Ok(serde_json::Value::Null);
        };
        let Some(table_arc) = story.tables.get_mut(&target.table_id) else {
            return Ok(serde_json::Value::Null);
        };
        let table = std::sync::Arc::make_mut(table_arc);
        let (nr, nc) = (table.nrows(), table.ncols());
        for r in 0..nr {
            for c in 0..nc {
                let Some(cell) = table.cell_mut(r, c) else {
                    continue;
                };
                let cur = cell.text.text.trim().to_string();
                if cur.is_empty() || cur.contains('\n') || cur.contains('\u{2028}') || cur.contains(" / ") {
                    continue;
                }
                let en_opt = alias_en(&cur).map(|s| s.to_string()).or_else(|| map.get(&cur).cloned());
                if let Some(en) = en_opt
                    && !en.is_empty()
                    && !cur.eq_ignore_ascii_case(&en)
                {
                    let is_bold = story_is_bold(&cell.text);
                    let insert_pos = cell.text.len();
                    let en_str = format!("\n{en}");
                    cell.text.insert(insert_pos, &en_str);
                    let en_start = insert_pos + 1;
                    let en_end = cell.text.len();
                    if en_start < en_end {
                        cell.text.format_chars(en_start..en_end, |f| {
                            f.over.font_style = Some(if is_bold { "Bold Italic".to_string() } else { "Italic".to_string() });
                        });
                    }
                    replaced_cells += 1;
                }
            }
        }
        Ok(serde_json::Value::Null)
    });
    total_replaced += replaced_cells;

    // 2b. Auto-fit table frame height and shift notes if necessary
    let mut table_new_y1 = None;
    if let Some(active) = app.session.active() {
        let (si, pi) = active.doc.page_loc(target.page_idx).unwrap_or((0, 0));
        let margin_y1 = active.doc.spreads.get(si).and_then(|s| s.pages.get(pi)).map(|p| p.margin_rect().y1).unwrap_or(800.0);
        let notes_reserve = if target.notes_fid.is_some() { 50.0 } else { 0.0 };
        let max_table_y1 = (margin_y1 - notes_reserve).max(60.0);

        let mut test_doc = (*active.doc).clone();
        if let Some(rect) = test_doc.item(designcraft_doc::ItemId(target.table_fid)).map(|it| it.bounds()) {
            if let Some(it) = test_doc.item_mut(designcraft_doc::ItemId(target.table_fid)) {
                it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(rect.x0, rect.y0, rect.x1, rect.y0 + 10000.0));
            }
            let cs = designcraft_compose::compose_story(&test_doc, target.table_sid, &Default::default());
            let measured_h = cs.frames.first().and_then(|f| f.tables.first().map(|t| t.rect.height()).or(Some(f.content_height)));

            if let Some(th) = measured_h {
                let (inset_top, inset_bottom) = test_doc
                    .item(designcraft_doc::ItemId(target.table_fid))
                    .and_then(|it| it.text_frame())
                    .map(|tf| (tf.options.inset[0], tf.options.inset[2]))
                    .unwrap_or((0.0, 0.0));
                let total_needed = th + inset_top + inset_bottom + 6.0;

                let title_bottom = target.title_fid.and_then(|fid| active.doc.item(designcraft_doc::ItemId(fid))).map(|it| it.bounds().y1);
                let table_y0 = if let Some(tb) = title_bottom { rect.y0.max(tb + 6.0) } else { rect.y0 };

                let fit_h = total_needed.min(max_table_y1 - table_y0).max(40.0);
                let new_y1 = table_y0 + fit_h;
                let _ = app.session.edit(|doc, _| {
                    if let Some(it) = doc.item_mut(designcraft_doc::ItemId(target.table_fid)) {
                        it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(rect.x0, table_y0, rect.x1, new_y1));
                    }
                    Ok(serde_json::Value::Null)
                });
                table_new_y1 = Some(new_y1);
            }
        }
    }

    // 3. Notes and Sources Footnotes Textbox
    let mut notes_updated = false;
    if let Some(fid) = target.notes_fid
        && let Some(text) = frame_text(app, fid)
    {
        let mut lines_out: Vec<(String, bool)> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (label, colon, content) = parse_note_line(line);
            let lower_lbl = label.to_lowercase();
            // If it's already an English line that was generated earlier, skip it
            if lower_lbl.starts_with("source") || lower_lbl.starts_with("note") {
                continue;
            }
            let (label_en, colon_en, content_en) = translate_note_item(&label, &colon, &content, map);
            let has_tab = line.contains('\t');
            let id_line = if has_tab {
                if content.is_empty() { format!("{label}\t{colon}") } else { format!("{label}\t{colon}\t{content}") }
            } else {
                if content.is_empty() { format!("{label} {colon}") } else { format!("{label} {colon} {content}") }
            };
            let en_line = if has_tab {
                if content_en.is_empty() { format!("{label_en}\t{colon_en}") } else { format!("{label_en}\t{colon_en}\t{content_en}") }
            } else {
                if content_en.is_empty() { format!("{label_en} {colon_en}") } else { format!("{label_en} {colon_en} {content_en}") }
            };
            lines_out.push((id_line, false));
            lines_out.push((en_line, true));
        }

        if !lines_out.is_empty()
            && let Some(active) = app.session.active()
            && let Some(item) = active.doc.item(designcraft_doc::ItemId(fid))
            && let Some(tf) = item.text_frame()
        {
            let sid = tf.story;
            let orig_len = text.len();
            let full_text = lines_out.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>().join("\n");

            let _ = app.run("text.select", json!({"story": sid.0, "anchor": 0, "focus": orig_len}));
            let _ = app.run("text.insert", json!({"text": full_text}));

            let _ = app.session.edit(|doc, _| {
                if let Some(story) = doc.story_mut(sid) {
                    let ranges = story.para_ranges();
                    for (idx, (_, is_italic)) in lines_out.iter().enumerate() {
                        if let Some(r) = ranges.get(idx) {
                            story.format_chars(r.start..r.end, |f| {
                                f.over.size = Some(8.5);
                                f.over.font_style = Some(if *is_italic { "Italic".to_string() } else { "Regular".to_string() });
                            });
                        }
                    }
                    story.rev += 1;
                }
                Ok(serde_json::Value::Null)
            });

            // Fit note frame height and reposition below the table
            if let Some(active) = app.session.active() {
                let (si, pi) = active.doc.page_loc(target.page_idx).unwrap_or((0, 0));
                let margin_y1 = active.doc.spreads.get(si).and_then(|s| s.pages.get(pi)).map(|p| p.margin_rect().y1).unwrap_or(800.0);

                let mut test_doc = (*active.doc).clone();
                let old_rect = test_doc.item(designcraft_doc::ItemId(fid)).map(|it| it.bounds()).unwrap_or(designcraft_geom::Rect::ZERO);
                if let Some(it) = test_doc.item_mut(designcraft_doc::ItemId(fid)) {
                    it.path =
                        designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(old_rect.x0, old_rect.y0, old_rect.x1, old_rect.y0 + 2000.0));
                }
                let cs = designcraft_compose::compose_story(&test_doc, sid, &Default::default());
                let h = cs.frames.first().map(|f| f.content_height).unwrap_or(lines_out.len() as f64 * 16.0);
                let fit_h = (h + 6.0).max(lines_out.len() as f64 * 14.0 + 8.0).max(24.0);

                let _ = app.session.edit(|doc, _| {
                    if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
                        let rect = it.bounds();
                        let target_y0 = if let Some(tbl_y1) = table_new_y1 { (tbl_y1 + 8.0).min(margin_y1 - fit_h) } else { rect.y0 };
                        it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(rect.x0, target_y0, rect.x1, target_y0 + fit_h));
                    }
                    Ok(serde_json::Value::Null)
                });
            }

            notes_updated = true;
            total_replaced += 1;
        }
    }

    // If notes frame exists but was not updated above, ensure it is still pushed below resized table
    if !notes_updated
        && let Some(fid) = target.notes_fid
        && let Some(tbl_y1) = table_new_y1
        && let Some(active) = app.session.active()
    {
        let (si, pi) = active.doc.page_loc(target.page_idx).unwrap_or((0, 0));
        let margin_y1 = active.doc.spreads.get(si).and_then(|s| s.pages.get(pi)).map(|p| p.margin_rect().y1).unwrap_or(800.0);
        let _ = app.session.edit(|doc, _| {
            if let Some(it) = doc.item_mut(designcraft_doc::ItemId(fid)) {
                let rect = it.bounds();
                if rect.y0 < tbl_y1 + 6.0 {
                    let fit_h = rect.height().max(24.0);
                    let target_y0 = (tbl_y1 + 8.0).min(margin_y1 - fit_h);
                    it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(rect.x0, target_y0, rect.x1, target_y0 + fit_h));
                }
            }
            Ok(serde_json::Value::Null)
        });
    }

    total_replaced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_and_short_cells_are_never_sent_to_the_model() {
        assert!(!translatable_cell(""));
        assert!(!translatable_cell("  "));
        assert!(!translatable_cell("50000"));
        assert!(!translatable_cell("3.14"));
        assert!(!translatable_cell("1,234.56"));
        assert!(!translatable_cell("-12"));
        assert!(!translatable_cell("45%"));
        assert!(!translatable_cell("Rp1.234"));
        assert!(!translatable_cell("2024"));
        assert!(!translatable_cell("No"));
        assert!(!translatable_cell("AB"));
        assert!(translatable_cell("Kecamatan Karawang Timur"));
        assert!(translatable_cell("Luas Wilayah"));
    }

    #[test]
    fn summary_aliases_map_to_english_deterministically() {
        assert_eq!(alias_en("Jumlah"), Some("Total"));
        assert_eq!(alias_en("tahun"), Some("Year"));
        assert_eq!(alias_en("Kecamatan"), Some("Subdistrict"));
        assert_eq!(alias_en("Kabupaten Karawang"), None);
    }

    #[test]
    fn pair_cells_keeps_the_shape_and_pairs_translatable_cells() {
        let translations = HashMap::from([("Luas Wilayah".to_string(), "Area".to_string())]);
        let matrix = vec![vec!["No".into(), "Luas Wilayah".into(), "50000".into()], vec!["Jumlah".into(), "".into(), "2024".into()]];
        let paired = pair_cells(&matrix, &translations);
        assert_eq!(paired.len(), matrix.len());
        assert_eq!(paired[0].len(), matrix[0].len());
        assert_eq!(paired[0][0], "No");
        assert_eq!(paired[0][2], "50000");
        assert_eq!(paired[1][0], "Jumlah\u{2028}Total");
        assert_eq!(paired[1][1], "");
        assert_eq!(paired[1][2], "2024");
        let cell = &paired[0][1];
        assert_eq!(cell.matches('\u{2028}').count(), 1, "{cell}");
        assert_eq!(cell.replace('\u{2028}', " "), "Luas Wilayah Area");
    }

    #[test]
    fn test_is_pure_kicker_or_num() {
        assert!(is_pure_kicker_or_num("Tabel"));
        assert!(is_pure_kicker_or_num("Tabel\nTable"));
        assert!(is_pure_kicker_or_num("02.01"));
        assert!(is_pure_kicker_or_num("Table 02.01"));
        assert!(!is_pure_kicker_or_num("Jumlah Penduduk Menurut Desa/Kelurahan"));
        assert!(!is_pure_kicker_or_num("Tabel 02.01 Jumlah Penduduk"));
    }

    #[test]
    fn test_extract_table_title_parts() {
        let (title, prefix) = extract_table_title_parts("Tabel 01.01 Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025");
        assert_eq!(prefix, Some("Tabel 01.01".to_string()));
        assert_eq!(title, "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025");

        let (title2, prefix2) = extract_table_title_parts("Banyaknya Penduduk");
        assert_eq!(prefix2, None);
        assert_eq!(title2, "Banyaknya Penduduk");
    }

    #[test]
    fn test_parse_and_translate_note_items() {
        let (l1, c1, cnt1) = parse_note_line("Keterangan\t:\t-");
        assert_eq!(l1, "Keterangan");
        assert_eq!(c1, ":");
        assert_eq!(cnt1, "-");

        let (l2, c2, cnt2) = parse_note_line("Sumber : BPS Kabupaten Karawang/Dinas terkait");
        assert_eq!(l2, "Sumber");
        assert_eq!(c2, ":");
        assert_eq!(cnt2, "BPS Kabupaten Karawang/Dinas terkait");

        let mut map = HashMap::new();
        map.insert("BPS Kabupaten Karawang/Dinas terkait".into(), "Statistics of Karawang Regency Office/Relevant Agency".into());

        let (en_l1, en_c1, en_cnt1) = translate_note_item(&l1, &c1, &cnt1, &map);
        assert_eq!(en_l1, "Note");
        assert_eq!(en_c1, ":");
        assert_eq!(en_cnt1, "-");

        let (en_l2, en_c2, en_cnt2) = translate_note_item(&l2, &c2, &cnt2, &map);
        assert_eq!(en_l2, "Source");
        assert_eq!(en_c2, ":");
        assert_eq!(en_cnt2, "Statistics of Karawang Regency Office/Relevant Agency");
    }

    #[test]
    fn pair_cells_leaves_untranslated_cells_untouched() {
        let matrix = vec![vec!["Kecamatan Karawang Timur".into()]];
        let paired = pair_cells(&matrix, &HashMap::new());
        assert_eq!(paired[0][0], "Kecamatan Karawang Timur");
    }

    #[test]
    fn write_plain_rtf_escapes_braces_and_newlines() {
        let path = std::env::temp_dir().join(format!("dc-ai-test-{}.rtf", std::process::id()));
        write_plain_rtf("a{b}c\\d\ne", &path).expect("write");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.starts_with("{\\rtf1"), "{content}");
        assert!(content.contains("a\\{b\\}c\\\\d"), "{content}");
        assert!(content.contains("\\par"), "{content}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn collect_translation_units_dedupes_and_skips_numbers() {
        let st = PubgenState {
            card1_records: vec![
                vec!["Tabel".into(), "01.01".into(), "Luas Wilayah".into(), "".into()],
                vec!["Tabel".into(), "01.02".into(), "Luas Wilayah".into(), "".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into(), "file".into()],
            card1_range_all: true,
            cover_title: "KECAMATAN KARAWANG TIMUR".into(),
            cover_title2: "DALAM ANGKA".into(),
            ..Default::default()
        };
        let units = collect_translation_units(&st);
        let texts: Vec<&str> = units.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(texts.len(), texts.iter().collect::<std::collections::HashSet<_>>().len(), "deduped: {texts:?}");
        assert!(texts.contains(&"Luas Wilayah"), "{texts:?}");
        assert!(texts.contains(&"KECAMATAN KARAWANG TIMUR"), "{texts:?}");
        assert!(!texts.iter().any(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit())), "{texts:?}");
    }
}
