# Mini Plan: Pubgen Main Menu Replaces the Pubgen Panel

- **Date:** 2026-10-28
- **Status:** Proposed (not implemented)
- **Area:** `crates/ui-egui`, `apps/designcraft`, `xtask`
- **Feature gate:** `pubgen` (unchanged)
- **Goal:** add one top-level **Pubgen** menu whose entries expose every function of today's
  Pubgen dock panel, and remove the Pubgen tab from the dock. The classic menus and every
  DesignCraft command stay in place.
- **Non-goal:** removing or renaming DesignCraft features; moving Pubgen generation logic into
  the engine in this pass.

---

## 1. Executive summary

The Pubgen panel is a long form. A dropdown menu cannot host that form directly, so each menu
entry opens the corresponding card in a resizable window (or runs a zero-parameter action).
That is the whole UX change:

```
before: right dock tab "Pubgen" -> one long scroll of cards
after:  top menu "Pubgen"      -> one window per card, opened from the menu
```

The panel's code is reused, not rewritten. Upstream files receive only short, feature-gated,
append-style hooks. The existing `dock_extended.rs` fork is deleted, which *reduces* the local
diff against upstream.

---

## 2. Scope

### In scope

- New `Pubgen` top-level menu, inserted after `File`.
- Menu entries for all current panel functions (compiler, data/Excel, CSV pages, CSV tables,
  TOC, cover, manual imports, page utilities, AI actions, About).
- A dialog/window host that renders the existing card functions and keeps background jobs
  polling even when no Pubgen UI is visible.
- UI command table + dispatch so menu rows, the command palette, the control channel and MCP
  all reach the same handlers (palette/control parity is a later phase, menu is phase 1).
- Remove the Pubgen dock tab and the now-unnecessary `dock_extended.rs` seam.
- Tests, screenshots, and upstream-sync documentation.

### Out of scope

- Rewriting Pubgen generation as engine commands (`pubgen.pages` etc.). Tracked separately in
  `development_pubgen_feature.md` section 6.
- Redesigning the cards themselves.
- Translating the new labels into every interface language (labels go through `i18n::tr`;
  English fallback is acceptable for this pass).
- The in-progress vector chart engine UI (`panels/pubgen/charts/`). Reserve a `Charts`
  submenu for it, but do not wire it yet.

---

## 3. Current state (verified)

### Menu mechanics

| What | Where | Note |
| :--- | :--- | :--- |
| Menu bar drawing | `crates/ui-egui/src/chrome/app_bar.rs:234` | calls `menus::menu_bar(app, ui)` |
| Menu bar implementation | `crates/ui-egui/src/menus/bar.rs:85` | iterates `menu_tree()` |
| Tree data | `crates/ui-egui/src/menus/tables.rs:2` (`MENUS`) | upstream file, do not edit |
| Tree parser | `crates/ui-egui/src/menus/tree.rs:83` | returns `Vec<(&'static str, Vec<Item>)>` |
| Row rendering / dispatch | `crates/ui-egui/src/menus/tree.rs:179` | `Item::Cmd` -> `enable` -> `activate` |
| Activation | `crates/ui-egui/src/menus/invoke.rs:22` | unknown ids fall through to `app.run(id, params)` at line 120 |
| Enabled state | `crates/ui-egui/src/menus/state.rs:28` | unknown ids are enabled |
| UI command execution | `crates/ui-egui/src/menus/run.rs:20` | `run_ui(app, id, p) -> Option<Result<...>>` |
| Command palette / search | `crates/ui-egui/src/menus/quick_apply.rs:158` | engine specs + `UI_COMMANDS` |
| Menu introspection | `crates/engine` command specs + `control.rs:176` (`ui.menu.list`) | menu paths also feed shortcut import |

### Pubgen panel entry points

