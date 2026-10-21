# RTF Stylesheet Import + Multi-Level "TOC Paragraph" — Implementation Plan

> **Status:** READY
> **Date:** 2026-10-21
> **Target Branch:** `feature/pubgen-modular-templates`
> **Area:** `crates/textimport/src/rtf.rs`, `crates/ui-egui/src/panels/pubgen.rs`

---

## 0. Verified current state (do not re-derive)

| Claim | Reality | Anchor |
|---|---|---|
| RTF drops paragraph styles | `"stylesheet"` is in `SKIP`; importer returns `para_styles: vec![]` | `rtf.rs:48`, `rtf.rs:334` |
| Body paragraphs carry `\sN` | Yes: `\pard\plain \ltrpar\s2 …` | samples |
| `toc.generate` supports levels | Already: `entries[].level` 1–9, `TOC Level n` styles, indent `(n-1)*12` | `engine/src/cmd/toc.rs:95`, `:286` |
| PubGen emits multi-level TOC | No: one entry, and post-process flattens every line to `TOC Level 1` | `pubgen.rs:7371`, `pubgen.rs:7120-7123` |
| PubGen can scope to RXX pages | Partly: `pageTitle` word-prefix filter on the page's first text box already exists | `toc.rs:233`, `page_first_text` `toc.rs:254` |
| `TOC_KINDS` | `[&str; 3]` | `pubgen.rs:76` |

Sample evidence (gitignored `plan/pubgen/samples/`, verified):

- `R01..R06.rtf`: stylesheet defines `heading 1` (`\s1`, `\fs40`), `heading 2` (`\s2`, `\fs32`), `heading 3` (`\s3`). Body tags **only** `\s2`.
- `ulasan_bab_1.rtf`: body tags `\s1` and `\s2`.
- `A02..A04`, `G00..G02`: no body `\sN` tags (only `\s35` once). Contribution to a paragraph TOC is zero — scoping is effectively "RXX only".
- No `\csN` (character style) tags in any sample → character styles out of scope.

### Three traps the original plan got wrong

1. **`"stylesheet"` must stay in `SKIP`.** Removing it would emit the whole stylesheet table as body text. Parse the destination separately, keep skipping it in the main loop.
2. **Stylesheet names are lowercase.** Word writes `heading 1;`. `Styles::para` is exact-match (`crates/doc/src/styles.rs:752`), so the imported style would be `"heading 1"` while the TOC plan asks for `"Heading 1"`. **Must normalize names.**
3. **`\pard` resets paragraph style.** RTF `\pard` resets to the default paragraph style; body paragraphs are `\pard\plain …\s2 …`. If `\sN` is never cleared, every following paragraph inherits the last heading style.

Additionally: on `\par` the existing code writes `p.para = cur_para` (`rtf.rs:207-212`). The new `p.style` write goes beside it.

---

## 1. Task A — RTF stylesheet parsing (`crates/textimport/src/rtf.rs`)

### A1. Add a standalone stylesheet parser

Keep `SKIP` untouched. Before the main loop, add:

```rust
/// `\sN` index → the style it names. Parsed from the `\stylesheet` destination, which the body
/// loop keeps skipping.
fn parse_stylesheet(chars: &[char]) -> Vec<(u32, ImportedStyle)>
```

Steps:

