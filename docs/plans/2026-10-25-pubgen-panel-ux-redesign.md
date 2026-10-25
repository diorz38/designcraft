# Proposal: Pubgen Panel Ergonomics & Workflow UX Redesign

- **Date:** 2026-10-25
- **Author:** Antigravity Agent & diorz38
- **Target Component:** `crates/ui-egui/src/panels/pubgen/mod.rs` (`pub fn show`, cards 1–3)
- **Status:** Implemented

---

## 1. Executive Summary & Motivation

In production use of the **Publication Generator (Pubgen)** panel, users frequently interact with three core capabilities:
1. **Language Mode** (`Single Lang` vs `Dual Lang` / Bilingual)
2. **Master Excel Ingest** (`Excel to CSV Data…` + Auto-generate pages, tables & cover)
3. **Table of Contents** (`Add TOC`, `Rebuild TOC`, layouts, and title configuration)

### The Current Usability Bottlenecks

Currently, the panel layout is arranged in legacy feature-chronological order rather than actual user workflow:

| Feature | Current Location | Usability Issue |
| :--- | :--- | :--- |
| **Lang Mode** | Small outline button in top header band (`pubgen_header_band`) | Easy to miss; lacks a clear visual status indicator of what mode the active document/session is in. |
| **Excel to CSV Data** | At the very bottom of **Card 3 (Quick Actions)** | **Severe friction**: In BPS workflows, the multi-sheet Excel file is the primary entry point for 90%+ of publications. Users must scroll past 4 large cards (Page from CSV, Table from CSV, Cover, Header/Footer) just to click the starting button. |
| **Table of Contents (TOC)** | Buried inside **Card 3 (Quick Actions)** between Table Options and Page Renumbering | TOC generation and rebuilding (after pagination shifts) is a primary editorial phase. It is currently obscured within minor utilities. |

---

## 2. Actual User Workflow Analysis

The natural user mental model follows a clear 4-step pipeline:

```
[Step 1: Setup Mode]  ──>  [Step 2: Ingest & Generate]  ──>  [Step 3: Cover & Styling]  ──>  [Step 4: TOC & Finalize]
• Single vs Dual Lang       • Excel (.xlsx) Import           • Cover Preset & Overrides       • Add / Rebuild TOC
• AI Provider status        • Auto-generate all sheets       • Header & Footer templates      • List of Tables / Charts
                            • Or manual CSV import                                            • Renumber / Page Setup
```

---

## 3. Proposed Redesign Architectures

### Option A: Chronological Workflow Sections (Recommended)

Keeps the unified single-panel scroll but re-orders the visual hierarchy to match the natural workflow:

```
┌─────────────────────────────────────────────────────────────┐
│ PUBGEN  [ AI Translator ✓ ]   [ Single Lang | Dual Lang* ]  │ <-- Prominent Segmented Switch
├─────────────────────────────────────────────────────────────┤
│ 1. DATA SOURCE & GENERATE (Master Excel)                    │
│    [☑ Auto-generate pages, tables & cover from sheets]      │
│    [====== Excel to CSV Data (.xlsx)... ======] (Primary)   │
│    Active: KBDA2026.xlsx (42 records)                       │
│    [ Download Generated CSVs as ZIP... ]                    │
├─────────────────────────────────────────────────────────────┤
│ 2. TABLE OF CONTENTS (Dedicated Section)                    │
│    Kind: [ TOC Entry ▼ ]    Layout: [ TablePage (BPS) ▼ ]   │
│    Title: [ DAFTAR ISI / CONTENTS                          ]│
│    [☑ Hanging Indent]  [☑ Pair Bilingual (ID / EN)]         │
│    [     Add TOC     ]   [   Rebuild TOC   ]                │
├─────────────────────────────────────────────────────────────┤
│ 3. MANUAL IMPORT & FINE-TUNING (Collapsible Accordion)      │
│    ▶ Publication Cover (Style, Theme & Overrides)           │
│    ▶ Page from CSV (Column Mappings & Ingest)               │
│    ▶ Table from CSV (Theme & Table Options)                 │
│    ▶ Header & Footer Templates                              │
├─────────────────────────────────────────────────────────────┤
│ 4. PAGE UTILITIES                                           │
│    [ Double-sided ]  [ Add Blank Page ]  [ Renumber ]       │
└─────────────────────────────────────────────────────────────┘
```

#### Advantages of Option A:
- **Instant access:** The most important button (`Excel to CSV Data…`) is visible immediately without scrolling.
- **Workflow alignment:** Follows the publication building sequence from top to bottom.
- **Backward compatible:** All underlying state variables (`PubgenState`) and functions remain unchanged; only the UI rendering sequence in `pubgen::show` is rearranged.

---

### Option B: Tabbed Navigation Interface (Zero-Scroll)

If the user prefers a compact, fixed-height panel without vertical scrolling:

- **Tab 1: Import & Build**
  - Mode switch (Single / Dual Lang).
  - Excel master import & auto-generate.
  - Quick catalog summary and manual CSV generation.
- **Tab 2: Cover & Front Matter**
  - Cover style selection and metadata fields.
  - Header & Footer template rules.
- **Tab 3: Table of Contents**
  - Dedicated full-featured TOC editor: Kind (`TOC Entry`, `TOC Table`, `TOC Chart`, `TOC Paragraph`), Layout styles, Indentation, and Rebuild triggers.
- **Tab 4: Utilities**
  - Page margin tools, facing pages switcher, page renumbering, table options.

---

## 4. Implementation Plan for Future Fix

1. **Phase 1: Header Re-organization**
   - Implemented `toggle_lang_button` with prominent active accent fill (`t.accent`, blue) and outline when inactive. Dual Lang is the default mode for newly initialized panels.
   - Added About icon (`ℹ`) right-aligned in the header row (`[Pubgen] [toggle Lang Mode] [AI Translator] {justify-between} [about_icon]`).
   - Added About PubGen modal dialog popup (`about_pubgen_dialog`), showcasing `pubgen_logo.png`, app name `"PubGen"` (`"Publication Generator"`), description, and author attribution text: `"Developed by IPDS BPS Karawang, Budi Yunior (dior@bps.go.id)"`.

2. **Phase 2: Master Excel Elevation**
   - Extracted the Excel import block (`ui.checkbox(&mut st.excel_auto_generate, ...)` and `resp_excel = outline_button("Excel to CSV Data…")`) out of `card3_quick_actions_ui`.
   - Placed it into dedicated top section `master_excel_ui(app, ui, st, t)` directly below the header band.

3. **Phase 3: Dedicated TOC Section**
   - Moved the TOC UI block out of `card3_quick_actions_ui` into its own top-level section `card_toc_ui(app, ui, st, t)`.
   - Added `toc_insert_after` checkbox ("Insert on new page after selected page") to automatically insert a new page and render the generated TOC.
   - Added clean visual framing and intuitive button grouping (`Add TOC` & `Rebuild TOC`).

4. **Phase 4: Collapsible Advanced Sections**
   - Wrapped legacy `card1_ui` (manual CSV column mapping), `card2_ui` (manual table insertion), and Publication Cover into collapsible accordion headers (`manual_import_accordion_ui`), keeping the panel clean by default when relying on the all-in-one Excel workflow.
