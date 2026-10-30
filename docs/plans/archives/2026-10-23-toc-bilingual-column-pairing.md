# TOC Bilingual Column Pairing (ID / EN on one row) — Implementation Plan

> **Date:** 2026-10-23
> **Status:** implemented 2026-10-23 (Phases 1–3 + guide/ROADMAP; `pairSeparator` stored but UI-less as planned)
> **Scope:** `crates/doc/src/lib.rs` (TocEntry), `crates/engine/src/cmd/toc.rs` (collect/pair), `crates/ui-egui/src/panels/pubgen.rs` (state, TocPlan, TOC card), docs
> **Reference:** `docs/plans/2026-10-22-bilingual-id-en-pubgen.md` (bilingual RXX layout), `crates/engine/src/cmd/toc.rs` (TOC engine)

## 0. Problem

A dual-lang review page (`RXX` with `file_en`, `bilingual_output = true`) is one spread with two
stories: the Indonesian RTF flows into a left column frame, the English RTF into a right column
frame (`pubgen.rs:5906-5981`). Both RTFs carry the same heading style names (`Heading 1`,
`Heading 2`), so `toc.generate` lists one row per paragraph and the reader sees four rows for
page 4:

    Proporsi fisik daratan ........ 4
    Analisis metrik jarak ........ 4
    Physical proportions of the landmass ........ 4
    Distance metric analysis ........ 4

Wanted: one row per heading pair, joined the same way bilingual table titles already join
(`" / "`, `pubgen_locale.rs::split_pair`):

    Proporsi fisik daratan / Physical proportions of the landmass ........ 4
    Analisis metrik jarak / Distance metric analysis ........ 4

## 0.1 Verified current state (anchors)

| Fact | Anchor |
|---|---|
| `TocEntry` fields + serde; persisted inside `Toc.entries`, so per-entry options survive rebuilds | `doc/src/lib.rs:746-796` |
| Engine collects rows per story independently; sort key `(abs, y, x)` interleaves rows by baseline, so ID/EN rows may group by column or interleave | `engine/src/cmd/toc.rs:201-265` (`collect`), key at `:259` |
| Rows are written from `Vec<Found>` in `fill`; `sort_entries` runs after `collect` | `engine/src/cmd/toc.rs:339-353` |
| `toc.entries` query shares `collect` + `sort_entries` | `engine/src/cmd/toc.rs:53-59` |
| `chapterNumber` prefixes the entry text with the page's first text box during `collect` (both ID and EN rows on the page would get the same `"BAB 01"` prefix) | `engine/src/cmd/toc.rs:245-253` |
| Entry params parsed/validated in `entries_param` (name caps, one-line separator) | `engine/src/cmd/toc.rs:82-135` |
| `TocPlan::from_state` builds the Paragraph-TOC levels: `TOC_NARRATIVE_STYLE` (level 1), `Heading 1` (2), `Heading 2` (3) | `pubgen.rs:8273-8298` |
| `TocPlan::from_toc` rebuilds from stored `Toc.entries`; `TocLevel` carries style/level/words/chapter_number only | `pubgen.rs:8302-8320` |
| `entries_json` emits `style/level/entryStyle/pageNumber/separator/pageTitle/chapterNumber` | `pubgen.rs:8373-8396` |
| TOC card checkboxes live at | `pubgen.rs:4694-4695` (raw English labels, no i18n) |
| Bilingual RXX layout: `left_rect`/`right_rect` frames, ID RTF left, EN RTF right, header frames full width | `pubgen.rs:5906-5981` |
| The sheet name `RXX [Dual Lang]` is **not** parsed anywhere; dual-lang comes from `bilingual_output` + `file_en` | `pubgen_excel.rs` (no such suffix) |
| `Found` is engine-internal (`pub struct Found`), fields `level/text/page/entry/key` | `engine/src/cmd/toc.rs:191-197` |
| Existing TOC tests to model after: engine `toc_lists_headings_in_page_order_and_updates`; pubgen `test_toc_paragraph_lists_a_generated_review_page_with_its_chapter_line` | `toc.rs:479`, `pubgen.rs:12023` |

