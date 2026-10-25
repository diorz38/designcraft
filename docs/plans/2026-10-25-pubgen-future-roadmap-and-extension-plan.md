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

This document details the architectural specifications for the next six major pillars:
1. **Native Statistical Infographics & Vector Chart Generator (`C01`, `C02`)**
2. **Multi-Chapter Publication Binder & Project Compiler ("Book" / Batch Mode)**
3. **Statistical Layout Preflight & Quality Assurance (QA Audit)**
4. **BPS Statistical AI Assistant & Official Terminology Memory**
5. **Codebase Modularization (Decomposing `mod.rs` Monolith)**
6. **One-Click Dual Export Pipelines (Print-Ready PDF/X-4 vs. Web Distribution)**

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
PubGen will bundle an authoritative, editable terminology dictionary injected into the system prompt:

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
    "Keterangan": "Description"
  }
}
```

### 5.3 Automated Executive Narrative Summarizer (Drafting `ulasan_bab`)
- **Capability:** When generating a chapter from raw data tables without pre-authored RTF narrative files, the AI Assistant analyzes the table figures (e.g., highest population subdistrict, largest agricultural yield) and drafts the introductory chapter review following official BPS analytical phrasing.
- **Strict Formatting Output:** Emits pre-styled paragraphs conforming to `Heading 1`, `Heading 2`, `Lead Paragraph`, and indented body text.

---

## 6. Pillar 5: Codebase Modularization (Decomposing `mod.rs`)

### 6.1 Motivation & Technical Debt
Currently, [`crates/ui-egui/src/panels/pubgen/mod.rs`](file:///f:/GitHub/designcraft/crates/ui-egui/src/panels/pubgen/mod.rs) exceeds 17,500 lines of Rust code. It houses UI rendering, state management, Excel conversions, table rendering, 13 cover styles, TOC parsing, and header/footer engines in a single file.

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

## 8. Implementation Roadmap & Milestones

| Milestone | Deliverables | Estimated Effort |
| :--- | :--- | :--- |
| **M1: Modularization** | Split `mod.rs` into submodules (`covers/`, `tables/`, `toc/`, `header_footer/`). Zero functional regressions. | 2–3 days |
| **M2: Preflight QA Audit** | Build `audit/` engine checking cell overset, missing sources, and image DPI. Add Audit button to UI. | 3–4 days |
| **M3: Glossary Memory** | Implement `bps_glossary.json` injection into AI translation system prompt with user UI override editor. | 2 days |
| **M4: Vector Chart Engine** | Implement native vector bar/line/donut chart rendering from `CXX` Excel sheets. | 5–7 days |
| **M5: Batch Multi-Chapter Binder** | Implement project manifest and one-click compiler for entire `BAB_01`..`BAB_10` workbook directories. | 5–6 days |
| **M6: Dual PDF Profiles** | Integrate one-click PDF/X-4 (print) and compact hyperlinked PDF (web) export presets. | 3–4 days |

---

## 9. Conclusion

Implementing this architectural roadmap elevates PubGen from a specialized layout assistant to an unmatched, comprehensive desktop publishing platform for governmental, institutional, and enterprise publications.
