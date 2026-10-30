//! AI Base cover styles (style ids 100+): preset data, the prompt builder, the
//! Cloudflare setup dialog and the per-style options pane.
//!
//! The presets are plain data compiled in always (Excel can name them on any build);
//! generation itself needs the `ai-cover-online` feature — without it the pane shows
//! the hint and every entry point returns `imagegen::ImageGenError::Unavailable`.

use egui::RichText;
use serde_json::json;

use super::{PubgenState, Tokens};
use crate::DesignApp;
use designcraft_imagegen as imagegen;

/// Cover styles `AI_STYLE_BASE + i` map to `AI_PRESETS[i]` and to
/// `assets/templates/covers/100_ai_base.designcraft`.
pub const AI_STYLE_BASE: usize = 100;

/// One colour direction: prompt words for the generator + the three hexes the
/// palette binding recolors the cover band and accents with.
pub struct AiPalette {
    pub name: &'static str,
    pub words: &'static str,
    pub primary: &'static str,
    pub accent: &'static str,
    pub dark: &'static str,
}

pub struct AiPreset {
    pub name: &'static str,
    /// The style part of the prompt (scene/motif/art direction).
    pub style_words: &'static str,
    /// Hover description in the style picker.
    pub description: &'static str,
    pub palettes: &'static [AiPalette],
}

pub const AI_PALETTES: &[AiPalette] = &[
    AiPalette {
        name: "Warm Terracotta",
        words: "terracotta red, mustard yellow, sage green, warm cream",
        primary: "#b5543b",
        accent: "#e8a34d",
        dark: "#3e2c23",
    },
    AiPalette {
        name: "Pastel Dream",
        words: "soft pastel pink, lilac, baby blue, off-white",
        primary: "#8e7cc3",
        accent: "#e8a0bf",
        dark: "#4a4458",
    },
    AiPalette {
        name: "Mono Slate",
        words: "monochrome dark slate, charcoal, silver grey, near-black",
        primary: "#53616c",
        accent: "#9fb1bc",
        dark: "#1c252c",
    },
    AiPalette { name: "Forest Gold", words: "deep forest green, moss, gold, cream", primary: "#2f5d50", accent: "#c9a227", dark: "#1c2b24" },
    AiPalette {
        name: "Corporate Blue",
        words: "dark navy blue #0A1931, deep royal blue #003399, vibrant electric cyan blue #00A8FF, soft silver-gray #E0E6ED, clean solid white background",
        primary: "#003399",
        accent: "#00a8ff",
        dark: "#0a1931",
    },
    AiPalette {
        name: "Teal Lime",
        words: "deep teal green, dark pine green, chartreuse lime green gradient, mint turquoise, crisp white",
        primary: "#006d6f",
        accent: "#84cc16",
        dark: "#0a2e28",
    },
];

