//! Groq (OpenAI-compatible) chat-completions client: batched JSON-array translation
//! with fence stripping, count-mismatch errors and 429 Retry-After backoff.
//!
//! The API key lives only in the Authorization header: never logged, never in an
//! error string, never written into a document. Without the \`groq\` feature the type
//! exists and every call returns \`TranslateError::Unavailable\`.

#[cfg(feature = "groq")]
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicU32};

use super::{Lang, TranslateError};

/// Last `x-ratelimit-remaining-requests` value Groq returned for this key
/// (server-computed, so it already covers every consumer of the key).
/// `u32::MAX` = nothing seen yet.
static GROQ_REMAINING_REQUESTS: AtomicU32 = AtomicU32::new(u32::MAX);

/// Approximate remaining chat-completion requests on the configured Groq key
/// today, straight from Groq's own rate-limit response header. `None` until the
/// first request against the key in this app session.
pub fn remaining_requests() -> Option<u32> {
    match GROQ_REMAINING_REQUESTS.load(std::sync::atomic::Ordering::Relaxed) {
        u32::MAX => None,
        v => Some(v),
    }
}

/// Remember the rate-limit header from a chat-completions response (200 or 429).
#[cfg(feature = "groq")]
fn store_remaining(r: &ureq::Response) {
    if let Some(v) = r.header("x-ratelimit-remaining-requests").and_then(|v| v.trim().parse::<u32>().ok()) {
        GROQ_REMAINING_REQUESTS.store(v, Ordering::Relaxed);
    }
}

