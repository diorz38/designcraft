# Header/Footer Frames for Pages and Tables — Implementation Plan

## Context

DesignCraft has the underlying *machinery* for page headers/footers but not the user-facing feature.
Users can build one manually: create a parent page (`layout.parents.new`), draw text frames on it,
type `PAGE_NUMBER` / insert text variables, and apply the parent to pages (`layout.pages.applyParent`).
The sample document does exactly that (`crates/engine/src/sample.rs:261-277`). What is missing is InDesign's
named **Header/Footer frames** feature: pick a preset, choose single- or double-sided, and the app
places styled header/footer frames for you — re-editable afterwards.

Table header/footer rows already work end to end: `RowKind::{Header, Body, Footer}`,
`repeat_header`/`repeat_footer`, repeat-on-break in compose, and `table.options` accepts
`headerRows`/`footerRows`/`repeatHeader`/`repeatFooter`. The missing piece is purely UI: there is
no Table Options dialog, so a user cannot set row counts or repeat toggles without raw commands.

**Outcome:**
1. A `Layout ▸ Header/Footer…` dialog (3 presets + custom, single/double-sided, page scope) that
   generates parent-page text frames and remembers its settings per document.
2. A `Table ▸ Table Options…` dialog exposing header/footer row counts and repeat-on-break toggles.
3. Quick Actions shortcuts in Pubgen Card 3 for both dialogs.

---

## Technical Specifications (Reference Architecture)

