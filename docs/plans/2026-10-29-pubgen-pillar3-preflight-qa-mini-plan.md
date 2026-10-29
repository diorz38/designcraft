# PubGen Pillar 3 — Statistical Layout Preflight & QA Audit Mini-Plan

- **Document:** `docs/plans/2026-10-29-pubgen-pillar3-preflight-qa-mini-plan.md`
- **Date:** 2026-10-29
- **Status:** Draft — not started. M2 of [`2026-10-25-pubgen-future-roadmap-and-extension-plan.md`](2026-10-25-pubgen-future-roadmap-and-extension-plan.md) (Pillar 3)
- **Branch:** `feature/pillar3-preflight-audit` (proposed, off `pubgen-0.7.X`)
- **Target:** `crates/engine/src/cmd/`, `crates/ui-egui/src/panels/pubgen/`, `ROADMAP.md`, `docs/`

---

## 1. Goal

Close the gap between the generic preflight (already shipped) and the roadmap's Pillar 3: implement the four missing statistical-publication rules — `PRF-03` missing `Sumber:`/`Catatan:`, `PRF-04` table-number sequence, `PRF-05` dual-language parity, `PRF-07` spine/bleed guard — plus the remaining half of `PRF-01` (native table *cell* overset), and expose them behind a **Pubgen Preflight Audit** command with an **"Audit Publication"** button that opens an interactive, click-to-remediate checklist. Six phases (0–5), zero regressions, no new crates.

## 2. Current State (verified 2026-10-29 on `pubgen-0.7.X`)

| Component | State | Notes |
| :--- | :--- | :--- |
| `crates/engine/src/cmd/preflight.rs:116` | `check(s, min_ppi)` — generic preflight | Walks `d.stories` (composed via `s.cache.get`), spreads/items, swatches. Returns `Vec<Issue>` (`:15`, `{severity, kind, message, item?, page?}`) |
| `preflight.rs:125-145` | Story overset + unplaced stories | Covers `PRF-02` and `PRF-01` **only for tables laid out as text stories** |
| `crates/compose/src/table.rs:352-361` | Native table cells carry their own `overset_at` (`t.overset_at`) | Composed story exposes them at `cs.frames[].tables[].cells[].text` — **never read by engine preflight**. `PRF-01` cell overset is therefore reportable with a small walk, no new composition |
| `preflight.rs:28-31` | `preflight.run` spec, `{minPpi?: 150}` | Menu `Window › Output › Preflight`; `Issue` is `Serialize` only (no `Deserialize`) |
| `crates/ui-egui/src/panels/properties/links.rs:264-305` | `preflight_panel` | Click an issue → `selection.set {ids:[item]}` + `crate::canvas::go_to_page(app, page)` — the exact remediation pattern to reuse |
| `crates/engine/src/cmd.rs:239` | `v.extend(preflight::specs());` | Registration point for a sibling module |
| `crates/ui-egui/src/panels/pubgen/excel.rs:122` | `format_table_number` → canonical `"XX.XX"` (2-digit, dot) | The numbering scheme `PRF-04` validates |
| `excel.rs:630-651` | `matches_table_note_keyword` (`sumber`, `source`, `catatan`, `note`, `keterangan`, `ket.`/`ket`), `is_table_note_or_source_row`, `TableNoteItem` (`:623`) | The `PRF-03` keyword vocabulary. Lives in **ui-egui** — engine cannot call it (layering, `cargo xtask layers`), so the matcher is duplicated engine-side (§4.3) |
| `crates/ui-egui/src/panels/pubgen/tables.rs:67-70, 272-275` | Table sheets carry note rows into the placed table; `bilingual_output` doubles the reserve height | Note rows land as table rows whose first cell starts with a keyword |
| `crates/ui-egui/src/panels/pubgen/ai.rs:1349-1443` | `PageTarget` + `detect_page_target(page_idx)` | Dual-lang review-page detection (EN "Chapter NN" header, BAB line, two body columns ≥120 chars). Also ui-egui-only; engine needs its own compact walk (§4.4) |
| `crates/ui-egui/src/panels/pubgen/state.rs:366` | `bilingual_output` — Dual Lang flag | UI-side gate for when `PRF-05` runs |
| `crates/doc/src/lib.rs:370` | `settings.bleed: [f64; 4]` — top, bottom, inside, outside (order per `engine/src/separations.rs:50`) | `PRF-07` bleed rule input; zeros = document declares no bleed |
| `crates/doc/src/page.rs:428` | `Spread::spine_x()` | `PRF-07` spine rule input; centre for single pages, right edge for left-only (RTL) spreads |
| `crates/doc/src/page.rs:14` | `Margins {top, bottom, inside, outside}`; `PageSide::Left/Right/Single` | Facing-page detection for the spine rule |
| `crates/engine/src/cmd/mod` (cmd.rs:172-246) | `command_specs()` aggregate | No `PRF-` identifier exists anywhere in the codebase yet (roadmap status confirmed) |
| `crates/ui-egui/src/panels/pubgen/widgets.rs:219-245` | Header band: "Translate this Page" button row | Home for the **"Audit Publication"** button |
| CI | `cargo xtask ci` + the pubgen lint gate (`clippy -p designcraft-ui-egui --features pubgen --all-targets -- -D warnings`, added with Pillar 4) | The panel code is otherwise never linted |

