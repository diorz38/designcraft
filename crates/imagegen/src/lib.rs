//! AI image generation for Pubgen covers: Cloudflare Workers AI (SDXL), text-to-image.
//!
//! Pure bytes-in / bytes-out: no doc, no egui. The provider is an optional cargo
//! feature (`cloudflare`); without it every type still exists and every entry point
//! returns [`ImageGenError::Unavailable`], so consumers compile unchanged in the
//! default (and wasm) build — the same contract as `designcraft-translate`.
//!
//! The API token lives only in the Authorization header: never logged, never in an
//! error string, never written into a document.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]
#![forbid(unsafe_code)]
// Tests may panic (repo clippy.toml intent); the crate-level denies above only bite
// non-test code, where a panic can lose a document.
#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used, clippy::panic))]

pub mod cloudflare;

/// Estimated Cloudflare Workers AI Neurons cost per SDXL 18-step image.
pub const ESTIMATED_NEURONS_PER_IMAGE: u64 = 220;

/// Cloudflare free daily neuron limit (resets 00:00 UTC).
pub const DAILY_FREE_NEURONS: u64 = 10_000;

/// Maximum safe batch size on free tier before daily quota exhaustion (~10,000 / 220).
pub const MAX_SAFE_FREE_BATCH: usize = 45;

/// Cover aspect buckets matched to the SDXL ~1-megapixel training buckets
/// (dimensions are multiples of 8; forcing print-resolution sizes causes
/// repetition artifacts, so the caller upscales afterwards if ever needed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    /// A4 / A5 (ISO 1 : √2).
    Iso,
    /// Folio / F4 (1 : 1.54).
    Folio,
    /// US Letter (1 : 1.29).
    Letter,
    /// Full wraparound spread (landscape front + spine + back).
    Wrap,
}

impl Aspect {
    /// Pixel dimensions the provider is asked for.
    pub const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Iso => (832, 1176),
            Self::Folio => (768, 1184),
            Self::Letter => (896, 1152),
            Self::Wrap => (1216, 832),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImageGenError {
    #[error("image generation is not available in this build (compile with the cloudflare feature)")]
    Unavailable,
    #[error("image generation cancelled")]
    Cancelled,
    /// Daily free allocation exhausted (HTTP 429). `retry_after_secs` comes from the
    /// Retry-After header when present; the free tier resets at 00:00 UTC.
    #[error("provider quota reached (retry after {0:?} s)")]
    Quota(Option<u64>),
    /// Credentials missing or rejected (HTTP 401/403).
    #[error("provider rejected the credentials")]
    Auth,
    #[error("request failed: {0}")]
    Network(String),
    /// Provider answered, but not with a usable image.
    #[error("bad provider response: {0}")]
    BadResponse(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// One text-to-image request. Dimensions come only from [`Aspect`] — raw user
/// numbers never reach the payload (hostile-input rule).
#[derive(Debug, Clone)]
pub struct GenRequest {
    pub prompt: String,
    pub negative_prompt: String,
    pub width: u32,
    pub height: u32,
    pub num_steps: u32,
    /// `None` lets the provider pick (a new variation every call).
    pub seed: Option<u32>,
}

impl GenRequest {
    /// Steps clamped into the provider's sane range (presets pass 15–18; values
    /// beyond 25 only burn quota without visible quality).
    pub fn new(prompt: impl Into<String>, negative_prompt: impl Into<String>, aspect: Aspect, num_steps: u32) -> Self {
        let (width, height) = aspect.dimensions();
        Self { prompt: prompt.into(), negative_prompt: negative_prompt.into(), width, height, num_steps: num_steps.clamp(1, 25), seed: None }
    }

    /// Estimated Cloudflare Workers AI Neurons cost per SDXL 18-step image.
    pub const ESTIMATED_NEURONS_PER_IMAGE: u64 = 220;

    /// Cloudflare free daily neuron limit (resets 00:00 UTC).
    pub const DAILY_FREE_NEURONS: u64 = 10_000;

    /// Maximum safe batch size on free tier before daily quota exhaustion (~10,000 / 220).
    pub const MAX_SAFE_FREE_BATCH: usize = 45;

    /// Attach a deterministic random seed to this generation request.
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = Some(seed);
        self
    }

    /// The module-default negative prompt: every spelling of "text" the model
    /// reacts to, so covers stay free of garbled AI typography and borders.
    pub fn clean_cover_negative_prompt() -> String {
        "text, letters, words, typography, title, author name, signature, watermark, logo, \
         frame, borders, lorem ipsum, blurry, cut-and-paste seams, low quality"
            .to_string()
    }
}

/// Cloudflare Workers AI is compiled in? (`cfg!(feature = "cloudflare")`.)
pub fn compiled_in() -> bool {
    cloudflare::compiled_in()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_dimensions_match_the_documented_buckets() {
        assert_eq!(Aspect::Iso.dimensions(), (832, 1176));
        assert_eq!(Aspect::Folio.dimensions(), (768, 1184));
        assert_eq!(Aspect::Letter.dimensions(), (896, 1152));
        assert_eq!(Aspect::Wrap.dimensions(), (1216, 832));
        for a in [Aspect::Iso, Aspect::Folio, Aspect::Letter, Aspect::Wrap] {
            let (w, h) = a.dimensions();
            assert_eq!(w % 8, 0, "{a:?} width must stay a multiple of 8");
            assert_eq!(h % 8, 0, "{a:?} height must stay a multiple of 8");
            let mp = f64::from(w) * f64::from(h) / 1_048_576.0;
            assert!(mp > 0.7 && mp < 1.5, "{a:?} leaves the ~1MP bucket ({mp})");
        }
    }

    #[test]
    fn request_clamps_steps_and_takes_aspect_dimensions() {
        let r = GenRequest::new("p", "n", Aspect::Folio, 999);
        assert_eq!((r.width, r.height), (768, 1184));
        assert_eq!(r.num_steps, 25);
        assert_eq!(GenRequest::new("p", "n", Aspect::Iso, 0).num_steps, 1);
        assert_eq!(r.seed, None);
    }

    #[test]
    fn negative_prompt_names_the_text_family() {
        let n = GenRequest::clean_cover_negative_prompt();
        for word in ["text", "letters", "words", "typography", "watermark", "logo"] {
            assert!(n.contains(word), "negative prompt must contain {word}");
        }
    }

    #[test]
    fn availability_flag_follows_the_compiled_feature() {
        assert_eq!(compiled_in(), cfg!(feature = "cloudflare"));
    }

    /// Without the feature every entry point is a stub that never touches the network.
    #[cfg(not(feature = "cloudflare"))]
    #[test]
    fn without_the_feature_generation_is_unavailable() {
        use std::sync::atomic::AtomicBool;
        let client = cloudflare::CloudflareClient::new("acct", "tok");
        let cancel = AtomicBool::new(false);
        let err = client.generate(&GenRequest::new("p", "n", Aspect::Iso, 18), &cancel).expect_err("without the feature every call errors");
        assert!(matches!(err, ImageGenError::Unavailable), "{err}");
    }
}
