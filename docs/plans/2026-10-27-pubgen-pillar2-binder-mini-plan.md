# PubGen Pillar 2 — Multi-Chapter Binder & Project Compiler Mini-Plan

- **Document:** `docs/plans/2026-10-27-pubgen-pillar2-binder-mini-plan.md`
- **Date:** 2026-10-27
- **Status:** Proposed — M5 of [`2026-10-25-pubgen-future-roadmap-and-extension-plan.md`](2026-10-25-pubgen-future-roadmap-and-extension-plan.md) §3 / §9
- **Branch:** `feature/pillar2-pubgen-binder` (off `pubgen-0.7.X` at `ebb8f51a`)
- **Target:** `crates/ui-egui/src/panels/pubgen/` (new `compile.rs` + panel card), `crates/engine/src/lib.rs` + `cmd/prefs.rs` (one Prefs field), docs
- **Effort:** ≈ 5.5 days (M5 estimate: 5–6 days)

---

## 1. Goal

Compile a whole municipal publication — cover, inside cover, front matter, 8–12 chapter workbooks, global Daftar Isi / Daftar Tabel / Daftar Grafik, running headers and back cover — from one `pubgen_project.json` manifest into **one** `.designcraft` document (with optional PDF) in one command, with continuous Arabic pagination over roman-numbered front matter.

The compiler must be deterministic, never destroy a previous good build on failure, never panic, and be reachable from the panel, the command palette and the control channel.

## 2. Current State (verified 2026-10-27 on `pubgen-0.7.X` @ `ebb8f51a`)

| Component | State | Constraint it imposes |
| :--- | :--- | :--- |
| `crates/engine/src/cmd/book.rs:257` | Generic Book commands (`book.new/open/add/paginate/syncStyles/exportPdf`) over `.dcbook` | Numbering and PDF merge are **per document**; `toc.generate` is document-scoped. A `.dcbook` of per-chapter documents cannot produce a global Daftar Tabel/Grafik without new cross-file machinery |
| `crates/ui-egui/src/panels/pubgen/excel.rs:2060` | `execute_convert_excel` parses an XLSX to `{stem}_catalog.csv` + one `{sheet}.csv` per table, written **beside the workbook** | Relative `file` references in the catalog (`resolve_catalog_related_file`, `helpers.rs:556`) resolve against the catalog folder or the cwd, so the catalog must stay beside the workbook |
| `crates/ui-egui/src/panels/pubgen/excel.rs:3343` | `execute_excel_to_csvs` loads the generated catalog into Card 1 and table paths into Card 2 | Reusable per chapter without UI |
| `crates/ui-egui/src/panels/pubgen/excel.rs:3376` | `execute_excel_and_generate` = convert → front cover → inside cover → pages → back cover | The single-workbook template for a chapter step; it assumes one workbook per document |
| `crates/ui-egui/src/panels/pubgen/pages.rs:500` | `execute_generate_pages` appends pages from `card1_records`, auto-inserts tables/RTFs/images, tags titles with `toc_style` / `TOC Chapter` | Chapters can be emitted sequentially into one document; the tags already feed `toc.generate` |
| `crates/ui-egui/src/panels/pubgen/covers.rs:820` | Front cover applies a template `before: 0` | Inserting before page 0 always works, regardless of how many chapters follow |
| `covers.rs:212` | Inside cover duplicates spread 0 to page 1 | The initial blank page from `file.new` survives at page 2 (`covers.rs:1976` asserts 4 pages: front, inside, initial, back) — the compiler must normalize it |
| `covers.rs:411`, `covers.rs:302` | Back cover duplicates spread 0 at the end and moves itself to the last page | Must run once, after every chapter |
| `crates/ui-egui/src/panels/pubgen/toc.rs:255`, `toc.rs:1402` | `TocPlan` + `generate_toc` wrap `toc.generate` with width, hanging indent, billingual pairing and continuation headers | Global TOCs can reuse the panel's plans, not a parallel implementation |
| `crates/engine/src/cmd/toc.rs:24` | `toc.generate` takes `page`, `replace`, `autoflow`, returns `pagesAdded` | Generation adds pages; page numbers recorded in earlier lists go stale — needs a two-pass finalize |
| `crates/ui-egui/src/panels/pubgen/toc.rs:11` | `TOC_NARRATIVE_STYLE = "TOC Chapter"` marks `GXX`/`RXX` titles | The running header cannot reuse that paragraph style without changing TOC membership; use a character style |
| `crates/engine/src/cmd/layout.rs:680` | `layout.section` supports `lowerRoman`, `startNumber`, restart | Front matter roman + body arabic restart needs exactly two section edits, applied after pagination is final |
| `crates/engine/src/cmd/layout.rs:160` | `layout.pages.insert {after, count}` | TOC placeholder pages can be reserved before body generation |
| `crates/engine/src/cmd/layout.rs:317`, `:298`, `:459` | Parent spreads, `applyParent`, `documentSetup` | Master setup is already a command pipeline; no new engine work |
| `crates/engine/src/cmd/header_footer.rs:61` | `layout.headerFooter.apply` with `HfScope::Range` (`crates/doc/src/header_footer.rs:22`) | Apply headers to a 1-based **absolute page range**, after all page insertion |
| `crates/compose/src/vars.rs:127` | A running header without a styled paragraph carries the **last value from earlier pages** | One tagged chapter title propagates across the whole chapter |
| `crates/engine/src/cmd/header_footer.rs:281` | `{runningHeader}` expands to the text variable named `Running Header` | The compiler must define that variable (or the token prints literal text) |
| `crates/ui-egui/src/menus.rs:18`, `crates/ui-egui/src/control.rs:162` | `UI_COMMANDS` + `app.run` dispatch; control `ui.menu.invoke` with params reaches UI commands | A UI command is enough for headless/MCP access; no engine command is required for orchestration |
| No `pubgen.*` engine commands exist | All Pubgen behaviour is UI-layer, mutating documents through engine commands | Follow that layering; do not move Excel parsing into `engine` |
| `crates/engine/src/tests_file_guard.rs:64` | `ai.rs` is already allow-listed for file I/O | A new `compile.rs` that reads manifests needs one allow-list reason string |

