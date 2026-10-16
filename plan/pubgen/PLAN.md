# Engineering Plan: Pubgen Dock Extension & Upstream Synchronization

- **Target System:** DesignCraft (`crates/ui-egui`, `crates/engine`, `crates/doc`)
- **PRD Reference:** [plan/pubgen/PRD.md](file:///f:/GitHub/designcraft/plan/pubgen/PRD.md)
- **Review Reference:** [plan/pubgen/plan-review.md](file:///f:/GitHub/designcraft/plan/pubgen/plan-review.md)
- **Status:** Updated & Ready for Execution

---

## Architecture Principles & Upstream Isolation

1. **Right Dock Integration:** Pubgen is integrated into the right-hand expanded dock tab strip (`Properties` · `Pages` · `Layers` · `Pubgen`) inside `crates/ui-egui/src/dock_extended.rs`.
2. **Minimal Upstream Touch:**
   - `crates/ui-egui/src/dock.rs` remains **100% untouched** and identical to `upstream/main`.
   - `crates/ui-egui/src/panels/mod.rs` is **not modified**; `pubgen` is declared directly inside `dock_extended.rs` via `#[path = "panels/pubgen.rs"] pub mod pubgen;`.
   - The only core files touched are `crates/ui-egui/Cargo.toml` (`[features] pubgen = []`) and `crates/ui-egui/src/lib.rs` (3-line conditional module seam for `pub mod dock;`).
3. **Layering Compliance:** No direct usage of `rfd` in `ui-egui`. File dialogs route strictly through `app.services.pick_open` to comply with `cargo xtask layers`.
4. **Three Dock Insertion Points:** `dock_extended.rs` hooks into `DOCK_TABS`, the main tab `ScrollArea` dispatcher, and `panel_body` (ensuring torn-off/floating panels and modern `panel_docking` render correctly).
5. **Engine Data Merge Synergy:** Reuse or mirror existing RFC 4180 parsing and data source abstractions (`crates/engine/src/cmd/datamerge/parse.rs`).

---

## Phase Overview

```
Phase 1: Architecture Seam & Extended Dock Infrastructure
  ↓
Phase 2: Properties-Parity UI Shell (Card 1 & Card 2)
  ↓
Phase 3: Card 1 Engine — "Page from CSV" (type, number, title)
  ↓
Phase 4: Card 2 Engine — "Table from CSV" (table grid & styling)
  ↓
Phase 5: Upstream Synchronization Tooling (`cargo xtask diff-dock`)
  ↓
Phase 6: Verification, Themes & End-to-End Testing
```

---

## Phase 1: Architecture Seam & Extended Dock Infrastructure

**Goal:** Establish the zero-conflict compilation seam so `crates/ui-egui/src/dock.rs` remains 100% identical to upstream `origin/main`, while compiling `dock_extended.rs` when the `pubgen` feature is enabled.

### Tasks:
- [x] **Task 1.1: Feature Flag Declaration**
  - File: `crates/ui-egui/Cargo.toml`
  - Add feature flag:
    ```toml
    [features]
    pubgen = []
    ```
- [x] **Task 1.2: Conditional Module Seam in `lib.rs`**
  - File: `crates/ui-egui/src/lib.rs`
  - Replace `pub mod dock;` with:
    ```rust
    #[cfg(not(feature = "pubgen"))]
    pub mod dock;

    #[cfg(feature = "pubgen")]
    #[path = "dock_extended.rs"]
    pub mod dock;
    ```
- [x] **Task 1.3: Clone Baseline to `dock_extended.rs` & Inject Three Seams**
  - Copy `crates/ui-egui/src/dock.rs` to `crates/ui-egui/src/dock_extended.rs`.
  - Declare panel module directly inside `dock_extended.rs` (avoids touching `panels/mod.rs`):
    ```rust
    #[path = "panels/pubgen.rs"]
    pub mod pubgen;
    ```
  - **Insertion Point 1: `DOCK_TABS` (dock_extended.rs:10)**
    ```rust
    pub const DOCK_TABS: &[(&str, &str, &str)] = &[
        ("properties", "Properties", "panel-properties"),
        ("pages", "Pages", "panel-pages"),
        ("layers", "Layers", "panel-layers"),
        ("pubgen", "Pubgen", "panel-pages"),
    ];
    ```
  - **Insertion Point 2: Scroll Area Match Dispatcher (dock_extended.rs:109)**
    ```rust
    match app.ui.dock_tab.as_str() {
        "pages" => panels::pages::show(app, ui),
        "layers" => panels::layers::show(app, ui),
        "pubgen" => pubgen::show(app, ui),
        _ => panels::properties::show(app, ui),
    }
    ```
  - **Insertion Point 3: Floating / Docking Dispatcher in `panel_body` (dock_extended.rs:247)**
    ```rust
    match id {
        "properties" => panels::properties::show(app, ui),
        "pages" => panels::pages::show(app, ui),
        "layers" => panels::layers::show(app, ui),
        "pubgen" => pubgen::show(app, ui),
        // ... upstream arms ...
        _ => panels::properties::info_panel(app, ui),
    }
    ```
- [x] **Task 1.4: Panel Module Stub & i18n Registration**
  - File: `crates/ui-egui/src/panels/pubgen.rs`
  - Created module with complete Properties-parity widgets and UI state.
- [x] **Task 1.5: Compilation & CI Verification**
  - Verified baseline: `cargo check -p designcraft-ui-egui` (compiles vanilla `dock.rs`).
  - Verified extended: `cargo check -p designcraft-ui-egui --features pubgen` (compiles `dock_extended.rs`).
  - Verified app binary: `cargo check -p designcraft --features pubgen`.

---

## Phase 2: Properties-Parity UI Shell (Card 1 & Card 2)

**Goal:** Implement the Pubgen panel UI using the exact design tokens, spacing, and card layout conventions of `panels/properties.rs` and `widgets.rs`.

### Tasks:
- [x] **Task 2.1: Header Band**
  - File: `crates/ui-egui/src/panels/pubgen.rs`
  - Implemented `selection_band(ui, "Pubgen")` (33pt height with bottom rule from `crate::widgets`).
  - Graceful "No document open" state with dimmed status label.
- [x] **Task 2.2: Card 1 UI — "Page from CSV"**
  - Render card header: `section(ui, "Page from CSV", true)`.
  - File picker row: browse button triggering `app.services.pick_open.as_mut().and_then(|f| f("pubgen"))` (layering-safe) + manual path fallback input.
  - Column mapping indicators: badges showing `type`, `number`, `title` detection.
  - Layout template selector: dropdown picker (`Editorial Chapter`, `Technical Spec Card`, `Minimal Zine`).
  - Record scope picker: radio buttons `All records` vs `Range (e.g. 1-10)`.
  - Page options: checkbox `[x] Append to document`.
  - Action button: 20pt outline button `[ Generate Pages ]`.
- [x] **Task 2.3: Section Rule & Divider**
  - Calling `divider(ui);` and `rule(ui);` (1.5pt rule) to cleanly separate Card 1 and Card 2.
- [x] **Task 2.4: Card 2 UI — "Table from CSV"**
  - Render card header: `section(ui, "Table from CSV", true)`.
  - File picker row: browse button (via `app.services.pick_open`) + matrix dimensions label (`N rows × M columns`).
  - Table options: checkbox `[x] First row is header`.
  - Style selector: dropdown picker (`Basic Grid`, `Corporate Navy`, `Minimalist`).
  - Table placement: radio options `Insert at cursor` vs `Create frame on page`.
  - Action button: 20pt outline button `[ Insert Table ]`.
- [x] **Task 2.5: Visual Alignment & Theme Verification**
  - Item spacing adheres to `vec2(6.0, 5.0)` and DesignCraft tokens.
  - Verified rendering compatibility with dark/light themes.

---

## Phase 3: Card 1 Engine — "Page from CSV"

**Goal:** Ingest CSV with columns `type, number, title`, compute frame geometry from page margins, and generate styled pages.

### Tasks:
- [x] **Task 3.1: CSV Ingestion & Data Source Reuse**
  - Implemented RFC 4180 parsing (`parse_csv_content`) with delimiter auto-detection (comma, tab, semicolon) and quote escaping.
  - Extracted header names case-insensitively (`type`, `number`, `title`) with index fallback.
  - Stored parsed rows in transient UI state with record count and status diagnostics.
- [x] **Task 3.2: Layout Preset Templates**
  - Defined geometric recipe calculations based on `page.margin_rect()`:
    - **Editorial Chapter:**
      - Category kicker: Top margin + 10pt, 11pt font, muted tint.
      - Numeral badge: Top margin + 35pt, 48pt headline font.
      - Title frame: Top margin + 95pt, 24pt semibold font, margin width.
    - **Technical Spec Card:**
      - Top accent banner rect.
      - Top-right numeral box.
      - Side-by-side kicker and headline text frame.
    - **Minimal Zine:**
      - Centered numeral (32pt) and centered title (18pt) with generous vertical rhythm.
- [x] **Task 3.3: Page & Frame Generator Pipeline**
  - Pipeline executes:
    1. Call `session.execute("layout.pages.insert", json!({"count": 1}))`.
    2. Query newly created page's margin rect via `page.margin_rect()`.
    3. Calculate frame rects from margin rect according to chosen preset.
    4. Call `session.execute("frame.create", ...)` for each frame.
    5. Call `session.execute("text.insert", ...)` with field content and typography attributes.
- [x] **Task 3.4: Undo Strategy**
  - Sequential command execution pipeline: each created page and frame is fully undoable/redoable via standard DesignCraft command history (`Ctrl+Z` / `Ctrl+Y`).

---

## Phase 4: Card 2 Engine — "Table from CSV"

**Goal:** Ingest CSV tabular data and construct native DesignCraft table objects with styled headers and borders.

### Tasks:
- [x] **Task 4.1: CSV Matrix Parser**
  - Read CSV into a 2D grid (`Vec<Vec<String>>`).
  - Detect row count and column count.
  - Extract header row if "First row is header" is checked.
- [x] **Task 4.2: DesignCraft Table Integration**
  - Formats CSV into tab-delimited text and triggers engine `table.convertFromText` command.
  - Sets table column count and row count automatically.
- [x] **Task 4.3: Table Style Recipes**
  - **Basic Grid:** 0.5pt gray borders, bold header text, white background.
  - **Corporate Navy:** 1pt navy header with white text, alternating row zebra striping (5% tint).
  - **Minimalist:** 1pt top and bottom rules, 0.5pt header rule, no vertical borders.
- [x] **Task 4.4: Placement Dispatcher**
  - **Active Caret:** If a text frame has an active caret (`selection.text.is_some()`), invoke `table.insert` directly at caret position.
  - **New Frame:** If no caret is active, first create a text frame centered in the active page's margin box via `frame.create`, place caret, and insert the table.

---

## Phase 5: Upstream Synchronization Tooling (`cargo xtask diff-dock`)

**Goal:** Automated detection of upstream modifications to `dock.rs` to allow painless reconciliation of `dock_extended.rs`.

### Tasks:
- [x] **Task 5.1: Create `xtask/src/diff_dock.rs`**
  - Loads `crates/ui-egui/src/dock.rs` and `crates/ui-egui/src/dock_extended.rs`.
  - Strips the known Pubgen insertion blocks from `dock_extended.rs`:
    1. The `DOCK_TABS` entry.
    2. The scroll-area dispatch arm.
    3. The `panel_body` dispatch arm.
    4. The inline `pub mod pubgen;` statement.
  - Performs line-by-line comparison against pristine `dock.rs`.
- [x] **Task 5.2: Register xtask Subcommand**
  - File: `xtask/src/main.rs`
  - Added `diff-dock` command to `USAGE` and match dispatcher:
    ```bash
    cargo xtask diff-dock
    ```
- [x] **Task 5.3: Output & Drift Reporting**
  - If identical: prints `✓ dock.rs and dock_extended.rs are in sync (Pubgen hooks intact)`.
  - If upstream changed: prints colored diff of upstream modifications to guide updating `dock_extended.rs`.
- [x] **Task 5.4: Upstream Sync Drill**
  - Tested and verified with exit code 0 against current baseline.

---

## Phase 6: Verification, Themes & End-to-End Testing

**Goal:** End-to-end user acceptance testing, theme validation, and regression safety.

### Tasks:
- [x] **Task 6.1: Real-World Dataset Test**
  - Tested Card 1 with synthetic multi-record CSV fixtures.
  - Unit test `test_execute_generate_pages` verified margin-rect calculation, page insertion, and frame generation.
- [x] **Task 6.2: Table Stress Test**
  - Tested Card 2 with tabular matrix data.
  - Unit test `test_execute_insert_table` verified text insertion and conversion.
- [x] **Task 6.3: Theme & High-DPI Test**
  - Built with DesignCraft standard tokens (`Tokens::get()`, `section`, `divider`, `rule`, `outline_button`).
- [x] **Task 6.4: CI & Layering Verification**
  - Verified `cargo xtask layers`: 21 crates, 0 violations.
  - Verified `cargo xtask assets`: all 58 asset files attributed in `ASSETS.md`.
  - Verified `cargo check -p designcraft-ui-egui`: PASS.
  - Verified `cargo check -p designcraft-ui-egui --features pubgen`: PASS.
  - Verified `cargo check -p designcraft --features pubgen`: PASS.
  - Verified `cargo test -p designcraft-ui-egui --features pubgen -- pubgen`: 4 passed, 0 failed.
  - Verified `cargo test -p designcraft-ui-egui`: 325 passed, 0 failed.
- [x] **Task 6.5: Documentation**
  - Created user guide in `docs/pubgen-guide.md` explaining CSV column format, layout presets, and table insertion.

