# Plan: Replace Pubgen Side Panel with Dedicated Top-Level Menu & Popup Dialogs

- **Date:** 2026-10-28
- **Author:** Antigravity Agent & diorz38
- **Branch:** `feature/pubgen-panel-ux-redesign`
- **Target Files:**
  - `crates/ui-egui/src/dock_extended.rs` (Pubgen menu & dialog dispatch)
  - `crates/ui-egui/src/menus/bar.rs` (1-line menu injection)
  - `crates/ui-egui/src/app.rs` (1-line popup dialog render hook)
  - `crates/ui-egui/src/panels/pubgen/dialogs.rs` (New: modal dialog forms for Pubgen options)
  - `crates/ui-egui/src/panels/pubgen/state.rs` (Dialog open/close state flags)
- **Status:** Planned

---

## 1. Executive Summary & Objective

Currently, the Pubgen workflow is hosted inside an expanded tab in the right dock panel (`DOCK_TABS`). This introduces two major downsides:
1. **Dock Clutter & Upstream Divergence:** Adding `pubgen` to `DOCK_TABS` changes the dock layout, alters default column tab counts, and requires downstream overrides in `panel_docking/tests.rs`.
2. **Horizontal Constraint Friction:** Complex forms (multi-column CSV mapping, full-featured cover design, table styling, and multi-sheet options) feel cramped inside a 260pt dock column.

### The New Architecture:
- **Dedicated Top-Level Menu:** A `Pubgen` top-level menu in the main app bar (between `Window` and `Help` or beside `Window`).
- **Modal Popup Windows:** Every form option (Excel options, TOC builder, Cover generator, Header/Footer setup, Manual CSV/Table, AI Terminology) opens as a **floating / modal popup dialog** with proper margins, full field visibility, and standard `OK / Cancel / Generate` actions.
- **Complete Side Panel Removal:** Once the menu and dialogs are in place, `pubgen` is removed from `DOCK_TABS` and the side dock returns to clean InDesign parity (`Properties`, `Pages`, `Layers`).

```
┌────────────────────────────────────────────────────────────────────────┐
│ File  Edit  Layout  Type  Object  Table  View  Window  Pubgen  Help   │
└──────────────────────────────────────────────────────────┬─────────────┘
                                                           │
                                ┌──────────────────────────▼──────────────┐
                                │ ▶ Language Mode                         │
                                │     ✓ Dual Lang (Bilingual ID / EN)     │
                                │       Single Lang (Indonesian)          │
                                │       Single Lang (English)             │
                                │ ─────────────────────────────────────── │
                                │ ▶ Master Excel (.xlsx) Import…          │
                                │   Auto-Generate from Active Excel       │
                                │   Download Generated CSVs as ZIP        │
                                │ ─────────────────────────────────────── │
                                │ ▶ Table of Contents (TOC) Options…      │
                                │   Add Table of Contents                 │
                                │   Rebuild All TOCs                      │
                                │ ─────────────────────────────────────── │
                                │ ▶ Publication Cover Generator…          │
                                │ ▶ Manual CSV / Table Importer…          │
                                │ ▶ Header & Footer Templates…            │
                                │ ─────────────────────────────────────── │
                                │ ▶ Page & Layout Utilities               │
                                │     Set Standard BPS Margins            │
                                │     Toggle Facing Pages (Single/Double) │
                                │     Add Blank Page                      │
                                │     Renumbering & Sections…             │
                                │ ─────────────────────────────────────── │
                                │ ▶ AI Translation & Terminology…         │
                                │   Compile Publication (Project Wizard)… │
                                │ ─────────────────────────────────────── │
                                │   About PubGen…                         │
                                └─────────────────────────────────────────┘
```

---

### Language Mode Replacement & Workflow:
- **Location 1 (Top-Level Pubgen Menu):** A dedicated `Language Mode` submenu with checkmarks showing the active mode (`✓ Dual Lang (Bilingual ID / EN)` vs `Single Lang (Indonesian)` vs `Single Lang (English)`). Selecting any flips the document generation mode instantly.
- **Location 2 (Inside Generation Dialogs):** In the **Master Excel Import Dialog** and **Manual CSV Dialog**, a prominent segmented switch (`[ Single Lang | Dual Lang* ]`) appears right at the top header, so the mode is crystal clear before starting batch generation.

---

### Set Page Margins Replacement & Workflow:
- **Location 1 (Pubgen Menu › Page & Layout Utilities):** `Set Standard BPS Margins` is a direct action in the menu. Clicking it runs `execute_set_page_margins(app)` immediately across all spreads and parent pages (Top 20.20 mm, Bottom 20 mm, Inside 19 mm, Outside 15 mm, while safely preserving 0-margin Cover and Divider pages).
- **Location 2 (Page & Layout Utilities Dialog):** In the `Renumbering & Sections…` popup window, a dedicated **BPS Standard Margins** button and custom margin adjustment fields allow fine-tuning margins per page or spread.