1. Find the `\stylesheet` control word (word chars only, as the main loop does).
2. Walk forward with `depth = 1` (already inside the destination group) until `depth == 0`. Inside, a `{` raises depth, `}` lowers it. Escaped `\{`, `\}` and `\\` do not count — check for a preceding `\`.
3. At `depth == 1`, a `{` whose next control word is `s` (`{\s1 …}`) opens a style definition. Capture that inner group's chars (same brace matching, one level deeper).
4. From the definition, extract:
   - `\sN` → index (`None` → not a paragraph style, skip; `\csN` groups are ignored — they start with `c`, not `s`).
   - `\sbasedonN` → base index, resolved to a name after the loop.
   - `\b` / `\i` (honour `\b0` / `\i0`) → char style name via `"Bold" | "Italic" | "Bold Italic"`, mirroring `State::format` (`rtf.rs:25-33`). Store in `chars.font_style`.
   - `\fsN` → `chars.size = Some(N / 2.0)`, same 0.1–1296 clamp as the body (`rtf.rs:305-307`).
   - `\ql/\qr/\qc/\qj/\qd` → `para.align`.
   - `\li/\ri/\fi/\sb/\sa` → twips → pt (`param as f64 / 20.0`) with the same clamps as body (`rtf.rs:222-235`).
   - Style **name**: the plain-text segment preceding the terminating `;` (e.g. `…\styrsid4067997 heading 1;` → `heading 1`). Decode `\'hh` through the existing `cp1252` (`rtf.rs:337`). Ignore `\snextN`, `\slinkN`, `\spriorityN`, `\sqformat`, `\sautoupd`, `\sunhideused`, `\styrsidN`, `{\*\…}` groups.
5. Resolve `based_on` indices to names after the loop.
6. Normalize names — `fn normalize_style_name(raw: &str) -> String`:
   - `eq_ignore_ascii_case("heading N")` → `"Heading N"`
   - `"title"` → `"Title"`, `"subtitle"` → `"Subtitle"`, `"normal"` → `"[Basic Paragraph]"` (`story::BASIC_PARAGRAPH`)
   - otherwise capitalize the first letter, leave the rest.
   ```rust
   // ponytail: only the names the TOC plan asks for are canonicalized. Bodies with other named
   // styles keep Word's casing until a user maps them.
   ```

### A2. Bind paragraphs to styles in the main loop

New state next to `cur_para` (`rtf.rs:73`):

```rust
let styles = parse_stylesheet(&chars);          // index → name
let mut cur_style_idx: Option<u32> = None;
```

- control word `"pard"` (`rtf.rs:216`): also `cur_style_idx = None;` — but only outside `\listtext`, same guard as `cur_para`.
- control word `"s"` with `param` (`Some(n)` where `n >= 0`): `cur_style_idx = Some(n as u32)`. `\s0` means the default style → `None`.
- `"par" | "sect"` (`rtf.rs:207`) and the raw newline branch (`rtf.rs:144-152`): after `p.para = cur_para.clone()`, add `p.style = style_name(cur_style_idx)`, where the helper returns the mapped name or `BASIC_PARAGRAPH`.
- final paragraph write at `rtf.rs:326-330`: set `p.style` too.

`\sN` must be handled **after** the `skip` early-return so stylesheet-internal `\sN` never leaks (it already can't: the whole destination is skipped).

### A3. Return used styles only

At `rtf.rs:334`, replace the empty vectors:

```rust
let used: HashSet<String> = st.paras.iter().map(|p| p.style.clone()).collect();
let para_styles = keep_used(styles, &used);   // style + its based_on chain, as docx.rs:561-573
Ok(Imported { story: st, para_styles, char_styles: vec![], warnings })
```

`keep_used` is the same closure shape as `docx.rs:561`; copy it locally (4 lines) rather than exporting.

Style attributes stay on the style; run-level `\b`/`\fs` are **not** merged into runs (they are already present in typical Word output, and merging would duplicate the style's formatting).

### A4. Tests (`rtf.rs` `#[cfg(test)]`)

Use inline synthetic RTF strings — `plan/` is gitignored, so never depend on `plan/pubgen/samples/`.

1. `stylesheet_names_reach_paragraphs` — `{\stylesheet{\s1\b\fs40 heading 1;}{\s2\fs32 heading 2;}}` plus two body paragraphs tagged `\s1` / `\s2`. Assert `para_styles` names are `["Heading 1", "Heading 2"]`, `chars.font_style == Some("Bold")`, `chars.size == Some(20.0)` / `Some(16.0)`, and `story.paras[i].style` matches.
2. `pard_resets_paragraph_style` — paragraph with `\s1` then `\pard` paragraph → second paragraph style is `BASIC_PARAGRAPH`.
3. `stylesheet_text_never_becomes_body_text` — assert story text does not contain `"heading 1"` or `"sbasedon"`.
4. `only_used_styles_are_returned` — stylesheet with 3 styles, body uses 1 → `para_styles.len() == 1`.
5. `based_on_chain_is_kept` — `\s2\sbasedon1` where style 1 is used only as a base → both returned.

Existing `rtf` tests must keep passing (no style tags → `para_styles` empty, all paragraphs `BASIC_PARAGRAPH`).

---

## 2. Task B — "TOC Paragraph" kind + RXX scoping (`crates/ui-egui/src/panels/pubgen.rs`)

No engine changes. No new `PubgenState` fields — the level styles come from constants.

### B1. Kind tables (`pubgen.rs:76-104`)

```rust
pub const TOC_KINDS: [&str; 4] = ["TOC Entry", "TOC Table", "TOC Chart", "TOC Paragraph"];
```

- `toc_kind_words(3)` → `&["BAB"]` — scoping primitive. Verified: RXX pages' first text box is the `BAB 01` chapter line (`pubgen.rs:4930`), DIV pages' first box is the `BAB` prefix (`pubgen.rs:5040`), while table/chart pages start with `Tabel`/`Grafik`. So `pageTitle: ["BAB"]` excludes table and chart pages with zero new engine code.
- `toc_kind_title(3)` → `Some("Daftar Isi")`.
- `toc_kind_of_words` → add the `3` arm: `words` contains `bab` → `3`. Check it **before** the table/chart arms.
- `default_toc_title()` (`pubgen.rs:67`) unchanged.

The ComboBox at `pubgen.rs:3915-3921` iterates `TOC_KINDS`, so the new item appears with no UI edit. The title auto-fill at `pubgen.rs:3926-3927` already reacts to `toc_kind_title`.

### B2. Multi-level plan (`pubgen.rs:6573-6645`)

Replace the single `style` field with an ordered list:

```rust
struct TocLevel {
    style: String,
    level: u8,
    words: Vec<String>,
}

pub struct TocPlan {
    levels: Vec<TocLevel>,   // was: style: String
    title: String,
    words: Vec<String>,      // pageTitle filter of level 1 (kept for kind detection)
    layout: usize,
    hanging: bool,
    include_self: bool,
}
```

`from_state`:

```rust
let levels = if st.toc_kind == 3 {
    vec![
        TocLevel { style: st.toc_style.clone(), level: 1, words: vec!["BAB".into()] },
        TocLevel { style: "Heading 1".into(),     level: 2, words: vec![] },
        TocLevel { style: "Heading 2".into(),     level: 3, words: vec![] },
    ]
} else {
    vec![TocLevel { style: st.toc_style.clone(), level: 1, words: toc_kind_words(st.toc_kind).iter().map(|w| w.to_string()).collect() }]
};
```

`from_toc` (rebuild path): `toc.entries.len() > 1` → rebuild `levels` from the stored entries (style, level, `page_title`); otherwise the single-entry path as today.

`entry_json` → `entries_json` returning a `Value::Array`, one object per level:

```rust
{"style": l.style, "level": l.level, "pageNumber": "after", "separator": "\t"}
```
plus `"pageTitle": l.words` when non-empty. Call site: `pubgen.rs:7371` `"entries": [plan.entry_json()]` → `"entries": plan.entries_json()`.

Add `fn is_paragraph_toc(&self) -> bool { self.levels.len() > 1 }` for the branches below.

### B3. Preserve levels through post-processing

`ensure_styles` in the engine (`toc.rs:286`) already creates `TOC Level 1..3` and `fill` already tags each paragraph with the level's style (`toc.rs:373`). The flattening happens **only** in PubGen's post-process. Two edits:

1. `enrich_toc_story` (`pubgen.rs:6647`): early-return when `plan.is_paragraph_toc()`. Its catalog-record coalescing/prefixing is for the table/chart lists; for a paragraph TOC the engine output is already correct, and rewriting the text would desynchronize styles from paragraphs.
2. `apply_toc_post_process` (`pubgen.rs:7099-7137`): in the `layout != 1` branch, stop forcing `"TOC Level 1"`. Wrap the existing rewrite loop:

```rust
let style_name = if pi == 0 { "TOC Title" } else { "TOC Level 1" };
// The engine tags each entry with its own "TOC Level n": for a paragraph TOC leave the
// entries' styles alone; the single-level lists keep the historical flattening.
if plan.is_paragraph_toc() && pi > 0 {
    continue;
}
```

`pi == 0` (the title line) is still rewritten to `TOC Title` in both cases.

`ensure_toc_entry_style` (`pubgen.rs:6909`) needs no change: its non-TablePage branch already walks every style `starts_with("TOC Level")` and applies `indent = (n-1)*12 + hanging`, which is the requested L1=0 / L2=12 / L3=24 behaviour. Do **not** add the plan's 14/28 pt numbers.

Continuation headers (`sync_toc_continuation_headers`, `pubgen.rs:7203`) use `plan.title_or_contents()`; nothing to change.

### B4. Title/header text

`toc_header_line` / `is_toc_header_line` (`pubgen.rs:6532-6548`) only run for TablePage layout. Nothing to change.

---

## 3. Task C — Tests & gates (Task A + B)

### PubGen tests (`pubgen.rs` `#[cfg(test)]`)

1. `toc_paragraph_kind_tables` — `toc_kind_words(3) == ["BAB"]`, `toc_kind_title(3) == Some("Daftar Isi")`, `toc_kind_of_words(&["BAB".into()]) == 3`, and the existing kind 0/1/2 detection still holds.
2. `toc_paragraph_plan_has_three_levels` — `TocPlan::from_state` with `toc_kind: 3`, `toc_style: "TOC Entry"` → `levels` = `[("TOC Entry",1,["BAB"]), ("Heading 1",2,[]), ("Heading 2",3,[])]`; `entries_json()` round-trips them.
3. `toc_paragraph_scan_finds_styled_paragraphs` — build a doc: a chapter page with a `BAB 01` frame tagged `TOC Entry`, an RTF-placed story with paragraphs styled `Heading 1` / `Heading 2` (apply styles via `style.paragraph.apply` after `file.place`), and a table page whose first box starts `Tabel`. Run `toc.entries` with the 3-entry plan. Assert: chapter page listed at level 1, the heading paragraphs at levels 2/3, table page absent.
4. `toc_paragraph_keeps_toc_level_styles` — `execute_add_toc` with kind 3; assert the generated story's paragraphs use `TOC Title`, `TOC Level 1`, `TOC Level 2`, `TOC Level 3` in document order (and not all `TOC Level 1`).
5. `toc_paragraph_title_is_daftar_isi` — the frame's first line is `Daftar Isi`.

Reuse the existing synthetic-RTF pattern from `test_execute_generate_pages_ulasan_rtf_layout` (`pubgen.rs:11087`) — do not read `plan/pubgen/samples/`.

### textimport tests

Task A4 list. Add `#[cfg(test)] mod tests` cases only; no new fixtures on disk.

Regression to watch: `test_execute_page_from_rtf_sample_file` (`pubgen.rs:10190`) depends on gitignored samples — pre-existing, leave it alone.

### Gates

```
cargo fmt
cargo clippy -D warnings
cargo test -p designcraft-textimport
cargo test -p designcraft-engine
cargo test -p designcraft-ui-egui
cargo xtask layers
cargo xtask wasm
```

`never-crash` check for Task A: the stylesheet parser indexes input-derived chars. Every access uses `chars.get(i)`, brace depth is `saturating_sub`/bounded, style indices are `u32` from parsed params, and a malformed/unterminated stylesheet yields an empty map (warnings-free) rather than a panic. No `unwrap`/`expect` in the new code.

---

## 4. Order of work

1. **Task A** (RTF stylesheet) first — without it, level 2/3 entries cannot exist.
2. **Task B** (kind + plan + post-process) second.
3. **Task C** tests land with their task, not at the end.

One commit per task id, e.g.:

- `M?: rtf import stylesheet paragraph styles`
- `M?: pubgen TOC Paragraph multi-level kind`

---

## 5. Acceptance criteria

- [ ] RTF with `{\stylesheet{\s1 heading 1;}{\s2 heading 2;}}` imports styles named exactly `Heading 1` / `Heading 2`, and body paragraphs carry them.
- [ ] `\pard` resets the paragraph style; stylesheet text never reaches the story.
- [ ] Placing `R01.rtf` yields `Heading 2` paragraphs in the DesignCraft document.
- [ ] PubGen Card 3 offers `TOC Paragraph`, which fills the title with `Daftar Isi`.
- [ ] A `TOC Paragraph` TOC lists chapter titles at level 1, `Heading 1` at level 2, `Heading 2` at level 3, with indents 0 / 12 / 24 pt.
- [ ] Table and chart pages are absent from a `TOC Paragraph` (via `pageTitle: ["BAB"]`).
- [ ] Existing TOC Entry / TOC Table / TOC Chart behaviour is unchanged.
- [ ] `cargo xtask ci` green.

---

## 6. Deliberately out of scope

- Character-style (`\csN`) import and stylesheet character styles.
- Configurable L1/L2/L3 style pickers in `PubgenState` (constants suffice; add when a publication needs different style names).
- Any `rxx_pages` session tag / engine-side page scoping — `pageTitle` already covers it.
- Level detection from `\outlinelevel` (the samples tag `\sN` directly; add only for producers that omit style tags).
