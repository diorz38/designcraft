# DesignCraft Pubgen Feature: Complete User & Reference Guide

**Document Version:** 1.0.0  
**Target Application:** DesignCraft (`crates/ui-egui`, `crates/engine`, `crates/doc`)  
**Feature Flag:** `pubgen` (`--features pubgen`)

---

## Table of Contents

1. [Executive Overview](#1-executive-overview)
2. [Interface Integration & Layout Architecture](#2-interface-integration--layout-architecture)
3. [Card 1: Page from CSV (Automated Page & Catalog Generation)](#3-card-1-page-from-csv-automated-page--catalog-generation)
   - [Supported CSV Data Schemas](#31-supported-csv-data-schemas)
   - [Layout Preset Templates](#32-layout-preset-templates)
   - [Record Selection & Scope](#33-record-selection--scope)
   - [Page Placement Options](#34-page-placement-options)
   - [Execution Pipeline & Transactional Undo](#35-execution-pipeline--transactional-undo)
4. [Card 2: Table from CSV (Advanced Publication Table Import)](#4-card-2-table-from-csv-advanced-publication-table-import)
   - [Data Schema & Matrix Detection](#41-data-schema--matrix-detection)
   - [Auto-Detected Multi-Level Stacked Headers (Up to 3 Levels)](#42-auto-detected-multi-level-stacked-headers-up-to-3-levels)
   - [Column Numbering `(1), (2)…`](#43-column-numbering-1-2)
   - [Table Footer & Summary Rows](#44-table-footer--summary-rows)
   - [Conditional Category & Subtotal Rule Framing](#45-conditional-category--subtotal-rule-framing)
   - [Footnotes & Source Notes Auto-Detection](#46-footnotes--source-notes-auto-detection)
   - [Table Styles & Formatting Presets](#47-table-styles--formatting-presets)
   - [Smart Table Placement Dispatching](#48-smart-table-placement-dispatching)
5. [Card 3: Quick Actions & Document Utilities](#5-card-3-quick-actions--document-utilities)
   - [Single-Sided / Double-Sided Layout Toggle](#51-single-sided--double-sided-layout-toggle)
   - [Add Blank Page](#52-add-blank-page)
   - [Live Metadata & Status Indicators](#53-live-metadata--status-indicators)
6. [CLI & Control Channel Automation](#6-cli--control-channel-automation)
   - [Headless Execution via `designcraft-cli`](#61-headless-execution-via-designcraft-cli)
   - [Control Channel JSON-RPC API](#62-control-channel-json-rpc-api)
7. [Upstream Synchronization & Developer Maintenance](#7-upstream-synchronization--developer-maintenance)
   - [The Conditional Compilation Seam](#71-the-conditional-compilation-seam)
   - [Verifying Drift via `cargo xtask diff-dock`](#72-verifying-drift-via-cargo-xtask-diff-dock)
   - [Running Unit Tests](#73-running-unit-tests)
   - [Packaging Portable Windows Builds](#74-packaging-portable-windows-builds)

---

## 1. Executive Overview

The **Pubgen** (Publishing Generator) feature in DesignCraft is a high-performance, native Rust module designed for rapid, automated document assembly and catalog generation. Pubgen bridges structured tabular data (CSV/TSV files) directly into DesignCraft’s layout engine, allowing publishing professionals to:

- **Generate Multi-Page Catalogs, Magazines, or Books:** Automatically build styled document spreads from structured CSV files (containing chapter titles, section kickers, and numerals).
- **Import Publication-Quality Tables:** Convert raw CSV/TSV data into native DesignCraft tables complete with auto-detected multi-level stacked headers (up to 3 levels), column index numbers, styled footers, subtotal framing, and external source footnotes.
- **Perform Instant Document Repagination:** Toggle non-destructively between single-sided and double-sided (facing pages) layouts or insert blank pages relative to active canvas selections with a single click.

Pubgen operates with complete visual parity to DesignCraft’s native **Properties** panel and is engineered using a zero-footprint feature seam (`--features pubgen`) that guarantees zero git merge conflicts when pulling upstream engine updates.

---

## 2. Interface Integration & Layout Architecture

Pubgen is seamlessly integrated into DesignCraft's right dock tab strip alongside native panels:

```
+-------------------------------------------------------+
|  [Properties]   [Pages]   [Layers]   [Pubgen]         | ← Right Dock Tab Strip
+-------------------------------------------------------+
|  Pubgen                 [ Set Page Margin ]           | ← Header Band (33pt)
+-------------------------------------------------------+
|  Page from CSV                                        | ← Card 1: Section Header
|  Source:  [ Browse… ]  articles.csv (24 rows)        |
|  Columns: [type: "Feature"] [number: "01"] [title]    |
|  Layout:  [ 1 Row · 3 Columns (Default)           v ] |
|  Records: (•) All   ( ) Range [ 1 - 10 ]              |
|  Target:  [x] Append to document                      |
|           [ Generate Pages ]                          |
+-------------------------------------------------------+ ← Section Divider Rule (1.5pt)
|  Table from CSV                                       | ← Card 2: Section Header
|  Source:  [ Browse… ]  data.csv (12 rows × 4 cols)    |
|  Headers: [x] First row is header                     |
|           [x] Add column numbers (1), (2)…            |
|           [x] Table footer (last row)                 |
|           [x] Condition rules (Jumlah, Total…)        |
|  Style:   [ Horizontal Rules                      v ] |
|  Target:  ( ) At cursor  ( ) New frame  (•) At page   |
|           Badge: Target: Page 2 (below title)         |
|           [ Insert Table ]                            |
+-------------------------------------------------------+ ← Section Divider Rule (1.5pt)
|  Quick Actions                                        | ← Card 3: Quick Actions
|  [ Single-sided -> Double-sided ]  [ + Add Blank Page ]|
|  Status: Mode: Double-sided · Selected: Page 1        |
+-------------------------------------------------------+
```

### Visual & Design Tokens
Pubgen strictly adopts DesignCraft’s InDesign-parity design token system (`crate::theme::Tokens`):
- **Header Band:** 33pt header band with "Pubgen" title and **[ Set Page Margin ]** button to apply standard publication margins (Top: 20.20 mm, Bottom: 20 mm, Inside: 19 mm, Outside: 15 mm).
- **Section Headers:** `section(ui, title, true)` with 11.5pt semibold typography.
- **Rules & Dividers:** `rule(ui)` (1.5pt solid rule) and `divider(ui)` between card blocks.
- **Form Controls:** 21pt input fields (`FIELD_H = 21.0`), 20pt outline buttons (`outline_button(...)`), and chevron dropdown pickers (`dropdown(...)`).
- **Theme Support:** Fully adaptive across all 4 brightness themes (**Dark**, **Medium Dark**, **Medium Light**, **Light**) and high-DPI display scales (100%, 125%, 150%, 200%).

---

## 3. Card 1: Page from CSV (Automated Page & Catalog Generation)

Card 1 parses structured CSV files and sequentially constructs document pages using predefined geometric layout templates.

### 3.1 Supported CSV Data Schemas

The parser accepts standard RFC 4180 CSV, semicolon-delimited (`;&`), or tab-delimited (`\t`) files. Headers are evaluated case-insensitively for the following roles:

| Role | Header Aliases Checked | Description | Example Value |
| :--- | :--- | :--- | :--- |
| **Type / Kicker** | `type`, `category`, `kicker`, `section` | Section classification or category kicker | `FEATURE`, `SPECS`, `CHAPTER` |
| **Number / Index** | `number`, `num`, `issue`, `index` | Numeral callout or chapter index | `01`, `Chapter 4`, `№ 12` |
| **Title / Headline** | `title`, `headline`, `name`, `subject` | Primary chapter title or article headline | `Autonomous Systems in Layout` |

*Fallback Behavior:* If specific header names are missing, Pubgen automatically maps positional columns:
- Column 0 `->` **Type**
- Column 1 `->` **Number**
- Column 2 `->` **Title**

#### Sample CSV File (`sample_chapters.csv`)
```csv
type,number,title
FEATURE,01,The Evolution of Digital Typesetting
SPECS,02,High-Speed Layout Engine Architecture
CASE STUDY,03,Publishing Workflows at Scale
```

---

### 3.2 Layout Preset Templates

Pubgen computes exact frame geometries relative to the active document's page margins (`margin_rect()`):

1. **1 Row · 3 Columns (Default):**
   - Formats the page header into **1 single horizontal row containing 3 side-by-side columns**:
     - **Column 1 (Type):** Fitted snugly to the text width of the category string (right-aligned).
     - **Column 2 (Number):** Fitted snugly to the text width of the numeral string.
     - **Column 3 (Title):** Fills the remaining available page margin width with reserved end padding before the right margin.
2. **1 Row · 3 Equal Cols:**
   - Distributes Type, Number, and Title across **3 equal-width columns** (1/3 of the page margin width each).
3. **Editorial Chapter (Stacked):**
   - **Top Kicker:** Positioned at top margin + 20pt (11pt bold, muted tint).
   - **Numeral Badge:** Large 48pt headline font at top margin + 44pt.
   - **Title Frame:** Full margin-width text frame at top margin + 110pt (24pt semibold font).
4. **Technical Spec Card:**
   - Top horizontal accent rule with a top-right numeral box and two-column specification headline block.
5. **Minimal Zine:**
   - Clean centered typography with generous vertical rhythm (32pt centered numeral and 18pt centered title).

---

### 3.3 Record Selection & Scope

- **All records:** Processes every row in the CSV file sequentially.
- **Range (e.g. 1-10):** Limits generation to a specific 1-based row interval (e.g., `1-5` or `10-25`).

---

### 3.4 Page Placement Options

- **[x] Append to document:** Appends all generated pages to the end of the document.
- **[ ] Insert after active page:** Inserts newly created pages directly after the currently selected page or spread in the canvas/Pages panel.

---

### 3.5 Execution Pipeline & Transactional Undo

When you click **[ Generate Pages ]**:
1. Pubgen reads and parses the target CSV dataset.
2. Loops through the selected record range, executing `layout.pages.insert` for each row.
3. Obtains each new page's margin rectangle (`margin_rect()`).
4. Executes `frame.create` and `text.insert` commands for each calculated frame position.
5. **Single-Step Undo Guarantee:** The entire batch operation is executed as a single atomic transaction. Pressing `Ctrl+Z` immediately reverses the entire generation batch, removing all created pages and frames in one step.

---

## 4. Card 2: Table from CSV (Advanced Publication Table Import)

Card 2 converts raw tabular data into native DesignCraft publication table objects featuring automated professional typographic formatting.

```
+---------------------------------------------------------------------------------+
|                                 LOCATION                                        | ← Stacked Level 1 (Merged)
+----------------------------------------+----------------------------------------+
|                PROVINCE                |                 REGION                 | ← Stacked Level 2 (Merged)
+-------------------+--------------------+-------------------+--------------------+
|  District Name    |  Population        |  Area (sq km)     |  Density           | ← Header Row
+-------------------+--------------------+-------------------+--------------------+
|  (1)              |  (2)               |  (3)              |  (4)               | ← Column Numbers Row
+===================+====================+===================+====================+
|  Central District |  1,245,000         |  450.5            |  2,763             | ← Data Body Row
|  North Suburbs    |  840,000           |  620.0            |  1,354             |
+-------------------+--------------------+-------------------+--------------------+
|  Jumlah Total     |  2,085,000         |  1,070.5          |  1,947             | ← Category Subtotal Rule
+===================+====================+===================+====================+
|  Total Summary    |  2,085,000         |  1,070.5          |  1,947             | ← Table Footer (Thick Rule)
+-------------------+--------------------+-------------------+--------------------+
  Source: Regional Government Office 2026                                            ← Auto-Extracted Footnote
```

---

### 4.1 Data Schema & Matrix Detection

Pubgen reads standard CSV and TSV matrix files, automatically calculating and displaying the dimensions in the UI (e.g., `Source: data.csv (12 rows × 4 cols)`).

---

### 4.2 Auto-Detected Multi-Level Stacked Headers (Up to 3 Levels)

When **[x] First row is header** is enabled, Pubgen scans column headers for double-dash notation (`Group--Sub--Detail`):

- **3-Level Header Example (`sample_3stack_table.csv`):**
  ```csv
  Location--Province--District,Location--Province--Population,Location--Region--Area,Category
  ```
- **Horizontal Merging:** Adjacent columns sharing identical top-level or mid-level prefix strings (e.g., `Location`, `Province`) are merged horizontally across child columns.
- **Vertical Merging:** Columns with fewer hierarchy levels (e.g., single-level `Category` or double-level `Location--Region`) automatically merge vertically downward across remaining header rows with vertical and horizontal text centering.
- **Max Stack Limit:** Supports up to 3 stacked header rows.

---

### 4.3 Column Numbering `(1), (2)…`

When **[x] Add column numbers (1), (2)…** is checked:
- Pubgen inserts a dedicated index row directly beneath the header rows.
- Formats every column as `(1)`, `(2)`, `(3)`... across the entire table width.
- Essential for official reports, government statistics, and technical catalogs.

---

### 4.4 Table Footer & Summary Rows

When **[x] Table footer (last row)** is checked:
- Designates the final data row as a table footer (`RowKind::Footer`).
- When paired with the **Horizontal Rules** style, the footer row receives a 0.5pt thin top rule and a 1.0pt thick bottom rule.

---

### 4.5 Conditional Category & Subtotal Rule Framing

When **[x] Condition rules (Jumlah, Total…)** is checked:
- Pubgen inspects the text in Column 1 of every data row.
- Rows matching statistical subtotal keywords (`Jumlah`, `Tahun`, `Kab`, `Kabupaten`, `Kec`, `Kecamatan`, `Total`) automatically receive top and bottom 0.5pt thin rules, neatly framing intermediate and terminal summaries.

---

### 4.6 Footnotes & Source Notes Auto-Detection

If the last row of a CSV file contains text only in Column 1 with all remaining columns empty:
```csv
Sources: Regional Government Office 2026,,,,
```
Pubgen automatically:
1. Excludes the line from the table matrix so grid columns remain perfectly aligned.
2. Synthesizes a standalone text frame positioned directly below the generated table frame (with a 6pt gap).
3. Populates the frame with the footnote/source text in 9pt italic typography.

---

### 4.7 Table Styles & Formatting Presets

| Style Preset | Border & Cell Styling |
| :--- | :--- |
| **Basic Grid** | Standard 0.5pt gray grid borders, bold header text, white background. |
| **Corporate Navy** | Dark navy header background with white text, alternating 5% tint zebra striping on body rows. |
| **Minimalist** | Light 0.5pt horizontal rules, no vertical borders. |
| **Horizontal Rules** | Publication **booktabs** style: 1.0pt top and bottom rules, 0.5pt header rule, 0.5pt column numbers rule, 0.5pt footer rule, no vertical or intermediate body rules. |

---

### 4.8 Smart Table Placement Dispatching

- **At cursor:** Inserts tab-delimited table text inline at the active text caret position and converts it to a native table via `table.convertFromText`.
- **New frame:** Creates a new text frame centered within the page margins starting at the top margin.
- **At page (Smart Header Avoidance):** Targets the active page selected in the Pages panel or canvas. Pubgen automatically inspects existing items on that page, detects the bottom edge of any page title/header frame, and positions the new table frame directly below it with a 20pt safety buffer. Displays a live indicator badge (e.g., `Target: Page 2 (below title)`).

---

## 5. Card 3: Quick Actions & Document Utilities

Card 3 provides quick-access document manipulation utilities directly inside the Pubgen panel.

```
+-------------------------------------------------------+
|  Quick Actions                                        |
|  [ Single-sided -> Double-sided ]  [ + Add Blank Page ]|
|  Status: Mode: Double-sided · Selected: Page 1        |
+-------------------------------------------------------+
```

### 5.1 Single-Sided / Double-Sided Layout Toggle

- **Dynamic Detection:** Inspects whether the document is currently in **Facing Pages** (Double-sided spreads) or **Single-sided** mode.
- **One-Click Repagination:** Clicking the toggle invokes `layout.documentSetup` to non-destructively convert the layout mode. Existing text frames, margins, and placed graphics are safely translated to match the new spread geometry.

---

### 5.2 Add Blank Page

- **Selection Awareness:** Identifies the currently selected page on the canvas or in the Pages panel.
- **Instant Insertion:** Calls `layout.pages.insert` to insert 1 blank page directly after the selected page.
- **Auto-Scroll:** Automatically scrolls the main canvas viewport to center the newly inserted page.

---

### 5.3 Live Metadata & Status Indicators

Displays document layout state in real-time:
```
Mode: Double-sided · Selected: Page 1
```

---

## 6. CLI & Control Channel Automation

All Pubgen features can be driven headlessly or remotely via scripts and API channels.

### 6.1 Headless Execution via `designcraft-cli`

You can execute Pubgen scripts using `designcraft-cli`:

```bash
# Execute a Pubgen generation script against a template document
designcraft-cli script generate_catalog.dcs --in template.designcraft --save catalog.designcraft --export catalog.pdf
```

#### Sample Script (`generate_catalog.dcs`)
```dcs
# Select CSV data source
data.source.select {"path": "samples/sample_chapters.csv"}

# Import table with Horizontal Rules style
table.insert {"rows": 10, "cols": 4, "headerRows": 1, "width": 500}

# Save document
file.save {"path": "output/catalog.designcraft"}
```

---

### 6.2 Control Channel JSON-RPC API

When DesignCraft is launched with `--control 7979`, external processes can send JSON-RPC commands:

```json
{
  "id": 1,
  "method": "engine.execute",
  "params": {
    "command": "layout.pages.insert",
    "params": { "count": 2 }
  }
}
```

---

## 7. Upstream Synchronization & Developer Maintenance

### 7.1 The Conditional Compilation Seam

Pubgen is isolated from upstream changes using conditional compilation:
- **Upstream Baseline:** `crates/ui-egui/src/dock.rs` remains **100% untouched**.
- **Extended Dock:** `crates/ui-egui/src/dock_extended.rs` is compiled when `--features pubgen` is active.

```rust
// In crates/ui-egui/src/lib.rs:
#[cfg(not(feature = "pubgen"))]
pub mod dock;

#[cfg(feature = "pubgen")]
#[path = "dock_extended.rs"]
pub mod dock;
```

---

### 7.2 Verifying Drift via `cargo xtask diff-dock`

To verify that `dock_extended.rs` remains in sync with `dock.rs` after pulling upstream commits:

```bash
cargo xtask diff-dock
```

**Output when synchronized:**
```
✓ dock.rs and dock_extended.rs are in sync (Pubgen hooks intact).
```

---

### 7.3 Running Unit Tests

Run Pubgen's unit test suite:

```bash
cargo test -p designcraft-ui-egui --features pubgen -- pubgen
```

---

### 7.4 Packaging Portable Windows Builds

To package a standalone, portable Windows x64 ZIP containing static CRT binaries and sample datasets (organized under `samples\csv\` and `samples\xlsx\`):

```powershell
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release -p designcraft -p designcraft-cli --features designcraft/pubgen --target x86_64-pc-windows-msvc
```

Then assemble the release folder under `dist\designcraft-portable-x64\` with a `portable.txt` marker file and `samples\csv\` / `samples\xlsx\` subdirectories.
