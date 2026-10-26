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
];

pub const AI_PRESETS: &[AiPreset] = &[
    AiPreset {
        name: "Retro Waves",
        style_words: "minimalist mid-century modern abstract background, flowing organic waves and curving colour blocks, 1970s retro aesthetic, smooth flat vector shapes",
        description: "AI-generated mid-century waves and colour blocks under the house typography; the accent band follows the chosen palette.",
        palettes: AI_PALETTES,
    },
    AiPreset {
        name: "Dark Fantasy",
        style_words: "dark fantasy atmosphere, misty mountain silhouettes and ancient towers, dramatic moody clouds, painterly matte texture",
        description: "AI-generated misty fantasy landscape under the house typography; the accent band follows the chosen palette.",
        palettes: AI_PALETTES,
    },
    AiPreset {
        name: "Minimal Sci-Fi",
        style_words: "minimal retro sci-fi background, geometric horizon lines, large sun disc and faint starfield, clean flat vector shapes",
        description: "AI-generated retro-futurist geometry under the house typography; the accent band follows the chosen palette.",
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

/// The full prompt sent for a preset + palette: style words, palette words and the
/// layout constraints (negative prompt lives in `imagegen::GenRequest`).
pub fn build_prompt(preset: &AiPreset, palette: &AiPalette) -> String {
    format!(
        "{}, {}, book cover background art, vertical composition, generous empty negative space \
         at the top for typography, clean flat vector illustration, high dynamic range",
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

impl AiCoverSetupDialog {
    pub fn open(app: &DesignApp) -> Self {
        let prefs = &app.session.prefs;
        Self {
            account_id: prefs.cover_ai_cloudflare_account_id.clone(),
            token: prefs.cover_ai_cloudflare_api_token.clone(),
            error: None,
            close_requested: false,
        }
    }

    /// Is a usable-looking key configured? (Checked without the network; the real
    /// proof is the Verify call.)
    pub fn key_configured(app: &DesignApp) -> bool {
        let prefs = &app.session.prefs;
        !prefs.cover_ai_cloudflare_account_id.trim().is_empty() && !prefs.cover_ai_cloudflare_api_token.trim().is_empty()
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

/// The Generate Cover Page entry for AI styles. Phase 4 replaces this with the
/// background job; until then it fails with a clear, actionable message instead of
/// the procedural paths' "Unsupported cover style".
pub(crate) fn execute_generate_ai_cover(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    match preset(st.cover_style) {
        Some(_) if !imagegen::compiled_in() => Err("AI Base covers need a build with --features ai-cover-online.".into()),
        Some(_) if !AiCoverSetupDialog::key_configured(app) => {
            Err("AI Base needs a Cloudflare account: pick AI Setup\u{2026} under Cover Images and verify your key.".into())
        }
        Some(_) => Err("AI Base generation is not wired in this build yet \u{2014} the online background job lands next.".into()),
        None => Err(format!("Unsupported cover style: {}", st.cover_style)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_unique_names_and_full_palettes() {
        assert_eq!(AI_PRESETS.len(), 3);
        for (i, p) in AI_PRESETS.iter().enumerate() {
            assert!(!p.name.is_empty());
            assert_eq!(AI_PRESETS.iter().position(|q| q.name == p.name), Some(i), "duplicate preset name {}", p.name);
            assert_eq!(p.palettes.len(), 4, "{} needs the shared four palettes", p.name);
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
        assert!(prompt.contains("negative space"), "{prompt}");
        assert!(!prompt.contains("text, "), "{prompt}");
    }
}
