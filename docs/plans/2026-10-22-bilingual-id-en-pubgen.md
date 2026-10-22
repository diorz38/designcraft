# Bilingual (Indonesian–English) Generated Pages — Implementation Plan

> **Status:** READY (rev 4 — phased execution with Bilingual Toggle in Phase 1)
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

## 5. Phased Execution Plan

```
Phase 1: Bilingual Toggle UI & Locale Foundation
   │
   ▼
Phase 2: Ingestion & Workbook Parsing (Excel & RTF)
   │
   ├───────────────────────────┬───────────────────────────┐
   ▼                           ▼                           ▼
Phase 3: Tables & Charts    Phase 4: Dividers & Chapters   Phase 5: Covers & Templates
(Inline Bilingual)          (Dual Spreads / Slotted)       (Cover Templates & Slots)
   │                           │                           │
   └───────────────────────────┼───────────────────────────┘
                               │
                               ▼
Phase 6: Quality Gates, Regressions & Visual Verification
                               │
                               ▼
Phase 7: UI Localization (Indonesian App Interface)
```

---

### Phase 1: Bilingual Toggle UI & Locale Foundation

**Goal:** Deliver the primary user-facing **Bilingual Toggle** in the Pubgen header band, wire up `PubgenState` persistence, establish the `pubgen_locale` helper module, and register the core language tag — maintaining 100% byte-identity for existing single-language runs.

- **Files:**
  - Modify: `crates/ui-egui/src/panels/pubgen.rs` (`pubgen_header_band`, `PubgenState`, TOC ternary replacement)
  - Create: `crates/ui-egui/src/panels/pubgen_locale.rs`
  - Modify: `crates/doc/src/attrs.rs` (`language_tag`)
  - Test: `crates/ui-egui/src/panels/pubgen_locale.rs` (unit tests), `crates/doc/src/attrs.rs` (unit tests)

- **Step 1.1: Add Bilingual Mode Toggle in Pubgen Header Band**
  - In `pubgen_header_band` (`pubgen.rs:500-522`), immediately after the `"Set Page Margin"` button in the horizontal layout, add the Bilingual Toggle:
    ```rust
    // after the Page Margin button inside the same horizontal row
    let label = if st.bilingual_output { "Dual Lang" } else { "Single Lang" };
    let tip = "Single Lang: generate the active output language only.\n\
               Dual Lang (Bilingual): generate both languages, per the sheet pattern rules (see docs/pubgen-guide.md).";
    if outline_button(ui, label, 90.0).on_hover_text(tip).clicked() {
        st.bilingual_output = !st.bilingual_output;
    }
    ```
  - Display immediate visual feedback in the status line (`"Mode: Dual Lang (Bilingual)"` / `"Mode: Single Lang"`).
  - Clear separation of concerns: this toggle controls *mode* (single vs dual/bilingual); `st.locale` controls *which* language when in single-language mode.
  - The toggle does not mutate the active document, catalog, or rows; it governs generation output mode.

- **Step 1.2: Add Bilingual State Fields to `PubgenState`**
  - Add fields with `#[serde(default)]` to `PubgenState` (`pubgen.rs:167-310`):
    ```rust
    #[serde(default)]
    pub locale: Locale,            // single-language mode target (default: Locale::Id)
    #[serde(default)]
    pub bilingual_output: bool,    // false = Single Lang (default), true = Dual Lang / Bilingual
    #[serde(default)]
    pub doc_language: String,      // default: "" (preserves existing run language; non-empty stamps language)
    ```
  - Ensure string serialization (`"id"` / `"en"`) for `Locale`.
  - Add unit test for serde round-trip: missing fields in old state JSON default gracefully without crashing or altering document output.

- **Step 1.3: Create `pubgen_locale.rs` Scaffolding**
  - Implement `Locale`, `LocaleWords`, and `words(locale: Locale) -> &'static LocaleWords`.
  - Implement parsing utilities:
    - `split_pair(s: &str) -> Option<(&str, &str)>` (splits on exact `" / "`, bare `/` returns `None`).
    - `split_table_title(s: &str) -> (String, Option<String>)`.
    - `split_chart_title_parts(s: &str) -> ChartTitleParts`.
    - `italicize_ranges(app: &mut DesignApp, story: u64, text: &str, ranges: &[(usize, usize)])` with char-to-byte conversion.
  - Implement `type_aliases`, `total_words`, `detect_from_records`.