| Card | Function | File:line |
| :--- | :--- | :--- |
| Panel body / state / job polling | `show` | `panels/pubgen/mod.rs:75` |
| Header band (mode, AI, margin, about) | `pubgen_header_band` | `panels/pubgen/widgets.rs:164` |
| Publication compiler | `compile_card_ui` | `panels/pubgen/compile.rs:1516` |
| Master Excel | `master_excel_ui` | `panels/pubgen/excel.rs:3544` |
| Page from CSV | `card1_body_ui` | `panels/pubgen/widgets.rs:465` |
| Table from CSV | `card2_body_ui` | `panels/pubgen/widgets.rs:586` |
| TOC | `card_toc_ui` | `panels/pubgen/widgets.rs:1278` |
| Manual import accordion | `manual_import_accordion_ui` | `panels/pubgen/widgets.rs:1376` |
| Cover | `card_cover_body_ui` | `panels/pubgen/widgets.rs:740` |
| Page utilities | `page_utilities_ui` | `panels/pubgen/page_utils.rs:313` |
| AI setup dialog | `ai::dialog_ui` | `panels/pubgen/ai.rs:435` |
| Persisted panel state | `PANEL_ID` / `PubgenState` | `panels/pubgen/mod.rs:67`, `state.rs` |

### Dock seam (to be removed)

`crates/ui-egui/src/dock_extended.rs` contains exactly three Pubgen hooks:

- `#[path = "panels/pubgen/mod.rs"] pub mod pubgen;` (lines 9-10)
- `("pubgen", "Pubgen", "panel-pages")` in `DOCK_TABS` (line 17)
- `"pubgen" => pubgen::show(app, ui)` in `panel_body` (line 129)

`crates/ui-egui/src/lib.rs:16-20` selects `dock_extended.rs` under `#[cfg(feature = "pubgen")]`.
`crates/ui-egui/src/panel_docking/tests.rs` has 21 lines of local `#[cfg(feature = "pubgen")]`
expectations that assume the tab exists.

### Important consequence

The panel is not a list of commands yet. Most card actions call `execute_*` UI functions
directly. A menu row can therefore do one of two things:

1. open a dialog that reuses the card UI (the default in this plan), or
2. queue an intent that the dialog host executes with the same `PubgenState` the card uses.

Both are allowed; nothing is lost as long as every panel action has a menu entry that reaches
the same code.

---

## 4. Target menu

Insert after `File`, before `Edit`:

```
File
Pubgen
  Publication
    New Project Manifest...        opens Compiler window (new-project form)
    Open Project Manifest...       file picker + Compiler window
    --------------------------------------------------
    Compile                        runs the compile job
    Cancel Compile                 enabled while a job runs
    Compile Report...              opens Compiler window scrolled to status
  Data
    Import Excel Workbook...       picker + Excel card window
    Page from CSV...               Page-from-CSV window
    Table from CSV...              Table-from-CSV window
    Download Generated CSVs as ZIP...
    Translate Data with AI...      opens AI window
  Cover
    Cover Designer...              Cover card window (procedural + AI styles)
  Table of Contents
    TOC Options...                 TOC card window
    Insert TOC
    Rebuild TOC
  Page Utilities
    Single-sided / Double-sided    toggle, label shows the target state
    Add Blank Page                 inserts after the current page
    Header/Footer...               existing dialog
    Table Options...               existing dialog
    Numbering & Section Options... existing dialog
    Set Page Margin                applies the Pubgen margin preset
    Renumber Pages...              Page Utilities window
  AI
    AI Translator Settings...      existing AI setup dialog
    Translate This Page            direct action (needs a review/table page)
    Fix Glossary Terms             direct action
    Retranslate (Glossary)         direct action
    Translate Table Range...       opens the range section of the AI window
  ----------------------------------------------------
  Dual Language Output             checkable; toggles `PubgenState::bilingual_output`
  About PubGen                     existing modal
Edit
Layout
Type
...
```

Rules for the tree:

- Menu entries are **data** (`Item::Cmd`), built in `pubgen_ui/menu.rs`; no upstream table edits.
- Labels are explicit, so a provider id does not need to exist in `UI_COMMANDS` for the row to
  render. `menu_enabled` returns `true` for unknown ids; `activate` falls through to
  `app.run`, where the Pubgen dispatcher handles it.
- Existing engine/UI commands are reused wherever possible (`layout.pages.insert`,
  `layout.documentSetup`, `layout.section`, the header/footer and table-options dialogs).
