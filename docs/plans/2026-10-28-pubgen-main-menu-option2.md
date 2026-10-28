# Plan: Replace Pubgen Side Panel with Dedicated Top-Level Menu & Popup Dialogs

- **Date:** 2026-10-28
- **Author:** Antigravity Agent & diorz38
- **Branch:** `feature/pubgen-panel-ux-redesign`
- **Target Files:**
  - `crates/ui-egui/src/dock_extended.rs` (Pubgen menu, dialog dispatch, & unit tests)
  - `crates/ui-egui/src/menus/bar.rs` (1-line menu injection)
  - `crates/ui-egui/src/app.rs` (1-line popup dialog render hook)
  - `crates/ui-egui/src/panels/pubgen/dialogs.rs` (Modal dialog forms for Pubgen options)
  - `crates/ui-egui/src/panels/pubgen/state.rs` (Dialog open/close state flags)
  - `crates/ui-egui/src/panels/pubgen/mod.rs` (`with_state` atomic access and `poll_active_jobs`)
  - `crates/ui-egui/src/panels/pubgen/compile.rs` (Parameterless compilation wizard invocation)
  - `crates/ui-egui/src/panel_docking/tests.rs` (Restored 100% upstream dock assertions)
- **Status:** Completed & Verified (Commit `8cd99e51`)

---

## 1. Executive Summary & Objective

Previously, the Pubgen workflow was hosted inside an expanded tab in the right dock panel (`DOCK_TABS`). This introduced two major downsides:
1. **Dock Clutter & Upstream Divergence:** Adding `pubgen` to `DOCK_TABS` altered the dock layout, changed default column tab counts, and required downstream overrides in `panel_docking/tests.rs`.
2. **Horizontal Constraint Friction:** Complex forms (multi-column CSV mapping, full-featured cover design, table styling, and multi-sheet options) felt cramped inside a narrow 260pt dock column.

### Implemented Architecture:
- **Dedicated Top-Level Menu:** A `Pubgen` top-level menu in the main app bar (positioned right beside `Window` and `Help`).
- **Modal Popup Windows:** Every form option (Excel options, TOC builder, Cover generator, Header/Footer setup, Manual CSV/Table, AI Terminology) opens as a **floating / modal popup dialog** with draggable title bars, vertical scrolling, full field visibility, and standard `OK / Cancel / Generate` actions.
- **Complete Side Panel Removal:** `pubgen` was removed from `DOCK_TABS`, returning the dock strictly to 100% Adobe InDesign parity (`Properties`, `Pages`, `Layers`).

```
┌────────────────────────────────────────────────────────────────────────┐
│ File  Edit  Layout  Type  Object  Table  View  Window  Pubgen  Help   │
└──────────────────────────────────────────────────────────┬─────────────┘
                                                           │
                                ┌──────────────────────────▼────────────────────────────────┐
                                │ Master Excel Import & Generate…                           │
                                │ Table of Contents…                                        │
                                │ Publication Cover…                                        │
                                │ Header & Footer Setup…                                    │
                                │ ───────────────────────────────────────────────────────── │
                                │ ▶ Manual Ingest & Tables                                  │
                                │     Page from CSV…                                        │
                                │     Table from CSV…                                       │
                                │ ▶ Page & Layout Utilities                                 │
                                │     Set Standard BPS Margins (20/20/19/15 mm)             │
                                │     Page Margins & Renumbering Options…                   │
                                │ ───────────────────────────────────────────────────────── │
                                │   ✓ Dual Language Mode (Bilingual ID / EN)                │
                                │   AI Translation Setup…                                   │
                                │   Publication Compile Wizard…                             │
                                │ ───────────────────────────────────────────────────────── │
                                │   About PubGen…                                           │
                                └───────────────────────────────────────────────────────────┘
```

---

### Language Mode Placement & Workflow:
- **Location 1 (Top-Level Pubgen Menu):** A checkable toggle `✓ Dual Language Mode (Bilingual ID / EN)`. Clicking it toggles `st.bilingual_output` instantly with live checkmark feedback.
- **Location 2 (Inside Generation Dialogs):** Prominently placed inside the **Master Excel Import Dialog** and **Manual Ingest Dialog** so users can verify the mode immediately before running generation.

---

### Set Page Margins Placement & Workflow:
- **Location 1 (Pubgen Menu › Page & Layout Utilities):** `Set Standard BPS Margins (20/20/19/15 mm)` is a 1-click direct action that runs `execute_set_page_margins(app)` immediately across all spreads and parent pages (Top 20.20 mm, Bottom 20 mm, Inside 19 mm, Outside 15 mm, while safely preserving 0-margin Cover and Divider pages).
- **Location 2 (Page Margins & Renumbering Dialog):** Inside the `Page Margins & Renumbering Options…` popup window, custom margin controls and section renumbering options are available.

---

## 2. Zero-Conflict Upstream Strategy (Option 2)

To ensure seamless upstream syncs and git rebases from the original DesignCraft repository:

