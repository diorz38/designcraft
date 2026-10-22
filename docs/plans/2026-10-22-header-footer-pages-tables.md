# Header/Footer frames for pages and tables

## Context

DesignCraft has the *machinery* for page headers/footers but not the feature. Users can build one
manually: create a parent page (`layout.parents.new`), draw text frames on it, type `PAGE_NUMBER` /
insert text variables, and apply the parent to pages (`layout.pages.applyParent`). The sample
document does exactly that (`crates/engine/src/sample.rs:261-277`). What is missing is InDesign's
named **Header/Footer frames** feature: pick a preset, choose single- or double-sided, and the app
places styled header/footer frames for you — re-editable afterwards.

Table header/footer rows already work end to end: `RowKind::{Header,Body,Footer}`,
`repeat_header`/`repeat_footer`, repeat-on-break in compose, and `table.options` accepts
`headerRows`/`footerRows`/`repeatHeader`/`repeatFooter`. The missing piece is purely UI: there is
no Table Options dialog, so a user cannot set the row counts or toggles without the panel fragment.

Outcome: a `Layout ▸ Header/Footer…` dialog (3 presets + custom, single/double-sided, page scope)
that generates parent-page text frames and remembers its settings, and a `Table ▸ Table Options…`
dialog exposing the existing header/footer row options.

## Model — `crates/doc/src/header_footer.rs` (new)

```rust
pub enum Sided { Single, Double }               // Double = left/right page variants
pub enum HfScope { All, Section(usize), Range { first: usize, last: usize } }  // absolute page idx

pub struct SidedText { pub single: String, pub left: String, pub right: String }

pub struct HeaderFooter {
    pub preset: String,        // "minimal" | "book" | "editorial" | "custom"
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
}
pub struct SidedItems { pub single: Option<ItemId>, pub left: Option<ItemId>, pub right: Option<ItemId> }
```

Serde: `#[derive(Serialize, Deserialize, PartialEq)]`, every field `#[serde(default, skip_serializing_if …)]`.
Add to `Document` (`crates/doc/src/lib.rs:628`): `pub header_footer: Option<HeaderFooter>` with
`#[serde(default, skip_serializing_if = "Option::is_none")]`, and `header_footer: None` to the
literal constructor at `crates/doc/src/build.rs:126`. Old files load as `None`; `Document::check()`
does not enumerate fields.

Template tokens expanded at apply: `{page}` → `story::PAGE_NUMBER` char, `{section}` →
`SECTION_MARKER`, `{filename}`/`{chapter}`/`{runningHeader}` → the `vars::var_char(i)` of the
matching default variable (`Document::variable(name)`, `crates/doc/src/vars.rs:284`).

**Do not** identify generated frames by `Item.label` — labels are user-writable and `remove_item`
does not scrub `Page.overridden`. Store `ItemId`s in the spec (same pattern as
`Document.endnote_story`, `Toc.story`, `IndexSpec.story`) and rewrite text in place.

## Engine — `crates/engine/src/cmd/header_footer.rs` (new)

Register in `crates/engine/src/cmd/mod.rs` (`v.extend(header_footer::specs())` near `layout`).

1. **`layout.headerFooter.get`** (`query`) — returns the stored spec + resolved page count and
   the current page, or `null`.