- Dynamic labels are allowed (for example, "Switch to Single-sided"); avoid adding checkmark
  logic to `menus::state::checked` in phase 1.

### Panel -> menu mapping (authoritative)

| Panel element | Menu path | Handler | Mode |
| :--- | :--- | :--- | :--- |
| Header: Dual/Single Lang toggle | `Pubgen > Dual Language Output` | `pubgen.mode.bilingual` | direct |
| Header: AI Translator | `Pubgen > AI > AI Translator Settings...` | `pubgen.ai.setup` | dialog |
| Header: Translate this Page | `Pubgen > AI > Translate This Page` | `pubgen.ai.page` | intent |
| Header: Fix Terms | `Pubgen > AI > Fix Glossary Terms` | `pubgen.ai.fixTerms` | intent |
| Header: Retranslate (Glossary) | `Pubgen > AI > Retranslate (Glossary)` | `pubgen.ai.retranslate` | intent |
| Header: Translate Tables... | `Pubgen > AI > Translate Table Range...` | `pubgen.ai.range` | dialog |
| Header: Set Page Margin | `Pubgen > Page Utilities > Set Page Margin` | `pubgen.pages.margins` | direct |
| Header: About | `Pubgen > About PubGen` | `pubgen.about` | dialog |
| Compiler: manifest path + Open | `Publication > Open Project Manifest...` | `pubgen.project.open` | dialog |
| Compiler: New | `Publication > New Project Manifest...` | `pubgen.project.new` | dialog |
| Compiler: Compile / Cancel | `Publication > Compile` / `Cancel Compile` | `pubgen.compile`, `pubgen.compile.cancel` | intent |
| Compiler: summary / progress / status | shown inside the Compiler window | - | - |
| Excel: auto-generate, picker, status | `Data > Import Excel Workbook...` | `pubgen.data.excel` | dialog |
| Excel: Download ZIP | `Data > Download Generated CSVs as ZIP...` | `pubgen.data.zip` | direct |
| Page from CSV: all fields + Generate | `Data > Page from CSV...` | `pubgen.data.pageCsv` | dialog |
| Table from CSV: all fields + Insert | `Data > Table from CSV...` | `pubgen.data.tableCsv` | dialog |
| TOC: fields + Add/Rebuild | `Table of Contents > TOC Options...` | `pubgen.toc.options` | dialog |
| TOC: Insert | `Table of Contents > Insert TOC` | `pubgen.toc.insert` | direct |
| TOC: Rebuild | `Table of Contents > Rebuild TOC` | `pubgen.toc.rebuild` | direct |
| Manual import: Cover section | `Cover > Cover Designer...` | `pubgen.cover` | dialog |
| Manual import: Page/Table sections | same as the Data entries above | - | dialog |
| Utilities: facing pages | `Page Utilities > Single-sided / Double-sided` | `pubgen.pages.facing` | direct |
| Utilities: Add Blank Page | `Page Utilities > Add Blank Page` | `pubgen.pages.addBlank` | direct |
| Utilities: Header/Footer | `Page Utilities > Header/Footer...` | existing UI handler | direct |
| Utilities: Table Options | `Page Utilities > Table Options...` | existing UI handler | direct |
| Utilities: Numbering & Sections | `Page Utilities > Numbering & Section Options...` | `layout.section` | direct |
| Utilities: Renumber / Continue | `Page Utilities > Renumber Pages...` | `pubgen.pages.renumber` | dialog |
| Utilities: status line | shown inside the Page Utilities window | - | - |

`direct` means the dispatcher performs the action immediately (or queues an intent consumed at
the start of the next frame). `dialog` means it opens the matching window. `intent` means the
action needs `PubgenState` plus an egui context, so it is queued and executed by the host.

---

## 5. Architecture

### 5.1 New module

```
crates/ui-egui/src/pubgen_ui/
  mod.rs          feature gate, panel module declaration, shared ids
  menu.rs         builds Vec<Item> for the Pubgen menu
  commands.rs     PUBGEN_COMMANDS + enabled() + run()
  host.rs         window/dialog host + intent queue + job tick
  windows.rs      one function per window (Compile, Data, Cover, TOC, Utilities, AI)
```

