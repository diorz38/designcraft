//! Cloudflare Workers AI REST client: one POST per image, cover-bucket PNG back.
//!
//! Endpoint: `POST {base}/accounts/{account_id}/ai/run/@cf/black-forest-labs/flux-1-schnell`
//! with a JSON body (`prompt`, `steps` only — the schema rejects every other
//! property) and `Bearer` auth. The reply is JSON `{"result":{"image":"<base64
//! JPEG>"}}`: decoded here, then center-cropped to the requested aspect bucket
//! and re-encoded as PNG, so callers keep the old bytes-in / image-bytes-out
//! contract. If Cloudflare ever changes that contract, this file is the only
//! place that adapts.
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
/// A ~1MP image never reaches this; a prose error page does. Input-derived cap.
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

    /// Generate one image; returns PNG bytes in the requested aspect bucket.
    pub fn generate(&self, req: &GenRequest, cancel: &AtomicBool) -> Result<Vec<u8>, ImageGenError> {
        #[cfg(feature = "cloudflare")]
        {
            if cancel.load(Ordering::Relaxed) {
                return Err(ImageGenError::Cancelled);
            }
            let url = format!("{}/accounts/{}/ai/run/@cf/black-forest-labs/flux-1-schnell", self.base_url, self.account_id);
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
            let mut raw = Vec::new();
            // `into_reader()` hands back a trait object; `by_ref()` makes it `Sized`
            // so `take` applies and the cap actually bounds the read.
            reader.by_ref().take((MAX_IMAGE_BYTES + 1) as u64).read_to_end(&mut raw)?;
            if raw.len() > MAX_IMAGE_BYTES {
                return Err(ImageGenError::BadResponse(format!("response exceeds the {MAX_IMAGE_BYTES}-byte cap")));
            }
            if raw.is_empty() {
                return Err(ImageGenError::BadResponse("provider returned an empty response".into()));
            }
            let jpeg = decode_flux_image(&raw)?;
            to_cover_png(&jpeg, req)
        }
        #[cfg(not(feature = "cloudflare"))]
        {
            let _ = (req, cancel);
            Err(ImageGenError::Unavailable)
        }
    }
}

/// The exact JSON contract of the flux-1-schnell text-to-image call (pinned by a
/// test). The schema is strict: `width`, `height`, `seed` and `negative_prompt`
/// are each rejected with error 5006 (verified against the live endpoint), so
/// those GenRequest fields stay local — the target size drives the post-crop
/// ([`to_cover_png`]) and seeds cannot pin variations on this endpoint.
#[cfg(feature = "cloudflare")]
fn request_body(req: &GenRequest) -> String {
    serde_json::json!({
        "prompt": req.prompt,
        "steps": flux_steps(req.num_steps),
    })
    .to_string()
}

/// flux-1-schnell is a distilled 4-step model; the endpoint accepts 1–8 steps.
/// Presets ask for SDXL-era 15–18, which lands at the 8-step ceiling.
#[cfg(feature = "cloudflare")]
fn flux_steps(steps: u32) -> u32 {
    steps.clamp(1, 8)
}

/// The flux reply wraps the image as base64 inside JSON — unlike SDXL, which
/// answered with raw image bytes. A malformed body is a provider contract
/// change: error, never a guess.
#[cfg(feature = "cloudflare")]
fn decode_flux_image(raw: &[u8]) -> Result<Vec<u8>, ImageGenError> {
    let v: serde_json::Value = serde_json::from_slice(raw).map_err(|e| ImageGenError::BadResponse(format!("reply is not JSON: {e}")))?;
    let b64 =
        v.pointer("/result/image").and_then(|s| s.as_str()).ok_or_else(|| ImageGenError::BadResponse("reply has no result.image field".into()))?;
    use base64::Engine as _;
    let bytes =
        base64::engine::general_purpose::STANDARD.decode(b64).map_err(|e| ImageGenError::BadResponse(format!("result.image is not base64: {e}")))?;
    if bytes.is_empty() {
        return Err(ImageGenError::BadResponse("provider returned an empty image".into()));
    }
    Ok(bytes)
}

