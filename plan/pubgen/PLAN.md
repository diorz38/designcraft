# Engineering Plan: Pubgen Dock Extension & Upstream Synchronization

- **Target System:** DesignCraft (`crates/ui-egui`, `crates/engine`, `crates/doc`)
- **PRD Reference:** [plan/pubgen/PRD.md](file:///f:/GitHub/designcraft/plan/pubgen/PRD.md)
- **Review Reference:** [plan/pubgen/plan-review.md](file:///f:/GitHub/designcraft/plan/pubgen/plan-review.md)
- **User Guide:** [docs/pubgen-guide.md](../docs/pubgen-guide.md)
- **Complete Guide:** [user_complete_guide.md](../user_complete_guide.md)
- **Dev / Ops Runbook:** [development_pubgen_feature.md](../development_pubgen_feature.md)
- **Status:** Implemented behind the `pubgen` feature flag; in maintenance mode (see [Known Gaps & Follow-ups](#known-gaps--follow-ups))
- **Last updated:** 2026-10-17

---

## Current State (verified against the tree)

The `pubgen` feature is fully wired and test-covered. Everything below is checked against the actual source, not the original PRD.

| Area | File / location | State |
| :--- | :--- | :--- |
| Feature flag | `crates/ui-egui/Cargo.toml` → `[features] pubgen = []` | Done |
| Module seam | `crates/ui-egui/src/lib.rs:16-20` (`#[cfg]` / `#[path]` on `pub mod dock`) | Done |
| Extended dock | `crates/ui-egui/src/dock_extended.rs` (clone of `dock.rs` + 4 hooks) | Done |
| Pubgen panel | `crates/ui-egui/src/panels/pubgen.rs` (~2920 lines incl. tests) | Done |
| Drift tool | `xtask/src/diff_dock.rs` + `diff-dock` subcommand in `xtask/src/main.rs:50` | Done |
| Unit tests | `crates/ui-egui/src/panels/pubgen.rs` `mod tests` | **24 tests** |
| Sample data | `plan/pubgen/samples/` | **9 files** (6 CSV + 1 `.designcraft` + 2 `.pdf`) |
| User docs | `docs/pubgen-guide.md`, `user_complete_guide.md` | Done |

Upstream baseline `crates/ui-egui/src/dock.rs` is **untouched**; `crates/ui-egui/src/panels/mod.rs` is **not modified** (`pubgen` is declared inside `dock_extended.rs` via `#[path]`).

---

## Architecture Principles & Upstream Isolation

1. **Right Dock Integration:** Pubgen is a tab in the right-hand expanded dock strip (`Properties` · `Pages` · `Layers` · `Pubgen`) inside `crates/ui-egui/src/dock_extended.rs`.
2. **Minimal Upstream Touch:** Only two core files are touched — `crates/ui-egui/Cargo.toml` (the `[features]` block) and `crates/ui-egui/src/lib.rs` (the 3-line cfg seam). `dock.rs` stays byte-identical to `upstream/main`, and `panels/mod.rs` is untouched.
3. **Layering Compliance:** No `rfd` in `ui-egui`. File dialogs route through `app.services.pick_open` (`pubgen.rs:131`, `:231`); unknown-path file reads use `std::fs::read_to_string` (`:639`, `:676`). `cargo xtask layers` passes.
4. **Three Dock Insertion Points + module decl** (`dock_extended.rs`):
   - Module decl: `#[path = "panels/pubgen.rs"] pub mod pubgen;` (lines 9-10)
   - `DOCK_TABS` entry `("pubgen", "Pubgen", "panel-pages")` (line 14)
   - Expanded-strip `ScrollArea` dispatcher arm `"pubgen" => pubgen::show(app, ui),` (line 122)
   - `panel_body` dispatcher arm (torn-off / floating panels) (line 276)
5. **Engine Reuse:** The engine already owns RFC 4180 parsing and data-source abstractions (`crates/engine/src/cmd/datamerge/`), plus every command Pubgen emits (`layout.pages.insert`, `frame.create`, `text.insert`, `table.convertFromText`, `table.options`, `style.table.create`, `style.table.apply`, `layout.documentSetup`).

---

## Phase Overview

```
Phase 1: Architecture Seam & Extended Dock Infrastructure     [DONE]
  ↓
Phase 2: Properties-Parity UI Shell (Cards 1 & 2)             [DONE]
  ↓
Phase 3: Card 1 Engine — "Page from CSV"                      [DONE]
  ↓
Phase 4: Card 2 Engine — "Table from CSV"                     [DONE]
  ↓
Phase 5: Upstream Synchronization Tooling (diff-dock)         [DONE]
  ↓
Phase 6: Verification, Themes & End-to-End Testing            [DONE]
  ↓
Phase 7: Card 3 — Quick Actions & Document Utilities          [DONE]
  ↓
Phase 8: Advanced Table Dev beyond the PRD                    [DONE]
```

---

## Phase 1: Architecture Seam & Extended Dock Infrastructure

**Goal:** zero-conflict compilation seam so `dock.rs` stays identical to upstream while `dock_extended.rs` compiles under `--features pubgen`.

### Tasks
- [x] **Task 1.1: Feature Flag Declaration** — `crates/ui-egui/Cargo.toml` `[features] pubgen = []`.
- [x] **Task 1.2: Conditional Module Seam** — `lib.rs` selects `dock` vs `#[path = "dock_extended.rs"] pub mod dock;` under `#[cfg(feature = "pubgen")]`.
- [x] **Task 1.3: Clone Baseline & Inject Hooks** — `dock_extended.rs` = copy of `dock.rs` plus the module decl and the three insertion points listed in Architecture §4.
- [x] **Task 1.4: Panel Module** — `crates/ui-egui/src/panels/pubgen.rs` created with Properties-parity widgets and persisted UI state (`PubgenState`, serialized under `pubgen_panel_state`).
  - **Correction vs earlier draft:** *no* i18n registration was added; the "Pubgen" label is not present in `crates/ui-egui/src/i18n.rs`, so it renders untranslated (falls back to the literal). Tracked in Known Gaps.
- [x] **Task 1.5: Compilation & CI Verification** — `cargo check -p designcraft-ui-egui`, `... --features pubgen`, and `cargo check -p designcraft --features pubgen` all pass.

---

## Phase 2: Properties-Parity UI Shell (Cards 1 & 2)

**Goal:** Pubgen panel uses the exact tokens, spacing and card conventions of the Properties panel (helpers live in `crates/ui-egui/src/widgets.rs`).

### Tasks
- [x] **Task 2.1: Header Band** — `selection_band(ui, "Pubgen")` (33pt + bottom rule); dimmed "no document open" state.
- [x] **Task 2.2: Card 1 UI — "Page from CSV"** — `section(ui, "Page from CSV", true)`; browse button via `app.services.pick_open` + manual path fallback; delimiter badge + column-detection badges (`type` / `number` / `title`); layout preset combo (**5 presets**, see Phase 3); record scope radios (`All` / `Range` with from/to); `Append to document` checkbox; `[ Generate Pages ]` outline button.
- [x] **Task 2.3: Section Rules** — `divider(ui)` / `rule(ui)` separate the cards.
- [x] **Task 2.4: Card 2 UI — "Table from CSV"** — `section(ui, "Table from CSV", true)`; browse button + live matrix dimensions (`N rows × M columns`); options `First row is header`, `Add column numbers (1), (2)…`, `Table footer (last row)`, `Condition rules (Jumlah, Tahun, Kab, Kec…)`; style combo (**4 styles**, see Phase 4); placement radios (`At cursor` / `New frame` / `At page`); live `Target:` badge; `[ Insert Table ]` outline button.
  - **Correction vs earlier draft:** style list is 4 (adds **Horizontal Rules**), placement is 3 options (adds **At page**, the default), and Card 2 carries three extra option checkboxes beyond the PRD.
- [x] **Task 2.5: Visual Alignment & Theme Verification** — spacing `vec2(6.0, 5.0)`, `FIELD_H = 21.0` fields, `outline_button`/`dropdown`/`rule`/`divider`; token-driven so it adapts across Dark / Medium Dark / Medium Light / Light.

---

## Phase 3: Card 1 Engine — "Page from CSV"

**Goal:** ingest CSV (`type, number, title`), compute frame geometry from page margins, generate styled pages.

### Tasks
- [x] **Task 3.1: CSV Ingestion** — `parse_csv_content` / `parse_csv_content_with_delim` (RFC 4180 quoting) with `detect_delimiter` auto-detection (frequency × consistency × header heuristics) over **comma / tab / semicolon**; `delimiter_name` for the badge. Header roles resolved case-insensitively with positional fallback: type ← `type|category|kicker|kind` (col 0), number ← `number|no|num|id|#` (col 1), title ← `title|name|headline|chapter` (col 2). Rows stored in `PubgenState` with status diagnostics.
- [x] **Task 3.2: Layout Preset Templates** — 5 presets, indices in `PubgenState::card1_preset`:
  - `0` **1 Row · 3 Columns** (default) — single header row; Type and Number frames fitted to estimated text width (`estimate_text_width`), Title takes the remaining margin width with trailing space.
  - `1` **1 Row · 3 Equal Cols** — 1/3 width each.
  - `2` **Editorial Chapter (Stacked)** — kicker / large numeral badge / full-width title frame, stacked from the top margin.
  - `3` **Technical Spec Card** — accent banner, numeral box, side-by-side kicker + headline.
  - `4` **Minimal Zine** — centered numeral and title with generous vertical rhythm.
  - *(Numeric offsets are computed in `execute_generate_pages`; the earlier draft's fixed pt values were illustrative and are not the source of truth.)*
- [x] **Task 3.3: Page & Frame Generator Pipeline** — per record: `layout.pages.insert {count:1}` → read the new page margins → `frame.create` per preset frame → `text.insert` with content/typography.
- [x] **Task 3.4: Undo Strategy** — sequential command pipeline; each created page and frame is individually undoable/redoable via standard history (`Ctrl+Z` / `Ctrl+Y`).
  - **Correction vs earlier draft:** there is **no single-step transaction**. The engine exposes no public batch/undo-group API, so a full generation batch is *not* one undo step. See Known Gaps.

---

## Phase 4: Card 2 Engine — "Table from CSV"

**Goal:** ingest a CSV matrix and build native DesignCraft tables with styled headers and borders.

### Tasks
- [x] **Task 4.1: CSV Matrix Parser** — 2D `Vec<Vec<String>>` in `card2_matrix`; row/col counts shown in the panel; header row extracted when `First row is header` is set.
- [x] **Task 4.2: Table Integration** — matrix rendered to tab-delimited text then `table.convertFromText {columnSeparator:"tab"}`; `table.options {headerRows, ...}` sets protected header/footer rows.
- [x] **Task 4.3: Table Style Recipes** — 4 styles: **Basic Grid**, **Corporate Navy**, **Minimalist**, **Horizontal Rules**. Non-custom styles applied via `style.table.create` + `style.table.apply`; **Horizontal Rules** is applied by direct document mutation (custom table style + `CellStroke` rules) so booktabs geometry is exact.
- [x] **Task 4.4: Placement Dispatcher** — `At cursor` (requires an active caret, else errors with guidance), `New frame` (centered frame in the page margin box), and `At page` (detects the bottom edge of the generated page title/header on the active/selected page and drops the table 20pt below, resizing height to fit — with a live `Target: Page N (below title)` badge). `target_page_index` resolves the active page.

---

## Phase 5: Upstream Synchronization Tooling (`cargo xtask diff-dock`)

### Tasks
- [x] **Task 5.1: `xtask/src/diff_dock.rs`** — loads `dock.rs` and `dock_extended.rs`, strips the known Pubgen hooks (module decl / `panels/pubgen.rs` path, the `"pubgen" => pubgen::show(app, ui),` arms, and the `DOCK_TABS` entry), normalizes whitespace, and compares line-by-line.
- [x] **Task 5.2: Subcommand Registration** — `diff-dock` listed in `xtask/src/main.rs` `USAGE` and dispatched (`main.rs:50`).
- [x] **Task 5.3: Output & Drift Reporting** — in sync prints `✓ dock.rs and dock_extended.rs are in sync (Pubgen hooks intact).`; drift prints the differing upstream lines to guide re-applying the hooks.
- [x] **Task 5.4: Upstream Sync Drill** — verified exit code 0 against the current baseline.

---

## Phase 6: Verification, Themes & End-to-End Testing

### Tasks
- [x] **Task 6.1: Card 1 Dataset Test** — `test_execute_generate_pages` covers margin-rect geometry, page insertion and frame generation; `test_load_kbda_sample_file_with_semicolon` exercises the real semicolon-delimited `KBDA2026_chapters.csv` sample.
- [x] **Task 6.2: Table Stress Test** — `test_execute_insert_table`, `..._at_page`, `..._with_col_numbers`, `..._horizontal_rules`, `..._auto_merged_headers`, `..._3stack_headers`, `..._footer_and_footnote`, `..._row_conditions_kbda_2005`, `..._intermediate_condition_rows`, and the on-disk `sample_3stack_table.csv` end-to-end test.
- [x] **Task 6.3: Theme & High-DPI** — all rendering flows through DesignCraft tokens (`Tokens::get()`, `section`, `divider`, `rule`, `outline_button`, `selection_band`); no hardcoded colours.
- [x] **Task 6.4: CI & Layering Verification** — `cargo xtask layers` (0 violations) and `cargo xtask assets` (attributed) pass; `cargo check` passes for the vanilla crate, `--features pubgen`, and the app; `cargo test -p designcraft-ui-egui --features pubgen -- pubgen` → **24 passed**.
  - **Correction vs earlier draft:** the pubgen suite is 24 tests, not 4.
- [x] **Task 6.5: Documentation** — `docs/pubgen-guide.md`, `user_complete_guide.md`, and `development_pubgen_feature.md`.

---

## Phase 7: Card 3 — Quick Actions & Document Utilities

**Goal:** in-panel document utilities, beyond the original PRD.

### Tasks
- [x] **Task 7.1: Facing-Pages Toggle** — detects the current layout mode and flips it via `layout.documentSetup {facingPages}`.
- [x] **Task 7.2: Add Blank Page** — `layout.pages.insert {after, count:1}` relative to the active/selected page, then scrolls the canvas to it (`crate::canvas::go_to_page`).
- [x] **Task 7.3: Live Metadata Indicator** — status line (`Mode: Double-sided · Selected: Page N`) plus per-action status messages (`card3_status`).
- [x] **Task 7.4: Tests** — `test_quick_actions_switch_facing_pages`, `test_quick_actions_insert_blank_page`.
- [x] **Task 7.5: Table of Contents Rebuild & Self-Inclusion Toggle** — `execute_rebuild_toc` re-scans the document, updates all entry page numbers, re-enriches prefixes and hanging indents, and reflows continuation pages; `toc_include_self` toggle allows optional indexing of the TOC page itself; custom `toc_title` field.
- [x] **Task 7.6: Page Renumbering & Section Controls** — `execute_renumber_page` (`layout.section`) restarts numbering on selected page with arabic/roman styles and auto-refreshes TOC; `execute_continue_numbering` restores continuous numbering.
- [x] **Task 7.7: TablePage TOC Layout Style** — BPS statistical catalog layout ("Daftar Tabel"): `Tabel\thalaman` column header band in muted grey (`TOC TablePage Header`), standalone number column (`4.2.1`), hanging-indented title column (`TOC Level 1`), and bold section headers without page numbers or dot leaders (`TOC Section Header`).
- [x] **Task 7.8: Multi-Page TOC Continuation Headers** — for multi-page TOCs in both `TablePage` and `Standard` modes, automatically creates top-margin continuation headers (`Tabel\thalaman` in TablePage, or `<Title> (Continued)` in Standard) on every continuation page, adjusts threaded flow frames to begin cleanly below the header (`m.y0 + 24.0`), loops autoflow reflows, and cleanly de-duplicates/resyncs headers upon `Rebuild TOC`.
- [x] **Task 7.9: Page from RTF** — `execute_page_from_rtf` imports RTF documents directly onto the selected page or into an active text frame; supports automatic multi-page autoflow (`autoflow: true` with threaded text frames across margins and column guides); preserves RTF character styles and paragraphs; features inline path edit and Browse file picker in Quick Actions card.
- [x] **Task 7.10: Page from Image** — `execute_page_from_image` places images on selected or new pages with proportional fitting (`fitProportionally`), content centering (`centerContent`), tight frame boundary snapping (`fitFrameToContent`), and background layer ordering (`to: "back"` for full-page background images without cropping or overflow).

---

## Phase 8: Advanced Table Dev beyond the PRD

**Goal:** publication-grade table features added during development.

### Tasks
- [x] **Task 8.1: Multi-Level Stacked Headers** — `Group--Sub--Detail` header notation (double-dash) builds a stacked header grid up to **3 levels**, with horizontal merges for shared prefixes and vertical merges (with centering) for shallower columns.
- [x] **Task 8.2: Column Numbering** — optional `(1), (2), (3)…` row immediately after the header rows.
- [x] **Task 8.3: Table Footer** — the last data row becomes `RowKind::Footer`; with Horizontal Rules it gets a 0.5pt top rule and 1.0pt bottom rule.
- [x] **Task 8.4: Conditional Summary Rules** — rows whose first (or index-prefixed second) column starts with `Jumlah` / `Tahun` / `Kab` / `Kabupaten` / `Kec` / `Kecamatan` / `Total` receive 0.5pt framing rules (`matches_pubgen_row_condition`, `detect_condition_rows`).
- [x] **Task 8.5: Footnote / Source Auto-Detection** — a trailing row with text only in column 1 is excluded from the grid and emitted as a standalone text frame directly below the table (`detect_footnote_row`).
- [x] **Task 8.6: Delimiter Auto-Detection** — shared with Card 1.

---

## Known Gaps & Follow-ups

1. **No single-step undo for a generation batch.** `execute_generate_pages` issues commands sequentially, so N pages/frames produce N history entries. To match the PRD's "one `Ctrl+Z`" promise, move the generation into a single engine command (mirroring `data.merge`, which is one undo entry by construction) or add a transaction/undo-group API.
2. **Card 1 still duplicates Data Merge.** `data.merge` / `data.source.*` (`crates/engine/src/cmd/datamerge/`) already parse CSV, select record ranges and generate pages. Card 1 could delegate to that path instead of re-implementing CSV ingest in the UI.
3. **"Pubgen" label is not localized.** No entry in `crates/ui-egui/src/i18n.rs`; the dock tab/label renders as the literal English string.
4. **CI does not build the `pubgen` feature.** `cargo xtask ci` and `cargo xtask wasm` check default features, so the seam can rot silently. Consider adding a `cargo check/test --features pubgen` step (and a `diff-dock` gate) to `ci`.
5. **Docs drift.** `PRD.md` and `docs/pubgen-guide.md` still list the earlier 3 presets / 3 styles / 2 placements and the "single-step undo" claim; reconcile them with Phases 3/4/7/8 above.

---

## File Inventory

```
crates/ui-egui/
├── Cargo.toml                 [features] pubgen = []
└── src/
    ├── lib.rs                 cfg seam: dock vs dock_extended (lines 16-20)
    ├── dock.rs                100% upstream baseline (untouched)
    ├── dock_extended.rs       baseline + 4 pubgen hooks
    └── panels/
        ├── mod.rs             untouched
        └── pubgen.rs          panel UI, generators, parsers, 24 tests
xtask/src/
├── main.rs                    diff-dock dispatch (line 50)
└── diff_dock.rs               hook-stripping line diff
plan/pubgen/
├── PRD.md · PLAN.md · plan-review.md
└── samples/
    ├── sample_chapters.csv · sample_table.csv · sample_merge_col_table.csv
    ├── sample_3stack_table.csv · kbda_2005.csv · KBDA2026_chapters.csv
    └── Untitled-1.designcraft · Untitled-1.pdf · Untitled-2.pdf
```

---

## Maintenance Quick Reference

| Action | Command |
| :--- | :--- |
| Check dock drift | `cargo xtask diff-dock` |
| Compile the feature | `cargo check -p designcraft-ui-egui --features pubgen` |
| Run Pubgen tests | `cargo test -p designcraft-ui-egui --features pubgen -- pubgen` |
| Run the app with Pubgen | `cargo run -p designcraft --features pubgen` |
| Full quality gate | `cargo xtask ci` |
| Reconcile after upstream touches `dock.rs` | copy `dock.rs` → `dock_extended.rs`, re-apply the 4 hooks, re-run `diff-dock` |
