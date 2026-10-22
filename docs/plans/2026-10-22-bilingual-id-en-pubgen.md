# Bilingual (Indonesian–English) Generated Pages — Implementation Plan

> **Status:** READY (rev 3 — mixed per-sheet model, verified against `acabc3af`)
> **Date:** 2026-10-22
> **Target Branch:** `feature/pubgen-modular-templates`
> **Scope:** Pubgen (`crates/ui-egui/src/panels/pubgen*.rs`), Excel/RTF ingestion, template slots, TOC, UI locale
> **Reference:** `plan/pubgen/PLAN.md` (current state), `docs/plans/2026-10-20-data-driven-template-system.md`

---

## 0. Verified current state (do not re-derive)

| Claim | Reality | Anchor |
|---|---|---|
| Pubgen state is one flat struct, serde-persisted | `PubgenState`, ~90 fields, `#[serde(default)]` on newer ones | `pubgen.rs:167-310` |
| No document-language concept | `card1_*`, `card2_*`, `toc_*`, `cover_*` only | `pubgen.rs:172` |
| Content language is never *chosen*, but it is always *set* | every generated run inherits `CharAttrs::language` default `"English: USA"` (the `frame.create`/`type.char` calls pass font/size only) | `doc/src/attrs.rs:1167`, `pubgen.rs:5648-5661` |
| Type words are matched by hardcoded literals | `"Ulasan"`, `"BAB"`, `"Tabel"`, `"Grafik"`, `"Note"`, `"COVER"` | `pubgen.rs:4673,4686-4691,4752,4764,4988,5243,5624-5640,5697` |
| TOC chrome is hardcoded ID/EN pair | `["Tabel","Table"]`, `["Grafik","Chart"]`, `["BAB"]`, `"Daftar Table"`, `"\thalaman"` | `pubgen.rs:81-112,6660-6673` |
| Czech/English row rules only | `jumlah/tahun/kabupaten/kab/kecamatan/kec/total` | `pubgen.rs:4447-4476` |
| Excel defaults emit Indonesian | `parse_catalog_entry` returns `"Tabel"`, RXX→`"Ulasan"` | `pubgen_excel.rs:117,153,156,336` |
| RTF importer discards `\lang` | no `lang`/`deflang` handling; only `\stylesheet` styles | `rtf.rs:293-588` |
| Template engine binds slots, no locale | `text_slots: HashMap<String,String>` | `template/mod.rs:123-292` |
| Slot value splits across frames by lines | `split_slot_value` lines→words→chars | `template/types.rs`, spec `template/mod.rs:30` |
| TOC engine filters by word prefix | `pageTitle: ["Tabel","Table"]` | `engine/src/cmd/toc.rs:78,232-235,278-284` |
| `language_tag` knows no Indonesian *name* | `"indonesian"` absent; bare `"id"` works via locale-code path (`locale_code` accepts a 2–3 letter language token) | `doc/src/attrs.rs:272-345,353-368` |
| `is_english("Indonesian")` is already `false` | `language_tag` returns `None` → `is_some_and` false; the name is *not* mis-detected as English | `compose/src/lib.rs:3960-3963` |
| No Indonesian hyphenation | `hypher 0.1.8` has no `indonesian` feature; `for_language("id")` → `None` | `compose/src/hyphen/mod.rs:81-95`, `compose/Cargo.toml:24` |
| UI i18n has 14 locales, no `id` | `LANGUAGES` list | `i18n.rs:20-37`, `i18n/*.rs` |
| Sheet types | `COVER`, `AXX`, `GXX`, `BXX`, `DIV`, `RXX`, `CXX_XX`, `XX_XX` (table) | `pubgen_excel.rs:159-600, 927-1020` |
| Cover keys today | `title1` / `title2` are two *lines of one language*, and since `acabc3af` `title2` owns the template's `title#2` frame (second headline line, smaller, accent colour) | `pubgen_excel.rs:202-232`, `pubgen.rs:285,1419,1773,1823` |
| Cover is **not** row-driven | `execute_generate_cover` reads `st.cover_*`; only the `COVER` sheet feeds them (`apply_excel_cover`), no catalog row | `pubgen.rs:1767,7831` |
| Cover templates have no EN slot | 10 files; `000_wave_curves` and `003_chevron_medallion` have exactly **one** `#slot:text:title` and no `title#2`; `004`/`005` carry 4 title slots across 2 spreads | `assets/templates/covers/*.designcraft` |
| Divider template slots | `chapter_num`, `first_word`, `chapter_title`, `image`; `chapter_title` y 72–134, image starts y 135.77 → **no vertical room** for a second title | `002_chapter_devider/document.json` |
| Table template slots | `table_num`, `table_title`, `table:main`, `source`; `table_num` holds the **number** (`"01.01"`), not the word "Tabel" | `001_single_table_standard/document.json` |
| Ulasan template slots | `chapter_num`, `chapter_title`, `narrative_body` | `001_ulasan_chapter/document.json` |
| Chart pages have **no** template | chart title/image emitted directly by code; title already `Arial Bold` 11 pt over the whole story | `pubgen.rs:5198-5241,5648-5661` |
| Table title frames are zero-height rules | `table_num` box `(36,36)-(348,36.5)`, `table_title` `(36,60)-(348,60.5)`, `table:main` starts y 100 | template `document.json` anchors |
| `text.select` takes **UTF-8 byte** offsets | `anchor`/`focus` are byte offsets; `floor_char_boundary` applied | `engine/src/cmd/text.rs:63,67-69` |
| Existing chart title selection is already short | `text.select` gets `text.chars().count()` as a *byte* offset | `pubgen.rs:5652` |
| `cargo xtask ci` does not build `--features pubgen` | no occurrence in `xtask/src/main.rs` or workflows; the `pubgen` runtime path is only checked when run explicitly | `xtask/src/main.rs`, `ui-egui/Cargo.toml:45` |
| Template `title#n` own-value support is landed | committed in `acabc3af` (`template/mod.rs`, `types.rs`, tests) — prerequisite, not pending | `engine/src/cmd/template/mod.rs:256-288` |

