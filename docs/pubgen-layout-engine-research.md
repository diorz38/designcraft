# PubGen Layout Engine: Comprehensive Research & Specifications

## 1. Overview

This document provides a complete technical analysis of all page generation, layout presets, and Table of Contents (TOC) mechanisms currently implemented in DesignCraft's **Publication Generator (`PubGen`)**. It serves as the definitive reference for refactoring and evolving PubGen into a modular, template-driven architecture.

---

## 2. Inventory of Generated Page Types

PubGen currently generates 4 major categories of pages:

```text
PubGen Page Generation
├── 1. Publication Covers (Styles 0–9)
├── 2. Interior Publication Pages
│   ├── Special Type: Ulasan (Editorial Chapter)
│   ├── Special Type: Grafik (Chart & Visualization)
│   ├── Special Type: Note (Publication Note / A02)
│   └── 6 Layout Presets (0 to 5) for Standard Data Pages
├── 3. Statistical Tables & Data Grids
│   ├── Multi-level merged headers ("--")
│   ├── Auto-inserted column numbers "(1), (2)..."
│   └── Auto-extracted footnote & source attribution
└── 4. Table of Contents (TOC)
    ├── Standard Dot-Leader Layout
    ├── TablePage Catalog Layout (3-column tab stop)
    └── Multi-Page Continuation & Auto-Header Sync
```

---

## 3. Detailed Specifications: Interior Page Layouts (`execute_generate_pages`)

Located in [`crates/ui-egui/src/panels/pubgen.rs`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L4343-L4812).

### 3.1 Input Data & Pipeline
- Driven by catalog rows (`st.card1_records`):
  - Column 0: **Type** (e.g. `Tabel`, `Grafik`, `Ulasan`, `Note`, `BAB`)
  - Column 1: **Number** (e.g. `01`, `02.01`, `2.1`)
  - Column 2: **Title** (e.g. `Jumlah Penduduk Menurut Kecamatan`)
  - Column 3: **File** (e.g. `02_01.csv`, `A02.rtf`, `C02_01.png`)
- Page geometry context:
  - Dynamically queries `page.margin_rect()` for `[margin.x0, margin.y0, margin.x1, margin.y1]`.

---

### 3.2 Special Type Handlers

#### A. "Ulasan" (Editorial Chapter / Narrative Header)
- **Condition**: `type_val.eq_ignore_ascii_case("Ulasan")`
- **Header Structure**:
  1. **Chapter Tag Box**: `[margin.x0, margin.y0 + 20, margin.x1, margin.y0 + 56]`  
     Font: `Arial Black`, Size: `24.0 pt`, Text: `"BAB 01"` (split from title or number).
  2. **Chapter Title Box**: `[margin.x0, margin.y0 + 62, margin.x1, margin.y0 + 102]`  
     Font: `Arial Black`, Size: `20.0 pt`, Text: `"Pemerintahan"`. Tagged with `st.toc_style` for TOC indexing.
- **Narrative Body Placement**:
  - Automatically resolves RTF file from `row[3]`.
  - Creates text frame: `[margin.x0, margin.y0 + 116, margin.x1, margin.y1]`.
  - Places formatted RTF story into frame via `execute_page_from_rtf()`.

#### B. "Grafik" (Chart & Visualizations)
- **Condition**: `is_chart_type(type_val)` (matches `"grafik"`, `"chart"`, `"diagram"`, `"figure"`, `"gambar"`).
- **Header Structure**:
  - Centered 2-line title in `Arial Bold`:
    `Grafik 02.01\nPanjang Jalan Kecamatan Karawang Timur 2026`
  - Position: `[margin.x0, margin.y0 + 20, margin.x1, margin.y0 + 68]`. Tagged with `st.toc_style`.
- **Graphic Placement**:
  - Resolves related image (`.png`, `.jpg`).
  - Creates graphic frame: `[margin.x0, margin.y0 + 76, margin.x1, margin.y1]`.
  - Insets image and applies object fitting:
    - `mode: "fitProportionally"`
    - `mode: "centerContent"`
    - `mode: "fitFrameToContent"`