pub const AI_PRESETS: &[AiPreset] = &[
    AiPreset {
        name: "Retro Waves",
        style_words: "minimalist mid-century modern abstract wallpaper, flowing organic waves and curving colour blocks, 1970s retro aesthetic, smooth flat vector shapes, continuous panoramic art",
        description: "AI-generated mid-century waves and colour blocks under the house typography; the accent band follows the chosen palette.",
        palettes: AI_PALETTES,
    },
    AiPreset {
        name: "Geometric Business",
        style_words: "flat vector 2D modern geometric business wallpaper, dramatic scale contrast with radical hexagon size differences, one massive prominent hero hexagon frame, floating medium hexagons, and tiny delicate micro-hexagon accents, large angled diagonal thick rounded stripes across, smooth gradient color overlays, clean blank copy space, sharp crisp vector edges, flat graphic design, continuous panoramic flow",
        description: "AI-generated modern geometric hexagons and diagonal stripes under the house typography; the accent band follows the chosen palette.",
        palettes: AI_PALETTES,
    },
    AiPreset {
        name: "Minimal Sci-Fi",
        style_words: "minimal retro sci-fi background, geometric horizon lines, massive glowing sun disc in sky, faint starfield, clean flat vector shapes, wide panorama",
        description: "AI-generated retro-futurist geometry under the house typography; the accent band follows the chosen palette.",
        palettes: AI_PALETTES,
    },
    AiPreset {
        name: "Dynamic Waves Spread",
        style_words: "flat vector 2D graphic design, dramatic scale contrast with radical size differences between shapes, one massive broad sweeping corporate curved color panel dominating the spread, paired with slim secondary accent ribbons and small delicate floating geometric accent marks, sharp crisp vector edges, expansive clean blank copy space, continuous panoramic flow",
        description: "AI-generated double A4 spread of corporate waves with central spine; front cover on right, back cover on left.",
        palettes: AI_PALETTES,
    },
    AiPreset {
        name: "Vibrant S-Curve Spread",
        style_words: "flat vector 2D graphic design, extreme scale contrast and radical shape size hierarchy, one gigantic broad sweeping S-curve ribbon band flowing across the spread, contrasted with narrow sharp accent ribbons and small minimalist geometric accents, smooth gradient overlays, sharp clean vector contour edges, generous blank negative copy space, continuous panoramic flow",
        description: "AI-generated double A4 spread featuring vibrant S-curve ribbons and geometric accent fields across front and back covers.",
        palettes: AI_PALETTES,
    },
];

/// The AI style a `cover_style` id selects (`100 + i` → `AI_PRESETS[i]`).
pub fn preset_index(cover_style: usize) -> Option<usize> {
    cover_style.checked_sub(AI_STYLE_BASE).filter(|i| *i < AI_PRESETS.len())
}

pub fn preset(cover_style: usize) -> Option<&'static AiPreset> {
    preset_index(cover_style).and_then(|i| AI_PRESETS.get(i))
}

/// The names the style ComboBox and the Excel COVER sheet use ("AI Retro Waves"…).
pub fn style_name(cover_style: usize) -> Option<String> {
    preset(cover_style).map(|p| format!("AI {}", p.name))
}

/// The full prompt sent for a preset + palette: the flux recipe validated in
/// `ai_cover_generation/` — pure style words, palette words, and the flat-vector
/// cover wrapper. No SDXL dimension tail (flux has a fixed square output, a
/// strict two-field schema, and no reaction to the mockup-trigger words that
/// wording had to avoid with SDXL).
pub fn build_prompt(preset: &AiPreset, palette: &AiPalette) -> String {
    format!("{}, {}, minimalist 2d vector cover design, sleek layout, flat vector art, no text", preset.style_words, palette.words)
}

// ---------------------------------------------------------------------------
// Setup dialog (mirrors ai::AiSetupDialog: prefill from prefs, prove with one tiny
// real request, persist through prefs.set)
// ---------------------------------------------------------------------------

/// The "AI Base Cover" setup modal owned by the panel.
#[derive(Clone, Debug)]
pub struct AiCoverSetupDialog {
    pub account_id: String,
    pub token: String,
    pub error: Option<String>,
    /// Set by a successful verify: the dialog closes and stays closed.
    pub close_requested: bool,
}

/// Helper to read Cloudflare credentials from engine Prefs, falling back to process env
/// or a local `.env` file.
pub fn get_credentials(prefs: &designcraft_engine::Prefs) -> (String, String) {
    let mut acct = prefs.cover_ai_cloudflare_account_id.trim().to_string();
    let mut tok = prefs.cover_ai_cloudflare_api_token.trim().to_string();

    if acct.is_empty()
        && let Ok(v) = std::env::var("CLOUDFLARE_ACCOUNT_ID")
    {
        acct = v.trim().to_string();
    }
    if tok.is_empty()
        && let Ok(v) = std::env::var("CLOUDFLARE_API_TOKEN")
    {
        tok = v.trim().to_string();
    }

    if (acct.is_empty() || tok.is_empty())
        && let Ok(content) = std::fs::read_to_string(".env")
    {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim();
                let v = v.trim().trim_matches('"').trim_matches('\'').trim();
                if acct.is_empty() && (k.eq_ignore_ascii_case("CLOUDFLARE_ACCOUNT_ID") || k.eq_ignore_ascii_case("CLAUFFLARE_ACCOUNT_ID")) {
                    acct = v.to_string();
                } else if tok.is_empty() && (k.eq_ignore_ascii_case("CLOUDFLARE_API_TOKEN") || k.eq_ignore_ascii_case("CLAUFFLARE_API_TOKEN")) {
                    tok = v.to_string();
                }
            }
        }
    }

    (acct, tok)
}

