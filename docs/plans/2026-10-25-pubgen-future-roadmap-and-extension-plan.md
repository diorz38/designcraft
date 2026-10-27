# Architecture & Feature Extension Plan: PubGen Enterprise Publication System

- **Document:** `docs/plans/2026-10-25-pubgen-future-roadmap-and-extension-plan.md`
- **Date:** 2026-10-25
- **Authors:** diorz38 & Antigravity Agent
- **Status:** Strategic Roadmap & Engineering Specification
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

---

## 2. Pillar 1: Native Statistical Infographics & Vector Chart Engine

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

### 2.4 Theme & Typography Harmonization
- Colors are automatically bound to the chapter's `DIV` sheet palette (*Dark Teal*, *Warm Ochre*, *Navy Blue*, *Emerald Green*, etc.).
- Axis and legend typography automatically use the document's standard sans-serif font family in regular/semibold cuts.

---

## 3. Pillar 2: Multi-Chapter Binder & Project Compiler ("Book" / Batch Mode)

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

### 5.3 Automated Executive Narrative Summarizer (Drafting `ulasan_bab`)
- **Capability:** When generating a chapter from raw data tables without pre-authored RTF narrative files, the AI Assistant analyzes the table figures (e.g., highest population subdistrict, largest agricultural yield) and drafts the introductory chapter review following official BPS analytical phrasing.
- **Strict Formatting Output:** Emits pre-styled paragraphs conforming to `Heading 1`, `Heading 2`, `Lead Paragraph`, and indented body text.

> **Status (2026-10-27): implemented**, split into a deterministic core and the model call:
> - **Deterministic (`pubgen/summarize.rs`):** resolves each catalog row's table CSV, dedupes by path, parses Indonesian and English number formats (`1.234,56`, `1,234.56`, `(1.234)`, `45%`, `Rp…`), skips total rows, and derives per-column sums/max/min plus the top and bottom rows. It builds the prompt and turns the model's blocks into RTF whose `\stylesheet` carries `Heading 1` / `Heading 2` / `Lead Paragraph` / `Normal`.
> - **Model (`designcraft-translate`):** `OnlineTranslator::complete(system, user)` runs one `temperature: 0` Groq chat completion (shared auth, status mapping and `Retry-After` handling with translation; non-Groq builds return `Unavailable`). The model returns `{"blocks":[{"role":"heading1"|"heading2"|"lead"|"body","text":"…"}]}` and never writes RTF. Strict parsing; one parse failure retries with a terser prompt, then the error surfaces in the status row — no prose salvage.
> - **UI:** **Draft Review** sits beside "Translate this Page" in the Pubgen header, enabled when Groq is configured, an RXX review page is selected, and the catalog has table CSV rows. While the request runs the window shows progress with a working **Cancel** (closing it mid-draft cancels too); when the blocks arrive it lists them, and **Apply** writes a temp RTF — creating the pubgen temp dir when the session has none — before `file.place` places it into the page's English body frame (imported styles, one undo step). Only a placed narrative clears the draft: a failed write keeps the blocks so the user can retry, and Discard never touches the document. The job mirrors `TranslateJob` (background thread, channel, per-frame `poll`, cancel flag, wasm stub), and the button is off on the web build rather than arming a stub job that could never report back.

---

## 6. Pillar 5: Codebase Modularization (Decomposing `mod.rs`)

### 6.1 Motivation & Technical Debt
Currently, [`crates/ui-egui/src/panels/pubgen/mod.rs`](file:///f:/GitHub/designcraft/crates/ui-egui/src/panels/pubgen/mod.rs) exceeds 17,500 lines of Rust code. It houses UI rendering, state management, Excel conversions, table rendering, 13 cover styles, TOC parsing, and header/footer engines in a single file.

> **Status (2026-10-25): executed.** `mod.rs` is a 198-line panel entrypoint (`show()` + wiring); the domain modules are `state`, `helpers`, `csv`, `themes`, `tables`, `pages`, `page_utils`, `toc`, `covers`, `header_footer`, `excel`, `widgets`, `ai`, `locale`, `cover_art`, `divider_art`, plus `testutil` for shared test helpers. The 154-test suite is distributed across the domain files. Executed as flat single files; the proposed `covers/`, `toc/`, `header_footer/` trees and `charts/`, `audit/` were not created. The compile-time A/B re-measurement was skipped — incremental-build benefit is implied by the split, not measured.

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

| Milestone | Deliverables | Estimated Effort |
| :--- | :--- | :--- |
| **M1: Modularization** | Split `mod.rs` into submodules (`covers/`, `tables/`, `toc/`, `header_footer/`). Zero functional regressions. | 2–3 days |
| **M2: Preflight QA Audit** | Build `audit/` engine checking cell overset, missing sources, and image DPI. Add Audit button to UI. | 3–4 days |
| **M3: Glossary Memory** | Implement `bps_glossary.json` injection into AI translation system prompt with user UI override editor. | 2 days — **✅ implemented 2026-10-27 (see §5)** |
| **M4: Vector Chart Engine** | Implement native vector bar/line/donut chart rendering from `CXX` Excel sheets. | 5–7 days |
| **M5: Batch Multi-Chapter Binder** | Implement project manifest and one-click compiler for entire `BAB_01`..`BAB_10` workbook directories. | 5–6 days — **✅ implemented 2026-10-27 (see §3; manual two-workbook check pending)** |
| **M6: Dual PDF Profiles** | Integrate one-click PDF/X-4 (print) and compact hyperlinked PDF (web) export presets. | 3–4 days |
| **M7: Universal Profiles** | Decouple BPS-specific keywords into `PubgenProfile` schema; ship built-in profiles (Corporate, Academic, Catalog) + custom profile import/export. | 4–5 days |

---

## 10. Conclusion

Implementing this architectural roadmap elevates PubGen from a specialized municipal layout assistant to a world-class, multi-domain desktop publishing platform capable of delivering publication-grade output for governmental, institutional, academic, and enterprise organizations worldwide.

