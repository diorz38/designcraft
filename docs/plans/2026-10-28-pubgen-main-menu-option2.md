# Plan: Pubgen Dedicated Top-Level Menu (Option 2)

- **Date:** 2026-10-28
- **Author:** Antigravity Agent & diorz38
- **Branch:** `feature/pubgen-panel-ux-redesign`
- **Target files:** `dock_extended.rs`, `menus/bar.rs`, `menus/run.rs`
- **Status:** Planned

---

## 1. Goal

Add a **dedicated `Pubgen` top-level menu** to the main menu bar, isolated in `dock_extended.rs` (the pubgen-owned file), injected via a single `#[cfg(feature = "pubgen")]` call in `menus/bar.rs`.

This avoids editing the upstream `MENUS` static array entirely — zero risk of merge conflicts when the main repo updates `menus/tables.rs`.

---

## 2. Architecture

### Injection point

**`crates/ui-egui/src/menus/bar.rs` — `menu_bar()` function (line 85–94)**

```rust
// CURRENT:
pub fn menu_bar(app: &mut DesignApp, ui: &mut egui::Ui) {
    let lang = app.ui.language.clone();
    let mut menus = menu_tree();
    if crate::i18n::is_rtl(&lang) {
        menus.reverse();
    }
    for (menu, entries) in menus {
        menu_button(ui, crate::rtl::widget(ui, crate::i18n::tr(&lang, menu)), |ui| {
            menu_column(app, ui, &[(menu, entries)]);
        });
    }
}

// PATCH (add ONE cfg block after the for loop):
pub fn menu_bar(app: &mut DesignApp, ui: &mut egui::Ui) {
    // ... existing code unchanged ...
    for (menu, entries) in menus {
        menu_button(ui, crate::rtl::widget(ui, crate::i18n::tr(&lang, menu)), |ui| {
            menu_column(app, ui, &[(menu, entries)]);
        });
    }
    // Pubgen dedicated menu — lives entirely in dock_extended.rs (no upstream conflict)
    #[cfg(feature = "pubgen")]
    crate::dock::pubgen_menu(app, ui);
}
```

### How UI commands work (from `menus/run.rs`)

Menu items dispatch through `app.run(id, params)`, which routes:
- `"window.panel"` → `panel_docking::command` → activates panel tab
- `"toc.generate"` / `"toc.update"` → engine commands (already in Layout menu)
- `"pubgen.compileProject"` → `crate::dock::pubgen::compile_project(app, p)` (already wired in `run.rs` line 159–160)
- New `"pubgen.*"` UI commands → add to `run_ui_inner` match arm in `menus/run.rs` with `#[cfg(feature = "pubgen")]`

---

## 3. Complete Function Inventory of the Pubgen Panel

### 3.1 Panel Sections (`mod.rs`)

| Section function | UI call | Engine command called |
|-----------------|---------|----------------------|
| `pubgen_header_band()` | renders header with lang toggle + AI status | — (state mutation) |
| `master_excel_ui()` | renders Section 1: Excel import + auto-generate | `execute_excel_and_generate()` → file picker → generate pages |
| `card_toc_ui()` | renders Section 2: TOC controls | `execute_add_toc()`, `execute_rebuild_toc()` |
| `manual_import_accordion_ui()` | renders Section 3: collapsible Cover/CSV/HF | delegates to sub-UIs |
| `page_utilities_ui()` | renders Section 4: page utils | `execute_renumber_page()`, `execute_set_page_margins()` |
| `about_pubgen_dialog()` | About PubGen modal | — |
| `ai_quota_popup_ui()` | AI quota remaining popup | — |
| `compile_card_ui()` | Compilation project card | `compile_project()` |

### 3.2 Excel Module (`excel.rs`)