`mod.rs` declares the existing panel implementation so its code is reused without editing
`panels/mod.rs` (upstream):

```rust
#[path = "../panels/pubgen/mod.rs"]
pub mod panel;

pub mod menu;
mod commands;
mod host;
mod windows;
```

References that currently use `crate::dock_extended::pubgen::...` (today only
`panels/pubgen/charts/spec.rs:10`) become `crate::pubgen_ui::panel::...`.

### 5.2 The four hooks

All hooks are compiled only with `--features pubgen` and are short and append-style.

**A. `crates/ui-egui/src/lib.rs`**

```rust
#[cfg(feature = "pubgen")]
pub mod pubgen_ui;
```

and the dock seam becomes plain `pub mod dock;` (the `dock_extended` branch is deleted).

**B. `crates/ui-egui/src/menus/bar.rs`** (inside `menu_bar`, after `let mut menus = menu_tree();`)

```rust
#[cfg(feature = "pubgen")]
{
    let at = menus.iter().position(|(m, _)| *m == "File").map_or(menus.len(), |i| i + 1);
    menus.insert(at, ("Pubgen", crate::pubgen_ui::menu::items(app, ui.ctx())));
}
```

**C. `crates/ui-egui/src/menus/run.rs`** (top of `run_ui_inner`)

```rust
#[cfg(feature = "pubgen")]
if let Some(r) = crate::pubgen_ui::commands::run(app, id, p) {
    return Some(r);
}
```

**D. `crates/ui-egui/src/app.rs`** (next to `dialogs::show(self, &ctx);`)

```rust
#[cfg(feature = "pubgen")]
crate::pubgen_ui::host::show(self, &ctx);
```

`host::show` runs every frame, on every platform, so compile/translation/AI-cover jobs keep
polling even when no Pubgen window is open. This is required: today the polling lives in
`panels/pubgen/mod.rs::show`, which only runs while the panel tab is drawn.

### 5.3 Command table and dispatch

`commands.rs` owns ids, labels and handlers:

```rust
pub const COMMANDS: &[(&str, &str)] = &[
    ("pubgen.project.new", "New Project Manifest..."),
    ("pubgen.project.open", "Open Project Manifest..."),
    ("pubgen.compile", "Compile"),
    ("pubgen.compile.cancel", "Cancel Compile"),
    ("pubgen.data.excel", "Import Excel Workbook..."),
    // ...
];
```

`run` returns `None` for ids it does not own, so the rest of the app is untouched:

```rust
pub fn run(app: &mut DesignApp, id: &str, p: &Value) -> Option<Result<Value, String>> {
    let handler = COMMANDS.iter().find(|(cid, _)| *cid == id)?;
    Some(match id {
        "pubgen.compile" | "pubgen.compile.cancel" | "pubgen.ai.page" | ... => {
            queue_intent(app, id); // consumed by host::show next frame
            Ok(Value::Null)
        }
        "pubgen.pages.addBlank" => {
            let page = crate::canvas::current_page(app).unwrap_or(0);
            app.run("layout.pages.insert", json!({ "after": page, "count": 1 }))
        }
        "pubgen.pages.facing" => {
            let facing = app.session.active().is_some_and(|s| s.doc.settings.facing_pages);
            app.run("layout.documentSetup", json!({ "facingPages": !facing }))
        }
        "pubgen.pages.margins" => crate::pubgen_ui::panel::execute_set_page_margins(app).map(Value::from).map_err(|e| e),
        _ => {
            open_window(app, id);
            Ok(Value::Null)
        }
    })
}
```

The intent queue uses egui temp data (no new serialized state):

```rust
fn queue_intent(app: &DesignApp, id: &str) {
    if let Some(ctx) = &app.egui_ctx {
        ctx.data_mut(|d| d.insert_temp(intent_id(), id.to_string()));
        ctx.request_repaint();
    }
}
```

`host::show` drains it with the same `PubgenState` the cards use:

- Intents that only open a window set `st.open_window = Some(kind)`.
- Intents that run an action call the corresponding `windows::*` action function.
- Actions that need `ctx` get the one passed to `host::show`.
- Every intent path is listed in `COMMANDS`, so a missing handler is a unit-test failure.

