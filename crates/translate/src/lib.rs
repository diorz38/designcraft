//! AI translation for Pubgen: Indonesian -> English, offline (NLLB-200) or online (Groq).
//!
//! Pure text-in / text-out: no doc, no egui. The heavy backends are optional cargo
//! features (\`nllb\`, \`groq\`); without them every type still exists and every entry
//! point returns \`TranslateError::Unavailable\`, so consumers compile unchanged in the
//! default (and wasm) build.
//!
//! Licence note that must reach the UI: NLLB-200 is CC-BY-NC-4.0 - non-commercial
//! (offline path only; the Groq path has no NLLB involvement).

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]
#![forbid(unsafe_code)]

pub mod models;
pub mod nllb;
pub mod online;
pub mod system;

use std::path::Path;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Indonesian,
    English,
}

impl Lang {
    /// NLLB-200 language code (\`ind_Latn\` / \`eng_Latn\`).
    pub const fn nllb_code(self) -> &'static str {
        match self {
            Self::Indonesian => "ind_Latn",
            Self::English => "eng_Latn",
        }
    }

    /// Human label for dialogs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Indonesian => "Indonesian",
            Self::English => "English",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    #[error("translation is not available in this build (compile with the groq or nllb feature)")]
    Unavailable,
    #[error("model files missing in {0}: run the model downloader or pick another folder")]
    ModelMissing(String),
    #[error("model files in {0} do not match the expected graphs")]
    ModelMismatch(String),
    #[error("translation cancelled")]
    Cancelled,
    #[error("provider error: {0}")]
    Provider(String),
    #[error("rate limited (retry after {0}s)")]
    RateLimited(u64),
    #[error("download error: {0}")]
    Download(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Which backend a translation run uses. One \`translate\`; panel and job code are
/// provider-blind.
pub enum Backend {
    Offline(Translator),
    Online(OnlineTranslator),
}

impl Backend {
    pub fn translate(
        &mut self,
        texts: &[String],
        src: Lang,
        tgt: Lang,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(usize, usize),
    ) -> Result<Vec<String>, TranslateError> {
        match self {
            Self::Offline(t) => t.translate(texts, src, tgt, cancel, progress),
            Self::Online(t) => t.translate(texts, src, tgt, cancel, progress),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Offline(_) => "offline",
            Self::Online(_) => "online",
        }
    }
}

/// Offline NLLB-200 translator (a stub unless the \`nllb\` feature is compiled in).
#[derive(Default)]
pub struct Translator;

impl Translator {
    /// Load the model files from \`dir\` (encoder, decoder, tokenizer).
    pub fn load(_dir: &Path) -> Result<Self, TranslateError> {
        nllb::load(_dir)
    }

    pub fn translate(
        &mut self,
        texts: &[String],
        src: Lang,
        tgt: Lang,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(usize, usize),
    ) -> Result<Vec<String>, TranslateError> {
        nllb::translate(texts, src, tgt, cancel, progress)
    }
}

/// Online translator (Groq free tier; OpenAI-compatible chat completions).
pub struct OnlineTranslator {
    inner: online::OnlineTranslator,
}

impl OnlineTranslator {
    pub fn new(key: &str, model: &str) -> Self {
        Self { inner: online::OnlineTranslator::new(key, model) }
    }

    /// The model ids the account may use (Groq \`GET /openai/v1/models\`).
    pub fn list_models(&self) -> Result<Vec<String>, TranslateError> {
        self.inner.list_models()
    }

    pub fn translate(
        &mut self,
        texts: &[String],
        src: Lang,
        tgt: Lang,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(usize, usize),
    ) -> Result<Vec<String>, TranslateError> {
        self.inner.translate(texts, src, tgt, cancel, progress)
    }
}

/// Offline inference compiled in? (\`cfg!(feature = "nllb")\`.)
pub fn is_available() -> bool {
    nllb::compiled_in()
}

/// Online (Groq) backend compiled in? (\`cfg!(feature = "groq")\`.)
pub fn online_available() -> bool {
    online::compiled_in()
}

/// Are the model files for the offline model present in \`dir\`?
pub fn model_files_present(dir: &Path) -> bool {
    models::files_present(dir)
}

/// Split one long text into model-sized segments: short paragraphs stay whole, long
/// ones split on sentence ends. Consumers rejoin the parts with "\n".
pub fn segment(text: &str) -> Vec<String> {
    segment_text(text)
}

/// Rough model-input ceiling, in words (~200 tokens; Indonesian averages ~1.3 tokens/word).
const SEGMENT_MAX_WORDS: usize = 150;

fn segment_text(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        if para.split_whitespace().count() <= SEGMENT_MAX_WORDS {
            out.push(para.to_string());
            continue;
        }
        // Long paragraph: accumulate sentences up to the ceiling.
        let mut cur = String::new();
        for sentence in split_sentences(para) {
            let would = cur.split_whitespace().count() + sentence.split_whitespace().count();
            if !cur.is_empty() && would > SEGMENT_MAX_WORDS {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(sentence);
        }
        if !cur.trim().is_empty() {
            out.push(cur.trim().to_string());
        }
    }
    out
}

/// Sentence ends: \`.\`, \`!\`, \`?\` followed by a space or end (decimal points kept together).
fn split_sentences(para: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (i, c) in para.char_indices() {
        if matches!(c, '.' | '!' | '?') {
            let next = para[i + 1..].chars().next();
            let last = para[start..i].chars().last();
            let digit_run = last.is_some_and(|p| p.is_ascii_digit()) && next.is_some_and(|n| n.is_ascii_digit());
            if !digit_run && next.is_none_or(|n| n == ' ') {
                out.push(para[start..=i].trim_end());
                start = i + 1;
            }
        }
    }
    if start < para.len() {
        let rest = para[start..].trim();
        if !rest.is_empty() {
            out.push(rest);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_keeps_short_paragraphs_whole() {
        assert_eq!(segment("Satu dua tiga.\nEmpat lima."), vec!["Satu dua tiga.", "Empat lima."]);
        assert!(segment("").is_empty());
        assert!(segment("\n \n").is_empty());
    }

    #[test]
    fn segment_splits_long_paragraph_on_sentences() {
        let sentence = "Kalimat ini memiliki sepuluh suku kata untuk penghitungan resmi. ";
        let para = sentence.repeat(40);
        let parts = segment(&para);
        assert!(parts.len() > 1, "long paragraph must split");
        for p in &parts {
            assert!(p.split_whitespace().count() <= SEGMENT_MAX_WORDS, "segment over ceiling");
        }
        // Rejoined, the same words in the same order.
        let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(norm(&parts.join(" ")), norm(&para));
    }

    #[test]
    fn segment_keeps_decimal_points_together() {
        let para = "Jumlahnya adalah 3.14 kali lipat dari tahun sebelumnya menurut data resmi yang dirilis. ".repeat(20);
        let parts = segment(&para);
        assert!(parts.iter().any(|p| p.contains("3.14")), "decimal point must not split a sentence");
    }

    #[test]
    fn lang_codes_match_the_plan() {
        assert_eq!(Lang::Indonesian.nllb_code(), "ind_Latn");
        assert_eq!(Lang::English.nllb_code(), "eng_Latn");
    }

    #[test]
    fn availability_flags_follow_the_compiled_features() {
        // The offline path is a stub until the nllb feature lands.
        let err = Translator::load(Path::new(".")).err().expect("offline load must error without a model");
        assert!(matches!(err, TranslateError::Unavailable));
        assert!(!is_available());
        assert_eq!(online_available(), cfg!(feature = "groq"));
    }
}
