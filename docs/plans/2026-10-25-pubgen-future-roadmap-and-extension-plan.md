# Architecture & Feature Extension Plan: PubGen Enterprise Publication System

- **Document:** `docs/plans/2026-10-25-pubgen-future-roadmap-and-extension-plan.md`
- **Date:** 2026-10-25
- **Authors:** diorz38 & Antigravity Agent
- **Status:** Strategic Roadmap & Engineering Specification — **implementation status verified 2026-10-30** (each pillar carries a status line; summary in §1.1 and in the §9 milestone table)
- **Target Subsystem:** `crates/ui-egui/src/panels/pubgen/`, `crates/engine/`, `crates/render/`

---

## 1. Executive Summary & Vision

**PubGen** has successfully evolved from a collection of CSV layout scripts into a production-grade automated publishing engine within DesignCraft. It reliably processes complex multi-sheet statistical Excel workbooks, formats bilingual tabular matrices, automates front/inside/back cover generation, builds synchronized Tables of Contents, and translates narrative reviews in-place with strict paragraph style retention.

The next strategic milestone is transforming PubGen into a **unified enterprise publication operating system** tailored for statistical agencies (such as Badan Pusat Statistik / BPS) and institutional reporting.

This document details the architectural specifications for the next seven major pillars:
1. **Native Statistical Infographics & Vector Chart Generator (`C01`, `C02`)**
2. **Multi-Chapter Publication Binder & Project Compiler ("Book" / Batch Mode)**
3. **Statistical Layout Preflight & Quality Assurance (QA Audit)**
4. **BPS Statistical AI Assistant & Official Terminology Memory**
5. **Codebase Modularization (Decomposing `mod.rs` Monolith)**
6. **One-Click Dual Export Pipelines (Print-Ready PDF/X-4 vs. Web Distribution)**
7. **Universal Publication Profiles & General-Purpose Extensibility (Beyond BPS)**

### 1.1 Implementation Status Summary (verified 2026-10-30)

| # | Pillar | Status | Where |
| :--- | :--- | :--- | :--- |
| 1 | Native Statistical Infographics & Vector Chart Engine | **Implemented** 2026-10-29 — vector charting engine in `charts/`, **9** archetypes (clustered, stacked, horizontal, line, area, donut, population pyramid, gauge, GeoJSON choropleth), BPS theme palette binding, interactive dialog, Publication menu | §2 |
| 2 | Multi-Chapter Binder & Project Compiler | **Implemented** 2026-10-27 — `pubgen.compileProject` + the Publication Compiler card | §3 |
| 3 | Statistical Layout Preflight / QA Audit | **Implemented** 2026-10-30 — `preflight.pubgen` merges the generic preflight with `PRF-01/03/04/05/07`; the Pubgen **Audit Publication** dialog lists issues with click-to-select/jump-to-page, a Digital/Print distribution profile, and per-finding triage tags (false positive / ignored) | §4 |
| 4 | BPS Statistical AI Assistant & Terminology Memory | **Implemented except the narrative drafter** 2026-10-27 — glossary/terminology memory, vocabulary collection and AI term drafts ship; the §5.3 narrative summarizer was stripped the same day (see §5.3) | §5 |
| 5 | Codebase Modularization | **Implemented (executed)** 2026-10-25 — flat modules plus a `charts/` directory; `mod.rs` is 505 lines | §6 |
| 6 | One-Click Dual Export Pipelines | **Partial** — the PDF engine has PDF/X-4, CMYK, bleed/marks, bookmarks and hyperlinks; Pubgen print/web presets do not exist | §7 |
| 7 | Universal Publication Profiles | **Implemented** 2026-10-30 — `PubgenProfile` with 4 bundled presets, profile-driven sheet routing / nomenclature / geometry, UI switcher + customizer and compiler-manifest `profile`; custom-profile file import/export and cross-session persistence are missing | §8 |

Legend: **Implemented** = usable end to end with tests; **Implemented except …** = the pillar ships but one named sub-feature does not; **Partial** = the underlying capability exists but the pillar's Pubgen-facing workflow is missing; **Planned** = design only; **Not implemented** = no code.

---

## 2. Pillar 1: Native Statistical Infographics & Vector Chart Engine

> **Status (2026-10-29): Implemented, and ahead of this specification.** `crates/ui-egui/src/panels/pubgen/charts/` holds the engine split as predicted (`spec`, `layout`, `emitter`, `palette`, plus `geojson`, `stock_maps` and `dialog`), with 73 `#[test]` functions. It renders **9** archetypes — `bar_clustered`, `bar_stacked`, `horizontal_bar`, `line`, `area`, `donut`, `population_pyramid`, `gauge` and `map` (GeoJSON choropleth) — so the three added on 2026-10-29/30 go beyond the six below. Category/series data is extracted from Excel `CXX_XX` sheets, mapped into the page margin box, coloured from the chapter DIV theme palette, with rotated X-axis labels, the interactive **Vector Chart** dialog (`pubgen.vectorChart`, Ctrl+Shift+V), and automatic indexing in Daftar Grafik (List of Charts). Map sheets (`MXX_XX`) generate from embedded stock administrative maps, and a decorative map overlay can sit behind the cover text frames and logo.

