# RTF Stylesheet Maintenance and Multi-Level "TOC Paragraph" Implementation Plan

> **Status:** PROPOSED  
> **Date:** 2026-10-21  
> **Target Branch:** `feature/pubgen-modular-templates`  
> **Area:** `crates/textimport`, `crates/engine/src/cmd/toc.rs`, `crates/ui-egui/src/panels/pubgen.rs`

---

## 1. Goal & Context

When generating publication pages from Excel sheets named `RXX` (e.g. `R01`, `R02` for narrative chapter reviews), the placed `.rtf` files currently lose their semantic paragraph styles (`Title`, `Heading 1`, `Heading 2`) because `crates/textimport/src/rtf.rs` skips the `\stylesheet` destination and returns an empty `para_styles` list.

This plan delivers:
1. **Full RTF Stylesheet Maintenance:** Parsing RTF `\stylesheet` tables and mapping `\sN` style tags to named DesignCraft paragraph styles (`Heading 1`, `Heading 2`, `Title`, etc.).
2. **Multi-Level TOC Heading Declarations:** Allowing Table of Contents generation to include multiple hierarchical heading levels (e.g. Level 1: `Page Title` / `Title`, Level 2: `Heading 1`, Level 3: `Heading 2`).
3. **"TOC Paragraph" Style / Preset in PubGen:**
   - Default TOC Title: `"Daftar Isi"`.
   - Dedicated TOC Preset named `"TOC Paragraph"` alongside `"TOC Entry"`, `"TOC Table"`, and `"TOC Chart"`.
   - Scoped filtering that processes **only pages generated from sheet name `RXX`** (and other declared patterns such as `GXX` / narrative notes `A02`).

---

## 2. Architecture & Data Flow

```
+-------------------------------------------------------------------------+
| Excel Workbook (Sheets: COVER, DIV, R01, R02, 01_01, G01...)           |
+-------------------------------------------------------------------------+
                                    |
                  [parse_rtf_review_sheet_with_template]
                                    v
+-------------------------------------------------------------------------+
| PubGen Catalog & Page Generation (pubgen.rs / pubgen_excel.rs)          |
| - Identifies RXX sheets as type "Ulasan"                                 |
| - Places header text boxes ("BAB 01", chapter title)                    |
| - Invokes execute_page_from_rtf / file.place                            |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| RTF Parser (crates/textimport/src/rtf.rs)                               |
| 1. Parses {\stylesheet{\s1\sbasedon0 \b Heading 1;}...}                 |
| 2. Tracks \sN tags on paragraphs -> sets ParaFormat.style               |
| 3. Returns Imported { story, para_styles: [Heading 1, Heading 2...], .. }|
+-------------------------------------------------------------------------+
                                    |
                      [file.place / add_styles]
                                    v
+-------------------------------------------------------------------------+
| DesignCraft Document (doc.styles.para & story.paras)                    |
| - Paragraph styles registered in document                               |
| - Story paragraphs bound to "Title", "Heading 1", "Heading 2"           |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| TOC Engine & PubGen "TOC Paragraph" (pubgen.rs / cmd/toc.rs)            |
| - TOC Title: "Daftar Isi"                                               |
| - Filters pages to only those generated from RXX / declared patterns    |
| - Multi-level entries: Title (L1), Heading 1 (L2), Heading 2 (L3)       |
| - Emits nested, hierarchical Table of Contents with page references     |
+-------------------------------------------------------------------------+
```

---

## 3. Tasks Breakdown

### Task 1: RTF Stylesheet Parser (`crates/textimport/src/rtf.rs`)

**Objective:** Parse `\stylesheet` destination and tag body paragraphs with named styles.

1. **Remove `"stylesheet"` from `const SKIP: &[&str]`**.
2. **Implement Stylesheet Table Parsing:**
   - In `run_import(bytes: &[u8])`:
     - Maintain `styles_map: HashMap<usize, ImportedStyle>` mapping `\sN` (or `\csN`) index to style metadata.
     - Parse style definition groups `{\s<idx> ... <StyleName>;}`:
       - Extract style index `\sN`.
       - Extract base style `\sbasedonN` if present.
       - Extract formatting overrides (`\b`, `\i`, `\fsN`, `\ql`, `\qc`, `\qr`, `\qj`).
       - Extract style name ending with semicolon `;` (e.g. `Heading 1`, `Heading 2`, `Title`).
3. **Bind Body Paragraphs:**
   - Track `cur_para_style: String` (defaults to `BASIC_PARAGRAPH`).
   - On `\sN`, look up style name from `styles_map` and set `cur_para_style`.
   - When creating or flushing paragraphs (`\par`), apply `p.style = cur_para_style.clone()`.
4. **Populate `Imported`:**
   - Return `Imported { story, para_styles, char_styles, warnings }`.
5. **Unit Tests:**
   - Add tests verifying RTF with `{\stylesheet{\s1 Heading 1;}{\s2 Heading 2;}}` produces `para_styles` with names `"Heading 1"`, `"Heading 2"`, and paragraphs tagged accordingly.

---

### Task 2: Multi-Level Heading TOC Declaration (`crates/engine/src/cmd/toc.rs`)

**Objective:** Support multi-level heading hierarchies in Table of Contents generation.