## 3. Principles

1. **Everything is a command.** The audit is `preflight.pubgen` in `crates/engine/src/cmd/` — reachable from CLI, MCP and the control channel; the button is a thin caller. UI shows results, never computes them.
2. **Rules are read-only diagnostics.** Nothing mutates the document. Remediation = the user clicks an issue and fixes it; the audit never "renumbers" or "fills" anything (auto-fix is out of scope, §8).
3. **Heuristics only fire on Pubgen-shaped content.** A novel has no `Sumber:` rows and no `Tabel 02.04` titles. Each rule no-ops when its trigger pattern is absent, so the audit is safe to run on any open document. Sequence and parity rules are silent unless they find ≥2 candidate items.
4. **Layering is respected.** New rules live in the engine; the small keyword/page-shape matchers that already exist in `ui-egui` (`excel.rs`, `ai.rs`) are duplicated in the engine module with pointer comments rather than moved (moving them would drag ui-egui tests into the engine; revisit in Pillar 5 if drift bites).
5. **Generic preflight stays generic.** `preflight.run`'s output shape and issue set grow only by `PRF-01` (`cellOverset` — a native-table defect, not a Pubgen rule). Pubgen rules are a separate command so non-pubgen documents, CI oracles and the status-bar count do not change.
6. **One phase = one commit.** `cargo xtask ci` green after every commit, including the pubgen lint gate.
7. **Never crash.** Walks are depth-bounded and index-safe exactly like the existing `preflight.rs` (`missing_glyphs` depth cap 16, `get()` everywhere). Input-derived numbers (frame bounds, sequence numbers) are hostile: `total_cmp`, saturating arithmetic, no `unwrap` outside tests.
8. **Stdlib first.** No new crates. `serde_json` for the result, geometry already in `designcraft_geom`/`designcraft_doc`.

## 4. Architecture Decisions

### 4.1 Command surface

```text
preflight.run    {minPpi?: 150}                       ← unchanged (+ new kind "cellOverset")
preflight.pubgen {minPpi?: 150}                       ← NEW
  → {errors, warnings, issues: [{severity, kind, message, item?, page?}]}
  = preflight.run's generic check + the Pubgen rules below (one call = the full checklist)
```

One merged command keeps the audit button a single `app.run`; CLI users get the same checklist for CI (`designcraft-cli run --cmd 'preflight.pubgen={"minPpi":300}'`). `minPpi` forwards to the generic check so a print audit can raise the bar to 300 in one call.

### 4.2 Rule → kind → severity map

