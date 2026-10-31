//! Style thumbnail textures: painted fallback cover previews and hover tooltips.

use super::*;

pub(crate) fn adjust_hex(hex: &str, factor: f32) -> String {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() != 6 {
        return hex.to_string();
    }
    let r = u8::from_str_radix(clean.get(0..2).unwrap_or("00"), 16).unwrap_or(128);
    let g = u8::from_str_radix(clean.get(2..4).unwrap_or("00"), 16).unwrap_or(128);
    let b = u8::from_str_radix(clean.get(4..6).unwrap_or("00"), 16).unwrap_or(128);
    let clamp = |c: f32| c.clamp(0.0, 255.0) as u8;
    let r2 = clamp(r as f32 * factor);
    let g2 = clamp(g as f32 * factor);
    let b2 = clamp(b as f32 * factor);
    format!("#{r2:02x}{g2:02x}{b2:02x}")
}

/// Image files a style thumbnail may use.
const THUMB_IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

/// The folder a user's own thumbnails persist in, so replacing a PNG survives restarts and
/// updates: override it with `DESIGNCRAFT_PUBGEN_THUMBS`, otherwise it is the per-user data
/// folder (`%APPDATA%\DesignCraft\PubgenThumbs`, `~/Library/Application Support/...` on macOS,
/// `$XDG_DATA_HOME` on Linux).
pub(crate) fn user_thumbs_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library/Application Support/DesignCraft/PubgenThumbs"));
    }
    if cfg!(target_os = "windows") {
        return std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .or_else(|| home.map(|h| h.join("AppData/Roaming")))
            .map(|a| a.join("DesignCraft").join("PubgenThumbs"));
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| home.map(|h| h.join(".local/share")))
        .map(|d| d.join("designcraft/pubgen-thumbs"))
}

/// The folders scanned for thumbnails, highest priority first; the first folder holding a style's
/// image wins. Relative sample folders are anchored to the crate too, so the app finds them from
/// any working directory.
fn thumb_dirs() -> Vec<std::path::PathBuf> {
    let legacy = ["plan/pubgen/samples/thumb", "samples/thumb", "outputs"];
    let mut dirs = Vec::new();
    if let Some(d) = std::env::var_os("DESIGNCRAFT_PUBGEN_THUMBS") {
        dirs.push(std::path::PathBuf::from(d));
    }
    if let Some(dir) = user_thumbs_dir() {
        // Made once so the folder is there to drop a replacement into.
        static READY: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        READY.get_or_init(|| {
            let _ = std::fs::create_dir_all(&dir);
        });
        dirs.push(dir);
    }
    dirs.extend(legacy.iter().map(std::path::PathBuf::from));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    dirs.extend(legacy.iter().map(|rel| root.join(rel)));
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        dirs.extend(legacy.iter().map(|rel| dir.join(rel)));
        dirs.push(dir.join("pubgen/thumbs"));
    }
    dirs
}

/// Lower-case letters and digits only, so `Wave_Curves`, `wave curves` and `wave-curves` agree.
fn slug(name: &str) -> String {
    name.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

/// The built-in styles' display labels, which are also alias file names. The compiler that holds
/// them is native-only; on the web there are no files to name, so the list is empty.
#[cfg(not(target_arch = "wasm32"))]
fn style_labels() -> &'static [&'static str] {
    super::compile::COVER_STYLES
}
#[cfg(target_arch = "wasm32")]
fn style_labels() -> &'static [&'static str] {
    &[]
}

/// A cover style the app can show: a built-in style (every index below the AI base) or an AI preset.
fn ai_style_indices() -> std::ops::Range<usize> {
    super::ai_cover::AI_STYLE_BASE..super::ai_cover::AI_STYLE_BASE + super::ai_cover::AI_PRESETS.len()
}

fn valid_cover_style(style: usize) -> bool {
    style < super::ai_cover::AI_STYLE_BASE || ai_style_indices().contains(&style)
}

fn cover_style_indices() -> impl Iterator<Item = usize> {
    (0..super::ai_cover::AI_STYLE_BASE).chain(ai_style_indices())
}

/// The names a style's thumbnail may be filed under: the sample names it shipped with, plus its
/// `COVER_STYLES` label.
fn style_aliases(style: usize) -> impl Iterator<Item = &'static str> {
    let samples: &[&'static str] = match style {
        0 => &["wave_curves", "karawang_timur_cover_2026"],
        1 => &["geometric_medallion", "karawang_timur_geometric_cover"],
        2 => &["folded_origami", "karawang_timur_origami_cover"],
        3 => &["chevron_medallion", "karawang_timur_chevron_cover"],
        4 => &["karawang_timur_hexagon_cover", "honeycomb_hexagon"],
        5 => &["corporate_headline", "karawang_timur_corporate_cover"],
        6 => &["diagonal_arrow", "karawang_timur_diagonal_cover"],
        7 => &["modern_horizon", "karawang_timur_modern_horizon_cover"],
        8 => &["business_report", "karawang_timur_business_cover"],
        s if s >= super::ai_cover::AI_STYLE_BASE => &["ai_base", "ai_base_template"],
        _ => &[],
    };
    samples.iter().copied().chain(style_labels().get(style).copied())
}

