# PRD: Pubgen Dock Extension & Upstream Synchronization

- **Document Version:** 1.1.0
- **Status:** Approved / In Planning
- **Target App:** DesignCraft (`/`, `crates/ui-egui`)
- **Author / Context:** PubCraft-Rust Project

---

## 1. Executive Summary & Objectives

### 1.1 Problem Statement
The upstream `designcraft` repository is under rapid, heavy development. Introducing custom publishing tools (specifically, automated CSV document generators) directly into core UI files causes severe git merge friction when pulling upstream updates.

### 1.2 Proposed Solution: Parallel Extension Architecture
1. **Pristine Upstream Tracking:** Keep `crates/ui-egui/src/dock.rs` 100% untouched and identical to upstream `origin/main`.
2. **Parallel Extended Module:** Create `crates/ui-egui/src/dock_extended.rs`, which mirrors `dock.rs` but injects the `Pubgen` tab into the dock tab strip (`Properties`, `Pages`, `Layers`, `Pubgen`) and delegates rendering to `panels::pubgen`.
3. **Conditional Compilation Seam:** Switch between `dock.rs` and `dock_extended.rs` using a feature flag (`feature = "pubgen"`).
4. **Diff & Re-sync Automation:** Maintain an automated diff / check workflow (`cargo xtask diff-dock`) so whenever upstream updates `dock.rs`, developers can immediately visualize the delta, reconcile `dock_extended.rs`, and rebuild with zero guesswork.
5. **Pubgen UI Parity with Properties Panel:** Match the exact InDesign-style layout language of the **Properties** panel (selection band, section headers, 1.5pt rules, 21pt fields, 20pt outline buttons).
6. **Card-Based Functionality:**
   - **Card 1: Page from CSV** — Generates multi-page documents from CSV rows (columns: `type`, `number`, `title`) with predefined text template layouts.
   - **Card 2: Table from CSV** — Converts CSV data into formatted, native DesignCraft tables with headers and styling.

---

## 2. Architecture & File Seam

```
/
├── Cargo.toml                              ← Workspace definition
├── crates/
│   └── ui-egui/
│       ├── Cargo.toml                      ← Defines `[features] pubgen = []`
│       └── src/
│           ├── lib.rs                      ← Seam: selects `dock` or `dock_extended`
│           ├── dock.rs                     ← 100% UNTOUCHED upstream baseline
│           ├── dock_extended.rs            ← Extended dock with Pubgen tab hook
│           └── panels/
│               ├── mod.rs                  ← `pub mod pubgen;`
│               └── pubgen.rs               ← Pubgen UI panel & layout generator
└── xtask/
    └── src/
        └── diff_dock.rs                    ← Tooling to diff `dock.rs` vs `dock_extended.rs`
```

### 2.1 The Compilation Seam in `lib.rs`
In `crates/ui-egui/src/lib.rs`, the module declaration uses conditional compilation:

```rust
#[cfg(not(feature = "pubgen"))]
mod dock;

#[cfg(feature = "pubgen")]
#[path = "dock_extended.rs"]
mod dock;
```

* When building vanilla DesignCraft: `dock.rs` is compiled.
* When building with `--features pubgen` (or if `default = ["pubgen"]` is set locally): `dock_extended.rs` is compiled.
* Upstream git commits never touch `dock_extended.rs`, completely eliminating git conflict markers on upstream files.

---

## 3. The `dock_extended.rs` Delta Specification

`dock_extended.rs` is an exact clone of `dock.rs` with **only two discrete insertion points**:

### Delta Point 1: Dock Tab Array
```rust
// Upstream dock.rs:
pub const DOCK_TABS: &[(&str, &str, &str)] = &[
    ("properties", "Properties", "panel-properties"),
    ("pages", "Pages", "panel-pages"),
    ("layers", "Layers", "panel-layers"),
];

// dock_extended.rs:
pub const DOCK_TABS: &[(&str, &str, &str)] = &[
    ("properties", "Properties", "panel-properties"),
    ("pages", "Pages", "panel-pages"),
    ("layers", "Layers", "panel-layers"),
    ("pubgen", "Pubgen", "panel-pages"), // <--- PUBGEN HOOK
];
```

### Delta Point 2: Tab Content Match Dispatcher
```rust
// In dock_extended.rs inside `show()`:
egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match app.ui.dock_tab.as_str() {
    "pages" => panels::pages::show(app, ui),
    "layers" => panels::layers::show(app, ui),
    "pubgen" => panels::pubgen::show(app, ui), // <--- PUBGEN PANEL DISPATCH
    _ => panels::properties::show(app, ui),
});
```

---

## 4. Upstream Synchronization Workflow

### 4.1 Git Pull Procedure
When updating from the upstream remote:

```bash
# 1. Fetch and merge upstream changes
git checkout main
git pull upstream main
```

Because `dock.rs` was never modified, **`dock.rs` merges cleanly without conflicts**.

### 4.2 Automated Diff & Health Verification (`cargo xtask diff-dock`)
We add a dedicated xtask command:
```bash
cargo xtask diff-dock
```