impl AiCoverSetupDialog {
    pub fn open(app: &DesignApp) -> Self {
        let (account_id, token) = get_credentials(&app.session.prefs);
        Self { account_id, token, error: None, close_requested: false }
    }

    /// Is a usable-looking key configured? (Checked without the network; the real
    /// proof is the Verify call.)
    pub fn key_configured(app: &DesignApp) -> bool {
        let (acct, tok) = get_credentials(&app.session.prefs);
        !acct.is_empty() && !tok.is_empty()
    }

    /// Prove the credentials with one tiny real generation (256 px, one step: a few
    /// Neurons), then persist through prefs.set. Returns false (with self.error set)
    /// when the check fails — the dialog stays open so the user can fix the setup.
    pub fn verify_and_accept(&mut self, app: &mut DesignApp) -> bool {
        self.error = None;
        if !imagegen::compiled_in() {
            self.error = Some("This build has no online image generation (compile with --features ai-cover-online).".into());
            return false;
        }
        let client = imagegen::cloudflare::CloudflareClient::new(&self.account_id, &self.token);
        let req = imagegen::GenRequest::new("flat grey rectangle", "", imagegen::Aspect::Iso, 1);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        match client.generate(&req, &cancel) {
            Ok(_) => {
                let _ = app.run(
                    "prefs.set",
                    json!({
                        "coverAiCloudflareAccountId": self.account_id.trim(),
                        "coverAiCloudflareApiToken": self.token.trim(),
                    }),
                );
                self.close_requested = true;
                true
            }
            Err(e) => {
                self.error = Some(match e {
                    imagegen::ImageGenError::Auth => {
                        "Cloudflare rejected the credentials — check the account id and the token's Workers AI Edit permission.".into()
                    }
                    other => format!("Setup check failed: {other} — fix the account id or token and try again."),
                });
                false
            }
        }
    }
}

