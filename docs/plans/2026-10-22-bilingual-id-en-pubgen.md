# Bilingual (Indonesian–English) Generated Pages — Implementation Plan

> **Status:** READY
> **Date:** 2026-10-22
> **Target Branch:** `feature/pubgen-modular-templates`
> **Scope:** Pubgen (`crates/ui-egui/src/panels/pubgen*.rs`), Excel/RTF ingestion, template slots, TOC, UI locale
> **Reference:** `plan/pubgen/PLAN.md` (current state), `docs/plans/2026-10-20-data-driven-template-system.md`

---

## 0. Verified current state (do not re-derive)

| Claim | Reality | Anchor |
|---|---|---|
| Pubgen state is one flat struct, serde-persisted | `PubgenState`, ~90 fields, `#[serde(default)]` on newer ones | `pubgen.rs:167-310` |
| No document-language concept | `card1_*`, `card2_*`, `toc_*`, `cover_*` only | `pubgen.rs:167` |
| Language of *content* is never set | `frame.create`/`type.char` calls carry font/size, not `language` | `pubgen.rs:5527-5563` |
| Type words are matched by hardcoded literals | `"Ulasan"`, `"BAB"`, `"Tabel"`, `"Grafik"`, `"Note"`, `"COVER"` | `pubgen.rs:4630,4696,4932,5055,5696` |
| TOC chrome is hardcoded ID/EN pair | `["Tabel","Table"]`, `["Grafik","Chart"]`, `["BAB"]`, `"Daftar Table"`, `"\thalaman"` | `pubgen.rs:76-112, 6604-6619` |
| Czech/English row rules only | `jumlah/tahun/kabupaten/kab/kecamatan/kec/total` | `pubgen.rs:4391-4418` |
| Excel defaults emit Indonesian | `parse_catalog_entry` returns `"Tabel"`, RXX→`"Ulasan"` | `pubgen_excel.rs:117,153,156,336` |
| RTF importer discards `\lang` | no `lang`/`deflang` handling; only `\stylesheet` styles | `rtf.rs:293-588` |
| Template engine binds slots, no locale | `text_slots: HashMap<String,String>` | `template/mod.rs:123-292` |
| TOC engine filters by word prefix | `pageTitle: ["Tabel","Table"]` | `engine/src/cmd/toc.rs:78,232-235,278-284` |
| `language_tag` knows no Indonesian *name* | `"indonesian"` absent; bare `"id"` works via locale-code path | `doc/src/attrs.rs:272-345` |
| No Indonesian hyphenation | `hypher 0.1.8` has no `indonesian` feature; `for_language("id")` → `None` | `compose/src/hyphen/mod.rs:81-95`, `compose/Cargo.toml:24` |
| UI i18n has 14 locales, no `id` | `LANGUAGES` list | `i18n.rs:20-37`, `i18n/*.rs` |
| Covers/dividers/notes use `#slot:` tags | e.g. `chapter_num`, `chapter_title`, `narrative_body`, `table_num`, `source` | template `document.json` labels |
| Table `source` footnote is plain text, not a slot | `execute_insert_table_matrix` emits the footnote frame directly | `pubgen.rs:5711-6330`, template label `#slot:text:source` |

---

## 1. What "bilingual" means here — decision first

Three models, only one fits a statistical publication:

| Model | Shape | Verdict |
|---|---|---|
| **A. Two parallel documents from one workbook** | `BAB_01_2025.id.designcraft` + `.en.designcraft`, identical geometry, one source of truth | **Recommended default** |
| B. One document, dual-language columns per page | ID text frame left, EN right, same page | Optional later; needs new templates + narrower measure |
| C. One document, language toggled after generation | switch chrome words only | Insufficient: content itself is monolingual |

**Recommendation:** implement A as the product behaviour, with the content pipeline itself language-agnostic so B is a template change later, not a rewrite.

Definition used throughout:

- **Content language** = the words in the user's data (CSV/Excel/RTF). Never translated by the app.
- **Chrome language** = words the app supplies: TOC titles, table column headers, kickers (`BAB`/`Chapter`, `Tabel`/`Table`, `Grafik`/`Figure`), source labels (`Sumber`/`Source`), `halaman`/`page`, date/number formatting, page-number styles.
- The app must never mix them. Data language is data; chrome language is a single `LocaleWords` value.

---

## 2. Architecture: one `LocaleWords` module, one flag

### 2.1 New module

Create `crates/ui-egui/src/panels/pubgen_locale.rs`, declared from `pubgen.rs` with `#[path]` (same trick `pubgen_excel.rs` uses at `pubgen.rs:18-19`). It owns every user-visible word Pubgen invents:

