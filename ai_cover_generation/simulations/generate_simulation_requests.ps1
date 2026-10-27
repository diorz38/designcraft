# Simulation script for Pubgen Publication Cover AI requests
$ErrorActionPreference = 'Stop'

$simDir = Join-Path $PSScriptRoot "simulations"
if (!(Test-Path $simDir)) {
    New-Item -ItemType Directory -Path $simDir | Out-Null
}

$presets = @(
    @{
        id = 100
        name = "Retro Waves"
        slug = "retro_waves"
        words = "minimalist mid-century modern abstract wallpaper, flowing organic waves and curving colour blocks, 1970s retro aesthetic, smooth flat vector shapes, continuous panoramic art"
    },
    @{
        id = 101
        name = "Geometric Business"
        slug = "geometric_business"
        words = "flat vector 2D modern geometric business wallpaper, dramatic scale contrast with radical hexagon size differences, one massive prominent hero hexagon frame, floating medium hexagons, and tiny delicate micro-hexagon accents, large angled diagonal thick rounded stripes across, smooth gradient color overlays, clean blank copy space, sharp crisp vector edges, flat graphic design, continuous panoramic flow"
    },
    @{
        id = 102
        name = "Minimal Sci-Fi"
        slug = "minimal_sci_fi"
        words = "minimal retro sci-fi background, geometric horizon lines, massive glowing sun disc in sky, faint starfield, clean flat vector shapes, wide panorama"
    },
    @{
        id = 103
        name = "Dynamic Waves Spread"
        slug = "dynamic_waves_spread"
        words = "flat vector 2D graphic design, dramatic scale contrast with radical size differences between shapes, one massive broad sweeping corporate curved color panel dominating the spread, paired with slim secondary accent ribbons and small delicate floating geometric accent marks, sharp crisp vector edges, expansive clean blank copy space, continuous panoramic flow"
    },
    @{
        id = 104
        name = "Vibrant S-Curve Spread"
        slug = "vibrant_s_curve_spread"
        words = "flat vector 2D graphic design, extreme scale contrast and radical shape size hierarchy, one gigantic broad sweeping S-curve ribbon band flowing across the spread, contrasted with narrow sharp accent ribbons and small minimalist geometric accents, smooth gradient overlays, sharp clean vector contour edges, generous blank negative copy space, continuous panoramic flow"
    }
)

$palettes = @(
    @{
        id = 0
        name = "Warm Terracotta"
        slug = "warm_terracotta"
        words = "terracotta red, mustard yellow, sage green, warm cream"
        primary = "#b5543b"; accent = "#e8a34d"; dark = "#3e2c23"
    },
    @{
        id = 1
        name = "Pastel Dream"
        slug = "pastel_dream"
        words = "soft pastel pink, lilac, baby blue, off-white"
        primary = "#8e7cc3"; accent = "#e8a0bf"; dark = "#4a4458"
    },
    @{
        id = 2
        name = "Mono Slate"
        slug = "mono_slate"
        words = "monochrome dark slate, charcoal, silver grey, near-black"
        primary = "#53616c"; accent = "#9fb1bc"; dark = "#1c252c"
    },
    @{
        id = 3
        name = "Forest Gold"
        slug = "forest_gold"
        words = "deep forest green, moss, gold, cream"
        primary = "#2f5d50"; accent = "#c9a227"; dark = "#1c2b24"
    },
    @{
        id = 4
        name = "Corporate Blue"
        slug = "corporate_blue"
        words = "dark navy blue #0A1931, deep royal blue #003399, vibrant electric cyan blue #00A8FF, soft silver-gray #E0E6ED, clean solid white background"
        primary = "#003399"; accent = "#00a8ff"; dark = "#0a1931"
    },
    @{
        id = 5
        name = "Teal Lime"
        slug = "teal_lime"
        words = "deep teal green, dark pine green, chartreuse lime green gradient, mint turquoise, crisp white"
        primary = "#006d6f"; accent = "#84cc16"; dark = "#0a2e28"
    }
)

# Random selection of 1 palette per preset (deterministic random seed for repeatability)
# Style 100 -> Palette 1 (Pastel Dream)
# Style 101 -> Palette 4 (Corporate Blue)
# Style 102 -> Palette 0 (Warm Terracotta)
# Style 103 -> Palette 5 (Teal Lime)
# Style 104 -> Palette 3 (Forest Gold)
$chosenPalettes = @(1, 4, 0, 5, 3)

$title = "KARAWANG TIMUR" # Default title in Pubgen state
$negPrompt = "text, typography, letters, words, numbers, fonts, lorem ipsum, dummy copy, watermarks, logo, icons, symbols, realistic photo, photograph, 3d render, perspective view, angled mockup, dropshadow, realistic book mockup, brochure mockup, folded paper, standing card, desk, table, presentation board, collage, multi-panel, split-screen, seam, divider line, gutter, picture frame, framing, borders, margins, textured paper, checkerboard, transparent grid, grey and white grid, photo placeholder, template, booklet, brochure layout, document template, uniform shape size, equal sized shapes, repetitive tiling, monotonous pattern, wallpaper texture, liquid marble, fluid acrylic, paint swirls, dense ripples, messy zebra stripes, clipping artifacts, busy details, author name, signature"

for ($i = 0; $i -lt $presets.Count; $i++) {
    $preset = $presets[$i]
    $palIdx = $chosenPalettes[$i]
    $pal = $palettes[$palIdx]

    $fullPrompt = "$($preset.words), $($pal.words), minimalist 2d vector cover design, sleek layout, flat vector art, no text"
    $stem = "s$($preset.id)_p$($pal.id)__$($preset.slug)__$($pal.slug)"

    $reqObj = [ordered]@{
        style_id = $preset.id
        preset = $preset.name
        palette = $pal.name
        palette_colors = [ordered]@{
            primary = $pal.primary
            accent = $pal.accent
            dark = $pal.dark
        }
        method = "POST"
        url = "https://api.cloudflare.com/client/v4/accounts/<account_id>/ai/run/@cf/black-forest-labs/flux-1-schnell"
        headers = [ordered]@{
            "Content-Type" = "application/json"
            "Authorization" = "Bearer <redacted>"
        }
        body = [ordered]@{
            prompt = $fullPrompt
            steps = 8
        }
        local_only_never_sent = [ordered]@{
            target_bucket = @(1216, 832)
            steps_requested = 18
            seed_title = $title
            negative_prompt = $negPrompt
            note = "flux-1-schnell rejects width/height/seed/negative_prompt (error 5006); it always returns a 1024x1024 JPEG which the client center-crops to 1216x832 PNG"
        }
    }

    $jsonStr = $reqObj | ConvertTo-Json -Depth 10
    $outJsonPath = Join-Path $simDir "$($stem)_request.json"
    [System.IO.File]::WriteAllText($outJsonPath, $jsonStr, [System.Text.Encoding]::UTF8)
    Write-Output "Generated simulation request: $outJsonPath"
}