## 3. Principles

1. **One compiled document.** The binder produces one `.designcraft`; `.dcbook` splitting is a later export, not the build format. This is what makes global TOCs, one numbering chain and one PDF possible with today's engine.
2. **Reuse the chapter pipeline, don't fork it.** A chapter is still `convert_excel_to_csv_data` + `PubgenState` + `execute_generate_pages`; the compiler only sequences and parameterizes it.
3. **Save only on success.** The compile runs in an unsaved document. Failure or cancel leaves the open document unsaved and the previous output file untouched.
4. **Page numbers are computed once, at the end.** Front-matter, TOC and chapter pages are inserted first; sections, running headers and TOC entries are finalized afterwards.
5. **The manifest is hostile input.** Bounded chapter count, validated paths, no panics, no implicit network calls, explicit errors that name the failing workbook and step.
6. **One phase = one commit.** `cargo xtask ci` green after each commit.
7. **No new crates.** `serde`, `serde_json`, `std::fs`, the existing `zip`/XLSX reader and egui are sufficient.

## 4. Architecture Decisions

### 4.1 Compiled document vs. `.dcbook` — decision

**Chosen: one document.**

| Requirement | One `.designcraft` | `.dcbook` of chapters |
| :--- | :--- | :--- |
| Continuous Arabic numbering | Native sections | Possible via `book.paginate` |
| Global Daftar Isi / Tabel / Grafik | `toc.generate` is document-wide | Needs a new cross-file collector, plus page-offset math |
| Running header per chapter | One variable, last-before-page semantics | Needs per-document headers; works, but TOC stays unsolved |
| One PDF with correct TOC page numbers | `file.exportPdf` | `book.exportPdf` merges independent PDFs |
| Incremental recompile | Whole book rebuild | Per-chapter rebuild |

The `.dcbook` alternative is recorded and rejected for M5 because the cross-file TOC collector is a larger, separate feature. A future `pubgen.splitBook` can publish the compiled chapters as a `.dcbook` for per-chapter editing.

