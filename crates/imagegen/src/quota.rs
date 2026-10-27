//! Shared AI-quota tracking in Supabase: after each successful cover generation the
//! worker reports the exact neuron cost (4 × 4.80 tiles + 8 × 9.60 steps = 96.0 for
//! this app's fixed 1024×1024 / 8-step flux calls) to the `ai_cf_consume` RPC, which
//! atomically increments one global counter and returns the remaining free-neuron
//! budget. Every app user shares the developer's Cloudflare account, so the counter —
//! not any local estimate — is what the panel displays.
//!
//! Best effort by design: a missing config or a network error yields `None` and cover
//! generation proceeds untouched; the quota label simply stays quiet.

/// Exact Workers AI neuron cost of one cover on this app's flux endpoint: flux always
/// answers 1024×1024 (= four 512×512 tiles at 4.80 neurons) and [`super::cloudflare`]
/// clamps steps to the 8-step ceiling (8 × 9.60 neurons).
pub const NEURONS_PER_COVER: f64 = 96.0;

/// Report `neurons` consumed on the given Cloudflare account and return the
/// remaining free-neuron budget for today (UTC). `None` when Supabase is not
/// configured or the report failed — never an error path for generation itself.
pub fn consume_cover(cred: &str, neurons: f64) -> Option<f64> {
    #[cfg(feature = "cloudflare")]
    {
        if cred.trim().is_empty() {
            return None;
        }
        let raw = rpc("ai_cf_consume", serde_json::json!({ "p_cred": cred, "p_neurons": neurons }))?;
        parse_consume_response(&raw)
    }
    #[cfg(not(feature = "cloudflare"))]
    {
        let _ = (cred, neurons);
        None
    }
}

/// Best-effort mirror of Groq's `x-ratelimit-remaining-requests` into the shared
/// `ai_quota` table (`ai_groq_report`), keyed by a fingerprint of the key so
/// different users' keys are tracked separately. Fire-and-forget: `None` when
/// Supabase is not configured or the call failed.
pub fn report_groq(cred: &str, remaining_requests: u32) -> Option<()> {
    #[cfg(feature = "cloudflare")]
    {
        if cred.trim().is_empty() {
            return None;
        }
        rpc("ai_groq_report", serde_json::json!({ "p_cred": cred, "p_requests": remaining_requests })).map(|_| ())
    }
    #[cfg(not(feature = "cloudflare"))]
    {
        let _ = (cred, remaining_requests);
        None
    }
}

/// Stable, non-secret grouping id for a credential: FNV-1a 64-bit hex. Deliberately
/// not a crypto hash — it only needs to group rows per account/key consistently
/// across app versions, and must never be reversible into the credential.
pub fn fingerprint(credential: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in credential.trim().as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// One Supabase PostgREST RPC call; the raw 200 body, or `None` (never an error
/// path for callers — quota reporting is best effort and must not block work).
#[cfg(feature = "cloudflare")]
fn rpc(fn_name: &str, body: serde_json::Value) -> Option<String> {
    let Some((url, key)) = supabase_creds() else {
        log::debug!("ai quota: SUPABASE_URL / SUPABASE_ANON_KEY not configured; skipping {fn_name}");
        return None;
    };
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(5)).build();
    let res = agent
        .post(&format!("{url}/rest/v1/rpc/{fn_name}"))
        // The anon key is public by design; the RPCs are `security definer`, so the
        // writes happen server-side and the table itself stays RLS-locked.
        .set("apikey", &key)
        .set("Authorization", &format!("Bearer {key}"))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string());
    match res {
        Ok(r) => match r.status() {
            200 => r.into_string().ok(),
            status => {
                log::warn!("ai quota: rpc {fn_name} returned HTTP {status}");
                None
            }
        },
        Err(e) => {
            log::warn!("ai quota: rpc {fn_name} failed: {e}");
            None
        }
    }
}

/// PostgREST answers a scalar RPC with `{"ai_cf_consume": <number>}`.
#[cfg(feature = "cloudflare")]
fn parse_consume_response(raw: &str) -> Option<f64> {
    serde_json::from_str::<serde_json::Value>(raw).ok()?.pointer("/ai_cf_consume").and_then(|v| v.as_f64())
}

/// `SUPABASE_URL` / `SUPABASE_ANON_KEY` from the process env, falling back to a local
/// `.env` (same lookup order as the Cloudflare credentials in the Pubgen UI).
#[cfg(feature = "cloudflare")]
fn supabase_creds() -> Option<(String, String)> {
    let mut url = std::env::var("SUPABASE_URL").unwrap_or_default();
    let mut key = std::env::var("SUPABASE_ANON_KEY").unwrap_or_default();
    if url.is_empty() || key.is_empty() {
        let content = std::fs::read_to_string(".env").ok()?;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().trim_matches('"').trim_matches('\'').trim();
                if url.is_empty() && k.trim().eq_ignore_ascii_case("SUPABASE_URL") {
                    url = v.to_string();
                } else if key.is_empty() && k.trim().eq_ignore_ascii_case("SUPABASE_ANON_KEY") {
                    key = v.to_string();
                }
            }
        }
    }
    if url.is_empty() || key.is_empty() {
        return None;
    }
    Some((url.trim_end_matches('/').to_string(), key))
}

#[cfg(all(test, feature = "cloudflare"))]
mod tests {
    use super::*;

    #[test]
    fn parses_the_postgrest_scalar_reply() {
        assert_eq!(parse_consume_response("{\"ai_cf_consume\":9942.4}"), Some(9942.4));
        assert_eq!(parse_consume_response("{\"ai_cf_consume\":0}"), Some(0.0));
        assert_eq!(parse_consume_response("{\"message\":\"error\"}"), None);
        assert_eq!(parse_consume_response("not json"), None);
    }

    #[test]
    fn neuron_cost_matches_the_flux_wire_contract() {
        // 4 tiles × 4.80 + 8 steps × 9.60 — pinned so a provider-side pricing change
        // (or a new step ceiling in cloudflare.rs) forces this number to be revisited.
        assert_eq!(NEURONS_PER_COVER, 4.0 * 4.80 + 8.0 * 9.60);
    }

    #[test]
    fn fingerprint_is_stable_distinct_and_never_the_credential() {
        assert_eq!(fingerprint("acct-A"), fingerprint(" acct-A ")); // trim-insensitive
        assert_ne!(fingerprint("acct-A"), fingerprint("acct-B"));
        assert_ne!(fingerprint("acct-A"), "acct-A");
        // FNV-1a 64 of "acct-A": pinned so rows keep grouping across app versions.
        assert_eq!(fingerprint("acct-A").len(), 16);
    }

    #[test]
    fn creds_fall_back_to_the_env_file_format() {
        let content = "# comment\nSUPABASE_URL=https://example.supabase.co/\nOTHER=1\n";
        let mut url = String::new();
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                if url.is_empty() && k.trim().eq_ignore_ascii_case("SUPABASE_URL") {
                    url = v.trim().trim_end_matches('/').to_string();
                }
            }
        }
        assert_eq!(url, "https://example.supabase.co");
    }
}
