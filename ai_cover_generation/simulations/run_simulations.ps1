# Live simulation of Pubgen Publication Cover AI generation
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing

$repoRoot = "E:\Github\designcraft-pubgen-0.7.X"
$simDir = Join-Path $repoRoot "ai_cover_generation\simulations"
if (!(Test-Path $simDir)) {
    New-Item -ItemType Directory -Path $simDir | Out-Null
}

# 1. Read Cloudflare credentials from .env
$envFile = Join-Path $repoRoot ".env"
$acct = ""
$tok = ""
if (Test-Path $envFile) {
    foreach ($line in Get-Content $envFile) {
        if ($line -match "^\s*CLOUDFLARE_ACCOUNT_ID\s*=\s*(.*)$") { $acct = $matches[1].Trim() }
        if ($line -match "^\s*CLOUDFLARE_API_TOKEN\s*=\s*(.*)$") { $tok = $matches[1].Trim() }
    }
}

if ([string]::IsNullOrWhiteSpace($acct) -or [string]::IsNullOrWhiteSpace($tok)) {
    throw "Cloudflare credentials not found in $envFile"
}

Write-Output "Using Cloudflare Account: $acct, Token: $($tok.Substring(0, 10))..."

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

# 1 random color scheme per preset
$chosenPalettes = @(1, 4, 0, 5, 3)

$title = "KARAWANG TIMUR"
$negPrompt = "text, typography, letters, words, numbers, fonts, lorem ipsum, dummy copy, watermarks, logo, icons, symbols, realistic photo, photograph, 3d render, perspective view, angled mockup, dropshadow, realistic book mockup, brochure mockup, folded paper, standing card, desk, table, presentation board, collage, multi-panel, split-screen, seam, divider line, gutter, picture frame, framing, borders, margins, textured paper, checkerboard, transparent grid, grey and white grid, photo placeholder, template, booklet, brochure layout, document template, uniform shape size, equal sized shapes, repetitive tiling, monotonous pattern, wallpaper texture, liquid marble, fluid acrylic, paint swirls, dense ripples, messy zebra stripes, clipping artifacts, busy details, author name, signature"

$url = "https://api.cloudflare.com/client/v4/accounts/$acct/ai/run/@cf/black-forest-labs/flux-1-schnell"
$headers = @{
    "Authorization" = "Bearer $tok"
    "Content-Type" = "application/json"
}

$results = @()

