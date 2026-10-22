# Bilingual (Indonesian–English) Generated Pages — Implementation Plan

> **Status:** READY (rev 2 — mixed per-sheet model)
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
| Slot value splits across frames by lines | `split_slot_value` lines→words→chars | `template/types.rs`, spec `template/mod.rs:30` |
| TOC engine filters by word prefix | `pageTitle: ["Tabel","Table"]` | `engine/src/cmd/toc.rs:78,232-235,278-284` |
| `language_tag` knows no Indonesian *name* | `"indonesian"` absent; bare `"id"` works via locale-code path | `doc/src/attrs.rs:272-345` |
| No Indonesian hyphenation | `hypher 0.1.8` has no `indonesian` feature; `for_language("id")` → `None` | `compose/src/hyphen/mod.rs:81-95`, `compose/Cargo.toml:24` |
| UI i18n has 14 locales, no `id` | `LANGUAGES` list | `i18n.rs:20-37`, `i18n/*.rs` |
| Sheet types | `COVER`, `AXX`, `GXX`, `BXX`, `DIV`, `RXX`, `CXX_XX`, `XX_XX` (table) | `pubgen_excel.rs:159-600, 927-1020` |
| Cover keys today | `title1` / `title2` are two *lines of one language* | `pubgen_excel.rs:202-232` |
| Divider template slots | `chapter_num`, `first_word`, `chapter_title`, `image` | `002_chapter_devider/document.json` |
| Table template slots | `table_num`, `table_title`, `table:main`, `source` | `001_single_table_standard/document.json` |
| Ulasan template slots | `chapter_num`, `chapter_title`, `narrative_body` | `001_ulasan_chapter/document.json` |
| Chart pages have **no** template | chart title/image emitted directly by code | `pubgen.rs:5198-5241` |
| Table title frames are zero-height rules | `table_num` box `(36,36)-(348,36)`, `table_title` `(36,60)-(348,60)` | template `document.json` anchors |

---

## 1. Model: Mixed, driven by the sheet-name pattern

There is no single layout for "bilingual". Each sheet pattern gets its own rule, and the rule is **fixed by the pattern** — the user does not choose per row.