/// The AI options shown under the style picker when an AI Base style is selected:
/// the palette direction (which also recolors the cover accents) and the key setup.
pub(crate) fn ai_cover_options_pane(app: &mut DesignApp, ui: &mut egui::Ui, st: &mut PubgenState, t: &Tokens) {
    let Some(preset) = preset(st.cover_style) else { return };

    ui.horizontal(|ui| {
        ui.label(RichText::new("AI Palette:").size(10.5).color(t.text));
        let palettes = preset.palettes;
        let cur = palettes.get(st.cover_ai_palette).map(|p| p.name).unwrap_or(palettes[0].name);
        let prev = st.cover_ai_palette;
        craft_ui::ComboBox::from_id_salt("pubgen_cover_ai_palette").selected_text(cur).width(150.0).show_ui(ui, |ui| {
            for (idx, p) in palettes.iter().enumerate() {
                ui.selectable_value(&mut st.cover_ai_palette, idx, p.name);
            }
        });
        if st.cover_ai_palette != prev
            && let Some(p) = palettes.get(st.cover_ai_palette)
        {
            st.cover_color_primary = p.primary.into();
            st.cover_color_accent = p.accent.into();
            st.cover_color_dark = p.dark.into();
        }
    });
    if let Some(p) = palettes_of(st.cover_style, st.cover_ai_palette) {
        ui.add(egui::Label::new(RichText::new(format!("Colours: {}", p.words)).size(9.5).color(t.text_dim)).wrap());
    }

    // Key setup: the button mirrors the AI Translator one; Verify runs one tiny generation.
    let ready = AiCoverSetupDialog::key_configured(app);
    let (label, tip) = if !imagegen::compiled_in() {
        ("AI Offline", "This build has no online image generation (compile with --features ai-cover-online)")
    } else if ready {
        ("AI Ready \u{2713}", "Cloudflare credentials configured — click to review")
    } else {
        ("AI Setup…", "Connect a Cloudflare account (free tier) to generate AI backgrounds")
    };
    ui.horizontal(|ui| {
        if ui.button(label).on_hover_text(tip).clicked() {
            st.cover_ai_dialog = Some(AiCoverSetupDialog::open(app));
        }
        if let Some(err) = &st.cover_ai_error {
            ui.add(egui::Label::new(RichText::new(err).size(10.0).color(egui::Color32::from_rgb(220, 80, 80))).wrap());
        }
    });

    if let Some(dialog) = &mut st.cover_ai_dialog {
        let mut close = false;
        egui::Window::new("AI Base Cover Setup").collapsible(false).resizable(false).show(ui.ctx(), |ui| {
            ui.label("Cloudflare Workers AI (free tier: ~45 images/day).");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Account ID:");
                ui.add(egui::TextEdit::singleline(&mut dialog.account_id).desired_width(220.0));
            });
            ui.horizontal(|ui| {
                ui.label("API Token:");
                ui.add(egui::TextEdit::singleline(&mut dialog.token).password(true).hint_text("Workers AI · Edit").desired_width(220.0));
            });
            ui.hyperlink_to("Create a token", "https://dash.cloudflare.com/profile/api-tokens");
            ui.label(RichText::new("Stored locally in DesignCraft settings.").size(10.0));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Verify & Use").clicked() {
                    // The synchronous proof takes a few seconds (one tiny generation).
                    dialog.verify_and_accept(app);
                    close = dialog.close_requested;
                    if let Some(err) = &dialog.error {
                        st.cover_ai_error = Some(err.clone());
                    } else {
                        st.cover_ai_error = None;
                    }
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                if let Some(err) = &dialog.error {
                    ui.add(egui::Label::new(RichText::new(err).size(10.0).color(egui::Color32::from_rgb(220, 80, 80))).wrap());
                }
            });
        });
        if close {
            st.cover_ai_dialog = None;
        }
    }
}

fn palettes_of(cover_style: usize, palette_idx: usize) -> Option<&'static AiPalette> {
    preset(cover_style).and_then(|p| p.palettes.get(palette_idx))
}

