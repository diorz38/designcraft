# AI Cover Art Prompt Specifications (SDXL 3:2 Double A4 Landscape)

To prevent SDXL from generating 3D tabletop mockups, folded brochures, collages, frames, or visible seam dividers, prompts use **pure style keywords** (no `booklet`, `cover`, `template`, `front cover`, or `back cover`) and explicitly enforce **Aspect Ratio 3:2**, **Width 1216**, and **Height 832** with full-bleed edge-to-edge continuous composition.

When DesignCraft places the 1216×832 image into the publication pages:
- **Front Cover (main cover)**: Aligns to the **right edge** (`fit_align = 5`), cropping the right A4 half.
- **Back Cover**: Aligns to the **left edge** (`fit_align = 3`), cropping the left A4 half.
- **Inside Cover (2nd cover)**: Renders the background in **light gray-scale** (values $\ge 210$) with solid **Black** text for readability.

---

## SDXL Optimal Resolution
* **Width:** `1216`
* **Height:** `832`
* **Aspect Ratio:** `3:2` (Native SDXL bucket resolution—generates continuous full-bleed widescreen art without seam artifacts or distortion)

---

## Universal Negative Prompt (Anti-Mockup & Anti-Seam)
```text
text, typography, letters, words, numbers, fonts, lorem ipsum, dummy copy, watermarks, logo, icons, symbols, realistic photo, photograph, 3d render, perspective view, angled mockup, dropshadow, realistic book mockup, brochure mockup, folded paper, standing card, desk, table, presentation board, collage, multi-panel, split-screen, seam, divider line, gutter, picture frame, framing, borders, margins, textured paper, checkerboard, transparent grid, clipping artifacts, busy details, author name, signature
```

---

## Style 1: Dynamic Waves
### Positive Prompt
```text
abstract flowing dynamic wave ribbons, smooth curved gradient shapes, dark navy blue #0A1931, deep royal blue #003399, vibrant electric cyan blue #00A8FF, soft silver-gray #E0E6ED, clean solid white background, flat 2D graphic design, minimalist corporate aesthetic, width 1216 height 832, aspect ratio 3:2 landscape, wide panoramic wallpaper, full bleed edge to edge, continuous seamless artwork, generous blank copy space for typography, high resolution vector illustration
```

---

## Style 2: Vibrant S-Curve

### Positive Prompt
```text
flat vector graphic, sweeping curved bands, vibrant wavy orange-to-yellow gradient ribbon swooping across the composition in an elegant S-curve, deep royal blue background, curving lime-green field with diagonal textured pattern, soft light-blue-to-white gradient curve, 2D graphic design, smooth gradients, sharp clean edges, width 1216 height 832, aspect ratio 3:2 landscape, wide panoramic wallpaper, full bleed edge to edge, continuous seamless artwork, generous blank copy space for typography, high resolution
```

---

## Recommended Generation Settings — Flux (current, validated 2026-10-27)
| Setting | Value |
| --- | --- |
| **Model** | `@cf/black-forest-labs/flux-1-schnell` (Cloudflare Workers AI) |
| **Params** | `prompt`, `steps: 4` only — endpoint rejects `width`/`height` (schema error 5006) and extra properties |
| **Response** | JSON `{"result":{"image":"<base64 JPEG>"}}` — decode before saving, **not** raw bytes |
| **Native size** | fixed 1024 × 1024 |
| **Cover size** | center-crop to 1023 × 700 (exact 1216:832 ratio) → high-quality resize **1216 × 832** |

Flux prompt template (all AI Base styles + md styles):
```text
Minimalist 2d vector cover design, {style words}, {palette words}, sleek layout, flat vector art, no text
```
Drop the SDXL tail (`width 1216 height 832, aspect ratio 3:2, full bleed edge to edge…`) — meaningless for the fixed-size endpoint. Flux ignores the mockup triggers (`cover`, `template`) that derail SDXL, and renders text-free flat vector reliably without a negative prompt. Validated outputs: `ai_cover_generation/*_flux_raw.jpg` + `*_1216x832.png`.