| Sheet pattern | Content source (ID) | Content source (EN) | Layout when bilingual |
|---|---|---|---|
| `COVER` | row `title_id` (today's `title1`) | row `title_en` (today's `title2`) | one cover page, two title blocks |
| `AXX` / `GXX` (publication note / narrative) | RTF (existing `file` key) | same RTF | unchanged, one page — the RTF itself is authored in the wanted language; identical for single- and dual-language publications |
| `BXX` (front-matter chapter) | RTF `file_id` (today's `file`) | RTF `file_en` | two consecutive pages, ID page then EN page (same rule as `RXX`; **[CONFIRM]** if BXX should instead stay one page) |
| `DIV` (divider / chapter opener) | row `title_id` (today's `title`) | row `title_en` | one divider page; new text box slot `title_en`, smaller font, different colour from the ID title |
| `RXX` (review chapter) | RTF `file_id` (today's `file`) | RTF `file_en` | **two pages**: page 1 = ID paragraph, page 2 = EN paragraph |
| `XX_XX` (data table) | title cell, ID part | title cell, EN part after `" / "` | one page: `Tabel` on row 1, `Table` on row 2 (row 2 italic); title line 1 = ID, line 2 = EN italic |
| `CXX_XX` (chart / figure) | single title cell | same cell, EN parts after `" / "` | one page: `Chart` italic, words between `" / "` and `","` italic |

Single-language mode (`bilingual_output = false`) renders only the `locale` half of each pattern and must stay byte-identical to today's output for `Locale::Id`.

### 1.1 Delimiters (exact)

- Table / chart separator is **`" / "`** — space, slash, space. A bare `/` is data (`Tenaga Medis/Dokter`) and must never split.
- Chart title shape: `"<ID kicker> / <EN kicker> <number> <ID subtitle> / <EN subtitle>, <year>"`, e.g.
  `"Grafik / Chart 02.01 Panjang Jalan Kecamatan Karawang Timur / Road Length in East Karawang District, 2026"`.
  Italic = the EN kicker (`Chart`) **and** everything between the second `" / "` and the last `","`; the trailing `, 2026` stays roman.
- Table title shape: `"<ID title> / <EN title>"`. Line 1 = ID (before `" / "`), line 2 = EN (after), italic.
- Both parse rules are pure string functions and belong in `pubgen_locale.rs`, not in the generator loop.

---

## 2. Architecture: one `LocaleWords` module + per-pattern split helpers

### 2.1 New module

Create `crates/ui-egui/src/panels/pubgen_locale.rs`, declared from `pubgen.rs` with `#[path]` (same trick `pubgen_excel.rs` uses at `pubgen.rs:18-19`). It owns every user-visible word Pubgen invents and every bilingual split rule:

```rust
pub struct LocaleWords {
    pub table: &'static str,      // "Tabel" / "Table"
    pub figure: &'static str,     // "Grafik" / "Chart"
    pub chapter: &'static str,    // "BAB" / "Chapter"
    pub review: &'static str,     // "Ulasan" / "Review"
    pub note: &'static str,       // "Keterangan" / "Note"
    pub divider: &'static str,    // "Pembatas" / "Divider"
    pub toc_contents: &'static str, // "Daftar Isi" / "Contents"
    pub toc_tables: &'static str,   // "Daftar Tabel" / "List of Tables"
    pub toc_figures: &'static str,  // "Daftar Grafik" / "List of Figures"
    pub page_column: &'static str,  // "halaman" / "page"
    pub source: &'static str,       // "Sumber" / "Source"
    pub total_words: &'static [&'static str],
    pub type_aliases: &'static [&'static str],
}

pub enum Locale { Id, En }
pub fn words(locale: Locale) -> &'static LocaleWords;
pub fn detect_from_records(type_col: usize, rows: &[Vec<String>]) -> Locale;

/// "ID / EN" -> ("ID", "EN"). None when there is no `" / "`.
pub fn split_pair(s: &str) -> Option<(&str, &str)>;

/// Two-line table title: ("ID", Some("EN")) or ("ID", None).
pub fn split_table_title(s: &str) -> (String, Option<String>);

/// Chart title parts with the exact italic spans a renderer must emphasise.
pub struct ChartTitleParts {
    pub id_kicker: String,        // "Grafik"   (roman)
    pub en_kicker: Option<String>,// "Chart"    (italic)
    pub number: String,           // "02.01"    (roman)
    pub id_subtitle: String,      // roman
    pub en_subtitle: Option<String>, // italic, up to the last comma
    pub suffix: String,           // ", 2026"   (roman)
    pub italic_ranges: Vec<(usize, usize)>, // char offsets into the joined display string
}
pub fn split_chart_title_parts(s: &str) -> ChartTitleParts;

/// Apply `type.char {fontStyle: Italic}` over char ranges of a story.
pub fn italicize_ranges(app: &mut DesignApp, story: u64, text: &str, ranges: &[(usize, usize)]);
```

Rules:
- Every literal listed in §0's hardcoded-words rows moves here — one match, one place to extend.
- `toc_kind_words`, `toc_kind_title`, `toc_header_line`, `is_toc_header_line`, `is_section_header` become locale-parameterized, not `if english { … } else { … }` (that ternary at `pubgen.rs:6604-6613` is the pattern to replace).
- Input *recognition* stays permissive: `type_aliases` accepts both languages on read (`Table`, `Chart`, `BAB`, `Chapter`, `Ulasan`, `Review`, `Note`, `Keterangan`).
- `italicize_ranges` takes **char** offsets. Byte offsets break on `U+2028`/non-ASCII; `text.chars()` indexing is mandatory.

### 2.2 State

Add to `PubgenState` (all `#[serde(default)]`):

```rust
pub locale: Locale,            // single-language mode: which half of each pattern renders
pub bilingual_output: bool,    // false = Single Lang (default), true = Dual Lang
pub doc_language: String,      // "Indonesian" / "English: USA" — char language on generated text
```

Serialization uses a string (`"id"`/`"en"`) for `locale`, not the enum.

There is deliberately **no** "dual document" mode: the chosen model puts both languages in the same publication, per pattern.

### 2.3 The one control: header toggle

`pubgen_header_band` (`pubgen.rs:500-522`) already draws the panel title `"Pubgen"` and the `"Set Page Margin"` button in one `ui.horizontal`. Add the language-mode toggle **there**, immediately after the Page Margin button:

```rust
// after the Page Margin button inside the same horizontal row
let label = if st.bilingual_output { "Dual Lang" } else { "Single Lang" };
let tip = "Single Lang: generate the active output language only.\n\
           Dual Lang: generate both languages, per the sheet pattern rules (see docs/pubgen-guide.md).";
if outline_button(ui, label, 90.0).on_hover_text(tip).clicked() {
    st.bilingual_output = !st.bilingual_output;
}
```

Rules:
- Default `false` → button reads `"Single Lang"` on first open and for every existing persisted state.
- One control, two states, no dropdown and no separate checkbox. `locale` (which single language) stays in its own existing Language block (P5) — the toggle only answers *how many* languages.
- Implied default single language when the user has never touched `locale` is `Locale::Id`.
- Toggle does not touch the document, the catalog, or any loaded rows; it only changes what the next generate call emits. No confirmation dialog.
- Label goes through `i18n::tr` in P5 (`"Single Lang"` / `"Dual Lang"` as keys).
- `pubgen.rs:518-521` status line is reused to echo the new mode after a toggle (`"Mode: Dual Lang"`), same colour convention.

Every phase below that says "single-language mode" / "bilingual mode" reads this flag.

---

## 3. Data source format per sheet pattern

### 3.1 Key naming (sheets)

Canonical keys, with today's names kept as read aliases so existing workbooks keep working:

| Pattern | Canonical | Read aliases | Notes |
|---|---|---|---|
| `COVER` | `title_id`, `title_en` | `title1` → `title_id`; `title2` → `title_en` | if `title_en` missing, `title2` stays a second ID line (legacy two-line ID title); if `title_id` missing, `title1`/`title` used |
| `DIV` | `title_id`, `title_en` | `title`, `judul` → `title_id` | plus existing `image`/`file`/`template` |
| `RXX`, `BXX` | `file_id`, `file_en` | `file`, `rtf`, `path` → `file_id` | `file_en` optional; missing → single-language only |
| `AXX`, `GXX` | `file` (unchanged) | `rtf`, `path`, `document` | RTF carries whatever language it is written in; no `_en` key |
| `XX_XX` | title cell uses `" / "` | none | **Excel content unchanged** |
| `CXX_XX` | title cell uses `" / "` | none | number still extracted from the same cell |

`parse_cover_sheet` (`pubgen_excel.rs:202`) currently maps `title1`/`title2`; move to `title_id`/`title_en` with the alias above. `extract_field_for_keys` / `extract_field_lenient` already handle the lenient `key;value`, `key:value`, `key = value`, `key | value` forms — reuse, do not re-implement.

### 3.2 Catalog CSV schema

Current: `type,number,title,file[,template][,image1,image2]`. Columns stay positional (`row.get(3)` = file, `row.get(4)` = template).

Append at the end, only when at least one row has a value:

```
…,title_en,file_en
```

- `title_en` is populated **by `DIV` only** (tables/charts carry their EN title inside the `title` cell via `" / "`; COVER is not in the catalog).
- `file_en` is populated **by `RXX`/`BXX`**.
- A row without EN data leaves the cell empty; status line reports the count.
- Legacy workbooks emit exactly the bytes they emit today (snapshot test).

### 3.3 RTF

- Parse `\langN`, `\deflangN`, `\langfe` in `crates/textimport/src/rtf.rs`; map LCID → language name; write to `CharAttrs.language` (`engine/src/cmd/text.rs:163-169`). Reset per group/`\pard` like `\sN`.
- LCID 1057 = Indonesian → `"Indonesian"`. Add `"indonesian" => "id"` to `language_tag` (`doc/src/attrs.rs:272`) so `is_english("Indonesian") == false`.
- No Indonesian hyphenation in `hypher 0.1.8`; do not add a dependency. Document the limitation.

---

## 4. Files that must change

| File | Change | Why |
|---|---|---|
| `crates/ui-egui/src/panels/pubgen_locale.rs` | **new** — `Locale`, `LocaleWords`, `split_pair`, `split_table_title`, `split_chart_title_parts`, `italicize_ranges`, alias/keyword tables, `detect_from_records` | Single source of chrome words and bilingual split rules |
| `crates/ui-egui/src/panels/pubgen.rs` | header-band `Single Lang`/`Dual Lang` toggle (§2.3); state (`locale`/`bilingual_output`/`doc_language`); literals → `words(locale)`; `toc_*`/`is_section_header` locale-parameterized; table renderer emits 2-line `Tabel`/`Table` + 2-line title with italic EN; chart renderer uses `ChartTitleParts` + `italicize_ranges`; `Ulasan` path emits ID page then EN page; `DIV` path places `title_en`; `doc_language` on generated runs | Core of the feature |
| `crates/ui-egui/src/panels/pubgen_excel.rs` | `ExcelCoverData.title_id/title_en` + alias mapping; `parse_rtf_review_sheet_with_template` reads `file_id`/`file_en`; `parse_divider_sheet_with_template` reads `title_id`/`title_en`; `parse_b_sheet` images unchanged; `convert_excel_to_csv_data` emits `title_en,file_en` only when present | Ingestion |
| `crates/textimport/src/rtf.rs` | `\lang`/`\deflang`/`\langfe` → `CharAttrs.language`; tests | RTF content language |
| `crates/doc/src/attrs.rs` | `language_tag`: add `"indonesian" => "id"` | Composition must not treat ID as English |
| `crates/engine/src/cmd/template/mod.rs` | optional `lang` param applied to bound text runs; document in the spec string | Template output carries language |
| `crates/engine/src/cmd/toc.rs` | no code change; callers pass locale words | Verify only |
| `assets/templates/dividers/002_chapter_devider.designcraft` | **new slot** `#slot:text:title_en` — smaller font than `chapter_title`, different swatch (palette `Accent`) | DIV English title |
| `assets/templates/dividers/001_ulasan_chapter.designcraft` | optional `#slot:text:chapter_title_en` for the EN page; otherwise the EN page reuses the same slots with EN values | Verify; add only if the EN page needs a distinct style |
| `assets/templates/data_tables/001_single_table_standard.designcraft` | **new slots** `#slot:text:table_num_en` (italic, directly under `table_num`) and `#slot:text:table_title_en` (italic, under `table_title`); grow the title band so two lines fit | Avoid `split_slot_value` line-splitting the EN value across frames |
| `assets/templates/front_matter/001_publication_note.designcraft` | verify `title` slot accepts `Keterangan`/`Note` | Chrome slot |
| `crates/ui-egui/src/i18n.rs` + `crates/ui-egui/src/i18n/id.rs` | **new** Indonesian UI table; add `("id", "Bahasa Indonesia")` | Panel chrome in Indonesian |
| `pubgen_excel.rs` tests + `pubgen.rs` `mod tests` | per-pattern bilingual cases (§6) | Gates |
| `plan/pubgen/samples/` (gitignored) | bilingual workbook fixture with `title_id`/`title_en`, `file_id`/`file_en`, and `" / "` titles | Regression fixtures |
| `docs/pubgen-guide.md`, `docs/pubgen-guide_id.md` | per-sheet bilingual rules table (§1) | Docs |
| `plan/pubgen/PLAN.md` | append Phase 9 | Tracking |

Not changed: `dock.rs`, `panels/mod.rs`, `i18n/{de,fr,…}.rs`, chart rendering geometry, table matrix insertion.

---

## 5. Phases

```
P1 Locale core ──► P2 Ingestion ──► P3 Generation (per pattern) ──► P4 Gates/docs
                                          │
                                          └──► P5 UI locale
```

### P1 — Locale core (no behaviour change)
1. Add `pubgen_locale.rs`: `Locale`, `LocaleWords` (ID + EN), `split_pair`, `split_table_title`, `split_chart_title_parts`, `italicize_ranges`, alias tables.
2. Add the three state fields with defaults (`Locale::Id`, `false`, `"Indonesian"`); serde round-trip test.
3. Add the `Single Lang`/`Dual Lang` toggle to `pubgen_header_band` (§2.3) — pure UI + one bool, no generation change yet.
4. Replace the TOC ternary with `words(locale)`; assert byte-identical output both locales.
5. `language_tag("Indonesian") == Some("id")` test in `crates/doc`.

### P2 — Ingestion
1. `ExcelCoverData.title_id/title_en` + alias fallback (`title1`/`title2`, `title`); test legacy two-line ID cover.
2. `parse_rtf_review_sheet_with_template`: `file_id` with `file`/`rtf`/`path` aliases, plus optional `file_en`. Same for `is_b_sheet` rows.
3. `parse_divider_sheet_with_template`: `title_id`/`title_en` (+ `title`/`judul` alias).
4. `convert_excel_to_csv_data`: `CatalogEntryItem` gains `title_en`/`file_en`; columns appended **at the end**, only when any row has a value. Two snapshot tests (legacy workbook, bilingual workbook).
5. `load_card1_csv` (`pubgen.rs:4370`): detect `title_en`/`file_en` headers by name; auto-set `locale` from `type` words; badge in Card 1.
6. RTF `\lang*` → `CharAttrs.language`; LCID 1057 and `\deflang` fallback tests.
7. No change to table/chart sheet parsing — their bilingual data is inside the title cell.

### P3 — Generation, per pattern
Thread `&LocaleWords` through `execute_generate_pages` (`pubgen.rs:4638`) and the code-path renderers.

Every bilingual branch below is gated on `st.bilingual_output` (the header toggle, §2.3). With the toggle on `Single Lang`, each pattern renders only the half selected by `st.locale` and the output must match today's pages.

1. **COVER** — `execute_generate_cover` (`pubgen.rs:1767`): ID block from `title_id` (+ `title2` legacy line), EN block from `title_en`. Single-language mode uses only the active locale's block.
2. **AXX/GXX** — no behaviour change; only `doc_language` is set.
3. **BXX** — treat as `Ulasan`: page 1 renders `file_id`, page 2 renders `file_en` (skipped in single-language mode; EN page only when `file_en` set). **[CONFIRM]** — if BXX must stay one page, drop the EN page and keep the ID one.
4. **DIV** — `title_id` → existing `chapter_title`/`first_word`/`chapter_num` slots; `title_en` → new `title_en` slot. Same for the duplicate code path at `pubgen.rs:5109`.
5. **RXX (`Ulasan`)** — emit two spreads per row: ID spread (`chapter_num`/`chapter_title`/`narrative_body` from `file_id`), then EN spread (same slots, values from `title_en`-equivalent — for RXX the EN body is `file_en`; the EN chapter heading comes from the EN RTF's own first paragraph unless a `title_en` key exists → **use `title_en` if present, else the EN RTF's first paragraph heading**). Same logic in both the template path (`pubgen.rs:4751`) and the code path (`pubgen.rs:4987`).
6. **XX_XX tables** — template path: bind `table_num="<num>"`, `table_num_en="Table"`, `table_title=<ID>`, `table_title_en=<EN italic>`. Code path: `split_table_title` → two frames or one frame with `FORCED_LINE_BREAK`, then `italicize_ranges` over the EN line. `Tabel`/`Table` word choice follows `words()` in single-language mode; in bilingual mode row 1 is always `Tabel`, row 2 always `Table` (per the chosen model), regardless of `locale`.
7. **CXX_XX charts** — `pubgen.rs:5198`: build the display string from `ChartTitleParts`, then `italicize_ranges` for `en_kicker` and `en_subtitle`. No template change (charts are code-only).
8. Set `doc_language` on every generated story (`type.char {"attrs":{"language": st.doc_language}}` after `text.select`), including slot-bound template text; add `lang` to the `template.page.apply` payload.
9. Row conditions: `total_words` = `jumlah/tahun/kabupaten/kab/kecamatan/kec/total/sum/subtotal/year/district/subdistrict` (permissive both ways).
10. TOC: entry text keeps the ID kicker + number; EN title appended as `" — <EN>"`? **[CONFIRM]** Default in this plan: TOC entry = `"<kicker> <number> <ID title>"` only, EN not listed; the EN page/chapter is reachable but unlisted. Decide before P3 finishes.

### P4 — Tests, gates, docs
1. Unit tests:
   - `split_pair`: `"A / B"` → `("A","B")`; `"Medis/Dokter"` → `None`; `"A/B"` → `None`.
   - `split_table_title`: ID-only, ID+EN, EN containing `/`.
   - `split_chart_title_parts` on the plan's example: italic ranges cover exactly `Chart`, and `Road…District` but not `, 2026`.
   - `toc_header_line` ID → `"Tabel\thalaman"`, EN → `"Table\tpage"`.
   - Cover `title_id`/`title_en` + legacy `title1`/`title2` fallback.
   - RTF `\deflang1057` → run language `"Indonesian"`; `\lang` resets per group.
   - `matches_pubgen_row_condition` matches `Jumlah` and `Total`.
   - Catalog conversion: legacy bytes identical; bilingual emits `title_en,file_en`.
   - `PubgenState` serde round-trip, including `bilingual_output` defaulting to `false` when absent from JSON.
   - Header toggle flips `bilingual_output` and leaves `locale`, rows and document untouched.
2. Gates: `cargo xtask ci`, `cargo check/test -p designcraft-ui-egui --features pubgen -- pubgen`, `cargo xtask layers`, `cargo xtask assets`, `cargo xtask wasm`.
3. Visual: `cargo run --release -p designcraft -- --sample --control 7979`, generate bilingual, `ui.screenshot`; check table two-line titles do not clip, divider `title_en` is smaller/differently coloured, chart italics land on the right characters.
4. Docs: per-pattern table (§1) in both guides; Phase 9 in `plan/pubgen/PLAN.md`.

### P5 — UI locale
1. `i18n/id.rs` full table following `i18n/de.rs`; register `("id", "Bahasa Indonesia")` in `LANGUAGES` and the `tr` dispatch.
2. `dock_extended.rs` "Pubgen" tab label through `tr`.
3. Audit remaining raw literals in the Pubgen cards (`"Columns:"`, `"Layout:"`, `"Scope:"`, `"TOC title:"`, `"Template:"`, `"Sheet table:"`, …).
4. Interface language, output language (`locale`) and language mode (`Single Lang`/`Dual Lang`) are three separate controls; tooltips document each. The mode toggle label also routes through `tr`.

---

## 6. Traps

1. **Positional catalog columns.** `template` is `row.get(4)`, `file` is `row.get(3)` (`pubgen.rs:4674`, `:4735`). Append `title_en,file_en` at the end; never insert mid-schema.
2. **`headers.len()` gates UI.** `card1_headers.get(4)` is the template badge (`pubgen.rs:580`), and `headers.len()` gates the auto-insert checkbox (`pubgen.rs:586`) and the file info line. Re-derive by named lookup after appending columns.
3. **Bare `/` is data.** Split on `" / "` only. `"Tenaga Medis/Dokter"`, `"km/jam"`, dates `2026/01` must survive untouched.
4. **Char offsets, not bytes.** `italicize_ranges` must count `char`s; the display string contains `U+2028` (one char, three bytes) and may contain non-ASCII.
5. **Slot splitting beats you.** `split_slot_value` splits a multi-frame slot value by lines→words→chars. Feeding `"ID / EN"` into a single slot would split it across frames. Use separate `*_en` slots for template-bound text.
6. **Zero-height title frames.** `table_num`/`table_title` in `001_single_table_standard` are zero-height rule frames; adding a second line there needs the band grown, not just text.
7. **DO NOT localize `type` on read.** Accept both words; only output words change.
8. **Empty EN must not overwrite ID.** `file_en`/`title_en` empty → fall back to ID value *and* flag the row in the status line; never silently emit an empty page.
9. **RTF `\lang` lives in groups.** Reset on `\pard`/group close or the last language leaks (same bug class as `\sN`).
10. **Language name vs tag.** `language_tag("Indonesian")` is currently `None` → `is_english` returns `true`.
11. **No Indonesian hyphenation.** EN copy hyphenates, ID does not — a visible difference. Accept and document, or set both to hyphenate-off.
12. **Text expansion.** EN ~10–20% longer; grow frames (`fitFrameToContent`) or reserve width in templates, never shrink type per language.
13. **Two-page chapters and idempotency.** Generating twice must not duplicate the EN spread. The RXX/BXX two-page path has to be keyed off the row, not off cursor position.
14. **Numbers are never reformatted.** `1.234,56` vs `1,234.56` is display-only; do not rewrite source cells.
15. **Undo.** Generation is already N history entries (`plan/pubgen/PLAN.md` gap 1); bilingual roughly doubles it. Do not add a new document per language.
16. **wasm.** Keep file I/O behind `app.services.pick_open` / `std::fs`; no `rfd`.

---

## 7. Deliberately out of scope

- Translating user content automatically (no MT, dictionary or lookup).
- Two separate output documents (`*.id.designcraft` / `*.en.designcraft`).
- Indonesian hyphenation patterns or a new crate dependency.
- RTL or CJK interaction.
- Reconciling `PRD.md`'s stale preset/style counts (separate docs task).

---

## 8. Acceptance criteria

1. The Pubgen header shows `Single Lang` / `Dual Lang` beside `Set Page Margin`; it starts as `Single Lang` and switches with one click.
2. A legacy monolingual workbook converts and generates exactly as today (`Locale::Id`, `bilingual_output = false`): same catalog bytes, same pages.
3. `COVER` reads `title_id`/`title_en` (with `title1`/`title2` still working) and renders both title blocks, each in its own language.
4. `DIV` shows the ID title and, below it, the EN title from `title_en` in a smaller font and a different colour.
5. `RXX` produces two pages — ID body from `file_id`, EN body from `file_en` — with no duplicated content on regeneration.
6. A table sheet whose title is `"ID / EN"` renders `Tabel` / `Table` (second row italic) and a two-line title with only the EN line italic; a title containing a bare `/` is unchanged.
7. A chart title `"Grafik / Chart 02.01 … / …, 2026"` renders `Chart` and the words between the second `" / "` and the last comma in italic, everything else roman.
8. Generated runs carry `language = "Indonesian"` / `"English: USA"`; RTF chapters carry the RTF's declared language.
9. UI language `Bahasa Indonesia` translates the Pubgen panel; output language stays independent.
10. `cargo xtask ci` passes including `--features pubgen`, `layers`, `assets`, `wasm`.
