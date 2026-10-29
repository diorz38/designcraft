//! AI translation for Pubgen (plan docs/plans/2026-10-22-offline-ai-translation-pubgen.md):
//! the setup dialog, the background translate job, the collect pass, EN-lookup /
//! table-pairing helpers, and page-targeted in-place translation of generated RXX pages.
//! Provider-blind: an online Groq backend or the offline translator behind one \`translate\`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use egui::{Color32, RichText};
use serde_json::{Value, json};

use super::{PubgenState, detect_delimiter, effective_col, parse_csv_content_with_delim, resolve_catalog_related_file};
use crate::DesignApp;
use designcraft_translate as translate;

pub const PROVIDER_OFFLINE: usize = 0;
pub const PROVIDER_GROQ: usize = 1;

pub(crate) const DIALOG_TAB_SETUP: usize = 0;
pub(crate) const DIALOG_TAB_TERMINOLOGY: usize = 1;

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
    pub tab: usize,
    /// Editable vocabulary rows (Indonesian / AI draft / official), seeded from the
    /// resolved glossary, most-collected first.
    pub terms: Vec<GlossaryTerm>,
    pub terms_filter: String,
    pub terms_status: Option<(String, bool)>,
}

impl AiSetupDialog {
    /// Prefill from the live glossary (collection and AI drafts live there) and the
    /// persisted preferences; probe the hardware synchronously (a few microseconds — no
    /// thread needed).
    pub fn open(app: &DesignApp, st: &PubgenState) -> Self {
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
            tab: DIALOG_TAB_SETUP,
            terms: sorted_terms(&st.glossary),
            terms_filter: String::new(),
            terms_status: None,
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
        match t.translate(&["tes".into()], translate::Lang::Indonesian, translate::Lang::English, None, &cancel, &mut |_, _tot| {}) {
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

/// BPS terminology editor: search, edit the three columns (Indonesian / AI draft /
/// official), add or remove rows, reset to the bundled asset and save the vocabulary as
/// the user's glossary (`prefs.pubgen_glossary_path`).
fn terminology_pane(app: &mut DesignApp, st: &mut PubgenState, dialog: &mut AiSetupDialog, ui: &mut egui::Ui) {
    ui.label(
        RichText::new(
            "Official terminology memory. Matching is case-insensitive and whole-cell only: \
             \"Luas Wilayah\" is replaced, \"Luas Wilayah (km²)\" still goes to the model. \
             Official English is what pages use; the AI column is kept for review.",
        )
        .size(10.0),
    );
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.add(egui::TextEdit::singleline(&mut dialog.terms_filter).hint_text("Indonesian or English").desired_width(180.0))
            .accessible_name("Search terminology");
        if ui
            .button("Add term")
            .on_hover_text("Add a row: fill the Indonesian term and the official English — the AI column stays empty for manually added terms.")
            .clicked()
        {
            dialog.terms.push(GlossaryTerm::default());
            dialog.terms_status = None;
        }
        if ui
            .button("Collect Vocabulary")
            .on_hover_text("Walk every generated page and add its Indonesian words and phrases to this list with blank English: table headers, titles, note and source lines, and non-generic prose phrases.")
            .clicked()
        {
            let added = super::vocab::collect_document(app, st);
            st.glossary.terms.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.id.to_lowercase().cmp(&b.id.to_lowercase())));
            dialog.terms = st.glossary.terms.clone();
            dialog.terms_status = Some((
                if added == 0 { "No new terms — the vocabulary already covers the document.".to_string() } else { format!("Collected {added} new term(s) with blank English.") },
                true,
            ));
        }
        #[cfg(not(target_arch = "wasm32"))]
        if ui
            .button("Fill AI Drafts")
            .on_hover_text("Ask the AI for an English draft for every term whose AI and Official columns are empty (up to 200 per click). Your Official column stays under your control — drafts are review evidence only.")
            .clicked()
            && let Some((msg, ok)) = fill_blank_drafts(app, st, ui.ctx().clone())
        {
            dialog.terms_status = Some((msg, ok));
        }
        ui.checkbox(&mut st.auto_fill_ai_drafts, "Auto")
            .on_hover_text("On: after every collection run (generate, insert table, Excel import), blank terms are sent for an AI draft automatically.");
    });
    let needle = dialog.terms_filter.trim().to_lowercase();
    let mut remove: Option<usize> = None;
    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
        egui::Grid::new("pubgen_terminology_grid").num_columns(5).striped(true).show(ui, |ui| {
            ui.label(RichText::new("Indonesian").size(10.0).strong());
            ui.label(RichText::new("AI English").size(10.0).strong());
            ui.label(RichText::new("Official English").size(10.0).strong());
            ui.label(RichText::new("Seen").size(10.0).strong());
            ui.label("");
            ui.end_row();
            for (idx, term) in dialog.terms.iter_mut().enumerate() {
                let hay = format!("{} {} {}", term.id, term.ai, term.official).to_lowercase();
                if !needle.is_empty() && !hay.contains(&needle) {
                    continue;
                }
                ui.add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut term.id)).accessible_name(format!("Term {} Indonesian", idx + 1));
                ui.add_sized([130.0, 22.0], egui::TextEdit::singleline(&mut term.ai)).accessible_name(format!("Term {} AI English", idx + 1));
                ui.add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut term.official))
                    .accessible_name(format!("Term {} Official English", idx + 1));
                ui.label(RichText::new(if term.count > 0 { term.count.to_string() } else { String::new() }).size(10.0).weak());
                if ui.small_button("Remove").clicked() {
                    remove = Some(idx);
                }
                ui.end_row();
            }
        });
    });
    if let Some(idx) = remove {
        if idx < dialog.terms.len() {
            dialog.terms.remove(idx);
        }
        dialog.terms_status = None;
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        #[cfg(not(target_arch = "wasm32"))]
        if ui
            .button("Import CSV…")
            .on_hover_text("Load a two-column glossary sheet (Indonesian, English); the English becomes the official column, Save writes them to the JSON.")
            .clicked()
        {
            let ctx = ui.ctx().clone();
            let _ = app.ask_open("glossary", crate::app::file_dialogs::Bound::App, move |_app, path| {
                super::with_state(&ctx, |st| {
                    if let Some(dialog) = &mut st.ai_dialog {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                let imported = terms_from_csv(&content);
                                if imported.is_empty() {
                                    dialog.terms_status = Some((format!("No terms found in {path} — expected two columns: Indonesian, English."), false));
                                } else {
                                    let count = imported.len();
                                    let (added, updated) = merge_terms(&mut dialog.terms, imported);
                                    let msg = if added + updated == 0 {
                                        format!("The sheet's {count} term(s) already match the list.")
                                    } else {
                                        format!("Imported {added} new term(s) and set {updated} official translation(s) of {count} — Save to keep them.")
                                    };
                                    dialog.terms_status = Some((msg, true));
                                }
                            }
                            Err(err) => dialog.terms_status = Some((format!("Could not read {path}: {err}"), false)),
                        }
                    }
                });
                Ok(serde_json::Value::Null)
            });
        }
        if ui
            .button("Reset to bundled")
            .on_hover_text("Go back to the bundled glossary: official English is refilled from the asset (AI drafts and collected terms are kept); clears the custom path saved in preferences.")
            .clicked()
        {
            // Clearing the path is what makes the bundled terms win again: the preference
            // outranks the asset whenever it is non-empty.
            let _ = app.run("prefs.set", json!({"pubgenGlossaryPath": ""}));
            let bundled = BpsGlossary::load("");
            for t in &mut st.glossary.terms {
                t.official.clear();
            }
            for (id, official) in bundled.prompt_pairs() {
                st.glossary.set_official(&id, &official);
            }
            st.glossary_loaded = true;
            dialog.terms = st.glossary.terms.clone();
            dialog.terms_status =
                Some((format!("Bundled terms restored ({} official term(s)); AI drafts and collected terms are kept.", bundled.terms.len()), true));
        }
        #[cfg(not(target_arch = "wasm32"))]
        if ui.button("Save…").clicked() {
            // Content without an Indonesian term, or an empty list, never reaches the file.
            match glossary_save_error(&dialog.terms) {
                Some(err) => dialog.terms_status = Some((err, false)),
                None => {
                    st.glossary.terms = dialog.terms.clone();
                    let ctx = ui.ctx().clone();
                    let _ = app.ask_save("bps_glossary.json", crate::app::file_dialogs::Bound::App, move |app, path| {
                        super::with_state(&ctx, |st| {
                            match st.glossary.save(std::path::Path::new(&path)) {
                                Ok(()) => {
                                    let _ = app.run("prefs.set", json!({"pubgenGlossaryPath": path}));
                                    st.glossary_loaded = true;
                                    if let Some(dialog) = &mut st.ai_dialog {
                                        dialog.terms_status = Some((format!("Saved {} term(s) to {path}.", st.glossary.terms.len()), true));
                                    }
                                }
                                Err(err) => {
                                    if let Some(dialog) = &mut st.ai_dialog {
                                        dialog.terms_status = Some((format!("Could not save: {err}"), false));
                                    }
                                }
                            }
                        });
                        Ok(serde_json::Value::Null)
                    });
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        ui.label(RichText::new("Importing and saving terminology need the desktop app.").size(10.0));
    });
    if let Some((msg, ok)) = &dialog.terms_status {
        let color = if *ok { Color32::from_rgb(120, 200, 120) } else { Color32::from_rgb(220, 80, 80) };
        ui.label(RichText::new(msg).size(10.0).color(color));
    }
}