---

## 0.1 Verification pass (2026-10-22, against `acabc3af`)

Every §0 row was re-read from source. Three earlier claims were wrong and are corrected above; six are
new facts that change the work estimate. The plan is implementable, with no blocker.

**Corrections**
1. `is_english("Indonesian")` is already `false` — the old trap-10 rationale ("returns `true`") was wrong. The `"indonesian" => "id"` arm is still wanted, for shaping/hyphenation, not for `is_english`.
2. `doc_language` must default to `""`, not `"Indonesian"`: content language is *always* set today (to `"English: USA"`), so stamping Indonesian breaks the byte-identity criterion.
3. Covers are `COVER`-sheet-driven, not catalog rows, and `title2` is already a real second headline frame.

**New work the plan under-counted**
4. Cover EN block needs a frame in each of the 10 cover templates; two have a single title slot.
5. The divider has no vertical room under `chapter_title` (image starts y 135.77).
6. Table title chrome is six per-preset renderers, and `table_num` holds the number, not the word.
7. Chart italic must be `Bold Italic`, and `text.select` is byte-based — the existing call already passes a char count.
8. Catalog appends must not rely on `find_col` (scans `.take(3)`) or `headers.len() >= 4` gating.
9. `plan/pubgen/samples/DUAL_LANG/*.rtf` declare LCID **14345**, not 1057.

---

## 1. Model: Mixed, driven by the sheet-name pattern

There is no single layout for "bilingual". Each sheet pattern gets its own rule, and the rule is **fixed by the pattern** — the user does not choose per row.