```rust
pub struct LocaleWords {
    pub table: &'static str,      // "Tabel" / "Table"
    pub figure: &'static str,     // "Grafik" / "Figure"
    pub chapter: &'static str,    // "BAB" / "Chapter"
    pub review: &'static str,     // "Ulasan" / "Review"
    pub note: &'static str,       // "Keterangan" / "Note"
    pub divider: &'static str,    // "Pembatas" / "Divider"
    pub toc_contents: &'static str, // "Daftar Isi" / "Contents"
    pub toc_tables: &'static str,   // "Daftar Tabel" / "List of Tables"
    pub toc_figures: &'static str,  // "Daftar Grafik" / "List of Figures"
    pub page_column: &'static str,  // "halaman" / "page"
    pub source: &'static str,       // "Sumber" / "Source"
    pub total_words: &'static [&'static str], // row-condition keywords
    pub type_aliases: &'static [&'static str], // accepted input words for `Tabel`
}

pub fn words(locale: Locale) -> &'static LocaleWords;
pub enum Locale { Id, En }
pub fn detect_from_records(type_col: usize, rows: &[Vec<String>]) -> Locale; // "Table" → En
```

Rules:
- Every literal listed in §0's hardcoded-words rows moves here — one match, one place to extend.
- `toc_kind_words`, `toc_kind_title`, `toc_header_line`, `is_toc_header_line`, `is_section_header` become locale-parameterized, not `if english { … } else { … }` (that ternary at `pubgen.rs:6604-6613` is the pattern to replace).
- Input *recognition* stays permissive: `type_aliases` must still accept both languages on read (`Table`, `Chart`, `BAB`, `Chapter`, `Ulasan`, `Review`, `Note`, `Keterangan`) so an Indonesian workbook still converts under an English target and vice versa.

### 2.2 State

Add to `PubgenState` (all `#[serde(default)]`, so existing temp state deserializes):

```rust
pub locale: Locale,            // chrome language for generated output
pub bilingual_output: bool,    // generate both `.id` and `.en` outputs
pub doc_language: String,      // "Indonesian" / "English: USA" — set as char language on generated text
```

Serialization uses a string (`"id"`/`"en"`), not the enum, to keep the JSON stable.

---

## 3. Data source format (the real decision)

### 3.1 Excel workbook (primary input)

Keep **one workbook, one sheet set**. Bilingual values live in dedicated columns/sheets rather than a duplicate workbook:

- `COVER` sheet: optional `title1_en`, `title2_en`, `office_name_en` keys. `parse_cover_sheet` (`pubgen_excel.rs:202`) reads them into new `ExcelCoverData` fields; missing → falls back to the ID value.
- Catalog sheets (`RXX`, `BXX`, `CXX`, `A02`, `DIV`, table sheets): first row gains an optional second line or a paired column for the English title. **Chosen convention:** extra sheet-level key row `title_en;…` (same lenient key/value parser already used by `extract_field_for_keys`, `pubgen_excel.rs:239`) plus optional `image1`/`image2` already present for `BXX`.
- English RTF/asset files follow a suffix convention: `B01.en.rtf`, `02_01.en.csv`; `file_en` key in the sheet. Existing ID paths untouched.

Why not a separate `_EN.xlsx`: two workbooks drift; the catalog generator already stitches sheets into one catalog CSV (`convert_excel_to_csv_data`, `pubgen_excel.rs:922`), so the language belongs in the row, not the file.

### 3.2 Catalog CSV schema

Current: `type,number,title,file[,template][,image1,image2]` (`pubgen_excel.rs:1000-1080`).

Extend, keeping backward compatibility (column order is positional at `pubgen.rs:4613-4619` and `row.get(3)` for `file` at `pubgen.rs:4680` etc.):

```
type,number,title,file,template,image1,image2,title_en,file_en
```

- Emit `_catalog.id.csv` and `_catalog.en.csv` from the same workbook (English rows use `title_en`/`file_en`; a row without an English title reuses `title` and is flagged in the status line).
- Card 1 loads either catalog; `locale` decides chrome. Loading `.en.csv` should also set `locale = En` automatically (detect from header/type words) with a visible badge.
- Card 2 table CSVs: sheet `02_01` and `02_01_en` both convert; the English sheet feeds `card2_sheet_tables` under a `… (EN)` label. Numeric cells are copied byte-for-byte — **never reformat data numbers**, only chrome and number *formatting attributes*.

### 3.3 RTF

