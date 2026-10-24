//! Groq (OpenAI-compatible) chat-completions client: batched JSON-array translation
//! with fence stripping, count-mismatch errors and 429 Retry-After backoff.
//!
//! The API key lives only in the Authorization header: never logged, never in an
//! error string, never written into a document. Without the \`groq\` feature the type
//! exists and every call returns \`TranslateError::Unavailable\`.

use std::sync::atomic::AtomicBool;
#[cfg(feature = "groq")]
use std::sync::atomic::Ordering;

use super::{Lang, TranslateError};

/// Units per request: small enough for free-tier rate caps, big enough to amortise.
#[cfg(feature = "groq")]
const BATCH: usize = 16;
#[cfg(feature = "groq")]
const MAX_429_RETRIES: usize = 3;

/// Online translator (Groq free tier; OpenAI-compatible chat completions).
#[cfg_attr(not(feature = "groq"), allow(dead_code))]
pub struct OnlineTranslator {
    key: String,
    model: String,
    #[cfg(feature = "groq")]
    agent: ureq::Agent,
    /// Overridable for tests (a canned local listener stands in for api.groq.com).
    #[cfg(feature = "groq")]
    base_url: String,
}

impl OnlineTranslator {
    pub fn new(key: &str, model: &str) -> Self {
        Self {
            key: key.trim().to_string(),
            model: model.trim().to_string(),
            #[cfg(feature = "groq")]
            agent: ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(30)).build(),
            #[cfg(feature = "groq")]
            base_url: "https://api.groq.com/openai/v1".to_string(),
        }
    }

    #[cfg(all(test, feature = "groq"))]
    fn with_base(key: &str, model: &str, base_url: String) -> Self {
        let mut t = Self::new(key, model);
        t.base_url = base_url;
        t
    }

    /// The model ids the account may use, sorted (Groq GET /openai/v1/models).
    pub fn list_models(&self) -> Result<Vec<String>, TranslateError> {
        #[cfg(feature = "groq")]
        {
            let res = self.agent.get(&format!("{}/models", self.base_url)).set("Authorization", &format!("Bearer {}", self.key)).call();
            let body = read_body(res)?;
            let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| TranslateError::Provider(format!("model list: {e}")))?;
            let mut ids: Vec<String> = v
                .get("data")
                .and_then(|d| d.as_array())
                .map(|a| a.iter().filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(str::to_string)).collect())
                .unwrap_or_default();
            ids.sort();
            Ok(ids)
        }
        #[cfg(not(feature = "groq"))]
        Err(TranslateError::Unavailable)
    }

    pub fn translate(
        &mut self,
        texts: &[String],
        src: Lang,
        tgt: Lang,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(usize, usize),
    ) -> Result<Vec<String>, TranslateError> {
        #[cfg(feature = "groq")]
        {
            let mut out = Vec::with_capacity(texts.len());
            for batch in texts.chunks(BATCH) {
                if cancel.load(Ordering::Relaxed) {
                    return Err(TranslateError::Cancelled);
                }
                out.extend(self.translate_batch(batch, src, tgt, cancel)?);
                progress(out.len(), texts.len());
            }
            Ok(out)
        }
        #[cfg(not(feature = "groq"))]
        {
            let _ = (texts, src, tgt, cancel, progress);
            Err(TranslateError::Unavailable)
        }
    }

    #[cfg(feature = "groq")]
    fn translate_batch(&self, batch: &[String], src: Lang, tgt: Lang, cancel: &AtomicBool) -> Result<Vec<String>, TranslateError> {
        let mut attempt = 0usize;
        // Plan salvage path: when the batch output is unusable (non-JSON, wrong count —
        // some reasoning models prose-ify their answer), retry the batch item-by-item.
        let mut item_fallback = false;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(TranslateError::Cancelled);
            }
            let res = if item_fallback {
                let mut out = Vec::with_capacity(batch.len());
                for item in batch {
                    out.extend(self.request_once(std::slice::from_ref(item), src, tgt)?);
                }
                Ok(out)
            } else {
                self.request_once(batch, src, tgt)
            };
            match res {
                Ok(out) => return Ok(out),
                Err(TranslateError::RateLimited(retry_after)) => {
                    attempt += 1;
                    if attempt > MAX_429_RETRIES {
                        return Err(TranslateError::Provider(format!("rate limited after {MAX_429_RETRIES} retries")));
                    }
                    if cancel.load(Ordering::Relaxed) {
                        return Err(TranslateError::Cancelled);
                    }
                    std::thread::sleep(std::time::Duration::from_secs(retry_after));
                }
                Err(TranslateError::Provider(_)) if !item_fallback => {
                    item_fallback = true;
                    attempt = 0;
                }
                Err(e) => return Err(e),
            }
        }
    }

    #[cfg(feature = "groq")]
    fn request_once(&self, batch: &[String], src: Lang, tgt: Lang) -> Result<Vec<String>, TranslateError> {
        let body = request_body(&self.model, batch, src, tgt);
        let res = self
            .agent
            .post(&format!("{}/chat/completions", self.base_url))
            .set("Authorization", &format!("Bearer {}", self.key))
            .set("Content-Type", "application/json")
            .send_string(&body);
        let (status, raw) = match res {
            Ok(r) => (r.status(), r.into_string().map_err(|e| TranslateError::Provider(e.to_string()))?),
            Err(ureq::Error::Status(code, r)) => {
                let retry = r.header("Retry-After").and_then(|v| v.trim().parse::<u64>().ok()).map(|s| s.clamp(1, 30));
                let raw = r.into_string().map_err(|e| TranslateError::Provider(e.to_string()))?;
                if code == 429 {
                    return Err(TranslateError::RateLimited(retry.unwrap_or(2)));
                }
                // Never echo the key: the body names the endpoint problem only.
                (code, raw)
            }
            Err(e) => return Err(TranslateError::Provider(format!("request failed: {e}"))),
        };
        if status != 200 {
            return Err(TranslateError::Provider(format!("HTTP {status}: {}", truncate(&raw, 300))));
        }
        let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| TranslateError::Provider(format!("bad JSON: {e}")))?;
        let content = v
            .pointer("/choices/0/message/content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| TranslateError::Provider("response has no choices[0].message.content".into()))?;
        let parsed = parse_translation(content)?;
        if parsed.len() != batch.len() {
            // Never silently reorder: name the batch and fail.
            return Err(TranslateError::Provider(format!(
                "model returned {} items for a batch of {} starting {:?}",
                parsed.len(),
                batch.len(),
                batch.first().map(|s| truncate(s, 40)).unwrap_or_default()
            )));
        }
        Ok(parsed)
    }
}

