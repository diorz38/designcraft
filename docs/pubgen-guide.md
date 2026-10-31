# Pubgen: The Publication Generator — Comprehensive User Guide

> **Welcome to Pubgen!** This guide is written for publication editors, layout designers, statisticians, and institutional authors. It explains every single feature, setting, and button in the **Pubgen** menus in clear, non-technical language.

---

## Table of Contents
1. [What is Pubgen?](#1-what-is-pubgen)
2. [Finding Pubgen in DesignCraft](#2-finding-pubgen-in-designcraft)
   - [The PubGen panel](#the-pubgen-panel)
   - [Switching back to DesignCraft's menus](#switching-back-to-designcrafts-menus)
   - [Bilingual Publication Toggle (Single Lang / Dual Lang)](#bilingual-publication-toggle-single-lang--dual-lang)
3. [Publication Cover (Cover Page Generator)](#3-publication-cover-cover-page-generator)
   - [Overview & Position](#overview--position)
   - [The 13 Executive Cover Styles](#the-13-executive-cover-styles)
   - [Hover Thumbnail Previews](#hover-thumbnail-previews)
   - [Theme Colors & Color Customization](#theme-colors--color-customization)
   - [Metadata & Content Fields](#metadata--content-fields)
   - [Custom Image & Logo Pickers](#custom-image--logo-pickers)
   - [Generating the Cover](#generating-the-cover)
4. [Page from CSV / Excel (Batch Page & Chapter Synthesizer)](#4-page-from-csv--excel-batch-page--chapter-synthesizer)
   - [How It Works](#how-it-works)
   - [Input Files (CSV & Excel)](#input-files-csv--excel)
   - [Column Mapping (Type, Number, Title)](#column-mapping-type-number-title)
   - [Layout Presets](#layout-presets)
   - [Record Scope & Placement](#record-scope--placement)
   - [Auto-Inserting Tables from 'file' Column](#auto-inserting-tables-from-file-column)
5. [Table from CSV / Excel (Statistical Table Builder)](#5-table-from-csv--excel-statistical-table-builder)
   - [How It Works](#how-it-works-1)
   - [Header Options & Multi-Level Stacked Headers](#header-options--multi-level-stacked-headers)
   - [Column Numbering Row: (1), (2), (3)...](#column-numbering-row-1-2-3)
   - [Table Footer & Auto-Detected Footnotes / Sources](#table-footer--auto-detected-footnotes--sources)
   - [Condition Rules: Smart Statistical Highlighting](#condition-rules-smart-statistical-highlighting)
   - [Table Styles (Horizontal Rules, Grid, Striped)](#table-styles)
   - [Placement Modes (At Page, New Frame, At Cursor)](#placement-modes)
   - [Multi-Sheet Excel Workbooks & ZIP Export](#multi-sheet-excel-workbooks--zip-export)
6. [Page & Layout Utilities & Editorial Tools](#6-page--layout-utilities--editorial-tools)
   - [Single-Sided vs. Double-Sided Spreads Toggle](#single-sided-vs-double-sided-spreads-toggle)
   - [Add Blank Page](#add-blank-page)
   - [Automated Table of Contents (Add TOC / Rebuild TOC)](#automated-table-of-contents-add-toc--rebuild-toc)
   - [Page from RTF (Rich Text Import & Autoflow)](#page-from-rtf-rich-text-import--autoflow)
   - [Page from Image (Object vs. Full-Bleed Background)](#page-from-image-object-vs-full-bleed-background)
   - [Page Renumbering & Sections (Roman vs. Arabic)](#page-renumbering--sections-roman-vs-arabic)
7. [Master Excel: One Workbook to Finished Pages](#7-master-excel-one-workbook-to-finished-pages)
   - [What each sheet becomes](#what-each-sheet-becomes)
8. [Vector Charts & Statistical Infographics](#8-vector-charts--statistical-infographics)
   - [The nine archetypes](#the-nine-archetypes)
   - [What the chart dialog asks for](#what-the-chart-dialog-asks-for)
   - [Charts from Excel](#charts-from-excel)
9. [Header & Footer Setup](#9-header--footer-setup)
10. [Publication Compiler (Multi-Chapter Binder)](#10-publication-compiler-multi-chapter-binder)
    - [The manifest](#the-manifest)
    - [What a compile does, in order](#what-a-compile-does-in-order)
11. [Audit Publication (Preflight QA)](#11-audit-publication-preflight-qa)
12. [Publication Profiles](#12-publication-profiles)
13. [AI Translation & Terminology Memory](#13-ai-translation--terminology-memory)
    - [Setup](#setup)
    - [Translating pages](#translating-pages)
    - [Terminology memory](#terminology-memory)
14. [Step-by-Step Practical Tutorials](#14-step-by-step-practical-tutorials)
    - [Tutorial 1: Creating a Complete Publication in 3 Minutes](#tutorial-1-creating-a-complete-publication-in-3-minutes)
    - [Tutorial 2: Designing a Custom Cover with Your Own Photo & Colors](#tutorial-2-designing-a-custom-cover-with-your-own-photo--colors)
    - [Tutorial 3: Formatting Complex Statistical Tables from Excel](#tutorial-3-formatting-complex-statistical-tables-from-excel)
    - [Tutorial 4: Compiling a Whole Publication from One Manifest](#tutorial-4-compiling-a-whole-publication-from-one-manifest)
15. [Frequently Asked Questions & Troubleshooting](#15-frequently-asked-questions--troubleshooting)
## 1. What is Pubgen?

**Pubgen** (Publication Generator) is an automated layout engine built directly into DesignCraft.

In conventional page layout programs (like Adobe InDesign or QuarkXPress), assembling a 300-page statistical yearbook or government report (such as *Kecamatan Dalam Angka* or an Annual Economic Census) requires hundreds of manual operations: drawing text frames, styling headers, formatting tables, placing logos, calculating TOC leader dots, and formatting covers.

Pubgen turns this into a fast, automated workflow:
- **Instant Covers**: Generates executive corporate covers with professional geometry, drop shadows, and clipped photographs — front, inside and back.
- **Instant Pages**: Reads your table of contents or chapter list from Excel/CSV and creates formatted pages in seconds.
- **Publication Tables**: Formats statistical tables with BPS-standard booktabs horizontal rules, stacked column headers, column numbers, and conditional highlighting for totals.
- **Vector Charts**: Turns `CXX_XX` sheets (or an existing table) into native, editable charts — nine archetypes, from bars and lines to population pyramids, gauges and GeoJSON maps.
- **One-Button Book**: A Master Excel workbook builds pages, tables, charts and cover in one action, and the Publication Compiler weaves many chapter workbooks into a single document from one manifest.
- **Quality Audit**: A statistical preflight checks table-cell overset, missing sources, numbering gaps, bilingual parity, image DPI and — for print — spine and bleed, with click-to-jump rows.
- **House Profiles & Terminology**: Publication profiles switch sheet routing, naming, geometry and table styling; an editable glossary keeps AI translations on official statistical wording.
- **Live Editorial Tools**: Handles double-sided facing pages, Roman/Arabic page renumbering, headers and footers, RTF text autoflow, and Table of Contents generation.

---

## 2. Finding Pubgen in DesignCraft

Pubgen lives in the **menu bar**. With Pubgen on — the default — the bar keeps DesignCraft's whole host tree and adds **Publication**, **Publication Tools** and **Shortcut** before **Help**:

```
+-----------------------------------------------------------------------------------------------------------+
|  File  Edit  Layout  Type  Object  Table  View  Window  Publication  Publication Tools  Shortcut  Help |
+-----------------------------------------------------------------------------------------------------------+
```

| Menu | What it holds |
|---|---|
| **File … Window** | DesignCraft's own menus, unchanged. |
| **Publication** | The generation pipeline: Generate from Excel (`Ctrl+Shift+G`), Compile Publication (`Ctrl+Shift+B`), Publication Profile, Vector Chart (`Ctrl+Shift+V`), Table of Contents (`Ctrl+Alt+T`), Publication Cover, Header & Footer Setup, Audit Publication (`Ctrl+Shift+Alt+A`). |
| **Publication Tools** | **Manual Ingest & Tables** (Page from CSV…, Table from CSV…), **Publication Profiles** (pick a profile, Customize Active Profile…, Apply Profile Margins), **Page & Layout Utilities** (Standard BPS Margins `Ctrl+Shift+M`, Page Margins & Renumbering Options…), the **Dual Language Mode** checkmark (`Ctrl+Shift+L`), and **AI Translate** (Translate Selected Page, Translate Table Pages…, Fix Terms with Glossary, Setup…). |
| **Shortcut** | Quick access to original DesignCraft actions: Export PDF…, Preflight Document, Package…, Place…, New Sample Document. |
| **Help** | DesignCraft's own Help rows, then **About PubGen…** and the menu-bar switch. |

So a complete publication is three menu visits away:

1. **Publication ▸ Generate from Excel** — read your Excel/CSV manuscript and build the pages.
2. **Publication ▸ Publication Cover** — synthesize the cover.
3. **Publication ▸ Table of Contents** — insert or rebuild the ToC.

Large works go through **Publication ▸ Compile Publication** instead (§10), and the statistical work through **Vector Chart** (§8), **Header & Footer Setup** (§9), **Publication Profile** (§12) and **Audit Publication** (§11).

The remaining editorial actions (single-sided spreads, blank pages, RTF and image pages, page renumbering) open from **Publication Tools ▸ Page & Layout Utilities…**, and the manual routes from **Publication Tools ▸ Manual Ingest & Tables**.

### The PubGen panel

Everything above is also a panel, opened from **Window ▸ PubGen Panel**. The panel is the working surface; the menus are the shortcuts.

| Area | What it does |
|---|---|
| **Header band** | Profile selector with a one-line summary of the active profile, **AI Translator**, **Translate this Page** (or *Translate Review / Translate Table on this Page*), **Fix Terms**, **Retranslate (Glossary)**, **Translate Tables…**, **Audit Publication**, **Set Page Margin** and the Single/Dual Lang badge. |
| **Publication Compiler** | One manifest → the whole publication (§10), desktop only. |
| **1. Data Source & Generate (Master Excel)** | The one-button workbook workflow (§7). |
| **2. Table of Contents** | TOC style, title, layout and **Add TOC / Rebuild TOC** (§6). |
| **3. Manual Import & Fine-Tuning** | A collapsible group holding **Page from CSV** (§4) and **Table from CSV** (§5) for hand-driven work. |
| **4. Page Utilities** | Spreads toggle, blank page, page renumbering and sections (§6). |

Dialogs open on top: Master Excel, Publication Cover, Table of Contents, Header & Footer, Vector Chart, Audit Publication, the profile customizer, the AI setup dialog and About PubGen.

### Switching back to DesignCraft's menus
Pubgen's actions also exist in DesignCraft's full menu set, under a **Pubgen** menu that sits just before **Help**. To reach the plain DesignCraft bar — without the three Pubgen menus — switch it:

- **Help ▸ Show DesignCraft Menus**, or
- the same button in **Help ▸ About PubGen…**.

The bar now shows DesignCraft's full menus with Pubgen appended before Help. Choose **Pubgen ▸ Show PubGen Menus** to switch back. The setting is per session: a fresh start opens in Pubgen mode.

### Bilingual Publication Toggle (`Single Lang` / `Dual Lang`)
The **Dual Language Mode** row in the **Publication Tools** menu ticks or unticks bilingual output (unticked = Single Lang, the default):
- **Single Lang**: Standard monolingual generation matching legacy publications.
- **Dual Lang**: Generates bilingual Indonesian-English content adhering to BPS statistical standards:
  - **Covers**: Binds the English title into the dedicated `#slot:text:title_en` frame where a cover provides one, and otherwise into the second headline line (`title#2`).
  - **Dividers (`DIV`)**: Emits dual-language chapter headings, with English subtitles rendered in the accent swatch.
  - **Review Chapters (`RXX`) & Front Matter (`BXX`)**: Synthesizes sequential spreads (Indonesian spread followed by English spread).
  - **Statistical Tables (`XX_XX`)**: Produces 2-row header chrome (`Tabel` / italic `Table`) and 2-line titles split on `" / "` with italicized English.
  - **Charts (`CXX_XX`)**: Emits bilingual title structures (Indonesian line, then `Figure NN.NN …` in italic) with correct English typography.

### AI, in one paragraph
AI is optional and needs the online build (`cargo run --release -p designcraft --features pubgen,ai-translate-online`). It translates pages and table cells through **Groq**, fills covers through **Cloudflare Workers AI** (§3), and keeps wording consistent through an editable terminology memory. The setup, the page and batch translation buttons, the glossary editor and the limits are all in §13.

---

## 3. Publication Cover (Cover Page Generator)

The **Publication Cover** section allows you to synthesize a complete, professional publication cover with a single click. Open it from **Publication ▸ Publication Cover**, or use the same fields as part of the Master Excel and compiler flows (§7, §10).

Beyond the front cover, **Add Inside cover and Back Cover** inserts a greyscale inside cover on page 2 and a back cover at the end. **Map Overlay** places a decorative administrative map behind the cover text and logo, from the bundled stock maps or your own `.geojson`.

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

### The 13 Executive Cover Styles

Pubgen provides 13 corporate design styles:

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

#### 11. Tri-Color Bands (`🎨 Tri-Color`)
- **Visual Personality**: Executive, ordered, institutional — a flat colour system instead of artwork.
- **Design Elements**:
  - A **tri-colour masthead rule** across the top edge in blue (`#0088D2`), green (`#74BF45`) and orange (`#F38B1E`), echoed by the stripes above the institutional band.
  - A **right-hand cityscape panel** bleeding off the top-right corner, underlined by a blue/green/orange bar.
  - A blue accent bar above the headline; the headline in **Arial Black** ink, the second headline line in orange.
  - A **two-tone year** (blue + ink) in giant **Arial Black**, with the **Volume** line beneath it and a green rule.
  - **Position Lock**: publication metadata (Line #1: `Katalog BPS: <number>`, Line #2: `ISSN/ISBN <number>` in crisp **Arial Bold**) sits top-right on the white masthead.
  - **Position Lock**: a solid blue institutional band at the foot, carrying the logo on a white plate and the agency line in **Arial Bold Italic** white.
- **Best For**: Government statistical publications, annual reports, corporate profiles, and any cover that must carry a strict institutional palette.

#### 12. Prism Diamonds (`💎 Prism`)
- **Visual Personality**: Jewel-like, modern, dramatic — a midnight cover built from one repeated form.
- **Design Elements**:
  - A **deep midnight field** (`#0A2233`) with a soft deep-teal diamond mass behind the photograph.
  - A **cascade of diamond shapes** in eleven sizes, from a 12 pt fleck to the 300 pt photograph itself: solid blue, green and orange, pale tints, and two hairline outlines, stepping from the upper left down to the lower right and bleeding off the right edge.
  - The **cityscape photograph cut as the largest diamond**, its lower-left corner layered over by a pale blue diamond and a white fleck.
  - Headline in **Arial Black** ice-white, the second headline line in orange with wide tracking, and a tri-colour diamond mark in the top-left corner.
  - A **two-tone year** (blue + white) in giant **Arial Black** with the **Volume** line beneath it and a green rule.
  - **Position Lock**: publication metadata (Line #1: `Katalog BPS: <number>`, Line #2: `ISSN/ISBN <number>` in crisp **Arial Bold**) sits top-right on the midnight field.
  - **Position Lock**: a green seam over a solid blue institutional band at the foot, carrying the logo on a white plate, the agency line in **Arial Bold Italic** white, and a closing trio of diamonds.
  - The template brings its own swatches (`Prism Night`, `Prism Deep`, `Prism Blue`, `Prism Green`, `Prism Orange`, `Prism Mint`, `Prism Ice`, `Prism Sky`), so the Theme dropdown does not recolour it.
- **Best For**: Flagship statistical yearbooks, annual reports, corporate profiles, and covers that need a striking, contemporary look.

#### 13. Quarter Stack (`▢ Quarter Stack`)
- **Visual Personality**: Warm, tactile, editorial — a deck of cards laid on a paper table.
- **Design Elements**:
  - A **warm paper field** (`#F6F1E7`) instead of white, with the metadata and a trio of small quarter-rounded chips in the top-left corner.
  - A **fanned stack of four cards** offset by 30 pt, each rounding exactly **one** corner (bottom-right, top-left, top-right, then the photograph card's bottom-left with a 110 pt radius) in blush `#E7C7B3`, sage `#93A98C`, clay `#C75B39` and the photograph, bleeding off the right edge.
  - A **cascading chip cluster** below the stack — teal `#2F6F6B`, marigold `#E8A33D`, blush and clay quarter-rounded chips of four different sizes — echoing the cards at a smaller scale.
  - The headline in **Arial Black** ink `#1F2A44`, the second headline line in tracked clay, and a **two-tone year** (clay + ink) in giant **Arial Black** with the **Volume** line beneath and a sage rule.
  - **Position Lock**: publication metadata (Line #1: `Katalog BPS: <number>`, Line #2: `ISSN/ISBN <number>` in crisp **Arial Bold**) sits top-left on the paper field.
  - **Position Lock**: a solid ink institutional band at the foot, carrying the logo on a paper plate rounded on one corner, the agency line in **Arial Bold Italic** paper, and three closing chips.
  - The template brings its own warm swatches (`Stack Paper`, `Stack Ink`, `Stack Clay`, `Stack Marigold`, `Stack Sage`, `Stack Blush`, `Stack Teal`), so the Theme dropdown does not recolour it.
- **Best For**: Yearbooks, institutional profiles, cultural and social reports, and covers that should feel printed rather than corporate.

---

### AI Base Cover Generation (Online / Styles ≥ 100)

Pubgen introduces **AI Base Covers**, blending generative text-to-image AI backgrounds with DesignCraft's crisp vector typography, institutional branding plates, and strict layout geometry.

```
+-------------------------------------------------------------------+
| Publication Cover                                                 |
| Style: [ ── AI Base (online) ──                                 ] |
|        [ Retro Waves           ▼ ]                                |
| Palette:[ Warm Terracotta ] [ Pastel Dream ] [ Mono Slate ] [ Forest Gold ]|
|                                                                   |
| Title:  [ STATISTIK DAERAH                                  ]      |
| Title 2:[ KABUPATEN KARAWANG                                ]      |
| Year:   [ 2026        ]  Volume: [ Volume 23 2026          ]      |
|                                                                   |
| [  Generate Cover Page  ]                                         |
+-------------------------------------------------------------------+
```

#### The 3 Curated AI Styles
1. **Retro Waves (`AI Retro Waves`, Style 100)**:
   - **Visual Personality**: Flowing, organic, 1970s risograph and modern editorial curves.
   - **Composition**: Sweeping retro curved ribbons with generous negative space crafted specifically to keep typography clear and high-contrast.
   - **Best For**: General statistical overviews, cultural publications, environmental and regional digests.
2. **Geometric Business (`AI Geometric Business`, Style 101)**:
   - **Visual Personality**: Clean, authoritative, flat 2D modern business geometry.
   - **Composition**: Large angled diagonal thick rounded stripes across, floating rounded hexagons, geometric accent frames, and generous negative space for vector titles.
   - **Best For**: Governmental yearbooks, regional statistical monographs (KCDA), official departmental summaries.
3. **Minimal Sci-Fi (`AI Minimal Sci-Fi`, Style 102)**:
   - **Visual Personality**: High-tech, blueprint aesthetics, crisp structural gradients.
   - **Composition**: Clean technological lines, subtle isometric grid accents, and open architectural space.
   - **Best For**: Industrial statistics, digital economy, census data, scientific directories.

#### The 6 Cohesive Color Palettes
Each AI preset can be combined with one of six tailored palettes:
- **Warm Terracotta**: Rich terracotta (`#d96b43`), warm ochre (`#e8a838`), and charcoal slate (`#2c3539`).
- **Pastel Dream**: Soft lavender (`#a29bfe`), gentle coral (`#fab1a0`), and deep midnight violet (`#2d3436`).
- **Mono Slate**: Steel blue (`#4b6584`), refined light gray (`#a5b1c2`), and deep graphite (`#1e272e`).
- **Forest Gold**: Emerald green (`#20bf6b`), rich gold (`#f7b731`), and deep pine forest (`#0b3b24`).
- **Corporate Blue**: Dark navy blue (`#0a1931`), deep royal blue (`#003399`), and electric cyan (`#00a8ff`).
- **Teal Lime**: Deep teal green (`#006d6f`), dark pine green (`#0a2e28`), and chartreuse lime green (`#84cc16`).

#### Cloudflare Workers AI Setup & Credentials
AI Base covers are powered by **Cloudflare Workers AI** running the Stable Diffusion XL (SDXL) model:
1. Click **AI Setup…** in the Publication Cover dialog (or next to the Translate button).
2. Enter your **Cloudflare Account ID** and **Cloudflare API Token**.
3. **Token Scoping**: Create the token in your Cloudflare dashboard with the `Workers AI: Edit` permission only. Never grant account-wide administrator privileges.
4. Click **Verify**: Pubgen performs a tiny 1-step test generation to confirm your credentials without wasting quota. The token is stored locally in your private user preferences and is never embedded into documents or committed to Git.
5. **Privacy Note**: Only abstract artistic prompt templates leave your machine. Publication titles, author names, and confidential regional data stay strictly local and are composed onto the cover as native vector text.

#### Batch Generation Economics & Quota Pre-Flight
- **Neuron Economics**: Each 18-step SDXL cover consumes approximately **~220 Neurons**. Cloudflare provides a free tier of **10,000 Neurons per day** (resetting daily at 00:00 UTC).
- **Safe Batch Size**: The free daily quota comfortably covers up to **~45 publication covers per day**.
- **Pre-Flight Warning**: When you launch a batch larger than 45 covers from an Excel workbook, Pubgen displays a safety prompt: *"45 of N fit today's free quota; the rest will fail after ~45 — continue?"*.
- **Deterministic Series Seeds**: Pubgen hashes each publication title (`hash(title)`) into a stable 32-bit random seed. Publications in the same series get visually cohesive art that remains consistent across regenerations.
- **Stop-on-429 & Resume**: If your Cloudflare quota is exhausted mid-batch (HTTP 429), Pubgen stops generation cleanly without crashing, saves all completed covers, and reports the exact titles that were skipped.
- **Re-run Skipping**: Re-running a batch automatically checks for existing covers (tagged with `#ai_preset:` in the background frame) and only generates the missing ones, making "generate the rest tomorrow" a single click.

#### Excel Integration (`COVER` / `COVERS` Sheet)
You can specify AI Base styles directly in your Excel workbook:
- **Style Column**: `AI Retro Waves`, `AI Geometric Business`, `AI Minimal Sci-Fi`, `AI Dynamic Waves Spread`, or `AI Vibrant S-Curve Spread` (legacy alias `Dark Fantasy` is also supported).
- **Palette Column**: `Teal Lime`, `Corporate Blue`, `Forest Gold`, `Mono Slate`, `Pastel Dream`, or `Warm Terracotta`.

---

### Hover Thumbnail Previews
- **Quick Preview Pills**: Under the style dropdown, you will see quick-switch pills for all 13 vector corporate styles plus the 3 AI Base styles.
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
- **Photo**: Used in *Corporate Headline*, *Diagonal Arrow*, *Modern Horizon* and *Tri-Color Bands*. Vertical architectural, city skyline, or landmark photograph. In *Diagonal Arrow* it fills the full-height panel on the right, behind the diagonal banner and the arrow. In *Modern Horizon* it is clipped by the sweeping arc across the upper page, so choose a wide cityscape with interest along the horizon.
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

## 4. Page from CSV / Excel (Batch Page & Chapter Synthesizer)

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

## 5. Table from CSV / Excel (Statistical Table Builder)

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

## 6. Page & Layout Utilities & Editorial Tools

The **Publication Tools ▸ Page & Layout Utilities…** dialog holds the essential shortcuts for publication assembly.

```
+-------------------------------------------------------------------+
| Page Margins & Renumbering Options                                |
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
| **TOC Entry** | every page whose title carries the tag style, plus the narrative pages (the original behaviour) | *left as typed* |
| **TOC Table** | only pages whose **first text box** starts with `Tabel` or `Table` (case-insensitive) | `Daftar Table` |
| **TOC Chart** | only pages whose **first text box** starts with `Grafik` or `Chart` (case-insensitive) | `Daftar Grafik` |
| **TOC Paragraph** | the narrative pages (the `GXX` and `RXX` sheets) with the headings under them, nested | `Daftar Isi` |

Picking **TOC Table** or **TOC Chart** fills the Title field for you and centres the title line; you can still edit
the title. The filter is stored in the document, so **Rebuild TOC** and *Layout ▸ Update Table of Contents* keep it.

**TOC Paragraph** is the nested list of a publication: the narrative pages (the front matter from the `GXX`
sheets and the chapter reviews from the `RXX` sheets) at the first level, the `Heading 1` and `Heading 2`
paragraphs of their bodies below them. No other page — a table, a chart, a divider or a publication note —
is listed. A narrative page's entry carries its chapter line and its title, both of its text boxes reading
`BAB 01 Geografi ......... 2`; the chapter line only counts when it has its own box above the title, so a page
that opens with another line in the title's box keeps its own text. A `GXX` sheet with no title row (only the
RTF link) puts the RTF at the top of the page, so the page's title is the largest centred line among the RTF's
opening paragraphs — the heading, written as a Word heading in some sheets and as direct formatting in others.
That leaves out the ISSN line above it (right-aligned) and the publication's title below it (set smaller), and
copes with an empty opening paragraph.

Each list keeps the paragraph styles its rows use: `TOC Level 1…n` for the contents, `TOC Table Level 1…n`
for the Tables list and `TOC Chart Level 1…n` for the Charts list. The styles carry the list's tab stop and
indent, so generating or rebuilding one list never moves the rows of another.

**Include TOC page in TOC entries** lists the table of contents itself in the list it makes — for every TOC
style, the nested paragraph list included.

**Pair bilingual headings (ID / EN)** merges the left-column and right-column headings of a Dual Lang
page (`RXX` with `file_en`) into one row, joined like bilingual table titles:

    Proporsi fisik daratan / Physical proportions of the landmass ........ 4

Pairing follows each column's own vertical order (top heading with top heading, second with second) and
needs the two-column Dual Lang layout: rows whose frames overlap — a full-width heading, a table or chart
page — stay their own rows, and a heading without its twin keeps its own row too. The pair reads at the
Indonesian heading's level, whatever styles the two RTFs gave the twins. The option is stored in
the list's definition, so **Rebuild TOC** keeps pairing; with the checkbox off nothing changes.

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
5. Click **Rebuild TOC** in **Publication ▸ Table of Contents…**: all page references automatically update to show Roman numerals for front matter and Arabic numbers for body pages!

---

## 7. Master Excel: One Workbook to Finished Pages

The **1. Data Source & Generate (Master Excel)** card at the top of the PubGen panel turns one `.xlsx` workbook into a finished chapter — pages, tables, charts and the cover — in a single action.

| Control | What it does |
|---|---|
| **Auto-generate pages, tables & cover from sheets** | On by default. Picking a workbook then runs the whole pipeline: "Page from CSV", every sheet's table with its default options, every chart page, and the cover built from the `COVER` sheet. Turn it off to convert the workbook to CSVs only. |
| **Excel to CSV Data (.xlsx)…** | Choose the workbook. Pubgen writes one catalog CSV plus one table CSV per data sheet, then (with auto-generate on) creates the pages. |
| **Active: …** | Names the loaded workbook or catalog and the number of catalog records it holds. |
| **Download … / Download Generated CSVs as ZIP…** | Saves every generated catalog and table CSV in one `.zip`. |
| Status line | Reports what was generated, or the first error, plus the vocabulary count described in §13. |

### What each sheet becomes

| Sheet name | Becomes |
|---|---|
| `COVER` | The Publication Cover values: `style`, `title1`, `title2`, `year`, `volume`, `no_catalogue`, ISSN/ISBN, `office_name`, `image_1`, `image_lanscape`, `office_logo`, `event_theme`. |
| `DIV` | A chapter divider page (procedural art or a template, coloured from the sheet's theme). |
| `GXX` | Front-matter narrative pages (Kata Pengantar, Penjelasan Teknis…). |
| `BXX` | Front-matter narrative pages that also carry the English half. |
| `RXX` | A review spread: Indonesian body, plus the English column in Dual Lang. |
| `XX_XX` | A statistical table page (`01_01`, `02_03`…), with its table inserted automatically. |
| `CXX_XX` | A chart page — vector when the sheet carries a `chart_type` block, otherwise its picture (§8). |
| `MXX_XX` | A GeoJSON choropleth map page (§8). |
| `A02` | A **Publication Note** row: the `Keterangan Publikasi` title plus the sheet's RTF note page. No table CSV is produced. |

Routing is **profile-driven** (§12): the cover, divider, narrative, table and chart patterns come from the active profile, and Pubgen switches profile automatically when it recognises BRS, corporate-financial or academic sheet names. An explicit profile choice always wins.

---

## 8. Vector Charts & Statistical Infographics

Charts are native document objects — vector bars, lines and polygons with real text — not pictures. They stay sharp at any zoom and in print, and they are editable afterwards.

Open the chart builder from **Publication ▸ Vector Chart** (`Ctrl+Shift+V`), the **Chart Options…** row in **Object ▸ Chart Options…**, or by letting Master Excel turn a `CXX_XX` sheet into a chart page.

### The nine archetypes

| Archetype | Values accepted in `chart_type` | Typical use |
|---|---|---|
| Clustered bars | `bar_clustered`, `bar`, `column`, `batang` | side-by-side comparison per category |
| Stacked bars | `bar_stacked`, `stacked`, `stackedbar`, `stackedcolumn` | parts of a whole per category |
| Horizontal bars | `horizontal_bar`, `hbar`, `bar_h`, `batanghorizontal`, `ranked` | ranked regional comparison |
| Line | `line`, `garis`, `trend` | multi-year series |
| Area | `area`, `filled`, `arealine` | trends with volume |
| Donut | `donut`, `doughnut`, `pie`, `donat`, `lingkaran` | shares of a total |
| Population pyramid | `population_pyramid`, `pyramid`, `piramidapenduduk`, `piramida` | age/sex distribution |
| Gauge | `gauge`, `speedometer`, `dial`, `alatukur` | one metric against a target |
| Choropleth map | `map`, `choropleth`, `peta`, `geomap`, `thematicmap` | values by area |

Donut charts fit up to two series — one ring per series (for example two years) — with a period band under each ring, two-decimal comma percentages, and elbow leader lines to labels placed outside small slices.

### What the chart dialog asks for

1. **Import from Table** — pick a table already in the document (the dropdown lists `Tabel 01.01 — … (Page N)`), choose the page, then press **Auto-fill** to read the categories, series and labels from that table.
2. **Insert Location** — *Selected page*, *After table*, or *Current page*.
3. **Chart Type & Titles** — the archetype plus **Title (ID)**, a two-line field that follows the BPS format (`Grafik 01.01` on line 1, the data title on line 2), and **Title (EN)**, rendered as `Figure 01.01 …` in italic in Dual Lang.
4. **Data Columns & Rows** — the category column letter, the row span, and up to three series, each with its column letter and its Indonesian and English legend labels.
5. **Maps** (choropleth only) — a stock administrative map or a custom `.geojson` path, a colour theme, and a sample village CSV for trying the engine.
6. **Formatting & Display Options** — *Number (1.234,5)*, *Percentage (%)* or *Currency IDR (Rp)*, a maximum-value override (blank computes a "nice" ceiling), and **Show Value Labels**, **Show Legend**, **Show Gridlines**.

### Charts from Excel

A chart sheet is a block of `key` / `value` pairs. Only `chart_type` is mandatory:

```
chart_type        bar_clustered
title             Pertumbuhan Ekonomi Menurut Sektor
title_en          Economic Growth by Sector
category_col      A
series_1_col      B
series_1_label    Pertanian
series_1_label_en Agriculture
series_2_col      C
series_2_label    Industri
data_start_row    5
data_end_row      12
value_format      percent
show_value_labels true
```

Other accepted keys: `data_start_row` / `data_end_row`, `max_value`, `show_legend`, `show_grid`, `geojson`. Series 2 and 3 use the same `series_N_col` / `series_N_label` / `series_N_label_en` pattern. Without a `chart_type` the sheet is placed as a picture, exactly as before.

In the dialog's **Data Table (CSV / TSV)** section, **Preview** shows the rows as an Excel-like grid with a row-number gutter (header rows, those before `data_start_row`, are unnumbered); **Edit** switches back to the raw CSV editor. Both views read the same data.

Charts are indexed automatically: a vector chart page appears in **Daftar Grafik** / *List of Charts* like any other figure. Map pages can also feed a decorative map overlay behind the cover text and logo.

---

## 9. Header & Footer Setup

**Publication ▸ Header & Footer Setup** opens the template dialog. Headers and footers are document objects, so they belong to the page, not to a text frame.

- **Style** — built-in **Modern Minimal**, **Editorial Book** and **Corporate Report**, plus any templates you saved yourself (marked **★**). Hover a row, or the eye button, for a rendered preview.
- **Sided** — double-sided (mirrored left/right) or single-sided.
- **H. Off / F. Off** — header and footer distance from the page edge, in points.
- **Scope** — the whole document, one section (**Sections…**), or a page range.
- **Header / Footer text** — single, left and right fields. They accept `{page}`, `{section}` and `{runningHeader}`; a tab character splits the two halves of a single field.
- **Objects** — header rule and footer rule with their weights, and a top accent bar (height and bleed).
- **Colours** — primary, accent, dark, rule and text.
- **Save As New Template…** stores the current setup under a name for reuse; the bin button deletes a custom template.
- **Apply Header & Footer** writes it to the document; **Remove from Document** clears it.

The Publication Compiler can apply a preset for you: `"headerFooter": { "preset": "book", "scope": "body" }` (§10).

---

## 10. Publication Compiler (Multi-Chapter Binder)

The **Publication Compiler** card (top of the panel) builds an entire publication from one manifest. Instead of opening ten chapter workbooks one after another, you describe them once in `pubgen_project.json` and press **Compile**.

- **Manifest** — type the path, or use **Open…**. The last manifest you opened is remembered and used when the field is empty.
- **New…** — the project builder form: publication title, bilingual flag, save path, cover style and theme, chapter list, margins, pagination and PDF export. It writes a ready-to-run `pubgen_project.json`.
- **Compile** — runs the pipeline with a progress bar and a working **Cancel**. A dot marks the Publication menu row while it runs.
- A build that fails or is cancelled leaves the previous document untouched: the compiled file is saved second-to-last. A document already open in a tab is never overwritten — the build is skipped instead.

### The manifest

```json
{
  "version": 1,
  "publicationTitle": "Kecamatan Karawang Timur Dalam Angka 2026",
  "bilingual": true,
  "profile": "bps_dda",
  "output": {
    "document": "build/karawang-timur.designcraft",
    "pdf": "build/karawang-timur.pdf"
  },
  "document": {
    "preset": "A4",
    "facingPages": true,
    "bleedMm": 3,
    "marginsMm": { "top": 20.2, "bottom": 20, "inside": 19, "outside": 15 }
  },
  "cover": {
    "source": "workbooks/BAB_01_2025.xlsx",
    "style": 0,
    "themePreset": 0,
    "includeInside": true,
    "includeBack": true
  },
  "frontMatter": { "workbook": "" },
  "chapters": [
    { "chapter": 1, "path": "workbooks/BAB_01_2025.xlsx", "title": "Geografi" },
    { "chapter": 2, "path": "workbooks/BAB_02_2025.xlsx", "title": "Pemerintahan" }
  ],
  "pagination": { "frontMatterFormat": "lowerRoman", "bodyFormat": "arabic", "restartAtBody": true },
  "toc": {
    "placement": "afterFrontMatter",
    "lists": ["contents", "tables", "charts"],
    "titleBilingual": true,
    "hangingIndent": true
  },
  "headerFooter": { "preset": "book", "scope": "body" },
  "build": { "keepIntermediate": true, "exportPdf": true }
}
```

Every relative path — chapter workbooks, cover source, output files — resolves against the folder holding the manifest. Unknown keys are ignored, and every section may be omitted to take its default.

### What a compile does, in order

1. A new document with the manifest's page size, facing pages, margins and bleed (profile margins fill in when the manifest leaves them at the defaults).
2. Parent pages for the body.
3. Front cover and (optionally) inside cover, from the `COVER` sheet.
4. Front matter, numbered in lowercase Roman when `restartAtBody` is on.
5. TOC placeholders for the requested lists.
6. Each chapter in order: divider, reviews, charts, tables — body numbering restarts at Arabic 1 and continues across chapters.
7. Back cover.
8. The contents lists, page numbering and running headers.
9. Save, then export the PDF when `build.exportPdf` is on.

The compiler is desktop-only: the web build reports that compiling needs the desktop app. From the control channel or a script: `{"command":"pubgen.compileProject","params":{"path":"…/pubgen_project.json","pdf":"…/out.pdf"}}`; add `"sync": false` to run it step by step in the panel instead.

The PDF this produces is a normal export. The dedicated Print (PDF/X-4) and Web presets are not implemented yet — export those from **File ▸ Export PDF…** for now.

---

## 11. Audit Publication (Preflight QA)

**Publication ▸ Audit Publication** (`Ctrl+Shift+Alt+A`), or the **Audit Publication** button in the panel header, runs the publication checklist before you send anything to print or publish online.

| Rule | Checks | Severity |
|---|---|---|
| `PRF-01` | Text clipped inside a native table cell (column too narrow, row too shallow). | Error |
| `PRF-02` | Story overset: narrative text that overflowed without a continuation frame. | Error |
| `PRF-03` | `Sumber:` / `Catatan:` note lines with nothing after the colon. | Warning |
| `PRF-04` | Table and figure numbering: gaps and duplicates, grouped by kind and chapter (a bilingual pair on one page is not a duplicate). | Warning |
| `PRF-05` | Dual-language parity: review pages whose English body is empty, a placeholder, or still the Indonesian text. | Warning |
| `PRF-06` | Raster images below the DPI floor for the chosen profile. | Warning |
| `PRF-07` | Print only: graphics crossing a trim edge without 3 mm bleed, and text within 5 mm of the spine (or across it). | Error |

- **Digital (online PDF)** — 150 ppi floor, no spine/bleed guards.
- **Print (press)** — 300 ppi floor, plus the spine and bleed guards.

Click any row to select the offending object and jump to its page. The **FP / IG** button cycles a tag — *false positive* or *ignored*. Tagged rows dim, can be hidden with **hide tagged**, and stop counting in the header tally; tags are keyed by rule and message, so they survive a re-run and a regeneration. **Min ppi** overrides the profile floor, and **Re-run** refreshes the list. The dialog also re-runs itself when the document changed since the last audit, so a row never points at an object that no longer exists.

---

## 12. Publication Profiles

A profile is the publication's house style, declared once: which sheets are what, what things are called, page geometry, front-matter structure and table styling. Switch profile and the same workbook produces a different publication.

| Profile | For | Geometry | Tables |
|---|---|---|---|
| **BPS Daerah Dalam Angka** (`bps_dda`) | Official KDA/KCDA yearbooks (default) | A4, facing pages, 20.2 / 20 / 19 / 15 mm | column-number row, theme palette, chapter dividers |
| **BPS Berita Resmi Statistik** (`bps_brs`) | Press releases and periodic releases | A4, single page, 18 mm all round | no column numbers, zebra rows |
| **Laporan Tahunan Korporat** (`corporate_annual`) | Corporate annual and sustainability reports | A4, facing, 25 / 25 / 22 / 18 mm, `Section` / `Table` / `Figure` labels | horizontal rules only, corporate navy |
| **Monograf Akademik & Riset** (`academic_monograph`) | Research papers, policy briefs, LKIP/LAKIP | A4, facing, 25 / 25 / 25 / 20 mm, `Gambar` for figures | APA-style three-line tables, monochrome |

Where to use it:

- **Panel header** — the profile dropdown, the **Auto-detect** badge when Pubgen recognised the workbook, a one-line summary of page size, margins and column numbering, and **Sesuaikan…** for the customizer.
- **Publication Tools ▸ Publication Profiles** — the same list as checkmarks, **Customize Active Profile…**, and **Apply Profile Margins**.
- **Publication Tools ▸ Page & Layout Utilities ▸ Standard BPS Margins** (`Ctrl+Shift+M`) — applies 20 / 20 / 19 / 15 mm regardless of profile.
- The compiler honours `"profile"` in the manifest (§10).

The **customizer** edits the active profile in place: page size and facing pages, the four margins, chapter/table/chart prefixes, TOC and list titles, source and note labels, front-matter numbering, chapter dividers, inside and back cover, and table styling. **Apply to this session** keeps your edits; **Reset to BPS DDA** goes back to the flagship defaults.

Two limits worth knowing: a customised profile lives for the session only (the active choice is not saved between launches), and profiles cannot be imported or exported as files yet.

---

## 13. AI Translation & Terminology Memory

Pubgen's optional AI features run through **Groq** (online translation) and **Cloudflare Workers AI** (AI cover backgrounds, §3). Everything below needs the online build (`--features pubgen,ai-translate-online`) and a configured key.

### Setup

Open **Publication Tools ▸ AI Translate ▸ Setup…**, or press **AI Translator** in the panel header. Paste your Groq API key from [console.groq.com/keys](https://console.groq.com/keys), pick a model (**Refresh models** fills the list from your account), then **Use AI**. The key is stored in your local DesignCraft preferences. The dialog is honest about the trade: catalog text is sent to Groq's cloud, and free-tier rate limits make large jobs slower rather than blocking them.

### Translating pages

Dual Lang must be on (**Publication Tools ▸ Dual Language Mode**, `Ctrl+Shift+L`). Then:

- **Translate this Page** — the label becomes **Translate Review on this Page** on an `RXX` review page (chapter title and right-hand body are replaced with English, paragraph styles preserved) or **Translate Table on this Page** on a table page (title, header cells and the source/note box are translated in place, English on a new line in italic).
- **Translate Tables…** — batch translation over a page range, opened from the same button or **Publication Tools ▸ AI Translate ▸ Translate Table Pages…**.
- A progress bar and a working **Cancel** run in the background; translations are cached, so a second Generate or Translate over the same rows is instant.

### Terminology memory

An official glossary keeps AI output on standard statistical wording — *Kecamatan* → *Subdistrict*, *PDRB* → *GRDP*, *Luas Wilayah* → *Total Area*.

- The bundled glossary is `assets/pubgen/bps_glossary.json`. Resolution order: your saved path in preferences, then the asset, then the build folder. A missing or malformed file degrades to an empty glossary, never an error.
- The glossary is injected into the prompt (up to 200 terms, `id = en`) and applied **deterministically to whole cells** after translation.
- The **Terminology** tab in the AI setup dialog is a full editor: search, add, edit, remove, **Import CSV…** (two columns — Indonesian, English — comma, semicolon, tab or pipe; header optional), **Reset to bundled**, and **Save…** to a `bps_glossary.json` of your own.
- **Collect Vocabulary** reads the generated pages (headers, row labels, titles, note and source lines, plus phrase runs) and adds the Indonesian terms it finds with an empty English column; every Excel generation does this automatically and reports `Vocabulary: N new term(s) collected`.
- **Fill AI Drafts** fills those blank English cells from the model term by term. The **Auto** checkbox beside it repeats the step after each generation.
- **Fix Terms** applies the glossary to translations already on the selected page; **Retranslate (Glossary)** corrects those terms and re-runs only the texts the glossary flagged (one request batch).

> **Not in the product:** the earlier *Draft Review* narrative summarizer was removed in October 2026. Pubgen does not draft chapter prose from table figures today; the AI work is translation and terminology. If you need a commentary paragraph, write it and place it as RTF.

---

## 14. Step-by-Step Practical Tutorials

### Tutorial 1: Creating a Complete Publication in 3 Minutes

Follow this workflow to create a complete statistical booklet from zero:

1. **New Document**:
   - Press `Ctrl+N` (File > New Document). Choose `A4`, Facing Pages checked.
2. **Generate Cover Page**:
   - Open **Publication ▸ Publication Cover…**.
   - Choose style **`⬡ Hexagon`** (or hover over the pills to preview).
   - Enter your Title: `KARAWANG TIMUR` and Year: `2026`.
   - Click **Generate Cover Page**. Your cover is created on Page 1!
3. **Generate Content Pages**:
   - Open **Publication Tools ▸ Manual Ingest & Tables ▸ Page from CSV…**.
   - Click **Browse...** and select `samples/KBDA2026_chapters.csv`.
   - Preset: `1 Row · 3 Columns`.
   - Click **Generate Pages**. 28 formatted pages are appended to your document!
4. **Insert Statistical Tables**:
   - Go to Page 2 in the document.
   - Open **Publication Tools ▸ Manual Ingest & Tables ▸ Table from CSV…**.
   - Click **Browse...** and select `samples/kbda_2005.csv`.
   - Style: `Horizontal Rules (Booktabs)`. Placement: `At page (below title)`.
   - Check `[x] Condition rules (Jumlah, Tahun, Kab, Kec...)`.
   - Click **Insert Table**. The table appears directly beneath the page header!
5. **Add Table of Contents**:
   - In **Publication Tools ▸ Page & Layout Utilities…**, click **Add Blank Page**.
   - Open **Publication ▸ Table of Contents…**, then set Title: `DAFTAR TABEL`, Layout: `TablePage`.
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

### Tutorial 4: Compiling a Whole Publication from One Manifest

For a yearbook with eight to twelve chapter workbooks, the compiler replaces the manual loop:

1. **Collect the workbooks** in one folder, for example `workbooks/BAB_01_2025.xlsx` … `workbooks/BAB_10_2025.xlsx`, with the `COVER` sheet in chapter 1 or its own workbook.
2. **Pick a profile** in the panel header — `BPS Daerah Dalam Angka` for a KDA/KCDA yearbook (§12). The manifest's own `"profile"` wins when it sets one.
3. **Create the manifest**: in the Publication Compiler card press **New…**, fill in the title, the chapter list, margins and whether to export a PDF, then choose where to save `pubgen_project.json`. The full schema is in §10.
4. **Compile**: press **Compile**. The progress bar names each step — new document, masters, front cover, inside cover, front matter, list placeholders, each chapter, back cover, contents lists, page numbering, running headers, save, PDF. **Cancel** stops cleanly and leaves any previous build untouched.
5. **Check it**: read the status line (pages, chapters, warnings), then run **Audit Publication** (§11) and click any flagged row to jump straight to it.
6. **Export**: with `"exportPdf": true` the PDF is written beside the document; otherwise use **File ▸ Export PDF…**.


---

## 15. Frequently Asked Questions & Troubleshooting

#### Q: My CSV file has semicolons instead of commas. Will Pubgen recognize it?
**A:** Yes! Pubgen features an intelligent delimiter sniffer that automatically identifies semicolons (`;`), commas (`,`), and tabs (`\t`). You do not need to convert your file.

#### Q: Why is my Table of Contents showing `#` instead of page numbers?
**A:** If page numbers show `#`, it means the text frame was placed on a master/parent page rather than a document page. Make sure you are generating the TOC on a normal document page.

#### Q: Can I edit the text and tables after Pubgen generates them?
**A:** Absolutely! Everything Pubgen creates—text frames, character runs, table cells, and vector paths—are **100% native DesignCraft objects**. You can double-click any text to edit it, adjust table column widths with the table tool, or change swatch colors at any time.

#### Q: Where are my settings saved in the Portable Demo?
**A:** The portable demo includes a file named `portable.txt`. This instructs DesignCraft to store all your settings, crash recovery files, and preferences inside the local `DesignCraftData/` folder right next to `designcraft.exe`. Nothing is written to your Windows registry or `%APPDATA%`, keeping your computer completely clean.

#### Q: A chart page appeared as an ordinary picture instead of a vector chart. Why?
**A:** A `CXX_XX` sheet is drawn as a vector chart only when it carries a `chart_type` row (§8). If that row is missing or the value is not one of the nine archetypes, Pubgen keeps the old behaviour and places the sheet's picture. Add `chart_type`, then regenerate or use **Publication ▸ Vector Chart** to build the chart from the table on the page.

#### Q: Can I hide an audit warning I have decided is fine?
**A:** Yes. Click the **FP / IG** button on that row to tag it *false positive* or *ignored*. Tagged rows dim, stop counting in the header tally, and can be hidden entirely with **hide tagged**. Tags are keyed by rule and message, so they survive a re-run and even a regeneration.

#### Q: I chose a publication profile and it was back to BPS DDA after restarting. Is that a bug?
**A:** No. The active profile and the customizer's edits live for the session; only the compiler manifest's `"profile"` is stored with the publication. Profiles also cannot be imported or exported as files yet (§12).

#### Q: Why is my chapter prose not translated or drafted by AI?
**A:** The AI features translate page and table text and maintain the terminology glossary; the earlier **Draft Review** narrative summarizer was removed in October 2026. Place your commentary as RTF and translate that page if you need an English version (§13).

#### Q: Compile does nothing on the web build. Why?
**A:** Compiling reads and writes files and needs the desktop app; the web build says so instead of pretending. On desktop, a manifest whose output document is already open in a tab is skipped so the build cannot overwrite your work.

#### Q: How do I export my publication to high-quality PDF?
**A:** Use **File ▸ Export PDF…**. You can choose PDF/X-4, PDF/X-1a:2003 or PDF/A-2b, set CMYK and spot inks, bleed with crop and bleed marks, image compression and 300 ppi output for commercial offset printing.

---

*DesignCraft Publication Generator (Pubgen) — Engineered for Speed, Elegance, and Precision.*
