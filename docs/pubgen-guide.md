# Pubgen: The Publication Generator — Comprehensive User Guide

> **Welcome to Pubgen!** This guide is written for publication editors, layout designers, statisticians, and institutional authors. It explains every single feature, setting, and button in the **Pubgen** panel in clear, non-technical language.

---

## Table of Contents
1. [What is Pubgen?](#1-what-is-pubgen)
2. [Finding the Pubgen Panel in DesignCraft](#2-finding-the-pubgen-panel-in-designcraft)
3. [Card 1: Publication Cover (Cover Page Generator)](#3-card-1-publication-cover-cover-page-generator)
   - [Overview & Position](#overview--position)
   - [The 10 Executive Cover Styles](#the-10-executive-cover-styles)
   - [Hover Thumbnail Previews](#hover-thumbnail-previews)
   - [Theme Colors & Color Customization](#theme-colors--color-customization)
   - [Metadata & Content Fields](#metadata--content-fields)
   - [Custom Image & Logo Pickers](#custom-image--logo-pickers)
   - [Generating the Cover](#generating-the-cover)
4. [Card 2: Page from CSV / Excel (Batch Page & Chapter Synthesizer)](#4-card-2-page-from-csv--excel-batch-page--chapter-synthesizer)
   - [How It Works](#how-it-works)
   - [Input Files (CSV & Excel)](#input-files-csv--excel)
   - [Column Mapping (Type, Number, Title)](#column-mapping-type-number-title)
   - [Layout Presets](#layout-presets)
   - [Record Scope & Placement](#record-scope--placement)
   - [Auto-Inserting Tables from 'file' Column](#auto-inserting-tables-from-file-column)
5. [Card 3: Table from CSV / Excel (Statistical Table Builder)](#5-card-3-table-from-csv--excel-statistical-table-builder)
   - [How It Works](#how-it-works-1)
   - [Header Options & Multi-Level Stacked Headers](#header-options--multi-level-stacked-headers)
   - [Column Numbering Row: (1), (2), (3)...](#column-numbering-row-1-2-3)
   - [Table Footer & Auto-Detected Footnotes / Sources](#table-footer--auto-detected-footnotes--sources)
   - [Condition Rules: Smart Statistical Highlighting](#condition-rules-smart-statistical-highlighting)
   - [Table Styles (Horizontal Rules, Grid, Striped)](#table-styles)
   - [Placement Modes (At Page, New Frame, At Cursor)](#placement-modes)
   - [Multi-Sheet Excel Workbooks & ZIP Export](#multi-sheet-excel-workbooks--zip-export)
6. [Card 4: Quick Actions & Editorial Utilities](#6-card-4-quick-actions--editorial-utilities)
   - [Single-Sided vs. Double-Sided Spreads Toggle](#single-sided-vs-double-sided-spreads-toggle)
   - [Add Blank Page](#add-blank-page)
   - [Automated Table of Contents (Add TOC / Rebuild TOC)](#automated-table-of-contents-add-toc--rebuild-toc)
   - [Page from RTF (Rich Text Import & Autoflow)](#page-from-rtf-rich-text-import--autoflow)
   - [Page from Image (Object vs. Full-Bleed Background)](#page-from-image-object-vs-full-bleed-background)
   - [Page Renumbering & Sections (Roman vs. Arabic)](#page-renumbering--sections-roman-vs-arabic)
7. [Step-by-Step Practical Tutorials](#7-step-by-step-practical-tutorials)
   - [Tutorial 1: Creating a Complete Publication in 3 Minutes](#tutorial-1-creating-a-complete-publication-in-3-minutes)
   - [Tutorial 2: Designing a Custom Cover with Your Own Photo & Colors](#tutorial-2-designing-a-custom-cover-with-your-own-photo--colors)
   - [Tutorial 3: Formatting Complex Statistical Tables from Excel](#tutorial-3-formatting-complex-statistical-tables-from-excel)
8. [Frequently Asked Questions & Troubleshooting](#8-frequently-asked-questions--troubleshooting)

---

## 1. What is Pubgen?

**Pubgen** (Publication Generator) is an automated layout engine built directly into DesignCraft.

In conventional page layout programs (like Adobe InDesign or QuarkXPress), assembling a 300-page statistical yearbook or government report (such as *Kecamatan Dalam Angka* or an Annual Economic Census) requires hundreds of manual operations: drawing text frames, styling headers, formatting tables, placing logos, calculating TOC leader dots, and formatting covers.

Pubgen turns this into a fast, automated workflow:
- **Instant Covers**: Generates executive corporate covers with professional geometry, drop shadows, and clipped photographs.
- **Instant Pages**: Reads your table of contents or chapter list from Excel/CSV and creates formatted pages in seconds.
- **Publication Tables**: Formats statistical tables with BPS-standard booktabs horizontal rules, stacked column headers, column numbers, and conditional highlighting for totals.
- **Live Editorial Tools**: Handles double-sided facing pages, Roman/Arabic page renumbering, RTF text autoflow, and Table of Contents generation.

---

## 2. Finding the Pubgen Panel in DesignCraft

When you open DesignCraft, look at the **Right Dock Panel**:

```
+-------------------------------------------------------+
|  [Properties]   [Pages]   [Layers]   [Pubgen]         |
+-------------------------------------------------------+
|                                                       |
|  ▼ Page from CSV / Excel                              |
|  ▼ Table from CSV / Excel                             |
|  ▼ Publication Cover                                  |
|  ▼ Quick Actions                                      |
|                                                       |
+-------------------------------------------------------+
```

Click the **Pubgen** tab. You will see four collapsible cards:
1. **Page from CSV**
2. **Table from CSV**
3. **Publication Cover**
4. **Quick Actions**

You can click the triangle arrow `▼` next to any section header to expand or collapse it.

---

## 3. Card 1: Publication Cover (Cover Page Generator)

The **Publication Cover** section allows you to synthesize a complete, professional publication cover with a single click.

```
+-------------------------------------------------------------------+
| Publication Cover                                                 |
| Style: [ Folded Origami        ▼ ]  [👁 Preview]                   |
| Preview: [🌊 Wave] [🔷 Geometric] [📐 Origami] [🔰 Chevron] [⬡ Hexagon]|
|          [🏢 Corporate] [↗ Arrow] [🌇 Horizon] [📊 Report] [🌆 Urban] |
|                                                                   |
| Theme:  [ Corporate Sapphire ▼ ]                                  |
| Colors: [ Primary: #0072bc ] [ Accent: #f5ab19 ] [ Dark: #111827 ]|
|                                                                   |
| Title:  [ KARAWANG TIMUR                                   ]      |
| Title 2:[ Title 2                                            ]      |
| Year:   [ 2026        ]  Volume: [ Volume 23 2026          ]      |
|                                                                   |
| Catalog:[ 1102001.3215113 ]  Code: (•) ISSN ( ) ISBN [ 2828-7436 ]|
|                                                                   |
| Cover Images:                                                     |
| Honeycomb: [ outputs/city_landscape.jpg         ] [Browse...]     |
| Logo:      [ outputs/bps_logo.png               ] [Browse...]     |
| Logo Text: [ BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG     ]      |
|                                                                   |
| [  Generate Cover Page  ]                                         |
+-------------------------------------------------------------------+
```

### Overview & Position
- **Always Page 1 (Front Cover)**: When you click **Generate Cover Page**, Pubgen automatically inserts a new page before the existing Page 1 and navigates to it. Your existing layout and content are preserved.
- **Proportional Scaling**: Whether your document is **A4**, **A5**, or **B5**, all polygons, frames, and typography scale proportionally (`sx`, `sy`, `s_avg`) so the layout always looks balanced.

---

### The 10 Executive Cover Styles

Pubgen provides 10 corporate design styles:

#### 1. Wave Curves (`🌊 Wave`)
- **Visual Personality**: Organic, modern, flowing.
- **Design Elements**:
  - Two large sweeping asymmetrical curves across the canvas (a navy blue lower wave and an upper white wave).
  - Concentric circular cradle rings in amber gold.
  - A prominent circular medallion in the center holding the landscape or city photo.
  - Title and metadata neatly arranged in high-contrast text blocks.
- **Best For**: Environmental reports, regional overviews, public services, social publications.

#### 2. Geometric Medallion (`❖ Medallion`)
- **Visual Personality**: Sleek, architectural, structured.
- **Design Elements**:
  - Crisp diagonal color-blocked dividing planes.
  - Centered circular photo cutout bordered by glowing gold and navy concentric rings with soft ambient drop shadow.
  - Dynamic diagonal accent ribbon across the top and bottom.
- **Best For**: Economic censuses, urban planning, industrial statistics, corporate annual reports.

#### 3. Folded Origami (`◈ Origami`)
- **Visual Personality**: High-tech, faceted, dimensional.
- **Design Elements**:
  - Interlocking geometric polygons with simulated paper-fold lighting and depth.
  - A high-impact panoramic photograph spanning the upper half of the cover.
  - Asymmetric amber-gold geometric facets creating dynamic forward motion.
  - Large focal typography block in the lower corner.
- **Best For**: Technology, innovation, trade, commercial directories, modern regional digests.

#### 4. Chevron & Medallion (`◆ Chevron`)
- **Visual Personality**: Prestigious, authoritative, classic institutional.
- **Design Elements**:
  - Clean, crisp white canvas background.
  - Bold downward corporate sapphire chevron framing the primary publication title.
  - Large central circular medallion holding the skyline photo, framed with a golden ring.
  - Giant focal year numerals (`2026`) in **Arial Black** centered directly over the medallion.
  - Bottom institutional branding bar with BPS emblem.
- **Best For**: Official government publications, national statistical digests, formal commemorative editions.

#### 5. Honeycomb Hexagon (`⬡ Hexagon`)
- **Visual Personality**: Executive, sophisticated, premium corporate.
- **Design Elements**:
  - Deep Midnight Navy/Slate canvas (`#111827`) providing rich depth.
  - Triple vertical architectural stripes in tiered amber gold (`0.85`, `0.60`, `0.35` opacity) along the right side.
  - Corporate sapphire chevron banner pointing inward from the left margin, underlined by crisp gold piping.
  - **12-Vertex Tri-Hexagon Honeycomb Frame**: An interlocking triple-hexagon polygon cluster with a 10pt crisp white framing border, soft drop shadow (`distance: 6`, `size: 18`), holding the clipped photograph, with delicate gold contour piping.
  - Giant 60pt **Arial Black** focal year callout on the lower left with an amber underline rule.
- **Best For**: Flagship publications, executive summaries, international releases, premium statistical editions.

#### 6. Corporate Headline (`🏢 Corporate`)
- **Visual Personality**: Contemporary corporate, bold architectural, clean editorial.
- **Design Elements**:
  - Crisp white canvas with an asymmetric top purple framing header (`#6c5ce7`).
  - **Position Lock**: Top-right publication metadata (Line #1: `Katalog BPS: <number>`, Line #2: `ISSN/ISBN <number>` in crisp **Arial Bold**).
  - Upper-right corporate accent card with soft ambient drop shadow.
  - Tall vertical architectural / cityscape photograph (`[221, 168, 565, 780]`) framed by an accent L-bracket on the upper-left.
  - Middle-left dual-tone year display (`20` in dark charcoal `#1e293b`, `26` in purple `#6c5ce7`) with **Volume** (`Volume 23 2026`) directly underneath and an accent underline rule.
  - Asymmetric semi-transparent focal headline purple block (`[0, 458, 340, 680]`, 85% opacity) featuring a 46pt bezier rounded top-right corner, letting the background cityscape photo subtly show through while framing bold white title typography (**Arial Black**).
  - **Position Lock**: Bottom-left official agency logo paired with multi-line institutional branding (**Arial Bold Italic**).
  - Bottom-right grounding purple corner block and mid-right edge accent tab.
- **Best For**: Annual business reports, urban economic reviews, municipal profiles, modern regional digests.

#### 7. Diagonal Arrow (`↗ Arrow`)
- **Visual Personality**: Directional, architectural, modern corporate.
- **Design Elements**:
  - Crisp white canvas with a full-height cityscape photo panel bleeding off the right edge (`[286, 0, 595, 842]`).
  - A bold navy diagonal banner leaning across the top (`[113, 0, 400, 0, 287, 256, 0, 256]`), its lower edge cutting into the photograph.
  - An amber corner wedge in the top-left (`[0, 0, 87, 0, 0, 196]`), separated from the banner by a thin white channel.
  - **Position Lock**: publication metadata (Line #1: `Katalog BPS: <number>`, Line #2: `ISSN/ISBN <number>` in crisp **Arial Bold**) sits top-right, over the photograph.
  - A navy forward wedge crossing out of the white field into the photo (`[0, 519, 352, 591, 0, 663]`), with a soft drop shadow.
  - The publication year set in giant 44pt **Arial Black**, centred in that wedge, with the **Volume** line (`Volume 23 2026`) directly beneath it.
  - Middle-left headline stack in **Arial Black**: the region level, the publication title in the accent colour on one full-width line, and `DALAM ANGKA` beneath it.
  - **Position Lock**: bottom-left official agency logo paired with multi-line institutional branding (**Arial Bold Italic**).
- **Best For**: Corporate annual reports, business reviews, city profiles, investment and industry publications.

#### 8. Modern Horizon (`🌇 Horizon`)
- **Visual Personality**: Editorial, institutional, modern annual-report cover.
- **Design Elements**:
  - A **full-bleed cityscape photograph** across the top, clipped by a sweeping arc that runs from the lower half of the left edge to the middle of the right edge (the traced geometry of `design-5.designcraft`).
  - A **bold orange wedge** that follows the underside of that arc: fat along the left edge, tapering out where the right-hand accent block begins, so the two read as one continuous accent.
  - A **dark institutional block** filling the lower page, with a large rounded top-left corner; thin white seams separate the photo, the wedge and the block.
  - A **right-hand accent block** carrying the publication year in giant **Arial Black** with the **Volume** line directly beneath it.
  - **Position Lock**: publication metadata (Line #1: `Katalog BPS: <number>`, Line #2: `ISSN/ISBN <number>` in crisp **Arial Bold**) sits top-right over the photograph.
  - Middle-left headline stack in **Arial Black** inside the dark block: the region level, the publication title in the accent colour, one word per line, and `DALAM ANGKA` beneath it.
  - **Position Lock**: bottom-left official agency logo paired with multi-line institutional branding (**Arial Bold Italic**).
  - Its own palette (**Report Navy & Orange**, `#082641` + `#f5871f`) matches the reference cover; picking the style selects that theme automatically, and any other theme still works.
- **Best For**: Annual statistical yearbooks, regional profiles in numbers, institutional reports, government publications.

#### 9. Business Report (`📊 Report`)
- **Visual Personality**: Bold, corporate, high-contrast business-to-business.
- **Design Elements**:
  - Crisp white canvas; an **arrow-shaped skyscraper photograph** (traced from `design-10.designcraft`) with a soft drop shadow — plus its **chevron sliver**, separated by a thin white gap yet carrying the same image, so the buildings continue across the gap.
  - An **orange band along the top edge** and an **orange corner** in the bottom-right, framing the composition.
  - A **navy plane crossing the lower left, tucked behind the photograph**, with a **white circle medallion** carrying the two-tone year (first half dark, second half accent) in giant **Arial Black** and the **Volume** line directly beneath.
  - Headline stack top-left in **Arial Black**: the region level (navy), the publication title in the accent colour on one full-width line, and `DALAM ANGKA` (navy).
  - **Position Lock**: publication metadata (Line #1: `Katalog BPS: <number>`, Line #2: `ISSN/ISBN <number>` in crisp **Arial Bold**, white) sits top-right, over the orange band.
  - **Position Lock**: official agency logo paired with multi-line institutional branding (**Arial Bold Italic**, white) on the navy plane, bottom-left.
  - Its own palette (**Report Navy & Orange**, `#082641` + `#f5871f`); picking the style selects that theme automatically, and any other theme still works.
- **Best For**: Business reports, corporate annual reviews, investment and industry publications, city profiles.

#### 10. Urban Angle (`🌆 Urban`)
- **Visual Personality**: Angular, urban and editorial — a diagonal grid of flat colour planes.
- **Design Elements**:
  - An **orange corner wedge** filling the top-left and cutting diagonally down to the left edge.
  - A **navy diagonal band** running from the top edge into the left margin, carrying the year.
  - A **cityscape photograph** filling the right-hand panels, clipped by the band and the bottom wedge.
  - A **navy bottom wedge** rising from the lower-right corner.
  - The **year in giant Arial Black**, split into two tones (white + orange) and **rotated** to run along the band, with the **Volume** line beside it.
  - Headline stack on the white negative-space panel: region level, publication title in the accent colour (one word per line), and `DALAM ANGKA`.
  - **Position Lock**: publication metadata (Line #1: `Katalog BPS: <number>`, Line #2: `ISSN/ISBN <number>`) sits top-right over the photograph.
  - **Position Lock**: top-left official agency logo paired with multi-line institutional branding (**Arial Bold Italic**) over the orange wedge.
  - Its own palette (deep petrol navy + marigold, `#034460` + `#fcb91e`); picking the style selects that theme automatically, and any other theme still works.
- **Best For**: City and regional profiles, urban development reports, modern corporate covers.

---

### Hover Thumbnail Previews
- **Quick Preview Pills**: Under the style dropdown, you will see 10 quick-switch pills (`🌊 Wave`, `🔷 Geometric`, `📐 Origami`, `🔰 Chevron`, `⬡ Hexagon`, `🏢 Corporate`, `↗ Arrow`, `🌇 Horizon`, `📊 Report`, `🌆 Urban`).
- **Hover to Preview**: Simply move your mouse over any pill (or click the small eye icon `👁` next to the dropdown). A floating tooltip instantly displays the actual rendered cover thumbnail so you can choose your favorite look before generating.

---

### Theme Colors & Color Customization
You can choose from pre-configured institutional themes or type your own hex color codes:
- **Primary Color** (Default: `#0072bc` / Corporate Sapphire): Used for primary banners, title blocks, and major geometric elements.
- **Accent Color** (Default: `#f5ab19` / Amber Gold): Used for highlight rules, kicker labels, medallion rings, and year underlines.
- **Dark Color** (Default: `#111827` / Midnight Navy): Used for dark canvas backgrounds, contrasting subtitle text, and deep frames.

---

### Metadata & Content Fields
Fill in your publication details:
- **Title**: The cover headline (e.g., `KARAWANG TIMUR`), typed in a multi-row box and placed on the page as written. Pubgen formats it in **Arial Black**.
- **Title 2**: The second headline line (e.g., `DALAM ANGKA`). It sits in the cover design's own frame, below the headline and in its own style — smaller and in another colour. The field starts with the dummy text `Title 2`.
- **Year**: The publication year (e.g., `2026`). Rendered in giant **Arial Black** numerals.
- **Volume**: The edition text (e.g., `Volume 23 2026`).
- **Catalog BPS**: Enter the official catalog number (e.g., `1102001.3215113`). Pubgen automatically prepends `Katalog BPS:` if not already typed.
- **Identifier**: Select **ISSN** or **ISBN** and type the number (e.g., `2828-7436`).
- **Organization Text**: Enter your agency name (e.g., `BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG`). Rendered in **Arial Bold Italic**. Use `\n` to break onto a second line.

---

### Custom Image & Logo Pickers
Pubgen dynamically displays the image pickers required for the selected style:
- **Skyline Image**: Used in *Wave Curves* and *Chevron & Medallion*. Click **Browse...** to pick any high-resolution skyline or landmark image (`.jpg`, `.png`, `.webp`).
- **Landscape Image**: Used in *Folded Origami* and *Geometric Medallion*.
- **Honeycomb Image**: Used in *Honeycomb Hexagon*. Automatically clips your photo into the interlocking 12-sided honeycomb frame.
- **Photo**: Used in *Corporate Headline*, *Diagonal Arrow* and *Modern Horizon*. Vertical architectural, city skyline, or landmark photograph. In *Diagonal Arrow* it fills the full-height panel on the right, behind the diagonal banner and the arrow. In *Modern Horizon* it is clipped by the sweeping arc across the upper page, so choose a wide cityscape with interest along the horizon.
- **Logo**: Click **Browse...** to choose your institution's logo file (e.g., BPS emblem, city coat of arms). It automatically fits proportionally into the branding frame without distortion.

---

### Generating the Cover
Once your fields are set, click the prominent blue button:
👉 **[ Generate Cover Page ]**

In less than 100 milliseconds, Pubgen will:
1. Insert a new page at the very beginning of the document.
2. Build all vector paths, polygons, banners, and shadows.
3. Place and clip the photos and logo.
4. Render all text frames with exact typography hierarchy (**Arial Black**, **Arial Bold**, **Arial Bold Italic**).
5. Navigate your screen to the newly created cover page.

---

## 4. Card 2: Page from CSV / Excel (Batch Page & Chapter Synthesizer)

If you have a table of contents or catalog of tables in Excel or CSV, this tool will generate dozens or hundreds of formatted publication pages automatically.

```
+-------------------------------------------------------------------+
| Page from CSV / Excel                                             |
| File: [ samples/KBDA2026_chapters.csv               ] [Browse...] |
| Delimiter: Semicolon (;) | Columns: 3 | Records: 28               |
|                                                                   |
| Column Mapping:                                                   |
| Type col:   [ 0: type   ▼ ]                                       |
| Number col: [ 1: number ▼ ]                                       |
| Title col:  [ 2: title  ▼ ]                                       |
|                                                                   |
| Preset:     [ 1 Row · 3 Columns                         ▼ ]       |
| Scope:      (•) All records    ( ) Range: [ 1 ] to [ 10 ]         |
| Placement:  [x] Append to document                                |
|             [x] Auto-insert table from 'file' col                 |
|                                                                   |
| [  Generate Pages  ]                                              |
+-------------------------------------------------------------------+
```

### How It Works
Instead of manually creating a new page, adding text boxes, and typing "Tabel 1.1.1 Jumlah Penduduk Menurut Desa...", Pubgen reads your structured list and generates the pages for you.

### Input Files (CSV & Excel)
- Click **Browse...** to select your `.csv`, `.tsv`, or `.xlsx` (Excel) file.
- **Automatic Delimiter Detection**: Works seamlessly whether your CSV uses commas (`,`), semicolons (`;`, standard in European/Indonesian Excel), or tabs.

### Column Mapping (Type, Number, Title)
Pubgen looks for three key pieces of information in each row:
1. **Type**: Category or section level (e.g., `Tabel`, `BAB`, `Grafik`, `Subbab`).
2. **Number**: The code or index (e.g., `1.1.1`, `BAB II`, `Grafik 3.2`).
3. **Title**: The descriptive name (e.g., `Luas Wilayah Menurut Desa/Kelurahan`).

If your file has headers like `type`, `number`, `title` (or `kategori`, `nomor`, `judul`), Pubgen selects them automatically. You can also adjust the dropdowns manually.

### Layout Presets
Choose how the top header of each generated page should look:
- **1 Row · 3 Columns (Recommended for BPS Publications)**:
  - Formats the page header into a clean single line across the top:
    `Tabel` (left) | `1.1.1` (number) | `Luas Wilayah Menurut Desa...` (title).
  - Automatically calculates column widths based on text length so there is no awkward spacing.
- **Editorial Chapter (Stacked)**:
  - Large bold kicker, giant numeral, and full-width headline. Perfect for section divider pages (e.g. *BAB 1 GEOGRAFI*).
- **Technical Spec Card**:
  - Two-column technical layout with a top divider rule.
- **Minimal Zine**:
  - Centered typography with generous white space.

### Record Scope & Placement
- **All records**: Generates a page for every row in your data.
- **Range (e.g. 1-10)**: Generates pages only for specific rows.
- **Append to document**: Adds new pages to the end of the document. Uncheck to insert immediately after the active page.

### Auto-Inserting Tables from 'file' Column
If your catalog CSV contains a 4th column called `file` pointing to table CSV files (e.g., `01_01.csv`, `01_02.csv`), checking:
`[x] Auto-insert table from 'file' col`
will **both create the page AND automatically place the corresponding styled table** directly under the page title!

### Publication Note Pages (`Note`)
Catalog rows typed `Note` get a dedicated layout instead of the preset header: a centred, fully transparent
Arial title box (`Keterangan Publikasi`, or the row's own title when present) and the RTF file from the `file`
column placed below it. The Excel → CSV Data converter emits this row automatically for the workbook sheet named
exactly `A02` (row 1: `title;Keterangan Publikasi`, row 2: `file;notes/A02.rtf`), so that sheet produces no
table CSV — only the catalog entry. Other `A##` sheet names stay ordinary sheets.

---

## 5. Card 3: Table from CSV / Excel (Statistical Table Builder)

Statistical publications require immaculate tables with clear lines, proper alignment, and total row formatting. Pubgen generates native, editable DesignCraft tables directly from your data.

```
+-------------------------------------------------------------------+
| Table from CSV / Excel                                            |
| File: [ samples/sample_3stack_table.csv             ] [Browse...] |
| Detected: 12 rows × 6 columns | Semicolon                         |
|                                                                   |
| Options:                                                          |
| [x] First row is header                                           |
| [x] Add column numbers (1), (2), (3)...                           |
| [x] Table footer (last row)                                       |
| [x] Condition rules (Jumlah, Tahun, Kab, Kec...)                  |
|                                                                   |
| Style:     [ Horizontal Rules (Booktabs)                ▼ ]       |
| Placement: [ At page (below title)                      ▼ ]       |
| Target:    Page 2 (below title)                                   |
|                                                                   |
| [  Insert Table  ]                                                |
+-------------------------------------------------------------------+
```

### How It Works
Select a CSV or Excel file, configure your styling options, and click **Insert Table**. A native, vector table is placed and formatted instantly.

---

### Header Options & Multi-Level Stacked Headers

#### First row is header
Check this when row 1 contains your column names. It applies bold typography, header background styling, and locks the header row.

#### Multi-Level Stacked Headers (`Group--Sub--Detail`)
In complex statistical tables, columns are often grouped under common parents (for example: *Laki-Laki* and *Perempuan* grouped under *Jenis Kelamin*).
- In your CSV or Excel file, simply name columns using double-dashes (`--`):
  `Jenis Kelamin--Laki-Laki`
  `Jenis Kelamin--Perempuan`
  `Total`
- Pubgen automatically detects the double-dashes and creates a **multi-level stacked header** (up to 3 levels):
  - Common ancestor columns (*Jenis Kelamin*) automatically **merge horizontally** across their children.
  - Standalone columns (*Total*) automatically **merge vertically downward** so the table header is aligned.

---

### Column Numbering Row: (1), (2), (3)...
Official statistical standards (such as BPS Indonesia) require a column numbering row immediately beneath the header titles.
- Check **`[x] Add column numbers (1), (2)...`**.
- Pubgen inserts an extra row with `(1)`, `(2)`, `(3)`... centered across all columns, separated by a thin 0.5pt rule.

---

### Table Footer & Auto-Detected Footnotes / Sources

#### Table footer (last row)
Check this if your last row is a total or summary row. It formats the row with a 0.5pt top separator rule and a 1.0pt bottom boundary rule.

#### Auto-Detected Footnotes / Sources
If the last row of your CSV contains a footnote or source citation (e.g., `Sumber: Dinas Kependudukan dan Pencatatan Sipil`):
- Pubgen automatically detects that only column 1 has text.
- It excludes the footnote from the table grid so column widths are not distorted.
- It automatically creates a separate text frame positioned directly below the table with the citation text formatted in italic.

---

### Condition Rules: Smart Statistical Highlighting
When working with official data, summary rows need to stand out.
- Check **`[x] Condition rules (Jumlah, Tahun, Kab, Kec...)`**.
- Pubgen automatically scans the first column of every data row.
- If a row starts with keywords like `Jumlah`, `Total`, `Tahun`, `Kabupaten`, or `Kecamatan`:
  - It automatically adds top and bottom 0.5pt rules.
  - It renders the text in bold.
  - Intermediate subtotals and final totals are formatted.

---

### Table Styles
- **Horizontal Rules (Booktabs - Recommended)**:
  The international gold standard for academic and government publishing:
  - 1.0pt thick top border.
  - 0.5pt thin header separator.
  - 0.5pt column numbers separator.
  - No vertical grid lines (clean, readable data flow).
  - 1.0pt thick bottom border.
- **Corporate Navy**:
  - Dark blue header row with white text.
  - Alternating light grey zebra-striping on data rows for easy scanning.
- **Basic Grid**:
  - Full boxed grid lines around every cell.

---

### Placement Modes
- **At page (Recommended)**: Automatically targets the page currently selected in your **Pages** tab, finds the title frame at the top, and places the table directly below the title with a 20pt buffer.
- **New frame**: Centers the table on the page starting from the top margin.
- **At cursor**: Places the table directly inside an active text cursor.

---

### Multi-Sheet Excel Workbooks & ZIP Export
If you open a multi-sheet Excel file (`.xlsx`):
- Pubgen automatically extracts all sheets.
- A **Sheet table** dropdown appears: switch between sheets (`Tabel 1.1`, `Tabel 1.2`, etc.) with one click.
- **Download CSVs as ZIP...**: Click this button at any time to export all extracted sheets and catalog files as a neat `.zip` archive on your computer.

---

## 6. Card 4: Quick Actions & Editorial Utilities

The **Quick Actions** section contains essential shortcuts for publication assembly.

```
+-------------------------------------------------------------------+
| Quick Actions                                                     |
| Mode: Double-sided · Selected: Page 1                             |
|                                                                   |
| [ Single-sided / Double-sided Toggle ]                            |
| [ Add Blank Page ]                                                |
|                                                                   |
| Table of Contents:                                                |
| Title:      [ DAFTAR TABLE                     ]                  |
| TOC style:  [ TOC Table ▼ ]                                       |
| TOC layout: [ TablePage (BPS Catalog) ▼ ]                         |
| [x] Indent wrapped TOC entries (hanging indent)                   |
| [ ] Include TOC page in TOC entries                               |
| [  Add TOC  ]   [  Rebuild TOC  ]                                 |
|                                                                   |
| Page from RTF:                                                    |
| Path: [ preface.rtf                         ] [Browse...]         |
| [  Insert RTF Story  ]                                            |
|                                                                   |
| Page from Image:                                                  |
| Path: [ pembatas_bab.jpg                    ] [Browse...]         |
| Mode: (•) Page margins (Object)   ( ) Page background             |
| [  Insert Image Page  ]                                           |
|                                                                   |
| Page Renumbering:                                                 |
| Style: [ Roman (i, ii, iii)  ▼ ]  Start #: [ 1 ]                  |
| [  Restart page #  ]   [  Continue numbering  ]                   |
+-------------------------------------------------------------------+
```

---

### Single-Sided vs. Double-Sided Spreads Toggle
- **Double-sided (Facing Pages)**: Essential for printed books and magazines. Left (verso) and right (recto) pages face each other with mirrored margins (inside binding gutter vs. outside margin).
- **Single-sided**: Used for digital distribution, PDF reports read on tablets or laptops.
- Click the button to toggle between modes non-destructively at any time.

---

### Add Blank Page
In formal publishing, major chapters or section dividers should always start on a right-hand page (odd-numbered page, recto).
- Click **Add Blank Page** to insert a single blank page directly after your selected page to balance the spread.

---

### Automated Table of Contents (Add TOC / Rebuild TOC)

Pubgen scans your document's text frames and builds a Table of Contents with page leader dots automatically.

#### TOC Style (what the list contains)
| TOC style | Lists | Title filled in when picked |
|---|---|---|
| **TOC Entry** | every page whose title carries the tag style (the original behaviour) | *left as typed* |
| **TOC Table** | only pages whose **first text box** starts with `Tabel` or `Table` (case-insensitive) | `Daftar Table` |
| **TOC Chart** | only pages whose **first text box** starts with `Grafik` or `Chart` (case-insensitive) | `Daftar Grafik` |

Picking **TOC Table** or **TOC Chart** fills the Title field for you and centres the title line; you can still edit
the title. The filter is stored in the document, so **Rebuild TOC** and *Layout ▸ Update Table of Contents* keep it.

A document can hold several lists at once — a *Daftar Table* and a *Daftar Grafik* side by side, for example.
**Add TOC** makes a list on the page in view, replacing the one already there (one page holds one list).

#### Layout Modes:
1. **Standard Layout**:
   - Classic publishing format:
     `Tabel 1.1 Luas Wilayah .................................... 12`
2. **TablePage Layout (BPS Catalog Standard)**:
   - Formatted specifically for official statistical catalogs:
     - Left column header: `Tabel`
     - Right column header: `Halaman`
     - Clean number column: `1.1`
     - Title column with deep hanging indent so wrapped lines stay aligned.
     - Bold section headers (`1. Geografi`) without page numbers.
     - **Automatic continuation headers**: If the list spans multiple pages, `Tabel` and `Halaman` are automatically repeated at the top of continuation pages!

#### Rebuild TOC
If you add, delete, or rearrange pages later, click **Rebuild TOC**. Pubgen refreshes the list **on the page in
view** with its own TOC style, title and layout, so the other lists in the document are left alone. Renumbering a
page restarts every list in the document, so all page numbers stay right.

---

### Page from RTF (Rich Text Import & Autoflow)
Quickly import a Word or text processor document (e.g. *Kata Pengantar* / Preface or Executive Summary):
1. Save your document as Rich Text Format (`.rtf`).
2. Click **Browse...** to select the `.rtf` file.
3. Click **Insert RTF Story**.
4. Pubgen places the formatted text, automatically flows it across page margins, and **creates new pages with linked text boxes** until all text fits. Bold, italic, font sizes, hanging indents, and bullet points are preserved.

---

### Page from Image (Object vs. Full-Bleed Background)
Quickly insert photos or full-page chapter dividers (*Pembatas BAB*):
- **Page margins (Object)**: Fits the photo neatly within the page margins.
- **Page background**: Scales the image to fill the entire page bounds, centers it, and moves it to the back layer (`to: "back"`). Perfect for full-bleed chapter intro art.

---

### Page Renumbering & Sections (Roman vs. Arabic)
Books typically have two numbering sections:
1. **Front Matter** (Cover, Preface, Table of Contents): Roman numerals (`i, ii, iii...`).
2. **Main Body** (Chapter 1 onwards): Arabic numerals (`1, 2, 3...`).

#### How to use:
1. Navigate to your Preface or Table of Contents page.
2. Under **Page Renumbering**, select **Style: Roman (i, ii, iii...)**, Start #: `1`. Click **Restart page #**.
3. Navigate to Page 1 of your main chapters.
4. Select **Style: Arabic (1, 2, 3...)**, Start #: `1`. Click **Restart page #**.
5. Click **Rebuild TOC** in the Table of Contents card: all page references automatically update to show Roman numerals for front matter and Arabic numbers for body pages!

---

## 7. Step-by-Step Practical Tutorials

### Tutorial 1: Creating a Complete Publication in 3 Minutes

Follow this workflow to create a complete statistical booklet from zero:

1. **New Document**:
   - Press `Ctrl+N` (File > New Document). Choose `A4`, Facing Pages checked.
2. **Generate Cover Page**:
   - Open the **Pubgen** panel.
   - Expand **Publication Cover**.
   - Choose style **`⬡ Hexagon`** (or hover over the pills to preview).
   - Enter your Title: `KARAWANG TIMUR` and Year: `2026`.
   - Click **Generate Cover Page**. Your cover is created on Page 1!
3. **Generate Content Pages**:
   - Expand **Page from CSV**.
   - Click **Browse...** and select `samples/KBDA2026_chapters.csv`.
   - Preset: `1 Row · 3 Columns`.
   - Click **Generate Pages**. 28 formatted pages are appended to your document!
4. **Insert Statistical Tables**:
   - Go to Page 2 in the document.
   - Expand **Table from CSV**.
   - Click **Browse...** and select `samples/kbda_2005.csv`.
   - Style: `Horizontal Rules (Booktabs)`. Placement: `At page (below title)`.
   - Check `[x] Condition rules (Jumlah, Tahun, Kab, Kec...)`.
   - Click **Insert Table**. The table appears directly beneath the page header!
5. **Add Table of Contents**:
   - In **Quick Actions**, click **Add Blank Page**.
   - Under Table of Contents, set Title: `DAFTAR TABEL`, Layout: `TablePage`.
   - Click **Add TOC**. Your Table of Contents is generated with leader dots!
6. **Export**:
   - Go to `File` > `Export` > `PDF (Print)`. Your publication is ready for print or digital distribution!

---

### Tutorial 2: Designing a Custom Cover with Your Own Photo & Colors

Want a corporate identity with your company or municipal colors?

1. In **Publication Cover**:
   - Select Style: **`❖ Medallion`** or **`⬡ Hexagon`**.
2. **Change Colors**:
   - Click the color box or type:
     - Primary: `#1e3a8a` (Deep Royal Blue)
     - Accent: `#10b981` (Emerald Green)
     - Dark: `#0f172a` (Slate Dark)
3. **Choose Your Photo**:
   - Under **Cover Images**, click **Browse...** next to *Landscape* or *Honeycomb*.
   - Pick a high-resolution photo from your computer (`.jpg` or `.png`).
4. **Choose Your Logo**:
   - Click **Browse...** next to *Logo* and pick your organization's logo with a transparent background.
   - Type your organization's name in *Logo Text*.
5. Click **Generate Cover Page**. Pubgen creates the cover with your exact colors, custom logo, and photo!

---

### Tutorial 3: Formatting Complex Statistical Tables from Excel

When working with tables that have grouped columns (e.g. Population by Age and Gender):

1. In Excel, prepare your column headers with double dashes for grouped columns:
   | Kecamatan | Usia 0-14--Laki-Laki | Usia 0-14--Perempuan | Usia 15-64--Laki-Laki | Usia 15-64--Perempuan | Total |
2. Save the sheet as a `.csv` or `.xlsx`.
3. In Pubgen, under **Table from CSV / Excel**:
   - Click **Browse...** and select your file.
   - Check `[x] First row is header`.
   - Check `[x] Add column numbers (1), (2)...`.
   - Check `[x] Table footer (last row)` (for the grand total).
   - Check `[x] Condition rules (Jumlah, Tahun, Kab, Kec...)`.
   - Style: `Horizontal Rules (Booktabs)`.
4. Click **Insert Table**.
5. Result: A multi-level stacked table with merged group headers, BPS column numbers `(1), (2), (3)...`, and styled total lines!

---

## 8. Frequently Asked Questions & Troubleshooting

#### Q: My CSV file has semicolons instead of commas. Will Pubgen recognize it?
**A:** Yes! Pubgen features an intelligent delimiter sniffer that automatically identifies semicolons (`;`), commas (`,`), and tabs (`\t`). You do not need to convert your file.

#### Q: Why is my Table of Contents showing `#` instead of page numbers?
**A:** If page numbers show `#`, it means the text frame was placed on a master/parent page rather than a document page. Make sure you are generating the TOC on a normal document page.

#### Q: Can I edit the text and tables after Pubgen generates them?
**A:** Absolutely! Everything Pubgen creates—text frames, character runs, table cells, and vector paths—are **100% native DesignCraft objects**. You can double-click any text to edit it, adjust table column widths with the table tool, or change swatch colors at any time.

#### Q: Where are my settings saved in the Portable Demo?
**A:** The portable demo includes a file named `portable.txt`. This instructs DesignCraft to store all your settings, crash recovery files, and preferences inside the local `DesignCraftData/` folder right next to `designcraft.exe`. Nothing is written to your Windows registry or `%APPDATA%`, keeping your computer completely clean.

#### Q: How do I export my publication to high-quality PDF?
**A:** Go to the main menu: `File` > `Export` > `PDF (Print)`. You can choose PDF/X standards, embed ICC color profiles, and set 300 DPI image quality for commercial offset printing.

---

*DesignCraft Publication Generator (Pubgen) — Engineered for Speed, Elegance, and Precision.*