#### C. "Note" (Publication Note / A02 Sheet)
- **Condition**: `type_val.eq_ignore_ascii_case("Note")`
- **Header Structure**:
  - Centered, 100% transparent Arial title: `"Keterangan Publikasi"` (or custom title).
  - Position: `[margin.x0, margin.y0 + 20, margin.x1, margin.y0 + 56]`.
- **Narrative Placement**:
  - Resolves `.rtf` file from column 3.
  - Creates body text frame: `[margin.x0, margin.y0 + 64, margin.x1, margin.y1]`.
  - Places RTF story via `execute_page_from_rtf()`.

---

### 3.3 The 6 Standard Page Layout Presets (`st.card1_preset`)

When `type_val` is not a special handler (`Ulasan`, `Grafik`, `Note`), PubGen generates header text frames using one of 6 presets:

| Preset # | Name | Geometry & Layout Behavior |
|---|---|---|
| **0** (Default) | **1 Row · 3 Columns** | `Row Y: [margin.y0 + 20, margin.y0 + 80]`.<br>• **Col 1 (Type)**: Fitted tightly to text width, right-aligned.<br>• **Col 2 (Number)**: Fitted tightly to content width.<br>• **Col 3 (Title)**: Takes remaining available width with 20pt right margin; tagged with `st.toc_style`. |
| **1** | **1 Row · 3 Equal Cols** | 3 columns of equal width: `(total_width - 2 * gutter) / 3`.<br>• Col 1: Type uppercase.<br>• Col 2: Number.<br>• Col 3: Title (tagged with `st.toc_style`). |
| **2** | **Editorial Chapter (Stacked)** | 3 stacked vertical boxes across full margin width:<br>• Box 1 (Type): `Y [y0 + 20, y0 + 42]`<br>• Box 2 (Number): `Y [y0 + 46, y0 + 106]`<br>• Box 3 (Title): `Y [y0 + 112, y0 + 220]` (tagged with `st.toc_style`). |
| **3** | **Technical Spec Card** | • Accent graphic rectangle banner: `Y [y0, y0 + 6]` across full margin.<br>• Numeral box top-right: `[x1 - 80, y0 + 16, x1, y0 + 56]`.<br>• Kicker box: `[x0, y0 + 16, x1 - 90, y0 + 36]`.<br>• Title box: `[x0, y0 + 40, x1 - 90, y0 + 110]` (tagged with `st.toc_style`). |
| **4** | **Minimal Zine** | • Large Numeral box: `[x0, y0 + 60, x1, y0 + 110]`.<br>• Large Title box: `[x0, y0 + 120, x1, y0 + 200]` (tagged with `st.toc_style`). |
| **5** | **TOC Line (1 para/record)** | Combines `Type + Number + Title` into a single combined paragraph in one box: `[x0, y0 + 20, x1, y0 + 60]` (tagged with `st.toc_style`). |

---

### 3.4 Auto-Table Placement (`card1_auto_insert_tables`)

When enabled and column 3 references a `.csv` file:
1. PubGen resolves the table path relative to catalog CSV directory.
2. Calls `execute_insert_table_matrix()`.
3. Detects bottom of generated title frame (`title_bottom`).
4. Inserts table frame starting at `table_y0 = title_bottom + 20.0 pt`.
5. Allocates available height between `table_y0` and `margin.y1`, adjusting for footnotes.

---

## 4. Detailed Specifications: Table Engine (`execute_insert_table_matrix`)

Located in [`crates/ui-egui/src/panels/pubgen.rs`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L5007-L5450).

### 4.1 Structural Features:
1. **Trailing Footnote / Source Detection**:
   - Inspects matrix from the bottom up. Rows where only column 0 has text and all remaining columns are empty are extracted into footnotes and removed from table data.
2. **Multi-level Hierarchical Headers (`--` Syntax)**:
   - Header cells containing `--` (e.g., `Laki-laki -- 2024 -- Jumlah`) are parsed into up to 3 stacked header rows.
   - Parent categories automatically group common ancestors across adjacent columns.