### 2.1 Problem Statement
In official publications (*Kecamatan Dalam Angka*, *Provinsi Dalam Angka*), visual data summaries appear between the narrative reviews (`RXX`) and data tables (`XX_XX`). Currently, workbooks carry chart data in sheets named `C01`, `C02`, etc. Editors must generate charts externally in Microsoft Excel or Python, export them as static images, and manually place them into the publication.

### 2.2 Functional Architecture
PubGen will introduce a native vector charting engine that compiles Excel chart sheets directly into DesignCraft native vector frames (`designcraft_doc::Item` with `Content::Shape(Path)`):

```
┌────────────────────────────────────────────────────────┐
│ Sheet: C01_01 (Excel)                                  │
│ - Chart Type: bar_clustered / donut / line             │
│ - Title (ID/EN): Penduduk Menurut Desa / Population    │
│ - Data Range: Categories, Series 1 (L), Series 2 (P)   │
└──────────────────────────┬─────────────────────────────┘
                           │ Ingest & Parse
                           ▼
┌────────────────────────────────────────────────────────┐
│ DesignCraft Vector Infographic Generator               │
│ - Coordinate mapping to page margin box                │
│ - Chapter color theme palette application (from DIV)   │
│ - Axis baseline, grid ticks & data value labels        │
│ - Dual-language title & legend text boxes              │
└──────────────────────────┬─────────────────────────────┘
                           │ Emit Native Items
                           ▼
┌────────────────────────────────────────────────────────┐
│ Page Spread: Vector Sharp, Infinitely Scalable Chart   │
└────────────────────────────────────────────────────────┘
```

### 2.3 Supported Chart Archetypes
1. **Vertical Clustered & Stacked Bars:**
   - Population pyramids, age brackets, gender distribution.
   - Automatic label rotation for long subdistrict names.
2. **Horizontal Ranked Bars:**
   - Regional comparisons (e.g., area size or production by subdistrict ranked descending).
3. **Multi-Year Line & Area Trends:**
   - Multi-year economic indicators (inflation, economic growth, poverty rate).
   - High-contrast marker dots and styled stroke rules.
4. **Donut / Radial Breakdown:**
   - Land use percentages, sector contributions to GRDP (*PDRB*).
5. **Population Pyramid (added 2026-10-29):**
   - Age/sex distribution with male on the left, female on the right, a 4-step "nice" axis scale and a gender summary card. (Beyond the original specification.)
6. **Gauge & GeoJSON Choropleth Map (added 2026-10-29):**
   - Single-metric dials for target/percentage indicators; `MXX_XX` sheets render polygons with contrast-adaptive value labels and BPS three-line bilingual titles. (Beyond the original specification.)

### 2.4 Theme & Typography Harmonization
- Colors are automatically bound to the chapter's `DIV` sheet palette (*Dark Teal*, *Warm Ochre*, *Navy Blue*, *Emerald Green*, etc.).
- Axis and legend typography automatically use the document's standard sans-serif font family in regular/semibold cuts.

---

## 3. Pillar 2: Multi-Chapter Binder & Project Compiler ("Book" / Batch Mode)

> **Status (2026-10-27): implemented** — see the full status after §3.3 (the design is [`2026-10-27-pubgen-pillar2-binder-mini-plan.md`](2026-10-27-pubgen-pillar2-binder-mini-plan.md)). The engine also ships generic book support — `book.new/open/add/remove/styleSource/list/paginate/syncStyles/exportPdf` over `.dcbook` in `crates/engine/src/cmd/book.rs` — which gives continuous page numbers and style synchronisation across separate files, but it merges per-document PDFs rather than compiling one publication. The mini-plan deliberately compiles a **single `.designcraft`** (roman front matter, continuous Arabic body, grow-only two-pass TOC) instead of chaining `.dcbook`.

### 3.1 Problem Statement
A complete municipal publication comprises 8 to 12 distinct chapters (`BAB_01_2025.xlsx` through `BAB_10_2025.xlsx`), plus overarching preliminary front matter (Cover, Halaman Judul, Kata Pengantar, Daftar Isi, Daftar Tabel, Daftar Grafik, Penjelasan Teknis). Currently, editors must open and generate each chapter workbook sequentially, then manually merge or balance page counts.

### 3.2 Technical Specification: Project Manifest (`pubgen_project.json`)
Introduce a publication project descriptor that groups chapter workbooks:

```json
{
  "publication_title": "Kecamatan Karawang Timur Dalam Angka 2026",
  "bilingual": true,
  "cover": {
    "style": 11,
    "theme_preset": 0,
    "include_inside": true,
    "include_back": true
  },
  "chapters": [
    { "chapter": 1, "path": "workbooks/BAB_01_2025.xlsx" },
    { "chapter": 2, "path": "workbooks/BAB_02_2025.xlsx" },
    { "chapter": 3, "path": "workbooks/BAB_03_2025.xlsx" }
  ],
  "pagination": {
    "front_matter_format": "roman_lower",
    "body_format": "arabic",
    "restart_at_body": true
  }
}
```

### 3.3 Execution Pipeline
1. **Phase 1: Master Setup**
   - Applies document margins (Top 20.20 mm, Bottom 20 mm, Inside 19 mm, Outside 15 mm).
   - Establishes Parent Pages (Master Spreads) for Left and Right facing pages.
2. **Phase 2: Front Matter Compilation**
   - Generates Front Cover (Spread 0) and Inside Cover (Spread 1).
   - Generates dynamic Front Matter (Kata Pengantar with dynamic signatory placement, Penjelasan Teknis, Daftar Singkatan).
   - Numbers front matter with lowercase Roman numerals (`i`, `ii`, `iii`...).
3. **Phase 3: Sequential Chapter Emission**
   - Processes each `BAB_XX.xlsx` in sequence:
     - Chapter Divider (using specified template or procedural vector fallback).
     - Narrative Review Spreads (`RXX`).
     - Statistical Infographics (`CXX`).
     - Data Tables (`XX_XX`).
   - Maintains continuous Arabic pagination (`1, 2, 3...`) across all chapters.
   - Dynamic running headers update to current chapter title automatically.
4. **Phase 4: Global Master Table of Contents**
   - Compiles consolidated master TOCs:
     - **Daftar Isi / Table of Contents** (listing all front matter and chapter titles).
     - **Daftar Tabel / List of Tables** (aggregating every generated table across all chapters).
     - **Daftar Grafik / List of Charts** (aggregating every generated infographic).
5. **Phase 5: Back Cover Finalization**
   - Generates Back Cover echoing front cover geometry and places it on the final page of the document.

> **Status (2026-10-27): implemented** as `pubgen.compileProject` plus the Publication Compiler card (design: [`2026-10-27-pubgen-pillar2-binder-mini-plan.md`](2026-10-27-pubgen-pillar2-binder-mini-plan.md)). One `pubgen_project.json` compiles covers, front matter, chapters, the three global lists, running headers and the back cover into one `.designcraft` (optional PDF); front matter is lowercase roman and the body restarts at arabic 1. Manifest paths resolve against the manifest folder; the build is skipped for a document already open in a tab; saving is the second-to-last step, so a failure or cancel leaves the previous build untouched. Short form: `sync: true` for the command palette, control channel and scripts; the panel runs one step per frame with a Cancel button.

---

## 4. Pillar 3: Statistical Layout Preflight & Quality Assurance (QA Audit)

> **Status (2026-10-30): implemented** on `feature/pillar3-preflight-audit` as `preflight.pubgen` (`crates/engine/src/cmd/preflight_pubgen.rs`, Window › Output › Pubgen Audit) plus the **Publication ▸ Audit Publication…** menu entry and its modeless dialog (`crates/ui-egui/src/panels/pubgen/audit.rs`, rendered app-wide from `pubgen_dialogs` like every Pubgen dialog, with a button in the panel header band as well). The command merges the generic `preflight.run` checklist with the statistical rules in one result (`{minPpi?} → {errors, warnings, issues}`). `PRF-01` cell overset lives in the generic walk (`crates/engine/src/cmd/preflight.rs` `cellOverset`, attributed to the hosting page frame like `missing_glyphs`); `PRF-02` story overset and `PRF-06` raster DPI were already covered by the generic preflight. The new rules: `PRF-03` scans story paragraphs for `Sumber:`/`Catatan:` note lines with nothing after the colon — note rows are *not* table cells, Pubgen places them as separate text frames with three tab fields (`label ␉ : ␉ content`), and hand-typed single-run lines are read too; `PRF-04` collects `Tabel/Grafik/Gambar NN.NN` title numbers and groups them by (kind, chapter) — a gap or a duplicate (≥3 occurrences, or 2 on different pages; a bilingual pair on one page is not a duplicate) is a warning; `PRF-05` is an engine-side twin of the panel's `detect_page_target` that keeps *empty* frames (an empty English column is the defect) and flags review pages whose English body is empty, a placeholder, or still the Indonesian text; `PRF-07` runs only for `Intent::Print` — a graphic crossing a trim edge by less than 3 mm with document bleed set is an error, and text within 5 mm of the spine (or across it) on facing-page spreads is an error. Clicking an issue row selects the item (`selection.set`) and jumps to its page (`canvas::go_to_page`), like the Preflight panel; the dialog re-runs on demand with an adjustable `min ppi` (default 150). The dialog re-runs itself silently whenever the document revision changed since the last run, so every row names an object that still exists (rows carry `item`+`page`, with a `page_of_item` fallback for parent-page frames). **Distribution profiles** (`2026-10-30`): `profile: "print" | "digital"` on `preflight.pubgen` (absent → the document's `Intent` decides) picks the rule set — Digital (online PDF) drops `PRF-07` and defaults `minPpi` to 150, Print (press) keeps the guards and 300; the result reports which profile ran. **Triage tags**: every unresolved row has a FP/IG cycle button (`false-positive` / `ignored`, keyed by `kind|message` so tags survive re-runs and regeneration); tagged rows dim or hide (checkbox), don't count in the header tally.