### 5.4 Dialog host

```rust
pub fn show(app: &mut DesignApp, ctx: &egui::Context) {
    let t = Tokens::get(ctx);
    let id = egui::Id::unique(panel::PANEL_ID);
    let mut st: PubgenState = ctx.data(|d| d.get_temp(id)).unwrap_or_else(default_state);
    drain_intents(app, ctx, &mut st);       // may set st.open_window
    panel::tick(app, &mut st, ctx);         // moved out of panel::show: jobs + quota popups
    panel::ai::dialog_ui(app, &mut st, &mut *ui); // existing AI setup modal
    if let Some(kind) = st.open_window.clone() {
        let mut open = true;
        egui::Window::new(kind.title())
            .id(egui::Id::unique(("pubgen_window", &kind)))
            .open(&mut open)
            .resizable(true)
            .default_width(560.0)
            .max_height(ctx.content_rect().height() - 80.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| windows::draw(app, ui, &mut st, kind, &t));
            });
        if !open { st.open_window = None; }
    }
    ctx.data_mut(|d| d.insert_temp(id, st));
}
```

Notes:

- `panel::tick` is a new function extracted from `show` lines 89-216. `show` calls it too, so the
  existing panel tests keep passing until the panel is deleted.
- `PubgenState` stays under the same egui id, so moving from dock to window preserves paths,
  mappings, status messages and in-flight jobs for the session.
- One window at a time keeps the state model simple; opening another replaces it (the panel was
  also a single scroll surface).
- The existing modals (`NewProjectDialog`, `AiSetupDialog`, quota popup, About) are rendered by
  the host, not inside the removed panel.

### 5.5 Behavior with no document

The compiler is the only card that works without an open document (it creates one). Menus that
need a document either:

- stay enabled and return the engine's normal error in the status line, or
- are disabled through `pubgen_ui::commands::enabled`.

Phase 1 shows all entries; phase 5 adds enabled checks for compile (no manifest), Add Blank Page
(no document), AI actions (no backend/no target) to mirror the panel's `add_enabled` behavior.

---

## 6. Removing the panel

1. Delete the three hooks from `dock_extended.rs` (module declaration, `DOCK_TABS` row,
   `panel_body` arm). The file then equals `dock.rs`; delete `dock_extended.rs`.
2. Revert `crates/ui-egui/src/lib.rs:16-20` to unconditional `pub mod dock;` and add the
   `pubgen_ui` line from 5.2.
3. Revert the 21 local lines in `crates/ui-egui/src/panel_docking/tests.rs` to upstream
   (remove every `#[cfg(feature = "pubgen")]` branch that mentions `pubgen`).
   Suggested check: `git diff upstream/main -- crates/ui-egui/src/panel_docking/tests.rs`
   should be empty afterwards.
4. Delete `xtask/src/diff_dock.rs` and its registration in `xtask/src/main.rs`; update the
   command list and the docs that mention `cargo xtask diff-dock`.
5. Keep `crates/engine/src/tests/file_guard.rs` entries: the same Pubgen modules still read
   files, now from windows instead of the dock.
6. `Window > Panels` automatically stops listing Pubgen because `DOCK_TABS` no longer has it.
   Saved layouts that mention `pubgen` must be checked: `panel_docking` filters layout ids
   against the known panel list, but add a test that an old layout containing `pubgen` loads
   without an empty tab or panic.
7. `https`/wasm: `pubgen_ui` is behind the `pubgen` feature; compile-only cards stay
   `#[cfg(not(target_arch = "wasm32"))]` and their menu rows are omitted on wasm.

Net effect on the upstream diff: `dock_extended.rs` disappears, the `lib.rs` dock seam
disappears, and `panel_docking/tests.rs` returns to upstream. This is a significant reduction
in future merge work.

---

## 7. Phases and tasks

### Phase 0 - Audit (S)

- [ ] List every interactive control in `panels/pubgen/*` and map it to a menu row in section 4.
- [ ] Flag actions that need `PubgenState` + `ctx` (translate, fix terms, retranslate, compile
      cancel, range translate) as intents.