### 4.2 Project manifest

`pubgen_project.json`, chosen by the user; all paths resolve **relative to the manifest's folder** (same rule as book chapters). Unknown members are ignored for forward compatibility; every member except `chapters` has a default.

```json
{
  "version": 1,
  "publication_title": "Kecamatan Karawang Timur Dalam Angka 2026",
  "bilingual": true,
  "output": {
    "document": "build/Karawang Timur 2026.designcraft",
    "pdf": "build/Karawang Timur 2026.pdf"
  },
  "document": {
    "preset": "A4",
    "facing_pages": true,
    "bleed_mm": 3.0,
    "margins_mm": { "top": 20.20, "bottom": 20.0, "inside": 19.0, "outside": 15.0 }
  },
  "cover": {
    "source": "workbooks/BAB_01_2025.xlsx",
    "style": 11,
    "theme_preset": 0,
    "include_inside": true,
    "include_back": true
  },
  "front_matter": { "workbook": "workbooks/00_Front_Matter.xlsx" },
  "chapters": [
    { "chapter": 1, "path": "workbooks/BAB_01_2025.xlsx", "title": "Geografi" },
    { "chapter": 2, "path": "workbooks/BAB_02_2025.xlsx", "title": "Iklim" }
  ],
  "pagination": {
    "front_matter_format": "roman_lower",
    "body_format": "arabic",
    "restart_at_body": true
  },
  "toc": {
    "placement": "after_front_matter",
    "lists": ["contents", "tables", "charts"],
    "title_bilingual": true,
    "hanging_indent": true
  },
  "header_footer": { "preset": "book", "scope": "body" },
  "build": { "keep_intermediate": true, "export_pdf": true }
}
```

Fields that map directly onto existing state: `bilingual` → `PubgenState.bilingual_output`; `cover.style` → `cover_style`; `cover.theme_preset` → `cover_theme_preset`; `header_footer.preset` → `layout.headerFooter.apply {preset}`. `cover.include_inside` and `cover.include_back` drive the two cover steps directly, so the legacy combined `cover_include_inside_back` flag is not used by the compiler.

Defaults: `output.document = "<manifest-stem>.designcraft"` beside the manifest; no PDF unless `build.export_pdf`; cover source = first chapter workbook that contains a `COVER` sheet; front matter = leading `G`/`B`/`Ulasan` rows of the first chapter (see §4.5); `toc.placement = "after_front_matter"`.

Validation (before any document is created):

- `chapters` non-empty, ≤ 64 (mirrors `MAX_BATCH_CHAPTERS = 4096` at a sane publication bound).
- Every chapter path exists, is a regular file, ends in `.xlsx`/`.xlsm`, and no path repeats.
- Chapter numbers are positive and unique; order is the manifest order (a warning when it is not ascending).
- Strings are capped: title 512 chars, path 4096 chars, TOC list ≤ 3 known values.
- Output directory exists and is writable; the output document is not open in another tab (compare canonical paths).
- A missing cover sheet or empty front matter is a warning, not an error.

### 4.3 Compile pipeline (one linear state machine)

```
Validate ─► NewDocument ─► MasterSetup ─► Covers ─► NormalizeSeedPage
        ─► FrontMatter ─► TocPlaceholders ─► Chapter(1..n) ─► BackCover
        ─► TocFinalize (2 passes) ─► Sections ─► RunningHeaders
        ─► SaveDocument ─► ExportPdf ─► Done
```