for ($i = 0; $i -lt $presets.Count; $i++) {
    $preset = $presets[$i]
    $pal = $palettes[$chosenPalettes[$i]]
    $stem = "s$($preset.id)_p$($pal.id)__$($preset.slug)__$($pal.slug)"

    $prompt = "$($preset.words), $($pal.words), minimalist 2d vector cover design, sleek layout, flat vector art, no text"

    $reqPayload = [ordered]@{
        prompt = $prompt
        steps = 8
    }
    $reqBodyJson = $reqPayload | ConvertTo-Json -Compress

    $docJson = [ordered]@{
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
        body = $reqPayload
        local_only_never_sent = [ordered]@{
            negative_prompt = $negPrompt
            seed_title = $title
            steps_requested = 18
            target_bucket = @(1216, 832)
            note = "flux-1-schnell rejects width/height/seed/negative_prompt (error 5006); it always returns a 1024x1024 JPEG which the client center-crops to 1216x832 PNG"
        }
    }

    $reqJsonFile = Join-Path $simDir "$($stem)_request.json"
    [System.IO.File]::WriteAllText($reqJsonFile, ($docJson | ConvertTo-Json -Depth 10), [System.Text.Encoding]::UTF8)

    Write-Output "[$($i+1)/$($presets.Count)] Generating Style $($preset.id) ($($preset.name)) with Palette $($pal.name)..."

    $rawBytes = $null
    $maxAttempts = 3
    for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
        try {
            $resp = Invoke-RestMethod -Uri $url -Method Post -Headers $headers -Body $reqBodyJson -TimeoutSec 120
            if ($resp.result -and $resp.result.image) {
                $rawBytes = [System.Convert]::FromBase64String($resp.result.image)
                break
            } else {
                throw "No result.image in Cloudflare response"
            }
        } catch {
            $errMsg = $_.Exception.Message
            if ($_.ErrorDetails) { $errMsg += " Details: " + $_.ErrorDetails.Message }
            Write-Warning "Attempt $attempt failed: $errMsg"
            if ($attempt -lt $maxAttempts) {
                Start-Sleep -Seconds 2
            } else {
                $results += [ordered]@{
                    style_id = $preset.id
                    preset = $preset.name
                    palette = $pal.name
                    ok = $false
                    error = $errMsg
                }
            }
        }
    }

    if ($rawBytes) {
        # Save raw 1024x1024 JPEG
        $rawFile = Join-Path $simDir "$($stem)_raw.jpg"
        [System.IO.File]::WriteAllBytes($rawFile, $rawBytes)

        # Center crop to 1216:832 ratio and resize to 1216x832 PNG (matching pipeline)
        $pngFile = Join-Path $simDir "$($stem).png"
        $ms = New-Object System.IO.MemoryStream(,$rawBytes)
        $srcBmp = [System.Drawing.Image]::FromStream($ms)
        $sw = $srcBmp.Width
        $sh = $srcBmp.Height

        $targetW = 1216
        $targetH = 832
        $targetRatio = [double]$targetW / [double]$targetH
        $srcRatio = [double]$sw / [double]$sh

        if ($srcRatio -gt $targetRatio) {
            $cw = [int]([double]$sh * $targetRatio)
            $ch = $sh
        } else {
            $cw = $sw
            $ch = [int]([double]$sw / $targetRatio)
        }
        $x = [int](($sw - $cw) / 2)
        $y = [int](($sh - $ch) / 2)

        $cropRect = New-Object System.Drawing.Rectangle($x, $y, $cw, $ch)
        $croppedBmp = New-Object System.Drawing.Bitmap($cw, $ch)
        $gCrop = [System.Drawing.Graphics]::FromImage($croppedBmp)
        $gCrop.DrawImage($srcBmp, (New-Object System.Drawing.Rectangle(0, 0, $cw, $ch)), $cropRect, [System.Drawing.GraphicsUnit]::Pixel)
        $gCrop.Dispose()

        $destBmp = New-Object System.Drawing.Bitmap($targetW, $targetH)
        $gDest = [System.Drawing.Graphics]::FromImage($destBmp)
        $gDest.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $gDest.DrawImage($croppedBmp, 0, 0, $targetW, $targetH)
        $gDest.Dispose()

        $destBmp.Save($pngFile, [System.Drawing.Imaging.ImageFormat]::Png)

        $destBmp.Dispose()
        $croppedBmp.Dispose()
        $srcBmp.Dispose()
        $ms.Dispose()

        Write-Output "  -> Saved raw JPEG: $($rawBytes.Length) bytes ($($rawFile | Split-Path -Leaf))"
        Write-Output "  -> Saved pipeline PNG: 1216x832 ($($pngFile | Split-Path -Leaf))"

        $results += [ordered]@{
            style_id = $preset.id
            preset = $preset.name
            palette = $pal.name
            ok = $true
            prompt = $prompt
            steps_sent = 8
            raw = [System.IO.Path]::GetFileName($rawFile)
            png = [System.IO.Path]::GetFileName($pngFile)
        }
    }

    # Respectful pause between calls
    Start-Sleep -Seconds 1
}

$manifest = [ordered]@{
    model = "@cf/black-forest-labs/flux-1-schnell"
    endpoint = "POST /client/v4/accounts/{account_id}/ai/run/@cf/black-forest-labs/flux-1-schnell"
    generated_unix = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    combos = $results.Count
    succeeded = ($results | Where-Object { $_.ok -eq $true }).Count
    request_contract = [ordered]@{
        prompt = "<style words>, <palette words>, minimalist 2d vector cover design, sleek layout, flat vector art, no text"
        steps = 8
    }
    results = $results
}

$manifestFile = Join-Path $simDir "manifest.json"
[System.IO.File]::WriteAllText($manifestFile, ($manifest | ConvertTo-Json -Depth 10), [System.Text.Encoding]::UTF8)
Write-Output "Simulation complete! Manifest written to $manifestFile"