This runs a 3-way semantic diff:
1. Strips the Pubgen hook lines from `dock_extended.rs`.
2. Compares the result against the newly pulled `dock.rs`.
3. If upstream modified `dock.rs` (e.g., token styles, layout alterations, dock behavior):
   - It outputs the exact lines changed by upstream.
   - It automatically patches or reports lines in `dock_extended.rs` needing updates.
4. Validates compilation with `cargo check -p designcraft-ui-egui --features pubgen`.

---

## 5. UI Architecture: 1-to-1 Parity with Properties Panel

The **Pubgen** panel (`crates/ui-egui/src/panels/pubgen.rs`) adopts the exact visual aesthetics, spacing, and card layout conventions of `panels/properties.rs`:

```
+-------------------------------------------------------+
|  PUBGEN                                         [•••] |  ← selection_band(ui, "Pubgen")
+-------------------------------------------------------+
|  Page from CSV                                        |  ← section(ui, "Page from CSV", true)
|  Source:  [ browse… ]  articles.csv (24 rows)        |
|  Columns: [type: "Chapter"] [number: "01"] [title]    |
|  Layout:  [ 6×9 Editorial Chapter                 v ] |
|  Records: (•) All   ( ) Range [ 1 - 10 ]              |
|  Target:  [x] Append to document                      |
|           [ Generate Pages ]                          |
+-------------------------------------------------------+  ← divider(ui) / rule(ui)
|  Table from CSV                                       |  ← section(ui, "Table from CSV", true)
|  Source:  [ browse… ]  pricing.csv (12 rows, 4 cols)  |
|  Headers: [x] First row is header                     |
|  Style:   [ Corporate Navy Accent                 v ] |
|  Width:   [ Fit to Margin Width                   v ] |
|           [ Insert Table ]                            |
+-------------------------------------------------------+
```

### 5.1 Design Tokens & Shared Widgets Used
To maintain complete design parity with `Properties`:
- **Header Band:** `selection_band(ui, "Pubgen")` with 33pt height and bottom section rule.
- **Section Headers:** `section(ui, title, true)` with 11.5pt semibold typography.
- **Dividers:** `rule(ui)` (1.5pt rule) and `divider(ui)` between cards.
- **Form Controls:**
  - 21pt input fields (`FIELD_H = 21.0`) with `#454545` fill and `#747474` border.
  - 20pt outline buttons (`outline_button(...)`).
  - Dropdown pickers with chevron segments (`dropdown(...)`).
  - Item spacing: `vec2(6.0, 5.0)`.

---

## 6. Card Specifications

### 6.1 Card 1: "Page from CSV"
* **Input Schema:**
  | Column | Role | Typography / Frame Preset |
  |---|---|---|
  | `type` | Category / Kicker | 11pt bold uppercase, 1.5 tracking, muted tint |
  | `number` | Numeral / Badge | 44pt headline font, primary text color |
  | `title` | Chapter Title | 24pt semibold font, margin-width text frame |

* **Layout Presets:**
  1. **Editorial Book / Chapter:** Top margin kicker, prominent numeral, flowing title frame.
  2. **Technical Card / Spec Sheet:** Accent banner, top-right number badge, structured title block.
  3. **Minimal Zine / Clean:** Centered number and title with generous whitespace.

* **Execution Pipeline:**
  1. Loops over selected CSV records.
  2. Inserts a new page per row (`layout.pages.insert`).
  3. Computes frame coordinates relative to page margin rect.
  4. Creates text frames (`frame.create`) and inserts styled text into stories (`text.insert`).
  5. Wrapped in an undo transaction: single `Ctrl+Z` reverses the entire batch.

---

### 6.2 Card 2: "Table from CSV"
* **Purpose:** Converts tabular CSV data into a native DesignCraft table object within the current document.
* **Controls:**
  - CSV file picker & live matrix dimensions (`N rows × M columns`).
  - "First row is header" toggle (determines header row formatting).
  - Table Style Preset dropdown:
    - `Basic Grid` (clean borders, light gray header).
    - `Corporate Navy` (dark navy header with white text, alternating zebra stripes).
    - `Minimalist` (horizontal rules only, no vertical borders).
  - Table Placement:
    - Insert at text cursor (if caret is active in a text frame).
    - Create new text frame on active page centered in margins.
* **Execution Pipeline:**
  1. Parses CSV rows into cell data grid.
  2. Executes `style.table.create` / `table.insert` via `app.session`.
  3. Populates rows, columns, and cell text with cell formatting.

---

## 7. Implementation Roadmap & Milestones

| Milestone | Deliverable | Acceptance Criteria |
|---|---|---|
| **M1: Seam & Baseline** | `dock_extended.rs`, feature flag in `Cargo.toml` & `lib.rs`, `diff-dock` xtask | `cargo check --features pubgen` passes; clean dock tab strip rendered |
| **M2: UI Shell with Cards** | `crates/ui-egui/src/panels/pubgen.rs` | Visual parity with Properties panel: Selection band, Card 1, Card 2 |
| **M3: Card 1 Engine** | CSV parser + 3 predefined geometry templates | Generates pages from `type, number, title` CSV with single-step undo |
| **M4: Card 2 Engine** | CSV-to-Table parser & native table creation | Creates styled DesignCraft table from CSV data on active page |
| **M5: Upstream Sync Drill** | Test upstream diff & sync validation | Simulated changes to `dock.rs` are successfully detected by `xtask diff-dock` |