1. **NewDocument** — `file.new {preset, facingPages, pages: 1, title}`; `layout.documentSetup` applies margins/bleed. No chapter is loaded yet.
2. **MasterSetup** — `layout.parents.new {prefix: "A", name: "Body"}`; the compiler remembers its id. The header/footer parent later inherits it (`resolve_parent_spread` sets `based_on`).
3. **Covers** — front cover, inside cover through the existing `execute_generate_front_cover` / `execute_generate_inside_cover`. Cover values come from the manifest's `cover.source` workbook (`apply_excel_cover`, `excel.rs:3256`), falling back to defaults. AI Base styles (`cover_style` ≥ 100) run the existing single-cover path; a network failure stops the compile with a named error.
4. **NormalizeSeedPage** — the blank `file.new` page survives at page 2 next to the inside cover (`covers.rs:1976`). Delete it **iff** it still has no items; otherwise keep it and warn. This is the only page deletion in the compile.
5. **FrontMatter** — a dedicated front-matter workbook is emitted exactly like a chapter (its catalog rows are authored in order). Without one, the leading `G`/`B`/`Ulasan` rows of chapter 1 (rows before the first `DIV`) are emitted as front matter and excluded from the later chapter pass. `G` sheets already drive `execute_generate_pages`; signatory blocks are already split by `helpers.rs:262/359` — no new renderer.
6. **TocPlaceholders** — insert one blank page per requested list at the insertion point (after front matter when `placement = "after_front_matter"`, before the back cover when `"end"`). Blank pages carry no `TOC Chapter` paragraph, so they never list themselves.
7. **Chapter(n)** — for each chapter, in manifest order:
   - `execute_convert_excel(path)` (unchanged; writes beside the workbook so relative RTF/table references keep working);
   - build a scratch `PubgenState` from defaults + the project profile, `load_card1_csv`, set `card2_sheet_tables`, `card1_range_all = true`, `translate_with_ai = false`;
   - when chapter 1 also supplied fallback front matter, generate only the remaining range;
   - `execute_generate_pages(app, &st)` appends pages; record `page_count()` before/after and the generated row counts;
   - tag the chapter divider's title text with the `Running Header` character style (§4.7).
8. **BackCover** — `execute_generate_back_cover` + `ensure_back_cover_at_end`, once.
9. **TocFinalize** — two passes over the lists in document order (`contents`, `tables`, `charts`), each `generate_toc(..., replace: true)`; see §4.6.
10. **Sections** — `layout.section {page: 1, startNumber: 1, style: "lowerRoman"}`; `layout.section {page: body_start, startNumber: 1, style: "arabic"}`. `body_start` is the first chapter page (absolute, 1-based), read after TOC growth settles.
11. **RunningHeaders** — define the character style and the `Running Header` variable, then `layout.headerFooter.apply {preset, scope: {range: [body_start, back_cover_start - 1]}}` (§4.7). Front matter and back cover get no running header.
12. **SaveDocument** — `file.saveAs {output.document}`; then optional `file.exportPdf {path}`.

Every step records `{step, chapter?, message, warnings}`; the first error stops the pipeline.

### 4.4 Chapter emission without the panel

Chapters are emitted through a small source abstraction so tests never need an XLSX:

```rust
pub(crate) enum ChapterSource {
    Workbook(PathBuf),                         // manifest path: convert first
    Catalog { catalog: PathBuf, tables: Vec<PathBuf> }, // tests / pre-converted CSVs
}
```

`ChapterSource::Catalog` is internal for M5; the manifest accepts only workbooks.

A scratch `PubgenState` is built per chapter rather than mutating the live panel state, so a failed chapter cannot corrupt the user's Card 1/2 settings. The profile copies: `bilingual_output`, TOC style/title, table defaults (`card2_first_row_header`, `card2_col_numbers`, `card2_table_footer`, `card2_row_conditions`), `card1_toc_prefix`, and `card1_auto_insert_tables`. `translate_with_ai` is always `false`: the compiler uses authored `title_en`/`file_en`; translating whole chapters is a separate job (§8).

### 4.5 Front matter ordering and the fallback

BPS front matter order is cover, inside cover, Kata Pengantar, Daftar Isi, Daftar Tabel, Daftar Grafik, Penjelasan Teknis, Daftar Singkatan. A dedicated `00_Front_Matter.xlsx` expresses that order directly in its catalog. The fallback (leading `G`/`B`/`Ulasan` rows of chapter 1) keeps existing single-workbook projects working, but the TOCs land after the whole leading block; any later front-matter rows in that workbook cannot be reordered. The report warns when the fallback is used.

