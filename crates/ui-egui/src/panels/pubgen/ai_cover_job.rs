//! Background job for generating AI Base cover backgrounds via Cloudflare Workers AI.
//!
//! Mirrors `TranslateJob` in `ai.rs`: spawns a background thread that performs HTTP
//! requests and writes the resulting PNG to disk. The UI thread mutates the document
//! through `template.page.apply` via `poll()` on the UI loop.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use designcraft_imagegen as imagegen;

use super::PubgenState;

/// Messages passed from worker thread to UI thread.
#[derive(Debug)]
pub enum CoverJobMsg {
    Progress(usize, usize),
    ItemDone { index: usize, title: String, path: PathBuf },
    Done(PathBuf),
    BatchDone { completed: Vec<(String, PathBuf)>, missing: Vec<String>, error: Option<String> },
    Failed(String),
}

/// One publication item in an AI cover batch request.
#[derive(Clone, Debug)]
pub struct CoverBatchItem {
    pub title: String,
    pub title2: String,
    pub title_en: String,
    pub year: String,
    pub volume: String,
    pub catalogue: String,
    pub code: String,
    pub agency: String,
    pub logo: String,
    pub cover_style: usize,
    pub palette_index: usize,
    pub req: imagegen::GenRequest,
    pub output_path: PathBuf,
}

/// Derive a deterministic seed from the publication title to ensure cohesive series with varied backgrounds.
pub fn seed_from_title(title: &str) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    title.hash(&mut hasher);
    (hasher.finish() & 0xFFFF_FFFF) as u32
}

/// Pre-flight quota check against the daily Cloudflare free limit (~10,000 Neurons / ~220 per image).
pub fn preflight_quota_warning(count: usize) -> Option<String> {
    if count > imagegen::MAX_SAFE_FREE_BATCH {
        Some(format!(
            "{} of {} fit today's free quota; the rest will fail after ~{} — continue?",
            imagegen::MAX_SAFE_FREE_BATCH,
            count,
            imagegen::MAX_SAFE_FREE_BATCH
        ))
    } else {
        None
    }
}

/// Check if a publication's cover page already exists in the document with the matching AI preset tag.
pub fn cover_page_exists(doc: &designcraft_doc::Document, title: &str, preset_name: &str) -> bool {
    let norm = |s: &str| -> String { s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect() };
    let want_title = norm(title);
    if want_title.is_empty() {
        return false;
    }
    let want_preset = norm(preset_name);

    for sp in &doc.spreads {
        let has_preset_tag = sp.items.iter().any(|it| {
            let l = norm(&it.label);
            let n = norm(&it.name);
            (l.contains(&want_preset) || n.contains(&want_preset)) || (l.contains("slotimagebackground") || l.contains("aipreset"))
        });
        if !has_preset_tag {
            continue;
        }
        let has_title = sp.items.iter().any(|it| {
            if let designcraft_doc::Content::Text(tf) = &it.content
                && let Some(story) = doc.story(tf.story)
            {
                let s_norm = norm(&story.text);
                s_norm.contains(&want_title) || want_title.contains(&s_norm)
            } else {
                false
            }
        });
        if has_title {
            return true;
        }
    }
    false
}

/// Provider abstraction to allow injecting mock/test generators without network access.
pub trait ImageGenProvider: Send + Sync + 'static {
    fn generate(&self, req: &imagegen::GenRequest, cancel: &AtomicBool) -> Result<Vec<u8>, imagegen::ImageGenError>;
    /// Non-secret grouping id for the shared quota tracker: the Cloudflare account id.
    fn cred_id(&self) -> String;
}

/// Default provider implementation using Cloudflare Workers AI.
pub struct CloudflareProvider {
    client: imagegen::cloudflare::CloudflareClient,
    account_id: String,
}

impl CloudflareProvider {
    pub fn new(account_id: &str, token: &str) -> Self {
        Self { client: imagegen::cloudflare::CloudflareClient::new(account_id, token), account_id: account_id.trim().to_string() }
    }
}

