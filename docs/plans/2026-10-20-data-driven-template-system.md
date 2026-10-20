# Data-Driven Page Template System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a data-driven, universal page template engine in DesignCraft that instantiates native `.designcraft` template files and populates tagged placeholder slots (`#slot:text:*`, `#slot:image:*`, `#slot:table:*`, `#slot:story:*`, `#swatch:*`), enabling unlimited cover styles (50+) and interior publication page styles with zero new Rust code required per style.

**Architecture:** 
1. **Engine Layer (`crates/engine/src/cmd/template.rs`)**: A new engine command `template.page.apply` that opens a `.designcraft` template file via `designcraft_format`, copies the template spread into the target document via deep-copy with ID renumbering (reusing and generalizing `datamerge::fill::copy_item`), overrides color swatches, and populates tagged frame slots with runtime data.
2. **Document & Slot Contract**: Standardized frame labeling conventions (`#slot:<type>:<key>`) to bind text, images, tables, and stories.
3. **Template Repository (`assets/templates/`)**: Canonical `.designcraft` template files organized into `covers/`, `dividers/`, `data_tables/`, and `editorial/`.
4. **PubGen UI Integration (`crates/ui-egui/src/panels/pubgen/`)**: Connect PubGen Cover Generation and Page Generation to the template engine, replacing thousands of lines of hardcoded coordinates with template bindings.

**Tech Stack:** Rust 2024 / 1.80+, `designcraft-engine`, `designcraft-doc`, `designcraft-format`, `serde_json`, `egui`.

---

## Task 1: Slot Contract Parser & Value Types

**Files:**
- Create: `crates/engine/src/cmd/template/types.rs`
- Modify: `crates/engine/src/cmd/template/mod.rs`
- Test: `crates/engine/src/cmd/template/tests.rs`

**Step 1: Write failing unit test for slot parsing**
Write unit tests in `crates/engine/src/cmd/template/tests.rs` verifying that strings like `#slot:text:title`, `#slot:image:hero`, `#slot:table:main`, `#slot:story:notes`, and legacy untyped `#slot:title` parse correctly into a strongly-typed `SlotKind`.

```rust
#[test]
fn test_parse_slot_tag() {
    assert_eq!(parse_slot_tag("#slot:text:title"), Some((SlotType::Text, "title")));
    assert_eq!(parse_slot_tag("#slot:image:hero"), Some((SlotType::Image, "hero")));
    assert_eq!(parse_slot_tag("#slot:table:main"), Some((SlotType::Table, "main")));
    assert_eq!(parse_slot_tag("#slot:story:body"), Some((SlotType::Story, "body")));
    // Untyped fallback defaults to Text
    assert_eq!(parse_slot_tag("#slot:title"), Some((SlotType::Text, "title")));
    // Non-slot items
    assert_eq!(parse_slot_tag("regular_frame"), None);
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test -p designcraft-engine test_parse_slot_tag`
Expected: FAIL (module not found).

**Step 3: Implement minimal slot parsing logic**
Implement `SlotType`, `SlotBinding`, and `parse_slot_tag` in `crates/engine/src/cmd/template/types.rs`.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotType {
    Text,
    Image,
    Table,
    Story,
}

pub fn parse_slot_tag(label: &str) -> Option<(SlotType, &str)> {
    let tag = label.trim();
    let rest = tag.strip_prefix("#slot:")?;
    if let Some((kind_str, key)) = rest.split_once(':') {
        let kind = match kind_str.to_ascii_lowercase().as_str() {
            "text" => SlotType::Text,
            "image" | "graphic" | "photo" => SlotType::Image,
            "table" => SlotType::Table,
            "story" | "rtf" => SlotType::Story,
            _ => SlotType::Text,
        };
        Some((kind, key.trim()))
    } else {
        Some((SlotType::Text, rest.trim()))
    }
}
```

**Step 4: Run test to verify it passes**
Run: `cargo test -p designcraft-engine test_parse_slot_tag`
Expected: PASS.

**Step 5: Commit**
```bash
git add crates/engine/src/cmd/template/
git commit -m "feat(engine): add slot contract parsing for template system"
```

---

## Task 2: Implement `template.page.apply` Command Core

**Files:**
- Create: `crates/engine/src/cmd/template/mod.rs`
- Modify: `crates/engine/src/cmd/mod.rs` (register `template.page.apply`)
- Test: `crates/engine/src/cmd/template/tests.rs`

**Step 1: Write failing test for template loading and page cloning**
Create a test document, save it as bytes with a tagged text frame `#slot:text:title` and a rectangle shape, then run `template.page.apply` to import it into a fresh document.

