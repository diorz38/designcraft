//! Cloudflare Workers AI REST client: one POST per image, binary image bytes back.
//!
//! Endpoint: `POST {base}/accounts/{account_id}/ai/run/@cf/stabilityai/stable-diffusion-xl-base-1.0`
//! with a JSON body (prompt, negative_prompt, width, height, num_steps, optional seed)
//! and `Bearer` auth. The reply body is the encoded image (PNG/JPEG) — if Cloudflare
//! ever changes that contract, this file is the only place that adapts.
//!
//! Without the `cloudflare` feature the type exists and every call returns
//! [`ImageGenError::Unavailable`].

use std::sync::atomic::AtomicBool;
#[cfg(feature = "cloudflare")]
use std::sync::atomic::Ordering;

use super::{GenRequest, ImageGenError};
#[cfg(feature = "cloudflare")]
use std::io::Read as _;

/// One generation costs well under 30 s on the free tier; leave headroom for
/// queueing at peak (the caller's worker thread owns the waiting, not the UI).
#[cfg(feature = "cloudflare")]
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
/// A ~1MP PNG never reaches this; a prose error page does. Input-derived cap.
#[cfg(feature = "cloudflare")]
const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;

/// Cloudflare Workers AI client (user's own account + scoped API token).
#[cfg_attr(not(feature = "cloudflare"), allow(dead_code))]
pub struct CloudflareClient {
    account_id: String,
    token: String,
    #[cfg(feature = "cloudflare")]
    agent: ureq::Agent,
    /// Overridable for tests (a canned local listener stands in for api.cloudflare.com).
    #[cfg(feature = "cloudflare")]
    base_url: String,
}

impl CloudflareClient {
    pub fn new(account_id: &str, token: &str) -> Self {
        Self {
            account_id: account_id.trim().to_string(),
            token: token.trim().to_string(),
            #[cfg(feature = "cloudflare")]
            agent: ureq::AgentBuilder::new().timeout(TIMEOUT).build(),
            #[cfg(feature = "cloudflare")]
            base_url: "https://api.cloudflare.com/client/v4".to_string(),
        }
    }

    #[cfg(all(test, feature = "cloudflare"))]
    fn with_base(account_id: &str, token: &str, base_url: String) -> Self {
        let mut c = Self::new(account_id, token);
        c.base_url = base_url;
        c
    }

    /// Generate one image; returns the encoded image bytes.
    pub fn generate(&self, req: &GenRequest, cancel: &AtomicBool) -> Result<Vec<u8>, ImageGenError> {
        #[cfg(feature = "cloudflare")]
        {
            if cancel.load(Ordering::Relaxed) {
                return Err(ImageGenError::Cancelled);
            }
            let url = format!("{}/accounts/{}/ai/run/@cf/stabilityai/stable-diffusion-xl-base-1.0", self.base_url, self.account_id);
            let res = self
                .agent
                .post(&url)
                .set("Authorization", &format!("Bearer {}", self.token))
                .set("Content-Type", "application/json")
                .send_string(&request_body(req));
            let (status, mut reader) = match res {
                Ok(r) => (r.status(), r.into_reader()),
                Err(ureq::Error::Status(code, r)) => {
                    let retry = r.header("Retry-After").and_then(|v| v.trim().parse::<u64>().ok());
                    // Never echo the token: error bodies are truncated for context only.
                    let raw = r.into_string().unwrap_or_default();
                    match code {
                        429 => return Err(ImageGenError::Quota(retry.map(|s| s.clamp(1, 3600)))),
                        401 | 403 => return Err(ImageGenError::Auth),
                        _ => return Err(ImageGenError::BadResponse(format!("HTTP {code}: {}", truncate(&raw, 300)))),
                    }
                }
                Err(e) => return Err(ImageGenError::Network(format!("request failed: {e}"))),
            };
            if !(200..300).contains(&status) {
                return Err(ImageGenError::BadResponse(format!("HTTP {status}")));
            }
            let mut bytes = Vec::new();
            // `into_reader()` hands back a trait object; `by_ref()` makes it `Sized`
            // so `take` applies and the cap actually bounds the read.
            reader.by_ref().take((MAX_IMAGE_BYTES + 1) as u64).read_to_end(&mut bytes)?;
            if bytes.len() > MAX_IMAGE_BYTES {
                return Err(ImageGenError::BadResponse(format!("image exceeds the {MAX_IMAGE_BYTES}-byte cap")));
            }
            if bytes.is_empty() {
                return Err(ImageGenError::BadResponse("provider returned an empty image".into()));
            }
            Ok(bytes)
        }
        #[cfg(not(feature = "cloudflare"))]
        {
            let _ = (req, cancel);
            Err(ImageGenError::Unavailable)
        }
    }
}