/// Units per request: small enough for free-tier rate caps, big enough to amortise.
#[cfg(feature = "groq")]
const BATCH: usize = 16;
#[cfg(feature = "groq")]
const MAX_429_RETRIES: usize = 3;
/// Terminology pairs appended to the system prompt (bounded, sorted upstream).
#[cfg(feature = "groq")]
const TERMS_IN_PROMPT: usize = 200;

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
        glossary: Option<&[(String, String)]>,
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
                out.extend(self.translate_batch(batch, src, tgt, glossary, cancel)?);
                progress(out.len(), texts.len());
            }
            Ok(out)
        }
        #[cfg(not(feature = "groq"))]
        {
            let _ = (texts, src, tgt, glossary, cancel, progress);
            Err(TranslateError::Unavailable)
        }
    }

    #[cfg(feature = "groq")]
    fn translate_batch(
        &self,
        batch: &[String],
        src: Lang,
        tgt: Lang,
        glossary: Option<&[(String, String)]>,
        cancel: &AtomicBool,
    ) -> Result<Vec<String>, TranslateError> {
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
                    out.extend(self.request_once(std::slice::from_ref(item), src, tgt, glossary)?);
                }
                Ok(out)
            } else {
                self.request_once(batch, src, tgt, glossary)
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
    fn request_once(&self, batch: &[String], src: Lang, tgt: Lang, glossary: Option<&[(String, String)]>) -> Result<Vec<String>, TranslateError> {
        let body = request_body(&self.model, batch, src, tgt, glossary);
        let raw = self.post_chat(&body)?;
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

    /// One chat completion with a caller-supplied prompt (the narrative drafter). Same auth,
    /// status and `Retry-After` handling as the translation call; `temperature: 0`.
    pub fn complete(&self, system: &str, user: &str) -> Result<String, TranslateError> {
        #[cfg(feature = "groq")]
        {
            let body = chat_body(&self.model, system, user);
            let mut attempt = 0usize;
            let raw = loop {
                match self.post_chat(&body) {
                    Err(TranslateError::RateLimited(retry_after)) if attempt < MAX_429_RETRIES => {
                        attempt += 1;
                        std::thread::sleep(std::time::Duration::from_secs(retry_after));
                    }
                    other => break other?,
                }
            };
            let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| TranslateError::Provider(format!("bad JSON: {e}")))?;
            v.pointer("/choices/0/message/content")
                .and_then(|c| c.as_str())
                .map(str::to_string)
                .ok_or_else(|| TranslateError::Provider("response has no choices[0].message.content".into()))
        }
        #[cfg(not(feature = "groq"))]
        {
            let _ = (system, user);
            Err(TranslateError::Unavailable)
        }
    }

    /// POST one chat-completions body; the model's raw JSON reply on 200.
    #[cfg(feature = "groq")]
    fn post_chat(&self, body: &str) -> Result<String, TranslateError> {
        let res = self
            .agent
            .post(&format!("{}/chat/completions", self.base_url))
            .set("Authorization", &format!("Bearer {}", self.key))
            .set("Content-Type", "application/json")
            .send_string(body);
        let (status, raw) = match res {
            Ok(r) => {
                store_remaining(&r);
                (r.status(), r.into_string().map_err(|e| TranslateError::Provider(e.to_string()))?)
            }
            Err(ureq::Error::Status(code, r)) => {
                store_remaining(&r);
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
        Ok(raw)
    }
}

/// One `temperature: 0` chat completion with a caller-supplied system/user prompt.
#[cfg(feature = "groq")]
fn chat_body(model: &str, system: &str, user: &str) -> String {
    let mut body = serde_json::json!({
        "model": model,
        "temperature": 0,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
    });
    if model.contains("gpt-oss")
        && let Some(obj) = body.as_object_mut()
    {
        obj.insert("reasoning_effort".into(), serde_json::json!("low"));
    }
    body.to_string()
}

/// Units per request kept at module scope for the tests that build batches.
#[cfg(feature = "groq")]
fn request_body(model: &str, batch: &[String], src: Lang, tgt: Lang, glossary: Option<&[(String, String)]>) -> String {
    let items = serde_json::to_string(batch).unwrap_or_default();
    let mut system = format!(
        "You are a translation engine. Translate each array item from {} to {}. Return a JSON \
         array of the same length and order. Keep numbers, proper nouns and formatting unchanged. \
         Output only the JSON array.",
        src.label(),
        tgt.label()
    );
    // Secondary net behind the deterministic whole-cell aliases: the terms shape the
    // model's word choice inside sentences. Sorted and capped so the prompt is stable
    // and bounded; `None` and an empty list keep today's prompt byte for byte.
    if let Some(terms) = glossary.filter(|t| !t.is_empty()) {
        let pairs: Vec<String> = terms.iter().take(TERMS_IN_PROMPT).map(|(id, en)| format!("{id} = {en}")).collect();
        system.push_str(&format!("\n\nOfficial terminology you MUST use: {}.", pairs.join("; ")));
    }
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
        let body = request_body("llama-3.3-70b", &["Kabupaten Karawang".into()], Lang::Indonesian, Lang::English, None);
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

    #[test]
    fn empty_glossary_keeps_the_prompt_byte_identical() {
        let plain = request_body("m", &["Jumlah".into()], Lang::Indonesian, Lang::English, None);
        let empty = request_body("m", &["Jumlah".into()], Lang::Indonesian, Lang::English, Some(&[]));
        assert_eq!(plain, empty);
        assert!(!plain.contains("Official terminology"), "{plain}");
    }

    #[test]
    fn glossary_terms_join_the_system_prompt() {
        let terms = vec![("Kabupaten".to_string(), "Regency".to_string()), ("Kecamatan".to_string(), "Subdistrict".to_string())];
        let body = request_body("m", &["Kecamatan Karawang".into()], Lang::Indonesian, Lang::English, Some(&terms));
        let v: serde_json::Value = serde_json::from_str(&body).expect("request body is valid JSON");
        let system = v.pointer("/messages/0/content").and_then(|c| c.as_str()).expect("system message");
        assert!(system.contains("Official terminology"), "{system}");
        assert!(system.contains("Kecamatan = Subdistrict"), "{system}");
        assert!(system.contains("Kabupaten = Regency"), "{system}");
        let user = v.pointer("/messages/1/content").and_then(|c| c.as_str()).expect("user message");
        assert_eq!(user, "[\"Kecamatan Karawang\"]", "the glossary must not leak into the payload");
    }

    #[test]
    fn glossary_terms_are_capped_at_the_prompt_budget() {
        let terms: Vec<(String, String)> = (0..TERMS_IN_PROMPT + 1).map(|i| (format!("istilah{i:03}"), format!("term{i:03}"))).collect();
        let body = request_body("m", &["x".into()], Lang::Indonesian, Lang::English, Some(&terms));
        let v: serde_json::Value = serde_json::from_str(&body).expect("request body is valid JSON");
        let system = v.pointer("/messages/0/content").and_then(|c| c.as_str()).expect("system message");
        assert!(system.contains("istilah199 = term199"), "{system}");
        assert!(!system.contains("istilah200 = term200"), "the 201st term must be dropped");
    }

    #[test]
    fn chat_body_pins_zero_temperature_and_both_prompts() {
        let body = chat_body("llama-3.3-70b", "system prompt", "user payload");
        let v: serde_json::Value = serde_json::from_str(&body).expect("chat body is valid JSON");
        assert_eq!(v.pointer("/temperature"), Some(&serde_json::json!(0)));
        assert_eq!(v.pointer("/messages/0/role"), Some(&serde_json::json!("system")));
        assert_eq!(v.pointer("/messages/0/content"), Some(&serde_json::json!("system prompt")));
        assert_eq!(v.pointer("/messages/1/role"), Some(&serde_json::json!("user")));
        assert_eq!(v.pointer("/messages/1/content"), Some(&serde_json::json!("user payload")));
        assert!(v.get("reasoning_effort").is_none(), "only gpt-oss gets the effort hint");
        let oss: serde_json::Value = serde_json::from_str(&chat_body("openai/gpt-oss-20b", "s", "u")).expect("valid JSON");
        assert_eq!(oss.get("reasoning_effort"), Some(&serde_json::json!("low")));
    }

    #[test]
    fn complete_round_trips_against_a_local_listener() {
        let content = "{\"blocks\":[{\"role\":\"lead\",\"text\":\"Hello\"}]}";
        let reply = serde_json::json!({"choices": [{"message": {"content": content}}]}).to_string();
        let base = canned_server(reply);
        let t = OnlineTranslator::with_base("test-key", "llama-3.3-70b", base);
        assert_eq!(t.complete("system", "user").expect("completion"), content);
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
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nx-ratelimit-remaining-requests: 812\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
        let out =
            t.translate(&["Kecamatan Karawang Timur".into()], Lang::Indonesian, Lang::English, None, &cancel, &mut |_, _| {}).expect("round trip");
        assert_eq!(out, vec!["Karawang Timur Subdistrict"]);
        // The canned reply carries x-ratelimit-remaining-requests: 812; the client must surface it.
        assert_eq!(remaining_requests(), Some(812));
    }

    #[test]
    fn translate_reports_a_count_mismatch_as_an_error() {
        let base = canned_server(format!("{{\"choices\":[{{\"message\":{{\"content\":\"{}\"}}}}]}}", content_array(&["satu"])));
        let mut t = OnlineTranslator::with_base("test-key", "m", base);
        let cancel = AtomicBool::new(false);
        let err = t
            .translate(&["satu".into(), "dua".into()], Lang::Indonesian, Lang::English, None, &cancel, &mut |_, _| {})
            .err()
            .expect("mismatch is an error");
        assert!(matches!(err, TranslateError::Provider(_)), "{err}");
    }
}