- Parse `\langN`, `\deflangN` (and `\langfe`) in `crates/textimport/src/rtf.rs`; map LCID → language name; write to `CharAttrs.language` (the same field `type.char` sets, `engine/src/cmd/text.rs:163-169`). Currently the importer leaves it unset, so every RTF run inherits English composition rules.
- LCID 1057 = Indonesian → `"Indonesian"`. Add the name to `language_tag` (`doc/src/attrs.rs:272`) so `is_english("Indonesian") == false` and quote/hyphenation rules stop assuming English.
- No Indonesian hyphenation dictionary exists in `hypher 0.1.8` (no `indonesian` feature). Do **not** add a dependency for this. `Lang::for_language("Indonesian")` returns `None` → justified text falls back to no hyphenation. Document that limitation; revisit only if hyphenation quality is reported as a defect.

---

## 4. Files that must change

| File | Change | Why |
|---|---|---|
| `crates/ui-egui/src/panels/pubgen.rs` | `locale`/`bilingual_output`/`doc_language` state; all literal word comparisons call `pubgen_locale::words`; `toc_kind_words/title/of_words`, `toc_header_line`, `is_toc_header_line`, `is_section_header` take `&LocaleWords`; `split_chapter_title`/`split_chart_title` take prefix words; `matches_pubgen_row_condition` takes keyword list; `execute_generate_pages` sets `language` on generated runs; `execute_generate_cover` passes localized labels; new "Language" UI block | Core of the feature |
| `crates/ui-egui/src/panels/pubgen_locale.rs` | **new** — `Locale`, `LocaleWords`, alias/keyword tables, `detect_from_records`, `toc_header` helper | Single source of chrome words |
| `crates/ui-egui/src/panels/pubgen_excel.rs` | `ExcelCoverData.title1_en/title2_en/office_name_en`; `parse_catalog_entry` returns locale-neutral/ID-EN pair; `parse_*_with_template` read `title_en`/`file_en`; `convert_excel_to_csv_data` emits `title_en,file_en` and both catalogs; `parse_cover_sheet` reads `_en` keys | Ingestion |
| `crates/textimport/src/rtf.rs` | `\lang`/`\deflang`/`\langfe` parsing → `CharAttrs.language`; tests | RTF content language |
| `crates/doc/src/attrs.rs` | `language_tag`: add `"indonesian" => "id"` (name path only; locale-code path already works) | Composition must not treat ID as English |
| `crates/engine/src/cmd/template/mod.rs` + `types.rs` | optional `lang` param applied to bound text runs; document it in the command spec string | Template output carries language |
| `crates/engine/src/cmd/toc.rs` | already accepts arbitrary `pageTitle` words — no code change; only callers pass locale words | Verify only |
| `crates/ui-egui/src/i18n.rs` + `crates/ui-egui/src/i18n/id.rs` | **new** full Indonesian UI table; add `("id", "Bahasa Indonesia")` to `LANGUAGES` | Panel chrome in Indonesian |
| `assets/templates/dividers/001_ulasan_chapter.designcraft` | optional `#slot:text:chapter_prefix` / localized labels; no new files | Templates stay language-neutral |
| `assets/templates/front_matter/001_publication_note.designcraft` | verify `title` slot accepts `Keterangan`/`Note`; add `#slot:text:source` if needed | Chrome slot |
| `assets/templates/data_tables/001_single_table_standard.designcraft` | ensure `#slot:text:table_num`/`table_title`/`source` exist (they do) | No change expected |
| `crates/ui-egui/src/panels/pubgen_excel.rs` tests + `pubgen.rs` `mod tests` | bilingual cases (§6) | Gates |
| `plan/pubgen/samples/` (gitignored) | `sample_chapters_en.csv`, bilingual workbook fixture, `B01.en.rtf` | Manual/regression fixtures |
| `docs/pubgen-guide.md`, `docs/pubgen-guide_id.md` | new "Bilingual output" section; reconcile stale preset counts (already noted in `plan/pubgen/PLAN.md` gap 5) | Docs |
| `plan/pubgen/PLAN.md` | append Phase 9 with this plan's tasks | Tracking |

Not changed: `dock.rs`, `dock_extended.rs`, `panels/mod.rs`, `i18n/{de,fr,…}.rs`, cover templates (labels are already slots).

---

## 5. Phases

```
P1 Locale core ──► P2 Ingestion ──► P3 Generation ──► P4 Dual output
                                    │
                                    └──► P5 UI locale (+ P6 gates/docs)
```