**Non-goals:** no sheet-name `[Dual Lang]` suffix parsing; no change to Table/Chart TOC kinds; no
i18n phase (TOC card labels are plain English strings today, consistent with neighbouring rows).

---

## Phase 1: Data model — `TocEntry::pair_columns` + `pair_separator` (`crates/doc`)

**Goal:** persist the pairing options per entry so `TocPlan::from_toc` rebuilds keep them.

- **Files:**
  - Modify: `crates/doc/src/lib.rs` (`TocEntry`, `TocEntry::new`, tests)

- **Changes:**
  - `TocEntry` gains:
    ```rust
    /// Pair this style's left-column and right-column headings on one page into one row
    /// ("Indonesian / English"). Rows whose frames overlap horizontally never pair.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pair_columns: bool,
    /// Text between the paired halves; `None` is " / ".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pair_separator: Option<String>,
    ```
  - `TocEntry::new` initializes both defaults (`false` / `None`).
  - Serde names are camelCase (`pairColumns`, `pairSeparator`), consistent with `pageTitle`.

- **Acceptance Criteria:**
  - [ ] Serde roundtrip test: `TocEntry` with both fields set survives `to_value` → `from_value` unchanged; defaults serialize to nothing (old documents and old TOC JSON parse unchanged).

---

## Phase 2: Engine — pair left/right column headings (`crates/engine/src/cmd/toc.rs`)

**Goal:** `fill` and `toc.entries` merge same-page, same-level left/right column headings into one
row before styling and sorting.

- **Files:**
  - Modify: `crates/engine/src/cmd/toc.rs` (`Found`, `collect`, new `pair_columns`, `fill`, `toc.entries`, `entries_param`, tests)

- **Changes:**
  1. `Found` gains `frame: ItemId`, `x0: f64`, `x1: f64` (the heading's frame bounds, spread
     coords — `collect` already has `item.bounds()` at `:230`), `column: usize`
     (`0` when the frame center is left of its page's center, else `1`; page bounds via
     `d.spreads[si].pages[in_spread]`), and `prefix_len: usize` (chars the `chapterNumber`
     enrichment added to `text`, so the pair merge can strip the duplicate `"BAB 01"` from the
     English half).
  2. New `fn pair_columns(found: Vec<Found>, entries: &[TocEntry]) -> Vec<Found>`, run in `fill`
     between `collect` and `sort_entries`, and in the `toc.entries` query:
     - No-op unless some entry has `pair_columns`.
     - Group rows by `(abs page, level, entry index)` — the abs page is `key.0`.
     - Within a group, candidate pairs are (left, right) rows whose frames are horizontally
       disjoint: `left.x1 <= right.x0 + 0.5`. Full-width frames (table/chart pages) overlap their
       neighbours and never pair — the guard keeps other TOC kinds safe if the option is ever set
       on them.
     - Among the disjoint pairs take the maximal pairing greedy by vertical order: sort lefts and
       rights by `key` (y then x), pair k-th with k-th while the disjoint guard holds; a left with
       no remaining right stays a single row, and vice versa (an English heading without its
       Indonesian twin must not disappear).
     - A merged row keeps the **left** row's `level/page/entry/key/frame` columns; its text is
       `format!("{left}{sep}{right}")` where `sep` = `entry.pair_separator` (`" / "` default),
       trimmed to one line the same way `separator` is (`one_line`, `MAX_SEPARATOR` cap). When
       `prefix_len` of the right row is > 0 and the left row carries the same chapter prefix, strip
       those chars from the right text first (no doubled `"BAB 01 … / BAB 01 Chapter …"`).
  3. `entries_param` validates `pairSeparator`: `one_line` + `MAX_SEPARATOR`, and stores `None`
     when it equals the default `" / "` (same convention as `separator`).
  4. Params doc string of `toc.generate` documents both options.