| Sheet pattern | Content source (ID) | Content source (EN) | Layout when bilingual |
|---|---|---|---|
| `COVER` | `COVER` sheet key `title_id` (today's `title1` → `st.cover_title`) | `COVER` sheet key `title_en` (**new**; not today's `title2`) | one cover page: headline + second line (ID), then the EN headline block |
| `AXX` / `GXX` (publication note / narrative) | RTF (existing `file` key) | same RTF | unchanged, one page — the RTF itself is authored in the wanted language; identical for single- and dual-language publications |
| `BXX` (front-matter chapter) | RTF `file_id` (today's `file`) | RTF `file_en` | two consecutive pages, ID page then EN page (same rule as `RXX`; **[CONFIRM]** if BXX should instead stay one page) |
| `DIV` (divider / chapter opener) | row `title_id` (today's `title`) | row `title_en` | one divider page; new text box slot `title_en`, smaller font, different colour from the ID title |
| `RXX` (review chapter) | RTF `file_id` (today's `file`) | RTF `file_en` | **two pages**: page 1 = ID paragraph, page 2 = EN paragraph |
| `XX_XX` (data table) | title cell, ID part | title cell, EN part after `" / "` | one page: `Tabel` on row 1, `Table` on row 2 (row 2 italic); title line 1 = ID, line 2 = EN italic (**[CONFIRM]** case: today the renderer uppercases the type word to `TABEL`/`TABLE`) |
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

/// Apply italic over char ranges of a story: char→byte convert, then select + `type.char`.
/// Existing bold must survive: set `"fontStyle": "Bold Italic"` when the run is bold, else `"Italic"`.
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
pub doc_language: String,      // "" = leave each run's own language; else e.g. "Indonesian"
```

- **`doc_language` default is `""` (leave alone), not `"Indonesian"`.** Generated runs inherit `CharAttrs::language` = `"English: USA"` today (`doc/src/attrs.rs:1167`). Forcing `"Indonesian"` on every generated story changes composition (no ID hyphenation, different `locl`) and would break the byte-identity guarantee (§8.2). Only write a language when the field is non-empty.
- Serialization uses a string (`"id"`/`"en"`) for `locale`, not the enum.

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
| `COVER` | `title_id`, `title_en` | `title1` → `title_id`; **`title2` stays the second ID line** (it owns the `title#2` template frame since `acabc3af`) | `title_en` is a *new* key; it must **not** alias `title2`, or the existing second ID line renders as the EN block. If `title_en` is missing → single-language cover only. If `title_id` missing, `title1`/`title` used |
| `DIV` | `title_id`, `title_en` | `title`, `judul` → `title_id` | plus existing `image`/`file`/`template` |
| `RXX`, `BXX` | `file_id`, `file_en` | `file`, `rtf`, `path` → `file_id` | `file_en` optional; missing → single-language only |
| `AXX`, `GXX` | `file` (unchanged) | `rtf`, `path`, `document` | RTF carries whatever language it is written in; no `_en` key |
| `XX_XX` | title cell uses `" / "` | none | **Excel content unchanged** |
| `CXX_XX` | title cell uses `" / "` | none | number still extracted from the same cell |

`parse_cover_sheet` (`pubgen_excel.rs:202-232`) currently maps `title1`/`title2`; add a `title_en` arm and leave `title2` mapped to the second ID line. **Careful:** that function normalizes the key first — it strips `_`, space and `-` (`pubgen_excel.rs:213`) — so the match arms are `"titleid"`, `"titleen"`, not `"title_id"`/`"title_en"`.

`extract_field_for_keys` / `extract_field_lenient` already handle the lenient `key;value`, `key:value`, `key = value`, `key | value` forms — reuse, do not re-implement. Note `extract_field_lenient`'s last loop returns the first cell that is *not* the key, so it cannot be used to look for an optional second key in the same row; use `extract_field_for_keys` for `file_en`.

### 3.2 Catalog CSV schema

Current: `type,number,title,file[,template][,image1,image2]`. Columns stay positional (`row.get(3)` = file, `row.get(4)` = template). The writer is `convert_excel_to_csv_data` (`pubgen_excel.rs:922`) — it picks between the `template` and `image1,image2` variants (`:1027-1080`), so `title_en,file_en` has to be appended in **both** branches.

Append at the end, only when at least one row has a value:

```
…,title_en,file_en
```

- `title_en` is populated **by `DIV` only** (tables/charts carry their EN title inside the `title` cell via `" / "`; COVER is not in the catalog — it reaches the generator through the `COVER` sheet, §3.1).
- `file_en` is populated **by `RXX`/`BXX`**.
- A row without EN data leaves the cell empty; status line reports the count.
- Legacy workbooks emit exactly the bytes they emit today (snapshot test).
- Consequence for RXX: a catalog row has **no** `title_en`, so the EN chapter heading always falls back to the EN RTF's first paragraph (§5 P3.5). Do not write a branch for a `title_en` key that cannot occur.

### 3.3 RTF

- Parse `\langN`, `\deflangN`, `\langfe` in `crates/textimport/src/rtf.rs`; map LCID → language name; write to `CharAttrs.language` (`engine/src/cmd/text.rs:163-169`). Reset per group/`\pard` like `\sN`.
- **LCIDs actually present in the real workbooks** (`plan/pubgen/samples/DUAL_LANG/*.rtf`): `\deflang14345` / `\lang14345` (0x3809) and `\lang1025` (0x0401, Arabic). The plan's `1057` (0x0421, Indonesian) does **not** appear. Map 14345 → `"Indonesian"` **and** keep 1057; add a test over the sample corpus.
- Add `"indonesian" => "id"` to `language_tag` (`doc/src/attrs.rs:272`) — **not** because `is_english` currently mistakes it for English (it already returns `false`), but so `compose/shape.rs` gets a BCP-47 tag for `locl` and `hyphen::Lang::for_language` can decide deliberately.
- No Indonesian hyphenation in `hypher 0.1.8` (no `indonesian`/`malay` feature: checked the crate's feature list); do not add a dependency. Document the limitation.

---

## 4. Files that must change

| File | Change | Why |
|---|---|---|
| `crates/ui-egui/src/panels/pubgen_locale.rs` | **new** — `Locale`, `LocaleWords`, `split_pair`, `split_table_title`, `split_chart_title_parts`, `italicize_ranges`, alias/keyword tables, `detect_from_records` | Single source of chrome words and bilingual split rules |
| `crates/ui-egui/src/panels/pubgen.rs` | header-band `Single Lang`/`Dual Lang` toggle (§2.3); state (`locale`/`bilingual_output`/`doc_language`); literals → `words(locale)`; `toc_*`/`is_section_header` locale-parameterized; table renderer emits 2-line `Tabel`/`Table` + 2-line title with italic EN; chart renderer uses `ChartTitleParts` + `italicize_ranges`; `Ulasan` path emits ID page then EN page; `DIV` path places `title_en`; `doc_language` on generated runs | Core of the feature |
| `crates/ui-egui/src/panels/pubgen_excel.rs` | `ExcelCoverData.title_id/title_en` + alias mapping; `parse_rtf_review_sheet_with_template` reads `file_id`/`file_en`; `parse_divider_sheet_with_template` reads `title_id`/`title_en`; `parse_b_sheet` images unchanged; `convert_excel_to_csv_data` emits `title_en,file_en` only when present | Ingestion |
| `crates/textimport/src/rtf.rs` | `\lang`/`\deflang`/`\langfe` → `CharAttrs.language`; tests | RTF content language |
| `crates/doc/src/attrs.rs` | `language_tag`: add `"indonesian" => "id"` | Composition needs a BCP-47 tag for `locl`/hyphenation (not for `is_english`, already `false`) |
| `crates/engine/src/cmd/template/mod.rs` | optional `lang` param applied to bound text runs; document in the spec string | Template output carries language |
| `crates/engine/src/cmd/toc.rs` | no code change; callers pass locale words | Verify only |
| `assets/templates/dividers/002_chapter_devider.designcraft` | **new slot** `#slot:text:title_en` — smaller font than `chapter_title`, different swatch (`Accent`). **Needs a layout change, not just a slot:** `chapter_title` occupies y 72–134 and the hero image starts at y 135.77, so the band must be split or the image moved down | DIV English title |
| `assets/templates/dividers/001_ulasan_chapter.designcraft` | optional `#slot:text:chapter_title_en` for the EN page; otherwise the EN page reuses the same slots with EN values | Verify; add only if the EN page needs a distinct style |
| `assets/templates/data_tables/001_single_table_standard.designcraft` | **new slots** for the EN line: `#slot:text:table_kind_en` (`Table`, italic) and `#slot:text:table_title_en` (italic). Also repoint `table_num`/`table_title` geometry: they are 0.5 pt rules at y 36 / y 60 and `table:main` starts at y 100, so the band must grow and `table:main` shift down | Avoid `split_slot_value` line-splitting the EN value across frames |
| `assets/templates/front_matter/001_publication_note.designcraft` | verify `title` slot accepts `Keterangan`/`Note` (slots are `title`, `story:note_body` — verified) | Chrome slot |
| `assets/templates/covers/*.designcraft` (10 files) | **new EN title frame** per template. `000` and `003` have a single `title` slot; `004`/`005` have 4 title slots over 2 spreads. No shared geometry — each needs its own placement, and `ASSETS.md` rows if new files are added | Cover EN block (§1) |
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
2. Add the three state fields with defaults (`Locale::Id`, `false`, `""` — empty `doc_language` means "leave each run's existing language", required for §8.2 byte-identity); serde round-trip test including `""`.
3. Add the `Single Lang`/`Dual Lang` toggle to `pubgen_header_band` (§2.3) — pure UI + one bool, no generation change yet.
4. Replace the TOC ternary with `words(locale)`; assert byte-identical output both locales.
5. `language_tag("Indonesian") == Some("id")` test in `crates/doc`.

### P2 — Ingestion
1. `ExcelCoverData.title_id/title_en` + alias fallback (`title1`/`title` → `title_id`; `title2` stays the second ID line, never an EN alias); test legacy two-line ID cover.
2. `parse_rtf_review_sheet_with_template`: `file_id` with `file`/`rtf`/`path` aliases, plus optional `file_en`. Same for `is_b_sheet` rows.
3. `parse_divider_sheet_with_template`: `title_id`/`title_en` (+ `title`/`judul` alias).
4. `convert_excel_to_csv_data`: `CatalogEntryItem` gains `title_en`/`file_en`; columns appended **at the end**, only when any row has a value. Two snapshot tests (legacy workbook, bilingual workbook).
5. `load_card1_csv` (`pubgen.rs:4370`): detect `title_en`/`file_en` headers by name; auto-set `locale` from `type` words; badge in Card 1.
6. RTF `\lang*` → `CharAttrs.language`; LCID 14345 (what the sample corpus actually declares) and 1057, plus `\deflang` fallback tests.
7. No change to table/chart sheet parsing — their bilingual data is inside the title cell.

### P3 — Generation, per pattern
Thread `&LocaleWords` through `execute_generate_pages` (`pubgen.rs:4638`) and the code-path renderers.

Every bilingual branch below is gated on `st.bilingual_output` (the header toggle, §2.3). With the toggle on `Single Lang`, each pattern renders only the half selected by `st.locale` and the output must match today's pages.

1. **COVER** — `execute_generate_cover` (`pubgen.rs:1767`): ID block = `st.cover_title` (+ `st.cover_title2` on the `title#2` frame), EN block = new `st.cover_title_en` fed from the `COVER` sheet key `title_en`. Remember the cover is **not** catalog-driven: the change lands in `parse_cover_sheet` → `ExcelCoverData` → `apply_excel_cover` → `execute_generate_cover`, not in the `card1` row loop. Single-language mode uses only the active locale's block. The 10 cover templates each need an EN frame first (§4).
2. **AXX/GXX** — no behaviour change; only `doc_language` is set.
3. **BXX** — treat as `Ulasan`: page 1 renders `file_id`, page 2 renders `file_en` (skipped in single-language mode; EN page only when `file_en` set). **[CONFIRM]** — if BXX must stay one page, drop the EN page and keep the ID one.
4. **DIV** — `title_id` → existing `chapter_title`/`first_word`/`chapter_num` slots; `title_en` → new `title_en` slot (template first needs the band split, §4). Same for the duplicate code path at `pubgen.rs:5110`.
5. **RXX (`Ulasan`)** — emit two spreads per row: ID spread (`chapter_num`/`chapter_title`/`narrative_body` from `file_id`), then EN spread (same slots; body from `file_en`). The EN chapter heading comes from the EN RTF's own first paragraph — a catalog row **cannot** carry `title_en` (§3.2), so do not branch on it. Same logic in both the template path (`pubgen.rs:4752`) and the code path (`pubgen.rs:4988`). Generate only the ID spread in single-language mode; skip the EN spread when `file_en` is empty.
6. **XX_XX tables** — template path: bind `table_num="<num>"` (the number, as today), plus the new `table_kind_en="Table"` and `table_title_en=<EN>`. Code path: `split_table_title` → two frames or one frame with `FORCED_LINE_BREAK`, then `italicize_ranges` over the EN line.
   - **`Tabel`/`Table` is a change to what the slot holds.** Today `table_num` receives `num_val` (`"01.01"`), and the word `Tabel` comes from the source `type` cell, uppercased (`type_val.to_uppercase()` at four sites: `pubgen.rs:5311,5341,5394,5447`). Decide: uppercase `"TABEL"/"TABLE"` (consistent with today) or title-case `"Tabel"/"Table"` (what §1 says). **[CONFIRM]** — the model text and the renderer disagree today.
   - The code-path renderers are **per preset** (0–5, `pubgen.rs:5300-5460`) and preset 5 ("TOC Line") joins type+number+title into one line; the two-row chrome must be implemented in each preset that is meant to show it, not once.
7. **CXX_XX charts** — `pubgen.rs:5199`: build the display string from `ChartTitleParts`, then `italicize_ranges` for `en_kicker` and `en_subtitle`. No template change (charts are code-only).
   - `frame_with_centered_bold_title` (`pubgen.rs:5648`) applies `{"fontFamily":"Arial","fontStyle":"Bold","size":11.0}` over the **whole** story. Italic must therefore be `"Bold Italic"`, not `"Italic"`, or the shift is lost.
   - That same function passes `text.chars().count()` to `text.select` (`:5652`), but `text.select` reads **byte** offsets (`cmd/text.rs:63`). With `U+2028` in the chart string the selection already stops short. Fix this while adding `italicize_ranges` — it is the same char→byte conversion.
8. Set `doc_language` on generated stories **only when `st.doc_language` is non-empty** (`type.char {"attrs":{"language": st.doc_language}}` after `text.select`), including slot-bound template text; add `lang` to the `template.page.apply` payload under the same condition. Empty means "leave each run's own language", which is what makes §8.2 true.
9. Row conditions: `total_words` = `jumlah/tahun/kabupaten/kab/kecamatan/kec/total/sum/subtotal/year/district/subdistrict` (permissive both ways).
10. TOC: entry text keeps the ID kicker + number; EN title appended as `" — <EN>"`? **[CONFIRM]** Default in this plan: TOC entry = `"<kicker> <number> <ID title>"` only, EN not listed; the EN page/chapter is reachable but unlisted. Decide before P3 finishes.

### P4 — Tests, gates, docs
1. Unit tests:
   - `split_pair`: `"A / B"` → `("A","B")`; `"Medis/Dokter"` → `None`; `"A/B"` → `None`.
   - `split_table_title`: ID-only, ID+EN, EN containing `/`.
   - `split_chart_title_parts` on the plan's example: italic ranges cover exactly `Chart`, and `Road…District` but not `, 2026`.
   - `toc_header_line` ID → `"Tabel\thalaman"`, EN → `"Table\tpage"`.
   - Cover `title_id`/`title_en` + legacy `title1`/`title2` fallback: `title2` must still render as the **second ID line**, and `title_en` absent must leave the cover single-language.
   - RTF `\deflang14345` (the LCID the sample workbooks use) and `\deflang1057` → run language `"Indonesian"`; `\lang` resets per group.
   - `matches_pubgen_row_condition` matches `Jumlah` and `Total`.
   - Catalog conversion: legacy bytes identical; bilingual emits `title_en,file_en`; `card1_headers.len()` gates unchanged for a 4-column legacy catalog.
   - `PubgenState` serde round-trip, including `bilingual_output` defaulting to `false` and `doc_language` defaulting to `""` when absent from JSON.
   - Header toggle flips `bilingual_output` and leaves `locale`, rows and document untouched.
   - `italicize_ranges` maps char offsets to byte offsets: a range after `U+2028` in the chart title lands on the right characters.
2. Gates: `cargo xtask ci`, `cargo check/test -p designcraft-ui-egui --features pubgen -- pubgen`, `cargo xtask layers`, `cargo xtask assets`, `cargo xtask wasm`. **`cargo xtask ci` does not enable the `pubgen` feature** (verified: no reference in `xtask/src/main.rs` or the workflows), so the explicit `--features pubgen` run above is the only thing that compiles this code — treat it as a required gate, not an extra.
3. Visual: `cargo run --release -p designcraft -- --sample --control 7979`, generate bilingual, `ui.screenshot`; check table two-line titles do not clip, divider `title_en` is smaller/differently coloured, chart italics land on the right characters.
4. Docs: per-pattern table (§1) in both guides; Phase 9 in `plan/pubgen/PLAN.md`.

### P5 — UI locale
1. `i18n/id.rs` full table following `i18n/de.rs`; register `("id", "Bahasa Indonesia")` in `LANGUAGES` and the `tr` dispatch.
2. `dock_extended.rs` "Pubgen" tab label through `tr`.
3. Audit remaining raw literals in the Pubgen cards (`"Columns:"`, `"Layout:"`, `"Scope:"`, `"TOC title:"`, `"Template:"`, `"Sheet table:"`, …).
4. Interface language, output language (`locale`) and language mode (`Single Lang`/`Dual Lang`) are three separate controls; tooltips document each. The mode toggle label also routes through `tr`.

---

## 6. Traps

1. **Positional catalog columns.** `template` is `row.get(4)`, `file` is `row.get(3)` (`pubgen.rs:4675`, `:4758`, `:4926`); the type/number/title reads clamp to `.min(2)` (`:4669-4671`). Append `title_en,file_en` at the end; never insert mid-schema.
2. **`headers.len()` gates UI.** `card1_headers.get(4)` is the template badge (`pubgen.rs:580`), and `headers.len() >= 4` gates the auto-insert checkbox (`pubgen.rs:586`); `find_col` only scans the first three headers with a `.min(2)` fallback (`:4389-4396`). A 6-column bilingual catalog will trip these. Re-derive every one by named lookup after appending columns.
3. **Bare `/` is data.** Split on `" / "` only. `"Tenaga Medis/Dokter"`, `"km/jam"`, dates `2026/01` must survive untouched.
4. **Char offsets in, byte offsets out.** Split rules produce `char` offsets (the display string contains `U+2028`, one char / three bytes, and may contain non-ASCII); `text.select` takes **UTF-8 byte** offsets (`cmd/text.rs:63`). `italicize_ranges` converts. Note `frame_with_centered_bold_title` already gets this wrong today (`pubgen.rs:5652` passes a char count).
5. **Slot splitting beats you.** `split_slot_value` splits a multi-frame slot value by lines→words→chars. Feeding `"ID / EN"` into a single slot would split it across frames. Use separate `*_en` slots for template-bound text.
6. **Zero-height title frames.** `table_num`/`table_title` in `001_single_table_standard` are 0.5 pt rule frames (`y 36→36.5`, `y 60→60.5`) and `table:main` starts at y 100. Adding a second line needs the band grown **and `table:main` moved down**, not just text. The divider has the same problem: `chapter_title` ends at y 134, hero image starts at y 135.77.
7. **DO NOT localize `type` on read.** Accept both words; only output words change.
8. **Empty EN must not overwrite ID.** `file_en`/`title_en` empty → fall back to ID value *and* flag the row in the status line; never silently emit an empty page.
9. **RTF `\lang` lives in groups.** Reset on `\pard`/group close or the last language leaks (same bug class as `\sN`).
10. **Language name vs tag.** `language_tag("Indonesian")` is `None` today, but this does **not** make `is_english` return `true` (it returns `false`); the tag is needed for `locl` shaping and for a deliberate hyphenation decision. State the real reason in the code comment.
10b. **`title2` is not spare.** Since `acabc3af` it is the cover's second ID headline line and owns the `title#2` frame. Aliasing it to EN silently converts every existing two-line cover into a bilingual one.
10c. **The cover is sheet-driven, not catalog-driven.** Changing only the `card1` loop will not reach the cover; the path is `parse_cover_sheet` → `ExcelCoverData` → `apply_excel_cover` → `execute_generate_cover`.
10d. **`type.char` byte offsets.** `text.select` is byte-based; the existing chart title selection already uses a char count (`pubgen.rs:5652`). Convert before selecting, or the wrong characters get italicised.
10e. **`--features pubgen` is not in `ci`.** The whole panel is behind the feature; a green `cargo xtask ci` does not mean this code compiles.
10f. **Table chrome is per preset.** `Tabel`/`Table` rows exist in six code renderers, not one, and are uppercased today.
10g. **Cover template EN slots do not exist.** Both `000` and `003` have a single title slot; plan template authoring before P3.1.
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
2. A legacy monolingual workbook converts and generates exactly as today (`Locale::Id`, `bilingual_output = false`, `doc_language = ""`): same catalog bytes, same pages. **This requires `doc_language` to default to empty and to be applied only when set** — every generated run today carries the `"English: USA"` default, so stamping `"Indonesian"` would change hyphenation and `locl` and break byte-identity.
3. `COVER` reads `title_id`/`title_en` (with `title1` still working and `title2` still the second ID line) and renders both title blocks, each in its own language.
4. `DIV` shows the ID title and, below it, the EN title from `title_en` in a smaller font and a different colour.
5. `RXX` produces two pages — ID body from `file_id`, EN body from `file_en` — with no duplicated content on regeneration.
6. A table sheet whose title is `"ID / EN"` renders `Tabel` / `Table` (second row italic) and a two-line title with only the EN line italic; a title containing a bare `/` is unchanged.
7. A chart title `"Grafik / Chart 02.01 … / …, 2026"` renders `Chart` and the words between the second `" / "` and the last comma in italic, everything else roman.
8. When `doc_language` is set, generated runs carry it (`"Indonesian"` / `"English: USA"`); RTF chapters carry the language the RTF declares (`\lang14345` and `\lang1057` both → `"Indonesian"`).
9. UI language `Bahasa Indonesia` translates the Pubgen panel; output language stays independent.
10. `cargo xtask ci` passes, **plus** an explicit `cargo check/test -p designcraft-ui-egui --features pubgen` (the gate `ci` omits), `layers`, `assets`, `wasm`.