### P1 — Locale core (no behaviour change yet)
1. Add `pubgen_locale.rs` with `Locale`, `LocaleWords` (ID + EN tables), `type_aliases`, `total_words`, `detect_from_records`.
2. Add the three `PubgenState` fields with defaults (`Locale::Id`, `false`, `"Indonesian"`); prove old states still load (serde round-trip test).
3. Replace the existing `if english { … } else { … }` TOC ternary with `words(locale)` — assert byte-identical output for both locales against current tests.
4. `language_tag("Indonesian") == Some("id")` test in `crates/doc`.

### P2 — Ingestion
1. `ExcelCoverData` `_en` fields + `parse_cover_sheet`; missing keys fall back to ID (test).
2. `convert_excel_to_csv_data` emits `title_en,file_en` **only when the workbook has any English value**, preserving the exact current column count otherwise (two snapshot tests: legacy workbook, bilingual workbook).
3. Emit `_catalog.id.csv` + `_catalog.en.csv`; status line reports which rows lacked English.
4. `load_card1_csv` (`pubgen.rs:4315`): detect column names `title_en`/`file_en`; auto-set `locale` from `type` words; badge in Card 1.
5. RTF `\lang*` → `CharAttrs.language`; LCID 1057 and `\deflang` fallback tests.
6. Card 2: recognize `_en` sheet tables; label them in the sheet selector.

### P3 — Generation
1. Thread `&LocaleWords` through `execute_generate_pages` (`pubgen.rs:4583`): kicker defaults, chart prefix, chapter prefix, TOC tag line, status strings.
2. `split_chapter_title`/`split_chart_title`: accept a prefix-word list instead of `"BAB"`/`"Grafik"`; keep both language words accepted on input.
3. Footer/source row: `detect_footnote_row` stays as-is (structure-based), but emit `"{source} {text}"` from `LocaleWords` when the row has no explicit label.
4. Row conditions: pass `total_words` (ID + EN: `jumlah/tahun/kabupaten/kab/kecamatan/kec/total/sum/subtotal/year/district/subdistrict`). Keep permissive matching so mixed-language tables still highlight.
5. Table header: `(1), (2), …` unchanged; the `Tabel/halaman` header comes from `toc_header_line(plan, words)`.
6. Set character language on every generated story (`type.char {"attrs":{"language": st.doc_language}}` after `text.select`), including slot-bound template text.
7. Templates: pass locale words into `text_slots` where a template expects a label; add `lang` to the `template.page.apply` payload.

### P4 — Dual output
1. `execute_generate_bilingual`: for each language in `[Id, En]` — clone the current source selection, build `PubgenState{locale, doc_language, ..}`, run the same generation into a new document (`file.new`), then rename/save as `<stem>.<locale>.designcraft`.
2. Guard: refuse dual generation to the *same* document; refuse when no English column data exists (return an explicit error, not a silent ID copy).
3. ZIP export: include both catalogs and both table sets; keep `{stem}_csvs.zip` name for compatibility, add `{stem}_csvs_bilingual.zip` only when dual requested.
4. Status line lists both documents and the count of fallback rows.

### P5 — UI locale
1. `i18n/id.rs`: complete table (follow `i18n/de.rs` shape — every key of the shared table plus the ID-specific CONTEXT rows).
2. Register `("id", "Bahasa Indonesia")` in `LANGUAGES` and the `tr` dispatch (`i18n.rs:4436`); `column()` untouched (5-column shared table not extended).
3. `dock_extended.rs` "Pubgen" tab label: pass through `tr` (closes known gap 3 in `plan/pubgen/PLAN.md`).
4. Pubgen card headings/labels already route through `tr` (`pubgen.rs:518-556` etc.) — audit and move the remaining raw literals (`"Columns:"`, `"Layout:"`, `"Scope:"`, `"TOC title:"`, `"Template:"`, `"Sheet table:"`, …) through `i18n::tr`.
5. Preferences: interface language and **output language** are separate controls; document the distinction in the tooltip.

### P6 — Tests, gates, docs
1. Unit tests (all in existing `mod tests`):
   - `toc_header_line` ID → `"Tabel\thalaman"`, EN → `"Table\tpage"`; figure variants.
   - `split_chapter_title` accepts `"BAB 01 X"` and `"Chapter 01 X"`; `split_chart_title` accepts `"Grafik"`/`"Figure"`.
   - `matches_pubgen_row_condition` matches `Jumlah` and `Total`.
   - Catalog conversion: legacy workbook byte-identical to today; bilingual workbook emits `title_en,file_en` + two catalogs.
   - Cover `_en` fallback.
   - RTF `\deflang1057` → run language `"Indonesian"`.
   - `execute_generate_pages` under `Locale::En`: title text uses `Chapter`, TOC tag still applied.
   - Dual output: two documents, English doc contains `title_en` and no ID chrome; missing `title_en` returns `Err`.
   - `PubgenState` serde round-trip with the new fields.