3. **Column Numbering Row**:
   - When enabled, inserts `(1)\t(2)\t(3)...` immediately below the header rows.
4. **Table Formatting & Placement**:
   - Placement modes:
     - `0`: At current text cursor.
     - `1`: New frame on active page.
     - `2`: At page below existing page title items.
   - Styling:
     - Header row background tint.
     - Alternating row tint (optional).
     - Standard thin inner borders, bolder header border.
     - Footnote frame placed below table with `y0 = table_bounds.y1 + 6.0 pt`.

---

## 5. Detailed Specifications: Table of Contents (`generate_toc`)

Located in [`crates/ui-egui/src/panels/pubgen.rs`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L5840-L6700).

### 5.1 Architecture & Flow

```text
[User clicks "Add TOC" or "Rebuild TOC"]
             ↓
[Build TocPlan from state or existing TOC definition]
             ↓
[Engine runs "toc.generate" (extracts paragraphs matching toc_style & words)]
             ↓
[enrich_toc_story() processes extracted lines]
  ├── Standard Mode: [Prefix + Title] ................... [Page]
  └── TablePage Mode: [Number]\t[Title]\t[Page] (BPS 3-column format)
             ↓
[apply_toc_post_process()]
  ├── Sets right-aligned tab stops with leader dots (". ")
  ├── Applies hanging indent on wrapped titles
  └── Triggers text.autoflow for multi-page overflow
             ↓
[sync_toc_continuation_headers()]
  ├── Detects continuation frames across spreads
  ├── Resizes story frames to leave room for continuation header
  └── Inserts continuation header: "Daftar Tabel (Lanjutan)"
```

### 5.2 TOC Layout Modes (`st.toc_layout_style`)

1. **Standard TOC (Mode 0)**:
   - Format: `<Title> .................... <PageNumber>`
   - Right-aligned tab stop at page margin width with leader dot `. `.
   - Optional hanging indent for wrapped multi-line titles.
2. **TablePage / BPS Catalog Format (Mode 1)**:
   - Format: `<Number>\t<Title>\t<PageNumber>`
   - Column 1: Aligned table/chapter number (e.g. `02.01`).
   - Column 2: Aligned left margin for wrapped title.
   - Column 3: Right-aligned page number.
   - Custom column header line: `Tabel\tJudul Tabel\tHalaman`.
   - Section headers (e.g. `2.1 Keadaan Geografis`) automatically styled without page numbers.

### 5.3 Multi-Page Overflow & Continuation
- When a TOC spans multiple pages, PubGen:
  1. Identifies continuation frames on pages `2..N`.
  2. Insets the text frame top by `TOC_CONT_HEADER_H (18pt) + TOC_CONT_HEADER_GAP (6pt)`.
  3. Inserts a dedicated header frame tagged with label `pubgen_toc_continuation_header`.
  4. Header text: `<Title> (Lanjutan)` (Indonesian) or `<Title> (Continued)` (English).

---

## 6. Synthesis: Blueprint for the Template Engine

Every page type analyzed above can be completely modeled by **5 Slot Types** in a `.designcraft` template:

1. **`#slot:text:<key>`**:
   - `chapter_num` (e.g. "BAB 01")
   - `chapter_title` (e.g. "Pemerintahan")
   - `title` (e.g. "Jumlah Penduduk")
   - `number` (e.g. "02.01")
   - `type` (e.g. "Tabel")
   - `source` (e.g. "Sumber: BPS")
2. **`#slot:story:<key>`**:
   - Narrative flow for editorial chapters (*Ulasan*) and publication notes (*A02*).
3. **`#slot:image:<key>`**:
   - Cover hero photos, medallion cutouts, chart images, logos.
4. **`#slot:table:<key>`**:
   - Container frame that receives data tables with pre-styled boundaries.
5. **`#slot:toc:<key>`**:
   - Container frame that receives generated TOC stories with pre-configured tab-stops and continuation rules.