/// The style a thumbnail file name shows, and how strongly: a numeric prefix (`0_`, `style0`) is
/// explicit, then an exact alias, then the longest alias the name starts with or contains.
fn style_thumb_match(name: &str) -> Option<(usize, u32)> {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let s = slug(stem);
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    let prefix_style = digits.parse::<usize>().ok().filter(|_| stem.chars().nth(digits.len()).is_some_and(|c| !c.is_alphanumeric()));
    if let Some(style) = prefix_style.filter(|s| valid_cover_style(*s)) {
        return Some((style, 1000));
    }
    if let Some(style) = s.strip_prefix("style").and_then(|rest| rest.parse::<usize>().ok()).filter(|s| valid_cover_style(*s)) {
        return Some((style, 900));
    }
    let mut best: Option<(usize, u32)> = None;
    for style in cover_style_indices() {
        for alias in style_aliases(style) {
            let a = slug(alias);
            let rank = if a.is_empty() {
                0
            } else if s == a {
                300
            } else if s.starts_with(&a) {
                200
            } else if s.contains(&a) {
                100
            } else {
                0
            };
            let score = rank + a.len().min(99) as u32;
            if rank > 0 && best.is_none_or(|(_, best_score)| score > best_score) {
                best = Some((style, score));
            }
        }
    }
    best
}

fn is_thumb_image(path: &std::path::Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| THUMB_IMAGE_EXTS.iter().any(|ext| e.eq_ignore_ascii_case(ext)))
}

/// The thumbnail file for a style: the first folder with a match wins, and inside it the strongest
/// match, so dropping `3_my_cover.png` next to `chevron_medallion.png` overrides the sample.
fn thumb_path(style_idx: usize) -> Option<std::path::PathBuf> {
    thumb_path_in(&thumb_dirs(), style_idx)
}

fn thumb_path_in(dirs: &[std::path::PathBuf], style_idx: usize) -> Option<std::path::PathBuf> {
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        let mut files: Vec<std::path::PathBuf> = entries.flatten().map(|entry| entry.path()).filter(|path| is_thumb_image(path)).collect();
        files.sort();
        let mut best: Option<(u32, std::path::PathBuf)> = None;
        for file in files {
            let Some(name) = file.file_name().and_then(|name| name.to_str()) else { continue };
            let Some((style, score)) = style_thumb_match(name) else { continue };
            if style == style_idx && best.as_ref().is_none_or(|(best_score, _)| score > *best_score) {
                best = Some((score, file));
            }
        }
        if let Some((_, path)) = best {
            return Some(path);
        }
    }
    None
}

fn style_thumbnail_texture(ctx: &egui::Context, style_idx: usize) -> Option<egui::TextureHandle> {
    // Keyed by the file's identity as well as the style: a replaced or regenerated PNG reloads on
    // the next hover, an unchanged one stays cached.
    let path = thumb_path(style_idx)?;
    let meta = std::fs::metadata(&path).ok()?;
    let key = egui::Id::unique(("pubgen_cover_style_thumbnail_tex", style_idx, path.clone(), meta.len(), meta.modified().ok()));
    if let Some(tex) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return Some(tex);
    }
    let bytes = std::fs::read(&path).ok()?;
    let pm = designcraft_render::decode_pixmap_page(&bytes, 0)?;
    let ci = egui::ColorImage::from_rgba_premultiplied([pm.width() as usize, pm.height() as usize], pm.data_as_u8_slice());
    let fitted = crate::widgets::fit_texture(ci, 300);
    let tex = ctx.load_texture(format!("pubgen_cover_thumb_{style_idx}"), fitted, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(key, tex.clone()));
    Some(tex)
}