- [ ] Confirm which actions already exist as engine/UI commands and can be reused.
- [ ] Confirm no action is reachable *only* from a panel widget that has no handler.

**Acceptance:** the mapping table in section 4 has a row for every interactive control; no
unmapped `outline_button` / `ui.button` remains in the panel source.

### Phase 1 - Menu skeleton (S)

- [ ] Create `pubgen_ui` with `mod.rs`, `menu.rs`, `commands.rs`, `host.rs`, `windows.rs`.
- [ ] Add the four hooks (5.2).
- [ ] Add `pubgen_ui::menu::items` with the full tree; every row maps to a command id in
      `COMMANDS`.
- [ ] Add `commands::run` with dialog/queue handlers; no panel removal yet.
- [ ] Keep the dock panel in place for comparison during this phase.
- [ ] Tests: menu contains `Pubgen` after `File`; every menu id is handled; no duplicate ids.

**Acceptance:** the new menu is visible and every entry opens its window or runs its action;
the dock panel still works; `cargo test -p designcraft-ui-egui --features pubgen` passes.

### Phase 2 - Windows (M)

Split cards into windows one at a time, keeping the panel as the reference:

- [ ] `windows::draw` dispatch for Compile, Data (Excel + Page CSV + Table CSV), Cover, TOC,
      Utilities, AI.
- [ ] Reuse `compile_card_ui`, `master_excel_ui`, `card1_body_ui`, `card2_body_ui`,
      `card_toc_ui`, `card_cover_body_ui`, `page_utilities_ui`.
- [ ] Move `panel::show` job polling into `panel::tick` and call it from the host.
- [ ] Render existing modals from the host (`ai::dialog_ui`, quota, new project, About).
- [ ] Set sensible default heights and scrolling for the large cards (Excel, Cover).
- [ ] Screenshot fixture per window (opt-in `CRAFT_UI_VISUAL_FIXTURES`).

**Acceptance:** every panel function is available from the menu; jobs progress with no Pubgen
window open; closing and reopening a window keeps state; no document -> no panic.

### Phase 3 - Remove the panel (S)

- [ ] Apply section 6.
- [ ] Remove/replace the `capture_pubgen_panel_fixture` test with menu/window fixtures.
- [ ] Revert `panel_docking/tests.rs` and the `diff-dock` tooling.
- [ ] Update `development_pubgen_feature.md`, `plan/pubgen/PLAN.md`, `ROADMAP.md`, and the user
      guide's "open the Pubgen tab" instructions to the new menu paths.

**Acceptance:** no Pubgen tab; both feature modes build; full test suite passes with
`--features pubgen`; no stale references to `dock_extended`/`diff-dock`.

### Phase 4 - Palette, control and macOS parity (S/M)

- [ ] Chain `pubgen_ui::commands::COMMANDS` into `menus::quick_apply::quick_commands()` so ⌘K
      and the app-bar search find the same entries.
- [ ] Chain them into `control::all_commands` (and optionally `ui.menu.list`) so scripts, MCP
      and the control channel see them.
- [ ] Add the same menu tree to the macOS native menu (`apps/designcraft/src/native_menu.rs`,
      built once at install; static labels only).
- [ ] Optional: add `pubgen_ui::commands::enabled` as a two-line hook in
      `menus/state.rs::enabled`.

**Acceptance:** `engine.commands`/`ui.inspect` show Pubgen actions; a control `ui.menu.invoke`
call opens the same window; macOS shows the menu; Windows/Linux unchanged.

### Phase 5 - Polish (S)

- [ ] i18n entries for the new labels (fallback English until translated).
- [ ] Menu item shortcuts if desired (Add Blank Page, Compile) via the keymap table.
- [ ] Empty-state messages in windows that need a document.
- [ ] Mixed-DPI / small-window screenshots.
- [ ] Update `docs/pubgen-guide.md` screenshots and text.

---

## 8. Upstream conflict budget

### Files touched in upstream modules