2. **`layout.headerFooter.apply`** — params: the spec fields +
   `applyToPages?: bool` (default true). One `s.edit(...)`:
   - Resolve target parent spread: `doc.header_footer.parent` if it still exists in
     `doc.parents`; else the first parent whose `ParentInfo.name == "Header/Footer"`; else
     `doc.add_parent("H", "Header/Footer", columns, gutter, current margins)`
     (`crates/doc/src/build.rs:238`). Store the id back in the spec.
   - Validate the parent has ≥1 page; for `Sided::Double` require a `Left` **and** a `Right` page.
     **Never use page index 0 as "left"** — `add_parent` reverses order for RTL binding
     (`build.rs:244-246`). Select by `PageSide`.
   - Ensure the paragraph style exists: push a `ParagraphStyle` directly into
     `doc.styles_mut().paragraph` (name from `Styles::unique_name`,
     `crates/doc/src/styles.rs:846`) inside the same edit; fall back to `story::BASIC_PARAGRAPH`.
     Do not call `style.paragraph.create` from inside the edit (double borrow).
   - For each needed side (single: the single parent page; double: the `Left` page and the
     `Right` page): if the spec already holds an `ItemId` for that side and the item still exists
     on the parent spread → replace the story text via `Document::story_mut` (set text, then
     `Story::format_paras(0..0, |p| p.style = style)`, `crates/doc/src/story.rs:459`) and update
     `Item.path` via `item_mut`; else `doc.add_text_frame(SpreadRef::Parent(pi), rect, layer,
     text, ParaFormat { style, .. })` (`crates/doc/src/edit.rs:271`), record the id in the spec.
   - Geometry: `page.bounds()` + `page.left_right_margins()` give the horizontal band; header rect
     top = `bounds.y0 + header_offset`; footer rect bottom = `bounds.y1 - footer_offset`. Height
     comes from auto-size, not a fixed band. Apply `TextFrameOptions { auto_size: HeightOnly,
     auto_size_ref: 0..2 (header, grows down) / 6..8 (footer, grows up), auto_size_use_min_height:
     true, auto_size_min_height: <one line> }` **after** `add_text_frame` via
     `item_mut(id)?.text_frame_mut()`, because `add_text_frame` discards style-driven options
     (`edit.rs:278-284`).
   - Header and footer must be in **separate stories** (auto-size only resizes the last frame of a
     thread — `crates/engine/src/autosize.rs:108-114`).
   - Spine-aware alignment: set physical `Align::Left` on right-hand pages and `Align::Right` on
     left-hand pages (compose hardcodes `left_page = false` for parent spreads,
     `crates/compose/src/lib.rs:685`, so `TowardsSpine` mirrors wrong in parent view).
   - If `sided == Single`, delete the left/right items (and vice versa) via `doc.remove_item`.
   - `rule`: with the preset flag on, also create/refresh a hairline `Item::new(... Shape::GraphicLine ...)`
     per side, tagged in the spec the same way (add `rule_items: SidedItems`).
   - Store the spec in `doc.header_footer`.
   - If `applyToPages`: compute absolute page indices for `scope` (Section: pages from
     `Section.start` to `min_by_key(next start) - 1`, else `page_count()` — replicate
     `crates/doc/src/vars.rs:259-266`; sections are not guaranteed sorted), clamp to
     `0..page_count`, and call `doc.apply_parent(&pages, Some(parent_id))`
     (`crates/doc/src/build.rs:643`).
3. **`layout.headerFooter.remove`** — remove generated items, `apply_parent(&pages, None)` for the
   scope, clear `doc.header_footer`.

Presets (code constants, same precedent as `PRESETS` in `crates/engine/src/cmd/file.rs:16`):

| preset | header | footer | sided default |
|---|---|---|---|
| minimal | `{filename}` left, `{page}` right | — | Single |
| book | `{chapter}` left, `{page}` right | — | Double |
| editorial | `{runningHeader}` left, `{section} {page}` right | `{filename}` left | Double |

## UI — `crates/ui-egui/src/header_footer.rs` (new)

Model on `crates/ui-egui/src/section_options.rs` (open / body / confirm, `ACTIONS_W` + `BODY_H`,
OK/Cancel in a side column):

- Preset radio list (3 predefined + `Custom`); picking a preset seeds sided + templates, and
  `Custom` leaves the text fields editable.
- Single-sided / Double-sided radios (`ui.radio` pattern at `section_options.rs:154-160`).
- Page scope: All / This Section / Range + two number fields, enabled per choice.
- Header offset, footer offset (measure fields via `d.pt(key)`), paragraph style combo (reuse
  `style_menu` from `section_options.rs:105`), `rule` checkbox.
- Header/Footer template fields — one pair for single-sided, two pairs (left page / right page)
  for double-sided. Show the token legend (`{page} {section} {chapter} {filename}
  {runningHeader}`).