- **Step 1.4: Refactor Existing TOC Chrome to Use `words(locale)`**
  - Replace the hardcoded Indonesian/English ternary in `pubgen.rs:6604-6613` with `words(st.locale)`.
  - Verify that single-language output is completely identical.

- **Step 1.5: Register Indonesian Language Tag in Core Doc**
  - In `crates/doc/src/attrs.rs`: add `"indonesian" => "id"` to `language_tag`.
  - Add unit test asserting `language_tag("Indonesian") == Some("id")`.

- **Verification:**
  - `cargo test -p designcraft-doc`
  - `cargo test -p designcraft-ui-egui --features pubgen -- pubgen_locale`
  - Interactive/headless verification: click Bilingual toggle in UI, verify state flips and UI reflects `"Dual Lang"` vs `"Single Lang"`.

---

### Phase 2: Ingestion & Workbook Parsing (Excel & RTF)

**Goal:** Parse bilingual metadata from Excel workbooks (`COVER`, `DIV`, `RXX`, `BXX`) and extract RTF language tagging (`\langN`), without altering single-language catalog schemas or breaking legacy workbooks.

- **Files:**
  - Modify: `crates/ui-egui/src/panels/pubgen_excel.rs`
  - Modify: `crates/ui-egui/src/panels/pubgen.rs` (`load_card1_csv`)
  - Modify: `crates/textimport/src/rtf.rs`
  - Test: `crates/textimport/tests` or `rtf.rs` inline tests, `pubgen_excel.rs` tests

- **Step 2.1: Excel Cover Parsing (`parse_cover_sheet`)**
  - In `pubgen_excel.rs:202-232`: add normalized key `"titleen"` mapping to `ExcelCoverData.title_en`.
  - Map `"titleid"`, `"title"`, `"title1"` to `title_id`.
  - **Critical:** `title2` must remain mapped to the second ID headline line (`title2`), NEVER aliased to `title_en`. If `title_en` is missing, `title_en` remains empty.

- **Step 2.2: Review & Divider Sheet Parsing**
  - `parse_rtf_review_sheet_with_template`: extract `file_id` (with aliases `file`, `rtf`, `path`) and optional `file_en` using `extract_field_for_keys`. Same for `BXX` front-matter sheets.
  - `parse_divider_sheet_with_template`: extract `title_id` (aliases `title`, `judul`) and `title_en`.

- **Step 2.3: Catalog CSV Generation (`convert_excel_to_csv_data`)**
  - Add `title_en` and `file_en` to `CatalogEntryItem`.
  - In `convert_excel_to_csv_data` (`pubgen_excel.rs:922`): append `,title_en,file_en` at the very end of the header and rows **only when at least one row has a non-empty EN value**.
  - Snapshot test: legacy workbooks emit byte-identical CSV (no extra trailing commas if no EN data).

- **Step 2.4: Card 1 Ingestion UI (`load_card1_csv`)**
  - In `pubgen.rs:4370`: look up `title_en` and `file_en` by header name, avoiding positional assumptions.
  - Guard `headers.len() >= 4` checks; display bilingual badge in Card 1 when dual-language columns exist.

- **Step 2.5: RTF Language Tag Extraction (`rtf.rs`)**
  - Parse `\langN`, `\deflangN`, `\langfe` control words.
  - Support LCID 14345 (0x3809) and 1057 (0x0421) -> `"Indonesian"`.
  - Set `CharAttrs.language`. Reset on `\pard` and group pop.

- **Verification:**
  - `cargo test -p designcraft-textimport`
  - `cargo test -p designcraft-ui-egui --features pubgen -- pubgen_excel`

---

### Phase 3: Inline Bilingual Generation — Tables & Charts

**Goal:** Generate single-page inline bilingual content for data tables (`XX_XX`) and charts (`CXX_XX`) when `bilingual_output` is enabled.

- **Files:**
  - Modify: `crates/ui-egui/src/panels/pubgen.rs` (table renderers, chart renderers)
  - Modify: `crates/ui-egui/src/panels/pubgen_locale.rs`
  - Modify: `assets/templates/data_tables/001_single_table_standard.designcraft`
  - Test: `pubgen.rs` table/chart bilingual tests