| Function | Role |
|----------|------|
| `execute_excel_to_csvs(st, path)` | Converts `.xlsx` → CSVs, updates `PubgenState` |
| `execute_excel_and_generate(app, st, path)` | Full pipeline: Excel → CSV → generate all pages |
| `execute_download_csv_zip(app, st)` | Packages generated CSVs as ZIP for download |
| `master_excel_ui(app, ui, st, t)` | **UI renderer** for Section 1 |
| `parse_xlsx_sheets(path)` | Low-level: parses `.xlsx` into `ExcelSheetData[]` |
| `convert_excel_to_csv_data(sheets, stem)` | Converts sheets → `ExcelConversionResult` |
| `execute_convert_excel(path)` | High-level conversion entry point |
| `apply_excel_cover(st, cover, base_dir)` | Applies cover data from Excel `COVER` sheet to state |
| `cover_style_index(spec)` | Maps cover style name → index |
| Many `parse_*_sheet()` fns | Specialized parsers: cover, table, chart, divider, RTF, note sheets |

### 3.3 TOC Module (`toc.rs`)

| Function | Role |
|----------|------|
| `execute_add_toc(app, st)` | **Main action**: generates TOC on new page after selection |
| `execute_rebuild_toc(app, st)` | **Main action**: rebuilds all existing TOCs |
| `rebuild_all_tocs(app, st)` | Low-level: rebuilds all TOC stories |
| `generate_toc(app, st, page, plan, replace)` | Core generator: produces TOC on a page |
| `enrich_toc_story(...)` | Styles and formats the generated TOC story |
| `ensure_toc_entry_style(...)` | Creates/updates TOC paragraph styles |
| `toc_kind_words(kind)` | Maps kind index → page-title filter words |
| `toc_kind_title(kind, bilingual)` | Default title per TOC kind + lang mode |
| `toc_kind_of_words(words)` | Reverse: words → kind index |
| `toc_story_on_page(app, page)` | Finds the TOC story on a given page |
| `page_margin_width(app, page)` | Width of the page within its margins |
| `apply_toc_post_process(...)` | Post-processing: continuation headers, styles |
| `add_toc_self_entry(...)` | Inserts a self-referencing TOC header row |

### 3.4 Covers Module (`covers.rs`)

| Function | Role |
|----------|------|
| `execute_generate_front_cover(app, st)` | Generates the front cover spread |
| `execute_generate_back_cover(app, st)` | Generates the back cover page |
| `execute_generate_inside_cover(app)` | Generates the inside front cover |
| `execute_generate_cover(app, st)` | Full cover generation (front + back + inside) |
| `trigger_generate_cover(app, st, ctx)` | Triggers AI cover generation job |
| `apply_ai_cover_result(app, st, bg_path)` | Applies a generated AI cover image |
| `replace_existing_covers(app, st)` | Removes existing cover pages before regenerating |
| `find_existing_covers(doc)` | Finds existing cover page indices |
| `apply_report_palette(st)` / `apply_urban_palette(st)` | Quick palette presets |

### 3.5 Pages Module (`pages.rs`)

| Function | Role |
|----------|------|
| `execute_generate_pages(app, st)` | **Core**: generates all publication pages from CSV catalog |
| `resolve_template_path_spec(catalog_path, spec)` | Resolves template file path |

### 3.6 Page Utilities (`page_utils.rs`)

| Function | Role |
|----------|------|
| `page_utilities_ui(app, ui, st, t)` | **UI renderer** for Section 4 |
| `execute_renumber_page(app, st)` | Renumbers pages from selected page |
| `execute_continue_numbering(app, st)` | Continues numbering from previous section |
| `execute_set_page_margins(app)` | Adjusts page margins |
| `execute_page_from_rtf(app, st, path)` | Inserts page from RTF file |
| `execute_page_from_image(app, st, path)` | Inserts page from image file |

### 3.7 Compile Module (`compile.rs`)

| Function | Role |
|----------|------|
| `compile_project(app, p)` | Compiles `pubgen_project.json` → full document |
| `poll_compile_job(app, st)` | Polls async compilation job, updates status |
| `compile_card_ui(app, ui, st, t)` | **UI renderer** for the compiler card |
| `compile_new_project_dialog_ui(...)` | UI for creating a new project manifest |

### 3.8 AI Module (`ai.rs`)