### 4.1 Purpose
Before distributing or sending publications to offset printing, editorial teams perform tedious visual checks. The **PubGen Preflight Audit** automates statistical formatting compliance.

### 4.2 Automated Inspection Rules
| Check ID | Name | Description | Severity |
| :--- | :--- | :--- | :--- |
| `PRF-01` | **Cell Overset Detection** | Detects text or numbers truncated within table cells where geometry cannot display full characters. | **Error** (Critical) |
| `PRF-02` | **Story Overset Detection** | Detects narrative text frames where RTF overflowed without an allocated continuation frame. | **Error** (Critical) |
| `PRF-03` | **Missing Source/Notes** | Scans table items for empty `Sumber:` or `Catatan:` rows. | **Warning** |
| `PRF-04` | **Table Number Sequence** | Validates sequential numbering across tables (e.g. flagging if `02.04` is followed by `02.06`, skipping `02.05`). | **Warning** |
| `PRF-05` | **Dual-Lang Parity** | In Dual Lang mode, flags review spreads where the English column is empty or still contains placeholder text. | **Warning** |
| `PRF-06` | **Raster DPI Inspection** | Checks embedded or linked photos/logos; flags assets below 300 DPI for print. | **Warning** |
| `PRF-07` | **Spine & Bleed Guard** | Verifies full-bleed cover elements extend 3 mm past trim margins, and no text sits within 5 mm of the spine. | **Error** |

### 4.3 UI Presentation
A dedicated **"Audit Publication"** button in PubGen opens an interactive checklist:
- Clicking any flagged item immediately selects and centers that frame/page on the canvas for instant remediation.

---

## 5. Pillar 4: BPS Statistical AI Assistant & Official Terminology Memory

### 5.1 Problem Statement
Generic AI models often produce literal translations that contradict standard BPS bilingual statistical nomenclature. For example:
- *Kecamatan* translated as "District" instead of official BPS **"Subdistrict"**.
- *Desa/Kelurahan* translated as "Town" instead of **"Village / Urban Village"**.
- *PDRB* translated literally instead of **"Gross Regional Domestic Product (GRDP)"**.
- *Luas Wilayah* translated as "Region Width" instead of **"Total Area"**.

### 5.2 Official Statistical Glossary Engine (`bps_glossary.json`)
PubGen bundles an authoritative, editable terminology dictionary injected into the system prompt and applied deterministically to whole cells:

```json
{
  "exact_mappings": {
    "Kecamatan": "Subdistrict",
    "Kabupaten": "Regency",
    "Desa": "Village",
    "Kelurahan": "Urban Village",
    "Luas Wilayah": "Total Area",
    "Kepadatan Penduduk": "Population Density",
    "Jenis Kelamin": "Sex",
    "Laki-laki": "Male",
    "Perempuan": "Female",
    "PDRB": "GRDP",
    "Laju Pertumbuhan": "Growth Rate",
    "Tahun": "Year",
    "Sumber": "Source",
    "Catatan": "Note",
    "Keterangan": "Note",
    "jumlah": "Total",
    "total": "Total",
    "provinsi": "Province",
    "propinsi": "Province",
    "ket.": "Note",
    "ket": "Note",
    "kec": "Subdistrict",
    "no": "No"
  }
}
```

> **Status (2026-10-27): implemented** on `feature/pillar4-bps-glossary` (mini-plan `2026-10-25-pubgen-pillar4-bps-ai-assistant-mini-plan.md`). Keys are lowercased at load for case-insensitive whole-cell matching; the file's original casing survives in a sorted `terms` list for the editor and the prompt. Resolution order: `Prefs.pubgenGlossaryPath` → `assets/pubgen/bps_glossary.json` under the working directory → `CARGO_MANIFEST_DIR/../..` (a missing or malformed file degrades to an empty map, never an error). `alias_en()`, `translatable_cell()` and `pair_cells()` take the loaded `BpsGlossary`; the Groq prompt appends `Official terminology you MUST use: id = en; …` capped at 200 terms, and `None`/empty keeps the previous prompt byte-identical. The AI setup dialog gained a **Terminology** tab: search, edit/add/remove, **Import CSV…** (a two-column `Indonesian,English` sheet — comma, semicolon, tab or pipe, header row optional — merged into the list case-insensitively: matching terms take the sheet's English, new ones are appended), **Reset to bundled** (which also clears the saved path so the bundled terms win again), and **Save…** to `bps_glossary.json` (refusing a half-filled row or an empty list, and never writing an empty glossary), which persists the chosen path through `prefs.set` (`pubgenGlossaryPath`) and reloads the in-session glossary. `docs/pubgen-glossary.example.csv` is the sheet to start from; the import only edits the dialog's rows until **Save…** writes them.
>
> **Extends beyond the original §5.2 scope (2026-10-27):** `vocab.rs` (305 lines, 5 tests) adds a **vocabulary collector** — it scans the panel's generated pages for Indonesian terms (whole header cells, row labels, titles, note/source lines, and stopword-delimited phrase runs; never single common words) and seeds the glossary's blank English column, which **Fill AI Drafts** then fills from the model term by term rather than rewriting prose. `ai.rs` carries 27 tests.

