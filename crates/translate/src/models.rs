/// The offline model catalogue and the fit/speed estimates the setup dialog shows.
/// A slice, so a smaller or merged-decoder variant can be added without touching the UI.
use std::path::Path;

/// One downloadable offline model.
#[derive(Debug, Clone, Copy)]
pub struct ModelOption {
    /// Stable id stored in Prefs (\`translation_model_id\`).
    pub id: &'static str,
    pub label: &'static str,
    /// On-disk download size, all files together.
    pub approx_bytes: u64,
    /// Peak resident estimate: weights + activations + KV headroom.
    pub peak_ram_bytes: u64,
    /// Pinned download base (Hugging Face resolve URLs).
    pub url: &'static str,
    /// Pinned sha256 per file name; empty until the offline path pins them.
    pub sha256: &'static [(&'static str, &'static str)],
    /// Files the model folder must contain (name within the folder).
    pub files: &'static [&'static str],
}

/// The catalogue. ponytail: one entry today — the fp32 export (~7 GB on disk, >4 GB
/// resident) is deliberately NOT offered; it cannot fit the 8 GB target. Pin the sha256
/// values when the offline inference path lands.
pub const MODEL_OPTIONS: &[ModelOption] = &[ModelOption {
    id: "nllb600-int8",
    label: "NLLB-200 distilled 600M \u{b7} int8 (recommended)",
    approx_bytes: 1_800_000_000,
    peak_ram_bytes: 4_200_000_000,
    url: "https://huggingface.co/TigreGotico/nllb-200-distilled-600M-onnx/resolve/main",
    sha256: &[],
    files: &[
        "encoder_model.onnx",
        "decoder_model.onnx",
        "decoder_model.onnx_data",
        "decoder_with_past_model.onnx",
        "decoder_with_past_model.onnx_data",
        "tokenizer.json",
    ],
}];

/// Does the machine run this model? Thresholds on *available* RAM, with 25% headroom
/// for whatever else the user has open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    Fits,
    Tight,
    TooSmall,
}

pub fn fit(opt: &ModelOption, mem: &crate::system::MemoryInfo) -> Fit {
    let peak = opt.peak_ram_bytes;
    if mem.available >= peak + peak / 4 {
        Fit::Fits
    } else if mem.available >= peak {
        Fit::Tight
    } else {
        Fit::TooSmall
    }
}

/// Crude, labelled-as-an-estimate speed model: cores x a per-core int8 constant, clamped.
#[derive(Debug, Clone, Copy)]
pub struct SpeedEstimate {
    pub tokens_per_sec: f32,
    pub per_paragraph_secs: f32,
    pub per_page_secs: f32,
}

pub fn estimate_speed(opt: &ModelOption, cores: usize) -> SpeedEstimate {
    let _ = opt.id; // per-model constant table lives here when a second model lands
    let tokens_per_sec = (0.9 + 0.55 * cores.min(8) as f32) * 4.2;
    let per_paragraph_secs = 60.0 / tokens_per_sec * 1.6; // ~90-token average paragraph
    let per_page_secs = per_paragraph_secs * 1.8; // title + chrome + ~0.8 body
    SpeedEstimate { tokens_per_sec, per_paragraph_secs, per_page_secs }
}

/// Are all of the model's files present in \`dir\`?
pub fn files_present(dir: &Path) -> bool {
    MODEL_OPTIONS.iter().any(|o| o.files.iter().all(|f| dir.join(f).is_file()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::MemoryInfo;

    const OPT: ModelOption = MODEL_OPTIONS[0];

    fn mem(available_gb: f64) -> MemoryInfo {
        MemoryInfo { total: (available_gb * 16.0) as u64, available: (available_gb * 1_000_000_000.0) as u64 }
    }

    #[test]
    fn fit_thresholds_match_the_plan() {
        assert_eq!(fit(&OPT, &mem(6.0)), Fit::Fits, "6 GB available >= 4.2 * 1.25 = 5.25");
        assert_eq!(fit(&OPT, &mem(5.0)), Fit::Tight, "5 GB available >= 4.2 but < 5.25");
        assert_eq!(fit(&OPT, &mem(3.0)), Fit::TooSmall, "3 GB available < 4.2");
        // Exactly at the boundaries.
        let boundary = 4_200_000_000u64;
        assert_eq!(fit(&OPT, &MemoryInfo { total: boundary * 2, available: boundary }), Fit::Tight);
        assert_eq!(fit(&OPT, &MemoryInfo { total: boundary * 2, available: boundary + boundary / 4 }), Fit::Fits);
    }

    #[test]
    fn speed_estimate_grows_with_cores_and_stays_bounded() {
        let slow = estimate_speed(&OPT, 2);
        let fast = estimate_speed(&OPT, 8);
        let faster = estimate_speed(&OPT, 16);
        assert!(slow.tokens_per_sec < fast.tokens_per_sec);
        assert!((fast.tokens_per_sec - faster.tokens_per_sec).abs() < f32::EPSILON, "clamped at 8 cores");
        assert!(fast.tokens_per_sec > 15.0 && fast.tokens_per_sec < 25.0, "plan: ~20 tok/s at 8 cores");
        assert!(slow.per_paragraph_secs > fast.per_paragraph_secs);
    }

    #[test]
    fn files_present_is_false_for_an_empty_dir() {
        assert!(!files_present(Path::new(".")));
        assert!(!files_present(Path::new("Z:/definitely/not/here")));
    }
}
