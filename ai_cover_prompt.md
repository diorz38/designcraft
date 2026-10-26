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

## Recommended Generation Settings
| Setting | Value |
| --- | --- |
| **Model** | SDXL 1.0 Base (or SDXL Lightning / Turbo) |
| **Dimensions** | **1216 × 832** (3:2) |
| **Steps** | 18 – 30 |
| **CFG Scale** | 7.0 – 8.0 |

flat vector 2D with modern geometric business layout. Color palette of deep teal green, dark pine green, chartreuse lime green gradient, mint turquoise, and crisp white. Geometric shapes featuring large angled diagonal thick rounded stripes across, floating rounded hexagons, thick teal hexagon border frames containing a transparent photo placeholder with a grey and white checkerboard pattern grid, gradient color overlays, clean blank negative copy space, sharp crisp vector edges, flat graphic design, print-ready document template, no realistic photo elements