- **Acceptance Criteria:**
  - [ ] Synthetic doc, two column frames on one page, `Heading 1` in each: `toc.generate` with
       `pairColumns` yields `Proporsi fisik daratan / Physical proportions of the landmass` +
       one page number — one row, not two (the user's exact example).
  - [ ] Two heading pairs on one page pair in vertical order (k-th with k-th), including the
       interleaved sort order case.
  - [ ] Count mismatch (2 ID, 1 EN) leaves the extra ID heading as its own row.
  - [ ] Full-width frames never pair (regression guard for Table/Chart TOCs).
  - [ ] `chapterNumber` prefix is not doubled in the merged row.
  - [ ] `toc.entries` reflects the same pairing.
  - [ ] With `pairColumns` absent/false the output is byte-identical to today (single-language
       documents unaffected).

---

## Phase 3: Pubgen — state, plan, checkbox (`crates/ui-egui/src/panels/pubgen.rs`)

**Goal:** the TOC card exposes the option; Add and Rebuild both honour it.

- **Files:**
  - Modify: `crates/ui-egui/src/panels/pubgen.rs` (`PubgenState`, `TocLevel`, `TocPlan`, TOC card, tests)

- **Changes:**
  1. `PubgenState`: `#[serde(default)] pub toc_pair_bilingual: bool` (default `false`).
  2. `TocLevel` gains `pair: bool`; `TocPlan::from_state` sets it from `st.toc_pair_bilingual`;
      `TocPlan::from_toc` maps it from the stored `e.pair_columns` (rebuilds keep their pairing
      whatever the checkbox shows); `entries_json` emits `"pairColumns": true` (and
      `"pairSeparator"` only when a non-default is ever added — not in this phase).
  3. TOC card checkbox after "Include TOC page in TOC entries" (`:4695`):
      label `"Pair bilingual headings (ID / EN)"`, hover text explaining it merges same-page
      left/right column headings of the Paragraph TOC into one row and needs the Dual Lang
      two-column layout. Visible for every TOC kind; the engine's overlap guard makes it a no-op
      where pairing makes no sense.
  4. No sheet-name, catalog, or generator changes: works with today's `RXX` + `file_en` Dual Lang
      pages and with pages generated before this feature.

- **Acceptance Criteria:**
  - [ ] Checkbox state persists across app restarts (serde).
  - [ ] Paragraph TOC + checkbox on a generated dual-lang RXX document: `execute_add_toc` produces
       the merged rows of §0; Rebuild TOC keeps them (stored entries).
  - [ ] Single-lang document with the checkbox on: TOC output unchanged (no right columns to pair).
  - [ ] New test alongside `test_toc_paragraph_lists_a_generated_review_page_with_its_chapter_line`
       (`:12023`) covers the end-to-end path.

---

## Phase 4: Verification, docs, quality gates

- **Files:**
  - Modify: `docs/pubgen-guide.md` (TOC section: the new checkbox and the pairing rule)
  - Modify: `ROADMAP.md` (progress-log line)
  - Modify: this plan (status line)

- **Acceptance Criteria:**
  - [ ] `cargo xtask ci` passes (fmt, clippy -D warnings, tests, assets, layers, wasm).
  - [ ] No `unwrap`/`expect`/`panic` in the new code (never-crash rules); all slicing at char
       boundaries (`prefix_len` is a char count — strip via chars, not bytes).
  - [ ] Manual check on `plan/pubgen/samples/DUAL_LANG`: generate pages from an RXX sheet, add a
       Paragraph TOC with the checkbox on, export PNG, and read it — rows read
       `"ID / EN …… n"`.
  - [ ] ROADMAP progress line + this plan's Status header updated.

---

## Risks / decisions locked

- **Pairing is geometry-based** (frame columns), not style- or language-based: ID and EN RTFs share
  heading style names, and text content matching is fragile. The horizontal-disjoint guard is what
  keeps other TOC kinds and full-width frames safe.
- **k-th with k-th per page** is the pairing semantic (relaxed from per (page, level) during
  implementation: the DUAL_LANG samples style the twins "heading 2" / "heading 1", so twins never share
  a level); a missing twin stays a single row. The merged row keeps the **left** row's level, entry and
  position, so sorting, continuation headers and hanging-indent behaviour are unchanged.
- **Merged row keeps the left row's position data**, so sorting, continuation headers and
  hanging-indent behaviour are unchanged.
- `pairSeparator` ships as a stored-but-UI-less option (engine accepts it; no control yet) — add a
  control only when someone needs a non-default joiner.