- **Step 3.1: Table Inline Title Generation (`XX_XX`)**
  - Use `split_table_title`: splits on `" / "`. Line 1 = ID (roman), Line 2 = EN (italic).
  - Code-path renderers (Presets 0–5 in `pubgen.rs:5300-5460`):
    - Emit two-row header chrome: `Tabel` on row 1, `Table` on row 2 (italic).
    - Emit two-line title: ID line, then EN line with `italicize_ranges`.
  - Template path:
    - In `001_single_table_standard.designcraft`: add `#slot:text:table_kind_en` (`"Table"`, italic) and `#slot:text:table_title_en` (italic).
    - Adjust geometry: expand title frame height and shift `table:main` downward to prevent overlap.
    - Bind `table_num` with number only (`"01.01"`), plus the new EN slots.

- **Step 3.2: Chart Bilingual Title & Italicization (`CXX_XX`)**
  - In `pubgen.rs:5198-5241`: parse chart title with `split_chart_title_parts`.
  - Format: `"<ID kicker> / <EN kicker> <number> <ID subtitle> / <EN subtitle>, <year>"`.
  - Apply `italicize_ranges`: convert char offsets to UTF-8 byte offsets; apply `"fontStyle": "Bold Italic"` (preserving Arial 11 pt Bold).
  - Fix existing selection bug where char counts were passed as byte offsets to `text.select`.

- **Step 3.3: Permissive Row Condition Matching**
  - Update `matches_pubgen_row_condition`: recognize bilingual row keywords (`jumlah`, `kabupaten`, `kecamatan`, `tahun`, `total`, `sum`, `subtotal`, `district`, `subdistrict`, `year`).

- **Verification:**
  - `cargo test -p designcraft-ui-egui --features pubgen -- test_table_bilingual`
  - `cargo test -p designcraft-ui-egui --features pubgen -- test_chart_bilingual_parts`
  - Visual verification: check that table titles do not clip and chart italics land on exact characters.

---

### Phase 4: Multi-Spread Editorial Generation — Dividers & Chapters

**Goal:** Implement bilingual layout for divider pages (`DIV`) and sequential two-spread generation for narrative review chapters (`RXX`) and front-matter (`BXX`).

- **Files:**
  - Modify: `crates/ui-egui/src/panels/pubgen.rs` (`execute_generate_pages`)
  - Modify: `assets/templates/dividers/002_chapter_devider.designcraft`
  - Modify: `assets/templates/dividers/001_ulasan_chapter.designcraft`
  - Test: `pubgen.rs` editorial generation tests

- **Step 4.1: Divider Slotted Layout (`DIV`)**
  - In `002_chapter_devider.designcraft`:
    - Add `#slot:text:title_en` frame below `chapter_title`.
    - Adjust vertical bounds so `chapter_title` and `title_en` fit cleanly without colliding with the hero image starting at y 135.77.
    - Style: smaller font size, Accent swatch.
  - In `pubgen.rs`: bind `title_id` and `title_en` into template slots.

- **Step 4.2: Review Chapters Sequential Spreads (`RXX` / `Ulasan`)**
  - When `st.bilingual_output` is enabled: emit two spreads per RXX catalog row.
    - Spread 1 (ID): populate `chapter_num`, `chapter_title`, and `narrative_body` from `file_id`.
    - Spread 2 (EN): populate same template slots with EN body from `file_en`; derive EN chapter heading from EN RTF first paragraph (since catalog row carries no `title_en`).
  - Single-language mode: emit ID spread only.
  - Skip EN spread if `file_en` is missing.
  - Ensure generation idempotency: re-generating does not create orphan duplicate spreads.

- **Step 4.3: Front Matter Chapters (`BXX`)**
  - Handle `BXX` matching the RXX rule: Spread 1 (ID from `file_id`), Spread 2 (EN from `file_en`).
  - Maintain `AXX`/`GXX` as single-page (preserving RTF-authored language).

- **Step 4.4: Document Language Setting**
  - When `st.doc_language` is non-empty, apply `type.char {"attrs":{"language": st.doc_language}}` to generated text runs and pass `lang` to `template.page.apply`.
  - When `st.doc_language` is empty (`""`), leave run languages untouched.

- **Verification:**
  - `cargo test -p designcraft-ui-egui --features pubgen -- test_ulasan_dual_spread`
  - Visual verification: divider page shows balanced dual titles; RXX produces clean ID then EN spreads.

---

### Phase 5: Cover Generation & Template Slots