| Function | Role |
|----------|------|
| `dialog_ui(app, st, ui)` | AI setup dialog (Groq / Offline / Terminology tabs) |
| `fill_blank_drafts(app, st, ctx)` | AI-fills blank bilingual drafts |
| `collect_translation_units(st)` | Gathers all translatable strings |
| `apply_page_result(app, st, target)` | Applies AI translation to a page |
| `apply_table_page_result(app, st, target)` | Applies AI translation to a table page |
| `push_groq_quota_popup(st)` | Shows Groq quota remaining popup |
| `backend_from_prefs(app)` | Reads AI provider + key from preferences |

### 3.9 Helpers (`helpers.rs`)

| Function | Role |
|----------|------|
| `target_page_index(app)` | Returns current page index (0-based) |
| `ensure_para_style(app, name)` | Creates paragraph style if absent |
| `frame_with_style(app, spread_idx, rect, text, style)` | Creates a styled text frame |
| `resolve_catalog_related_file(catalog_path, file_str, ext)` | Resolves relative CSV-referenced paths |
| `place_catalog_image(...)` | Places an image from catalog |
| `harmonize_signatory_block(...)` | Adjusts signatory/signature frame sizing |
| `story_is_overset(app, sid)` | Checks if a story is overset |

### 3.10 Locale (`locale.rs`)

| Type/Fn | Role |
|---------|------|
| `enum Locale { Id, En }` | Single-lang mode target locale |
| `struct LocaleWords` | Vocabulary per locale (table, figure, chapter, TOC titles, etc.) |
| `ID_WORDS`, `EN_WORDS` | Static locale word tables |

### 3.11 Key Engine Commands (from `crates/engine/src/cmd/`)

| Command ID | Label | What it does |
|-----------|-------|-------------|
| `toc.generate` | Table of Contents… | Generates a new TOC (entries, layout, title, page) |
| `toc.update` | Update Table of Contents | Regenerates all TOCs in document |
| `toc.entries` | TOC Entries (query) | Returns TOC entries without modifying document |

> **Note:** The Pubgen panel does NOT call `toc.generate`/`toc.update` directly — it uses its own `execute_add_toc()` / `execute_rebuild_toc()` which build a richer BPS-specific TOC via `generate_toc()` in `toc.rs`. The engine commands are the standard InDesign-compatible ones; pubgen's TOC is a superset.

---

## 4. Proposed `Pubgen` Menu Structure & Form Options