/// Units per request kept at module scope for the tests that build batches.
#[cfg(feature = "groq")]
fn request_body(model: &str, batch: &[String], src: Lang, tgt: Lang) -> String {
    let items = serde_json::to_string(batch).unwrap_or_default();
    let system = format!(
        "You are a translation engine. Translate each array item from {} to {}. Return a JSON \
         array of the same length and order. Keep numbers, proper nouns and formatting unchanged. \
         Output only the JSON array.",
        src.label(),
        tgt.label()
    );
    // Reasoning models burn hidden reasoning tokens that count against the provider's
    // TPM and add nothing to a translation. Pin the effort to low when supported.
    let mut body = serde_json::json!({
        "model": model,
        "temperature": 0,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": items},
        ],
    });
    if model.contains("gpt-oss")
        && let Some(obj) = body.as_object_mut()
    {
        obj.insert("reasoning_effort".into(), serde_json::json!("low"));
    }
    body.to_string()
}

/// Strip a triple-backtick json fence if present, then parse the JSON array.
#[cfg(feature = "groq")]
fn parse_translation(content: &str) -> Result<Vec<String>, TranslateError> {
    let fence: String = "`".repeat(3);
    let mut s = content.trim();
    if let Some(rest) = s.strip_prefix(&fence) {
        // The opening fence line may name the language.
        let body = rest.split_once('\n').map(|(_, body)| body).unwrap_or(rest);
        s = body.trim_end();
        if let Some(without) = s.strip_suffix(&fence) {
            s = without.trim_end();
        }
    }
    let v: serde_json::Value = serde_json::from_str(s).map_err(|e| TranslateError::Provider(format!("model output is not JSON: {e}")))?;
    let arr = v.as_array().ok_or_else(|| TranslateError::Provider("model output is not a JSON array".into()))?;
    Ok(arr.iter().map(|i| i.as_str().unwrap_or_default().to_string()).collect())
}

#[cfg(feature = "groq")]
fn read_body(res: Result<ureq::Response, ureq::Error>) -> Result<String, TranslateError> {
    match res {
        Ok(r) => r.into_string().map_err(|e| TranslateError::Provider(e.to_string())),
        Err(ureq::Error::Status(429, r)) => {
            let retry = r.header("Retry-After").and_then(|v| v.trim().parse::<u64>().ok()).map(|s| s.clamp(1, 30));
            Err(TranslateError::RateLimited(retry.unwrap_or(2)))
        }
        Err(ureq::Error::Status(code, r)) => {
            let raw = r.into_string().map_err(|e| TranslateError::Provider(e.to_string()))?;
            Err(TranslateError::Provider(format!("HTTP {code}: {}", truncate(&raw, 300))))
        }
        Err(e) => Err(TranslateError::Provider(format!("request failed: {e}"))),
    }
}