| Check | `kind` | Severity | Fires when |
| :--- | :--- | :--- | :--- |
| `PRF-01` cell overset | `cellOverset` | error | A native table cell's composed text has `overset_at` (truncated digits are silent data loss in a statistical publication) |
| `PRF-03` missing source/notes | `missingSourceNote` | warning | A table row's first non-empty cell matches a note keyword (`sumber`, `source`, `catatan`, `note`, `keterangan`, `ket.`/`ket` — case-insensitive) and the rest of the row (joined cells, colon stripped) is empty |
| `PRF-04` table-number sequence | `tableNumberSequence` | warning | Collected `Tabel`/`Table`/`Grafik`/`Chart`/`Gambar`/`Figure NN.NN` titles within one chapter prefix (`NN.`) are not strictly 01, 02, 03… — gaps and duplicates each get one issue naming the offender |
| `PRF-05` dual-lang parity | `dualLangParity` | warning | A detected dual-lang review page has an EN body column that is empty, placeholder-only (`…`, `...`, `-`, `–`, `TODO`), or byte-identical to the ID body column |
| `PRF-07` bleed guard | `bleedGuard` | error | Document intent is Print, `settings.bleed` is non-zero, and a graphic that crosses a trim edge extends past it less than 3 mm (8.505 pt) on that edge |
| `PRF-07` spine guard | `spineGuard` | error | Document intent is Print, a text frame on a facing-page spread sits within 5 mm (14.173 pt) of `spine_x()` (measured on its spine-side bound) |

`PRF-02` (story overset) and `PRF-06` (raster DPI) are already covered by the generic check and arrive through the merge for free.

### 4.3 `PRF-01` and the composed walk

`preflight.rs` already composes every story; the missing piece is iterating `cs.frames[].tables[]` the way `missing_glyphs` (`:46-75`) iterates cells for glyphs:

```rust
// preflight.rs, inside check()'s story loop:
for f in &cs.frames {
    for t in &f.tables {
        for (ci, cell) in t.cells.iter().enumerate() {
            if cell.text.overset_at.is_some() { /* push Issue kind "cellOverset",
                item = the owning page frame (same owner logic as missing_glyphs),
                message = "Overset table cell: N characters" (N from overset_at..len) */ }
        }
    }
}
```

Note rows of a table are themselves cell text, so `PRF-03`'s doc-level scan reuses the same `cs.frames[].tables[].cells` walk inside the new module.

### 4.4 `PRF-05` page detection without ui-egui

Engine-side twin of `detect_page_target` (`ai.rs:1376`), reduced to what parity needs — per spread, collect `(id, x0, y0, text)` of non-empty text frames, then require: one frame starting with "Chapter" (<80 chars), one two-word "BAB NN"/"CHAPTER NN" frame, and ≥2 further tall body frames below the header sorted by x0 (first = ID, last = EN). No `PageTarget` struct, no header prefix — just `(page_idx, id_body, id_text, en_body, en_header_id)`. ~40 lines; a comment on both sides names its twin to keep them in sync (Principle 4).

Placeholder check runs on the EN column text trimmed: empty, or in `["…", "...", "-", "–", "TODO"]` case-insensitively, or `==` the ID column text (untranslated column). Identical-text is a heuristic, accepted because it is a warning, never a mutation.

### 4.5 `PRF-04` collection

Scan every story's raw text (no composition needed — titles are plain paragraphs) for lines matching `^(Tabel|Table|Grafik|Chart|Gambar|Figure)\s+(\d{1,2})\.(\d{1,2})\b` (case-insensitive, trimmed). Group by the chapter number; sort within a group; report:

- **gap:** `NN.02` followed by `NN.04` → "Table number gap: NN.03 skipped (between NN.02 and NN.04)";
- **duplicate:** the same number twice → "Table number duplicated: NN.02 appears 2×".

Numbers are `u8`-parsed with `get()`/`ok_or` discipline (Principle 7). Pages come from the story's first placed frame via the existing `page_of` helper (`preflight.rs:33` — made `pub(crate)`).

### 4.6 `PRF-07` geometry

Millimetres convert once: `const MM: f64 = 72.0 / 25.4;` → `3.0 * MM`, `5.0 * MM`. Both rules run only when `d.settings.intent == Intent::Print` (mirrors `lowResolution`, `preflight.rs:288`).

- **Bleed:** skip entirely when all four `settings.bleed` entries are ≤ 0 (the document declares no bleed; flagging would be noise). Otherwise, for each graphic item whose bounds cross a trim edge (spread bounds), measure the overshoot past that edge with the item's accumulated transform (the walk in `check()` `:246-251` already carries `parent`); overshoot < 3 mm on any crossed edge → one issue per item naming the worst edge.
- **Spine:** facing-page spreads only (`sp.pages` contain both `Left` and `Right`; single pages have no spine — `spine_x()` falls back to centre, which would false-positive). For each `Content::Text` item: on a left page the frame's `x1` must be ≤ `spine_x − 5 mm`; on a right page its `x0` must be ≥ `spine_x + 5 mm`. Tolerance: the check uses `<` on the strict distance, so a frame exactly 5 mm away passes.

