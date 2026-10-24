/// Offline NLLB-200 inference (ORT + tokenizers), behind the \`nllb\` feature.
///
/// ponytail: the ONNX encoder/decoder sessions, greedy decode and KV-cache dance are the
/// planned follow-up (plan §1: merged decoder first, bind inputs by name, EOS stop at 400
/// new tokens, one segment in flight, cancel between segments and steps). Today the
/// feature compiles and reports \`Unavailable\` so the UI can stay honest; the Groq path
/// is the working backend.
use crate::{Lang, TranslateError};
use std::path::Path;
use std::sync::atomic::AtomicBool;

pub fn compiled_in() -> bool {
    cfg!(feature = "nllb")
}

pub fn load(_dir: &Path) -> Result<crate::Translator, TranslateError> {
    Err(unavailable())
}

pub fn translate(
    _texts: &[String],
    _src: Lang,
    _tgt: Lang,
    _cancel: &AtomicBool,
    _progress: &mut dyn FnMut(usize, usize),
) -> Result<Vec<String>, TranslateError> {
    Err(unavailable())
}

fn unavailable() -> TranslateError {
    TranslateError::Unavailable
}
