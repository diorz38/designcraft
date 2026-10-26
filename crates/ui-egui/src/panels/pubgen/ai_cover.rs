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
        style_words: "flat vector 2D modern geometric business wallpaper, large angled diagonal thick rounded stripes across, floating rounded hexagons, geometric hexagon accent frames, smooth gradient color overlays, clean blank copy space, sharp crisp vector edges, flat graphic design, continuous panoramic flow",
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
        style_words: "abstract flowing dynamic wave ribbons, smooth curved gradient shapes, clean solid background, flat 2D graphic design, minimalist corporate aesthetic, high resolution vector illustration, continuous flowing curves",
        description: "AI-generated double A4 spread of corporate waves with central spine; front cover on right, back cover on left.",
        palettes: AI_PALETTES,
    },
    AiPreset {
        name: "Vibrant S-Curve Spread",
        style_words: "flat vector graphic, sweeping curved bands, vibrant wavy gradient ribbon swooping across the composition in an elegant S-curve, geometric accent fields with fine diagonal texture, 2D graphic design, smooth gradients, sharp clean edges, continuous panoramic flow",
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

/// The full prompt sent for a preset + palette: pure style words, palette words,
/// native SDXL 3:2 landscape dimensions (1216x832), and full-bleed panoramic layout.
/// Avoids mockup triggers (booklet, cover, template, bifold, front/back cover).
pub fn build_prompt(preset: &AiPreset, palette: &AiPalette) -> String {
    format!(
        "{}, {}, width 1216 height 832, aspect ratio 3:2 landscape, wide panoramic wallpaper, full bleed edge to edge, flat 2D vector graphic, continuous seamless artwork, generous blank copy space for typography, high resolution",
        preset.style_words, palette.words
    )
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
        egui::ComboBox::from_id_salt("pubgen_cover_ai_palette").selected_text(cur).width(150.0).show_ui(ui, |ui| {
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
    fn prompt_carries_style_palette_and_layout_constraints() {
        let prompt = build_prompt(&AI_PRESETS[0], &AI_PALETTES[0]);
        assert!(prompt.contains("mid-century modern"), "{prompt}");
        assert!(prompt.contains("terracotta red"), "{prompt}");
        assert!(prompt.contains("copy space"), "{prompt}");
        assert!(prompt.contains("1216") && prompt.contains("832"), "{prompt}");
        assert!(prompt.contains("3:2"), "{prompt}");
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
        assert!(prompt.contains("geometric business"), "{prompt}");
        assert!(prompt.contains("floating rounded hexagons"), "{prompt}");
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
}