```rust
#[test]
fn test_template_page_apply_inserts_spread() {
    let mut s = Session::new();
    s.execute("document.create", &json!({"width": 595.28, "height": 841.89})).unwrap();

    // Create synthetic template document
    let tpl_bytes = create_synthetic_template("assets/test_template.designcraft");
    
    let res = s.execute("template.page.apply", &json!({
        "bytes": tpl_bytes,
        "position": "after",
        "target_page": 0,
        "slots": {
            "text": { "title": "Custom Publication Title" }
        }
    }));
    assert!(res.is_ok(), "template.page.apply should succeed: {:?}", res);
}
```

**Step 2: Run test to verify failure**
Run: `cargo test -p designcraft-engine test_template_page_apply_inserts_spread`
Expected: FAIL ("unknown command template.page.apply").

**Step 3: Implement `template.page.apply` engine command**
In `crates/engine/src/cmd/template/mod.rs`:
1. Parse arguments: `path` or `bytes`, `target_page` (or `position: before/after`), `slots`, `palette`.
2. Load template `Document` via `designcraft_format::load(&bytes)`.
3. Create new spread in active document with page dimensions matching the template page.
4. Deep-copy all items and stories from template spread into destination spread using item renumbering.
5. Register command spec in `crates/engine/src/cmd/mod.rs`.

**Step 4: Run test to verify it passes**
Run: `cargo test -p designcraft-engine test_template_page_apply_inserts_spread`
Expected: PASS.

**Step 5: Commit**
```bash
git add crates/engine/src/cmd/template/ crates/engine/src/cmd/mod.rs
git commit -m "feat(engine): implement template.page.apply command"
```

---

## Task 3: Text, Image, and Story Slot Injection

**Files:**
- Modify: `crates/engine/src/cmd/template/mod.rs`
- Test: `crates/engine/src/cmd/template/tests.rs`

**Step 1: Write failing test for slot substitution**
Verify that:
- Tagged text frames (`#slot:text:title`) have their story text replaced with the string while preserving paragraph styles.
- Tagged graphic frames (`#slot:image:hero`) adopt the new image asset and fit proportionally.
- Tagged story frames (`#slot:story:notes`) receive RTF or text file content.

```rust
#[test]
fn test_template_slot_substitution() {
    let mut s = Session::new();
    s.execute("document.create", &json!({})).unwrap();

    let res = s.execute("template.page.apply", &json!({
        "bytes": synthetic_cover_template(),
        "slots": {
            "text": { "title": "STATISTIK DAERAH", "year": "2026" },
            "image": { "hero": "outputs/city_skyline.jpg" }
        }
    })).unwrap();

    let doc = &s.active().unwrap().doc;
    // Verify story text changed
    let title_item = doc.items().find(|it| it.label == "#slot:text:title").unwrap();
    let text = doc.story_text(title_item.text_frame().unwrap().story).unwrap();
    assert_eq!(text.trim(), "STATISTIK DAERAH");
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test -p designcraft-engine test_template_slot_substitution`
Expected: FAIL (text not yet substituted).

**Step 3: Implement slot injection engine logic**
In `crates/engine/src/cmd/template/mod.rs`:
- Iterate over newly inserted items.
- For `SlotType::Text`: look up key in `slots["text"]`, replace story content via `story.replace(0..len, &new_val)`.
- For `SlotType::Image`: look up key in `slots["image"]`, place image file into frame, trigger `fitProportionally` and `centerContent`.
- For `SlotType::Story`: resolve RTF/Markdown path, import into text frame.

**Step 4: Run test to verify it passes**
Run: `cargo test -p designcraft-engine test_template_slot_substitution`
Expected: PASS.

**Step 5: Commit**
```bash
git add crates/engine/src/cmd/template/
git commit -m "feat(engine): support text, image, and story slot bindings in template engine"
```

---

## Task 4: Dynamic Palette / Swatch Re-tinting

**Files:**
- Modify: `crates/engine/src/cmd/template/mod.rs`
- Test: `crates/engine/src/cmd/template/tests.rs`