### Data Model — `crates/doc/src/header_footer.rs` (new)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sided { Single, Double }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HfScope {
    All,
    Section(usize),
    Range { first: usize, last: usize },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidedText {
    pub single: String,
    pub left: String,
    pub right: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidedItems {
    pub single: Option<ItemId>,
    pub left: Option<ItemId>,
    pub right: Option<ItemId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderFooter {
    pub preset: String,             // "minimal" | "book" | "editorial" | "custom"
    pub sided: Sided,
    pub scope: HfScope,
    pub parent: Option<SpreadId>,   // the parent spread this created/owns
    pub header_offset: f64,         // pt, page top edge → header frame top
    pub footer_offset: f64,         // pt, footer frame bottom → page bottom edge
    pub header: SidedText,          // templates, tokens expanded at apply
    pub footer: SidedText,
    pub style: String,              // paragraph style name
    pub rule: bool,                 // hairline rule preset flag
    pub header_items: SidedItems,   // created ItemIds per side (regenerated, not recreated)
    pub footer_items: SidedItems,
    pub rule_items: SidedItems,     // created graphic line ItemIds per side
}
```

Serde rules: Every field carries `#[serde(default, skip_serializing_if = "…")]`. Added to `Document`
(`crates/doc/src/lib.rs:628`): `pub header_footer: Option<HeaderFooter>` with
`#[serde(default, skip_serializing_if = "Option::is_none")]`, and `header_footer: None` in
literal constructor at `crates/doc/src/build.rs:126`.

### Presets

| Preset | Sided Default | Header Template | Footer Template | Hairline Rule |
|---|---|---|---|---|
| **minimal** | Single | `{filename}\u{E006}{page}` | — | false |
| **book** | Double | Recto: `{chapter}\u{E006}{page}`<br>Verso: `{page}\u{E006}{chapter}` | — | false |
| **editorial** | Double | Recto: `{runningHeader}\u{E006}{section} {page}`<br>Verso: `{page} {section}\u{E006}{runningHeader}` | `{filename}` | true |

Tokens:
- `{page}` → `story::PAGE_NUMBER` (`\u{E000}`)
- `{section}` → `story::SECTION_MARKER` (`\u{E001}`)
- `{filename}` → `"File Name"` (`Document::variable`)
- `{chapter}` → `"Chapter Number"` (`Document::variable`)
- `{runningHeader}` → `"Running Header"` (`Document::variable`)
- `\u{E006}` → `story::RIGHT_INDENT_TAB` (flushes subsequent text to the right margin of the frame)

---

## Phased Implementation Plan

### Phase 1: Model & Document State (`crates/doc`)

**Goal:** Establish the data structures for Header/Footer specifications, wire them into `Document`,
and ensure clean Serde serialization/deserialization with zero regression on older files.

- **Files:**
  - Create: `crates/doc/src/header_footer.rs`
  - Modify: `crates/doc/src/lib.rs` (register module, add field to `Document`)
  - Modify: `crates/doc/src/build.rs` (literal constructor `header_footer: None`)
  - Test: `crates/doc/src/header_footer.rs` (inline unit tests)

- **Steps:**
  1. Define `Sided`, `HfScope`, `SidedText`, `SidedItems`, and `HeaderFooter` in `crates/doc/src/header_footer.rs`.
  2. Implement Serde annotations (`camelCase`, `skip_serializing_if`).
  3. Export types in `crates/doc/src/lib.rs` and add `pub header_footer: Option<HeaderFooter>` to `Document`.
  4. Initialize `header_footer: None` in `crates/doc/src/build.rs:126`.
  5. Add unit tests for JSON serialization, deserialization, and backward compatibility.

- **Acceptance Criteria:**
  - [x] `cargo test -p designcraft-doc` passes with 0 failures.
  - [x] JSON round-trip test verifies all `HeaderFooter` fields serialize with camelCase keys.
  - [x] Deserializing an older `.designcraft` JSON document lacking `headerFooter` loads successfully with `header_footer == None`.
  - [x] `Document::check()` passes without error on documents with `header_footer` populated.

---

### Phase 2: Engine Commands & Token Expansion (`crates/engine`)

**Goal:** Implement engine commands for retrieving, applying, and removing page headers/footers with
proper parent page inheritance, token expansion, and auto-sizing.

- **Files:**
  - Create: `crates/engine/src/cmd/header_footer.rs`
  - Modify: `crates/engine/src/cmd/mod.rs` (register command specs)
  - Test: `crates/engine/src/cmd/header_footer.rs` (`mod tests`)

- **Steps:**
  1. **Token Expander:** Map `{page}` → `\u{E000}`, `{section}` → `\u{E001}`, `{filename}` → `"File Name"`,
     `{chapter}` → `"Chapter Number"`, `{runningHeader}` → `"Running Header"`. Safely handle missing variables without `unwrap()`.
  2. **`layout.headerFooter.get`:** Query command returning stored spec + resolved page count and active page index.
  3. **`layout.headerFooter.apply`:**
     - Resolve target parent spread: `doc.header_footer.parent` if still exists; else first parent named `"Header/Footer"`;
       else `doc.add_parent("H", "Header/Footer", columns, gutter, margins)`.
     - **Parent Inheritance:** Set created parent's `based_on` to `doc.parents.first().map(|p| p.id)` so pages don't lose existing master items.
     - **Validation:** Check `facing_pages` when `Sided::Double` is selected; select left/right by `PageSide` (RTL-safe).
     - **Paragraph Style:** Ensure style exists in `doc.styles_mut().paragraph` (or fallback to `story::BASIC_PARAGRAPH`).
     - **Item Creation / Update:** For each side, if `ItemId` already exists, update text via `st.replace(0..st.text.len(), &text)`
       and format paras with style; else call `doc.add_text_frame(SpreadRef::Parent(pi), rect, layer, text, para)`.
     - **Geometry & Auto-size:** Rect from `page.bounds()` + `page.left_right_margins()`. Set `TextFrameOptions`
       `auto_size: AutoSize::HeightOnly`, `auto_size_ref: 0` (header) / `6` (footer). Separate stories for header and footer.
     - **Hairline Rule:** When `rule: true`, create/update `Shape::GraphicLine` items recorded in `rule_items: SidedItems`.
     - **Scope Application:** Compute page range for `All`, `Section`, or `Range`, and call `doc.apply_parent(&pages, Some(parent_id))`.
  4. **`layout.headerFooter.remove`:** Remove generated text/rule items via `doc.remove_item`, clear parent from scope pages via `apply_parent(&pages, None)`, and set `doc.header_footer = None`.

- **Acceptance Criteria:**
  - [x] `cargo test -p designcraft-engine -- header_footer` passes.
  - [x] `layout.headerFooter.apply` on a 4-page facing document creates parent with header/footer frames; page markers resolve on doc pages and `#` on parent.
  - [x] Double-sided mode creates distinct left/right variants; switching to single-sided cleanly removes opposite items without leaking orphaned frames.
  - [x] Re-applying after changing a template string replaces text in place (`st.replace`) preserving item IDs and story invariants.
  - [x] `layout.headerFooter.remove` removes all generated text frames and rules and resets parent assignment on scoped pages.
  - [x] RTL binding documents correctly locate Left and Right pages by `PageSide` rather than index 0.

---

### Phase 3: Table Options Dialog (`crates/ui-egui`)

**Goal:** Provide an InDesign-style Table Options dialog enabling users to configure header/footer row
counts and repeat-on-break toggles from the UI.

- **Files:**
  - Create: `crates/ui-egui/src/table_options.rs` (or add to `dialogs.rs`)
  - Modify: `crates/ui-egui/src/dialogs.rs` (register `"tableOptions"` dialog arm and confirm dispatcher)
  - Modify: `crates/ui-egui/src/menus.rs` (add `"ui:app.tableOptionsDialog"` before `Cell Options`)
  - Test: `crates/ui-egui/src/dialogs.rs` (harness test for table options)

- **Steps:**
  1. Create `open_table_options(app)`: seeds dialog from `table.get` query (`headerRows`, `footerRows`, `repeatHeader`, `repeatFooter`).
  2. Build dialog UI:
     - Header Rows number field (`0..n-1`).
     - Footer Rows number field (`0..n-1`).
     - Repeat Header Rows checkbox (`repeatHeader`).
     - Repeat Footer Rows checkbox (`repeatFooter`).
  3. **Caret Gating:** When opened with no table active, display guided status banner:
     *"Place insertion point in a table to edit Table Options."*
  4. Implement confirm handler in `dialogs.rs`: calls `table.options` with new row counts and repeat flags.
  5. Add `"ui:app.tableOptionsDialog"` to Table menu before `>Cell Options`.

- **Acceptance Criteria:**
  - [x] Opening `Table ▸ Table Options…` with cursor in a table correctly seeds current header/footer counts and repeat toggles.
  - [x] Changing Header Rows to 2 and toggling Repeat Header Rows calls `table.options` and updates table structure.
  - [x] When a table breaks across threaded frames, header rows repeat on continuation frames as verified by `compose/src/table.rs`.
  - [x] Opening the dialog when cursor is not in a table shows the guided status banner without error or panic.

---

### Phase 4: Header/Footer Dialog & UI Wiring (`crates/ui-egui`)

**Goal:** Implement the interactive `Layout ▸ Header/Footer…` dialog with preset selection, token legend,
scope controls, and wiring across menus and panel context menus.

- **Files:**
  - Create: `crates/ui-egui/src/header_footer.rs`
  - Modify: `crates/ui-egui/src/dialogs.rs` (register `"headerFooter"`, confirm dispatcher)
  - Modify: `crates/ui-egui/src/menus.rs` (register `UI_COMMANDS`, Layout menu entry)
  - Modify: `crates/ui-egui/src/panels/pages.rs` (add to `page_menu()` context menu)
  - Test: `crates/ui-egui/src/dialogs.rs` (harness test for header/footer dialog)

- **Steps:**
  1. Build dialog body following `section_options.rs` pattern (`ACTIONS_W` + `BODY_H`, side-by-side action column):
     - Preset radio list: Minimal, Book, Editorial, Custom (seeds sided, templates, offsets, and rule).
     - Sided radios: Single-sided / Double-sided (disabled when `!facing_pages`).
     - Page Scope: All, This Section, Range (with start/end number inputs).
     - Geometry: Header offset, footer offset (`d.pt(key)`), rule checkbox.
     - Paragraph style dropdown menu.
     - Template input fields: 1 pair for single-sided, 2 pairs (left/right) for double-sided.
     - Token legend banner: `{page}  {section}  {chapter}  {filename}  {runningHeader}`.
  2. Implement confirm validation in `dialogs.rs`:
     - Validate offsets ≥ 0 and `header_offset + footer_offset < page_height`.
     - On error, retain dialog open with `status` error banner.
     - On success, dispatch `layout.headerFooter.apply`.
  3. Register UI command `app.headerFooterDialog` in `menus.rs` and add `"ui:app.headerFooterDialog"` in Layout menu after `cmd:layout.section`.
  4. Add `"Header/Footer…"` to Pages panel page context menu (`page_menu()`) beside `"Numbering & Section Options…"`.

- **Acceptance Criteria:**
  - [x] `Layout ▸ Header/Footer…` menu item opens the dialog with seeded document state.
  - [x] Right-clicking a page in the Pages panel shows `"Header/Footer…"` and opens the dialog.
  - [x] Selecting presets (`Minimal`, `Book`, `Editorial`) updates template fields and controls accordingly.
  - [x] Setting invalid geometry (e.g. negative offset or header/footer overlap) keeps dialog open with validation message.
  - [x] Clicking OK successfully executes `layout.headerFooter.apply` and updates canvas render.

---

### Phase 5: Pubgen Quick Actions & Localization (`crates/ui-egui`)

**Goal:** Add direct shortcut buttons to Pubgen Card 3 and register all UI labels in the i18n translation table.

- **Files:**
  - Modify: `crates/ui-egui/src/panels/pubgen.rs` (`card3_quick_actions_ui`)
  - Modify: `crates/ui-egui/src/i18n.rs` (`TABLE`)
  - Test: `crates/ui-egui/src/panels/pubgen.rs` (UI harness tests)

- **Steps:**
  1. In `card3_quick_actions_ui` (`pubgen.rs:3840-3864`), add one row of two `outline_button`s (`Header/Footer…` and `Table Options…`) reusing the `hw` half-width layout below Facing/Add Blank Page.
  2. Open dialogs directly with `Dialog::new("headerFooter", json!({}))` and `Dialog::new("tableOptions", json!({}))`.
  3. Register new label keys in `crates/ui-egui/src/i18n.rs` (`TABLE`):
     - `"Header/Footer…"`
     - `"Table Options…"`
     - `"Single-sided"`
     - `"Double-sided"`
     - `"Header Rows"`
     - `"Footer Rows"`
     - `"Repeat Header Rows"`
     - `"Repeat Footer Rows"`
     - `"Preset"`
     - `"Page Scope"`

- **Acceptance Criteria:**
  - [x] Pubgen panel Card 3 renders both `Header/Footer…` and `Table Options…` buttons with proper hover tooltips.
  - [x] Clicking Card 3's `Header/Footer…` button opens the Header/Footer dialog seeded with live document state.
  - [x] Clicking Card 3's `Table Options…` button opens the Table Options dialog seeded with table under cursor.
  - [x] All new labels translate across German, French, Spanish, Japanese, Simplified Chinese, and fall back cleanly to English.

---

### Phase 6: End-to-End Verification, CI Quality Gates & Docs

**Goal:** Ensure all quality gates pass without warnings or regressions, verify visually against InDesign standards, and update roadmap/parity documentation.

- **Files:**
  - Modify: `docs/parity-checklist.md`
  - Modify: `docs/gaps.md`
  - Modify: `ROADMAP.md`

- **Steps:**
  1. Run `cargo xtask ci` (formatting, clippy `-D warnings`, tests, asset tracking, layering, wasm build).
  2. Run headless sample test:
     ```bash
     cargo run -p designcraft-cli -- run --sample --cmd 'layout.headerFooter.apply={"preset":"editorial"}' --export outputs/hf_test.png
     ```
  3. Compare generated output against `plan/indesign/02-ui-ux.md`.
  4. Update `docs/parity-checklist.md` (check off Header/Footer frames).
  5. Update `docs/gaps.md` (close Edit Header/Footer gap).
  6. Add entry to `ROADMAP.md` progress log.

- **Acceptance Criteria:**
  - [x] `cargo xtask ci` passes completely (fmt, clippy, tests, assets, layers, wasm).
  - [x] `cargo xtask layers` confirms 0 layering violations.
  - [x] Exported document PNG renders header, footer, page number, and hairline rules in expected positions.
  - [x] `docs/parity-checklist.md` passes `cargo xtask parity`.

---

## Implementation Findings & Caveats (Verified against Codebase)

1. **Text Variable Name Mapping**:
   InDesign/DesignCraft predefined variables in `crates/doc/src/vars.rs:104-114` are named `"File Name"`,
   `"Chapter Number"`, and `"Running Header"`. `Document::variable(name)` checks exact name equality.
   Tokens `{filename}`, `{chapter}`, `{runningHeader}` must explicitly map to these exact names.
   Missing variables or invalid indices must be handled without `unwrap()`.

2. **Opposite Ends on a Single Line (`RIGHT_INDENT_TAB`)**:
   In single-sided pages or single-frame full-width headers/footers, placing text on both left and
   right margins requires `story::RIGHT_INDENT_TAB` (`\u{E006}`). `crates/compose/src/lib.rs:3283`
   natively flushes subsequent text to the right margin of the frame.

3. **Facing Pages vs. Single-Sided Documents**:
   `doc.add_parent()` in `crates/doc/src/build.rs:239` creates a 2-page spread only when
   `doc.settings.facing_pages` is `true`. On non-facing documents, it generates 1 page (`PageSide::Single`).
   `Sided::Double` must be disabled in the UI or validated with a user-facing error message when
   facing pages is off.

4. **Parent Page `based_on` Chain (Preventing Graphic/Guide Loss)**:
   Calling `doc.apply_parent(&pages, Some(header_parent_id))` replaces `page.parent`. If `H-Header/Footer`
   has `based_on: None`, existing parent items (background graphics, page borders, guides) on `A-Parent`
   will not be drawn. Setting `based_on: doc.parents.first().map(|p| p.id)` preserves existing parent
   items via the renderer's `doc.shown_parent_pages` chain (`crates/doc/src/build.rs:779`).

5. **Safe Story Text Mutation**:
   `Story` enforces strict invariants between `st.text`, `st.chars`, and `st.paras` (`Story::check()`).
   Replacing story text in place must use `st.replace(0..st.text.len(), &expanded_text)`
   (`crates/doc/src/story.rs:418`), never direct string assignment.

6. **Parent Spread `left_page = false` in Compose**:
   In `crates/compose/src/lib.rs:682-686`, parent spreads hardcode `left_page = false`. As a result,
   `Align::TowardsSpine` and `Align::AwayFromSpine` do not mirror dynamically on parent spreads.
   Using physical alignment (`Align::Left` for recto folio, `Align::Right` for verso folio, or
   `RIGHT_INDENT_TAB`) ensures identical appearance in both parent editing and document page views.

7. **Hairline Rule Tracking**:
   Graphic lines created when `rule: true` must be recorded in `rule_items: SidedItems` so that
   switching presets or toggling `rule: false` removes or refreshes the lines rather than leaving
   orphaned graphic items on the parent spread.

8. **Table Options Dialog Context**:
   `table.options` and `table.get` require an active insertion point in a table (`in_table`).
   When opened via the Pubgen Card 3 Quick Actions button without a table selected, the dialog
   body must display the guided message `"Place insertion point in a table to edit Table Options"`
   instead of failing silently.

---

## Skipped / Deferred

- Live preview pane inside the dialog (use the canvas instead).
- Per-section distinct header content beyond scope selection — tokens cover it.
- Full Table Options (Row Strokes, Column Strokes, Alternating Fills) — the panel already covers
  alternating fills; add only if users ask.