1. **`crates/ui-egui/src/menus/tables.rs` remains 100% UNTOUCHED.** Upstream's high-traffic static `MENUS` array has zero modifications.
2. **`crates/ui-egui/src/dock_extended.rs` is our fork's owner file.** All Pubgen menu rendering, submenus, state access, and dialog dispatching live here.
3. **Only two 1-line hooks exist in upstream files:**
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
4. **Dock Tests Returned to 100% Upstream Parity:** Removing `pubgen` from `DOCK_TABS` allows `crates/ui-egui/src/panel_docking/tests.rs` to revert to upstream standard assertions (`["properties", "pages", "layers"]`).

---

## 3. Popup Windows & Form Options Design

All existing card contents from the side panel are modularized into dedicated popup dialogs:

| Dialog | Trigger Menu Item | Size (WxH) | Key Content & Actions |
|---|---|---|---|
| **Excel Ingest & Generate** | `Master Excel Import & Generate…` | 480 × 520 | File picker, sheet detection summary, auto-generate checkbox, Single/Dual Lang toggle, AI translate toggle, `[Convert & Generate]` |
| **Table of Contents Options** | `Table of Contents…` | 460 × 520 | TOC Style (Entry, Table, Chart, Paragraph), Layout (Standard, TablePage), title field, hanging indent, page filter words, pair bilingual checkbox, `[Add TOC]`, `[Rebuild TOC]` |
| **Cover Generator** | `Publication Cover…` | 540 × 640 | Cover preset styles (12 styles), Colors (Primary, Accent, Dark), metadata overrides (Title 1, Title 2, Year, Volume, Agency), image slots (Hero, Skyline, Logo), AI Prompt & Palette, `[Generate Cover]` |
| **Header & Footer Setup** | `Header & Footer Setup…` | 480 × 540 | Style preset, sided rules, scope (All, Section, Range), rule weight, accent bar, colors, `[Apply H/F]`, `[Remove H/F]`, `[Save Template]` |
| **Manual CSV / Table** | `Manual Ingest & Tables ▶` | 520 × 580 | Tab 1: Page from CSV (column mappings, template resolution, range). Tab 2: Table from CSV (theme, row conditions, column widths). `[Generate Pages]`, `[Insert Table]` |
| **Page Utilities** | `Page Margins & Renumbering Options…` | 420 × 440 | BPS margin reset (20/20/19/15 mm), section renumbering start, Page from RTF, Page from Image |
| **AI Translation Setup** | `AI Translation Setup…` | 460 × 500 | `ai::AiSetupDialog`: Groq API key, model selection, BPS Glossary table editor, search & draft auto-fill |
| **Compilation Wizard** | `Publication Compile Wizard…` | 480 × 520 | Manifest path picker, chapter plans, PDF export |
| **About PubGen** | `About PubGen…` | 420 × 380 | `about_pubgen_dialog`: Logo, author attribution, BPS Karawang info |

---

## 4. Modal Dialog Implementation Pattern

Dialogs use DesignCraft's standard `modal_drag::modal` centered layout with draggable title bars:

```rust
// In crates/ui-egui/src/panels/pubgen/dialogs.rs:

pub fn pubgen_dialogs(app: &mut DesignApp, ctx: &egui::Context) {
    with_state(ctx, |st| {
        // 1. Poll background jobs (AI translation, AI covers, publication compiler)
        poll_active_jobs(app, st);

        // 2. Show AI setup modal if open
        if st.ai_dialog.is_some() {
            egui::Area::new(egui::Id::unique("pubgen_ai_dialog_area")).show(ctx, |ui| {
                ai::dialog_ui(app, st, ui);
            });
        }

        // 3. Show AI quota popup if any
        if st.ai_quota_popup.is_some() {
            egui::Area::new(egui::Id::unique("pubgen_quota_popup_area")).show(ctx, |ui| {
                ai_quota_popup_ui(st, ui, &Tokens::get(ctx));
            });
        }

        // 4. Render all active popup dialogs
        show_all_dialogs(app, ctx, st);
    });
}
```

Each dialog features:
- Draggable header bar with title and `✕` close button
- Centered auto-placement within the viewport
- Vertical scroll area (`egui::ScrollArea::vertical().max_height(...)`)
- Non-blocking: background jobs run smoothly in the background while dialogs remain responsive

---

## 5. Verification & Test Results

All quality gates and automated verification tests passed:

- [x] **Cargo Check (pubgen feature):** `cargo check -p designcraft-ui-egui --features pubgen` passes cleanly with 0 errors.
- [x] **Cargo Check (upstream non-feature):** `cargo check -p designcraft-ui-egui` passes cleanly with 0 errors.
- [x] **Workspace Check:** `cargo check --workspace` passes cleanly across all 23 crates.
- [x] **Unit & Integration Tests:** `cargo test -p designcraft-ui-egui --features pubgen` — **1,015 passed, 0 failed**.
- [x] **Docking Layout Tests:** All 48 `panel_docking::tests` pass with 0 failures.
- [x] **Architectural Layering:** `cargo xtask layers` verified: `OK: 23 crates, no layering violations`.
- [x] **Formatting:** `cargo fmt` verified and formatted cleanly.
- [x] **Git Delivery:** Committed as `8cd99e51` and pushed to remote branch `origin/feature/pubgen-panel-ux-redesign`.