impl ImageGenProvider for CloudflareProvider {
    fn generate(&self, req: &imagegen::GenRequest, cancel: &AtomicBool) -> Result<Vec<u8>, imagegen::ImageGenError> {
        self.client.generate(req, cancel)
    }

    fn cred_id(&self) -> String {
        self.account_id.clone()
    }
}

/// Fake provider for tests.
#[cfg(test)]
pub struct FakeImageGenProvider<F>(pub F)
where
    F: Fn(&imagegen::GenRequest, &AtomicBool) -> Result<Vec<u8>, imagegen::ImageGenError> + Send + Sync + 'static;

#[cfg(test)]
impl<F> ImageGenProvider for FakeImageGenProvider<F>
where
    F: Fn(&imagegen::GenRequest, &AtomicBool) -> Result<Vec<u8>, imagegen::ImageGenError> + Send + Sync + 'static,
{
    fn generate(&self, req: &imagegen::GenRequest, cancel: &AtomicBool) -> Result<Vec<u8>, imagegen::ImageGenError> {
        (self.0)(req, cancel)
    }

    fn cred_id(&self) -> String {
        "test-account".to_string()
    }
}

/// The running background cover job owned by `PubgenState`.
#[derive(Clone, Debug)]
pub struct CoverJob {
    #[cfg(not(target_arch = "wasm32"))]
    rx: Arc<std::sync::Mutex<std::sync::mpsc::Receiver<CoverJobMsg>>>,
    pub cancel: Arc<AtomicBool>,
    pub done: usize,
    pub total: usize,
    pub finished: bool,
    pub completed_path: Option<PathBuf>,
    pub batch_completed: Vec<(String, PathBuf)>,
    pub batch_missing: Vec<String>,
    pub unapplied: Vec<(String, PathBuf)>,
    /// Remaining Cloudflare free neurons today (shared Supabase counter), updated by
    /// the worker after each successful generation. `None` = not reported (yet).
    pub cover_remaining: Arc<std::sync::Mutex<Option<f64>>>,
}

impl CoverJob {
    /// Spawn the worker for a single cover request. Native only; wasm stub returns finished-empty.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn(account_id: &str, token: &str, req: imagegen::GenRequest, output_path: PathBuf, ctx: egui::Context) -> Self {
        let provider = Box::new(CloudflareProvider::new(account_id, token));
        Self::spawn_with_provider(provider, req, output_path, ctx)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn(_account_id: &str, _token: &str, _req: imagegen::GenRequest, _output_path: PathBuf, _ctx: egui::Context) -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            done: 0,
            total: 0,
            finished: true,
            completed_path: None,
            batch_completed: Vec::new(),
            batch_missing: Vec::new(),
            unapplied: Vec::new(),
            cover_remaining: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn_with_provider(provider: Box<dyn ImageGenProvider>, req: imagegen::GenRequest, output_path: PathBuf, ctx: egui::Context) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<CoverJobMsg>();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_thread = cancel.clone();
        let tx_thread = tx.clone();
        let cover_remaining = Arc::new(std::sync::Mutex::new(None));
        let rem_thread = cover_remaining.clone();
        let cred = provider.cred_id();