fn paint_fallback_cover_thumb(ui: &mut Ui, style_idx: usize, w: f32, h: f32, t: &Tokens) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 4.0, Color32::from_rgb(250, 250, 252));
    p.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Outside);

    match style_idx {
        0 => {
            let bottom_rect = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.45), rect.max);
            p.rect_filled(bottom_rect, egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, Color32::from_rgb(18, 38, 71));
            let center = rect.center() + vec2(0.0, -h * 0.02);
            p.circle_filled(center, w * 0.23, Color32::from_rgb(0, 114, 188));
            p.circle_stroke(center, w * 0.25, egui::Stroke::new(2.0, Color32::from_rgb(245, 171, 25)));
        }
        1 => {
            let band = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.28), rect.min + vec2(w, h * 0.72));
            p.rect_filled(band, 0.0, Color32::from_rgb(0, 114, 188));
            let c = rect.center() + vec2(w * 0.1, h * 0.08);
            p.circle_filled(c, w * 0.22, Color32::WHITE);
            p.circle_stroke(c, w * 0.22, egui::Stroke::new(2.0, Color32::from_rgb(245, 171, 25)));
        }
        2 => {
            let top_right = egui::Rect::from_min_max(rect.min + vec2(w * 0.4, 0.0), rect.min + vec2(w, h * 0.38));
            p.rect_filled(top_right, egui::CornerRadius { nw: 0, ne: 4, sw: 0, se: 0 }, Color32::from_rgb(100, 150, 200));
            let gold_facet = egui::Rect::from_min_max(rect.min + vec2(0.0, 0.0), rect.min + vec2(w * 0.4, h * 0.2));
            p.rect_filled(gold_facet, egui::CornerRadius { nw: 4, ne: 0, sw: 0, se: 0 }, Color32::from_rgb(245, 171, 25));
            let dark_facet = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.2), rect.min + vec2(w * 0.5, h * 0.45));
            p.rect_filled(dark_facet, 0.0, Color32::from_rgb(18, 38, 71));
        }
        3 => {
            // Style 3 (Chevron & Medallion): Downward chevron + horseshoe ring + footer + center year
            let chevron_pts = vec![
                rect.min,
                rect.min + vec2(w, 0.0),
                rect.min + vec2(w, h * 0.18),
                rect.min + vec2(w * 0.5, h * 0.53),
                rect.min + vec2(0.0, h * 0.18),
            ];

            let c = rect.min + vec2(w * 0.5, h * 0.58);
            p.circle_filled(c, w * 0.23, Color32::from_rgb(30, 41, 59));
            p.add(egui::Shape::convex_polygon(chevron_pts, Color32::from_rgb(0, 114, 188), egui::Stroke::NONE));
            p.circle_stroke(c, w * 0.28, egui::Stroke::new(3.5, Color32::from_rgb(245, 171, 25)));

            let footer = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.90), rect.max);
            p.rect_filled(footer, egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, Color32::from_rgb(18, 38, 71));
        }
        4 => {
            // Style 4 (Honeycomb Hexagon): Dark background + vertical stripes + chevron banner + tri-hexagon cluster
            p.rect_filled(rect, 4.0, Color32::from_rgb(17, 24, 39));

            // Stripes
            let s1 = egui::Rect::from_min_max(rect.min + vec2(w * 0.60, 0.0), rect.min + vec2(w * 0.68, h));
            let s2 = egui::Rect::from_min_max(rect.min + vec2(w * 0.70, 0.0), rect.min + vec2(w * 0.74, h));
            let s3 = egui::Rect::from_min_max(rect.min + vec2(w * 0.76, 0.0), rect.min + vec2(w * 0.78, h));
            p.rect_filled(s1, 0.0, Color32::from_rgba_unmultiplied(245, 171, 25, 200));
            p.rect_filled(s2, 0.0, Color32::from_rgba_unmultiplied(245, 171, 25, 140));
            p.rect_filled(s3, 0.0, Color32::from_rgba_unmultiplied(245, 171, 25, 90));

            // Left banner
            let banner = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.26), rect.min + vec2(w * 0.45, h * 0.50));
            p.rect_filled(banner, 0.0, Color32::from_rgb(0, 114, 188));
            p.line_segment(
                [rect.min + vec2(0.0, h * 0.50), rect.min + vec2(w * 0.45, h * 0.50)],
                egui::Stroke::new(2.0, Color32::from_rgb(245, 171, 25)),
            );

            // Tri-hexagon cluster representation
            let c = rect.min + vec2(w * 0.65, h * 0.48);
            p.circle_filled(c, w * 0.24, Color32::WHITE);
            p.circle_filled(c, w * 0.21, Color32::from_rgb(30, 41, 59));
            p.circle_stroke(c, w * 0.22, egui::Stroke::new(1.5, Color32::from_rgb(245, 171, 25)));
        }
        5 => {
            // Style 5 (Corporate Headline): White base + top bar + top-right metadata + photo vertical frame + rounded headline block + bottom-right block + bottom-left logo (position lock)
            p.rect_filled(rect, 4.0, Color32::WHITE);
            // Top bar
            let top_bar = egui::Rect::from_min_max(rect.min, rect.min + vec2(w * 0.90, h * 0.025));
            p.rect_filled(top_bar, 0.0, Color32::from_rgb(108, 92, 231));
            // Top-right metadata lines (position lock)
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.62, h * 0.035), rect.min + vec2(w * 0.94, h * 0.048)),
                1.0,
                Color32::from_rgb(100, 116, 139),
            );
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.70, h * 0.055), rect.min + vec2(w * 0.94, h * 0.068)),
                1.0,
                Color32::from_rgb(100, 116, 139),
            );
            // Bottom-right block
            let br_block = egui::Rect::from_min_max(rect.min + vec2(w * 0.72, h * 0.92), rect.max);
            p.rect_filled(br_block, egui::CornerRadius { nw: 0, ne: 0, sw: 0, se: 4 }, Color32::from_rgb(108, 92, 231));
            // Center-right vertical photo frame
            let photo = egui::Rect::from_min_max(rect.min + vec2(w * 0.38, h * 0.20), rect.min + vec2(w * 0.95, h * 0.92));
            p.rect_filled(photo, 0.0, Color32::from_rgb(140, 170, 200));
            // Top-left L bracket
            let l_h = egui::Rect::from_min_max(rect.min + vec2(w * 0.36, h * 0.19), rect.min + vec2(w * 0.55, h * 0.205));
            let l_v = egui::Rect::from_min_max(rect.min + vec2(w * 0.36, h * 0.19), rect.min + vec2(w * 0.38, h * 0.32));
            p.rect_filled(l_h, 0.0, Color32::from_rgb(108, 92, 231));
            p.rect_filled(l_v, 0.0, Color32::from_rgb(108, 92, 231));
            // Top-right corporate accent card
            let corp_box = egui::Rect::from_min_max(rect.min + vec2(w * 0.58, h * 0.13), rect.min + vec2(w, h * 0.28));
            p.rect_filled(corp_box, 0.0, Color32::from_rgb(108, 92, 231));
            // Middle-left year & volume line
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.31), rect.min + vec2(w * 0.34, h * 0.39)),
                2.0,
                Color32::from_rgb(30, 41, 59),
            );
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.41), rect.min + vec2(w * 0.32, h * 0.43)),
                1.0,
                Color32::from_rgb(108, 92, 231),
            );
            // Headline block with rounded top-right (semi-transparent)
            let hl_block = egui::Rect::from_min_max(rect.min + vec2(0.0, h * 0.54), rect.min + vec2(w * 0.58, h * 0.81));
            p.rect_filled(hl_block, egui::CornerRadius { nw: 0, ne: 8, sw: 0, se: 0 }, Color32::from_rgba_unmultiplied(108, 92, 231, 218));
            // Bottom-left logo & agency text (position lock)
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.87), rect.min + vec2(w * 0.14, h * 0.94)),
                2.0,
                Color32::from_rgb(108, 92, 231),
            );
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.16, h * 0.88), rect.min + vec2(w * 0.52, h * 0.90)),
                1.0,
                Color32::from_rgb(30, 41, 59),
            );
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.16, h * 0.91), rect.min + vec2(w * 0.46, h * 0.93)),
                1.0,
                Color32::from_rgb(30, 41, 59),
            );
        }
        6 => {
            // Style 6 (Diagonal Arrow): white base + right photo panel + navy top banner +
            // amber corner wedge + navy forward wedge carrying the year + mid-left headline
            p.rect_filled(rect, 4.0, Color32::WHITE);

            // Right-hand photo panel
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.48, 0.0), rect.max), 0.0, Color32::from_rgb(146, 172, 198));

            let navy = Color32::from_rgb(18, 38, 71);

            // Navy diagonal banner across the top
            let band = vec![
                rect.min + vec2(w * 0.19, 0.0),
                rect.min + vec2(w * 0.672, 0.0),
                rect.min + vec2(w * 0.481, h * 0.304),
                rect.min + vec2(0.0, h * 0.304),
            ];
            p.add(egui::Shape::convex_polygon(band, navy, egui::Stroke::NONE));

            // Amber corner wedge (top-left)
            let wedge = vec![rect.min, rect.min + vec2(w * 0.146, 0.0), rect.min + vec2(0.0, h * 0.233)];
            p.add(egui::Shape::convex_polygon(wedge, Color32::from_rgb(245, 171, 25), egui::Stroke::NONE));

            // Top-right metadata lines (position lock, over the photo)
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.56, h * 0.020), rect.min + vec2(w * 0.94, h * 0.032)),
                1.0,
                Color32::from_rgb(226, 232, 240),
            );
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.60, h * 0.040), rect.min + vec2(w * 0.94, h * 0.052)),
                1.0,
                Color32::from_rgb(226, 232, 240),
            );

            // Navy forward wedge carrying the year and volume
            let wedge2 = vec![rect.min + vec2(0.0, h * 0.617), rect.min + vec2(w * 0.591, h * 0.702), rect.min + vec2(0.0, h * 0.788)];
            p.add(egui::Shape::convex_polygon(wedge2, navy, egui::Stroke::NONE));
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.03, h * 0.672), rect.min + vec2(w * 0.27, h * 0.726)), 2.0, Color32::WHITE);
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.03, h * 0.735), rect.min + vec2(w * 0.26, h * 0.751)),
                1.0,
                Color32::from_rgb(226, 232, 240),
            );

            // Middle-left headline: level (navy) / title (amber) / DALAM ANGKA (navy)
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.425), rect.min + vec2(w * 0.42, h * 0.465)), 2.0, navy);
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.468), rect.min + vec2(w * 0.62, h * 0.508)),
                2.0,
                Color32::from_rgb(245, 171, 25),
            );
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.512), rect.min + vec2(w * 0.45, h * 0.552)), 2.0, navy);

            // Bottom-left logo & agency text (position lock)
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.885), rect.min + vec2(w * 0.13, h * 0.935)),
                2.0,
                Color32::from_rgb(0, 160, 220),
            );
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.15, h * 0.892), rect.min + vec2(w * 0.47, h * 0.905)), 1.0, navy);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.15, h * 0.915), rect.min + vec2(w * 0.42, h * 0.928)), 1.0, navy);
        }
        7 => {
            // Style 7 (Modern Horizon): full-bleed photo under a sweeping arc + orange swoosh +
            // navy bottom block with the rounded top-left corner carrying the headline
            p.rect_filled(rect, 4.0, Color32::WHITE);
            let navy = Color32::from_rgb(8, 38, 65);
            let orange = Color32::from_rgb(245, 135, 31);

            // Photograph fills the top arc: an approximation of the traced sweep.
            let arc = vec![rect.min, rect.min + vec2(w, 0.0), rect.min + vec2(w, h * 0.557), rect.min + vec2(0.0, h * 0.204)];
            p.add(egui::Shape::convex_polygon(arc, Color32::from_rgb(150, 172, 196), egui::Stroke::NONE));

            // Navy block: rounded top-left corner, flat lower half.
            let navy_pts = vec![
                rect.min + vec2(0.0, h * 0.627),
                rect.min + vec2(w * 0.326, h * 0.499),
                rect.min + vec2(w * 0.625, h * 0.549),
                rect.min + vec2(w, h * 0.549),
                rect.max,
                rect.min + vec2(0.0, h),
            ];
            p.add(egui::Shape::convex_polygon(navy_pts, navy, egui::Stroke::NONE));

            // Orange wedge following the arc (left) and the accent block (right).
            let orange_pts = vec![
                rect.min + vec2(0.0, h * 0.208),
                rect.min + vec2(w, h * 0.561),
                rect.min + vec2(w, h * 0.855),
                rect.min + vec2(w * 0.625, h * 0.855),
                rect.min + vec2(w * 0.625, h * 0.544),
                rect.min + vec2(0.0, h * 0.622),
            ];
            p.add(egui::Shape::convex_polygon(orange_pts, orange, egui::Stroke::NONE));

            // Top-right metadata (position lock)
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.56, h * 0.020), rect.min + vec2(w * 0.94, h * 0.032)), 1.0, navy);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.60, h * 0.040), rect.min + vec2(w * 0.94, h * 0.052)), 1.0, navy);

            // Headline: level (white) / title (orange) / DALAM ANGKA (white) inside the navy block
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.662), rect.min + vec2(w * 0.50, h * 0.706)), 2.0, Color32::WHITE);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.716), rect.min + vec2(w * 0.72, h * 0.766)), 2.0, orange);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.776), rect.min + vec2(w * 0.50, h * 0.820)), 2.0, Color32::WHITE);

            // Year & volume centred in the accent block
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.68, h * 0.645), rect.min + vec2(w * 0.92, h * 0.712)), 2.0, Color32::WHITE);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.72, h * 0.734), rect.min + vec2(w * 0.88, h * 0.748)), 1.0, Color32::WHITE);

            // Bottom-left logo & agency text (position lock)
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.885), rect.min + vec2(w * 0.14, h * 0.945)),
                2.0,
                Color32::from_rgb(0, 160, 220),
            );
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.16, h * 0.892), rect.min + vec2(w * 0.52, h * 0.905)), 1.0, Color32::WHITE);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.16, h * 0.915), rect.min + vec2(w * 0.46, h * 0.928)), 1.0, Color32::WHITE);
        }
        9 => {
            // Style 9 (Urban Angle): orange top-left wedge, navy diagonal band, cityscape photo on
            // the right, white negative-X, navy bottom wedge.
            let accent = Color32::from_rgb(252, 185, 30);
            let navy = Color32::from_rgb(3, 68, 96);
            // Photo region on the right.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min + vec2(w * 0.39, h * 0.43), rect.min + vec2(w, 0.0), rect.min + vec2(w, h * 0.99)],
                Color32::from_rgb(120, 150, 170),
                egui::Stroke::NONE,
            ));
            // Orange top-left wedge.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min, rect.min + vec2(w * 0.86, 0.0), rect.min + vec2(0.0, h * 0.43)],
                accent,
                egui::Stroke::NONE,
            ));
            // Navy diagonal band.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min + vec2(w * 0.50, 0.0), rect.min + vec2(w * 0.90, 0.0), rect.min + vec2(w * 0.44, h * 0.45)],
                navy,
                egui::Stroke::NONE,
            ));
            // Navy bottom wedge.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min + vec2(w * 0.39, h * 0.78), rect.min + vec2(w * 0.82, h), rect.min + vec2(w * 0.35, h)],
                navy,
                egui::Stroke::NONE,
            ));
            // Headline stack on the white left panel.
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.68), rect.min + vec2(w * 0.50, h * 0.72)), 2.0, navy);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.75), rect.min + vec2(w * 0.60, h * 0.80)), 2.0, accent);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.83), rect.min + vec2(w * 0.45, h * 0.87)), 2.0, navy);
            // Top-left company mark.
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.05, h * 0.02), rect.min + vec2(w * 0.12, h * 0.06)), 1.0, navy);
            // Rotated year in the band.
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.55, h * 0.10), rect.min + vec2(w * 0.78, h * 0.20)), 2.0, Color32::WHITE);
        }
        10 => {
            // Style 10 (Tri-Color Bands): tri-colour masthead rule, right-hand photo panel over a
            // tri-colour bar, two-tone year on the left, blue institutional band at the foot.
            let blue = Color32::from_rgb(0, 136, 210);
            let green = Color32::from_rgb(116, 191, 69);
            let orange = Color32::from_rgb(243, 139, 30);
            let ink = Color32::from_rgb(16, 42, 60);
            let band =
                |x0: f32, x1: f32, y0: f32, y1: f32| egui::Rect::from_min_max(rect.min + vec2(w * x0, h * y0), rect.min + vec2(w * x1, h * y1));
            p.rect_filled(rect, 4.0, Color32::WHITE);
            // Masthead rule.
            p.rect_filled(band(0.0, 0.333, 0.0, 0.013), 0.0, blue);
            p.rect_filled(band(0.333, 0.666, 0.0, 0.013), 0.0, green);
            p.rect_filled(band(0.666, 1.0, 0.0, 0.013), 0.0, orange);
            // Right-hand photograph and the bar under it.
            p.rect_filled(band(0.54, 1.0, 0.11, 0.60), 0.0, Color32::from_rgb(150, 172, 196));
            p.rect_filled(band(0.54, 0.693, 0.60, 0.613), 0.0, blue);
            p.rect_filled(band(0.693, 0.846, 0.60, 0.613), 0.0, green);
            p.rect_filled(band(0.846, 1.0, 0.60, 0.613), 0.0, orange);
            // Blue accent bar, headline and second headline line.
            p.rect_filled(band(0.075, 0.095, 0.11, 0.20), 0.0, blue);
            p.rect_filled(band(0.075, 0.52, 0.22, 0.265), 1.0, ink);
            p.rect_filled(band(0.075, 0.49, 0.295, 0.34), 1.0, ink);
            p.rect_filled(band(0.075, 0.34, 0.37, 0.405), 1.0, ink);
            p.rect_filled(band(0.075, 0.40, 0.415, 0.432), 1.0, orange);
            // Two-tone year, volume line and green rule.
            p.rect_filled(band(0.075, 0.235, 0.60, 0.645), 1.0, blue);
            p.rect_filled(band(0.235, 0.42, 0.60, 0.645), 1.0, ink);
            p.rect_filled(band(0.075, 0.30, 0.665, 0.678), 1.0, ink);
            p.rect_filled(band(0.075, 0.335, 0.703, 0.708), 0.0, green);
            // Institutional band: logo plate and agency line.
            p.rect_filled(band(0.0, 1.0, 0.823, 0.833), 0.0, green);
            p.rect_filled(band(0.0, 1.0, 0.833, 0.843), 0.0, orange);
            p.rect_filled(band(0.0, 1.0, 0.843, 1.0), 0.0, blue);
            p.rect_filled(band(0.075, 0.21, 0.868, 0.95), 0.0, Color32::WHITE);
            p.rect_filled(band(0.23, 0.55, 0.878, 0.892), 1.0, Color32::WHITE);
            p.rect_filled(band(0.23, 0.50, 0.900, 0.914), 1.0, Color32::WHITE);
        }
        11 => {
            // Style 11 (Prism Diamonds): a midnight field with a cascade of jewelled diamonds around
            // a diamond-cut photograph, an ice-white headline over a two-tone year, and the blue
            // institutional band at the foot.
            let night = Color32::from_rgb(10, 34, 51);
            let blue = Color32::from_rgb(0, 136, 210);
            let green = Color32::from_rgb(116, 191, 69);
            let orange = Color32::from_rgb(243, 139, 30);
            let ice = Color32::from_rgb(237, 244, 248);
            let band =
                |x0: f32, x1: f32, y0: f32, y1: f32| egui::Rect::from_min_max(rect.min + vec2(w * x0, h * y0), rect.min + vec2(w * x1, h * y1));
            let diamond = |cx: f32, cy: f32, r: f32| {
                vec![rect.min + vec2(cx, cy - r), rect.min + vec2(cx + r, cy), rect.min + vec2(cx, cy + r), rect.min + vec2(cx - r, cy)]
            };
            p.rect_filled(rect, 4.0, night);
            p.add(egui::Shape::convex_polygon(diamond(w * 0.55, h * 0.36, w * 0.36), Color32::from_rgb(14, 58, 80), egui::Stroke::NONE));
            p.add(egui::Shape::convex_polygon(diamond(w * 0.73, h * 0.24, w * 0.26), Color32::from_rgb(150, 172, 196), egui::Stroke::NONE));
            p.add(egui::Shape::convex_polygon(diamond(w * 0.49, h * 0.15, w * 0.08), green, egui::Stroke::NONE));
            p.add(egui::Shape::convex_polygon(diamond(w * 0.43, h * 0.25, w * 0.05), orange, egui::Stroke::NONE));
            p.add(egui::Shape::convex_polygon(diamond(w * 0.58, h * 0.36, w * 0.09), Color32::from_rgb(127, 198, 232), egui::Stroke::NONE));
            p.add(egui::Shape::convex_polygon(diamond(w * 0.93, h * 0.48, w * 0.11), Color32::from_rgb(191, 227, 204), egui::Stroke::NONE));
            p.add(egui::Shape::convex_polygon(diamond(w * 0.78, h * 0.56, w * 0.07), Color32::from_rgb(253, 178, 122), egui::Stroke::NONE));
            p.add(egui::Shape::convex_polygon(diamond(w * 0.88, h * 0.68, w * 0.05), green, egui::Stroke::NONE));
            // Headline, second line, and the two-tone year with its volume line.
            p.rect_filled(band(0.075, 0.64, 0.545, 0.573), 1.0, ice);
            p.rect_filled(band(0.075, 0.42, 0.600, 0.628), 1.0, ice);
            p.rect_filled(band(0.075, 0.33, 0.674, 0.694), 1.0, orange);
            p.rect_filled(band(0.075, 0.21, 0.716, 0.780), 1.0, blue);
            p.rect_filled(band(0.21, 0.37, 0.716, 0.780), 1.0, ice);
            p.rect_filled(band(0.075, 0.30, 0.818, 0.830), 1.0, ice);
            p.rect_filled(band(0.075, 0.32, 0.855, 0.858), 0.0, green);
            // Green seam, blue institutional band, logo plate and band diamonds.
            p.rect_filled(band(0.0, 1.0, 0.891, 0.896), 0.0, green);
            p.rect_filled(band(0.0, 1.0, 0.896, 1.0), 0.0, blue);
            p.rect_filled(band(0.075, 0.21, 0.920, 0.979), 0.0, Color32::WHITE);
            p.add(egui::Shape::convex_polygon(diamond(w * 0.78, h * 0.949, w * 0.025), green, egui::Stroke::NONE));
            p.add(egui::Shape::convex_polygon(diamond(w * 0.84, h * 0.957, w * 0.017), orange, egui::Stroke::NONE));
        }
        12 => {
            // Style 12 (Quarter Stack): a warm paper field, a fanned stack of cards that each round
            // a single corner, a cascading chip cluster and the ink institutional band.
            let paper = Color32::from_rgb(246, 241, 231);
            let ink = Color32::from_rgb(31, 42, 68);
            let clay = Color32::from_rgb(199, 91, 57);
            let marigold = Color32::from_rgb(232, 163, 61);
            let sage = Color32::from_rgb(147, 169, 140);
            let blush = Color32::from_rgb(231, 199, 179);
            let teal = Color32::from_rgb(47, 111, 107);
            let band =
                |x0: f32, x1: f32, y0: f32, y1: f32| egui::Rect::from_min_max(rect.min + vec2(w * x0, h * y0), rect.min + vec2(w * x1, h * y1));
            let one = |nw: u8, ne: u8, sw: u8, se: u8| egui::CornerRadius { nw, ne, sw, se };
            p.rect_filled(rect, 4.0, paper);
            // The fanned cards, each with one rounded corner, the photograph at the front.
            p.rect_filled(band(0.39, 0.89, 0.0, 0.36), one(0, 0, 22, 0), blush);
            p.rect_filled(band(0.44, 0.94, 0.024, 0.38), one(22, 0, 0, 0), sage);
            p.rect_filled(band(0.49, 0.99, 0.06, 0.42), one(0, 22, 0, 0), clay);
            p.rect_filled(band(0.55, 1.0, 0.095, 0.46), one(0, 0, 0, 26), Color32::from_rgb(150, 172, 196));
            // Chip cluster below the stack.
            p.rect_filled(band(0.72, 0.91, 0.50, 0.63), one(10, 0, 0, 0), teal);
            p.rect_filled(band(0.84, 1.0, 0.59, 0.72), one(0, 0, 0, 10), marigold);
            p.rect_filled(band(0.76, 0.84, 0.69, 0.75), one(0, 6, 0, 0), blush);
            p.rect_filled(band(0.93, 1.0, 0.69, 0.77), one(0, 6, 0, 0), clay);
            // Headline, second line, two-tone year and volume line.
            p.rect_filled(band(0.075, 0.62, 0.481, 0.514), 1.0, ink);
            p.rect_filled(band(0.075, 0.47, 0.530, 0.554), 1.0, ink);
            p.rect_filled(band(0.075, 0.33, 0.634, 0.660), 1.0, clay);
            p.rect_filled(band(0.075, 0.21, 0.679, 0.757), 1.0, clay);
            p.rect_filled(band(0.21, 0.38, 0.679, 0.757), 1.0, ink);
            p.rect_filled(band(0.075, 0.30, 0.788, 0.800), 1.0, ink);
            p.rect_filled(band(0.075, 0.32, 0.827, 0.830), 0.0, sage);
            // Ink band, paper logo plate and the band chips.
            p.rect_filled(band(0.0, 1.0, 0.896, 1.0), 0.0, ink);
            p.rect_filled(band(0.075, 0.21, 0.921, 0.981), one(0, 6, 0, 0), paper);
            p.add(egui::Shape::convex_polygon(
                vec![
                    rect.min + vec2(w * 0.74, h * 0.938),
                    rect.min + vec2(w * 0.79, h * 0.938),
                    rect.min + vec2(w * 0.79, h * 0.974),
                    rect.min + vec2(w * 0.74, h * 0.974),
                ],
                sage,
                egui::Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![
                    rect.min + vec2(w * 0.81, h * 0.947),
                    rect.min + vec2(w * 0.86, h * 0.947),
                    rect.min + vec2(w * 0.86, h * 0.983),
                    rect.min + vec2(w * 0.81, h * 0.983),
                ],
                marigold,
                egui::Stroke::NONE,
            ));
        }
        _ => {
            // Style 8 (Business Report, traced from design-10): white base, orange top band and
            // bottom-right corner, arrow-shaped photograph with its chevron sliver across a thin
            // white gap, navy plane crossing from the left with a white year circle, headline
            // top-left, agency block bottom-left.
            p.rect_filled(rect, 4.0, Color32::WHITE);
            let navy = Color32::from_rgb(30, 58, 95);
            let orange = Color32::from_rgb(240, 130, 35);

            // Orange bottom-right corner, then the navy plane crossing from the left.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min + vec2(w * 0.712, h * 0.877), rect.max, rect.min + vec2(w * 0.903, h)],
                orange,
                egui::Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![
                    rect.min + vec2(0.0, h * 0.351),
                    rect.min + vec2(w * 0.364, h * 0.497),
                    rect.min + vec2(w * 0.364, h * 0.738),
                    rect.min + vec2(w, h * 0.993),
                    rect.min,
                ],
                navy,
                egui::Stroke::NONE,
            ));

            // Orange band along the top edge.
            p.add(egui::Shape::convex_polygon(
                vec![rect.min + vec2(w * 0.416, 0.0), rect.min + vec2(w, 0.0), rect.min + vec2(w, h * 0.096), rect.min + vec2(w * 0.824, h * 0.17)],
                orange,
                egui::Stroke::NONE,
            ));

            // Arrow-shaped photograph, the white gap across it, then the chevron sliver.
            let arrow = [
                rect.min + vec2(w * 0.34, h * 0.376),
                rect.min + vec2(w, h * 0.108),
                rect.min + vec2(w, h * 0.745),
                rect.min + vec2(w * 0.34, h * 0.48),
            ];
            p.add(egui::Shape::convex_polygon(arrow.to_vec(), Color32::from_rgb(150, 172, 196), egui::Stroke::NONE));
            p.add(egui::Shape::convex_polygon(
                vec![
                    rect.min + vec2(w * 0.34, h * 0.48),
                    rect.min + vec2(w, h * 0.745),
                    rect.min + vec2(w * 0.99, h * 0.751),
                    rect.min + vec2(w * 0.371, h * 0.503),
                ],
                Color32::WHITE,
                egui::Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![
                    rect.min + vec2(w * 0.371, h * 0.503),
                    rect.min + vec2(w * 0.99, h * 0.751),
                    rect.min + vec2(w * 0.711, h * 0.866),
                    rect.min + vec2(w * 0.371, h * 0.73),
                ],
                Color32::from_rgb(108, 124, 140),
                egui::Stroke::NONE,
            ));

            // White year circle on the navy plane.
            p.circle_filled(rect.min + vec2(w * 0.252, h * 0.631), w * 0.16, Color32::WHITE);

            // Headline stack: level (navy), title (orange), DALAM ANGKA (navy).
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.114), rect.min + vec2(w * 0.6, h * 0.15)), 2.0, navy);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.152), rect.min + vec2(w * 0.62, h * 0.207)), 2.0, orange);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.06, h * 0.209), rect.min + vec2(w * 0.6, h * 0.245)), 2.0, navy);

            // Two-tone year and the volume line inside the circle.
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.135, h * 0.608), rect.min + vec2(w * 0.25, h * 0.663)), 2.0, navy);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.255, h * 0.608), rect.min + vec2(w * 0.365, h * 0.663)), 2.0, orange);
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.135, h * 0.687), rect.min + vec2(w * 0.405, h * 0.717)), 2.0, navy);

            // Agency block bottom-left: logo mark + white agency lines on the navy plane.
            p.rect_filled(egui::Rect::from_min_max(rect.min + vec2(w * 0.077, h * 0.917), rect.min + vec2(w * 0.145, h * 0.965)), 1.0, orange);
            p.rect_filled(
                egui::Rect::from_min_max(rect.min + vec2(w * 0.158, h * 0.919), rect.min + vec2(w * 0.521, h * 0.969)),
                1.0,
                Color32::from_gray(235),
            );
        }
    }
}