`execute_generate_pages` range parameters (`card1_range_all`, `card1_range_from`, `card1_range_to`) are 1-based and inclusive of the upper position; the compiler computes `k` = number of leading narrative rows and emits `1..=k` as front matter, then `k+1..=len` as chapter 1. If no leading narrative block exists, the compile inserts the TOC placeholders immediately after the inside cover and warns.

### 4.6 Global TOCs without drifting page numbers

`toc.generate` reads page names at generation time and its `autoflow` can add pages. Page insertion shifts later pages, invalidating numbers recorded in lists generated earlier. The compiler uses a **grow-only fixed point**:

1. Insert all list placeholders before the body (once).
2. After chapters and back cover, generate `contents`, then `tables`, then `charts` with `replace: true`, recording `pagesAdded`.
3. Any growth in step 2 shifts body pages. Re-run all three lists in place. Content is unchanged and the first pass already allocated the needed continuation pages, so pass 2 adds nothing; if it still adds pages, run exactly one more pass and then emit a warning in the report instead of looping.
4. Assert in tests that pass 2 reports `pagesAdded == 0` for all lists and that entries' page numbers equal a fresh `toc.entries` query.

`generate_toc` (UI-level) is reused so continuation headers, hanging indents, bilingual pairing and `TOC Chapter` levels behave exactly as in the panel. Compile constructs one `PubgenState` per list with `toc_kind` 3/1/2, the manifest title and `toc_include_self = false`.

### 4.7 Running headers per chapter

The `TOC Chapter` paragraph style cannot be used for the variable: the same style defines Daftar Isi membership. Instead:

1. Create character style `Running Header` (`style.character.create`) if absent.
2. After each chapter is generated, find its divider page and apply the character style to the chapter-title run (`style.character.apply`), leaving the paragraph style (`TOC Chapter`) intact.
3. `variables.define {name: "Running Header", type: "runningHeader", style: "Running Header", character: true, use: "lastOnPage"}`.
4. Apply `layout.headerFooter.apply` with the manifest preset and a 1-based absolute page range covering body pages only.

`RunningIndex::value` (`crates/compose/src/vars.rs:127`) returns the page's value or the last value from an earlier page, so the divisor's title propagates over every following table and chart page until the next chapter. The divider title is the manifestation's `chapters[].title` when the workbook title is missing or ambiguous.

### 4.8 Job, failure isolation, output

`CompileJob` is a small state machine: `next() -> StepResult`, a cancel flag checked between steps, and a `CompileReport`.

- The panel polls one step per frame (the existing `TranslateJob`/`NarrativeJob` pattern), shows `step / total`, the current chapter name and a Cancel button.
- `sync: true` (headless) runs all steps in one call.
- Cancel/error: the document stays open and unsaved with title `<publication> (compile stopped)`; the report names the step and workbook. The previous output file is never touched because saving is the second-to-last step.
- On success: save, optional PDF, report `{document, pdf?, pages, chapters: [{workbook, pages, tables, charts}], warnings, elapsedMs}`.
- A compile failure never closes the user's other documents.

### 4.9 UI and command surface

- New **Publication** card at the top of the Pubgen panel: manifest path with **Open…**, a chapter count/status line, **Compile**, progress bar, **Cancel**, and a report line. The card is hidden on wasm.
- `UI_COMMANDS` entry `("pubgen.compileProject", "Compile Publication…", None, "{path?, sync?: bool, pdf?: path} — …")`, so the command palette, `ui.menu.invoke` and `engine.execute` reach the same code path.
- `Prefs.pubgen_project_path` (camelCase `pubgenProjectPath`) remembers the last manifest; the panel reopens it. `cmd/prefs.rs` param doc gains the field.
- All strings go through `crate::i18n::tr`.

### 4.10 wasm and layering

The compile module is `#[cfg(not(target_arch = "wasm32"))]` behind the `pubgen` feature; a wasm stub returns `Err("compiling a publication needs the desktop app")`. No engine or lower crate gains an egui, `zip` or XLSX dependency; `cargo xtask layers` stays clean.

## 5. Planned File Structure