**Step 1: Write failing test for swatch re-tinting**
Verify that vector shapes using named swatches `Primary` (`#0072bc`) and `Accent` (`#f5ab19`) in the template are automatically re-tinted to user-specified hex colors (e.g. `Report Navy & Orange` `#082641` + `#f5871f`).

```rust
#[test]
fn test_template_palette_override() {
    let mut s = Session::new();
    s.execute("document.create", &json!({})).unwrap();

    s.execute("template.page.apply", &json!({
        "bytes": synthetic_cover_template(),
        "palette": {
            "Primary": "#082641",
            "Accent": "#f5871f"
        }
    })).unwrap();

    // Verify swatch color in document
    let active = s.active().unwrap();
    let swatch = active.doc.swatches.iter().find(|sw| sw.name == "Primary").unwrap();
    assert_eq!(swatch.color.to_hex(), "#082641");
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test -p designcraft-engine test_template_palette_override`
Expected: FAIL.

**Step 3: Implement swatch override logic**
In `crates/engine/src/cmd/template/mod.rs`:
- Accept `palette: HashMap<String, String>`.
- For each `(swatch_name, hex_color)`:
  - If target document has swatch with `swatch_name`, update color.
  - If not, create swatch with `swatch_name`.
  - Remap any direct item fill/stroke referring to that swatch.

**Step 4: Run test to verify it passes**
Run: `cargo test -p designcraft-engine test_template_palette_override`
Expected: PASS.

**Step 5: Commit**
```bash
git add crates/engine/src/cmd/template/
git commit -m "feat(engine): add palette swatch overrides to template engine"
```

---

## Task 5: Template Discovery & Fast Preview Extractor

**Files:**
- Create: `crates/engine/src/cmd/template/catalog.rs`
- Test: `crates/engine/src/cmd/template/tests.rs`

**Step 1: Write failing test for template catalog scanning**
Test scanning a directory of `.designcraft` template files, extracting their names, categories (from folder), and previews via `designcraft_format::read_preview()`.

```rust
#[test]
fn test_template_catalog_discovery() {
    let dir = tempfile::tempdir().unwrap();
    create_test_template_file(dir.path().join("001_wave.designcraft"));

    let catalog = scan_template_catalog(dir.path()).unwrap();
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[0].id, "001_wave");
    assert!(catalog[0].has_preview);
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test -p designcraft-engine test_template_catalog_discovery`
Expected: FAIL.

**Step 3: Implement template catalog scanner**
In `crates/engine/src/cmd/template/catalog.rs`:
- Recursively inspect directory for `*.designcraft`.
- Extract template ID from filename.
- Extract preview thumbnail bytes using `designcraft_format::read_preview()` without decoding full document.

**Step 4: Run test to verify it passes**
Run: `cargo test -p designcraft-engine test_template_catalog_discovery`
Expected: PASS.

**Step 5: Commit**
```bash
git add crates/engine/src/cmd/template/
git commit -m "feat(engine): implement template discovery and fast preview extraction"
```

---

## Task 6: Author Canonical Cover Templates (Styles 0–9)

**Files:**
- Create: `assets/templates/covers/` directory
- Populate:
  - `assets/templates/covers/000_wave_curves.designcraft`
  - `assets/templates/covers/001_geometric_medallion.designcraft`
  - `assets/templates/covers/002_folded_origami.designcraft`
  - `assets/templates/covers/003_chevron_medallion.designcraft` (from `design-1.designcraft`)
  - `assets/templates/covers/004_honeycomb_hexagon.designcraft` (from `design-2.designcraft`)
  - `assets/templates/covers/005_corporate_headline.designcraft` (from `design-3.designcraft`)
  - `assets/templates/covers/006_diagonal_arrow.designcraft` (from `design-4.designcraft`)
  - `assets/templates/covers/007_modern_horizon.designcraft` (from `design-5.designcraft`)
  - `assets/templates/covers/008_business_report.designcraft` (from `design-10.designcraft`)
  - `assets/templates/covers/009_urban_angle.designcraft` (from `design-6.designcraft`)
- Modify: `ASSETS.md` (add attribution rows for all new template assets)

**Step 1: Copy & tag frames in source reference designs**
Tag each frame in `design-1` through `design-10` with `#slot:text:title`, `#slot:text:year`, `#slot:image:hero`, `#slot:text:agency`, etc.

