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
    Done(PathBuf),
    Failed(String),
}

/// Provider abstraction to allow injecting mock/test generators without network access.
pub trait ImageGenProvider: Send + Sync + 'static {
    fn generate(&self, req: &imagegen::GenRequest, cancel: &AtomicBool) -> Result<Vec<u8>, imagegen::ImageGenError>;
}

/// Default provider implementation using Cloudflare Workers AI.
pub struct CloudflareProvider {
    client: imagegen::cloudflare::CloudflareClient,
}

impl CloudflareProvider {
    pub fn new(account_id: &str, token: &str) -> Self {
        Self { client: imagegen::cloudflare::CloudflareClient::new(account_id, token) }
    }
}

impl ImageGenProvider for CloudflareProvider {
    fn generate(&self, req: &imagegen::GenRequest, cancel: &AtomicBool) -> Result<Vec<u8>, imagegen::ImageGenError> {
        self.client.generate(req, cancel)
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
        Self { cancel: Arc::new(AtomicBool::new(false)), done: 0, total: 0, finished: true, completed_path: None }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn_with_provider(provider: Box<dyn ImageGenProvider>, req: imagegen::GenRequest, output_path: PathBuf, ctx: egui::Context) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<CoverJobMsg>();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_thread = cancel.clone();
        let tx_thread = tx.clone();

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

        Self { rx: Arc::new(std::sync::Mutex::new(rx)), cancel, done: 0, total: 1, finished: false, completed_path: None }
    }

    /// Cancel the job early.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
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
                        let msg = format!("Generating AI cover… ({done}/{total})");
                        st.cover_status = Some((msg, true));
                    }
                    CoverJobMsg::Done(path) => {
                        self.done = 1;
                        self.finished = true;
                        self.completed_path = Some(path);
                        finished_ok = true;
                    }
                    CoverJobMsg::Failed(err) => {
                        self.finished = true;
                        self.cancel.store(true, Ordering::Relaxed);
                        st.cover_status = Some((err.clone(), false));
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
}