```
crates/ui-egui/src/panels/pubgen/
├── compile.rs        ← new: ProjectManifest, load/validate, ChapterSource, CompilePlan,
│                       Compiler (steps), CompileJob, CompileReport, TOC fixed point,
│                       section/numbering, running header, save/PDF
├── mod.rs            ← register module; poll the job; wasm stub
├── state.rs          ← compile_job, compile_manifest_path (serde skip)
└── widgets.rs        ← Publication card (thin UI over compile.rs)

crates/ui-egui/src/
├── menus.rs          ← UI_COMMANDS entry
└── i18n.rs           ← new UI strings

crates/engine/src/
├── lib.rs            ← Prefs.pubgen_project_path
├── cmd/prefs.rs      ← param doc
└── tests_file_guard.rs ← allow-list compile.rs with a reason string

docs/plans/2026-10-25-pubgen-future-roadmap-and-extension-plan.md  ← §3 status + M5 row
ROADMAP.md            ← progress line
```

No new assets: covers, dividers, `front_matter/001_publication_note.designcraft` and `data_tables/` already exist. If a template is added, it needs a committed `.designcraft` and an `ASSETS.md` row (`cargo xtask assets`).

## 6. Phases

Branch `feature/pillar2-pubgen-binder`. Separate `CARGO_TARGET_DIR`.

---

### Phase 0 — Baseline (~0.1 d)

1. Record `cargo xtask ci` and the current test count.
2. Confirm assets referenced by the manifest exist and `cargo xtask assets` is green.

**Gate:** CI green; baseline recorded.

---

### Phase 1 — Manifest, planner, Prefs, conversion seam (~1.0 d)

1. **`compile.rs`** — `ProjectManifest` with `#[serde(default, rename_all = "camelCase")]` for every optional group; `load(path)`, `manifest_dir`, `resolve(relative)`, `validate() -> Vec<Warning>` and hard errors.
2. **Planner** — `CompilePlan::from_manifest` → ordered `Step`s and a `ChapterPlan` per workbook (chapter number, title, workbook path, source kind). No document mutation in this phase.
3. **Conversion seam** — split `execute_convert_excel` into `convert_excel_to_package(path) -> ConvertedExcelPackage` (already exists as `execute_convert_excel`) and an internal `PackageWriter` that can write to a supplied directory; keep the default beside the workbook. Do **not** move the writer into another crate.
4. **Prefs** — `pubgen_project_path: String::new()` in `crates/engine/src/lib.rs`; doc string in `cmd/prefs.rs`.
5. **Tests**
   - defaults when only `chapters` is present;
   - relative paths resolve against the manifest folder, not the cwd;
   - empty/duplicate/missing chapters, >64 chapters, `.csv` chapter, unwritable output dir → named errors;
   - chapter order preserved; descending numbers warn;
   - planner step order for 1, 2 and 10 chapters;
   - `ChapterSource::Catalog` plans expose catalog + tables unchanged.

**Deliverable:** A manifest can be loaded and planned; nothing writes a document yet.
**Gate:** `cargo xtask ci` green.

---

### Phase 2 — Document skeleton, covers, front matter, chapters, numbering (~1.5 d)

1. **Document skeleton** — `file.new` + `layout.documentSetup` (margins, bleed, facing pages, title); `layout.parents.new {prefix: "A"}`.
2. **Covers** — reuse the existing cover functions with values mapped from the manifest; normalize the seed blank page; assert the order front (0), inside (1), then content.
3. **Front matter** — dedicated workbook or the leading narrative block of chapter 1; set `PubgenState` ranges; tag nothing yet.
4. **Chapters** — scratch state per chapter, convert (or use `Catalog`), generate pages; record per-chapter page deltas.
5. **Sections** — roman front matter + arabic restart, applied after all chapters are emitted (TOC placeholders exist but are blank in this phase; ordering is already final).
6. **Tests**
   - two synthetic catalogs (`Catalog` source, RTF + table fixtures in a tempdir) → one document, chapter 1 before chapter 2, both narrative and table pages present;
   - front matter order and signatory block;
   - page labels: `i, ii, …` before body, `1, 2, …` from the first chapter page;
   - the seed page is absent when it was blank and kept (with a warning) when it was authored;
   - a failing chapter stops the pipeline and leaves the document unsaved.