| File | Change | Size |
| :--- | :--- | :--- |
| `crates/ui-egui/src/lib.rs` | add `pubgen_ui`, remove dock cfg seam | ~5 lines, net negative |
| `crates/ui-egui/src/menus/bar.rs` | insert one tree entry | ~5 lines |
| `crates/ui-egui/src/menus/run.rs` | try Pubgen dispatch | ~3 lines |
| `crates/ui-egui/src/app.rs` | call the host | ~2 lines |
| `crates/ui-egui/src/menus/quick_apply.rs` | optional chain | ~1 line |
| `crates/ui-egui/src/control.rs` | optional chain | ~1 line |
| `crates/ui-egui/src/menus/state.rs` | optional enabled hook | ~2 lines |
| `apps/designcraft/src/native_menu.rs` | macOS menu insertion | ~2 lines |
| `crates/ui-egui/src/menus/tables.rs` | **not touched** | 0 |
| `crates/ui-egui/src/menus/tree.rs` | **not touched** | 0 |
| `crates/ui-egui/src/dock.rs` | **not touched** | 0 |

Everything else is new files or files that are already Pubgen-owned
(`panels/pubgen/**`, docs, tests, xtask).

### Rules

1. Every upstream hook sits behind `#[cfg(feature = "pubgen")]`; an upstream build compiles
   exactly the original code.
2. Keep hooks append-only and at stable points (top of a function, after a known statement).
   Do not reformat around them.
3. Put `// pubgen:` on each hook line so a merge reviewer can find them with one search.
4. Commit Pubgen work separately from upstream sync merges. Never mix a rebase/merge commit with
   feature edits.
5. After every upstream merge, run:
   - `cargo check -p designcraft-ui-egui`
   - `cargo check -p designcraft-ui-egui --features pubgen`
   - `cargo test -p designcraft-ui-egui --features pubgen -- pubgen`
   - `cargo xtask ci`
6. Add the feature build/test to `xtask` (replace the current clippy-only `pubgen` task or add
   a `pubgen-ci` task) so a missing hook cannot rot silently.
7. If upstream refactors `menus::bar`, `run_ui_inner`, or `Item`, the compile fails at the hook
   rather than silently dropping the menu. Re-apply the hook; no drift tool is needed because
   no upstream code is copied.

### Conflict-resolution checklist after `git merge upstream/main`

- [ ] `lib.rs`: keep upstream dock changes; re-add `pubgen_ui`; keep `pub mod dock;` plain.
- [ ] `menus/bar.rs`: keep upstream `menu_bar` body; re-insert the tree entry.
- [ ] `menus/run.rs`: keep upstream arms; re-add the dispatch at the top.
- [ ] `app.rs`: re-add the host call after `dialogs::show`.
- [ ] `native_menu.rs`: re-add the native insertion.
- [ ] Run the four commands above.
- [ ] Open the app, check the `Pubgen` menu, open one window, run one compile.
- [ ] Screenshot the bar and one window (visual evidence).

---

## 9. Verification and tests

### Unit tests (`crates/ui-egui/src/pubgen_ui/`)

- [ ] `pubgen_menu_is_inserted_after_file`.
- [ ] `every_pubgen_menu_entry_has_a_handler` (id exists in `COMMANDS`).
- [ ] `every_pubgen_command_has_a_menu_entry` (no dead command).
- [ ] `command_ids_are_unique`.
- [ ] `opening_each_window_needs_no_document` (smoke; no panic).
- [ ] `intent_queue_executes_and_clears`.
- [ ] `tick_polls_jobs_without_the_panel` (use a fake/stub job or a compile plan that fails
      fast; assert status changes once).
- [ ] `old_layout_with_pubgen_loads` (panel_docking regression).

### Harness / screenshot tests

- [ ] Bar screenshot with the Pubgen menu closed and open.
- [ ] One screenshot per window (Compile, Data, Cover, TOC, Utilities, AI).
- [ ] Optional opt-in: `CRAFT_UI_VISUAL_FIXTURES=dir cargo test ... pubgen_ui`.

### Manual checks