pub(crate) fn show_style_thumbnail_tooltip(ui: &mut Ui, style_idx: usize, name: &str, t: &Tokens) {
    ui.vertical(|ui| {
        ui.label(RichText::new(name).size(12.0).strong().color(t.text_strong));
        ui.add_space(4.0);

        let ctx = ui.ctx().clone();
        if let Some(tex) = style_thumbnail_texture(&ctx, style_idx) {
            let size = tex.size_vec2();
            let thumb_w = 150.0;
            let thumb_h = (thumb_w * size.y / size.x.max(1.0)).min(220.0);
            let (rect, _) = ui.allocate_exact_size(vec2(thumb_w, thumb_h), Sense::hover());
            ui.painter().rect_filled(rect, 4.0, Color32::from_gray(30));
            ui.painter().image(tex.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Outside);
        } else {
            paint_fallback_cover_thumb(ui, style_idx, 150.0, 212.0, t);
        }

        let desc = match style_idx {
            0 => "Wave curves, concentric cradle ring, and circular medallion photo.",
            1 => "Modern diagonal polygon landscape background with circular cutout.",
            2 => "Geometric origami folded facet ribbons with hexagonal photo header.",
            3 => "Executive downward chevron crest with golden horseshoe medallion cradle.",
            4 => "Architectural tri-hexagon honeycomb cluster with corporate chevron banner and racing stripes.",
            5 => "Modern corporate architecture layout with top framing, headline focal block, and position-locked institutional metadata.",
            6 => "Navy diagonal banner and amber corner wedge over a half-bleed cityscape, with a forward navy wedge carrying the year and volume, and a position-locked agency footer.",
            7 => "Full-bleed cityscape under a sweeping arc with an accent wedge that follows it, a dark institutional block carrying the headline, and a position-locked agency footer.",
            8 => "Arrow-shaped skyline photograph with a chevron sliver across a white gap, an orange top band and corner, a navy plane carrying the white year circle, the headline top-left and the agency block bottom-left.",
            9 => "Orange corner wedge and a navy diagonal band carrying the rotated year over a right-hand cityscape, with a white negative-space headline panel and a navy bottom wedge.",
            10 => "Executive white grid: a tri-colour masthead rule, a right-hand cityscape panel over a blue/green/orange bar, a two-tone year and a blue institutional band.",
            11 => "Midnight prism: a cascade of jewelled diamonds around a diamond-cut photograph, an ice-white headline over a two-tone year, and a blue institutional band.",
            12 => "Warm paper stack: four cards fanned with one rounded corner each around a card-cut photograph, a cascading chip cluster, and an ink institutional band.",
            s if s >= super::ai_cover::AI_STYLE_BASE => match super::ai_cover::preset(s) {
                Some(p) => p.description,
                None => "",
            },
            _ => "",
        };
        ui.add_space(3.0);
        ui.label(RichText::new(desc).size(10.0).color(t.text_dim));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style_of(name: &str) -> Option<usize> {
        style_thumb_match(name).map(|(style, _)| style)
    }

    /// Every sample PNG that ships in `plan/pubgen/samples/thumb` still finds its style.
    #[test]
    fn sample_thumbnails_find_their_style() {
        for (name, style) in [
            ("wave_curves.png", 0),
            ("geometric_medallion.png", 1),
            ("folded_origami.png", 2),
            ("chevron_medallion.png", 3),
            ("honeycomb_hexagon.png", 4),
            ("karawang_timur_hexagon_cover.png", 4),
            ("corporate_headline.png", 5),
            ("diagonal_arrow.png", 6),
            ("modern_horizon.png", 7),
            ("business_report.png", 8),
            ("urban_angle.png", 9),
            ("tri_color_bands.png", 10),
            ("prism_diamonds.png", 11),
            ("quarter_stack.png", 12),
            ("ai_base.png", super::super::ai_cover::AI_STYLE_BASE),
            ("ai_base_template.png", super::super::ai_cover::AI_STYLE_BASE),
        ] {
            assert_eq!(style_of(name), Some(style), "{name}");
        }
    }

    /// A name the user makes up still lands: a numeric prefix, a style label, or spaces/case.
    #[test]
    fn user_file_names_pick_a_style() {
        assert_eq!(style_of("03_my_cover.png"), Some(3));
        assert_eq!(style_of("style7.jpg"), Some(7));
        assert_eq!(style_of("BPS Berita Resmi Statistik (BRS).png"), Some(13));
        assert_eq!(style_of("quarter-stack.webp"), Some(12));
        assert_eq!(style_of("photo_of_the_office.png"), None);
    }

    /// A numeric prefix outranks an alias, so `4_wave_curves.png` is honeycomb, not wave.
    #[test]
    fn a_numeric_prefix_outranks_an_alias() {
        let prefix = style_thumb_match("4_wave_curves.png");
        let alias = style_thumb_match("wave_curves.png");
        assert_eq!(prefix.map(|(style, _)| style), Some(4));
        assert!(prefix.is_some_and(|(_, score)| alias.is_some_and(|(_, alias_score)| score > alias_score)));
    }

    /// Folders are searched in order, an explicit name beats the sample it sits next to, and a
    /// folder without the style falls through to the next one.
    #[test]
    fn a_renamed_thumbnail_overrides_the_sample_in_the_same_folder() {
        let root = std::env::temp_dir().join(format!("designcraft-thumb-{}", std::process::id()));
        let first = root.join("first");
        let second = root.join("second");
        let _ = std::fs::remove_dir_all(&root);
        assert!(std::fs::create_dir_all(&first).is_ok() && std::fs::create_dir_all(&second).is_ok());
        let sample = first.join("chevron_medallion.png");
        let renamed = first.join("03_mine.png");
        let elsewhere = second.join("5_corporate.png");
        for path in [&sample, &renamed, &elsewhere] {
            assert!(std::fs::write(path, b"").is_ok());
        }

        assert_eq!(thumb_path_in(&[first.clone(), second.clone()], 3), Some(renamed));
        assert_eq!(thumb_path_in(&[first.clone(), second.clone()], 5), Some(elsewhere));
        assert_eq!(thumb_path_in(&[first], 0), None, "no file, so the painted fallback shows");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The checked-in samples still resolve through the folder itself (they are local-only, so a
    /// checkout without `plan/` skips this).
    #[test]
    fn the_repo_sample_folder_is_found() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plan/pubgen/samples/thumb");
        if !dir.is_dir() {
            return;
        }
        for (style, name) in [(0, "wave_curves.png"), (8, "business_report.png"), (12, "quarter_stack.png")] {
            let found = thumb_path_in(std::slice::from_ref(&dir), style);
            assert!(found.as_deref().and_then(|p| p.file_name()).and_then(|n| n.to_str()) == Some(name), "{style}");
        }
    }
}