**Deliverable:** A two-chapter publication compiles with correct order and numbering.
**Gate:** `cargo xtask ci` green; `cargo run --release -p designcraft --features pubgen -- --sample` manual check.

---

### Phase 3 — TOCs, running headers, back cover (~1.25 d)

1. **TOC placeholders** — one page per requested list at the manifest position.
2. **TOC finalize** — generate contents/tables/charts through `generate_toc`; two-pass grow-only loop; record `pagesAdded` per pass; warning if pass 3 still grows.
3. **Running header** — character style + variable + header/footer apply on the body page range.
4. **Back cover** — once, after chapters, before TOC finalize.
5. **Tests**
   - Daftar Isi lists front matter and every chapter; Daftar Tabel lists table pages only; Daftar Grafik lists chart pages only;
   - pass 2 adds zero pages and page numbers equal `toc.entries`;
   - body header resolves to the chapter title on the divider and on a later table page, and changes at the next chapter;
   - front matter and back cover carry no header;
   - back cover is the last page.

**Deliverable:** The compiled publication has all global lists, running headers and a back cover.
**Gate:** `cargo xtask ci` green.

---

### Phase 4 — Job, panel card, command, headless (~1.0 d)

1. **CompileJob** — step machine, cancel flag, report, one step per frame; `sync` path for headless.
2. **Publication card** — `widgets.rs`: path picker, Compile/Cancel, progress, report; uses `Tokens` and `i18n`.
3. **UI command** — `pubgen.compileProject` in `UI_COMMANDS`; control channel invokes it with `{path, sync: true}`.
4. **Save/PDF** — save only after all steps; optional PDF; report paths.
5. **Tests**
   - job steps are monotonic and cancel stops before the next step;
   - `sync` and stepwise runs produce the same page count and report;
   - the command is registered and its params doc validates;
   - failure leaves no file at the output path and keeps any pre-existing file byte-identical.

**Deliverable:** One click and one control call compile a manifest.
**Gate:** `cargo xtask ci` green; wasm build green (stub).

---

### Phase 5 — Hardening and closeout (~0.65 d)

1. File guard: allow-list `compile.rs` with a reason (manifest reads, output checks).
2. Bounds: 64 chapters, path/title lengths, `page_count` growth checks, saturating arithmetic in progress counts.
3. Polish errors: every message names `step`, `chapter` and the underlying engine error.
4. Docs: roadmap §3 status → implemented, M5 row checked; `ROADMAP.md` progress line; this document's revision log.
5. Run `cargo xtask assets`, `cargo xtask layers`, full CI, and a manual compile of two real chapter workbooks.

**Gate:** full CI green; manual publication opens, prints and exports.

**Effort:** 0 ≈ 0.1 · 1 ≈ 1.0 · 2 ≈ 1.5 · 3 ≈ 1.25 · 4 ≈ 1.0 · 5 ≈ 0.65 → **≈ 5.5 d**.

## 7. Risks & Mitigations

| Risk | Mitigation |
| :--- | :--- |
| TOC page numbers drift when a list grows | Placeholders before body; grow-only two-pass finalize; pass 2 must report zero growth; test compares against `toc.entries` |
| A shrinking TOC leaves blank continuation pages | Content does not change between passes, so growth is monotonic; shrinking is out of scope for M5 and reported if detected |
| Running header changes TOC membership | Use a character style run, never the `TOC Chapter` paragraph style |
| Seed blank page from `file.new` survives | Explicit normalize step after covers, with an empty-page check and a test |
| Chapter 1 supplies both front matter and body | Split by catalog index and emit two ranges; warn when no leading narrative block exists |
| Styles/swatches differ between chapters | One document means first definition wins; document this and keep `book.syncStyles` out of the path |
| Table CSV name collisions across workbooks | Conversion stays beside each workbook (existing behaviour); catalog names are stem-prefixed; detect duplicate generated table paths in the plan and error |
| AI cover network failure | Manifest style ≥ 100 is opt-in; a failure stops the compile before any save with a named error |
| Long compile blocks the UI | One step per frame; each chapter is the coarse unit; Cancel is honored between steps |
| Large workbooks exhaust memory | Drop the conversion package after each chapter; cap chapters; no matrix cache retained |
| Output document open in another tab | Compare canonical paths during validation and refuse with instructions |
| Partial output after failure | Save is the second-to-last step; failure leaves only an unsaved document |
| wasm build breaks | Whole module native-only with a stub; card hidden |
| Hostile manifest values | Bounded counts/strings, `get()` over indexing, checked page arithmetic, no network unless the cover style requires it |

