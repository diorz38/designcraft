# Plan: PubGen Menu Bar as Default, with DesignCraft Menu Toggle

- **Date:** 2026-10-29
- **Branch:** `feature/pubgen-panel-ux-redesign` (continues [2026-10-28-pubgen-main-menu-option2.md](2026-10-28-pubgen-main-menu-option2.md))
- **Status:** Completed & Verified

---

## 1. Objective

Make Pubgen the first thing an editor sees: by default the menu bar is
`File / Publication / PubGen Tools / Help` — DesignCraft's other menus
(Edit, Layout, Type, Object, Table, View, Window) are hidden. A toggle
switches between this PubGen bar and DesignCraft's full bar (with the
`Pubgen` menu appended before `Help`), in both directions.

## 2. Merge Contract (upstream-updates minimization)

Everything new lives in **one file**: `crates/ui-egui/src/menus/pubgen_bar.rs`.
An upstream DesignCraft merge can only conflict on **three one-line hooks**:

| Hook | File | Content |
|---|---|---|
| Module declaration | `crates/ui-egui/src/menus.rs` | `#[cfg(feature = "pubgen")] pub mod pubgen_bar;` |
| Dispatch | `crates/ui-egui/src/menus/bar.rs` | Early return at the top of `menu_bar`: `if crate::menus::pubgen_bar::menu_bar(app, ui) { return; }` (cfg-gated) |
| Test pin | `crates/ui-egui/src/menus/tests.rs` | `oversized_edit_menu_scrolls_to_preferences` sets DesignCraft mode first (the test scrolls the **Edit** menu, hidden in PubGen mode) |

No upstream menu table, command table, or UI state field is edited. The
fork-owned `dock_extended.rs` `pubgen_menu` now delegates to
`pubgen_bar::pubgen_menu` (one body, kept so the `Pubgen` menu of
DesignCraft mode and the PubGen bar share one implementation).

## 3. Behaviour

- **Default (PubGen mode):** `File` and `Help` are DesignCraft's own rows,
  drawn from the upstream `menu_tree()`; `Publication` holds the generation
  pipeline (Master Excel Import & Generate…, Publication Compile Wizard…,
  Statistical Vector Chart…, Table of Contents…, Publication Cover…,
  Header & Footer Setup…) plus `About PubGen…` and the toggle;
  `PubGen Tools` holds Manual Ingest & Tables, Page & Layout Utilities,
  Dual Language Mode and AI Translate. RTL layouts mirror the four menus.
- **DesignCraft mode:** upstream `menu_bar` untouched; `Pubgen` menu sits
  before `Help`; its footer holds `About PubGen…` and `Show PubGen Menus`.
- **State:** egui temp data under `Id::unique("pubgen_menus_on")`, default
  `true`. Per session (resets on restart); no persisted state field.
- **Toggle also in** the About PubGen dialog footer
  (`panels/pubgen/widgets.rs`).
- **macOS native menu** (`apps/designcraft/src/native_menu.rs`) is not
  changed; the toggle affects the in-window bar (Windows/Linux/web).

## 4. Verification

- `cargo test -p designcraft-ui-egui --features pubgen --lib` — 1099 passed
  (3 new in `pubgen_bar.rs`: bar contents, toggle round trip, action render).
- `cargo test -p designcraft-ui-egui --lib` — 759 passed (no-feature build).
- Live run (`--sample --control 7979`): bar renders `File / Publication /
  PubGen Tools / Help`; `Show DesignCraft Menus` switches to
  `File / Edit / Layout / Type / Object / Table / View / Window / Pubgen /
  Help`; `Pubgen ▸ Show PubGen Menus` switches back.

## 5. Deferred

- macOS native menu toggle (follow-up if wanted).
- Control-channel/MCP access to the toggle (would need a `UI_COMMANDS`
  entry; the flag is plain egui memory today).