**Step 2: Verify assets attribution**
Run: `cargo xtask assets`
Expected: PASS.

**Step 3: Commit**
```bash
git add assets/templates/covers/ ASSETS.md
git commit -m "assets(templates): add canonical cover templates for Styles 0 through 9"
```

---

## Task 7: Author Interior Page Templates (Dividers, Tables, Ulasan)

**Files:**
- Create: `assets/templates/dividers/001_ulasan_chapter.designcraft`
- Create: `assets/templates/data_tables/001_single_table_standard.designcraft`
- Create: `assets/templates/front_matter/001_publication_note.designcraft`
- Modify: `ASSETS.md`

**Step 1: Tag slots in interior page templates**
- `001_ulasan_chapter.designcraft`:
  - `#slot:text:chapter_num`
  - `#slot:text:chapter_title`
  - `#slot:story:narrative_body`
- `001_single_table_standard.designcraft`:
  - `#slot:text:table_num`
  - `#slot:text:table_title`
  - `#slot:table:main`
  - `#slot:text:source`
- `001_publication_note.designcraft`:
  - `#slot:text:title`
  - `#slot:story:note_body`

**Step 2: Verify assets attribution**
Run: `cargo xtask assets`
Expected: PASS.

**Step 3: Commit**
```bash
git add assets/templates/ ASSETS.md
git commit -m "assets(templates): add canonical interior page templates for dividers, tables, and notes"
```

---

## Task 8: Refactor PubGen Cover Generation to Use Template Engine

**Files:**
- Modify: `crates/ui-egui/src/panels/pubgen.rs` (`execute_generate_cover`)
- Test: `crates/ui-egui/tests/` (run existing 639 tests)

**Step 1: Update `execute_generate_cover` to call `template.page.apply`**
Replace the 2,020-line hardcoded `match st.cover_style` with a call to `template.page.apply`:

```rust
fn execute_generate_cover(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    let tpl_path = resolve_cover_template_path(st.cover_style);
    
    app.run("template.page.apply", json!({
        "path": tpl_path,
        "position": "before",
        "target_page": 0,
        "slots": {
            "text": {
                "level": level_text,
                "title": title_text,
                "year": year_text,
                "volume": volume_text,
                "catalogue": cat_text,
                "code": code_text,
                "agency": agency_text,
            },
            "image": {
                "skyline": skyline,
                "landscape": landscape,
                "logo": logo,
            }
        },
        "palette": {
            "Primary": primary,
            "Accent": accent,
            "Dark": dark,
        }
    }))?;

    Ok(format!("Cover page generated (Style #{})", st.cover_style))
}
```

**Step 2: Run test suite**
Run: `cargo test -p designcraft-ui-egui --features pubgen`
Expected: PASS.

**Step 3: Commit**
```bash
git add crates/ui-egui/src/panels/pubgen.rs
git commit -m "refactor(pubgen): delegate cover generation to template engine"
```

---

## Task 9: Refactor PubGen Page Generation (Card 1) to Use Template Engine

**Files:**
- Modify: `crates/ui-egui/src/panels/pubgen.rs` (`execute_generate_pages`)
- Test: `crates/ui-egui/tests/`

**Step 1: Delegate `Ulasan`, `Note`, and layout presets to interior templates**
Update `execute_generate_pages` to call `template.page.apply` for chapter dividers (`ulasan`), publication notes (`note`), and table pages.

**Step 2: Run test suite**
Run: `cargo test -p designcraft-ui-egui --features pubgen`
Expected: PASS.

**Step 3: Commit**
```bash
git add crates/ui-egui/src/panels/pubgen.rs
git commit -m "refactor(pubgen): delegate interior page generation to template engine"
```

---

## Task 10: Full Quality Gates & CI Verification

**Step 1: Run comprehensive tests**
Run: `cargo test --features pubgen`
Expected: ALL PASS.

**Step 2: Run Clippy**
Run: `cargo clippy --features pubgen -- -D warnings`
Expected: 0 warnings.

**Step 3: Verify formatting and layering**
Run: `cargo fmt --check`
Run: `cargo xtask layers`
Run: `cargo xtask assets`
Expected: ALL PASS.

**Step 4: Final commit and summary**
```bash
git commit -m "chore(pubgen): complete template-driven system implementation"
```