**Goal:** Support bilingual covers from the `COVER` sheet across all 10 cover templates, providing dedicated slots for English title blocks.

- **Files:**
  - Modify: `crates/ui-egui/src/panels/pubgen.rs` (`execute_generate_cover`, `apply_excel_cover`)
  - Modify: `assets/templates/covers/*.designcraft` (10 templates)
  - Test: `pubgen.rs` cover generation tests

- **Step 5.1: Cover Template Frame Slots**
  - Add `#slot:text:title_en` frame to all 10 cover template `.designcraft` files.
  - Specifically update single-title templates `000_wave_curves` and `003_chevron_medallion` to incorporate the second title slot.
  - Handle 2-spread templates `004` and `005` appropriately.
  - Ensure original art and geometries pass `cargo xtask assets`.

- **Step 5.2: Cover Execution Wireup**
  - In `apply_excel_cover`: populate `st.cover_title` (ID), `st.cover_title2` (ID second line), and new `st.cover_title_en` (EN block).
  - In `execute_generate_cover`: bind `title_en` into `#slot:text:title_en` when `st.bilingual_output` is true.
  - When `st.bilingual_output` is false, omit the EN slot and render single-language cover matching legacy output.

- **Verification:**
  - `cargo test -p designcraft-ui-egui --features pubgen -- test_cover_bilingual`
  - Generate covers for all 10 templates in both Single Lang and Dual Lang modes; verify visual layout.

---

### Phase 6: Quality Gates, Regressions & Visual Verification

**Goal:** Ensure all quality gates pass, regression suites verify byte-identity for single-language mode, and visual verification confirms clean layout.

- **Files:**
  - Test fixtures: `plan/pubgen/samples/`
  - Docs: `docs/pubgen-guide.md`, `docs/pubgen-guide_id.md`, `plan/pubgen/PLAN.md`

- **Step 6.1: Automated Quality Gates**
  - `cargo xtask ci`
  - Explicit pubgen gate: `cargo test -p designcraft-ui-egui --features pubgen`
  - Architecture checks: `cargo xtask layers`, `cargo xtask assets`, `cargo xtask wasm`

- **Step 6.2: Regression & Byte-Identity Suite**
  - Run regression test on legacy monolingual workbook: assert generated document and catalog CSV match baseline bytes.
  - Run bilingual test suite covering `split_pair`, `split_table_title`, `split_chart_title_parts`, RTF LCIDs 14345 & 1057, and serde round-trips.

- **Step 6.3: Visual Verification via Control Channel**
  - Run `cargo run --release -p designcraft -- --sample --control 7979`.
  - Capture screenshots using `ui.screenshot` and verify against layout expectations:
    - Table titles do not clip or overlap header rules.
    - Divider `title_en` sits comfortably above the hero image.
    - Chart italics apply to exact words.

- **Step 6.4: Documentation Updates**
  - Document per-sheet bilingual rules in `docs/pubgen-guide.md` and `docs/pubgen-guide_id.md`.
  - Log Phase 9 in `plan/pubgen/PLAN.md`.

---

### Phase 7: UI Localization (Indonesian App Interface)

**Goal:** Deliver Indonesian UI translation for the Pubgen panel and application chrome, keeping UI language strictly independent of publication output language.

- **Files:**
  - Create: `crates/ui-egui/src/i18n/id.rs`
  - Modify: `crates/ui-egui/src/i18n.rs`
  - Modify: `crates/ui-egui/src/dock_extended.rs`
  - Modify: `crates/ui-egui/src/panels/pubgen.rs`

- **Step 7.1: Indonesian Translation Dictionary**
  - Create `crates/ui-egui/src/i18n/id.rs` with translations matching `de.rs` / `fr.rs`.
  - Register `("id", "Bahasa Indonesia")` in `LANGUAGES` and wire `tr` dispatch.

- **Step 7.2: Pubgen Panel i18n Audit**
  - Route all Pubgen card headers, button labels, tooltips, and the Bilingual Toggle label (`"Single Lang"` / `"Dual Lang"`) through `i18n::tr`.
  - Ensure UI interface language selection is cleanly distinguished from `st.locale` (output language) and `st.bilingual_output` (bilingual toggle).

- **Verification:**
  - `cargo test -p designcraft-ui-egui`
  - Switch application interface language to `Bahasa Indonesia` in Preferences; verify all Pubgen cards and controls render in Indonesian.

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
