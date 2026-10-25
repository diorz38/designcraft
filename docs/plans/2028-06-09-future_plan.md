# Future Plan: PubGen Standalone Evolution & Separation from DesignCraft

- **Document:** `docs/plans/2028-06-09-future_plan.md`
- **Target Date / Horizon:** 2028-06-09
- **Authors:** diorz38 & Antigravity Agent
- **Target Architecture:** Standalone Publication Generator ("PubCraft" / "PubGen Standalone")
- **Status:** Strategic Long-Term Architecture & Spin-Off Blueprint

---

## 1. Executive Summary & Strategic Rationale

As **PubGen** matures into a comprehensive automated publication operating system—capable of ingesting multi-sheet statistical workbooks, orchestrating complex tables, generating native vector infographics, drafting grounded AI narrative reviews, and managing bilingual layouts—its operational requirements diverge significantly from traditional manual graphic-design software.

**DesignCraft** is built to compete with Adobe InDesign, offering interactive drawing tools, scissors, freeform vector path manipulation, complex color management policies, EPUB reflowable readers, and dozens of manual floating palettes.

For an automated publishing pipeline, **70% of those manual tools represent unnecessary overhead, binary bloat, and cognitive friction for non-designer users.**

By **2028-06-09**, when PubGen reaches full functional maturity, this plan details the spin-off and decoupling of PubGen into an independent, hyper-focused publishing engine: **PubGen Standalone** (or **PubCraft**).

---