/// flux always emits a fixed 1024×1024 square; the cover pipeline expects the
/// aspect bucket the request asked for. Center-crop to the target ratio, then
/// resize to the exact bucket pixels (at most ~1.2× upscale — invisible on flat
/// vector art). Output is PNG so callers keep the SDXL-era bytes contract.
#[cfg(feature = "cloudflare")]
fn to_cover_png(jpeg: &[u8], req: &GenRequest) -> Result<Vec<u8>, ImageGenError> {
    if req.width == 0 || req.height == 0 {
        return Err(ImageGenError::BadResponse(format!("invalid target dimensions {}×{}", req.width, req.height)));
    }
    let src = image::load_from_memory(jpeg).map_err(|e| ImageGenError::BadResponse(format!("image decode failed: {e}")))?;
    let (sw, sh) = (src.width(), src.height());
    let target_ratio = f64::from(req.width) / f64::from(req.height);
    let src_ratio = f64::from(sw) / f64::from(sh);
    // Largest centered crop with the target ratio (checked: dims come from the
    // provider, ratios from the Aspect table — both bounded, clamp anyway).
    let (cw, ch) = if src_ratio > target_ratio { ((f64::from(sh) * target_ratio) as u32, sh) } else { (sw, (f64::from(sw) / target_ratio) as u32) };
    let cw = cw.clamp(1, sw);
    let ch = ch.clamp(1, sh);
    let x = sw.saturating_sub(cw) / 2;
    let y = sh.saturating_sub(ch) / 2;
    let resized = src.crop_imm(x, y, cw, ch).resize_exact(req.width, req.height, image::imageops::FilterType::Lanczos3);
    let mut out = Vec::new();
    resized
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| ImageGenError::BadResponse(format!("PNG encode failed: {e}")))?;
    Ok(out)
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

    /// A tiny real JPEG wrapped in the flux JSON envelope, as the live endpoint sends it.
    fn flux_reply_jpeg(w: u32, h: u32) -> Vec<u8> {
        use base64::Engine as _;
        let img = image::DynamicImage::new_rgb8(w, h);
        let mut jpeg = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg).expect("encode jpeg");
        format!("{{\"result\":{{\"image\":\"{}\"}}}}", base64::engine::general_purpose::STANDARD.encode(jpeg)).into_bytes()
    }

    #[test]
    fn request_body_pins_the_contract() {
        let v: serde_json::Value = serde_json::from_str(&request_body(&req())).expect("valid JSON");
        assert_eq!(v.pointer("/prompt"), Some(&serde_json::json!("retro waves cover background")));
        assert_eq!(v.pointer("/steps"), Some(&serde_json::json!(8)), "18 clamps to the flux ceiling");
        assert_eq!(flux_steps(1), 1);
        assert_eq!(flux_steps(8), 8);
        assert_eq!(flux_steps(99), 8);
        // The strict flux schema rejects every other property (live-verified error 5006).
        for absent in ["negative_prompt", "width", "height", "seed", "guidance", "num_steps"] {
            assert!(v.get(absent).is_none(), "{absent} must be absent from the flux body");
        }
    }

    #[test]
    fn generate_round_trips_image_bytes_against_a_local_listener() {
        // Iso bucket: 1024×1024 square reply must come back as an 832×1176 PNG.
        let body = flux_reply_jpeg(1024, 1024);
        let base = canned_server("HTTP/1.1 200 OK", "", body);
        let c = CloudflareClient::with_base("acct", "tok", base);
        let cancel = AtomicBool::new(false);
        let out = c.generate(&req(), &cancel).expect("round trip");
        assert_eq!(&out[..4], b"\x89PNG", "output is PNG bytes");
        let img = image::load_from_memory(&out).expect("decodable PNG");
        assert_eq!((img.width(), img.height()), (832, 1176), "cropped+resized to the Iso bucket");
    }

    #[test]
    fn wide_bucket_crops_to_landscape() {
        let body = flux_reply_jpeg(1024, 1024);
        let base = canned_server("HTTP/1.1 200 OK", "", body);
        let c = CloudflareClient::with_base("acct", "tok", base);
        let cancel = AtomicBool::new(false);
        let mut r = req();
        r.width = 1216;
        r.height = 832;
        let out = c.generate(&r, &cancel).expect("round trip");
        let img = image::load_from_memory(&out).expect("decodable PNG");
        assert_eq!((img.width(), img.height()), (1216, 832), "Wrap bucket");
    }

    #[test]
    fn generate_rejects_malformed_flux_replies() {
        let cancel = AtomicBool::new(false);
        // 200 with a non-JSON body is not a flux reply.
        let base = canned_server("HTTP/1.1 200 OK", "", b"not json".to_vec());
        let err = CloudflareClient::with_base("a", "t", base).generate(&req(), &cancel).err().expect("non-JSON errors");
        assert!(matches!(err, ImageGenError::BadResponse(_)), "{err}");
        // 200 with JSON but no result.image.
        let base = canned_server("HTTP/1.1 200 OK", "", br#"{"result":{}}"#.to_vec());
        let err = CloudflareClient::with_base("a", "t", base).generate(&req(), &cancel).err().expect("missing field errors");
        assert!(matches!(err, ImageGenError::BadResponse(_)), "{err}");
        // 200 with JSON but invalid base64.
        let base = canned_server("HTTP/1.1 200 OK", "", br#"{"result":{"image":"!!not-base64!!"}}"#.to_vec());
        let err = CloudflareClient::with_base("a", "t", base).generate(&req(), &cancel).err().expect("bad base64 errors");
        assert!(matches!(err, ImageGenError::BadResponse(_)), "{err}");
        // 200 with base64 that is not a decodable image.
        let base = canned_server("HTTP/1.1 200 OK", "", br#"{"result":{"image":"AAAA"}}"#.to_vec());
        let err = CloudflareClient::with_base("a", "t", base).generate(&req(), &cancel).err().expect("bad image errors");
        assert!(matches!(err, ImageGenError::BadResponse(_)), "{err}");
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
        // 200 with an empty body is not a flux reply.
        let base = canned_server("HTTP/1.1 200 OK", "", Vec::new());
        let err = CloudflareClient::with_base("a", "t", base).generate(&req(), &cancel).err().expect("empty errors");
        assert!(matches!(err, ImageGenError::BadResponse(_)), "{err}");
    }

    #[test]
    fn generate_honours_the_cancel_flag_before_sending() {
        let base = canned_server("HTTP/1.1 200 OK", "", flux_reply_jpeg(4, 4));
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
        r.num_steps = 1;
        let res = client.generate(&r, &cancel);
        match res {
            Ok(bytes) => {
                println!("SUCCESS: Received {} PNG bytes from Cloudflare flux-1-schnell!", bytes.len());
                assert_eq!(&bytes[..4], b"\x89PNG");
                let img = image::load_from_memory(&bytes).expect("decodable PNG");
                assert_eq!((img.width(), img.height()), (832, 1176), "Iso bucket out");
            }
            Err(e) => {
                panic!("Cloudflare reachability check failed: {e}");
            }
        }
    }
}