### 5.3 Automated Executive Narrative Summarizer (Drafting `ulasan_bab`)
- **Capability:** When generating a chapter from raw data tables without pre-authored RTF narrative files, the AI Assistant analyzes the table figures (e.g., highest population subdistrict, largest agricultural yield) and drafts the introductory chapter review following official BPS analytical phrasing.
- **Strict Formatting Output:** Emits pre-styled paragraphs conforming to `Heading 1`, `Heading 2`, `Lead Paragraph`, and indented body text.

> **Status (2026-10-30): not implemented — removed 2026-10-27.** The deterministic drafter (`pubgen/summarize.rs`), the `OnlineTranslator::complete()` prompt path and the **Draft Review** button all landed on 2026-10-27 and were stripped the same day by `73a3ba33` ("terminology memory — vocab collection, AI drafts, retranslate; strip summarizer"). `designcraft-translate` still exposes `complete(system, user)` (online.rs), but nothing in `ui-egui` calls it, so no chapter narrative is drafted from table figures today. What the pillar gained instead is terminology work, not drafting: `vocab.rs` collects Indonesian terms from generated pages and **Fill AI Drafts** fills the glossary's blank English column (§5.2).

---

## 6. Pillar 5: Codebase Modularization (Decomposing `mod.rs`)

### 6.1 Motivation & Technical Debt
At the time of this plan, [`crates/ui-egui/src/panels/pubgen/mod.rs`](file:///f:/GitHub/designcraft/crates/ui-egui/src/panels/pubgen/mod.rs) exceeded 17,500 lines of Rust code (505 today — see the status line below). It houses UI rendering, state management, Excel conversions, table rendering, 13 cover styles, TOC parsing, and header/footer engines in a single file.

> **Status (2026-10-25): executed; refreshed 2026-10-30.** `mod.rs` became a small panel entrypoint (`show()` + wiring); the flat domain modules are `state`, `helpers`, `csv`, `themes`, `tables`, `pages`, `page_utils`, `toc`, `covers`, `header_footer`, `excel`, `widgets`, `ai`, `locale`, `cover_art`, `divider_art`, plus `testutil` for shared test helpers. The split kept growing with the AI-cover, Pillar 1/2/3 and Pillar 7 work: `ai_cover`, `ai_cover_job`, `thumbnails`, `audit`, `compile`, `profiles`, `vocab`. As of 2026-10-30 `mod.rs` is **505 lines** across **34 files / 35,241 lines**, with **362 `#[test]` functions** (`charts/` alone holds 73). `charts/` did become a directory, as proposed (`spec`, `layout`, `emitter`, `palette`, `geojson`, `stock_maps`, `dialog`); `audit` shipped as the flat `audit.rs` rather than `audit/`, and `covers/`, `toc/`, `header_footer/` remain flat single files. `summarize.rs` was removed (see §5.3). The compile-time A/B re-measurement was still not done — incremental-build benefit is implied by the split, not measured.

### 6.2 Proposed Submodule Architecture
Decompose `crates/ui-egui/src/panels/pubgen/` into decoupled domain modules:

```text
crates/ui-egui/src/panels/pubgen/
├── mod.rs                  // Panel entrypoint, PubgenState struct, top-level layout
├── state.rs                // State defaults, serialization, session cache
├── locale.rs               // Locale definitions (ID, EN, bilingual keywords)
├── excel.rs                // Excel workbook ingestion and CSV matrix conversion
├── covers/
│   ├── mod.rs              // Cover generation orchestration & branding extraction
│   ├── styles.rs           // Cover styles 000–012 definitions & template loaders
│   ├── front.rs            // Front cover generation & slot filling
│   ├── inside.rs           // Inside cover & grayscale transformations
│   └── back.rs             // Back cover generation & layout protection
├── tables/
│   ├── mod.rs              // Table from CSV / matrix orchestration
│   ├── parser.rs           // Condition rows, headers, notes extraction
│   ├── styles.rs           // Alternating row fills, border lines, rules
│   └── themes.rs           // DIV sheet color themes & swatch resolver
├── toc/
│   ├── mod.rs              // TOC UI and generation dispatcher
│   ├── paragraph.rs        // Styled paragraph TOC & bilingual heading pairing
│   ├── tables.rs           // List of Tables builder
│   └── layout.rs           // Dot leaders, right tab stops, hanging indents
├── header_footer/
│   ├── mod.rs              // Header & Footer UI and state
│   ├── layout.rs           // Running header rules, accent bars, margins
│   └── templates.rs        // Custom H&F preset management
├── charts/
│   ├── mod.rs              // Statistical infographic engine
│   └── vector.rs           // Bar, line, donut native vector emitter
├── ai/                     // AI translation provider setup, background jobs, prompts
└── audit/
    ├── mod.rs              // Preflight audit engine
    └── rules.rs            // Overset, DPI, metadata validation rules
```

### 6.3 Benefits
- **Compilation Speed:** Incremental compilation improves significantly because editing a cover style doesn't recompile the table or TOC engine.
- **Safety:** Eliminates git merge conflicts when multiple features are developed concurrently.
- **Maintainability:** Clear unit testing per domain.

---

## 7. Pillar 6: Dual Export & Distribution Pipelines

> **Status (2026-10-30): engine capability implemented, Pubgen one-click workflow not implemented.** The compiler's `ExportPdf` step (`compile.rs`) calls `file.exportPdf` with nothing but a path, so a compiled publication gets the default PDF, not a Print or Web profile. `file.exportPdf` already supports PDF/X-4 (`standard: "x4"`, CMYK output intent, conformance checks reported as warnings), PDF/X-1a:2003, PDF/A-2b, DeviceCMYK and spot inks, bleed boxes with crop/bleed marks (`PdfOptions { bleed, marks }`), image compression, tagged output, and — via `crates/pdf/src/links.rs` — hyperlink annotations plus a bookmark outline (`doc.hyperlinks`, `doc.bookmarks`). File › Export PDF… exposes those as presets (`crates/ui-egui/src/dialogs.rs`). **Missing for this pillar:** Pubgen-level **Print** and **Web** one-click presets (automatic 3 mm bleed extension, 300 DPI floor/downsampling), and a Pubgen builder that turns the generated TOC / list-of-tables / list-of-charts into PDF bookmarks and internal hyperlinks for the compiled publication.

### 7.1 One-Click "Print-Ready" PDF Profile (Offset Printing)
- Conformance to **PDF/X-4 (ISO 15930-7)**.
- RGB to **CMYK color profile conversion** for commercial printing presses.
- Automatic **3 mm bleed extension** and vector crop/trim marks.
- Vector font embedding and 300+ DPI bitmap downsampling.

### 7.2 One-Click "BPS Web Distribution" PDF Profile (Public Portal)
- Compact RGB color space for screen rendering.
- Intelligent JPEG/WebP compression tuned for low-bandwidth rural connections.
- **Interactive Navigation:**
  - Table of Contents entries hyperlinked directly to destination pages.
  - Full hierarchical PDF document bookmarks (Cover > Kata Pengantar > Bab 1 > Tabel 01.01).
- Searchable, selectable text layers with full Unicode support.

---

## 8. Pillar 7: Universal Publication Profiles & General-Purpose Extensibility (Beyond BPS)

> **Status (2026-10-30): implemented**, on `pubgen-0.8.X` (`746ab665` "implement Pillar 7 universal publication profiles and general-purpose extensibility", wired into the new menu layout by `40818e55`).
>
> **Schema.** `PubgenProfile` lives in `crates/ui-egui/src/panels/pubgen/profiles.rs` with `id`, `name`, `description`, `sector` (`bps_official` / `government` / `corporate` / `academic` / `custom`), `geometry`, `structure`, `nomenclature`, `sheet_routing` and `table_styling`. It differs from §8.3 below: the spec's `profile_id` is `id`, `styling` became `table_styling` (column-number row, zebra fills, horizontal-rules-only, default palette) and `bilingual_mode_allowed` was dropped, while `structure` (front matter, roman/arabic numbering, restart at body, chapter dividers, inside/back cover) was added. Values are sanitized on load (`sanitize()`), so a hostile profile cannot produce invalid geometry.
>
> **Presets.** Four bundled JSON files under `assets/pubgen/profiles/` (each with an `ASSETS.md` row, CC0): `bps_dda` (flagship default), `bps_brs`, `corporate_annual`, `academic_monograph` — the §8.4 table's `corporate_annual` and `academic_monograph` shipped; `technical_catalog` did not. `detect_profile_from_sheets` (`excel.rs:175`) picks a profile from sheet names (BRS-style → `bps_brs`, balance-sheet/income-statement/financial → `corporate_annual`, pendahuluan/metodologi/abstrak → `academic_monograph`, else `bps_dda`), and conversion accepts an explicit profile (`convert_excel_to_csv_data_with_profile`).
>
> **Wiring.** Not a decorative selector: sheet routing is parameterized (`is_cover_sheet_with_profile`, narrative/table/chart patterns in `excel.rs`), source/note detection follows the profile's labels (`excel.rs:685`), labels resolve through profile nomenclature with locale fallback (`locale.rs` `chapter_prefix` / `table_prefix` / `chart_prefix` / `toc_title`), page margins come from profile geometry (`page_utils.rs:51`), table options come from `table_styling` (`state.apply_profile` sets the column-number and rule options), and the compiler manifest accepts `"profile": "bps_brs"` (`compile.rs`).
>
> **UI.** Profile switcher in the panel header (`widgets.rs:222`) with a one-line summary, a **Customize Profile…** modal (`dialogs.rs:355`, `pubgen.profileCustomize`) with page size, margins, nomenclature, structure and table-styling fields plus **Apply to this session** / **Reset to BPS DDA**, and the `pubgen.selectProfile` / `pubgen.profileMargins` commands so the control channel and scripts reach the same actions. 6 `#[test]` functions in `profiles.rs`.
>
> **Still missing.** There is no `pubgen_profile.json` file import/export: `ProfileManager::from_json` exists but only parses the bundled presets (`parse_or_fallback`), so a custom profile lives only in `PubgenState::profile_custom` for the session. The active profile is not persisted either — `active_profile_id` has no `Prefs` key, so the choice resets on restart and a compile only sees it through the manifest's `profile`. §8.5's community sharing (`*.pubgen-profile.json`) is therefore unimplemented.

### 8.1 Problem Statement: Overcoming Single-Agency Bias
PubGen’s initial iterations were tailored around BPS (*Badan Pusat Statistik*) requirements:
- Fixed sheet naming expectations (`COVER`, `DIV`, `G00`, `R01`, `01_01`, `C01`).
- Hardcoded Indonesian keywords (`BAB`, `Tabel`, `Grafik`, `Sumber:`, `Catatan:`, `Daftar Isi`).
- Standard BPS layout geometry (Top 20.20 mm, Bottom 20 mm, Inside 19 mm, Outside 15 mm).
- Institutional branding assets (BPS logo, BerAKHLAK, SE2026).

While highly effective for BPS teams, general-purpose enterprise users (corporations, banks, universities, research NGOs like the UN, World Bank, and municipal agencies) require different section nomenclature, table styles, and design languages.

### 8.2 Architectural Solution: Decoupled Publication Profiles (`pubgen_profile.json`)
Decouple the **core algorithmic engine** (matrix parsing, hierarchical headers, vector layout, TOC building, AI drafting) from the **domain profile configuration**:

```text
┌────────────────────────────────────────────────────────┐
│                   PUBGEN CORE ENGINE                   │
│ (Excel Parser, Vector Layout, Tables, TOC, AI Writer)  │
└──────────────────────────┬─────────────────────────────┘
                           │ Configured by
                           ▼
┌────────────────────────────────────────────────────────┐
│             PUBLICATION PROFILE SELECTOR               │
│                                                        │
│  [ Preset: Corporate Annual Report ▼ ]                 │
│  ───────────────────────────────────────────────────   │
│  • BPS Statistical Publication (Official KDA / BDA)    │
│  • Corporate Annual Report (Financial / Enterprise)    │
│  • Academic & Research Monograph (Universities / NGOs) │
│  • Technical / Product Specification Catalog           │
│  • Custom User Profile (*.json / *.toml)               │
└────────────────────────────────────────────────────────┘
```

### 8.3 Profile Schema Specification (`PubgenProfile`)
Each profile defines:

```json
{
  "profile_id": "corporate_annual_report",
  "name": "Corporate Annual Report",
  "description": "Clean modern layout for enterprise annual & quarterly reports with executive summaries, KPI stat cards, and financial statements.",
  "nomenclature": {
    "chapter_prefix": "Section",
    "table_prefix": "Table",
    "chart_prefix": "Figure",
    "toc_title": "Table of Contents",
    "toc_tables_title": "List of Tables",
    "toc_charts_title": "List of Figures",
    "source_keywords": ["Source:", "Data:", "Reference:"],
    "note_keywords": ["Note:", "Notes:", "Disclaimer:"],
    "subtotal_keywords": ["Total", "Subtotal", "Consolidated", "Net"]
  },
  "sheet_routing": {
    "cover_patterns": ["(?i)^cover$", "(?i)^front_cover$"],
    "divider_patterns": ["(?i)^div.*", "(?i)^section.*"],
    "narrative_patterns": ["(?i)^exec_summary.*", "(?i)^narrative.*", "(?i)^r\\d+.*"],
    "table_patterns": ["(?i)^\\d+[_.]\\d+.*", "(?i)^table.*", "(?i)^fin_.*"],
    "chart_patterns": ["(?i)^c\\d+.*", "(?i)^chart.*", "(?i)^figure.*"]
  },
  "geometry": {
    "page_size": "A4",
    "facing_pages": true,
    "margins": {
      "top_mm": 25.0,
      "bottom_mm": 25.0,
      "inside_mm": 25.0,
      "outside_mm": 20.0
    }
  },
  "styling": {
    "table_style": "minimal_rules",
    "default_palette": "corporate_navy",
    "header_numbering_row": false,
    "bilingual_mode_allowed": false
  }
}
```

### 8.4 Built-in Out-of-the-Box Profiles

| Profile ID | Target Sector | Key Characteristics |
| :--- | :--- | :--- |
| **`bps_statistical`** *(Flagship Default)* | Statistical Offices (BPS) | Bilingual ID/EN, standard 20 mm margins, DIV theme palettes, column numbering row `(1) (2) (3)`, BPS branding slots. |
| **`corporate_annual`** | Public Companies, Banks, Startups | Sleek minimalist borders, KPI stat callout boxes, executive letter spreads, quarterly breakdown tables, English/monolingual or bilingual options. |
| **`academic_monograph`** | Universities, Research Think Tanks, NGOs (UN, World Bank) | Strict APA/Chicago manual table formatting, clean monochrome/navy rules, numbered equations, formal author/institution headers. |
| **`technical_catalog`** | Manufacturers, Retailers, Hardware vendors | Multi-column product specification grids, alternating row zebra striping, SKU & pricing column emphasis. |

### 8.5 User-Defined Profiles & Community Sharing
- **Profile Manager in PubGen:** A dropdown at the top of the Pubgen panel to switch profiles with one click.
- **Import / Export Custom Profile:** Users can export their custom settings to a `.pubgen-profile.json` file and share it across their organization, allowing any company or department to enforce their brand layout instantly.

---

## 9. Implementation Roadmap & Milestones

| Milestone | Deliverables | Estimated Effort | Status (verified 2026-10-30) |
| :--- | :--- | :--- | :--- |
| **M1: Modularization** | Split `mod.rs` into submodules (`covers/`, `tables/`, `toc/`, `header_footer/`). Zero functional regressions. | 2–3 days | **✅ implemented 2026-10-25** — flat modules, not directories; `charts/` did become a directory later, `audit` stayed flat; no compile-time measurement (see §6) |
| **M2: Preflight QA Audit** | Build `audit/` engine checking cell overset, missing sources, and image DPI. Add Audit button to UI. | 3–4 days | **✅ implemented 2026-10-30** — `preflight.pubgen` + the Audit Publication dialog, print/digital profiles, triage tags (see §4) |
| **M3: Glossary Memory** | Implement `bps_glossary.json` injection into AI translation system prompt with user UI override editor. | 2 days | **✅ implemented 2026-10-27**, extended with vocabulary collection and Fill AI Drafts (see §5) |
| **M4: Vector Chart Engine** | Implement native vector bar/line/donut chart rendering from `CXX` Excel sheets. | 5–7 days | **✅ implemented 2026-10-29** — 9 vector archetypes including pyramid, gauge and GeoJSON maps (see §2) |
| **M5: Batch Multi-Chapter Binder** | Implement project manifest and one-click compiler for entire `BAB_01`..`BAB_10` workbook directories. | 5–6 days | **✅ implemented 2026-10-27** — `pubgen.compileProject` plus the Publication Compiler card; a manual two-workbook check is pending (see §3) |
| **M6: Dual PDF Profiles** | Integrate one-click PDF/X-4 (print) and compact hyperlinked PDF (web) export presets. | 3–4 days | **⏳ partial** — engine supports PDF/X-4, CMYK, marks, bookmarks, hyperlinks, and the compiler has an `ExportPdf` step; the Pubgen presets and TOC-derived links/bookmarks are missing (see §7) |
| **M7: Universal Profiles** | Decouple BPS-specific keywords into `PubgenProfile` schema; ship built-in profiles (Corporate, Academic, Catalog) + custom profile import/export. | 4–5 days | **✅ implemented 2026-10-30**, minus `technical_catalog` and custom-profile import/export/persistence (see §8) |

---

## 10. Conclusion

Implementing this architectural roadmap elevates PubGen from a specialized municipal layout assistant to a world-class, multi-domain desktop publishing platform capable of delivering publication-grade output for governmental, institutional, academic, and enterprise organizations worldwide.

As of 2026-10-30, five of the seven pillars are complete end to end — the vector chart engine (now nine archetypes), the Multi-Chapter Binder compiler, the statistical preflight audit, modularization, and Universal Publication Profiles — and a sixth, the AI assistant, ships its terminology half (glossary, vocabulary collection, AI term drafts) without the narrative drafter it briefly had. Only **Pillar 6**, the Pubgen-level Print/Web export presets with TOC-derived bookmarks and hyperlinks, is still partial, and it sits on a PDF engine that already implements PDF/X-4, CMYK, bleed and marks. The remaining work is incremental on existing foundations rather than greenfield.