### SDXL fallback (legacy)
| Setting | Value |
| --- | --- |
| **Model** | SDXL 1.0 Base |
| **Dimensions** | **1216 × 832** (3:2) |
| **Steps** | ≤ 20 (server caps `num_steps`, error 5006 rejects > 20 despite md's earlier 18–30) |
| **CFG Scale** | 7.0 – 8.0 |
| **Response** | raw PNG bytes |


flat vector 2D with modern geometric business layout. Color palette of deep teal green, dark pine green, chartreuse lime green gradient, mint turquoise, and crisp white. Geometric shapes featuring large angled diagonal thick rounded stripes across, floating rounded hexagons, thick teal hexagon border frames containing a transparent photo placeholder with a grey and white checkerboard pattern grid, gradient color overlays, clean blank negative copy space, sharp crisp vector edges, flat graphic design, print-ready document template, no realistic photo elements

## Style 3
The issue isn't the model itself, but how Stable Diffusion XL (SDXL) interprets design-related prompt keywords:
 * SDXL interprets "book cover" literally as photography: SDXL's training data associates "book cover" or "publication" with photos of actual physical books (showing shadows, paper textures, binding, and spine angles), which distorts crisp vector shapes into fuzzy textures.
 * Abstract terms cause mud: Words like "governmental," "professional," or "official" don't give SDXL concrete visual shapes, so it defaults to muddy gradient clouds or chaotic textures.
 * Text generation interference: SDXL tries (and fails) to parse room for typography, often generating smudged graphic shapes where text usually sits.
How to Fix Your Prompt
To get precise geometric waves, clean lines, and corporate graphics, use vector graphic and UI design terminology instead of photographic/book terms.
Key Keywords to Include
 * Design style: flat vector art, graphic design layout, corporate visual identity, minimalist graphic cover
 * Shape descriptions: flowing wave lines, abstract geometric curves, overlapping dynamic gradient swooshes, clean layout
 * Composition: isolated on white background, 2d flat render, vector illustrator style
Key Words to Avoid / Put in Negative Prompt
 * Negative Prompt: photograph, 3d render, realistic, mockup, texture, noise, text, typography, blur, chaotic, realistic book photo
Prompt Example Templates
Option A: Modern Government / Corporate Annual Report
> Positive: Flat vector graphic cover design, corporate official report theme, dynamic flowing wave lines, blue and navy modern geometric swooshes, minimalist layout, vector illustration, sharp clean lines, white background, high contrast, 2d graphic design
> Negative: text, letters, photo, realistic paper texture, mockup, 3d, dark shadows, blur, noise
> 
Option B: Clean Geometric / Modern Public Policy
> Positive: Minimalist 2d vector cover design, professional government publication template, crisp intersecting curved wave lines, soft gradient overlay, cyan and charcoal gray graphic elements, sleek layout, vector art
> Negative: typography, title, photograph, canvas texture, fuzzy edges, distorted shapes
> 
Parameter Adjustments in Cloudflare Workers AI
When calling @cf/stabilityai/stable-diffusion-xl-base-1.0 via API, make sure your parameters enforce high graphic clarity:
const response = await env.AI.run(
  "@cf/stabilityai/stable-diffusion-xl-base-1.0",
  {
    prompt: "Flat 2d vector cover layout, blue flowing graphic wave lines, professional corporate presentation background, minimalist clean geometry, sharp lines",
    negative_prompt: "photograph, mockup, text, 3d, noisy, fuzzy, textured",
    num_steps: 30,      // Keeps lines sharp and defined
    guidance_scale: 7.5  // Forces strict adherence to vector keywords
  }
);

If you need even sharper, cleaner graphic lines, consider switching your model endpoint to @cf/black-forest-labs/flux-1-schnell, which handles flat vector layout aesthetics and geometry significantly better than standard SDXL.