## 8. Out of Scope

- Per-chapter `.dcbook` output (`pubgen.splitBook`).
- Incremental recompiles that only rebuild changed chapters.
- AI translation of whole chapters during compile; the compiler uses authored English and `translate_with_ai = false`.
- Vector chart engine (Pillar 1 / M4) and the QA preflight (Pillar 3).
- PDF/X-4 and web export profiles (M6), universal publication profiles (M7).
- Splitting one workbook into several chapters; the manifest maps one workbook to one chapter.
- Front-matter templates beyond the existing `front_matter/` and `dividers/` assets.
- EPUB output.

## 9. Dependency Impact

| Crate | Change | New deps |
| :--- | :--- | :--- |
| `designcraft-engine` | `Prefs.pubgen_project_path` | None |
| `designcraft-ui-egui` | `compile.rs`, panel card, UI command | None — reuses `serde_json`, `std::fs`, existing XLSX reader |
| Others | None | None |

## 10. Definition of Done

- [ ] `pubgen_project.json` loads with defaults, resolves paths against its own folder, and rejects hostile/invalid manifests with named errors.
- [ ] One compile from two or more workbooks produces a single `.designcraft` in manifest order: front cover, inside cover, front matter, Daftar Isi/Tabel/Grafik, chapters, back cover.
- [ ] Front matter is lowercase roman; body restarts at arabic 1 and runs continuously across chapters.
- [ ] The three global lists are correct and stable across a second generation pass.
- [ ] Running headers name the current chapter on every body page and are absent from front matter and back cover.
- [ ] A failing chapter leaves the previous output untouched and the unfinished document unsaved, with a report naming the step and workbook.
- [ ] The panel card compiles and cancels; `pubgen.compileProject` works through the control channel with `sync: true`.
- [ ] `cargo xtask ci` green (fmt, clippy, tests, assets, layers, wasm); wasm stub compiles.
- [ ] Roadmap §3 status and M5 are updated; `ROADMAP.md` has a progress line.

## 11. Decisions Taken (no open questions)

1. **Single compiled document** over `.dcbook`; splits are a later export.
2. **Front-matter workbook preferred**, first-chapter leading narrative rows as fallback.
3. **TOCs after front matter by default**, `"end"` available for archival layouts.
4. **Save only on success**; the compiled document stays unsaved on failure.
5. **Synchronous `sync` mode for headless**, frame-stepped job for the desktop panel.
6. **Character-style running header** to preserve `TOC Chapter` membership.
7. **Workbook paths only** in the manifest; `ChapterSource::Catalog` stays internal for tests.

## 12. Revision Log

**2026-10-27** — created, verified against `pubgen-0.7.X` @ `ebb8f51a`:

1. Confirmed no `pubgen.*` engine commands exist and kept orchestration in the UI layer, reachable through `UI_COMMANDS`.
2. Recorded the four code-level gotchas that shape the plan: the surviving seed page (`covers.rs:1976`), TOC autoflow growth (`cmd/toc.rs:631`), `RunningIndex` last-before-page semantics (`compose/src/vars.rs:127`), and `TOC Chapter` being both the narrative TOC filter and the divider title style.
3. Chose the grow-only two-pass TOC finalize instead of pre-computing reserved page counts, because entry counts and line heights are style-dependent.
4. Scoped the manifest to workbooks, with an internal catalog source for fast tests (no committed CSV/XLSX fixtures — `.gitignore` ignores `*.csv` and `plan/`).
5. Effort aligned with M5: ≈ 5.5 days.