/// The Generate Cover Page entry for AI styles.
pub(crate) fn execute_generate_ai_cover(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    match preset(st.cover_style) {
        Some(p) => {
            if !imagegen::compiled_in() {
                return Err("AI Base covers need a build with --features ai-cover-online.".into());
            }
            if !AiCoverSetupDialog::key_configured(app) {
                return Err("AI Base needs a Cloudflare account: pick AI Setup\u{2026} under Cover Images and verify your key.".into());
            }
            let candidate = format!("outputs/ai_covers/ai_cover_{}_{}.png", st.cover_style, st.cover_ai_palette);
            if std::path::Path::new(&candidate).is_file() {
                return super::covers::apply_ai_cover_result(app, st, &candidate);
            }
            Err(format!("AI Base style 'AI {}' generates in background via CoverJob.", p.name))
        }
        None => Err(format!("Unsupported cover style: {}", st.cover_style)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_unique_names_and_full_palettes() {
        assert_eq!(AI_PRESETS.len(), 5);
        for (i, p) in AI_PRESETS.iter().enumerate() {
            assert!(!p.name.is_empty());
            assert_eq!(AI_PRESETS.iter().position(|q| q.name == p.name), Some(i), "duplicate preset name {}", p.name);
            assert_eq!(p.palettes.len(), 6, "{} needs the shared palettes", p.name);
            for pal in p.palettes {
                for hex in [pal.primary, pal.accent, pal.dark] {
                    assert!(designcraft_color::Color::from_hex(hex).is_some(), "{}: bad hex {hex}", pal.name);
                }
            }
            assert_eq!(preset_index(AI_STYLE_BASE + i), Some(i));
            assert_eq!(style_name(AI_STYLE_BASE + i), Some(format!("AI {}", p.name)));
        }
        assert_eq!(preset_index(AI_STYLE_BASE + AI_PRESETS.len()), None, "past the table: not an AI style");
        assert_eq!(preset_index(0), None, "procedural styles are not AI styles");
    }

    #[test]
    fn prompt_carries_style_palette_and_flux_wrapper() {
        let prompt = build_prompt(&AI_PRESETS[0], &AI_PALETTES[0]);
        assert!(prompt.contains("mid-century modern"), "{prompt}");
        assert!(prompt.contains("terracotta red"), "{prompt}");
        assert!(prompt.contains("minimalist 2d vector cover design"), "{prompt}");
        assert!(prompt.contains("sleek layout"), "{prompt}");
        assert!(prompt.contains("flat vector art"), "{prompt}");
        assert!(prompt.contains("no text"), "{prompt}");
        // No SDXL dimension tail: flux has a fixed square output and ignores it.
        assert!(!prompt.contains("1216") && !prompt.contains("832"), "{prompt}");
        assert!(!prompt.contains("3:2"), "{prompt}");
        assert!(!prompt.contains("booklet"), "{prompt}");
        assert!(!prompt.contains("template"), "{prompt}");
        assert!(!prompt.contains("front cover"), "{prompt}");
        assert!(!prompt.contains("back cover"), "{prompt}");
    }

    #[test]
    fn geometric_business_prompt_adapts_to_palette_without_mockup_or_checkerboard() {
        let p_geo = &AI_PRESETS[1];
        assert_eq!(p_geo.name, "Geometric Business");
        let pal_teal = &AI_PALETTES[5];
        assert_eq!(pal_teal.name, "Teal Lime");
        let prompt = build_prompt(p_geo, pal_teal);
        assert!(prompt.contains("hero hexagon frame"), "{prompt}");
        assert!(prompt.contains("floating medium hexagons"), "{prompt}");
        assert!(prompt.contains("deep teal green"), "{prompt}");
        assert!(prompt.contains("chartreuse lime green"), "{prompt}");
        assert!(!prompt.contains("checkerboard"), "{prompt}");
        assert!(!prompt.contains("placeholder"), "{prompt}");
        assert!(!prompt.contains("template"), "{prompt}");
        assert!(!prompt.contains("mockup"), "{prompt}");

        // Verifies color adaptability: with Corporate Blue it has blue words, no teal
        let pal_corp = &AI_PALETTES[4];
        let prompt_corp = build_prompt(p_geo, pal_corp);
        assert!(prompt_corp.contains("geometric business"), "{prompt_corp}");
        assert!(prompt_corp.contains("navy blue"), "{prompt_corp}");
        assert!(!prompt_corp.contains("deep teal green"), "{prompt_corp}");
    }

    /// Live validation of every AI cover style against Cloudflare Workers AI:
    /// replays each preset × palette through the exact production request path
    /// (`build_prompt` + `Aspect::Wrap` + 18 steps, like `trigger_generate_cover`)
    /// and saves into `ai_cover_generation/` the wire request (token redacted),
    /// the raw provider JPEG and the cropped pipeline PNG for every combination.
    #[cfg(feature = "ai-cover-online")]
    mod live_all_styles {
        use super::*;
        use designcraft_imagegen as imagegen;
        use std::sync::atomic::AtomicBool;

        fn slug(s: &str) -> String {
            s.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect()
        }

        fn credentials(repo: &std::path::Path) -> (String, String) {
            let mut acct = std::env::var("CLOUDFLARE_ACCOUNT_ID").unwrap_or_default().trim().to_string();
            let mut tok = std::env::var("CLOUDFLARE_API_TOKEN").unwrap_or_default().trim().to_string();
            if acct.is_empty() || tok.is_empty() {
                if let Ok(content) = std::fs::read_to_string(repo.join(".env")) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.starts_with('#') || line.is_empty() {
                            continue;
                        }
                        if let Some((k, v)) = line.split_once('=') {
                            let v = v.trim().trim_matches('"').trim_matches('\'').trim().to_string();
                            if acct.is_empty() && k.trim().eq_ignore_ascii_case("CLOUDFLARE_ACCOUNT_ID") {
                                acct = v;
                            } else if tok.is_empty() && k.trim().eq_ignore_ascii_case("CLOUDFLARE_API_TOKEN") {
                                tok = v;
                            }
                        }
                    }
                }
            }
            (acct, tok)
        }

        #[test]
        #[ignore = "live Cloudflare run: spends free-tier Neurons and writes ai_cover_generation/ (AICOVER_COMBOS=101:3,104:1 retries a subset)"]
        fn live_generate_every_ai_style_and_palette() {
            let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let (acct, tok) = credentials(&repo);
            assert!(!acct.is_empty(), "CLOUDFLARE_ACCOUNT_ID not found (env or {}/.env)", repo.display());
            assert!(!tok.is_empty(), "CLOUDFLARE_API_TOKEN not found (env or {}/.env)", repo.display());

            let only: Vec<(usize, usize)> = std::env::var("AICOVER_COMBOS")
                .ok()
                .map(|spec| {
                    spec.split(',')
                        .filter_map(|pair| pair.trim().split_once(':'))
                        .filter_map(|(s, p)| Some((s.trim().parse().ok()?, p.trim().parse().ok()?)))
                        .collect()
                })
                .unwrap_or_default();

            let out_dir = repo.join("ai_cover_generation");
            let req_dir = out_dir.join("requests");
            std::fs::create_dir_all(&req_dir).expect("create ai_cover_generation/requests");

            let client = imagegen::cloudflare::CloudflareClient::new(&acct, &tok);
            let cancel = AtomicBool::new(false);
            let title = "Analisis Ketenagakerjaan Kabupaten Karawang 2026";
            let seed = super::super::super::ai_cover_job::seed_from_title(title);
            let neg = imagegen::GenRequest::clean_cover_negative_prompt();

            let mut manifest: Vec<serde_json::Value> = Vec::new();
            let mut quota_hit = false;
            for (i, preset) in AI_PRESETS.iter().enumerate() {
                for (j, pal) in preset.palettes.iter().enumerate() {
                    let style_id = AI_STYLE_BASE + i;
                    if !only.is_empty() && !only.contains(&(style_id, j)) {
                        continue;
                    }
                    let prompt = build_prompt(preset, pal);
                    let req = imagegen::GenRequest::new(prompt.clone(), neg.clone(), imagegen::Aspect::Wrap, 18).with_seed(seed);
                    let stem = format!("s{}_p{}__{}__{}", style_id, j, slug(preset.name), slug(pal.name));

                    let request_json = json!({
                        "style_id": style_id,
                        "preset": preset.name,
                        "palette": pal.name,
                        "palette_colors": {"primary": pal.primary, "accent": pal.accent, "dark": pal.dark},
                        "method": "POST",
                        "url": "https://api.cloudflare.com/client/v4/accounts/<account_id>/ai/run/@cf/black-forest-labs/flux-1-schnell",
                        "headers": {"Content-Type": "application/json", "Authorization": "Bearer <redacted>"},
                        "body": serde_json::from_str::<serde_json::Value>(&imagegen::cloudflare::request_body_for(&req)).expect("wire body is JSON"),
                        "local_only_never_sent": {
                            "negative_prompt": neg,
                            "seed_from_title": seed,
                            "seed_title": title,
                            "steps_requested": req.num_steps,
                            "target_bucket": [req.width, req.height],
                            "note": "flux-1-schnell rejects width/height/seed/negative_prompt (error 5006); it always returns a 1024x1024 JPEG which the client center-crops to the target bucket"
                        }
                    });
                    std::fs::write(
                        req_dir.join(format!("{stem}_request.json")),
                        serde_json::to_string_pretty(&request_json).expect("serialize request"),
                    )
                    .expect("write request json");

                    let entry = match client.generate_with_raw(&req, &cancel) {
                        Ok(out) => {
                            let png_path = out_dir.join(format!("{stem}.png"));
                            let raw_path = out_dir.join(format!("{stem}_raw.jpg"));
                            std::fs::write(&png_path, &out.png).expect("write png");
                            std::fs::write(&raw_path, &out.raw).expect("write raw jpeg");
                            println!("ok   {stem}: {} raw JPEG bytes, {} bucket PNG bytes", out.raw.len(), out.png.len());
                            json!({
                                "style_id": style_id, "preset": preset.name, "palette": pal.name, "ok": true,
                                "prompt": prompt, "steps_sent": 8,
                                "raw": raw_path.file_name().map(|s| s.to_string_lossy()).unwrap_or_default(),
                                "png": png_path.file_name().map(|s| s.to_string_lossy()).unwrap_or_default(),
                            })
                        }
                        Err(imagegen::ImageGenError::Auth) => panic!("Cloudflare rejected the credentials"),
                        Err(e) => {
                            println!("fail {stem}: {e}");
                            if matches!(e, imagegen::ImageGenError::Quota(_)) {
                                quota_hit = true;
                            }
                            json!({
                                "style_id": style_id, "preset": preset.name, "palette": pal.name, "ok": false,
                                "prompt": prompt, "error": e.to_string(),
                            })
                        }
                    };
                    manifest.push(entry);
                    if quota_hit {
                        break;
                    }
                }
                if quota_hit {
                    break;
                }
            }

            let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            // A filtered retry merges into an existing manifest so the folder keeps the full picture.
            let key = |e: &serde_json::Value| {
                format!("{}|{}", e.get("preset").and_then(|v| v.as_str()).unwrap_or(""), e.get("palette").and_then(|v| v.as_str()).unwrap_or(""))
            };
            let mut merged: std::collections::BTreeMap<String, serde_json::Value> = std::fs::read_to_string(out_dir.join("manifest.json"))
                .ok()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
                .and_then(|doc| doc.get("results").and_then(|r| r.as_array()).cloned())
                .map(|results| results.into_iter().map(|e| (key(&e), e)).collect())
                .unwrap_or_default();
            for e in &manifest {
                merged.insert(key(e), e.clone());
            }
            let mut results: Vec<serde_json::Value> = merged.into_values().collect();
            results.sort_by_key(|e| {
                (e.get("style_id").and_then(|v| v.as_u64()).unwrap_or(0), e.get("palette").and_then(|v| v.as_str()).unwrap_or("").to_string())
            });
            let ok = results.iter().filter(|e| e.get("ok").and_then(|v| v.as_bool()) == Some(true)).count();
            let doc = json!({
                "model": "@cf/black-forest-labs/flux-1-schnell",
                "endpoint": "POST /client/v4/accounts/{account_id}/ai/run/@cf/black-forest-labs/flux-1-schnell",
                "generated_unix": unix,
                "combos": results.len(),
                "succeeded": ok,
                "request_contract": {"prompt": "<style words>, <palette words>, minimalist 2d vector cover design, sleek layout, flat vector art, no text", "steps": 8},
                "results": results,
            });
            std::fs::write(out_dir.join("manifest.json"), serde_json::to_string_pretty(&doc).expect("serialize manifest")).expect("write manifest");
            assert!(ok > 0, "no combination generated successfully — see ai_cover_generation/manifest.json");
            println!("done: {ok}/{} combinations in ai_cover_generation/", doc["combos"]);
        }
    }
}
