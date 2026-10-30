# PubGen Pillar 5 — Codebase Modularization Mini-Plan

- **Document:** `docs/plans/2026-10-25-pubgen-pillar5-modularization-mini-plan.md`
- **Date:** 2026-10-25
- **Status:** Draft — execution plan for M1 of [`2026-10-25-pubgen-future-roadmap-and-extension-plan.md`](2026-10-25-pubgen-future-roadmap-and-extension-plan.md) (Pillar 5)
- **Target:** `crates/ui-egui/src/panels/pubgen/`

---

## 1. Goal

Decompose the 17,818-line `mod.rs` into domain modules with **zero functional regressions**, via small, bisectable, individually-CI-green commits. End state: `mod.rs` is a thin panel entrypoint (< ~500 lines).

## 2. Current State (measured 2026-10-25)

| File | Lines | Notes |
| :--- | ---: | :--- |
| `mod.rs` | 17,818 | ~10,700 production + ~7,000 `#[cfg(test)]` (~120 integration tests, lines 10,777→end) |
| `excel.rs` | 3,134 | already extracted |
| `cover_art.rs` | 1,924 | already extracted (cover style templates) |
| `ai.rs` | 1,486 | already extracted (AI provider, jobs, prompts) |
| `locale.rs` | 343 | already extracted |
| `divider_art.rs` | 336 | already extracted |

Production domains inside `mod.rs` (line ranges, snapshot 2026-10-25):

| Concern | Lines | ~Size |
| :--- | :--- | ---: |
| `PubgenState` + defaults + serde attrs | 62–708 | ~650 |
| Panel UI: `show()`, cards, thumbnails, tooltips, header band | 833–4,560 | ~3,700 |
| H&F UI + execute fns + template mgmt | 2,612–3,210 | ~600 |
| Covers: inside/back/front exec, branding, grayscale | 3,275–4,221 | ~950 |
| CSV parse + row-condition/footnote predicates | 4,565–4,988 | ~420 |
| `TableColorTheme` + swatch resolver | 4,989–5,061 | ~70 |
| Template path spec + row-template slots | 5,062–5,600 | ~540 |
| `execute_generate_pages` | 5,601–7,118 | ~1,550 |
| Cross-domain doc/frame/story helpers | 7,119–7,758 | ~640 |
| `execute_insert_table` + `execute_insert_table_matrix` | 7,759–8,687 | ~930 |
| Type/number plausibility helpers | 8,688–8,826 | ~140 |
| TOC engine (plan, enrich, styles, continuation) | 8,827–10,289 | ~1,450 |
| Page utils: renumber, continue, page-from-rtf/image | 10,290–10,467 | ~180 |
| Excel→CSV pipeline executes | 10,468–10,669 | ~200 |

## 3. Principles