/// The glossary terms sorted for the editor: most-seen first, then alphabetical.
fn sorted_terms(g: &BpsGlossary) -> Vec<GlossaryTerm> {
    let mut terms = g.terms.clone();
    terms.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.id.to_lowercase().cmp(&b.id.to_lowercase())));
    terms
}

/// Spawn the AI-draft job for the glossary's blank terms (AI and Official empty), up to
/// 200 per call. Returns the status text to show and whether a job was actually started;
/// `None` means a translation job is already running (the status lines already say so).
pub fn fill_blank_drafts(app: &DesignApp, st: &mut PubgenState, ctx: egui::Context) -> Option<(String, bool)> {
    if st.translate_job.as_ref().is_some_and(|j| !j.finished) {
        return None;
    }
    let Some((provider, key, model)) = backend_from_prefs(app) else {
        return Some(("Configure the AI provider first: click 'AI Translator' to set up your Groq key.".into(), false));
    };
    let blanks: Vec<String> = st
        .glossary
        .terms
        .iter()
        .filter(|t| t.official.trim().is_empty() && t.ai.trim().is_empty() && !t.id.trim().is_empty())
        .map(|t| t.id.clone())
        .collect();
    if blanks.is_empty() {
        return Some(("No blank terms — every term already has an AI draft or an official English.".into(), true));
    }
    let units: Vec<(String, String)> = blanks.iter().take(200).cloned().map(|id| (id.clone(), id)).collect();
    let requested = units.len();
    let remaining = blanks.len().saturating_sub(200);
    let more = if remaining > 0 { format!(" ({remaining} more after this)") } else { String::new() };
    let msg = format!("Requesting AI drafts for {requested} term(s){more}…");
    st.translate_job = Some(TranslateJob::spawn_drafts(&provider, &key, &model, units, st.glossary.prompt_pairs(), ctx));
    st.header_status = Some((msg.clone(), true));
    Some((msg, true))
}