```
┌─────────────────────────────────────────────────────────────┐
│ Pubgen                                                      │
├─────────────────────────────────────────────────────────────┤
│  Show Pubgen Panel                 (Cmd+Shift+G)            │
│  Compile Publication…              (pubgen.compileProject)  │
│  ─────────────────────────────────────────────────────────  │
│  ▶ Data Source & Generate                                   │
│     Excel to CSV Data…             (Pick .xlsx & Ingest)    │
│     Generate All Pages             (Batch generation)       │
│     Download Generated CSVs (ZIP)                           │
│  ─────────────────────────────────────────────────────────  │
│  ▶ Table of Contents (TOC)                                  │
│     Add Table of Contents…         (New TOC on active/next) │
│     Rebuild Table of Contents      (Update existing TOC)    │
│     Rebuild All Document TOCs      (Sync after renumber)    │
│  ─────────────────────────────────────────────────────────  │
│  ▶ Cover & Front Matter                                     │
│     Generate Front & Back Cover                             │
│     Cover Form Options…            (Focus Cover Form Card)  │
│  ─────────────────────────────────────────────────────────  │
│  ▶ Settings & Options                                       │
│     AI Translation & Terminology…  (AI Setup Dialog Modal)  │
│     Header & Footer Templates…     (Focus H/F Card)         │
│     Page Margins & Renumber…       (Focus Utilities Card)   │
│  ─────────────────────────────────────────────────────────  │
│  About PubGen…                     (About Dialog Modal)     │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. How Form Options & Dialogs Are Called

In DesignCraft, there are three distinct patterns for opening form options from a menu:

### Pattern A: Direct Modal Dialogs
These dialogs are rendered as centered draggable modal popups on the screen:
1. **AI Setup & Terminology Dialog**:
   - Calling mechanism: In `PubgenState`, set `st.ai_dialog = Some(ai::AiSetupDialog::open(app, &st));`
   - UI renderer: `ai::dialog_ui(app, &mut st, ui)` runs in the main UI loop when `st.ai_dialog.is_some()`.
   - Allows users to configure Groq API keys, model parameters, offline glossary terms, and view terminology translations.
2. **About PubGen Dialog**:
   - Calling mechanism: In `PubgenState`, set `st.show_about_dialog = true;`
   - UI renderer: `about_pubgen_dialog(app, ctx, &mut st.show_about_dialog, &t)`.
3. **New Compilation Project Wizard**:
   - Calling mechanism: In `PubgenState`, set `st.compile_new_dialog = Some(compile::NewProjectDialog::default());`
   - UI renderer: `compile_new_project_dialog_ui(app, ui, &mut st, &t)`.

### Pattern B: Form Activation via Dock Panel
For forms that live in the panel cards (Cover options, CSV column mapping, Table formatting, Header/Footer):
1. **Panel Activation**:
   - Calling `app.run("window.panel", json!({"panel": "pubgen"}))` activates and un-minimizes the Pubgen tab in the dock.
2. **Direct Accordion Expansion & Section Focus**:
   - In `PubgenState`, we add/utilize section expand flags (e.g. `st.manual_accordion_open = true`, `st.cover_accordion_open = true`).
   - When the user selects "Cover Form Options…" from the menu, the menu handler:
     1. Activates the pubgen panel (`window.panel`).
     2. Sets `st.cover_accordion_open = true` in `PubgenState`.
     3. Requests UI repaint so the user immediately lands with the form open and visible!

### Pattern C: Native OS File Dialog + Direct Pipeline
For actions like "Excel to CSV Data…":
1. Open native OS file picker: `if let Some(path) = app.services.pick_open.as_mut().and_then(|f| f("excel"))` or via `rfd`.
2. Directly execute `execute_excel_and_generate(app, &mut st, &path)`.
3. Post notification/status to `st.excel_status` and update document immediately.

---

## 6. The State Access Pattern in Menu Callbacks

Because `pubgen_menu(app, ui)` takes `ui: &mut egui::Ui`:
- `ui.ctx()` gives immediate access to the egui `Context`.
- We can inspect or update `PubgenState` cleanly through a small helper in `dock_extended.rs`:

```rust
// In dock_extended.rs:
pub(crate) fn with_pubgen_state<F, R>(ctx: &egui::Context, f: F) -> R
where
    F: FnOnce(&mut PubgenState) -> R,
{
    let id = egui::Id::unique(pubgen::PANEL_ID);
    let mut st: PubgenState = ctx.data(|d| d.get_temp(id)).unwrap_or_else(|| PubgenState {
        bilingual_output: true,
        cover_include_inside_back: true,
        ..Default::default()
    });
    let result = f(&mut st);
    ctx.data_mut(|d| d.insert_temp(id, st));
    result
}
```

This guarantees that clicking any menu item (like "Add TOC" or "AI Translation…") has synchronous, safe, single-source-of-truth access to the exact same `PubgenState` used by the dock panel.

---

## 7. Implementation Phases

### Phase 1 — Menu skeleton (no functionality, just structure)
- [ ] Add `pub fn pubgen_menu(app: &mut DesignApp, ui: &mut egui::Ui)` to `dock_extended.rs`
- [ ] Add `#[cfg(feature = "pubgen")] crate::dock::pubgen_menu(app, ui);` to `menus/bar.rs` after the `for` loop
- [ ] Add `"Show Pubgen Panel"` item → calls `app.run("window.panel", json!({"panel":"pubgen"}))`
- [ ] Add `"Compile Publication…"` item → calls `app.run("pubgen.compileProject", json!({}))`
- [ ] Verify: menu renders, panel opens, compile works