2. Gates before commit: `cargo xtask ci`, `cargo check/test -p designcraft-ui-egui --features pubgen -- pubgen`, `cargo xtask layers`, `cargo xtask assets`, `cargo xtask wasm`.
3. Visual check: `cargo run --release -p designcraft -- --sample --control 7979`, generate both languages, `ui.screenshot` and compare layout (English titles are wider — verify no overflow in the Editorial Chapter preset and in `data_tables/001_single_table_standard`).
4. Docs: "Bilingual output" section in both guides; append Phase 9 to `plan/pubgen/PLAN.md`; keep `plan/pubgen/PRD.md` out of scope (archived).

---

## 6. Traps (each one has burned a similar change)

1. **Positional catalog columns.** `execute_generate_pages` reads `type` via `card1_type_col` and `template` at `row.get(4)`, `file` at `row.get(3)` (`pubgen.rs:4613-4619`, `:4680`). Append `title_en,file_en` at the **end**; never insert mid-schema.
2. **DO NOT localize `type` on read.** Recognition must accept both; only *output* words change. Otherwise a `Tabel` row silently stops dispatching on an English run.
3. **`card1_headers.get(4)` is the template badge** (`pubgen.rs:580`). Appending columns changes `headers.len()`, which gates the auto-insert checkbox (`pubgen.rs:586`) and the `file` info line (`pubgen.rs:4349`) — re-derive via named lookup, not length.
4. **`row.get(3)` is `file`** for every branch. `file_en` must not overwrite `file` when empty.
5. **RTF `\lang` lives in groups.** Reset it on `\pard`/group close like `\sN`, or the last language leaks through the document (same bug class as `\sN` — see `docs/plans/2026-10-21-rtf-stylesheet-and-toc-paragraph-plan.md` §0 trap 3).
6. **Language name vs tag.** `language_tag("Indonesian")` currently returns `None`; `is_english` then returns `true` and composition applies English rules. Add the name mapping before wiring `doc_language`.
7. **No Indonesian hyphenation.** Justified English copy hyphenates, Indonesian copy will not. That is a visible difference between the two outputs. Accept and document, or set both outputs to hyphenate-off for parity.
8. **Text expansion.** English is ~10–20% longer. Fix by making title frames grow (`fitFrameToContent`) or by reserving width in presets — not by shrinking type per language (breaks the shared geometry contract).
9. **Number formatting.** `1.234,56` (ID) vs `1,234.56` (EN) is a *display* concern. Do not rewrite source numeric strings in CSV conversion; only apply a `number_format` locale to table cell styles if/when the doc layer exposes one.
10. **Template slot division.** `split_slot_value` splits a multi-frame value by lines→words→chars (`template/types.rs`). An English title in a two-frame cover slot splits differently than the ID title. Verify covers per language.
11. **Undo.** Generation is still N history entries per `plan/pubgen/PLAN.md` gap 1. Dual output doubles that; the dual pipeline should create each document fresh (`file.new`) so undo granularity stays sane.
12. **wasm.** All new file I/O must stay behind `app.services.pick_open` / `std::fs` calls in the same layering as today; do not add `rfd`.

---

## 7. Deliberately out of scope

- Translating user content automatically (no MT, no dictionary).
- Dual-language on the *same* page (model B) — template change and a new layout preset, not this plan.
- Extending the shared 5-column i18n `TABLE` (ID gets its own module, like `de`).
- Indonesian hyphenation patterns / a new crate dependency.
- RTL or CJK interaction.
- Reconciling `PRD.md`'s stale preset/style counts (separate docs task).

---

## 8. Acceptance criteria

1. One bilingual workbook converts to `_catalog.id.csv` + `_catalog.en.csv`; a monolingual workbook produces the current bytes unchanged.
2. `Locale::En` generation produces only English chrome (`Table 01.01`, `Chapter 01`, `List of Tables`, `Table\tpage`, `Source: …`) while preserving every source title verbatim.
3. `Locale::Id` output is byte-identical to today's output on the existing samples (`KBDA2026_chapters.csv`, `BAB_01_2025.xlsx`).
4. Generated runs carry `language = "Indonesian"`/`"English: USA"`; an RTF chapter's runs carry the RTF's declared language.
5. Dual output creates two documents with identical geometry and no cross-language content leakage; a missing English title returns an error naming the row.
6. UI language `Bahasa Indonesia` translates the Pubgen panel; the output-language control is independent of it.
7. `cargo xtask ci` passes including `--features pubgen`, `layers`, `assets`, `wasm`.