/// Render the modal, when open. Called from the panel's `show`.
pub fn dialog_ui(app: &mut DesignApp, st: &mut PubgenState, ui: &mut egui::Ui) {
    let Some(mut dialog) = st.ai_dialog.take() else { return };
    let mut open = true;
    let mut closed_by_x = false;
    super::dialogs::dialog_window(ui.ctx(), "Translate with AI — setup", crate::i18n::is_rtl(&app.ui.language), false, &mut open, 460.0, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut dialog.tab, DIALOG_TAB_SETUP, "Setup");
            ui.selectable_value(&mut dialog.tab, DIALOG_TAB_TERMINOLOGY, "Terminology");
        });
        ui.separator();

        if dialog.tab == DIALOG_TAB_TERMINOLOGY {
            terminology_pane(app, st, &mut dialog, ui);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    closed_by_x = true;
                }
            });
            // Keep the live in-memory glossary updated with dialog edits immediately
            st.glossary.terms = dialog.terms.clone();
            return;
        }

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
    /// Glossary-draft mode: the units are Indonesian terms and the results fill the
    /// glossary's AI English column instead of the page-translation cache.
    drafts: bool,
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
    pub fn spawn(provider: &str, key: &str, model: &str, units: Vec<(String, String)>, glossary: Vec<(String, String)>, ctx: egui::Context) -> Self {
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
                    match t.translate(
                        &sources,
                        translate::Lang::Indonesian,
                        translate::Lang::English,
                        Some(glossary.as_slice()),
                        &cancel_thread,
                        &mut |_, _| {},
                    ) {
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
        Self { rx: Arc::new(std::sync::Mutex::new(rx)), cancel, drafts: false, done: 0, total, finished: false }
    }

    /// Whether this job was started in glossary-draft mode (filling AI suggestions into terms).
    pub fn is_drafts(&self) -> bool {
        self.drafts
    }

    /// Draft mode: the units are glossary terms (Indonesian → English) and the results
    /// fill the glossary's AI English column, not the page-translation cache.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn_drafts(
        provider: &str,
        key: &str,
        model: &str,
        mut units: Vec<(String, String)>,
        glossary: Vec<(String, String)>,
        ctx: egui::Context,
    ) -> Self {
        let mut job = Self::spawn(provider, key, model, std::mem::take(&mut units), glossary, ctx);
        job.drafts = true;
        job
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn(
        _provider: &str,
        _key: &str,
        _model: &str,
        _units: Vec<(String, String)>,
        _glossary: Vec<(String, String)>,
        _ctx: egui::Context,
    ) -> Self {
        Self { drafts: false, done: 0, total: 0, finished: true }
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
                        if self.drafts {
                            let filled = pairs.iter().filter(|(k, en)| st.glossary.set_ai(k, en)).count();
                            let msg = format!("{filled} AI draft(s) filled from {} term(s) requested.", pairs.len());
                            st.header_status = Some((msg.clone(), true));
                            if let Some(d) = st.ai_dialog.as_mut() {
                                d.terms = sorted_terms(&st.glossary);
                                d.terms_status = Some((msg, true));
                            }
                        } else {
                            for (k, en) in pairs {
                                st.translations.insert(k.clone(), en.clone());
                                st.glossary.set_ai(&k, &en);
                            }
                        }
                        self.finished = true;
                        finished_ok = true;
                    }
                    JobMsg::Failed(err, partial) => {
                        // Keep whatever finished before the failure/cancel; the caller
                        // clears the slice signature so the next run translates only the
                        // rest.
                        if self.drafts {
                            for (k, en) in partial {
                                st.glossary.set_ai(&k, &en);
                            }
                            if let Some(d) = st.ai_dialog.as_mut() {
                                d.terms = sorted_terms(&st.glossary);
                                d.terms_status = Some((format!("AI drafts stopped: {err} — click Fill AI Drafts again for the rest."), false));
                            }
                        } else {
                            for (k, en) in partial {
                                st.translations.insert(k.clone(), en.clone());
                                st.glossary.set_ai(&k, &en);
                            }
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
        if t.is_empty() || !translatable_cell(t, &st.glossary) || seen.contains_key(t) {
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
    matrix.iter().flat_map(|row| row.iter()).filter(|c| translatable_cell(c, &st.glossary)).cloned().collect()
}

// ---------------------------------------------------------------------------
// BPS terminology glossary
// ---------------------------------------------------------------------------

/// Official BPS terminology: the bundled `assets/pubgen/bps_glossary.json` or the user's
/// override (`Prefs.pubgen_glossary_path`). Keys are lowercased at load so the whole-cell
/// One vocabulary entry: the Indonesian term, what the AI last translated it to, and the
/// official English the publication must use. Only `official` is ever applied
/// deterministically or put into the prompt; `ai` is review evidence, so curating a term
/// never loses what the model wrote.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GlossaryTerm {
    /// The Indonesian term, in its first-seen casing; matched case-insensitively.
    pub id: String,
    /// The AI's draft translation, when one has been seen. Empty until a translation runs.
    #[serde(default)]
    pub ai: String,
    /// The official English. Empty means collected, awaiting curation — such terms are
    /// never applied and never enter the prompt, so a draft cannot entrench itself.
    #[serde(default)]
    pub official: String,
    /// How often the collector saw the term; the editor sorts by it.
    #[serde(default)]
    pub count: usize,
}

impl GlossaryTerm {
    fn new(id: &str) -> Self {
        Self { id: id.trim().to_string(), ai: String::new(), official: String::new(), count: 0 }
    }

    /// Case-insensitive term identity: the trimmed, lowercased comparison every caller
    /// (grid edits, collector, corrections) shares.
    pub(crate) fn matches(&self, id: &str) -> bool {
        self.id.trim().to_lowercase() == id.trim().to_lowercase()
    }
}

/// The official-terminology memory: the collected BPS vocabulary and its curated English.
/// Loads from the user's file when set, else the bundled asset, else empty; a legacy
/// `{"exact_mappings": …}` file still loads — its pairs seed the `official` column.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BpsGlossary {
    /// Bundled seed first, then collected terms in collection order.
    #[serde(default)]
    pub terms: Vec<GlossaryTerm>,
}

const BPS_GLOSSARY_FILE: &str = "bps_glossary.json";

impl BpsGlossary {
    /// The configured glossary: the user's file when it loads, else the bundled asset,
    /// else an empty map (translation still runs; only the deterministic aliases go away).
    pub fn load(user_path: &str) -> Self {
        let user_path = user_path.trim();
        if !user_path.is_empty() {
            if let Some(g) = Self::from_file(std::path::Path::new(user_path)) {
                return g;
            }
            eprintln!("pubgen ai: glossary {user_path} cannot be read — falling back to the bundled terms");
        }
        for path in Self::bundled_candidates() {
            if let Some(g) = Self::from_file(&path) {
                return g;
            }
        }
        eprintln!("pubgen ai: bundled {BPS_GLOSSARY_FILE} not found — deterministic terminology is off");
        Self::default()
    }

    /// `<cwd>/assets/pubgen` then `<repo>/assets/pubgen`: tests run with the crate dir as
    /// the working directory, the app from the repo root.
    fn bundled_candidates() -> Vec<PathBuf> {
        let rel = std::path::Path::new("assets").join("pubgen").join(BPS_GLOSSARY_FILE);
        let from_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(&rel);
        let mut candidates = vec![rel.clone(), from_manifest];
        if let Ok(exe) = std::env::current_exe()
            && let Some(exe_dir) = exe.parent()
        {
            candidates.push(exe_dir.join(&rel));
        }
        candidates
    }

    /// Reads the three-column shape (`{"terms": …}`) or the legacy bundled shape
    /// (`{"exact_mappings": …}`, whose pairs become `official` English).
    fn from_file(path: &std::path::Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        let v: Value = serde_json::from_str(&text).ok()?;
        if let Some(list) = v.get("terms").and_then(Value::as_array) {
            let mut terms: Vec<GlossaryTerm> = Vec::with_capacity(list.len());
            for t in list {
                let id = t.get("id").and_then(Value::as_str).unwrap_or("").trim().to_string();
                if id.is_empty() {
                    continue;
                }
                let s = |k: &str| t.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
                terms.push(GlossaryTerm {
                    id,
                    ai: s("ai"),
                    official: s("official"),
                    count: t.get("count").and_then(Value::as_u64).unwrap_or(0) as usize,
                });
            }
            return Some(Self { terms });
        }
        let mut terms: Vec<GlossaryTerm> = Vec::new();
        if let Some(map) = v.get("exact_mappings").and_then(Value::as_object) {
            for (id, en) in map {
                let official = en.as_str().unwrap_or("").trim().to_string();
                if !official.is_empty() {
                    terms.push(GlossaryTerm { official, ..GlossaryTerm::new(id) });
                }
            }
        }
        terms.sort_by_key(|a| a.id.to_lowercase());
        Some(Self { terms })
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    fn find_mut(&mut self, id: &str) -> Option<&mut GlossaryTerm> {
        self.terms.iter_mut().find(|t| t.matches(id))
    }

    /// The official English for a whole cell, case-insensitively; `None` while the term
    /// has no official English yet (a collected or AI-draft-only term is never applied).
    pub fn lookup(&self, cell: &str) -> Option<&str> {
        self.terms.iter().find(|t| t.matches(cell)).filter(|t| !t.official.is_empty()).map(|t| t.official.as_str())
    }

    /// Sorted `(term, official)` pairs: a stable system-prompt addition. AI drafts and
    /// untranslated collected terms stay out — the prompt carries official terms only.
    pub fn prompt_pairs(&self) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> =
            self.terms.iter().filter(|t| !t.official.is_empty()).map(|t| (t.id.clone(), t.official.clone())).collect();
        pairs.sort();
        pairs
    }

    /// Collect a term with a blank English: new terms append, seen ones only count up.
    /// Returns true when the term is new to the vocabulary.
    pub fn collect(&mut self, id: &str) -> bool {
        let id = id.trim();
        if id.is_empty() {
            return false;
        }
        if let Some(t) = self.find_mut(id) {
            t.count += 1;
            return false;
        }
        self.terms.push(GlossaryTerm::new(id));
        true
    }

    /// Record the AI's draft translation of a term; unknown terms are ignored (only the
    /// collector adds vocabulary). Returns true when the draft changed.
    pub fn set_ai(&mut self, id: &str, ai: &str) -> bool {
        let ai = ai.trim();
        if ai.is_empty() {
            return false;
        }
        match self.find_mut(id) {
            Some(t) if t.ai != ai => {
                t.ai = ai.to_string();
                true
            }
            Some(_) => false,
            None => false,
        }
    }

    /// Set the official English, creating the term when unseen. Returns true when the
    /// official text changed.
    pub fn set_official(&mut self, id: &str, official: &str) -> bool {
        let official = official.trim();
        if official.is_empty() {
            return false;
        }
        match self.find_mut(id) {
            Some(t) => {
                if t.official != official {
                    t.official = official.to_string();
                    true
                } else {
                    false
                }
            }
            None => {
                let mut t = GlossaryTerm::new(id);
                t.official = official.to_string();
                self.terms.push(t);
                true
            }
        }
    }

    /// Remove a term (case-insensitive); true when it existed.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.terms.len();
        self.terms.retain(|t| !t.matches(id));
        self.terms.len() != before
    }

    /// Seed from `(Indonesian, official)` pairs — tests and the bundled reset path.
    pub fn from_pairs(pairs: &[(String, String)]) -> Self {
        let mut g = Self::default();
        for (id, official) in pairs {
            g.set_official(id, official);
        }
        g
    }

    /// Write the glossary JSON the loader reads back (`prefs.pubgen_glossary_path`).
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("glossary JSON: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("{}: {e}", path.display()))
    }
}

// ---------------------------------------------------------------------------
// Cell rules and table pairing
// ---------------------------------------------------------------------------