1. **Leaf-first extraction order** — pure/stateless code moves before engines; engines before UI.
2. **One extraction = one commit.** `cargo xtask ci` green after every commit. `mod.rs` shrinks monotonically. Any phase can stop; intermediate states are shippable.
3. **Tests travel with their domain** in the same commit (`#[cfg(test)] mod tests` per file). The green test suite IS the zero-regression gate — verified continuously, not at the end.
4. **Visibility:** `pub(crate)` within `panels::pubgen` during transition. Nothing new goes `pub` to the crate. No new `pub` API surface.
5. **Flat over deep.** One file per domain. Subdirectories (`toc/`, `covers/`) only when a file crosses ~1,500 lines. No empty dirs for future pillars (`charts/` is M4's job, not ours).
6. **Mechanical moves only.** No renaming, no signature changes, no "small cleanups along the way". Refactor commits contain moves + import fixes + nothing else.

## 4. Target Structure (end state)

```text
crates/ui-egui/src/panels/pubgen/
├── mod.rs              // panel entrypoint: show(), top-level layout, module wiring  (~500)
├── state.rs            // PubgenState, Default, defaults, serde
├── locale.rs           // (exists)
├── helpers.rs          // cross-domain doc/frame/story/style utilities
├── widgets.rs          // pubgen-only UI chrome: buttons, badges, logo, thumbnails
├── csv.rs              // delimiter detection, RFC4180 parse, row predicates
├── themes.rs           // TableColorTheme, DIV swatch resolver
├── tables.rs           // table generation engine (matrix → items)
├── pages.rs            // execute_generate_pages, template slots, page utils
├── toc.rs              // TOC engine (splits to toc/ when > 1,500 lines)
├── covers.rs           // cover executes; absorbs cover_art.rs + divider_art.rs
├── header_footer.rs    // H&F UI + executes + templates
├── excel.rs            // (exists)
└── ai.rs               // (exists)
```

Deviations from the Pillar 5 proposal in the roadmap doc (deliberate):

- `covers/` 5-file tree, `toc/` 4-file tree, `header_footer/` 3-file tree → **flat single files** until size forces a split. Splitting 900 lines into 3 files is over-organization.
- `charts/`, `audit/` **not created** — they belong to Pillars 1 and 3.
- Existing `cover_art.rs` / `divider_art.rs` are absorbed into `covers.rs` rather than left as parallel top-level files.

## 5. Phases

Branch: `refactor/pubgen-modularization` off `pubgen-0.7.X` **after** the current uncommitted ai.rs/mod.rs work is committed. Separate `CARGO_TARGET_DIR` per parallel-agent rule.

---

### Phase 0 — Baseline & guardrails (~0.25 d)

- Record baseline: `cargo xtask ci` result, `cargo build -p ui-egui` clean-incremental timing, full-build timing, `wc -l` snapshot (§2 table).
- Confirm the ~120 tests pass as-is; note any pre-existing flakes in the commit message.
- **Deliverable:** baseline numbers in commit message; no code change.
- **Gate:** green CI before touching anything.

### Phase 1 — `state.rs` (~0.25 d)

- Move: `PubgenState` (264), `impl Default` (568), all `default_*` fns (77–262), `detect_document_margins` / `execute_set_page_margins` if state-adjacent (709–832, judge by coupling).
- Tests: none dedicated — state is exercised everywhere. Verify suite stays green.
- **Gate:** mod.rs drops ~650 lines.

### Phase 2 — `helpers.rs` (~0.5 d)

- Move cross-domain doc/frame/story utilities (7,119–7,758): `target_page_index`, `ensure_para_style`, `page_title_index`, `tag_page_title`, `story_of_frame`, `frame_with_*`, `story_is_overset`, `italicize_*`, `place_catalog_image`, `harmonize_signatory_block`, `compute_signature_placement`, `split_chapter_title*`, `split_chart_title`, type predicates (`is_ulasan_type`, `is_divider_type`, `is_chart_type`).
- These are depended on by tables, pages, and toc — must move before those phases.
- **Gate:** mod.rs drops ~640 lines; all consumers updated to `use super::helpers::*` (or explicit paths).

### Phase 3 — `csv.rs` + `themes.rs` (~0.25 d)

- `csv.rs` (4,565–4,988): `delimiter_name`, `detect_delimiter`, `parse_csv_content*`, `load_card1_csv`/`load_card2_csv` (if pure), `detect_footnote_row`, `matches_pubgen_row_condition`, `is_index_or_empty`, `is_numeric_or_dash`, `find_named_col`, `effective_col`.
- `themes.rs` (4,989–5,061 + 157): `TableColorTheme`, `parse_table_color_theme`, `ensure_swatch`, `adjust_hex`, palette fns.
- Tests move: `test_parse_csv_content_*`, `test_detect_delimiter_*`, `test_detect_footnote_row`, `test_is_numeric_or_dash`, `test_matches_pubgen_row_condition_keywords`, `test_parse_table_color_theme`.
- **Gate:** mod.rs drops ~500 lines; pure-parser tests now live next to their code.

### Phase 4 — `toc.rs` (~1 d)

- Move TOC engine (8,827–10,289): `TocLevel`, `TocPlan`, `toc_kind_*`, `wrap_toc_title`, `wrap_paragraph_title`, `enrich_toc_story`, `enrich_paragraph_toc_story`, `ensure_toc_entry_style`, `apply_toc_post_process`, continuation-header fns, `generate_toc`, `execute_add_toc`, `execute_rebuild_toc`, `rebuild_all_tocs`.
- Tests move (~20 tests): all `test_*toc*`, `test_toc_*`, continuation, hanging-indent, bilingual-pairing tests.
- Largest test cluster — move carefully; keep test helpers (`add_page_text`, `toc_text_on_page`, …) in the new file's test module, duplicated only if mod.rs still needs them (prefer `#[cfg(test)] pub(crate)` helper re-export over duplication).
- **Gate:** mod.rs drops ~1,450 lines; TOC tests green in new home.

### Phase 5 — `tables.rs` (~0.75 d)

- Move table engine (7,759–8,687 + 8,688–8,826 predicates if not already in helpers): `execute_insert_table`, `execute_insert_table_matrix`, `table_cell_text`, `find_type_and_number_on_page`, plausibility fns.
- Tests move (~25 tests): the `test_execute_insert_table*` family, 3-stack headers, merged headers, col-numbers, horizontal rules, bilingual italic, theme-from-DIV, KBDA samples.
- **Gate:** mod.rs drops ~1,050 lines.

### Phase 6 — `pages.rs` (~1 d)

- Move (5,062–7,118 + 10,290–10,467): `resolve_template_path_spec`, `post_process_template_applied_page`, `RowSlots`, `row_template_slots`, `execute_generate_pages`, plus page utils (`execute_renumber_page`, `execute_continue_numbering`, `execute_page_from_rtf`, `execute_page_from_image`).
- Tests move (~30 tests): `test_execute_generate_pages*`, g-sheet title tests, page-from-rtf/image tests, renumber/continue, KBDA reproduction, kata-pengantar signature.
- Biggest, most state-coupled move. Do not combine with any other phase.
- **Gate:** mod.rs drops ~2,200 lines.

### Phase 7 — `covers.rs` (~0.5 d)

- Move cover executes (3,275–4,221): `convert_asset_id_to_grayscale`, `transform_inside_cover_item_shapes`, `execute_generate_inside_cover`, `find_back_cover_page`, `ensure_back_cover_at_end`, `MainCoverBranding`, `extract_main_cover_branding`, `resolve_agency_swatch`, `execute_generate_back_cover`, `execute_generate_front_cover`, `execute_generate_cover`.
- **Absorb** `cover_art.rs` + `divider_art.rs` into `covers.rs` (pure moves, `mod covers;` style — or keep as `covers/mod.rs` + `covers/templates.rs` since cover_art is already 1,924 lines; judge at execution: combined ~4,200 lines justifies `covers/` dir).
- Tests move: `test_execute_generate_cover_text_formatting`, `test_export_design_*`, `test_cover_style_*`, `test_excel_cover_sheet_*`.
- **Gate:** mod.rs drops ~950 lines; file count in dir shrinks by 2.

### Phase 8 — `header_footer.rs` (~0.5 d)

- Move H&F (2,612–3,210): template load/save/delete, `show_hf_preview_tooltip`, `card_header_footer_ui` (UI moves here too — first UI chunk), `execute_apply_header_footer`, `execute_remove_header_footer`.
- Tests move: `test_quick_actions_header_footer_and_table_options` (split if it spans domains).
- **Gate:** mod.rs drops ~600 lines.

### Phase 9 — UI split + final slim (~1 d)

- Move remaining panel UI (833–2,611 + 4,222–4,560) into `widgets.rs` (chrome: `pubgen_logo_texture`, `about_pubgen_dialog`, `primary_outline_button`, `pubgen_header_band`, `badge`, `style_thumbnail_texture`, `paint_fallback_cover_thumb`, `show_style_thumbnail_tooltip`, `card1/card2/cover/toc/quick-actions` bodies) and keep only `show()` + card orchestration in `mod.rs`.
- `manual_import_accordion_ui`, `master_excel_ui`, `page_utilities_ui` → nearest domain file or `widgets.rs`.
- **Deliverable:** `mod.rs` < ~500 lines; final tree matches §4.
- **Gate:** full suite green; `cargo xtask layers` passes.

### Phase 10 — Closeout (~0.25 d)

- Re-measure: incremental compile time for an edit inside covers vs. baseline (the M1 motivation metric), `wc -l` final table.
- Update the Pillar 5 section of the roadmap doc to reflect the executed structure (flat files, absorbed modules, test distribution).
- ROADMAP.md progress-log line; close M1.
- **Gate:** docs consistent; no dangling `mod.rs` internals.

## 6. Risks & Mitigations

| Risk | Mitigation |
| :--- | :--- |
| Hidden coupling: a "pure" fn captures `&mut PubgenState` or `&mut DesignApp` deeply | Phase 2 moves helpers *before* engines so shared dependencies have a home; if a move reveals coupling, stop, commit the discovery, adjust split — never force it |
| Test helpers shared across moved test clusters | `#[cfg(test)] pub(crate)` helper module inside pubgen; no duplication |
| Merge conflicts with active pubgen-0.7.X feature work | Do modularization on a dedicated branch started after current work is committed; keep phases short; land early |
| egui context / texture handles assume single call site | Texture caches stay in `widgets.rs`; verify with `ui.screenshot` spot-checks on cover card + H&F card |
| Compile-time regression from circular-ish module refs | Rust modules within one crate can't cycle at type level; use `super::` paths; if borrows tangle, that's a design signal — split state accessors, not visibility |

## 7. Out of Scope

- Any behavior change, rename, signature change, or cleanup.
- Creating `charts/` (M4) or `audit/` (M2).
- Moving tests to a `tests/` integration directory (they use internal APIs; in-module is correct).
- Extracting PubGen into its own crate (see `plan/pubgen/` spin-off docs — separate decision).

## 8. Definition of Done

- [ ] `mod.rs` < ~500 lines; tree matches §4.
- [ ] Every domain file carries its own `#[cfg(test)] mod tests`; total test count ≥ baseline (nothing lost in moves).
- [ ] `cargo xtask ci` green; `cargo xtask layers` green; wasm build green.
- [ ] Incremental-compile improvement measured and recorded.
- [ ] Roadmap doc Pillar 5 section + ROADMAP.md updated; M1 closed.