#[cfg(feature = "groq")]
fn truncate(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

pub fn compiled_in() -> bool {
    cfg!(feature = "groq")
}

#[cfg(all(test, feature = "groq"))]
mod tests {
    use super::*;

    #[test]
    fn parse_strips_json_fence() {
        let fence: String = std::iter::repeat('\u{60}').take(3).collect();
        let fenced = format!("{fence}json\n[\"Halo\", \"Dunia\"]\n{fence}");
        let out = parse_translation(&fenced).expect("fenced array parses");
        assert_eq!(out, vec!["Halo", "Dunia"]);
        let out = parse_translation("[\"a\", \"b\"]").expect("bare array parses");
        assert_eq!(out, vec!["a", "b"]);
        let prose = "Sure! Here is the translation: [\"a\"]";
        let err = parse_translation(prose).err().expect("prose is an error");
        assert!(matches!(err, TranslateError::Provider(_)));
        let obj = "{\"a\": 1}";
        let err = parse_translation(obj).err().expect("object is an error");
        assert!(matches!(err, TranslateError::Provider(_)));
    }

    #[test]
    fn request_body_pins_the_contract() {
        let body = request_body("llama-3.3-70b", &["Kabupaten Karawang".into()], Lang::Indonesian, Lang::English);
        let v: serde_json::Value = serde_json::from_str(&body).expect("request body is valid JSON");
        assert_eq!(v.pointer("/model"), Some(&serde_json::json!("llama-3.3-70b")));
        assert_eq!(v.pointer("/temperature"), Some(&serde_json::json!(0)));
        let system = v.pointer("/messages/0/content").and_then(|c| c.as_str()).expect("system message");
        assert!(system.contains("Indonesian to English"), "{system}");
        assert!(system.contains("same length and order"), "{system}");
        // The user message is a JSON-encoded array (escaped inside the outer JSON).
        let user = v.pointer("/messages/1/content").and_then(|c| c.as_str()).expect("user message");
        assert_eq!(user, "[\"Kabupaten Karawang\"]");
    }

    /// A canned TCP listener stands in for api.groq.com: one POST, one JSON reply.
    fn canned_server(reply_body: String) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            use std::io::{Read, Write};
            // Read the whole request (headers + Content-Length body) before replying: a
            // partial read would close the socket mid-request and reset the client.
            let mut req: Vec<u8> = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                req.extend_from_slice(&buf[..n]);
                let header_end = req.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4);
                if let Some(body_start) = header_end {
                    let cl: usize = String::from_utf8_lossy(&req)
                        .split("\r\n")
                        .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|v| v.trim().parse::<usize>().ok()))
                        .unwrap_or(0);
                    if req.len() >= body_start + cl {
                        break;
                    }
                }
            }
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                reply_body.len(),
                reply_body
            );
            let _ = stream.write_all(reply.as_bytes());
            let _ = stream.flush();
        });
        format!("http://{addr}/v1")
    }

    fn content_array(items: &[&str]) -> String {
        let quoted: Vec<String> = items.iter().map(|i| format!("\"{i}\"")).collect();
        format!("[{}]", quoted.join(", ")).replace('"', "\\\"")
    }

    #[test]
    fn translate_round_trip_against_a_local_listener() {
        let base = canned_server(format!("{{\"choices\":[{{\"message\":{{\"content\":\"{}\"}}}}]}}", content_array(&["Karawang Timur Subdistrict"])));
        let mut t = OnlineTranslator::with_base("test-key", "llama-3.3-70b", base);
        let cancel = AtomicBool::new(false);
        let out = t.translate(&["Kecamatan Karawang Timur".into()], Lang::Indonesian, Lang::English, &cancel, &mut |_, _| {}).expect("round trip");
        assert_eq!(out, vec!["Karawang Timur Subdistrict"]);
    }

    #[test]
    fn translate_reports_a_count_mismatch_as_an_error() {
        let base = canned_server(format!("{{\"choices\":[{{\"message\":{{\"content\":\"{}\"}}}}]}}", content_array(&["satu"])));
        let mut t = OnlineTranslator::with_base("test-key", "m", base);
        let cancel = AtomicBool::new(false);
        let err = t
            .translate(&["satu".into(), "dua".into()], Lang::Indonesian, Lang::English, &cancel, &mut |_, _| {})
            .err()
            .expect("mismatch is an error");
        assert!(matches!(err, TranslateError::Provider(_)), "{err}");
    }
}