1. **Hierarchy in `entries`:**
   - Verify `toc.generate` correctly accepts multiple style entries:
     ```json
     "entries": [
       { "style": "Title", "level": 1, "pageNumber": "after", "separator": "\t" },
       { "style": "Heading 1", "level": 2, "pageNumber": "after", "separator": "\t" },
       { "style": "Heading 2", "level": 3, "pageNumber": "after", "separator": "\t" }
     ]
     ```
2. **Formatting & Indents for Levels 1–9:**
   - In `ensure_toc_entry_style` (or dynamic level styles `TOC Entry L1`, `TOC Entry L2`, `TOC Entry L3`):
     - Level 1: Left indent = 0 pt.
     - Level 2: Left indent = 14 pt (or configurable).
     - Level 3: Left indent = 28 pt.
3. **Preserve Reading Order:**
   - Ensure `collect` in `crates/engine/src/cmd/toc.rs` sorts entries primarily by page location and baseline `(page, y, x)`, preserving the sequential flow of headings within chapters.

---

### Task 3: PubGen "TOC Paragraph" Kind & Sheet Scoping (`crates/ui-egui/src/panels/pubgen.rs`)

**Objective:** Add the `"TOC Paragraph"` preset and constrain processing to `RXX` pages.

1. **Extend `TOC_KINDS`:**
   ```rust
   pub const TOC_KINDS: [&str; 4] = [
       "TOC Entry",
       "TOC Table",
       "TOC Chart",
       "TOC Paragraph",
   ];
   ```
2. **Default Title for `TOC Paragraph`:**
   ```rust
   pub fn toc_kind_title(kind: usize) -> Option<&'static str> {
       match kind {
           1 => Some("Daftar Tabel"),
           2 => Some("Daftar Grafik"),
           3 => Some("Daftar Isi"),
           _ => None,
       }
   }
   ```
3. **Page Scoping / Filtering Mechanism:**
   - During publication generation (`generate_all_pages_from_catalog`):
     - Record page indices generated from `RXX` review sheets (and declared patterns like `GXX`, `A02`).
     - Tag chapter frames / stories with a distinct marker or metadata, or maintain `rxx_pages: Vec<usize>` in document / session state.
   - In `TocPlan`:
     - When `toc_kind == 3` (`"TOC Paragraph"`):
       - Only process pages belonging to `RXX` (or matching declared patterns `R*`, `G*`).
       - Exclude pages generated from data tables (`02_01.csv`, etc.).
       - Use multi-level entries: `[Title/Chapter Title (L1), Heading 1 (L2), Heading 2 (L3)]`.
4. **Configurable Heading Styles in PubGen State:**
   - Add fields in `PubgenState` for customizing which paragraph styles map to which TOC levels:
     - `toc_para_l1_style: String` (default: `"Title"` or `"Page Title"`).
     - `toc_para_l2_style: String` (default: `"Heading 1"`).
     - `toc_para_l3_style: String` (default: `"Heading 2"`).

---

### Task 4: UI Wireup in PubGen Panel

1. **Card 3 "TOC style" dropdown:**
   - Shows: `TOC Entry`, `TOC Table`, `TOC Chart`, `TOC Paragraph`.
   - Selecting `TOC Paragraph`:
     - Sets TOC title to `"Daftar Isi"`.
     - Displays level selector / heading preview.
2. **"Add TOC" / "Rebuild TOC" execution:**
   - Generates the `"Daftar Isi"` TOC using the multi-level plan.
   - Continuation pages and headers apply automatically via existing `sync_toc_continuation_headers`.

---

### Task 5: Testing & Quality Gates

1. **`crates/textimport` Tests:**
   - Test RTF with multi-level stylesheet (`\stylesheet{\s1 Heading 1;}{\s2 Heading 2;}`).
   - Verify style names, sizes, bold/italic, and story paragraph assignments.
2. **`crates/engine` TOC Tests:**
   - Test multi-level TOC generation with nested headings across spreads.
3. **`crates/ui-egui` PubGen Tests:**
   - Test `parse_rtf_review_sheet_with_template` and page generation from `R01`.
   - Test `execute_add_toc` with `TOC Paragraph` producing title `"Daftar Isi"`.
   - Test that table pages (`02_01.csv`) are ignored by `TOC Paragraph`, while `RXX` pages are correctly indexed.
4. **Quality Gates:**
   - `cargo fmt`
   - `cargo clippy -D warnings`
   - `cargo test -p designcraft-textimport`
   - `cargo test -p designcraft-engine`
   - `cargo test -p designcraft-ui-egui`
   - `cargo xtask layers` & `cargo xtask wasm`

---

## 4. User Review & Acceptance Criteria

- [x] RTF importer preserves named paragraph styles (`Heading 1`, `Heading 2`, `Title`) from `\stylesheet`.
- [x] Placed RTF in `RXX` pages maintains paragraph styles in DesignCraft's stylesheet.
- [x] PubGen Card 3 offers `"TOC Paragraph"` with default title `"Daftar Isi"`.
- [x] Generated TOC includes declared stylesheet levels (Level 1, Level 2, Level 3).
- [x] Only pages generated from `RXX` (and declared patterns) are processed for `"TOC Paragraph"`.