### 4.7 UI: the "Audit Publication" button and audit dialog

- **Button** (`widgets.rs`, header band, after "Translate this Page"): outline button, label `Audit Publication`, enabled whenever a document is open (no Groq/Dual-Lang gating — the audit works on single-lang and untouched documents too).
- **Dialog** (new `crates/ui-egui/src/panels/pubgen/audit.rs`, registered in `mod.rs`): a modeless `egui::Window` mirroring the narrative-preview window pattern — runs `app.run("preflight.pubgen", json!({"minPpi": st.audit_min_ppi}))` on open (and a **Re-run** button), shows the green/red dot + `N errors, M warnings` header exactly like `preflight_panel` (`links.rs:268-287`), then one row per issue: severity glyph (`⛔`/`⚠`), message, `page {label}` via `session.page_label`.
- **Remediation:** clicking a row = `selection.set {ids:[item]}` + `crate::canvas::go_to_page` — byte-for-byte the `preflight_panel` pattern (`links.rs:296-303`), which already selects and brings the page on-canvas. Zoom-to-frame is a later nicety (§8).
- **State** (`state.rs`): `#[serde(skip)] pub audit_issues: Option<Vec<AuditIssue>>` + `pub audit_open: bool` + `pub audit_min_ppi: f64` (default 150.0). `AuditIssue` is a small `Deserialize` struct in `audit.rs` (engine `Issue` does not implement `Deserialize`, and adding it would widen the engine's serde surface for a UI-only need).
- No i18n keys beyond what `tr` already resolves; labels follow the panel's existing English-first strings.

### 4.8 Rejected alternatives

- **Put the Pubgen rules inside `preflight.run`:** changes the generic contract and the status-bar count for every document; a non-pubgen user gains nothing and CI oracles churn. The merge happens in `preflight.pubgen`, not in `preflight.run`.
- **Compute rules UI-side from `card1_records`:** audits only what Pubgen itself generated, not what the user edited since; and violates Principle 1 (UI would own logic). The document is the source of truth.
- **Auto-fix (renumber tables, insert `Sumber:`):** mutations from a checker invite silent document changes; the roadmap asks for an interactive checklist, not a fixer (§8).
- **Move `matches_table_note_keyword`/`detect_page_target` into the engine now:** layering says ui-egui may depend on engine, not the reverse, so moving is legal — but it would drag `excel.rs`/`ai.rs` test scaffolding along for a ~10-line matcher. Duplicate with pointer comments; consolidate in Pillar 5 if they drift.

## 5. Planned File Structure

```text
crates/engine/src/cmd/preflight.rs        ← PRF-01: cellOverset walk (§4.3); page_of() → pub(crate)
crates/engine/src/cmd/preflight_pubgen.rs ← NEW: Issue reuse; PRF-03/04/05/07 checks;
                                            pub fn check(s, min_ppi) = generic + pubgen; specs() with preflight.pubgen
crates/engine/src/cmd.rs                  ← pub mod preflight_pubgen; v.extend(preflight_pubgen::specs())

crates/ui-egui/src/panels/pubgen/
├── audit.rs    ← NEW: AuditIssue, dialog window, run/re-run, click-to-remediate
├── state.rs    ← audit_open / audit_issues / audit_min_ppi
├── widgets.rs  ← "Audit Publication" header button
└── mod.rs      ← module registration + dialog wiring

ROADMAP.md                                      ← progress log line
docs/plans/2026-10-25-pubgen-future-roadmap…md  ← §4 status block rewritten to as-built
docs/gaps.md                                    ← strike the preflight-audit entry if listed
```

## 6. Phases

Branch `feature/pillar3-preflight-audit`. Separate `CARGO_TARGET_DIR`.

---

### Phase 0 — Baseline (~0.1 d)

1. Run `cargo xtask ci`; record the test count.
2. Confirm the two layering facts this plan leans on: `cargo xtask layers` green with a new engine module, and no `PRF-` identifiers in the codebase (`grep -r "PRF-" crates/`).

**Deliverable:** baseline recorded.
**Gate:** `cargo xtask ci` green.

---

### Phase 1 — `PRF-01` cell overset in the generic preflight (~0.25 d)

1. **`preflight.rs`** — in the story loop, walk `cs.frames[].tables[].cells` (§4.3); owner/location reuse the `missing_glyphs` owner pattern so nested cells report the host page frame. One issue per overset cell: `kind: "cellOverset"`, severity error, message `Overset table cell: {n} characters`, item + page filled via `page_of`.
2. Make `page_of` `pub(crate)` (Phase 2/3 reuse).
3. **Tests** (engine, synthetic sessions):
   - a table whose cell text overflows its geometry → exactly one `cellOverset` issue with the right `item`/`page`;
   - a fitting cell → none;
   - a table inside a note/cell (nested) still reports the host page frame (mirror `mojikumi_nested_tests`).

**Deliverable:** native-table cell truncation is visible in the Preflight panel and `preflight.run`.
**Gate:** `cargo xtask ci` green.

---

### Phase 2 — `PRF-03` + `PRF-04` in a new `preflight_pubgen.rs` (~0.75 d)

1. New module `crates/engine/src/cmd/preflight_pubgen.rs`: `fn pubgen_issues(s: &Session) -> Vec<Issue>` running the two rules; register `specs()` in `cmd.rs` (`preflight.pubgen`, label "Pubgen Preflight Audit", menu `["Window", "Output", "Pubgen Audit"]`, `{minPpi?: 150} → {errors, warnings, issues}`); `run` = generic `preflight::check` + `pubgen_issues`, merged in order (generic first, stable output).
2. **Keyword matcher** — engine-local `matches_note_keyword(cell: &str) -> bool`, the `excel.rs:630` list verbatim, lowercased trim, with a `// twin: ui-egui …/excel.rs matches_table_note_keyword` comment.
3. **`PRF-03`** — for every composed story, walk `frames[].tables[].cells` row-wise (cells share row structure via the table's grid — group by row index): a row whose first non-empty cell matches the keyword and whose remaining joined content (after the colon) trims empty → warning naming the keyword and the table's page. Skip keyword rows whose content is present. (ponytail: matches only the placed-table shape Pubgen emits today; sheet-level CSV auditing is a future rule, §8.)
4. **`PRF-04`** — collect title numbers from all story raw text (§4.5); group by chapter; sort with `total_cmp` on parsed `(u8, u8)`; emit gap/duplicate warnings with `page_of` of the story's first frame. Silent when <2 numbers share a chapter.
5. **Tests:**
   - table with `Sumber:` row and empty content → one `missingSourceNote`; content `BPS, 2025` → none; `Catatan: -` counts as present (a dash is legitimate "no notes");
   - `Tabel 02.01…02.04` clean → none; `02.01, 02.02, 02.04` → gap naming `02.03`; `02.02` twice → duplicate; different chapters (`01.01`, `02.01`) never cross-compared;
   - a novel-ish doc (plain paragraphs) → `pubgen_issues` empty (Principle 3).

**Deliverable:** `preflight.pubgen` exists with the two content rules; CLI-reachable.
**Gate:** `cargo xtask ci` green; `designcraft-cli describe preflight.pubgen` shows the documented params.

---

### Phase 3 — `PRF-05` + `PRF-07` (~0.75 d)

1. **`PRF-05`** — engine-side review-page walk (§4.4) over every page; EN column empty/placeholder/identical → warning `Dual-lang review page: English body is {empty|placeholder|untranslated}`, item = EN frame, page = page index. (ponytail: shape-heuristic only — an exotic custom layout that isn't a Pubgen RXX page won't match; that is the ceiling, and `detect_page_target` stays the UI's authority.)
2. **`PRF-07`** — bleed + spine guards per §4.6, both gated on `Intent::Print`; messages name the measured shortfall (`Cover graphic bleeds 1.2 mm past the right trim edge (< 3 mm)`, `Text frame 2.8 mm from the spine (< 5 mm)`), item + page filled.
3. **Tests:**
   - fabricated dual-lang page: EN column `""`, then `"…"`, then ID-identical text → three warnings with the right reason word; a translated EN column → none; a single-column page → none;
   - facing-pages doc: text frame 4 mm from spine → `spineGuard`; exactly 5 mm → none; single-page doc → no spine issues even with a centred frame;
   - print doc with `bleed: 9`: graphic crossing the trim by 1 mm → `bleedGuard`; crossing by 8.6 mm → none; screen-intent doc → neither rule runs;
   - all four rules on an empty document → no panics, empty output (Principle 7).

**Deliverable:** all four missing rules report through `preflight.pubgen`.
**Gate:** `cargo xtask ci` green.

---

### Phase 4 — UI: "Audit Publication" + audit dialog (~0.75 d)

1. **`state.rs`** — `audit_open: bool`, `#[serde(skip)] audit_issues: Option<Vec<audit::AuditIssue>>`, `audit_min_ppi: f64` (default 150.0); a min-PPI field belongs in the dialog (advanced row), not the header.
2. **`audit.rs`** — `AuditIssue {severity, kind, message, item: Option<u64>, page: Option<usize>}` (`Deserialize`); `audit_dialog(app, st, ui)` window per §4.7: header dot + counts, Re-run button, min-PPI `DragValue` (50–600), issue rows; click → `selection.set` + `canvas::go_to_page`; rows without an item still jump to the page. `#[cfg(not(target_arch = "wasm32"))]` not needed — the dialog does no I/O and no threading, so it ships on wasm as-is.
3. **`widgets.rs`** — "Audit Publication" outline button in the header band (§4.7); click sets `st.audit_open = true` and triggers the first run.
4. **`mod.rs`** — `mod audit;` + window invocation beside the existing preview-dialog wiring.
5. **Tests (ui-egui, `--features pubgen`):**
   - parsing a `preflight.pubgen` result `Value` into `Vec<AuditIssue>` round-trips item/page;
   - the dialog's enable/disable: button renders without a document and no-ops;
   - regression: opening and closing the dialog twice keeps state consistent (no duplicate rows).

**Deliverable:** one-click interactive checklist in the Pubgen panel.
**Gate:** `cargo xtask ci` green **including the pubgen lint gate** (`clippy -p designcraft-ui-egui --features pubgen --all-targets -- -D warnings`); manual smoke via `cargo run --release -p designcraft --features pubgen -- --sample` — run the audit on the sample, click a `lowResolution` row, confirm selection + page jump.

---

### Phase 5 — Closeout (~0.25 d)

1. Rewrite the roadmap's §4 status block to the as-built shape (command id, kinds, what the button does, the duplicated matchers and why).
2. `ROADMAP.md` progress line (dated when it lands); strike the closed gap in `docs/gaps.md` if present.
3. Re-run `cargo xtask assets`, `cargo xtask layers`, and the wasm check (`cargo xtask wasm`).
4. Record the final test count against Phase 0's baseline.

**Gate:** full CI green.

**Effort:** 0 ≈ 0.1 · 1 ≈ 0.25 · 2 ≈ 0.75 · 3 ≈ 0.75 · 4 ≈ 0.75 · 5 ≈ 0.25 → **≈ 2.85 d**.

## 7. Risks & Mitigations

| Risk | Mitigation |
| :--- | :--- |
| Heuristics false-positive on non-Pubgen documents | Rules fire only on Pubgen-shaped triggers (`Sumber:`/`Catatan:` keyword rows, `NN.NN` title lines, Chapter/BAB two-column pages); <2 candidates = silent (Principle 3); a plain-document test pins this |
| `PRF-04` misses numbers rendered as graphics (chart images with baked-in titles) | Documented limitation: the rule scans text stories only; chart titles placed as text *are* found (Pubgen writes them as text) |
| `PRF-05` misdetects a custom layout as a review page | It is a warning, never a mutation; the shape test is deliberately narrow (header + BAB + ≥2 body columns >120 chars) |
| Engine/ui-egui matcher drift (`matches_note_keyword`, page detection) | Pointer comments both sides (§4.2 of the decisions, Principle 4); consolidation candidate for Pillar 5 |
| Spine guard false-positives on single-sided documents | Rule runs only on spreads with both `Left` and `Right` pages; `spine_x()`'s centre fallback is never trusted |
| Bleed guard noise on screen-intent or no-bleed documents | Both `PRF-07` halves gated on `Intent::Print`; bleed half additionally requires non-zero `settings.bleed` |
| Audit walk cost on big documents | All walks reuse the already-cached compositions (`s.cache.get`), identical to `preflight.run`; the audit runs on click, not per frame |
| New engine module escapes lint like the panel did | Engine is linted by default `cargo xtask ci`; the panel side is covered by the existing pubgen lint gate (Phase 4 gate) |
| Never-crash standard | Depth-bounded recursion, `get()`/`let-else` throughout, `total_cmp` on floats, no `unwrap` outside `#[cfg(test)]`; empty-de document test in Phase 3 |
| wasm regression | No fs, no threads, no new deps; `cargo xtask wasm` in Phase 5 |

## 8. Out of Scope

- **Auto-fix** (renumber tables, insert missing `Sumber:` rows, nudge frames off the spine) — the roadmap asks for an interactive checklist; fixers are a separate, mutation-aware feature.
- Zoom-to-frame / canvas centering on the exact item (the panel today selects + jumps to the page; matching that is enough).
- Chart-number sequences as a separate rule from tables (the `Grafik`/`Chart` prefix is already collected in `PRF-04`'s scan; splitting severities per prefix can come later).
- Sheet-level (CSV/Excel) auditing before placement — the document is audited, not the source workbook.
- PDF/X-4 output-intent validation (Pillar 6), `preflight.profiles`/saved rule sets, Preflight-palette-style live re-checking on edit.
- Moving `matches_table_note_keyword`/`detect_page_target` into the engine (Pillar 5 modularization).

## 9. Dependency Impact

| Crate | Change | New deps |
| :--- | :--- | :--- |
| `designcraft-engine` | `cmd/preflight_pubgen.rs` (new), `cmd/preflight.rs` cell-overset walk, `cmd.rs` registration | None |
| `designcraft-ui-egui` | `pubgen/audit.rs` (new), header button, state fields, window wiring | None — `serde_json`, `egui::Window`/`ScrollArea` |
| `designcraft-doc` / `designcraft-compose` | None — `bleed`, `spine_x()`, `PageSide`, `overset_at` already exposed | None |

No new workspace dependencies, no new external crates.

## 10. Definition of Done

- [ ] `preflight.run` reports native-table cell overset (`kind: "cellOverset"`, severity error) with item + page.
- [ ] `preflight.pubgen {minPpi?}` exists, is documented in its params string, and returns the merged generic + Pubgen issue list; `designcraft-cli describe preflight.pubgen` shows it.
- [ ] `PRF-03` flags empty `Sumber:`/`Catatan:` note rows (warning) and stays silent on filled ones.
- [ ] `PRF-04` flags sequence gaps and duplicates within a chapter (warning) and never cross-compares chapters.
- [ ] `PRF-05` flags empty, placeholder or untranslated English bodies on detected dual-lang review pages (warning).
- [ ] `PRF-07` flags <3 mm bleed overshoot on print documents with bleed, and text frames within 5 mm of the spine on facing-page spreads (errors); both stay off elsewhere.
- [ ] The Pubgen header has an **"Audit Publication"** button; the dialog lists issues with counts and a Re-run; clicking a row selects the item and jumps to its page.
- [ ] A plain non-Pubgen document produces zero Pubgen-rule issues (pinned by test).
- [ ] `cargo xtask ci` green (fmt, clippy, tests, assets, layers, wasm) including the pubgen lint gate.
- [ ] Roadmap §4 status rewritten to as-built; `ROADMAP.md` progress line added.

## 11. Revision Log

**2026-10-29 (initial).** Written from the Pillar 3 section of the roadmap plus a verification pass over `preflight.rs`, `compose/table.rs`, `doc/page.rs`, `pubgen/excel.rs`, `pubgen/ai.rs`, `pubgen/tables.rs`, `widgets.rs` and `properties/links.rs`. Key findings folded in: native table cells already carry `overset_at` (so `PRF-01` is a walk, not a layout change); `Issue` is Serialize-only (UI gets its own `Deserialize` struct); `matches_table_note_keyword`/`detect_page_target` are ui-egui-side, hence the engine-local twins; `spine_x()`'s centre fallback disqualifies single-page spreads from the spine rule.
