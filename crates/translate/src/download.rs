/// In-app model downloader (feature \`download\`): streams each file to \`<dir>/<name>.part\`,
/// verifies the pinned sha256, then renames into place. A mismatch deletes the .part and
/// reports — a half-written model is never kept.

use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::models::ModelOption;
use crate::TranslateError;

/// Download every file of \`opt\` into \`dir\`. \`progress\` reports (files done, files total).
pub fn download_model(
    opt: &ModelOption,
    dir: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<(), TranslateError> {
    std::fs::create_dir_all(dir)?;
    for (i, file) in opt.files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(TranslateError::Cancelled);
        }
        let dest = dir.join(file);
        if dest.is_file() {
            progress(i + 1, opt.files.len());
            continue;
        }
        let url = format!("{}/{file}", opt.url.trim_end_matches('/'));
        let part = dir.join(format!("{file}.part"));
        download_one(&url, &part, cancel)?;
        if let Some(expected) = opt.sha256.iter().find(|(name, _)| name == file).map(|(_, hash)| *hash) {
            let got = sha256_file(&part)?;
            if !got.eq_ignore_ascii_case(expected) {
                let _ = std::fs::remove_file(&part);
                return Err(TranslateError::Download(format!(
                    "{file}: sha256 mismatch (got {got}, expected {expected}); the partial file was deleted"
                )));
            }
        }
        std::fs::rename(&part, &dest)?;
        progress(i + 1, opt.files.len());
    }
    Ok(())
}

fn download_one(url: &str, part: &Path, cancel: &AtomicBool) -> Result<(), TranslateError> {
    let mut res = ureq::get(url).call().map_err(|e| TranslateError::Download(e.to_string()))?;
    let mut out = std::fs::File::create(part)?;
    let mut reader = res.into_reader();
    let mut buf = [0u8; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(part);
            return Err(TranslateError::Cancelled);
        }
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                use std::io::Write;
                out.write_all(&buf[..n])?;
            }
            Err(e) => {
                let _ = std::fs::remove_file(part);
                return Err(TranslateError::Download(e.to_string()));
            }
        }
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, TranslateError> {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path)?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(format!("{:x}", h.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_from_a_bad_url_is_a_clean_error() {
        let cancel = AtomicBool::new(false);
        let dir = std::env::temp_dir().join(format!("dc-translate-test-{}", std::process::id()));
        let opt = crate::models::MODEL_OPTIONS[0];
        let err = download_model(&opt, &dir, &cancel, &mut |_, _| {}).err().expect("must fail offline");
        assert!(matches!(err, TranslateError::Download(_) | TranslateError::Io(_)), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
