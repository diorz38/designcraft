# PubGen Architecture Analysis: Scalable Cover & Page Template System

## 1. Executive Summary

This document analyzes the current implementation of Publication Generator (**PubGen**) styles in DesignCraft and evaluates its scalability. It addresses two critical architectural questions:
1. **Cover Styles Scalability**: What happens when scaling from 10 styles to **50+ styles** using the current hardcoded approach in `pubgen.rs`?
2. **Generalizing the Template Approach**: Can the template-based architecture be implemented for **any kind of predefined page style** (chapter dividers, statistical tables, editorial articles, forewords, and infographics)?

---

## 2. Current Implementation Assessment

### 2.1 File Location & Dimensions
All generation logic currently resides in a single monolithic file:
- **File**: [`crates/ui-egui/src/panels/pubgen.rs`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs)
- **Current Total Lines**: ~10,132 lines (~460 KB)
- **Status in Repository**: The largest single source file in the codebase.

### 2.2 Where Each Component Lives

| Component | Code Location in `pubgen.rs` | Description |
|---|---|---|
| **Cover Generation** | [`L1709–3708`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L1709-L3708) (`execute_generate_cover`) | Imperative commands (`frame.create`, `object.color`, `text.insert`) constructing geometries and text frames for each style. |
| **UI Cover Selector** | [`L1180–1215`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L1180-L1215) | ComboBox and quick-switcher pill buttons. |
| **Style Descriptions** | [`L1158–1170`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L1158-L1170) | Text summaries displayed on hover tooltips. |
| **Dynamic Image Inputs** | [`L1391–1544`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L1391-L1544) | Per-style image file pickers (e.g. Skyline, Landscape, Logo). |
| **Vector Fallback Thumbnails** | [`L760–1145`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L760-L1145) (`paint_fallback_cover_thumb`) | Vector drawing routines when rendered PNG thumbnails are absent. |
| **Page Layout Presets** | [`L4343–4600`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L4343-L4600) (`execute_generate_pages`) | Hardcoded layouts for 1-row/3-cols, Editorial Chapter ("Ulasan"), Technical Spec, TOC Line. |
| **TOC Generation** | [`L6571–6670`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L6571-L6670) (`generate_toc`) | Table of contents page builder. |

### 2.3 Style Mapping Breakdown (Styles 0–9)

