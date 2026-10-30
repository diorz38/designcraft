# PubGen Menu Bar — UX & Information-Anatomy Mini-Plan

- **Document:** `docs/plans/2026-10-30-pubgen-menu-bar-ux-mini-plan.md`
- **Date:** 2026-10-30
- **Status:** Done — implemented on `pubgen-0.8.X`, one commit per phase: `ed83a718` (0), `d1ce1c37` (1), `bf76c2fa` (2), `2bb51025` (3), `df5beb08` (4), `79c02d52` (5), `4b3d78c9` (6). Deviations from this draft: the ToC chord is **Ctrl+Alt+T** (F7 belongs to the Layers panel), `tableOfContents` moved to a UI command, the Window menu gains a flat **PubGen Panel** row (the fork's only dock panel; no "PubGen Panels" submenu), and the disabled hint reads "Open a document first". Wasm checked after Phase 6. Pre-existing local failures outside this plan's scope: `chrome::tests` (×4, window-geometry), `dock::pubgen::ai` glossary tests (×5, bundled-glossary data). Revised 2026-10-30: drop the "…" suffix in Publication, relabel Vector Chart, add the **Shortcut** menu.
- **Branch:** `feature/pubgen-menu-ux` (proposed, off `standalone` / `pubgen-0.8.X`)
- **Target:** `crates/ui-egui/src/menus/pubgen_bar.rs`, `crates/ui-egui/src/menus/` (bar, ui_commands, keys), `crates/ui-egui/src/panels/pubgen/state.rs`, `crates/ui-egui/src/dock_extended.rs`, `docs/`

---

## 1. Goal

The PubGen menu bar is functionally complete (the audit found **no accessibility gap**) but weak as
professional-app navigation: it hides DesignCraft's six host menus, has duplicate routes and a
duplicated About, carries zero keyboard shortcuts, mixes jargon labels, and uses hand-drawn toggle
state. This plan lands the audit's P0 fixes (P0-1 … P0-5) and its consistency P1s (P1-1 … P1-6) in
seven phases, each independently shippable, without touching upstream menu machinery beyond the
single existing hook in [`super::bar::menu_bar`].

Success = the audit's §2.7 metrics: ≥3 primary actions reachable in one hop, ≥6 registered
shortcuts, 0 hidden host menus, 0 duplicate routes, consistent toggle state, ≥80% of items with a
visual cue.

## 2. Current State (verified 2026-10-30 on `pubgen-0.8.X` @ `041f00f9`)

| Component | State | Notes |
| :--- | :--- | :--- |
| `menus/pubgen_bar.rs:28-55` | `menu_bar(app, ui) -> bool` | Returns `false` → DesignCraft's full bar draws instead. Slots `[0,1,2,3]` = File · Publication · PubGen Tools · Help (`:37`); **host menus Edit/Layout/Type/Object/View/Window are never drawn** in PubGen mode |
| `pubgen_bar.rs:87-142` | `pipeline()` | 7 items, all suffixed "…" (the suffix is **removed** in PubGen mode per this plan); "Master Excel Import & Generate…" (`:90`) and "Publication Compile Wizard…" (`:98`) both set `dialog_excel_open = true` — **the audit's duplicate route**; "Statistical Vector Chart…" (`:105`) becomes "Vector Chart" |
| `pubgen_bar.rs:145-192` | `content()` | Manual Ingest ▸, Page & Layout Utilities ▸, manual `"✓ "` prefix toggle for Dual Language (`:181-189`), AI Translate ▸ |
| `pubgen_bar.rs:166-169` | "Set Standard BPS Margins (20/20/19/15 mm)" | Direct `execute_set_page_margins`, no undo-group guarantee surfaced, label carries the numbers |
| `pubgen_bar.rs:250-263` | `about()` | About PubGen… + menu-bar toggle; called from **both** `pubgen_menu()` (`:275`) and `publication_menu()` (`:288`) — **the duplicated About** |
| `pubgen_bar.rs:77-79` | `item()` | `egui::Button` + `add_enabled`; no tooltip, no icon, no shortcut text |
| `pubgen_bar.rs:302-404` | tests | `pubgen_bar_shows_file_publication_content_and_help_only` **pins host menus as hidden** (`:363-365`) — must flip in Phase 1; `frame()` harness renders the real `crate::menus::menu_bar` |
| `menus.rs:38-64` | `Item::Cmd {label, id, params, shortcut}` | UI-only commands come from `ui_commands::UI_COMMANDS` (`:46`); `ui_label(id)` supplies label **and shortcut**; this is the upstream mechanism P0-4 plugs into |
| `menus/keys.rs:33` | `shortcuts(app, ctx)` | Consumes the command table + `UI_COMMANDS` and registers the keymap — the dispatch path a new PubGen UI command rides for free |
| `menus/state.rs:51` | `shortcut_of(app, id)` / `shortcut_text` (`menus.rs:38`) | Renders the chord next to a menu label — already wired for `Item::Cmd` |
| `dock_extended.rs:13` | `DOCK_TABS: &[(&str, &str, &str)]` (+ `ICON_PANELS`, `FLOATING_PANELS`) | The panel list a View/Window menu mirrors (P1-1) |
| `panels/pubgen/state.rs` | `dialog_*` flags, `bilingual_output`, `poll_active_jobs` | The state-flag pattern menus use; `poll_active_jobs` is the basis for the P1-6 status dot |
| `pubgen_bar.rs` design intent (`:8-9`) | "everything in one file so upstream touches one hook" | Host-menu restoration (P0-1) must respect this: draw **more** of `crate::menus::menu_tree()`, add no upstream tables |
| CI | `cargo fmt` + pubgen lint gate `clippy -p designcraft-ui-egui --features pubgen --all-targets -- -D warnings` + pinned menu tests | Same gates as Pillar 3 |

## 3. Principles

1. **One file, one hook.** All bar changes stay in `pubgen_bar.rs` (+ the `UI_COMMANDS` table and
   `state.rs` flags where a mechanism already exists). Upstream `menus/` grows only *data* (new
   `UI_COMMANDS` rows), never new control flow.
2. **Host menus are drawn, not reimplemented.** P0-1 reuses `crate::menus::menu_tree()` rows through
   the existing `designcraft_menu()` helper — zero duplication of upstream item logic, shortcuts and
   i18n come along free.
3. **Menu items may execute directly.** The audit's flag-only pattern stays for dialogs that genuinely
   live in the panel, but a menu item that can complete its action inline (margins, translate) does so.
4. **Every user-visible label change is a test change.** The pin tests in `pubgen_bar.rs` assert exact
   top-level sets and orderings; each phase that renames or adds menus updates them in the same commit.
5. **Shortcuts are registered, not hand-wired.** No new `ctx.input` scraping: a chord exists only if it
   is in `UI_COMMANDS`/the keymap, so the Shortcuts dialog, Quick Apply and menu suffixes all agree.
6. **One phase = one commit**, `cargo fmt -p designcraft-ui-egui`, the pubgen clippy gate, and the
   menu/panel tests green per commit. Prefix: `feat(pubgen-menu):` (matches `fix(pubgen):` house style).
7. **Never crash / no panics** — UI-only change, but new state reads go through `with_state` and
   `is_some_and`/`unwrap_or` exactly like the existing code.

## 4. Architecture Decisions

### 4.1 P0-1 — Restore host menus (the big one)

PubGen mode draws the **full host tree** plus the three PubGen menus:

```text
File · Edit · Layout · Type · Object · View · Window · Publication · Publication Tools · Shortcut · Help
```

- Implementation: `menu_bar()` iterates `crate::menus::menu_tree()` in upstream order and draws every
  menu through the existing `designcraft_menu()` helper, inserting Publication + Publication Tools +
  Shortcut between Window and Help (slot math from `:37` generalises to an ordered `Vec` build,
  RTL-mirrored).
- The audit's alternative (a single mega-menu on the right) is rejected: it keeps §1 violated and
  InDesign/Photoshop users expect the host anatomy.
- The "Show DesignCraft Menus" toggle becomes nearly moot (the bars now differ only by the three
  Pubgen menus); it stays as the escape hatch, moving to Help (see 4.3).
- Test flip: `pubgen_bar_shows_file_publication_content_and_help_only` becomes
  `pubgen_bar_keeps_the_host_menus_and_adds_the_pubgen_ones` — asserts all eleven top-levels present
  in order, `toggling_brings_designcrafts_menus_back…` keeps asserting the toggle (now: Pubgen menus
  appear/disappear, host menus never do).

### 4.8 NEW — the "Shortcut" menu (host quick access, after Publication Tools)

A flat, curated top-level menu holding **quick access to original DesignCraft functions** — the host
commands a PubGen operator reaches for most, without digging through Edit/Layout/Type/Object. It is
*data*, not new logic: each row is an existing engine/UI command id rendered through the same
`Item::Cmd` machinery as Phase 3 (so chords, gates and i18n come free).

Initial contents — as shipped (`pubgen_bar.rs::shortcut_menu`), all `UI_COMMANDS`/engine ids:

| Item | Command | Why |
| :--- | :--- | :--- |
| Export PDF… | `app.exportPdf` | The terminal action of every publication |
| Preflight Document | `preflight.run` | QC before export; complements Audit Publication |
| Package… | `app.packageDialog` | Hand-off to print |
| Place… | `app.placeDialog` | One-off images outside the Excel pipeline |
| Sample Document | `file.newSample` | Training / demo layouts without Master Excel |

Rules: every row must exist in the command table or `UI_COMMANDS` (no new commands — verify with
`designcraft-cli commands`), disabled rows keep their `enabled` gate and the Phase-4 disabled
tooltip. The menu is PubGen-mode only (in DesignCraft-bar mode these are already one menu away).

### 4.2 P0-2 — One action, one route

- "Publication Compile Wizard…" **goes away**; "Master Excel Import & Generate…" is relabelled
  **"Generate from Excel…"** (front-loaded verb, audit P0-5). The compile card is already
  always-visible in the panel — no second entry point needed. If the user wants a direct route later,
  it becomes a *focus-and-scroll* of the existing card, not a second flag.
- Single source of truth: one `UI_COMMANDS` entry `pubgen.generateFromExcel` replaces the two
  flag-setting menu rows (see 4.4), so the menu item, its chord and Quick Apply share one id.

### 4.3 P0-3 — About and the menu-bar toggle live in Help

`about()` is called only from the Help slot. Publication loses the footer; `pubgen_menu()`
(DesignCraft-bar mode) keeps it — that mode's whole point is one Pubgen menu, and the audit only
objects to the **duplicate**. Concretely: `publication_menu()` drops its `about()` call; Help gets
"About PubGen…" + "Show DesignCraft Menus ▸" beneath the upstream Help rows (drawn via the same
popup pattern, appended after `menu_column`).

### 4.4 P0-4 — Shortcuts through the existing machinery

Register PubGen UI commands in `UI_COMMANDS` (`menus/ui_commands.rs`) — label, gate, params-free
`run` that sets the state flag — instead of ad-hoc rows in `pubgen_bar.rs`:

| Command id | Shortcut | Opens / runs |
| :--- | :--- | :--- |
| `pubgen.generateFromExcel` | `Ctrl+Shift+G` | `dialog_excel_open = true` |
| `pubgen.compile` | `Ctrl+Shift+B` | compile card focus (panel) |
| `pubgen.tableOfContents` | `F7` | `dialog_toc_open = true` |
| `pubgen.audit` | `Ctrl+Shift+Alt+A` | `audit_open = true` (Pillar 3 command already exists for the engine path) |
| `pubgen.bpsMargins` | `Ctrl+Shift+M` | `execute_set_page_margins` |
| `pubgen.aiTranslatePage` | `Ctrl+Shift+T` | `ai::execute_translate_selected_page` |
| `pubgen.dualLanguage` | `Ctrl+Shift+L` | toggle `bilingual_output` |

`Item::Cmd` rendering then gives every row its label, gate (`enabled`), chord suffix
(`shortcut_text`) and dispatch from one table; `pubgen_bar.rs` builds its Publication / Publication
Tools popups from these ids. Chords avoid the upstream keymap's existing bindings — Phase 3 opens
with a collision check against `keys.rs` and the Shortcuts dialog (`shortcuts_dialog.rs`) shows them
editable like every other chord.

### 4.5 Labels

- "PubGen Tools" → **"Publication Tools"** (de-jargonised; the top-level "Publication" stays the
  pipeline, its sibling holds the loose operations — audit §2.6 anatomy).
- "Set Standard BPS Margins (20/20/19/15 mm)" → **"Standard BPS Margins"**; the numbers move into the
  row's tooltip (`on_hover_text`) and into the command's params doc.
- "Statistical Vector Chart…" → **"Vector Chart"** (short, scannable; "vector" is the product term).
- **No "…" suffix anywhere in Publication.** The ellipsis is the desktop convention for "opens a
  dialog"; in PubGen mode every pipeline row does that, so the suffix carries no information and the
  rows read cleaner without it (`Generate from Excel`, `Table of Contents`, `Publication Cover`,
  `Header & Footer Setup`, `Vector Chart`, `Audit Publication`). Submenu rows keep their `▸` caret —
  that cue stays. Applies to Publication only; Publication Tools submenu labels are untouched.
- i18n: every changed string goes through `crate::i18n::tr` as today; the Indonesian catalogue gets
  the new keys in the same commit.

### 4.6 P1 cluster

- **P1-1 View menu**: a host-tree **View** row is already restored by 4.1 for host entries; PubGen
  panel toggles (Pubgen panel, open dialogs) go under **Window ▸ PubGen Panels ▸**, mirroring
  `DOCK_TABS` + `FLOATING_PANELS` via `show_panel`/dock APIs — one loop, no hand-listed panels.
- **P1-2 undo hint**: `execute_set_page_margins` already runs engine commands (undoable); the menu row
  gets tooltip "Undo with Ctrl+Z"; the command lands in exactly one undo group (verify; wrap if not).
- **P1-3 toggles**: the `"✓ "` prefix becomes `Button::selected(is_bilingual)` (or a small check
  glyph painted by `item()` when a `checked: bool` parameter is set) — same helper for Dual Language
  and the menu-bar toggle, so checkmark rendering has one code path.
- **P1-4 cues**: with the Publication "…" suffix dropped (§4.5), a row's cues are: shortcut text
  (Phase 3), submenu caret (`menu_button`'s own), checkmark for toggles (P1-3), and the running dot
  (P1-6). No icon font dependency.
- **P1-5 disabled tooltips**: `item()` wraps `add_enabled(false, …).on_disabled_hover_text("Open a
  document first")` (or a per-item reason string parameter). One helper change, all rows benefit.
- **P1-6 status dot**: Publication rows for compile/AI/cover show a running indicator when
  `poll_active_jobs` reports a matching job (`"● Generate…"`); read-only state check at popup paint
  time.

### 4.7 P2 (explicitly out of scope here, listed for the roadmap)

Ribbon/toolbar for primary actions (P2-1), contextual AI submenu gating polish (P2-2), Recent
projects (P2-3), i18n catalogue completeness sweep of host strings (P2-4). Each is its own mini-plan
if promoted.

## 5. Phases

### Phase 0 — Baseline & duplication removal (P0-2, P0-3)
1. Add the missing pin tests first: Publication menu lists exactly the pipeline items (parse the
   drawn texts), About appears in exactly one PubGen-bar menu.
2. Remove the "Publication Compile Wizard…" row; drop `about()` from `publication_menu()`.
3. Tests updated; no behavioural risk. **Commit:** `feat(pubgen-menu): one route per action, About only in Help`.

### Phase 1 — Host menus + the Shortcut menu (P0-1, §4.8)
1. Generalise `slots` into an ordered build of `menu_tree()` + Publication + Publication Tools +
   Shortcut + Help.
2. Add the **Shortcut** menu: the §4.8 table rendered as `Item::Cmd` rows (or plain `item()` +
   `app.run` where a UI command does not exist yet — the rows move onto `UI_COMMANDS` in Phase 3).
3. Flip/extend the pin tests (§4.1) — eleven top-levels in order; Shortcut menu lists its rows.
4. Verify RTL mirroring with the existing `rtl` slot reversal.
**Commit:** `feat(pubgen-menu): keep DesignCraft's menus, add Publication, Publication Tools and Shortcut`.

### Phase 2 — Labels & relabel-safe i18n (P0-5)
"Generate from Excel…", "Standard BPS Margins" (+tooltip), "Publication Tools", **"Vector Chart"**,
and **strip the "…" suffix from every Publication row** (§4.5). Catalogue keys for `en`/`id`. Pin
tests updated with the new strings; the audit-guide docs (`docs/pubgen-guide.md` + `_id`) name the
new labels.
**Commit:** `feat(pubgen-menu): front-loaded labels, no ellipsis in Publication`.

### Phase 3 — Shortcuts via `UI_COMMANDS` (P0-4)
1. Collision check; add the §4.4 rows (id, label, chord, enabled-gate, run).
2. Rebuild `pipeline()`/`content()` rows from the table (state-flag `run`s move into `ui_commands.rs`).
3. Test: chord strings render next to the rows; one kittest press of `Ctrl+Shift+G` opens the Excel
   dialog (existing kittest click patterns apply).
**Commit:** `feat(pubgen-menu): register primary actions as UI commands with shortcuts`.

### Phase 4 — Toggle/cue/tooltip consistency (P1-3, P1-4, P1-5, P1-2)
`item()` gains `checked: Option<bool>` + `disabled_reason: &str`; submenu caret stays with
`menu_button`; margins tooltip. Tests: selected state draws a check for both toggles; disabled rows
carry hover text.
**Commit:** `feat(pubgen-menu): consistent toggles, cues and disabled hints`.

### Phase 5 — Window ▸ PubGen Panels (P1-1)
Loop `DOCK_TABS`/`FLOATING_PANELS` into a submenu that shows/toggles each panel (upstream dock API),
checkmark = currently visible. Test: submenu lists the Pubgen panel and toggles visibility state.
**Commit:** `feat(pubgen-menu): restore closed Pubgen panels from the Window menu`.

### Phase 6 — Live job indicator (P1-6)
"● …" prefix on Publication rows whose job is running (`poll_active_jobs`). Test: with a fake active
job flag, the row text changes.
**Commit:** `feat(pubgen-menu): running-job indicator in the Publication menu`.

Each phase ends with: `cargo fmt -p designcraft-ui-egui` · `cargo clippy -p designcraft-ui-egui
--features pubgen --all-targets -- -D warnings` · `cargo test -p designcraft-ui-egui --features
pubgen --lib menus::pubgen_bar`. Wasm build checked once after Phase 1 and again at the end
(`cargo xtask wasm`) — the bar is UI-only but Phase 1 changes what `menu_bar` draws on both targets.

## 6. Risks & Mitigations

| Risk | Mitigation |
| :--- | :--- |
| Host menus in PubGen mode double-translate or mismatch the DesignCraft-bar rendering | Reuse `designcraft_menu()` (same code path the toggle mode uses); pixel-diff one screenshot in the kittest run, no new rendering |
| Ten top-level menus crowd narrow windows / wasm mobile widths | The bar already wraps via egui `MenuBar`; add a shrink test at 1024 px in Phase 1's kittest pass |
| Shortcut collisions with upstream keymap or OS (Ctrl+Shift+B etc.) | Phase 3 opens with a collision sweep of `keys.rs` defaults; chords are user-editable via the existing Shortcuts dialog |
| Users depend on "Compile Wizard" wording (training material) | Phase 2 keeps the relabel atomic with a docs note in `docs/pubgen-guide.md` + `_id` |
| `UI_COMMANDS` grows unbounded / ids drift from panel flags | The `run` bodies are the same `with_state` one-liners moved verbatim; ids are `pubgen.*`-namespaced |

## 7. Acceptance

Audit §2.7, re-measured after Phase 6:

| Metric | Target |
| :--- | :--- |
| Primary actions 1 hop from the menu bar (Generate, Compile, Export) | ≥3 |
| Registered shortcuts | ≥6 |
| Host menus hidden in PubGen mode | 0 |
| Duplicate routes | 0 |
| Consistent toggle checkmarks | 2/2 (Dual Language, menu-bar toggle) |
| Shortcut menu present with host quick-access rows | ≥5 rows, 0 new commands |
| Publication rows without "…" | all |
| Items with a cue (shortcut text, caret, ellipsis, check) | ≥80% |

All pin tests green; pubgen lint gate green; `cargo xtask wasm` green.