- [ ] Windows/Linux: menu opens, every entry works, menu scrolls at small heights.
- [ ] macOS: native menu shows Pubgen; actions open windows.
- [ ] No document: compiler works; other entries show a clear message.
- [ ] In-flight compile/AI job continues while the window is closed.
- [ ] State (manifest path, CSV mapping, cover options) survives window close/open and a
      document switch.
- [ ] `Window > Panels` no longer lists Pubgen; old saved workspace still loads.
- [ ] Control channel: `engine.commands` lists the new ids (phase 4).

### Gates

```
cargo xtask ci
cargo check -p designcraft-ui-egui --features pubgen
cargo test  -p designcraft-ui-egui --features pubgen -- pubgen
cargo run --release -p designcraft --features pubgen -- --sample --control 7979
```

---

## 10. Risks and mitigations

| Risk | Impact | Mitigation |
| :--- | :--- | :--- |
| Menu cannot host the long form | High | Every "..." entry opens the existing card in a window; no 1:1 menu-only UI |
| Jobs stop because the panel is no longer drawn | High | `host::show` + `panel::tick` every frame in `app.rs`; test it |
| State lost between panel and window | Medium | Same `egui::Id::unique(PANEL_ID)` and `PubgenState`; one host |
| Very tall cards (Excel, Cover) | Medium | Resizable window, vertical scroll, max height, remember size later |
| Menu action needs `PubgenState`/`ctx` | Medium | Intent queue consumed by the host; every intent tested |
| Upstream edits the exact hook lines | Low | 2-5 line hooks, `// pubgen:` markers, feature-gated, separate commits |
| Upstream menu tests fail under `--features pubgen` | Low | Do not change `menu_tree()`; only insert into the drawn bar. Classic tree stays canonical |
| macOS native menu is built once | Low | Static labels in phase 4; no dynamic checkmarks there |
| Old saved layout references `pubgen` | Low | Revert test assumptions; add a load test; panel docking filters unknown ids |
| Wasm build | Low | Compiler window/rows `#[cfg(not(wasm32))]`; same as today |
| Hidden-menu customizations by path | Low | New paths simply do not inherit old hide entries; document it |
| Panel tests assume `show` | Low | Keep `show` as a thin wrapper over `tick` + cards until phase 3; update fixtures then |

---

## 11. Rollback

- Phases are separate commits. To restore the panel, revert the phase-3 commit (re-add the
  three dock hooks and the `dock_extended` file) and remove the `pubgen_ui` hooks from
  `menus/bar.rs`, `menus/run.rs`, `app.rs` and `lib.rs`.
- The menu can be hidden independently by removing the `menus/bar.rs` hook; windows and
  commands remain callable from the control channel, which is useful for debugging.
- Keep `dock_extended.rs` in git history; do not delete the file in the same commit as the
  panel removal if you expect to roll back quickly.

---

## 12. Effort estimate

| Phase | Effort | Notes |
| :--- | :--- | :--- |
| 0 Audit | 0.5 day | mostly reading and mapping |
| 1 Menu skeleton | 0.5-1 day | new module + four hooks |
| 2 Windows | 2-4 days | mostly wrapping existing cards; large surfaces need scroll/layout work |
| 3 Panel removal + cleanup | 0.5 day | deletes code; test reverts |
| 4 Palette/control/macOS | 1 day | three small hooks + native menu |
| 5 Polish | 1 day | i18n, fixtures, docs |
| **Total** | **5-7 days** | one developer, no engine refactor |

---

## 13. Open decisions

1. **Menu position:** after `File` (recommended, as above) or at the end after `Help`.
2. **Keep a floating Pubgen window for power users?** Recommended no for this pass; the menu
   opens one card at a time. Can be added later as `Pubgen > Open Pubgen Panel`.
3. **UI commands now vs engine commands later:** use UI commands for this pass; move generation
   into the engine in a follow-up so headless/CLI/MCP can run it.
4. **Direct AI actions vs dialog-only:** menu rows for page-level AI actions queue intents and
   run immediately; if that proves surprising, change them to open the AI window with the
   target selected.
5. **i18n now or later:** ship English labels through `i18n::tr`; translate after the menu
   stabilizes.
6. **One window at a time vs multiple:** one window (recommended) matches the old panel's
   single-surface model and keeps state simple.
