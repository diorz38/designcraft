# Pubgen Panel User Guide

The **Pubgen** panel integrates automated document generation directly into DesignCraft's right dock (`Properties` · `Pages` · `Layers` · `Pubgen`). It provides two primary workflows:

1. **Page from CSV**: Batch page generation from structured publication catalogs.
2. **Table from CSV**: Dynamic creation of native DesignCraft tables from arbitrary CSV or TSV data.

---

## 1. Page from CSV

### Data Schema
The Page from CSV workflow reads standard RFC 4180 CSV, semicolon-delimited, or tab-delimited files. The header row is automatically analyzed for the following case-insensitive column names:

| Column | Description | Example |
| :--- | :--- | :--- |
| `type` / `category` / `kicker` | Section header or subject classification | `FEATURE`, `OPINION`, `SPECS` |
| `number` / `num` / `issue` | Chapter number, page callout, or issue index | `01`, `Chapter 4`, `№ 12` |
| `title` / `headline` / `name` | Primary headline or article title | `Autonomous Systems in Typography` |

*Note: If specific header names are not found, the parser gracefully falls back to columns 0 (`type`), 1 (`number`), and 2 (`title`). The workflow is strictly restricted to the first 3 columns; any additional columns in the CSV (such as `file`, notes, or metadata) are ignored.*

### Layout Presets
Three geometric layout templates calculate frame coordinates dynamically from the active document page margins:

- **1 Row · 3 Columns (Default)**:
  - Formats the page header into **1 single horizontal row with 3 side-by-side columns**:
    - **Col 1 (Type)**: Fitted snugly to content text width, right-aligned.
    - **Col 2 (Number)**: Fitted snugly to numeral text width.
    - **Col 3 (Title)**: Takes the rest of the available width with space reserved at the end before the right page margin.
- **1 Row · 3 Equal Cols**:
  - Distributes Type, Number, and Title across 3 equal-width columns (1/3 each).
- **Editorial Chapter (Stacked)**:
  - Top category kicker in bold, muted tint.
  - Large headline numeral.
  - Main headline text frame across the full margin width.
- **Technical Spec Card**:
  - Horizontal top rule and two-column specification layout.
  - Distinct numeral badge and technical headline.
- **Minimal Zine**:
  - Centered typography with generous vertical rhythm.
  - Prominent numeral and centered title.

### Record Scope
- **All records**: Processes every data row in the CSV file sequentially.
- **Range (e.g. 1-10)**: Processes only the 1-based index range specified (e.g., `1-5` or `10-25`).

### Page Placement
- **[x] Append to document**: Appends newly generated pages to the end of the document.
- **[ ] Insert after active page**: Inserts pages directly following the currently selected spread/page.

---

## 2. Table from CSV

### Data Schema
The Table from CSV workflow accepts any valid CSV or TSV tabular file.
- The matrix dimensions (`N rows × M columns`) are automatically detected and displayed in the panel.
- **[x] First row is header**: Designates the first line as a table header row.
- **Auto-detected Stacked Headers (up to 3 levels, `Group--Sub--Detail`)**: If the header row contains columns separated by double-dashes (`--`) (e.g. `Item--Price`, `Item--Location--Province`, `Item--Location--Region`), Pubgen automatically constructs a publication-quality multi-level stacked header (supporting up to 3 stacked rows):
  - Common ancestor prefixes (e.g. `Item`, `Location`) merge horizontally across their child columns.
  - Columns with fewer levels (e.g. single-level `Item`, `Category`, `Rating`, or 2-level `Price`, `Stock`) automatically merge vertically downward across the remaining header rows with vertical and horizontal centering.
- **[ ] Add column numbers (1), (2)…**: Inserts an extra row immediately after the table header rows, numbering columns as `(1)`, `(2)`, `(3)`... across all columns (ideal for formal reports, catalogs, and statistical tables).
- **[ ] Table footer (last row)**: Designates the last row as a table footer (`RowKind::Footer`). When paired with the **Horizontal Rules** style, the footer row automatically receives a 0.5pt thin top border and 1.0pt thick bottom border, creating a publication-quality total / summary row.
- **[x] Condition rules (Jumlah, Tahun, Kab, Kec…)**: When paired with the **Horizontal Rules** style, rows where column 1 begins with statistical category or subtotal keywords (`Jumlah`, `Tahun`, `Kab`, `Kabupaten`, `Kec`, `Kecamatan`, `Total`) automatically receive top and bottom thin borders (0.5pt), cleanly framing intermediate and terminal summary rows.
- **Auto-detected Footnotes / Source Notes**: If the last row of the CSV contains text only in the first column with all subsequent columns empty (e.g. `Sources: Karawang Region Government Office,,,,,,` as in `sample_3stack_table.csv`), Pubgen automatically:
  - Excludes the line from the table matrix so grid columns remain clean.
  - Automatically synthesizes a standalone text box positioned directly below the generated table with the footnote/source text.
  - Treats the row immediately preceding the footnote as the last table row (which becomes the styled table footer if selected).