---

## 2. Zero-Conflict Upstream Strategy

To ensure painless rebasing and merging when updates arrive from upstream:

1. **`menus/tables.rs` remains 100% UNTOUCHED.** We do not insert items into upstream's static `MENUS` array.
2. **`dock_extended.rs` is our fork's owner file.** All Pubgen menu rendering and dialog dispatching live here.
3. **Only two 1-line hooks in upstream files:**
   - In `crates/ui-egui/src/menus/bar.rs` (inside `menu_bar()`):
     ```rust
     #[cfg(feature = "pubgen")]
     crate::dock::pubgen_menu(app, ui);
     ```
   - In `crates/ui-egui/src/app.rs` (inside the main UI frame after `dialogs::show()`):
     ```rust
     #[cfg(feature = "pubgen")]
     crate::dock::pubgen_dialogs(self, &ctx);
     ```
4. **Dock Tests Return to 100% Upstream Parity:** Removing `pubgen` from `DOCK_TABS` means `crates/ui-egui/src/panel_docking/tests.rs` no longer needs fork-specific assertions!

---

## 3. Popup Windows & Form Options Design

All existing card contents from the side panel are modularized into dedicated popup dialogs:

| Dialog | Trigger Menu Item | Size (WxH) | Key Content & Actions |
|---|---|---|---|
| **Excel Ingest & Generate** | `Master Excel (.xlsx) Import…` | 520 × 480 | File picker, Sheet detection summary, Auto-generate checkbox, Single/Dual Lang toggle, AI translate toggle, `[Convert & Generate]` |
| **Table of Contents Options** | `Table of Contents (TOC) Options…` | 480 × 520 | TOC Style (Entry, Table, Chart, Paragraph), Layout (Standard, TablePage), Title field, Hanging indent, Page filter words, Pair bilingual checkbox, Insert after selected page, `[Add TOC]`, `[Rebuild TOC]` |
| **Cover Generator** | `Publication Cover Generator…` | 560 × 640 | Cover preset styles (12 styles), Colors (Primary, Accent, Dark), Metadata overrides (Title 1, Title 2, Year, Volume, Agency), Image slots (Hero, Skyline, Logo), AI Prompt & Palette, `[Generate Cover]` |
| **Header & Footer Setup** | `Header & Footer Templates…` | 500 × 520 | Style preset, Sided rules, Scope (All, Section, Range), Rule weight, Accent bar, Colors, `[Apply H/F]`, `[Remove H/F]`, `[Save Template]` |
| **Manual CSV / Table** | `Manual CSV / Table Importer…` | 540 × 560 | Tab 1: Page from CSV (Column mappings, template resolution, range). Tab 2: Table from CSV (Theme, row conditions, column widths). `[Generate Pages]`, `[Insert Table]` |
| **Page Utilities** | `Page Margins & Renumbering…` | 420 × 360 | BPS margin reset (20/25/25mm), Section renumbering start, Page from RTF, Page from Image |
| **AI Translation & Terminology** | `AI Translation & Terminology…` | 580 × 600 | Existing `ai::AiSetupDialog`: Groq API key, model selection, BPS Glossary table editor, search & draft auto-fill |
| **Compilation Wizard** | `Compile Publication (Project Wizard)…` | 520 × 440 | Existing `compile::NewProjectDialog`: Manifest path, chapter plans, PDF export |
| **About PubGen** | `About PubGen…` | 420 × 380 | Existing `about_pubgen_dialog`: Logo, author attribution, BPS Karawang info |

---

## 4. Modal Dialog Implementation Pattern

Dialogs use DesignCraft's standard `modal_drag::modal` or `egui::Window` with center-screen placement:

```rust
// In crates/ui-egui/src/panels/pubgen/dialogs.rs:

pub fn show_all_dialogs(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if st.dialog_excel_open {
        excel_import_dialog(app, ctx, st);
    }
    if st.dialog_toc_open {
        toc_dialog(app, ctx, st);
    }
    if st.dialog_cover_open {
        cover_dialog(app, ctx, st);
    }
    if st.dialog_hf_open {
        header_footer_dialog(app, ctx, st);
    }
    if st.dialog_manual_open {
        manual_import_dialog(app, ctx, st);
    }
    if st.dialog_utils_open {
        page_utils_dialog(app, ctx, st);
    }
    if st.show_about_dialog {
        about_pubgen_dialog(app, ctx, &mut st.show_about_dialog, &Tokens::get(ctx));
    }
    if st.ai_dialog.is_some() {
        ai::dialog_ui_ctx(app, st, ctx);
    }
}
```