## 2. Decoupling: What Gets Stripped vs. What Gets Kept

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        DESIGNCRAFT WORKSPACE                           │
├───────────────────────────────────┬────────────────────────────────────┤
│   STRIP OUT (Manual Design)       │   KEEP & CARRY OVER (Core Engine)  │
├───────────────────────────────────┼────────────────────────────────────┤
│ • Pen, Pencil, Scissors Tools     │ • HarfBuzz Font Shaping (fonts)    │
│ • Bezier Anchor Point Editors     │ • Text Line Breaking & Hyphenation │
│ • EPUB / PPTX / IDML Interchange  │ • Optical Margin Alignment         │
│ • Audio / Video / PDF Forms       │ • Native Vector PDF/X-4 Writer     │
│ • 30+ Floating Panels (Glyphs,    │ • Pure Data Document Model (doc)   │
│   Swatches, Trapping, Flattener)  │ • Multi-Sheet Excel Matrix Parser  │
│ • Interactive Freehand Canvas UI  │ • Vector Infographics Engine       │
│                                   │ • BPS Glossary & AI Summarizer     │
└───────────────────────────────────┴────────────────────────────────────┘
```

### 2.1 What Gets Stripped Out
- **Manual Vector Drafting:** Pen tool, Scissors, Free Transform, Bezier anchor manipulation, Gradient mesh.
- **Legacy Interchange Converters:** Reflowable EPUB export, PowerPoint (`.pptx`) round-tripping, InDesign IDML complex schema mapping.
- **Interactive Multimedia:** Video, sound, digital interactive buttons, and PDF form fields.
- **Manual Control Clutter:** Floating panels for Glyphs, Trapping Presets, Color Management Policies, and Object States.

### 2.2 What Gets Kept (The Publishing Core)
- **`crates/doc`**: High-speed, pure data document tree (Spreads, Pages, Items, Stories, Tables).
- **`crates/compose`**: Text composition with HarfBuzz font shaping, multi-language hyphenation, and optical margin alignment.
- **`crates/pdf`**: Razor-sharp, vector-pure PDF/X-4 print compiler with CMYK/RGB profile support.
- **`crates/render`**: Low-latency Pixmap canvas renderer for instant page previews.
- **`crates/pubgen`**: Dedicated crate housing Excel matrix conversion, vector charts, table formatters, cover engines, and AI pipelines.

### 2.3 The Impact on Performance & Footprint
- **Binary Footprint:** Shrinks from ~85 MB to **< 15–20 MB**.
- **Cold Startup Time:** Drops from 350 ms to **under 50 milliseconds**.
- **Memory Footprint:** Operates smoothly under 40 MB RAM for 200-page publications.

---

## 3. The Three Operational Formats

Spinning off from DesignCraft unlocks three distinct execution environments tailored for enterprises and statistical agencies:

### 3.1 Format A: PubGen Studio (Zero-Clutter Desktop GUI)
A sleek, streamlined native desktop app built for statistical officers, accountants, and non-designers:
- **Single-Purpose Workflow Interface:**
  ```text
  [ 1. Select Excel File ] ──> [ 2. Pick Profile & Cover ] ──> [ 3. Export PDF ]
  ```
- **Interactive Live Spread Viewer:** Beautiful, distraction-free spread preview showing the generated publication with zoom and page flipping.
- **Zero Training Required:** Anyone who can use Excel can generate a professional 100-page publication in 3 clicks.

### 3.2 Format B: PubGen CLI (`pubgen-cli` for Server / Cloud CI/CD)
A headless, high-throughput command-line binary that runs in Docker, Linux servers, or cloud microservices:
- **Mass Batch Production:** Generate all 514 regional publications overnight without human interaction:
  ```bash
  pubgen-cli batch --input /data/2028_census/ --profile bps_statistical --out /dist/publications/
  ```
- **Automated Verification:** Exit code `0` on clean compilation; non-zero with diagnostic JSON on layout preflight failure (overset text or missing sources).

### 3.3 Format C: PubGen Web Portal (WASM In-Browser Publication Engine)
Leveraging DesignCraft's existing Rust WebAssembly compatibility:
- Officers drag-and-drop their Excel workbook directly into a web browser on the intranet portal.
- The publication is compiled and rendered **entirely on the client-side via WASM**, producing publication-grade PDFs with zero server compute costs and zero data transmission over public networks.

---

## 4. Multi-Domain General-Purpose Publication Profiles

PubGen Standalone completely decouples from single-agency BPS assumptions while keeping BPS as its flagship profile:

| Profile Identifier | Target Organization | Layout & Design Archetype |
| :--- | :--- | :--- |
| **`bps_statistical`** | Municipal & National Statistical Agencies | Bilingual ID/EN, standard 20 mm margins, DIV palettes, column numbers `(1) (2) (3)`, official branding slots. |
| **`corporate_annual`** | Enterprises, Banks, Public Companies | Sleek modern rules, executive letter spread, KPI metric cards, quarterly financial tables. |
| **`academic_monograph`** | Universities, Research Institutes, Think Tanks (UN, World Bank) | Strict APA/Chicago table formatting, clean monochrome/navy rules, numbered equations, formal author/institution headers. |
| **`technical_catalog`** | Manufacturers, Retailers, Hardware vendors | Product specification grids, alternating row zebra striping, SKU & pricing column emphasis. |
| **`custom_profile`** | Any Organization | Defined via portable `.pubgen-profile.json` sharing brand swatches, fonts, and nomenclature. |

---

## 5. Phased Transition Strategy (Roadmap to 2028-06-09)

```text
2026 Q4 – 2027 Q2               2027 Q3 – 2027 Q4              2028 Q1 – 2028-06-09
┌─────────────────────────┐     ┌────────────────────────┐     ┌─────────────────────────┐
│ Phase 1: Feature Maturing│ ──> │ Phase 2: Core Isolation│ ──> │ Phase 3: The Spin-Off   │
│ • Modularize mod.rs     │     │ • Extract crates/pubgen│     │ • New repo / binary:    │
│ • Vector Chart Engine   │     │ • Zero dependency on   │     │   PubCraft / PubGen     │
│ • Multi-Chapter Binder  │     │   manual tools         │     │ • Standalone Studio GUI │
│ • BPS AI Terminology    │     │ • Headless CLI runner  │     │ • Headless Docker CLI   │
└─────────────────────────┘     └────────────────────────┘     └─────────────────────────┘
```

1. **Phase 1: In-Place Maturation (Current Repository)**
   - Complete the feature roadmap in `docs/plans/2026-10-25-pubgen-future-roadmap-and-extension-plan.md`.
   - Mature vector charts, batch compilation, preflight QA, and multi-domain profiles inside `designcraft`.
2. **Phase 2: Architectural Decoupling (`crates/pubgen`)**
   - Extract PubGen logic from `crates/ui-egui/src/panels/pubgen/` into a dedicated workspace crate `crates/pubgen`.
   - Ensure `crates/pubgen` depends solely on `crates/doc`, `crates/compose`, and `crates/pdf`.
3. **Phase 3: The Standalone Spin-Off (Horizon: 2028-06-09)**
   - Create the dedicated project repository (`storytold/pubcraft` or `diorz38/pubgen`).
   - Package standalone distributions for Windows x64, macOS, Linux, and Docker.
   - Establish PubGen as the premier open-source automated publishing compiler in the world.

---

## 6. Conclusion

Separating PubGen from DesignCraft at its maturity stage is not only technically viable—it represents the natural, ultimate evolution of the product. By stripping away unused manual design tools and focusing 100% on automated, high-precision document synthesis, PubGen becomes the fastest, lightest, and most elegant automated publication compiler available.