/// Cells never sent to the model (a guard, not a prompt instruction — MT invents digits):
/// numeric/decimal/thousands, currency and %, dates and bare years, one short token,
/// the summary-word aliases (mapped deterministically instead), and empty cells.
pub fn translatable_cell(cell: &str, glossary: &BpsGlossary) -> bool {
    let t = cell.trim();
    if t.is_empty() {
        return false;
    }
    if t.chars().count() <= 2 && !t.contains(' ') {
        return false;
    }
    if alias_en(t, glossary).is_some() {
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

/// Deterministic summary-word aliases: English directly, no MT. Backed by the BPS
/// glossary; extend the list by adding entries to the JSON.
pub fn alias_en<'a>(cell: &str, glossary: &'a BpsGlossary) -> Option<&'a str> {
    glossary.lookup(cell)
}

/// Terms from a two-column glossary sheet (`Indonesian,English`, or tab-separated — the
/// delimiter is detected). A first row naming the columns is skipped, as are rows missing
/// either half. The sheet's casing is kept: the editor shows terms as written.
pub fn terms_from_csv(content: &str) -> Vec<(String, String)> {
    let matrix = parse_csv_content_with_delim(content, detect_delimiter(content));
    let mut out: Vec<(String, String)> = Vec::new();
    for (i, row) in matrix.iter().enumerate() {
        let id = row.first().map(|c| c.trim()).unwrap_or("");
        let en = row.get(1).map(|c| c.trim()).unwrap_or("");
        if id.is_empty() || en.is_empty() {
            continue;
        }
        if i == 0 && looks_like_glossary_header(id, en) {
            continue;
        }
        out.push((id.to_string(), en.to_string()));
    }
    out
}

/// A first row like `Indonesian,English` (or `Istilah,Terjemahan`), not a term.
fn looks_like_glossary_header(id: &str, en: &str) -> bool {
    let label = |s: &str| {
        matches!(
            s.trim().to_lowercase().as_str(),
            "indonesian" | "indonesia" | "id" | "istilah" | "kata" | "english" | "inggris" | "en" | "terjemahan" | "translation"
        )
    };
    label(id) && label(en)
}

/// Merge imported sheet rows into the editor's list: the same Indonesian term (ignoring
/// case and surrounding spaces) takes the imported English as its official, new terms are
/// appended. Returns `(added, updated)`.
pub fn merge_terms(target: &mut Vec<GlossaryTerm>, imported: Vec<(String, String)>) -> (usize, usize) {
    let mut added = 0;
    let mut updated = 0;
    for (id, en) in imported {
        match target.iter_mut().find(|t| t.matches(&id)) {
            Some(slot) => {
                if slot.official != en {
                    updated += 1;
                }
                slot.official = en;
            }
            None => {
                let mut t = GlossaryTerm::new(&id);
                t.official = en;
                target.push(t);
                added += 1;
            }
        }
    }
    (added, updated)
}

/// Why the dialog's rows cannot be saved: content without an Indonesian term, or nothing
/// to write at all. A term with no English yet is valid — it is a collected word awaiting
/// translation or curation.
pub fn glossary_save_error(terms: &[GlossaryTerm]) -> Option<String> {
    for (idx, t) in terms.iter().enumerate() {
        let (id, ai, official) = (t.id.trim(), t.ai.trim(), t.official.trim());
        if id.is_empty() && ai.is_empty() && official.is_empty() {
            continue; // a spare row the user has not filled in yet
        }
        if id.is_empty() {
            return Some(format!("Row {}: add the Indonesian term, or remove the row.", idx + 1));
        }
    }
    (!terms.iter().any(|t| !t.id.trim().is_empty())).then(|| "The glossary is empty — add at least one term before saving.".to_string())
}