        let spawned = std::thread::Builder::new().name("designcraft-ai-cover".into()).spawn(move || {
            let res = (|| -> Result<PathBuf, String> {
                if cancel_thread.load(Ordering::Relaxed) {
                    return Err("AI cover generation cancelled".into());
                }
                let _ = tx_thread.send(CoverJobMsg::Progress(0, 1));
                let bytes = provider.generate(&req, &cancel_thread).map_err(|e| match e {
                    imagegen::ImageGenError::Quota(_) => "Daily Cloudflare free quota reached; resets 00:00 UTC".to_string(),
                    imagegen::ImageGenError::Auth => "Cloudflare rejected the credentials — check Account ID and Token in AI Setup…".to_string(),
                    imagegen::ImageGenError::Cancelled => "AI cover generation cancelled".to_string(),
                    imagegen::ImageGenError::Unavailable => "AI Base covers need a build with --features ai-cover-online.".to_string(),
                    imagegen::ImageGenError::Network(msg) => {
                        format!("Network error during AI cover generation: {msg}")
                    }
                    imagegen::ImageGenError::BadResponse(msg) => {
                        format!("Cloudflare returned an error: {msg}")
                    }
                    imagegen::ImageGenError::Io(io_err) => {
                        format!("IO error: {io_err}")
                    }
                })?;

                if cancel_thread.load(Ordering::Relaxed) {
                    return Err("AI cover generation cancelled".into());
                }

                if let Some(parent) = output_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::write(&output_path, &bytes).map_err(|e| format!("Failed to save AI cover to {}: {e}", output_path.display()))?;

                // Best-effort shared quota report (None = Supabase not configured / offline).
                if let Some(remaining) = imagegen::quota::consume_cover(&cred, imagegen::quota::NEURONS_PER_COVER) {
                    *rem_thread.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(remaining);
                }

                Ok(output_path)
            })();

            match res {
                Ok(path) => {
                    let _ = tx_thread.send(CoverJobMsg::Done(path));
                }
                Err(err) => {
                    let _ = tx_thread.send(CoverJobMsg::Failed(err));
                }
            }
            ctx.request_repaint();
        });

        if spawned.is_err() {
            let _ = tx.send(CoverJobMsg::Failed("could not start the cover generation thread".into()));
        }