### Phase 2 — Data Source submenu
- [ ] Add `pubgen.excelDialog` to `UI_COMMANDS` (`#[cfg(feature = "pubgen")]`) and `run_ui_inner`
- [ ] Handler: opens OS file picker, calls `execute_excel_and_generate()` with pubgen state
- [ ] Add `pubgen.generatePages` → reads state, calls `execute_generate_pages()`
- [ ] Add `pubgen.downloadCsvs` → reads state, calls `execute_download_csv_zip()`
- [ ] Add Data Source submenu with these 3 items to `pubgen_menu()`

### Phase 3 — TOC submenu
- [ ] Add `pubgen.addToc` → reads state, calls `execute_add_toc()`
- [ ] Add `pubgen.rebuildToc` → reads state, calls `execute_rebuild_toc()`
- [ ] Add `enabled` guard: `pubgen.rebuildToc` enabled only when document has a TOC
- [ ] Add TOC submenu to `pubgen_menu()`

### Phase 4 — Cover & Utilities submenus
- [ ] Add `pubgen.generateCover` → reads state, calls `execute_generate_cover()`
- [ ] Add `pubgen.renumberPage` → reads state, calls `execute_renumber_page()`
- [ ] Add these submenus to `pubgen_menu()`

### Phase 5 — `enabled` guards & Quick Apply
- [ ] `pubgen.generatePages` enabled only when `st.catalog_csv_path` is non-empty
- [ ] `pubgen.addToc` / `pubgen.rebuildToc` enabled only when document is open
- [ ] Add pubgen commands to `Quick Apply` (they're auto-included from `UI_COMMANDS`)

---

## 8. Files Changed (complete list)

| File | Change type | Upstream conflict risk |
|------|------------|----------------------|
| `crates/ui-egui/src/dock_extended.rs` | Add `pubgen_menu()` fn + helper | **None** (pubgen-owned) |
| `crates/ui-egui/src/menus/bar.rs` | +1 `#[cfg]` line after `for` loop | **Very low** (stable code) |
| `crates/ui-egui/src/menus/run.rs` | Add `#[cfg(feature="pubgen")]` match arms | **Low** (add at end of match) |
| `crates/ui-egui/src/menus/ui_commands.rs` | Add `#[cfg(feature="pubgen")]` UI_COMMANDS entries | **Low** (append to end) |

> All changes in `tables.rs` (the high-risk upstream file) are **avoided entirely**.

---

## 9. Conflict Avoidance Rules

1. **Never add to `MENUS` in `tables.rs`** — the entire Pubgen menu lives in `dock_extended.rs`
2. **All new `UI_COMMANDS` entries use `#[cfg(feature = "pubgen")]`** and are appended at the end
3. **All new `run_ui_inner` match arms use `#[cfg(feature = "pubgen")]`** and are at the end of the match
4. **One touch in `bar.rs`**: a single line after the `for` loop, stable and easily resolved

---

## 10. Open Questions

- [ ] **Q1:** Does `DesignApp` expose `egui::Context` directly, or must state commands come from a UI context? → Inspect `DesignApp` struct in `crates/ui-egui/src/lib.rs`
- [ ] **Q2:** Should `pubgen.excelDialog` activate the panel first (so the user sees progress in the panel), or silently execute and show a status bar message?
- [ ] **Q3:** Keyboard shortcut for `Show Pubgen Panel`? Suggestion: `Cmd+Shift+G` (G for Generate)
- [ ] **Q4:** Where should the Pubgen menu appear in the bar? Recommendation: **between `Window` and `Help`** (after all domain menus, before help)

---

## 11. Quick Verification After Each Phase

```bash
cargo xtask ci           # fmt + clippy + tests + assets + layers + wasm
cargo test -p designcraft-ui  # panel_docking tests (tests.rs assertions on tab groups)
```

Key test to watch: `tests.rs` line 585–592 asserts panel group membership — the `pubgen` tab already appears in expected lists under `#[cfg(feature = "pubgen")]`, so no change needed there.