- **Style 0 ("Wave Curves")**: [`L1724–1837`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L1724-L1837)
- **Style 1 ("Geometric & Medallion")**: [`L1839–1960`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L1839-L1960)
- **Style 2 ("Folded Origami")**: [`L1962–2089`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L1962-L2089)
- **Style 3 ("Chevron & Medallion")**: [`L2091–2301`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L2091-L2301) *(derived from `design-1.designcraft`)*
- **Style 4 ("Honeycomb Hexagon")**: [`L2303–2588`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L2303-L2588) *(derived from `design-2.designcraft`)*
- **Style 5 ("Corporate Headline")**: [`L2590–2838`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L2590-L2838) *(derived from `design-3.designcraft`)*
- **Style 6 ("Diagonal Arrow")**: [`L2840–3032`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L2840-L3032) *(derived from `design-4.designcraft`)*
- **Style 7 ("Modern Horizon")**: [`L3034–3219`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L3034-L3219) *(derived from `design-5.designcraft`)*
- **Style 8 ("Business Report")**: [`L3221–3480`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L3221-L3480) *(derived from `design-10.designcraft`)*
- **Style 9 ("Urban Angle")**: [`L3482–3703`](file:///e:/Github/designcraft/crates/ui-egui/src/panels/pubgen.rs#L3482-L3703) *(derived from `design-6.designcraft`)*

### 2.4 Associated Assets
- **Rendered Thumbnails**: [`plan/pubgen/samples/thumb/`](file:///e:/Github/designcraft/plan/pubgen/samples/thumb/)
- **Source Reference Files**: [`plan/pubgen/samples/style/`](file:///e:/Github/designcraft/plan/pubgen/samples/style/) (`design-1.designcraft` to `design-10.designcraft`)

---

## 3. Scalability Risks: What Happens at 50+ Styles?

If the current monolithic pattern continues as new styles are added:

### 1. Codebase Bloat (25,000–30,000+ Lines)
Each style requires ~200 to 300 lines of imperative Rust code to set up points, shapes, colors, and text frames. At 50 styles:
- Cover generation logic alone: **~12,000–15,000 lines**.
- Fallback vector thumbnail rendering: **~2,000 lines**.
- UI image pickers and dynamic configuration: **~1,500 lines**.
- Total file size for `pubgen.rs` would exceed **25,000 lines (> 1.2 MB)**.

### 2. Tooling and Developer Experience Degradation
- **`rust-analyzer` Performance**: Language servers experience high latency when indexing, parsing, and typechecking huge source files. Auto-completions, inline type hints, and hover documentation become noticeably sluggish.
- **Incremental Compilation**: Any change in one style invalidates the entire 25k-line compilation unit, slowing down iterative compile/test cycles.

### 3. Collaboration Bottlenecks & Merge Conflicts
Because UI layout, state definitions, table generation, and all 50 cover styles live in the exact same file, any concurrent feature work or bug fixing in PubGen triggers frequent git merge conflicts.

### 4. Barrier to Non-Developer Contributors
Every style requires translating visual layout concepts into hundreds of manual JSON engine commands (`app.run("frame.create", json!({"rect": ...}))`). Designers cannot directly contribute or edit templates; a Rust developer must manually code every vector coordinate and text box.

---

## 4. Proposed Architectural Solutions

### 4.1 Solution 1: Modular Rust Architecture (Immediate Refactoring)

Transform `crates/ui-egui/src/panels/pubgen.rs` into a structured module folder:

```text
crates/ui-egui/src/panels/pubgen/
├── mod.rs                  # Primary panel entry point & tab manager
├── state.rs                # PubgenState struct and persistence
├── excel.rs                # Excel data ingestion & mapping
├── covers/
│   ├── mod.rs              # CoverStyle registry & dispatch
│   ├── types.rs            # CoverStyle trait & common data structures
│   ├── wave_curves.rs      # Style 0
│   ├── geometric.rs        # Style 1
│   ├── folded_origami.rs   # Style 2
│   ├── ...
│   └── urban_angle.rs      # Style 9
```

#### The `CoverStyle` Trait Interface
```rust
pub struct CoverImageSlot {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub default_path: &'static str,
    pub tooltip: &'static str,
}

pub trait CoverStyle: Send + Sync {
    fn id(&self) -> usize;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn image_slots(&self) -> &'static [CoverImageSlot];
    fn generate(&self, app: &mut DesignApp, st: &PubgenState) -> Result<(), String>;
    fn paint_vector_thumb(&self, painter: &egui::Painter, rect: egui::Rect, t: &Tokens);
}
```

---

### 4.2 Solution 2: Data-Driven / Template System (Recommended for 50+ Styles)

DesignCraft already has a native document format: `.designcraft`. In fact, Styles 3 through 9 were originally authored as `.designcraft` files and then manually reverse-engineered into Rust code.

In a template-driven system, templates remain native `.designcraft` files stored under an assets directory:

```text
assets/templates/covers/
├── 001_wave_curves.designcraft
├── 002_geometric_medallion.designcraft
├── ...
└── 050_executive_annual.designcraft
```

#### How the Template Pipeline Operates:
1. **Design Visually**: A graphic designer creates the cover page in DesignCraft's GUI.
2. **Standard Frame Tags**: Elements to be populated by PubGen are labeled with standard tags (e.g. `#slot:title`, `#slot:year`, `#slot:hero_image`).
3. **Template Instantiation Pipeline**:
   ```text
   [User Click] 
        ↓
   [Load template .designcraft]
        ↓
   [Apply Colors (Primary, Accent, Dark Swatches)]
        ↓
   [Inject Dynamic Text Strings into Tagged Frames]
        ↓
   [Swap Image Paths on Tagged Photo Frames]
        ↓
   [Insert as Page 1 into Active Document]
   ```

---

## 5. Generalizing the Template Approach to ANY Predefined Page Style

**Yes, the template approach can be universally applied to every kind of predefined page style.**

In professional desktop publishing (Adobe InDesign Master/Parent Pages, InDesign Server, QuarkXPress), this is the standard methodology. Interior publication pages benefit even more than covers because publication projects require consistent margins, running headers/footers, and structural page balance across hundreds of pages.

### 5.1 Predefined Page Style Catalog

| Page Category | Predefined Page Type | Tagged Slots in Template | Description |
|---|---|---|---|
| **Covers** | Front Cover & Back Cover | `#slot:title`, `#slot:year`, `#slot:hero_image`, `#slot:barcode` | Full bleed, bold graphic layouts, logo and agency branding. |
| **Front Matter** | Title & Copyright / CIP (*Katalog BPS*) | `#slot:catalogue_number`, `#slot:issn_isbn`, `#slot:metadata_block` | Formal publishing credentials, publisher details, publication codes. |
| | Foreword / Executive Summary (*Kata Pengantar*) | `#slot:preface_text`, `#slot:author_signature`, `#slot:author_photo` | Single or two-column text with headshot and signature block. |
| | Table of Contents (*Daftar Isi*) | `#slot:toc_header`, `#slot:toc_container` | Header styling, dot leaders, chapter & sub-chapter page numbers. |
| **Section Dividers** | Chapter Divider (*Pembatas Bab*) | `#slot:chapter_number`, `#slot:chapter_title`, `#slot:chapter_photo`, `#slot:summary_bullets` | Full-bleed photo or colored background separating chapters. |
| **Editorial / Narrative** | Chapter Analysis (*Ulasan*) | `#slot:title`, `#slot:dropcap_body`, `#slot:pull_quote`, `#slot:photo_with_caption` | 1, 2, or 3-column article flows with lead-in paragraphs and sidebars. |
| **Statistical Tables** | Single Table Full Page | `#slot:table_title`, `#slot:table_container`, `#slot:table_notes`, `#slot:source_footer` | Header banner, notes block, dynamic table frame, source footnote. |
| | Dual Table Page (Stacked or 2-Col) | `#slot:table_title_1..2`, `#slot:table_container_1..2` | Two statistical tables on a single page with aligned column widths. |
| | Table + Chart Combo Page | `#slot:table_title`, `#slot:table_container`, `#slot:chart_image` | Top half statistical table, bottom half visualization or regional map. |
| **Infographics** | Key Indicators / KPI Dashboard | `#slot:kpi_metric_1..4`, `#slot:map_image`, `#slot:takeaways_box` | Bold stat callouts (e.g. "98.4%", "1.2M") with summary cards. |
| **Back Matter** | Methodology, Glossary, Index | `#slot:section_title`, `#slot:alphabetical_flow_container` | Multi-column reference listings. |

---

### 5.2 The Universal "Slot Contract" Specification

To support any page style, DesignCraft frames are tagged using simple identifier conventions:

```text
       ┌─────────────────────────────────────────────────────────┐
       │   [#slot:text:chapter_num]   "BAB 01"                   │
       │   [#slot:text:chapter_name]  "Kependudukan"             │
       ├─────────────────────────────────────────────────────────┤
       │                                                         │
       │   [#slot:image:hero]                                    │
       │   (Auto-fits selected photo or illustration)            │
       │                                                         │
       ├─────────────────────────────────────────────────────────┤
       │   [#slot:table:main]                                    │
       │   (Auto-populates table rows/columns from CSV/Excel)    │
       │                                                         │
       ├─────────────────────────────────────────────────────────┤
       │   [#slot:text:source]       "Sumber: BPS Kab. Karawang" │
       └─────────────────────────────────────────────────────────┘
```

#### The 5 Universal Slot Types:

1. **Text Slots (`#slot:text:<key>`)**
   - Injects text from Excel/CSV columns or publication metadata.
   - Preserves all pre-configured typography: font family, weight, point size, line height, text color, letter-spacing, and alignment.
2. **Rich Story Slots (`#slot:story:<key>`)**
   - For long articles, narrative chapters (*Ulasan*), or prefaces (RTF/Markdown files).
   - Automatically flows across threaded text frames with support for drop-caps and pull-quotes.
3. **Image Slots (`#slot:image:<key>`)**
   - Swaps placeholder images with user-supplied files.
   - Respects pre-configured image fitting rules (`FillFrameProportionally`, `FitContentProportionally`, `Center`).
4. **Table Slots (`#slot:table:<key>`)**
   - Replaces a designated placeholder box with a populated data table.
   - Automatically inherits the template frame's dimensions, padding, borders, and alternating row tinting.
5. **Theme Swatches (`#swatch:primary`, `#swatch:accent`, `#swatch:dark`)**
   - Elements linked to document swatches automatically adopt the user's selected color palette across all pages.

---

### 5.3 Unified Template Directory Hierarchy

With this approach, the entire publication template system is organized as cleanly separated assets:

```text
assets/templates/
├── covers/                  # Front and back covers (50+ styles)
│   ├── 001_wave_curves.designcraft
│   ├── 002_geometric.designcraft
│   └── ...
├── front_matter/            # Front matter templates
│   ├── copyright_bps.designcraft
│   ├── foreword_executive.designcraft
│   └── toc_modern.designcraft
├── dividers/                # Chapter separator spreads
│   ├── fullbleed_photo.designcraft
│   ├── minimal_typography.designcraft
│   └── geometric_split.designcraft
├── editorial/               # Narrative & article layouts
│   ├── 1col_classic_story.designcraft
│   ├── 2col_magazine_spread.designcraft
│   └── 3col_newsletter.designcraft
├── data_tables/             # Statistical data layouts
│   ├── single_table_standard.designcraft
│   ├── double_table_stacked.designcraft
│   └── table_plus_chart.designcraft
└── infographics/            # Dashboard and highlight pages
    ├── 4card_kpi_summary.designcraft
    └── map_and_indicators.designcraft
```

---

### 5.4 Impact on `pubgen.rs` Codebase

Currently, `pubgen.rs` contains:
- ~2,020 lines for Cover generation.
- ~1,500 lines for Page generation presets and chapter headers.
- ~600 lines for vector fallback rendering.

**Adopting universal template bindings shrinks `pubgen.rs` by ~4,500–5,000 lines**, replacing hardcoded geometry loops with a concise, reusable template execution pipeline:

```rust
// Generic ~50-line template execution command:
app.run("template.apply", json!({
    "template": "templates/dividers/geometric_split.designcraft",
    "target_page": cur_page,
    "text_slots": {
        "chapter_num": "BAB 01",
        "chapter_name": "Pemerintahan & Wilayah",
        "source": "Sumber: BPS Kabupaten Karawang"
    },
    "image_slots": {
        "hero": "outputs/city_skyline.jpg"
    },
    "palette": {
        "primary": "#0072bc",
        "accent": "#f5ab19"
    }
}))?;
```

---

## 6. Cloud Template Distribution & GitHub Sync Architecture

### 6.1 Motivation & Principles

As the template library scales beyond 50+ covers, dividers, tables, and editorial layouts, storing all template files directly inside the core application git repository becomes counterproductive:
1. **Lightweight Core Repository**: Binary `.designcraft` files (ZIP containers with vector definitions, text styles, and embedded PNG thumbnails) cause unnecessary repository bloat if hundreds of templates are tracked directly in git history.
2. **Continuous Releases Without Binary Recompilation**: The design team or community can publish new templates weekly without requiring end-users to download a new application installer.
3. **Decoupled Asset Governance**: Follows DesignCraft’s sister asset patterns—such as [`storytold/craft-fonts`](https://github.com/storytold/craft-fonts) for typefaces and [`storytold/photocraft-corpus`](https://github.com/storytold/photocraft-corpus) for real-world test fixtures.

### 6.2 Companion Repository Structure (`storytold/designcraft-templates`)

A dedicated GitHub repository maintains the full collection:

```text
designcraft-templates/
├── templates.json                  # Manifest index with versioning, tags, and SHA256 checksums
├── covers/
│   ├── 000_wave_curves.designcraft
│   ├── 001_geometric_medallion.designcraft
│   ├── ...
│   └── 050_executive_annual.designcraft
├── dividers/
│   ├── 001_ulasan_chapter.designcraft
│   └── ...
├── data_tables/
│   ├── 001_single_table_standard.designcraft
│   └── ...
└── front_matter/
    ├── 001_publication_note.designcraft
    └── ...
```

#### Manifest Index (`templates.json`)
The remote repository publishes a machine-readable index:
```json
{
  "version": 1,
  "updated_at": "2026-10-20T12:00:00Z",
  "templates": [
    {
      "id": "050_executive_annual",
      "name": "Executive Annual Report",
      "category": "covers",
      "version": "1.0.0",
      "author": "DesignCraft Team",
      "license": "CC0-1.0",
      "path": "covers/050_executive_annual.designcraft",
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "tags": ["corporate", "annual-report", "formal"]
    }
  ]
}
```

### 6.3 Dual-Root Catalog Discovery

The engine’s catalog discovery (`template.catalog.list`) checks two storage layers:

```text
┌────────────────────────────────────────────────────────┐
│             DesignCraft Desktop App                    │
│                                                        │
│  ┌──────────────────────┐    ┌──────────────────────┐  │
│  │   Bundled Defaults   │    │    Synced Cache      │  │
│  │  (assets/templates/) │    │ (~/.designcraft/tpl) │  │
│  │     5–10 core styles │    │    50+ cloud styles  │  │
│  └──────────┬───────────┘    └──────────┬───────────┘  │
│             │                           │              │
│             └─────────────┬─────────────┘              │
│                           ▼                            │
│           Catalog Engine (template.catalog.list)       │
│                           │                            │
│                  PubGen & UI Panels                    │
└───────────────────────────┼────────────────────────────┘
                            │ (On "Sync" button click)
                            ▼
          GitHub: storytold/designcraft-templates
          ├── covers/
          ├── dividers/
          ├── data_tables/
          └── templates.json (Catalog index with shas)
```

1. **Bundled Core (`assets/templates/`)**:
   - Ships inside the application package (or embedded).
   - Contains a minimal set of 5–10 curated defaults (1–2 per category).
   - Guaranteed to work offline out-of-the-box with zero internet access.
2. **User Cache / Synced Directory**:
   - Platform-specific user data directory:
     - **Windows**: `%APPDATA%\DesignCraft\templates`
     - **macOS**: `~/Library/Application Support/DesignCraft/templates`
     - **Linux**: `~/.local/share/designcraft/templates`
   - Files downloaded from the GitHub repository are cached here.
   - User-authored custom `.designcraft` templates can also be dropped directly into this folder.

### 6.4 Sync & Update Mechanisms

1. **In-App GUI ("Sync Cloud Templates")**:
   - A single-click sync button in the Template Gallery / PubGen panel: `[🔄 Check for New Templates]`.
   - Fetches `templates.json` over HTTPS, compares local SHA256 hashes against remote checksums, and downloads only new or updated `.designcraft` files.
   - Instantly refreshes the in-app template list without restarting.
2. **CLI & Developer Tooling**:
   - `cargo xtask templates --sync` downloads the latest community pack for offline development.
   - `designcraft-cli template sync --repo <custom_or_official_url>` allows automated CI/CD and deployment environments to hydrate template caches.
3. **Private & Enterprise Repository Support**:
   - Organizations (such as government statistical agencies or corporate marketing departments) can point the sync URL to an internal private Git repository or intranet server to distribute proprietary brand templates across their organization.

---

## 7. Architectural Comparison

| Dimension | Current Approach (Monolith) | Solution 1: Modular Rust | Solution 2: Universal Template System | Solution 2 + GitHub Cloud Sync |
|---|---|---|---|---|
| **Cover Styles Supported** | 10 styles | 50+ (each in its own `.rs` file) | **Unlimited (0 lines of code)** | **Unlimited (Live Growing Library)** |
| **Interior Page Styles** | 6 hardcoded presets | 10–15 hardcoded presets | **Unlimited (Covers, Dividers, Tables)** | **Unlimited (Community Distributed)** |
| **Total Lines of Code** | ~10,132 lines (and growing) | ~8,000 lines (modularized) | **~4,000 lines (50% reduction)** | **~4,000 lines (50% reduction)** |
| **Adding a New Style** | Write 300+ lines of Rust math | Write 200+ lines of Rust math | **Save `.designcraft` file in UI** | **PR to GitHub Template Repo (0 app changes)** |
| **Compile Time Impact** | Heavy full-file recompile | Isolated incremental compiles | **Zero recompile needed** | **Zero recompile needed** |
| **Binary / Git Repo Bloat** | Low (code only) | Low (code only) | Moderate (assets in repo) | **Zero (assets stored in companion repo)** |
| **Designer Accessibility** | None (Programmers only) | None (Programmers only) | **Full visual design in app** | **Community contribution via GitHub PR** |
| **Custom / Agency Templates** | Impossible | Impossible | **Drop-in local files** | **Private Git or Intranet repo sync** |
| **Offline Operation** | Full | Full | **Full** | **Full (Bundled defaults + cached)** |

---

## 8. Phased Implementation Roadmap

1. **Phase 1: Engine Slot Contract & Apply Pipeline**
   - Implement `SlotType` and `parse_slot_tag` in `crates/engine/src/cmd/template/types.rs`.
   - Implement `template.page.apply` command with slot injection (`#slot:text:*`, `#slot:image:*`, `#slot:story:*`, `#slot:table:*`) and dynamic palette re-tinting.
2. **Phase 2: Local Catalog Discovery & Canonical Templates**
   - Implement `template.catalog.list` and `template.preview.get` in `crates/engine/src/cmd/template/catalog.rs`.
   - Package canonical starter templates under `assets/templates/` (`covers/`, `dividers/`, `data_tables/`, `front_matter/`).
3. **Phase 3: PubGen UI Integration**
   - Connect Cover Generation to `template.page.apply`.
   - Connect Card 1 Interior Page Generation to `template.page.apply` with "Modular Template" layout preset.
4. **Phase 4: Remote GitHub Template Repository & Sync**
   - Create `storytold/designcraft-templates` with `templates.json` index.
   - Add multi-root catalog scanning (`scan_template_catalogs`) supporting `assets/templates/` + user AppData.
   - Implement `template.sync` engine command and in-app "Sync Cloud Templates" button.