        Self {
            rx: Arc::new(std::sync::Mutex::new(rx)),
            cancel,
            done: 0,
            total: 1,
            finished: false,
            completed_path: None,
            batch_completed: Vec::new(),
            batch_missing: Vec::new(),
            unapplied: Vec::new(),
            cover_remaining,
        }
    }

    /// Spawn a batch cover generation job.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn_batch(account_id: &str, token: &str, items: Vec<CoverBatchItem>, ctx: egui::Context) -> Self {
        let provider = Box::new(CloudflareProvider::new(account_id, token));
        Self::spawn_batch_with_provider(provider, items, ctx)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn_batch(_account_id: &str, _token: &str, _items: Vec<CoverBatchItem>, _ctx: egui::Context) -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            done: 0,
            total: 0,
            finished: true,
            completed_path: None,
            batch_completed: Vec::new(),
            batch_missing: Vec::new(),
            unapplied: Vec::new(),
            cover_remaining: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Spawn a batch job with custom provider (for testing).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn_batch_with_provider(provider: Box<dyn ImageGenProvider>, items: Vec<CoverBatchItem>, ctx: egui::Context) -> Self {
        let total = items.len();
        let (tx, rx) = std::sync::mpsc::channel::<CoverJobMsg>();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_thread = cancel.clone();
        let tx_thread = tx.clone();
        let cover_remaining = Arc::new(std::sync::Mutex::new(None));
        let rem_thread = cover_remaining.clone();
        let cred = provider.cred_id();

        let spawned = std::thread::Builder::new().name("designcraft-ai-cover-batch".into()).spawn(move || {
            let mut completed: Vec<(String, PathBuf)> = Vec::new();
            let mut missing: Vec<String> = Vec::new();
            let mut err_msg: Option<String> = None;
            let mut done = 0usize;

            let mut iter = items.into_iter().enumerate();
            while let Some((idx, item)) = iter.next() {
                if cancel_thread.load(Ordering::Relaxed) {
                    missing.push(item.title);
                    for (_, rest) in iter {
                        missing.push(rest.title);
                    }
                    err_msg = Some("AI cover generation cancelled".to_string());
                    break;
                }

                let _ = tx_thread.send(CoverJobMsg::Progress(done, total));

                match provider.generate(&item.req, &cancel_thread) {
                    Ok(bytes) => {
                        if cancel_thread.load(Ordering::Relaxed) {
                            missing.push(item.title);
                            for (_, rest) in iter {
                                missing.push(rest.title);
                            }
                            err_msg = Some("AI cover generation cancelled".to_string());
                            break;
                        }

                        if let Some(parent) = item.output_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Err(e) = std::fs::write(&item.output_path, &bytes) {
                            missing.push(item.title);
                            for (_, rest) in iter {
                                missing.push(rest.title);
                            }
                            err_msg = Some(format!("Failed to save cover image: {e}"));
                            break;
                        }

                        completed.push((item.title.clone(), item.output_path.clone()));
                        let _ = tx_thread.send(CoverJobMsg::ItemDone { index: idx, title: item.title, path: item.output_path });
                        done = done.saturating_add(1);
                        let _ = tx_thread.send(CoverJobMsg::Progress(done, total));

                        // Best-effort shared quota report (None = Supabase not configured / offline).
                        if let Some(remaining) = imagegen::quota::consume_cover(&cred, imagegen::quota::NEURONS_PER_COVER) {
                            *rem_thread.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(remaining);
                        }
                    }
                    Err(imagegen::ImageGenError::Quota(_)) => {
                        // Stop-on-429: skip remaining items, record missing, report
                        missing.push(item.title);
                        for (_, rest) in iter {
                            missing.push(rest.title);
                        }
                        err_msg = Some("Daily Cloudflare free quota reached; resets 00:00 UTC".to_string());
                        break;
                    }
                    Err(imagegen::ImageGenError::Cancelled) => {
                        missing.push(item.title);
                        for (_, rest) in iter {
                            missing.push(rest.title);
                        }
                        err_msg = Some("AI cover generation cancelled".to_string());
                        break;
                    }
                    Err(e) => {
                        missing.push(item.title);
                        for (_, rest) in iter {
                            missing.push(rest.title);
                        }
                        err_msg = Some(format!("AI cover generation stopped: {e}"));
                        break;
                    }
                }
            }

            let _ = tx_thread.send(CoverJobMsg::BatchDone { completed, missing, error: err_msg });
            ctx.request_repaint();
        });

        if spawned.is_err() {
            let _ = tx.send(CoverJobMsg::Failed("could not start the cover generation batch thread".into()));
        }

        Self {
            rx: Arc::new(std::sync::Mutex::new(rx)),
            cancel,
            done: 0,
            total,
            finished: false,
            completed_path: None,
            batch_completed: Vec::new(),
            batch_missing: Vec::new(),
            unapplied: Vec::new(),
            cover_remaining,
        }
    }

    /// Cancel the job early.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Approximate covers still available today on the shared Cloudflare free tier
    /// (remaining neurons / 96, from the Supabase counter the workers update).
    pub fn remaining_covers(&self) -> Option<u64> {
        let neurons = (*self.cover_remaining.lock().unwrap_or_else(std::sync::PoisonError::into_inner))?;
        Some((neurons / imagegen::quota::NEURONS_PER_COVER).max(0.0) as u64)
    }

    /// " · Cloudflare AI: ~N covers left today" when the worker's quota report came
    /// back; empty otherwise (Supabase not configured, offline, or report failed).
    pub(crate) fn quota_suffix(&self) -> String {
        match self.remaining_covers() {
            Some(n) => format!(" \u{b7} Cloudflare AI: ~{n} covers left today"),
            None => String::new(),
        }
    }

    /// Drain progress messages; on Done, record the output path. Returns true when finished successfully.
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
            let guard = rx.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            while let Ok(msg) = guard.try_recv() {
                match msg {
                    CoverJobMsg::Progress(done, total) => {
                        self.done = done;
                        self.total = total;
                        let msg = format!("Generating AI covers… ({done}/{total})");
                        st.cover_status = Some((msg.clone(), true));
                        st.card1_status = Some((msg.clone(), true));
                        st.card3_status = Some((msg, true));
                    }
                    CoverJobMsg::ItemDone { index: _, title, path } => {
                        self.batch_completed.push((title.clone(), path.clone()));
                        self.unapplied.push((title, path));
                    }
                    CoverJobMsg::Done(path) => {
                        self.done = 1;
                        self.finished = true;
                        self.completed_path = Some(path);
                        st.cover_status = Some((format!("AI cover generated.{}", self.quota_suffix()), true));
                        finished_ok = true;
                    }
                    CoverJobMsg::BatchDone { completed, missing, error } => {
                        self.finished = true;
                        self.batch_completed = completed;
                        self.batch_missing = missing.clone();
                        if missing.is_empty() && error.is_none() {
                            let msg = format!("{} covers generated successfully!{}", self.batch_completed.len(), self.quota_suffix());
                            st.cover_status = Some((msg.clone(), true));
                            st.card1_status = Some((msg.clone(), true));
                            st.card3_status = Some((msg, true));
                            finished_ok = true;
                        } else {
                            self.cancel.store(true, Ordering::Relaxed);
                            let msg = if !missing.is_empty() {
                                format!("{} covers generated; {} missing: {}", self.batch_completed.len(), missing.len(), missing.join(", "))
                            } else {
                                error.unwrap_or_else(|| "Batch cover generation stopped".to_string())
                            };
                            st.cover_status = Some((msg.clone(), false));
                            st.card1_status = Some((msg.clone(), false));
                            st.card3_status = Some((msg, false));
                        }
                    }
                    CoverJobMsg::Failed(err) => {
                        self.finished = true;
                        self.cancel.store(true, Ordering::Relaxed);
                        st.cover_status = Some((err.clone(), false));
                        st.card1_status = Some((err.clone(), false));
                        st.card3_status = Some((err.clone(), false));
                        if err.contains("Cloudflare rejected") || err.contains("AI Setup") {
                            st.cover_ai_error = Some(err);
                        }
                    }
                }
            }
            finished_ok
        }
    }
}

