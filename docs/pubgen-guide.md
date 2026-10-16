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

*Note: If specific header names are not found, the parser gracefully falls back to columns 0 (`type`), 1 (`number`), and 2 (`title`).*

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

## 3. Upstream Synchronization & Maintenance

Pubgen is isolated from upstream changes via a conditional compilation seam:
- Upstream `crates/ui-egui/src/dock.rs` remains **100% untouched**.
- The extended dock lives in `crates/ui-egui/src/dock_extended.rs`.
- Check drift against upstream releases at any time by running:
  ```bash
  cargo xtask diff-dock
  ```