Each dialog has:
- Draggable header bar with title and `✕` close button
- ESC key dismisses the dialog
- Clear primary action button (e.g., `Generate`, `Apply`) and a `Cancel` button
- Non-blocking: background jobs (like AI translation or Excel parsing) display their progress indicators directly inside the dialog or as floating toast notifications.

---

## 5. Complete Function Inventory (Submodule Reference)

The subagent research mapped all ~120 pub/pub(crate) functions across the 15 modules:

1. **`excel.rs`**: `execute_excel_and_generate`, `execute_excel_to_csvs`, `execute_download_csv_zip`, `execute_convert_excel`, `parse_xlsx_sheets`, `convert_excel_to_csv_data`, `apply_excel_cover`.
2. **`toc.rs`**: `execute_add_toc`, `execute_rebuild_toc`, `rebuild_all_tocs`, `generate_toc`, `enrich_toc_story`, `ensure_toc_entry_style`, `sync_toc_continuation_headers`.
3. **`covers.rs`**: `execute_generate_cover`, `execute_generate_front_cover`, `execute_generate_back_cover`, `execute_generate_inside_cover`, `trigger_generate_cover`, `apply_ai_cover_result`.
4. **`pages.rs`**: `execute_generate_pages`, `resolve_template_path_spec`.
5. **`tables.rs`**: `execute_insert_table`, `execute_insert_table_matrix`, `detect_condition_rows`.
6. **`page_utils.rs`**: `execute_set_page_margins`, `execute_renumber_page`, `execute_continue_numbering`, `execute_page_from_rtf`, `execute_page_from_image`.
7. **`header_footer.rs`**: `execute_apply_header_footer`, `execute_remove_header_footer`, `execute_save_hf_template`, `execute_delete_hf_template`.
8. **`compile.rs`**: `compile_project`, `poll_compile_job`, `CompileJob`.
9. **`ai.rs`**: `dialog_ui`, `fill_blank_drafts`, `apply_page_result`, `apply_table_page_result`, `push_groq_quota_popup`.
10. **`locale.rs`**: `words`, `detect_from_records`, `split_pair`, `split_table_title`, `split_chart_title_parts`.
11. **`themes.rs`**: `TableColorTheme`, `parse_table_color_theme`, `ensure_swatch`.
12. **`csv.rs`**: `parse_csv_content_with_delim`, `load_card1_csv`, `load_card2_csv`.

---

## 6. Implementation Steps

### Phase 1: Dialog Infrastructure & State Flags
- Add dialog open boolean flags to `PubgenState` in `crates/ui-egui/src/panels/pubgen/state.rs`:
  - `dialog_excel_open: bool`
  - `dialog_toc_open: bool`
  - `dialog_cover_open: bool`
  - `dialog_hf_open: bool`
  - `dialog_manual_open: bool`
  - `dialog_utils_open: bool`
- Create `crates/ui-egui/src/panels/pubgen/dialogs.rs` extracting the card bodies into clean modal windows.

### Phase 2: Pubgen Top-Level Menu in `dock_extended.rs`
- Implement `pub fn pubgen_menu(app: &mut DesignApp, ui: &mut egui::Ui)` in `dock_extended.rs`.
- Inject `crate::dock::pubgen_menu(app, ui)` into `menus/bar.rs`.
- Wire each menu button to toggle the corresponding dialog flag in `PubgenState` via `with_pubgen_state(ui.ctx(), ...)`.

### Phase 3: Global Dialog Renderer Hook
- Implement `pub fn pubgen_dialogs(app: &mut DesignApp, ctx: &egui::Context)` in `dock_extended.rs`.
- Inject `crate::dock::pubgen_dialogs(self, &ctx);` into `crates/ui-egui/src/app.rs`.

### Phase 4: Side Panel Removal & Upstream Parity Clean Up
- Remove `"pubgen"` from `DOCK_TABS` in `dock_extended.rs`.
- Revert any pubgen-specific assertions in `crates/ui-egui/src/panel_docking/tests.rs` so tests are 100% standard upstream.
- Verify that `cargo test` and `cargo xtask ci` pass cleanly.

---

## 7. Verification Checklist

- [ ] `cargo check --features pubgen` compiles without warnings.
- [ ] Top bar displays `Pubgen` menu between `Window` and `Help`.
- [ ] Clicking `Master Excel (.xlsx) Import…` opens the Excel modal popup window.
- [ ] Clicking `Table of Contents Options…` opens the TOC modal popup window.
- [ ] Clicking `Publication Cover Generator…` opens the Cover modal popup window.
- [ ] Clicking `Header & Footer Templates…` opens the H/F modal popup window.
- [ ] Clicking `AI Translation & Terminology…` opens the AI modal dialog.
- [ ] The right dock panel has NO "Pubgen" tab (Properties, Pages, Layers only).
- [ ] Closing dialogs with `✕` or `Cancel` or `Esc` works reliably.
- [ ] Quality gate: `cargo xtask ci` passes all lint, test, and layer checks.