/// Determine the SDXL aspect ratio bucket matching the cover page dimensions.
pub fn aspect_for_dimensions(pw: f64, ph: f64) -> imagegen::Aspect {
    if pw <= 0.0 || ph <= 0.0 {
        return imagegen::Aspect::Iso;
    }
    // Landscape spread / wrap
    if pw > ph {
        return imagegen::Aspect::Wrap;
    }
    let ratio = ph / pw;
    if ratio >= 1.48 {
        imagegen::Aspect::Folio
    } else if ratio <= 1.35 {
        imagegen::Aspect::Letter
    } else {
        imagegen::Aspect::Iso
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_for_dimensions_maps_standard_sizes() {
        // A4: 595.28 x 841.89 -> ratio ~1.414 -> Iso
        assert_eq!(aspect_for_dimensions(595.28, 841.89), imagegen::Aspect::Iso);
        // Folio / F4: 612 x 936 -> ratio ~1.529 -> Folio
        assert_eq!(aspect_for_dimensions(612.0, 936.0), imagegen::Aspect::Folio);
        // US Letter: 612 x 792 -> ratio ~1.294 -> Letter
        assert_eq!(aspect_for_dimensions(612.0, 792.0), imagegen::Aspect::Letter);
        // Spread / wrap: landscape
        assert_eq!(aspect_for_dimensions(1190.56, 841.89), imagegen::Aspect::Wrap);
        // Zero / negative falls back to Iso
        assert_eq!(aspect_for_dimensions(0.0, 0.0), imagegen::Aspect::Iso);
    }

    #[test]
    fn job_poll_transitions_on_success() {
        let tmp = tempfile::tempdir().unwrap();
        let out_file = tmp.path().join("cover.png");
        let fake_png = b"\x89PNG\r\n\x1a\nfake png data".to_vec();

        let fake_provider = Box::new(FakeImageGenProvider(move |_req, _cancel| Ok(fake_png.clone())));
        let req = imagegen::GenRequest::new("test prompt", "", imagegen::Aspect::Iso, 18);
        let ctx = egui::Context::default();

        let mut job = CoverJob::spawn_with_provider(fake_provider, req, out_file.clone(), ctx);
        let mut st = PubgenState::default();

        let start = std::time::Instant::now();
        while !job.finished && start.elapsed().as_secs() < 3 {
            let _ = job.poll(&mut st);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(job.finished, "Job should have finished");
        assert!(job.completed_path.is_some(), "Completed path should be set");
        assert_eq!(job.completed_path.as_ref().unwrap(), &out_file);
        assert!(out_file.is_file(), "File must exist on disk");
        assert_eq!(std::fs::read(&out_file).unwrap(), b"\x89PNG\r\n\x1a\nfake png data");
    }

    #[test]
    fn job_poll_transitions_on_quota_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let out_file = tmp.path().join("cover.png");

        let fake_provider = Box::new(FakeImageGenProvider(|_req, _cancel| Err(imagegen::ImageGenError::Quota(Some(3600)))));
        let req = imagegen::GenRequest::new("test prompt", "", imagegen::Aspect::Iso, 18);
        let ctx = egui::Context::default();

        let mut job = CoverJob::spawn_with_provider(fake_provider, req, out_file, ctx);
        let mut st = PubgenState::default();

        let start = std::time::Instant::now();
        while !job.finished && start.elapsed().as_secs() < 3 {
            let _ = job.poll(&mut st);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(job.finished, "Job should have finished");
        assert!(job.completed_path.is_none(), "No completed path on error");
        let (status_msg, ok) = st.cover_status.as_ref().unwrap();
        assert!(!ok);
        assert!(status_msg.contains("Daily Cloudflare free quota reached"));
    }

    #[test]
    fn job_cancel_before_generation() {
        let tmp = tempfile::tempdir().unwrap();
        let out_file = tmp.path().join("cover.png");

        let fake_provider = Box::new(FakeImageGenProvider(|_req, _cancel| Ok(vec![1, 2, 3])));
        let req = imagegen::GenRequest::new("test prompt", "", imagegen::Aspect::Iso, 18);
        let ctx = egui::Context::default();

        let mut job = CoverJob::spawn_with_provider(fake_provider, req, out_file.clone(), ctx);
        job.cancel();

        let mut st = PubgenState::default();
        let start = std::time::Instant::now();
        while !job.finished && start.elapsed().as_secs() < 3 {
            let _ = job.poll(&mut st);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(job.finished);
        let (status_msg, ok) = st.cover_status.as_ref().unwrap();
        assert!(!ok);
        assert!(status_msg.contains("cancelled"));
        assert!(!out_file.exists(), "Cancelled job should not produce output file");
    }

    #[test]
    fn seed_from_title_is_deterministic_and_varies_by_title() {
        let s1 = seed_from_title("KCDAs Rotasi 2024");
        let s2 = seed_from_title("KCDAs Rotasi 2024");
        let s3 = seed_from_title("KCDAs Karawang 2024");

        assert_eq!(s1, s2, "Same title must produce identical seed");
        assert_ne!(s1, s3, "Different titles must produce varied seeds");
    }

    #[test]
    fn preflight_quota_warning_triggers_above_safe_limit() {
        assert_eq!(preflight_quota_warning(10), None);
        assert_eq!(preflight_quota_warning(45), None);
        let warning = preflight_quota_warning(50);
        assert!(warning.is_some());
        assert_eq!(warning.unwrap(), "45 of 50 fit today's free quota; the rest will fail after ~45 — continue?");
    }

    #[test]
    fn batch_job_stop_on_429_skips_remaining_and_reports_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let out0 = tmp.path().join("cover_0.png");
        let out1 = tmp.path().join("cover_1.png");
        let out2 = tmp.path().join("cover_2.png");

        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_thread = call_count.clone();

        let fake_provider = Box::new(FakeImageGenProvider(move |_req, _cancel| {
            let n = count_thread.fetch_add(1, Ordering::SeqCst);
            if n == 0 { Ok(b"\x89PNG\r\n\x1a\nitem0".to_vec()) } else { Err(imagegen::ImageGenError::Quota(Some(3600))) }
        }));

        let req = imagegen::GenRequest::new("prompt", "", imagegen::Aspect::Iso, 18);
        let items = vec![
            CoverBatchItem {
                title: "Publication 1".into(),
                title2: "".into(),
                title_en: "".into(),
                year: "2026".into(),
                volume: "".into(),
                catalogue: "".into(),
                code: "".into(),
                agency: "".into(),
                logo: "".into(),
                cover_style: 100,
                palette_index: 0,
                req: req.clone(),
                output_path: out0.clone(),
            },
            CoverBatchItem {
                title: "Publication 2".into(),
                title2: "".into(),
                title_en: "".into(),
                year: "2026".into(),
                volume: "".into(),
                catalogue: "".into(),
                code: "".into(),
                agency: "".into(),
                logo: "".into(),
                cover_style: 100,
                palette_index: 0,
                req: req.clone(),
                output_path: out1.clone(),
            },
            CoverBatchItem {
                title: "Publication 3".into(),
                title2: "".into(),
                title_en: "".into(),
                year: "2026".into(),
                volume: "".into(),
                catalogue: "".into(),
                code: "".into(),
                agency: "".into(),
                logo: "".into(),
                cover_style: 100,
                palette_index: 0,
                req,
                output_path: out2.clone(),
            },
        ];

        let ctx = egui::Context::default();
        let mut job = CoverJob::spawn_batch_with_provider(fake_provider, items, ctx);
        let mut st = PubgenState::default();

        let start = std::time::Instant::now();
        while !job.finished && start.elapsed().as_secs() < 3 {
            let _ = job.poll(&mut st);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(job.finished);
        assert_eq!(job.batch_completed.len(), 1, "Only first item should complete");
        assert_eq!(job.batch_missing.len(), 2, "Items 2 and 3 should be missing");
        assert!(out0.is_file(), "Completed item must exist on disk");
        assert!(!out1.exists(), "Failed item must not exist on disk");
        assert!(!out2.exists(), "Skipped item must not exist on disk");

        let (status_msg, ok) = st.cover_status.as_ref().unwrap();
        assert!(!ok);
        assert!(
            status_msg.contains("1 covers generated; 2 missing: Publication 2, Publication 3"),
            "Report should format missing list: {status_msg}"
        );
    }

    #[test]
    fn batch_job_cancel_mid_batch_keeps_completed() {
        let tmp = tempfile::tempdir().unwrap();
        let out0 = tmp.path().join("cover_0.png");
        let out1 = tmp.path().join("cover_1.png");

        let fake_provider = Box::new(FakeImageGenProvider(move |_req, _cancel| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            Ok(b"\x89PNG\r\n\x1a\nitem".to_vec())
        }));

        let req = imagegen::GenRequest::new("prompt", "", imagegen::Aspect::Iso, 18);
        let items = vec![
            CoverBatchItem {
                title: "Pub A".into(),
                title2: "".into(),
                title_en: "".into(),
                year: "2026".into(),
                volume: "".into(),
                catalogue: "".into(),
                code: "".into(),
                agency: "".into(),
                logo: "".into(),
                cover_style: 100,
                palette_index: 0,
                req: req.clone(),
                output_path: out0.clone(),
            },
            CoverBatchItem {
                title: "Pub B".into(),
                title2: "".into(),
                title_en: "".into(),
                year: "2026".into(),
                volume: "".into(),
                catalogue: "".into(),
                code: "".into(),
                agency: "".into(),
                logo: "".into(),
                cover_style: 100,
                palette_index: 0,
                req,
                output_path: out1.clone(),
            },
        ];

        let ctx = egui::Context::default();
        let mut job = CoverJob::spawn_batch_with_provider(fake_provider, items, ctx);
        let mut st = PubgenState::default();

        // Wait for first item to complete
        let start = std::time::Instant::now();
        while job.done == 0 && start.elapsed().as_secs() < 3 {
            let _ = job.poll(&mut st);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Cancel mid-batch
        job.cancel();

        while !job.finished && start.elapsed().as_secs() < 3 {
            let _ = job.poll(&mut st);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(job.finished);
        assert_eq!(job.batch_completed.len(), 1, "Completed items should be kept");
        assert!(out0.is_file(), "First item was completed before cancel");
    }
}