/// One table, two lines per cell: \`id\u{2028}en\` for translatable cells with a
/// translation; every skipped cell stays single-line. Same rows × cols by construction.
pub fn pair_cells(matrix: &[Vec<String>], translations: &HashMap<String, String>, glossary: &BpsGlossary) -> Vec<Vec<String>> {
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
                    if let Some(en) = alias_en(t, glossary) {
                        if t.eq_ignore_ascii_case(en) {
                            return cell.clone();
                        }
                        return format!("{t}\u{2028}{en}");
                    }
                    if !translatable_cell(t, glossary) {
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

/// Does `hay` (already lowercased) contain `needle` (a lowercased glossary key) between
/// non-letters, so `kecamatan` fires in "menurut kecamatan" but not inside "kecamatanan".
fn contains_whole_word(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    for (start, _) in hay.match_indices(needle) {
        let end = start + needle.len();
        let before = hay[..start].chars().next_back().is_none_or(|c| !c.is_alphanumeric());
        let after = hay[end..].chars().next().is_none_or(|c| !c.is_alphanumeric());
        if before && after {
            return true;
        }
    }
    false
}

/// Enforce the glossary on the cached translations, so edited official terms reach the page
/// without paying for a full re-translation. A source that *is* a term gets the official
/// English outright; a longer source keeps its translation unless it contains a term whose
/// official English is absent and the Indonesian term itself was not kept — those entries
/// are dropped (and returned) so a re-translation runs exactly them through the
/// glossary-aware prompt. Terms with no official English yet are never enforced.
/// Returns (corrected, sources marked for re-translation).
pub fn apply_glossary_corrections(glossary: &BpsGlossary, translations: &mut HashMap<String, String>) -> (usize, Vec<String>) {
    let mut corrected = 0usize;
    let mut recheck: Vec<String> = Vec::new();
    for (source, en) in translations.iter_mut() {
        if let Some(official) = glossary.lookup(source) {
            if !en.eq_ignore_ascii_case(official) {
                *en = official.to_string();
                corrected += 1;
            }
            continue;
        }
        let low = source.trim().to_lowercase();
        let en_low = en.to_lowercase();
        let wrong = glossary.terms.iter().any(|t| {
            !t.official.is_empty()
                && contains_whole_word(&low, &t.id.to_lowercase())
                && !contains_whole_word(&en_low, &t.id.to_lowercase())
                && !contains_whole_word(&en_low, &t.official.to_lowercase())
        });
        if wrong {
            recheck.push(source.clone());
        }
    }
    for key in &recheck {
        translations.remove(key);
    }
    (corrected, recheck)
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

pub(crate) fn temp_dir() -> PathBuf {
    std::env::temp_dir().join(format!("designcraft-pubgen-{}", std::process::id()))
}

/// Escape one run of text for RTF: backslash, braces; non-ASCII to \`u<dd>?\`; newlines
/// become \`par\`.
pub(crate) fn escape_rtf_text(text: &str) -> String {
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
    body
}

/// Minimal RTF writer around [`escape_rtf_text`].
pub fn write_plain_rtf(text: &str, path: &std::path::Path) -> Result<(), String> {
    let body = escape_rtf_text(text);
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

/// " · Groq: ~N requests left today" when the last Groq response reported its
/// rate-limit header this session; empty string otherwise (offline provider, or
/// a build without `ai-translate-online`).
pub fn groq_quota_suffix() -> String {
    match translate::remaining_requests() {
        Some(n) => format!(" · Groq: ~{n} requests left today"),
        None => String::new(),
    }
}

/// Queue the usage popup after a Groq-backed AI operation completes: Groq's own
/// rate-limit header (per key, server-computed), or an explicit "unavailable"
/// line so the user always gets feedback that AI ran.
pub(crate) fn push_groq_quota_popup(st: &mut PubgenState) {
    let line = match translate::remaining_requests() {
        Some(n) => format!("Groq AI: ~{n} requests left today"),
        None => "Groq AI: rate-limit info unavailable this session".to_string(),
    };
    st.ai_quota_popup = Some(vec![line]);
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
pub fn translate_note_item(
    label: &str,
    colon: &str,
    content: &str,
    map: &HashMap<String, String>,
    glossary: &BpsGlossary,
) -> (String, String, String) {
    let lower = label.to_lowercase();
    let label_en = if let Some(en) = glossary.lookup(label) {
        en.to_string()
    } else if lower.starts_with("keterangan")
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
    } else if let Some(official) = glossary.lookup(content_trimmed) {
        official.to_string()
    } else if let Some(ai_tr) = map.get(content_trimmed) {
        ai_tr.clone()
    } else if content_trimmed.contains('/') {
        let parts: Vec<String> = content_trimmed
            .split('/')
            .map(|p| {
                let pt = p.trim();
                glossary.lookup(pt).map(|s| s.to_string()).or_else(|| map.get(pt).cloned()).unwrap_or_else(|| pt.to_string())
            })
            .collect();
        parts.join("/")
    } else {
        content.to_string()
    };

    (label_en, colon_en, content_en)
}

/// Collect translatable units from a table page.
pub fn collect_table_page_units(app: &DesignApp, target: &TablePageTarget, glossary: &BpsGlossary) -> Vec<(String, String)> {
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
            if translatable_cell(&clean_title, glossary) {
                push(&mut out, &mut seen, &clean_title);
            }
            if clean_title != *first && translatable_cell(first, glossary) {
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
                    if translatable_cell(text, glossary) {
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
            if !c_trimmed.is_empty() && c_trimmed != "-" && translatable_cell(c_trimmed, glossary) {
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
                let en_opt = alias_en(&cur, &st.glossary).map(|s| s.to_string()).or_else(|| map.get(&cur).cloned());
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
            let (label_en, colon_en, content_en) = translate_note_item(&label, &colon, &content, map, &st.glossary);
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

/// Translates the currently active/selected page in place using AI or cached translations.
/// Automatically detects whether the page contains an RXX review chapter or an XX_XX data table.
pub fn execute_translate_selected_page(app: &mut DesignApp, st: &mut PubgenState, ctx: egui::Context) {
    let page_idx = app.session.active().and_then(|s| s.selection.page).or_else(|| crate::canvas::current_page(app));
    let Some(page_idx) = page_idx else {
        st.header_status = Some(("No active page selected.".into(), false));
        return;
    };
    let review_target = detect_page_target(app, page_idx);
    let table_target = detect_table_page_target(app, page_idx);

    if let Some(target) = table_target {
        let mut units = collect_table_page_units(app, &target, &st.glossary);
        units.retain(|(k, _)| !st.translations.contains_key(k));
        if let Some((provider, key, model)) = backend_from_prefs(app) {
            if !units.is_empty() {
                st.ai_table_target = Some(target);
                let terms = st.glossary.prompt_pairs();
                st.translate_job = Some(TranslateJob::spawn(&provider, &key, &model, units, terms, ctx));
                st.header_status = Some(("Translating table page with AI…".into(), true));
            } else {
                apply_table_page_result(app, st, &target);
                st.header_status = Some(("Table page translated from cache.".into(), true));
            }
        } else if units.is_empty() {
            apply_table_page_result(app, st, &target);
            st.header_status = Some(("Table page translated from cache.".into(), true));
        } else {
            st.header_status = Some(("AI translation: no backend configured — open Setup to configure API key.".into(), false));
        }
    } else if let Some(target) = review_target {
        let mut units = collect_page_units(app, &target);
        units.retain(|(k, _)| !st.translations.contains_key(k));
        if let Some((provider, key, model)) = backend_from_prefs(app) {
            st.ai_page_target = Some(target.clone());
            let terms = st.glossary.prompt_pairs();
            st.translate_job = Some(TranslateJob::spawn(&provider, &key, &model, units, terms, ctx));
            st.header_status = Some(("Translating review page with AI…".into(), true));
        } else if units.is_empty() {
            apply_page_result(app, st, &target);
            st.header_status = Some(("Review page translated from cache.".into(), true));
        } else {
            st.header_status = Some(("AI translation: no backend configured — open Setup to configure API key.".into(), false));
        }
    } else {
        st.header_status = Some(("Selected page is not a recognized Review or Table page.".into(), false));
    }
}

/// Batch translates all detected table pages within the specified page range `[from..=to_inclusive]` (0-indexed).
pub fn execute_translate_table_range(app: &mut DesignApp, st: &mut PubgenState, from: usize, to_inclusive: usize, ctx: egui::Context) {
    let mut targets = Vec::new();
    for p in from..=to_inclusive {
        if let Some(t) = detect_table_page_target(app, p) {
            targets.push(t);
        }
    }
    if targets.is_empty() {
        st.header_status = Some(("No table pages found in selected range.".into(), false));
        return;
    }
    let mut all_units = Vec::new();
    for t in &targets {
        all_units.extend(collect_table_page_units(app, t, &st.glossary));
    }
    all_units.retain(|(k, _)| !st.translations.contains_key(k));
    all_units.sort_by(|a, b| a.0.cmp(&b.0));
    all_units.dedup_by(|a, b| a.0 == b.0);

    if let Some((provider, key, model)) = backend_from_prefs(app) {
        if !all_units.is_empty() {
            st.ai_table_targets_batch = targets;
            let terms = st.glossary.prompt_pairs();
            st.translate_job = Some(TranslateJob::spawn(&provider, &key, &model, all_units, terms, ctx));
            st.header_status = Some(("Translating table pages with AI…".into(), true));
        } else {
            let mut count = 0;
            for t in &targets {
                count += apply_table_page_result(app, st, t);
            }
            st.header_status = Some((format!("{} table item(s) across {} page(s) translated from cache.", count, targets.len()), true));
        }
    } else if all_units.is_empty() {
        let mut count = 0;
        for t in &targets {
            count += apply_table_page_result(app, st, t);
        }
        st.header_status = Some((format!("{} table item(s) across {} page(s) translated from cache.", count, targets.len()), true));
    } else {
        st.header_status = Some(("AI translation: no backend configured — open Setup to configure API key.".into(), false));
    }
}

/// Applies glossary terms to existing translations on the active review or table page.
pub fn execute_fix_terms(app: &mut DesignApp, st: &mut PubgenState) {
    let page_idx = app.session.active().and_then(|s| s.selection.page).or_else(|| crate::canvas::current_page(app));
    let Some(page_idx) = page_idx else {
        return;
    };
    let review_target = detect_page_target(app, page_idx);
    let table_target = detect_table_page_target(app, page_idx);
    let (corrected, recheck) = apply_glossary_corrections(&st.glossary, &mut st.translations);
    if let Some(target) = table_target.as_ref() {
        apply_table_page_result(app, st, target);
    } else if let Some(target) = review_target.as_ref() {
        apply_page_result(app, st, target);
    }
    st.header_status = Some(if recheck.is_empty() {
        if corrected == 0 {
            ("The glossary already agrees with this page's translations.".into(), true)
        } else {
            (format!("Glossary applied: {corrected} term(s) corrected on the page."), true)
        }
    } else {
        (format!("Glossary applied: {corrected} term(s) corrected; {} text(s) marked for re-translation.", recheck.len()), true)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glossary() -> BpsGlossary {
        BpsGlossary::load("")
    }

    #[test]
    fn translate_job_drafts_flag_prevents_unwanted_re_generation() {
        let job_normal = TranslateJob {
            #[cfg(not(target_arch = "wasm32"))]
            rx: Arc::new(std::sync::Mutex::new(std::sync::mpsc::channel().1)),
            #[cfg(not(target_arch = "wasm32"))]
            cancel: Arc::new(AtomicBool::new(false)),
            drafts: false,
            done: 0,
            total: 0,
            finished: true,
        };
        assert!(!job_normal.is_drafts());

        let job_drafts = TranslateJob {
            #[cfg(not(target_arch = "wasm32"))]
            rx: Arc::new(std::sync::Mutex::new(std::sync::mpsc::channel().1)),
            #[cfg(not(target_arch = "wasm32"))]
            cancel: Arc::new(AtomicBool::new(false)),
            drafts: true,
            done: 0,
            total: 0,
            finished: true,
        };
        assert!(job_drafts.is_drafts(), "job in drafts mode must return is_drafts = true");
    }

    #[test]
    fn quota_popup_always_reports_something_after_a_groq_operation() {
        let mut st = PubgenState::default();
        push_groq_quota_popup(&mut st);
        let lines = st.ai_quota_popup.expect("popup queued");
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with("Groq AI: "), "{lines:?}");
    }

    #[test]
    fn numeric_and_short_cells_are_never_sent_to_the_model() {
        let g = glossary();
        assert!(!translatable_cell("", &g));
        assert!(!translatable_cell("  ", &g));
        assert!(!translatable_cell("50000", &g));
        assert!(!translatable_cell("3.14", &g));
        assert!(!translatable_cell("1,234.56", &g));
        assert!(!translatable_cell("-12", &g));
        assert!(!translatable_cell("45%", &g));
        assert!(!translatable_cell("Rp1.234", &g));
        assert!(!translatable_cell("2024", &g));
        assert!(!translatable_cell("No", &g));
        assert!(!translatable_cell("AB", &g));
        assert!(translatable_cell("Kecamatan Karawang Timur", &g));
        assert!(translatable_cell("Luas Sawah Irigasi", &g));
        assert!(!translatable_cell("Luas Wilayah", &g), "a whole-cell glossary term is aliased, not machine-translated");
    }

    #[test]
    fn summary_aliases_map_to_english_deterministically() {
        let g = glossary();
        assert_eq!(alias_en("Jumlah", &g), Some("Total"));
        assert_eq!(alias_en("tahun", &g), Some("Year"));
        assert_eq!(alias_en("Kecamatan", &g), Some("Subdistrict"));
        assert_eq!(alias_en("Kabupaten Karawang", &g), None);
    }

    #[test]
    fn bundled_glossary_covers_every_roadmap_term() {
        let g = glossary();
        let expected = [
            ("Kecamatan", "Subdistrict"),
            ("Kabupaten", "Regency"),
            ("Desa", "Village"),
            ("Kelurahan", "Urban Village"),
            ("Luas Wilayah", "Total Area"),
            ("Kepadatan Penduduk", "Population Density"),
            ("Jenis Kelamin", "Sex"),
            ("Laki-laki", "Male"),
            ("Perempuan", "Female"),
            ("PDRB", "GRDP"),
            ("Laju Pertumbuhan", "Growth Rate"),
            ("Tahun", "Year"),
            ("Sumber", "Source"),
            ("Catatan", "Note"),
            ("Keterangan", "Note"),
        ];
        for (id, en) in expected {
            assert_eq!(alias_en(id, &g), Some(en), "{id}");
        }
    }

    #[test]
    fn bundled_glossary_keeps_the_legacy_spellings_with_their_english() {
        // The parity gate: every spelling the pre-glossary hardcoded map resolved must still
        // resolve, to the same English it did then — not merely to something.
        let g = glossary();
        for (id, en) in [
            ("jumlah", "Total"),
            ("total", "Total"),
            ("tahun", "Year"),
            ("kabupaten", "Regency"),
            ("kecamatan", "Subdistrict"),
            ("kec", "Subdistrict"),
            ("provinsi", "Province"),
            ("propinsi", "Province"),
            ("keterangan", "Note"),
            ("ket.", "Note"),
            ("ket", "Note"),
            ("catatan", "Note"),
            ("sumber", "Source"),
            ("no", "No"),
        ] {
            assert_eq!(alias_en(id, &g), Some(en), "legacy spelling {id}");
        }
    }

    #[test]
    fn saving_is_refused_for_a_termless_row_or_an_empty_list() {
        let empty = Some("The glossary is empty — add at least one term before saving.".to_string());
        let term = |id: &str, ai: &str, official: &str| GlossaryTerm { id: id.into(), ai: ai.into(), official: official.into(), count: 0 };
        assert_eq!(glossary_save_error(&[term("Kecamatan", "", "Subdistrict")]), None);
        assert_eq!(glossary_save_error(&[]), empty);
        assert_eq!(glossary_save_error(&[term("", "", "")]), empty);
        assert_eq!(glossary_save_error(&[term("", "", "Subdistrict")]), Some("Row 1: add the Indonesian term, or remove the row.".to_string()));
        // A collected term with no English yet is a valid row: it awaits translation.
        assert_eq!(glossary_save_error(&[term("Desa", "", "")]), None);
        // A spare blank row among good ones is not an error.
        let terms = vec![term("Desa", "", "Village"), term("", "", "")];
        assert_eq!(glossary_save_error(&terms), None);
    }

    #[test]
    fn lookup_is_case_insensitive_on_the_whole_cell() {
        let g = glossary();
        assert_eq!(alias_en("KECAMATAN", &g), Some("Subdistrict"));
        assert_eq!(alias_en("  Jumlah \t", &g), Some("Total"));
        assert_eq!(alias_en("kEcAmAtAn", &g), Some("Subdistrict"));
        assert_eq!(alias_en("Kecamatan Karawang", &g), None);
        assert_eq!(alias_en("Jumlah Penduduk", &g), None);
        assert_eq!(alias_en("Total Luas", &g), None);
        assert_eq!(alias_en("Luas Wilayah", &g), Some("Total Area"));
        assert_eq!(alias_en("Luas Wilayah (km²)", &g), None, "multi-word entries match the whole cell only");
    }

    #[test]
    fn user_glossary_overrides_the_bundled_terms() {
        let path = std::env::temp_dir().join(format!("dc-glossary-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"exact_mappings":{"Wilayah":"Territory","Jumlah":"Sum"}}"#).expect("write");
        let g = BpsGlossary::load(&path.to_string_lossy());
        let _ = std::fs::remove_file(&path);
        assert_eq!(alias_en("Wilayah", &g), Some("Territory"));
        assert_eq!(alias_en("Jumlah", &g), Some("Sum"), "user override wins");
        assert_eq!(alias_en("Sumber", &g), None, "bundled terms are replaced, not merged");
    }

    #[test]
    fn glossary_corrections_fix_whole_cells_and_flag_wrong_prose() {
        let g = BpsGlossary::from_pairs(&[("kecamatan".to_string(), "Subdistrict".to_string()), ("jumlah".to_string(), "Total".to_string())]);
        let mut t = HashMap::new();
        // A whole-cell source is corrected outright: the AI or an older glossary wrote this.
        t.insert("Kecamatan".to_string(), "District".to_string());
        // Prose whose translation lost the official term is marked for re-translation.
        t.insert("Jumlah penduduk menurut kecamatan".to_string(), "Total population by district".to_string());
        // A kept Indonesian proper noun is left alone.
        t.insert("Kecamatan Karawang Timur".to_string(), "Kecamatan Karawang Timur".to_string());
        // Already-official text is untouched.
        t.insert("Jumlah penduduk".to_string(), "Total population".to_string());
        let (corrected, recheck) = apply_glossary_corrections(&g, &mut t);
        assert_eq!((corrected, recheck.len()), (1, 1));
        assert_eq!(t.get("Kecamatan").map(String::as_str), Some("Subdistrict"));
        assert!(t.contains_key("Kecamatan Karawang Timur"), "a kept proper noun stays");
        assert!(t.contains_key("Jumlah penduduk"), "already-official text stays");
        assert!(!t.contains_key("Jumlah penduduk menurut kecamatan"), "wrong prose is dropped for a glossary-aware re-run");
    }

    #[test]
    fn glossary_corrections_handle_multi_word_terms_and_agreeing_translations() {
        let g = BpsGlossary::from_pairs(&[("luas wilayah".to_string(), "Total Area".to_string())]);
        // The term fires as a whole phrase, the official English is missing: re-check.
        let mut t = HashMap::new();
        t.insert("Luas Wilayah (km\u{b2})".to_string(), "Region Size (km\u{b2})".to_string());
        let (corrected, recheck) = apply_glossary_corrections(&g, &mut t);
        assert_eq!((corrected, recheck.len()), (0, 1));
        assert_eq!(recheck, vec!["Luas Wilayah (km\u{b2})".to_string()]);
        assert!(!t.contains_key("Luas Wilayah (km\u{b2})"));
        // Term present (case-insensitively) and the official English already there: no-op.
        let mut t = HashMap::new();
        t.insert("luas wilayah".to_string(), "Total Area".to_string());
        let (corrected, recheck) = apply_glossary_corrections(&g, &mut t);
        assert_eq!((corrected, recheck.len()), (0, 0));
        assert_eq!(t.get("luas wilayah").map(String::as_str), Some("Total Area"));
    }

    #[test]
    fn malformed_or_missing_user_glossary_falls_back_to_the_bundled_terms() {
        let path = std::env::temp_dir().join(format!("dc-glossary-bad-{}.json", std::process::id()));
        std::fs::write(&path, "{ not json").expect("write");
        let g = BpsGlossary::load(&path.to_string_lossy());
        let _ = std::fs::remove_file(&path);
        assert_eq!(alias_en("Jumlah", &g), Some("Total"), "malformed user file falls back to the bundled asset");

        let missing = std::env::temp_dir().join(format!("dc-glossary-missing-{}.json", std::process::id()));
        let g = BpsGlossary::load(&missing.to_string_lossy());
        assert_eq!(alias_en("Jumlah", &g), Some("Total"), "missing user file falls back to the bundled asset");
    }

    #[test]
    fn saved_glossary_round_trips_through_the_loader() {
        let path = std::env::temp_dir().join(format!("dc-glossary-saved-{}.json", std::process::id()));
        let mut glossary = BpsGlossary::from_pairs(&[("Kustom".to_string(), "Custom".to_string())]);
        glossary.collect("Warungbambu");
        glossary.set_ai("Warungbambu", "Warungbambu (AI draft)");
        glossary.save(&path).expect("save");
        let loaded = BpsGlossary::load(&path.to_string_lossy());
        let _ = std::fs::remove_file(&path);
        assert_eq!(alias_en("KUSTOM", &loaded), Some("Custom"));
        assert_eq!(loaded.terms.len(), 2);
        let collected = loaded.terms.iter().find(|t| t.matches("Warungbambu")).expect("collected term kept");
        assert_eq!(collected.ai, "Warungbambu (AI draft)", "the AI draft survives the round trip");
        assert_eq!(collected.official, "", "an unofficial term is never applied");
        assert_eq!(alias_en("Warungbambu", &loaded), None);
    }

    #[test]
    fn prompt_pairs_are_sorted_for_a_stable_system_prompt() {
        let g = BpsGlossary::load("");
        let pairs = g.prompt_pairs();
        assert!(!pairs.is_empty());
        let mut sorted = pairs.clone();
        sorted.sort();
        assert_eq!(pairs, sorted);
    }

    #[test]
    fn pair_cells_keeps_the_shape_and_pairs_translatable_cells() {
        let g = glossary();
        let translations = HashMap::from([("Luas Sawah Irigasi".to_string(), "Irrigated Rice Field Area".to_string())]);
        let matrix = vec![vec!["No".into(), "Luas Sawah Irigasi".into(), "50000".into()], vec!["Jumlah".into(), "".into(), "2024".into()]];
        let paired = pair_cells(&matrix, &translations, &g);
        assert_eq!(paired.len(), matrix.len());
        assert_eq!(paired[0].len(), matrix[0].len());
        assert_eq!(paired[0][0], "No");
        assert_eq!(paired[0][2], "50000");
        assert_eq!(paired[1][0], "Jumlah\u{2028}Total");
        assert_eq!(paired[1][1], "");
        assert_eq!(paired[1][2], "2024");
        let cell = &paired[0][1];
        assert_eq!(cell.matches('\u{2028}').count(), 1, "{cell}");
        assert_eq!(cell.replace('\u{2028}', " "), "Luas Sawah Irigasi Irrigated Rice Field Area");
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

        let g = glossary();
        let (en_l1, en_c1, en_cnt1) = translate_note_item(&l1, &c1, &cnt1, &map, &g);
        assert_eq!(en_l1, "Note");
        assert_eq!(en_c1, ":");
        assert_eq!(en_cnt1, "-");

        let (en_l2, en_c2, en_cnt2) = translate_note_item(&l2, &c2, &cnt2, &map, &g);
        assert_eq!(en_l2, "Source");
        assert_eq!(en_c2, ":");
        assert_eq!(en_cnt2, "Statistics of Karawang Regency Office/Relevant Agency");

        // Verify that glossary lookup overrides fallback map translation for note content
        let mut custom_g = glossary();
        custom_g.terms.push(GlossaryTerm {
            id: "BPS Kabupaten Karawang".into(),
            ai: String::new(),
            official: "Central Statistics Agency Of Karawang Regency".into(),
            count: 1,
        });
        let mut ai_map = HashMap::new();
        ai_map.insert("BPS Kabupaten Karawang".into(), "BPS Karawang Regency".into());
        let (en_l3, en_c3, en_cnt3) = translate_note_item("Sumber", ":", "BPS Kabupaten Karawang", &ai_map, &custom_g);
        assert_eq!(en_l3, "Source");
        assert_eq!(en_c3, ":");
        assert_eq!(en_cnt3, "Central Statistics Agency Of Karawang Regency");
    }

    #[test]
    fn pair_cells_leaves_untranslated_cells_untouched() {
        let matrix = vec![vec!["Kecamatan Karawang Timur".into()]];
        let paired = pair_cells(&matrix, &HashMap::new(), &glossary());
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
                vec!["Tabel".into(), "01.01".into(), "Produksi Padi".into(), "".into()],
                vec!["Tabel".into(), "01.02".into(), "Produksi Padi".into(), "".into()],
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
        assert!(texts.contains(&"Produksi Padi"), "{texts:?}");
        assert!(texts.contains(&"KECAMATAN KARAWANG TIMUR"), "{texts:?}");
        assert!(!texts.iter().any(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit())), "{texts:?}");
    }

    #[test]
    fn glossary_sheet_rows_skip_the_header_and_incomplete_rows() {
        let csv = "Indonesian,English\nKecamatan,Subdistrict\n\nDesa,\nJumlah,Total\n";
        let terms = terms_from_csv(csv);
        assert_eq!(terms, vec![("Kecamatan".to_string(), "Subdistrict".to_string()), ("Jumlah".to_string(), "Total".to_string())]);
    }

    #[test]
    fn glossary_sheet_accepts_tabs_and_only_drops_a_real_header() {
        let tsv = "Istilah\tTerjemahan\nKecamatan\tSubdistrict\nEnglish, Notes\tKeterangan\n";
        let terms = terms_from_csv(tsv);
        assert_eq!(terms, vec![("Kecamatan".to_string(), "Subdistrict".to_string()), ("English, Notes".to_string(), "Keterangan".to_string())]);
        let no_header = terms_from_csv("Kabupaten,Regency\nDesa,Village\n");
        assert_eq!(no_header.len(), 2);
    }

    #[test]
    fn merge_terms_overrides_by_case_insensitive_key_and_appends_the_rest() {
        let mut target = vec![GlossaryTerm::new("Jumlah"), GlossaryTerm::new("Desa")];
        target[0].official = "Total".to_string();
        target[1].official = "Village".to_string();
        let imported = vec![
            ("jumlah".to_string(), "Sum".to_string()),
            ("desa".to_string(), "Village".to_string()),
            ("Wilayah".to_string(), "Territory".to_string()),
        ];
        let (added, updated) = merge_terms(&mut target, imported);
        assert_eq!((added, updated), (1, 1));
        assert_eq!(target.len(), 3);
        assert_eq!(target[0].official, "Sum", "the imported English wins on the existing term");
        assert_eq!(target[2].id, "Wilayah");
        assert_eq!(target[2].official, "Territory");
    }

    #[test]
    fn merge_terms_counts_nothing_when_the_sheet_matches() {
        let mut target = vec![GlossaryTerm::new("Kecamatan")];
        target[0].official = "Subdistrict".to_string();
        let (added, updated) = merge_terms(&mut target, vec![("Kecamatan".to_string(), "Subdistrict".to_string())]);
        assert_eq!((added, updated), (0, 0));
        assert_eq!(target.len(), 1);
    }

    #[test]
    fn example_glossary_sheet_imports_into_the_editor() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/pubgen-glossary.example.csv");
        let content = std::fs::read_to_string(path).expect("the example sheet ships with the docs");
        let imported = terms_from_csv(&content);
        assert_eq!(imported.len(), 23, "{imported:?}");
        let mut target = vec![];
        let (added, updated) = merge_terms(&mut target, imported);
        assert_eq!((added, updated), (23, 0));
        let glossary = BpsGlossary { terms: target };
        let path = std::env::temp_dir().join(format!("dc-glossary-example-{}.json", std::process::id()));
        glossary.save(&path).expect("save");
        let g = BpsGlossary::load(&path.to_string_lossy());
        let _ = std::fs::remove_file(&path);
        assert_eq!(alias_en("Keterangan", &g), Some("Note"));
        assert_eq!(alias_en("kec", &g), Some("Subdistrict"));
    }
}

#[cfg(test)]
pub(crate) mod flow_tests {
    use super::super::ai;
    use super::super::*;

    #[test]
    fn test_table_page_ai_translation_end_to_end() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Create Title frame on page 0
        let res_title = app
            .run(
                "frame.create",
                json!({
                    "spread": 0,
                    "rect": [36.0, 36.0, 400.0, 60.0],
                    "content": "text",
                    "text": "Tabel 01.01 Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025",
                    "caret": false
                }),
            )
            .unwrap();
        let _title_fid = res_title["id"].as_u64().unwrap();

        // 2. Insert table with note rows on page 0
        let matrix = vec![
            vec!["Desa/Kelurahan".into(), "Luas (km2)".into(), "Persentase".into()],
            vec!["Warungbambu".into(), "1.65".into(), "21.32".into()],
            vec!["Jumlah".into(), "1.65".into(), "21.32".into()],
            vec!["Keterangan".into(), ":".into(), "-".into()],
            vec!["Sumber".into(), ":".into(), "BPS Kabupaten Karawang/Dinas terkait".into()],
        ];
        let st = PubgenState { card2_placement: 1, card2_first_row_header: true, card2_table_footer: true, ..Default::default() };
        let ins_res = execute_insert_table_matrix(&mut app, &st, Some(&matrix), Some(0));
        assert!(ins_res.is_ok(), "Table insert should succeed: {ins_res:?}");

        // 3. Detect table page target
        let target_opt = ai::detect_table_page_target(&app, 0);
        assert!(target_opt.is_some(), "Should detect TablePageTarget on page 0");
        let target = target_opt.unwrap();
        assert!(target.title_fid.is_some(), "Must detect title frame");
        assert!(target.notes_fid.is_some(), "Must detect notes frame");

        // 4. Collect units
        let units = ai::collect_table_page_units(&app, &target, &st.glossary);
        let texts: Vec<&str> = units.iter().map(|(k, _)| k.as_str()).collect();
        assert!(
            texts.contains(&"Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025"),
            "Units must contain clean title: {texts:?}"
        );
        assert!(texts.contains(&"Desa/Kelurahan"), "Units must contain header cell: {texts:?}");
        assert!(texts.contains(&"Luas (km2)"), "Units must contain header cell: {texts:?}");
        assert!(texts.contains(&"Warungbambu"), "Units must contain data label: {texts:?}");
        assert!(texts.contains(&"BPS Kabupaten Karawang/Dinas terkait"), "Units must contain source text: {texts:?}");
        // Numbers and '-' must NEVER be collected
        assert!(!texts.contains(&"1.65"), "Must not contain numbers");
        assert!(!texts.contains(&"21.32"), "Must not contain numbers");
        assert!(!texts.contains(&"-"), "Must not contain dash");

        // 5. Populate translations map
        let mut translate_st = PubgenState::default();
        translate_st.translations.insert(
            "Luas Daerah Menurut Desa/Kelurahan di Kecamatan  Karawang Timur, 2025".into(),
            "Area by Village/Urban Village in Karawang Timur Subdistrict, 2025".into(),
        );
        translate_st.translations.insert("Desa/Kelurahan".into(), "Village/Urban Village".into());
        translate_st.translations.insert("Luas (km2)".into(), "Area (sq km)".into());
        translate_st.translations.insert("Persentase".into(), "Percentage".into());
        translate_st.translations.insert("Warungbambu".into(), "Warungbambu".into());
        translate_st
            .translations
            .insert("BPS Kabupaten Karawang/Dinas terkait".into(), "Statistics of Karawang Regency Office/Relevant Agency".into());

        // 6. Apply table page result
        let replaced = ai::apply_table_page_result(&mut app, &translate_st, &target);
        assert!(replaced > 0, "Must replace items: {replaced}");

        // 7. Verify Title: Line 1 Regular, Line 2 Italic
        let doc = &app.session.active().unwrap().doc;
        let title_it = doc.item(designcraft_doc::ItemId(target.title_fid.unwrap())).unwrap();
        let title_story = doc.story(title_it.text_frame().unwrap().story).unwrap();
        assert!(title_story.text.contains("Tabel 01.01 Luas Daerah"));
        assert!(title_story.text.contains("Table 01.01 Area by Village/Urban Village"));
        let title_lines: Vec<&str> = title_story.text.lines().collect();
        assert_eq!(title_lines.len(), 2, "Title must have 2 lines");
        let line2_pos = title_lines[0].len() + 1;
        assert_eq!(title_story.format_after(line2_pos).over.font_style.as_deref(), Some("Italic"));

        // 8. Verify Table cells: in same cell new line [italic]
        let table_story = doc.story(target.table_sid).unwrap();
        let table = table_story.tables.get(&target.table_id).unwrap();
        let header_cell = table.cell(0, 0).unwrap();
        assert!(header_cell.text.text.contains("Desa/Kelurahan\nVillage/Urban Village"));
        let en_pos = "Desa/Kelurahan\n".len();
        assert_eq!(header_cell.text.format_after(en_pos).over.font_style.as_deref(), Some("Italic"));

        let col_num_cell = table.cell(1, 0).unwrap();
        assert_eq!(col_num_cell.text.text.trim(), "(1)", "Column numbers row must be untouched");

        let jumlah_cell = table.cell(3, 0).unwrap();
        assert!(jumlah_cell.text.text.contains("Jumlah\nTotal"));
        let total_pos = "Jumlah\n".len();
        assert_eq!(jumlah_cell.text.format_after(total_pos).over.font_style.as_deref(), Some("Italic"));

        let num_cell = table.cell(2, 1).unwrap();
        assert_eq!(num_cell.text.text.trim(), "1.65", "Numeric cell must remain untouched");

        // 9. Verify Notes:
        // Keterangan : -
        // [italic]Note: -
        // Sumber: BPS Kabupaten Karawang/Dinas terkait
        // [italic]Source : Statistics of Karawang Regency Office/Relevant Agency
        let note_it = doc.item(designcraft_doc::ItemId(target.notes_fid.unwrap())).unwrap();
        let note_story = doc.story(note_it.text_frame().unwrap().story).unwrap();
        let note_lines: Vec<&str> = note_story.text.lines().collect();
        assert_eq!(note_lines.len(), 4, "Must have 4 note lines: {note_lines:?}");
        assert!(note_lines[0].starts_with("Keterangan"));
        assert!(note_lines[1].starts_with("Note"));
        assert!(note_lines[2].starts_with("Sumber"));
        assert!(note_lines[3].starts_with("Source"));

        let note_ranges = note_story.para_ranges();
        assert_ne!(note_story.format_after(note_ranges[0].start).over.font_style.as_deref(), Some("Italic"));
        assert_eq!(note_story.format_after(note_ranges[1].start).over.font_style.as_deref(), Some("Italic"));
        assert_ne!(note_story.format_after(note_ranges[2].start).over.font_style.as_deref(), Some("Italic"));
        assert_eq!(note_story.format_after(note_ranges[3].start).over.font_style.as_deref(), Some("Italic"));
    }
}