- Validation in confirm: offsets ≥ 0, header band must not cross the footer band, style must
  exist or be `[Basic Paragraph]`; on error keep the dialog open with `status` (pattern at
  `dialogs.rs:3050-3060`).

Wiring:
- `crates/ui-egui/src/dialogs.rs`: default fields in `Dialog::new`, title arm, body arm
  (`"headerFooter" => crate::header_footer::body(...)`), confirm arm.
- `crates/ui-egui/src/menus.rs`: `("app.headerFooterDialog", "Header/Footer…", None, "{}")` in
  `UI_COMMANDS`, `"ui:app.headerFooterDialog"` in the Layout list after `cmd:layout.section`
  (`menus.rs:425`), and a dispatch arm beside the `layout.section` special case (`menus.rs:2514`).
- Also add `"Header/Footer…"` to the Pages panel page context menu
  (`crates/ui-egui/src/panels/pages.rs` `page_menu()`), beside "Numbering & Section Options…".

## Table Options dialog

New dialog id `"tableOptions"` (same file or `crates/ui-egui/src/table_options.rs`), seeded from
the existing `table.get` query (`crates/engine/src/cmd/table.rs:406` → `headerRows`, `footerRows`,
`options.repeatHeader`, `options.repeatFooter`), applying `table.options`
(`crates/engine/src/cmd/table.rs:359`):

- Header Rows / Footer Rows number fields (whole numbers, 0..n−1; command already auto-adds rows).
- Repeat Header Rows / Repeat Footer Rows checkboxes.
- Disabled like Cell Options (needs a caret in a table).

Menu: `Table` menu (`crates/ui-egui/src/menus.rs:710-748`) — add `"ui:app.tableOptionsDialog"` just
before the `Cell Options` submenu.

## i18n

New strings fall back to English automatically (`crates/ui-egui/src/i18n.rs:4462`). Add rows to the
shared `TABLE` (`i18n.rs:40`) for the core labels (Header/Footer…, Single-sided, Double-sided,
Header Rows, Footer Rows, Repeat Header Rows, Repeat Footer Rows, Preset, Page Scope) and to
`crates/ui-egui/src/i18n/id.rs` for Indonesian, matching the branch's bilingual direction. No
completeness requirement.

## Verification

1. `cargo xtask ci` (fmt, clippy -D warnings, tests, assets, layers, wasm).
2. Engine tests in `crates/engine/src/cmd/header_footer.rs` (module `#[cfg(test)]`), synthetic:
   - apply on a 4-page facing doc creates one parent with header/footer frames; `doc.header_footer`
     is `Some`; page markers resolve on document pages, `#` on the parent view.
   - double-sided: left/right parent pages each carry their variant; single-sided removes the other.
   - re-apply after changing a template replaces text in place — item ids unchanged.
   - `layout.headerFooter.remove` leaves no generated items and clears parent application.
   - section scope covers only that section's pages (compute both bounds).
   - RTL binding: left page found by `PageSide`, not index.
3. Manual UI check: `cargo run --release -p designcraft -- --sample --control 7979`, then
   `{"id":1,"method":"engine.execute","params":{"command":"layout.headerFooter.apply","params":{...}}}`
   and `{"id":2,"method":"ui.screenshot","params":{"path":"/tmp/hf.png"}}`; read the PNG and compare
   the header/footer placement against `plan/indesign/02-ui-ux.md`.
4. Table: place caret in a table, open `Table ▸ Table Options…`, set header rows to 2 and toggle
   repeat; verify `table.get` reports the new values and that a table breaking across frames repeats
   the header (`crates/compose/src/table.rs:330-335`).
5. Docs: add a parity-checklist row for Header/Footer frames, update `docs/gaps.md:141` (remove the
   Edit Header/Footer gap for pages; keep the table Cell Options gap), add a `ROADMAP.md` progress
   line.

## Skipped / deferred

- Live preview pane inside the dialog (use the canvas instead).
- Per-section distinct header content beyond scope selection — tokens cover it.
- Full Table Options (Row Strokes, Column Strokes, Alternating Fills) — the panel already covers
  alternating fills; add only if users ask.