### Table Placement
- **At cursor**: If a text frame has an active caret selection, tab-delimited table text is inserted directly at the cursor and converted into a native table via `table.convertFromText`.
- **New frame**: Automatically creates a new text frame centered within the page margins starting at the top margin.
- **At page**: Targets the page currently selected in the **Pages** tab (or active in view). Automatically detects the bottom edge of the generated page title/header on that page and creates the table frame directly below it (with a 20pt buffer). Displays a live indicator badge (e.g. `Target: Page 2 (below title)`).

### Table Styles
- **Basic Grid**: Clean 0.5pt grid borders with bold header cells.
- **Corporate Navy**: High-contrast header styling with alternating zebra-striping.
- **Minimalist**: Clean minimal 0.5pt borders.
- **Horizontal Rules**: Publication-quality booktabs style featuring 1pt top and bottom rules, 0.5pt header separator, 0.5pt column numbers separator (if checked), 0.5pt footer separator (if Table footer checked), and no vertical or intermediate body borders.

---

## 3. Quick Actions

The **Quick Actions** card at the bottom of the Pubgen panel provides handy shortcuts matching the InDesign-style Properties panel workflow:

- **Single-sided / Double-sided Toggle**:
  - Dynamically detects the current document layout mode.
  - One-click toggle between **Facing Pages** (Double-sided spreads) and **Single-sided** layouts.
  - Non-destructively repaginates the document (`layout.documentSetup`) while translating existing text frames and graphics safely.
- **Add Blank Page**:
  - Automatically identifies the currently selected page (via canvas view, selection, or Pages tab).
  - Inserts 1 blank page directly after the selected page via `layout.pages.insert`.
  - Automatically scrolls the canvas view to the newly inserted page.
- **Add TOC (selected page)**:
  - Generates or updates a Table of Contents on the selected page with automatic title early wrapping and deep hanging indent.
  - Automatically creates continuation pages with threaded text frames when entries exceed 1 page.
- **Excel to CSV Data**:
  - Automatically converts a multi-sheet Excel file (`.xlsx`) into:
    1. **Catalog CSV** (`<workbook>_catalog.csv`): Extracts row 1 of each sheet into `type,number,title,file` (where `file` is the sheet name). Automatically loaded into **Page from CSV** (Card 1).
    2. **Table CSVs** (`<sheet_name>.csv`): Automatically parses each sheet, detecting merged columns/rows and converting them into stacked publication headers (`Parent--Child`). Automatically loaded into **Table from CSV** (Card 2) with a **Sheet table** dropdown selector to easily switch between sheets (`02_01.csv`, `02_02.csv`, etc.).
    3. **Combined ZIP Archive** (`<workbook>_csvs.zip`): Automatically packages the catalog CSV and all sheet table CSVs into a ready-to-use ZIP archive saved directly in the destination folder.
  - **Download CSVs as ZIP…**:
    - One-click export allowing the user to download / save all generated catalog and table CSV files packaged together in a single `.zip` file at any time.
    - Uses native OS Save File dialog on desktop and browser download delivery on web.
  - **Auto-insert table from 'file' column**: In Card 1 (**Page from CSV**), when the catalog contains a `file` column, an option `[x] Auto-insert table from 'file' col (e.g. 02_01.csv)` allows one-click generation of both pages and their corresponding styled tables positioned cleanly below each page title.
- **Live Metadata Indicator**:
  - Shows current layout status and selected target page at a glance (e.g. `Mode: Double-sided · Selected: Page 1`).

---

## 4. Upstream Synchronization & Maintenance

Pubgen is isolated from upstream changes via a conditional compilation seam:
- Upstream `crates/ui-egui/src/dock.rs` remains **100% untouched**.
- The extended dock lives in `crates/ui-egui/src/dock_extended.rs`.
- Check drift against upstream releases at any time by running:
  ```bash
  cargo xtask diff-dock
  ```