/// The exact JSON contract of the SDXL text-to-image call (pinned by a test).
#[cfg(feature = "cloudflare")]
fn request_body(req: &GenRequest) -> String {
    // `guidance` is left out on purpose: the server default (7.5) is what the presets
    // assume, and unknown params are the one thing a provider may reject instead of
    // ignoring. ponytail: add it when a preset first needs a different CFG.
    let mut body = serde_json::json!({
        "prompt": req.prompt,
        "negative_prompt": req.negative_prompt,
        "num_steps": req.num_steps,
        "width": req.width,
        "height": req.height,
    });
    if let Some(seed) = req.seed
        && let Some(obj) = body.as_object_mut()
    {
        obj.insert("seed".into(), serde_json::json!(seed));
    }
    body.to_string()
}

#[cfg(feature = "cloudflare")]
fn truncate(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

pub fn compiled_in() -> bool {
    cfg!(feature = "cloudflare")
}

#[cfg(all(test, feature = "cloudflare"))]
mod tests {
    use super::*;

    /// A canned TCP listener stands in for api.cloudflare.com: one POST, one reply.
    fn canned_server(status_line: &'static str, extra_headers: &'static str, body: Vec<u8>) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            use std::io::{Read, Write};
            // Read the whole request (headers + Content-Length body) before replying.
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
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra_headers}Connection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(reply.as_bytes());
            let _ = stream.write_all(&body);
            let _ = stream.flush();
        });
        format!("http://{addr}/client/v4")
    }

    fn req() -> GenRequest {
        let mut r = GenRequest::new("retro waves cover background", GenRequest::clean_cover_negative_prompt(), super::super::Aspect::Iso, 18);
        r.seed = Some(42);
        r
    }

    #[test]
    fn request_body_pins_the_contract() {
        let v: serde_json::Value = serde_json::from_str(&request_body(&req())).expect("valid JSON");
        assert_eq!(v.pointer("/prompt"), Some(&serde_json::json!("retro waves cover background")));
        assert!(v.pointer("/negative_prompt").and_then(|n| n.as_str()).is_some_and(|n| n.contains("typography")));
        assert_eq!(v.pointer("/num_steps"), Some(&serde_json::json!(18)));
        assert_eq!(v.pointer("/width"), Some(&serde_json::json!(832)));
        assert_eq!(v.pointer("/height"), Some(&serde_json::json!(1176)));
        assert_eq!(v.pointer("/seed"), Some(&serde_json::json!(42)));
        // No seed: the field is absent, so the provider rolls a fresh variation.
        let mut no_seed = req();
        no_seed.seed = None;
        let v: serde_json::Value = serde_json::from_str(&request_body(&no_seed)).expect("valid JSON");
        assert!(v.get("seed").is_none(), "seed must be absent, not null");
        assert!(v.get("guidance").is_none(), "guidance stays at the server default");
    }

    #[test]
    fn generate_round_trips_image_bytes_against_a_local_listener() {
        let png = vec![0x89, b'P', b'N', b'G', 1, 2, 3, 4, 5];
        let base = canned_server("HTTP/1.1 200 OK", "", png.clone());
        let c = CloudflareClient::with_base("acct", "tok", base);
        let cancel = AtomicBool::new(false);
        let out = c.generate(&req(), &cancel).expect("round trip");
        assert_eq!(out, png);
    }

    #[test]
    fn generate_maps_quota_auth_and_bad_responses() {
        let cancel = AtomicBool::new(false);
        // 429 with Retry-After: the daily free allocation is spent.
        let base = canned_server(
            "HTTP/1.1 429 Too Many Requests",
            "Retry-After: 120\r\n",
            br#"{"errors":[{"code":4006,"message":"Daily free limit reached"}]}"#.to_vec(),
        );
        let err = CloudflareClient::with_base("a", "t", base).generate(&req(), &cancel).err().expect("429 errors");
        assert!(matches!(err, ImageGenError::Quota(Some(120))), "{err}");
        // 401: credentials rejected, never leaking the token.
        let base = canned_server("HTTP/1.1 403 Forbidden", "", br#"{"errors":[{"code":10000}]}"#.to_vec());
        let err = CloudflareClient::with_base("a", "t", base).generate(&req(), &cancel).err().expect("403 errors");
        assert!(matches!(err, ImageGenError::Auth), "{err}");
        // 5xx: provider answered, image unusable.
        let base = canned_server("HTTP/1.1 500 Internal Server Error", "", b"boom".to_vec());
        let err = CloudflareClient::with_base("a", "t", base).generate(&req(), &cancel).err().expect("500 errors");
        assert!(matches!(err, ImageGenError::BadResponse(_)), "{err}");
        // 200 with an empty body is not an image.
        let base = canned_server("HTTP/1.1 200 OK", "", Vec::new());
        let err = CloudflareClient::with_base("a", "t", base).generate(&req(), &cancel).err().expect("empty errors");
        assert!(matches!(err, ImageGenError::BadResponse(_)), "{err}");
    }

    #[test]
    fn generate_honours_the_cancel_flag_before_sending() {
        let base = canned_server("HTTP/1.1 200 OK", "", vec![1, 2, 3]);
        let c = CloudflareClient::with_base("a", "t", base);
        let cancel = AtomicBool::new(true);
        let err = c.generate(&req(), &cancel).err().expect("cancelled errors");
        assert!(matches!(err, ImageGenError::Cancelled), "{err}");
    }

    #[test]
    #[ignore = "touches live Cloudflare network with .env credentials"]
    fn live_cloudflare_connectivity_test() {
        let (acct, tok) = {
            let mut a = String::new();
            let mut t = String::new();
            let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
            if let Ok(content) = std::fs::read_to_string(env_path) {
                for line in content.lines() {
                    if let Some((k, v)) = line.split_once('=') {
                        let k = k.trim();
                        let v = v.trim().trim_matches('"').trim_matches('\'').trim();
                        if k.eq_ignore_ascii_case("CLOUDFLARE_ACCOUNT_ID") {
                            a = v.to_string();
                        } else if k.eq_ignore_ascii_case("CLOUDFLARE_API_TOKEN") {
                            t = v.to_string();
                        }
                    }
                }
            }
            (a, t)
        };
        assert!(!acct.is_empty(), "CLOUDFLARE_ACCOUNT_ID not found in .env");
        assert!(!tok.is_empty(), "CLOUDFLARE_API_TOKEN not found in .env");

        let client = CloudflareClient::new(&acct, &tok);
        let cancel = AtomicBool::new(false);
        let mut r = req();
        r.width = 256;
        r.height = 256;
        r.num_steps = 1;
        let res = client.generate(&r, &cancel);
        match res {
            Ok(bytes) => {
                println!("SUCCESS: Received {} image bytes from Cloudflare Workers AI!", bytes.len());
                assert!(!bytes.is_empty());
            }
            Err(e) => {
                panic!("Cloudflare reachability check failed: {e}");
            }
        }
    }
}
