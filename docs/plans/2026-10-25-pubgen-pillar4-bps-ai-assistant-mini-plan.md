# PubGen Pillar 4 — BPS Statistical AI Assistant & Official Terminology Memory Mini-Plan

- **Document:** `docs/plans/2026-10-25-pubgen-pillar4-bps-ai-assistant-mini-plan.md`
- **Date:** 2026-10-25
- **Revised:** 2026-10-27 (verified against `pubgen-0.7.X`; see §11)
- **Status:** Implemented 2026-10-27 on `feature/pillar4-bps-glossary` (Phases 0–5) — M3 of [`2026-10-25-pubgen-future-roadmap-and-extension-plan.md`](2026-10-25-pubgen-future-roadmap-and-extension-plan.md) (Pillar 4); as-built shape in §5
- **Branch:** `feature/pillar4-bps-glossary` (off `pubgen-0.7.X`, created 2026-10-27)
- **Target:** `crates/translate/src/`, `crates/ui-egui/src/panels/pubgen/`, `crates/engine/src/`, `assets/pubgen/`

---

## 1. Goal

Implement the BPS Statistical Glossary Engine and Automated Executive Narrative Summarizer so that AI translation uses official BPS terminology deterministically, and raw table data can generate pre-styled chapter review narratives (`ulasan_bab`) without manual authoring. Two features, six phases (0–5), zero regressions.

## 2. Current State (verified 2026-10-27 on `pubgen-0.7.X`)

| Component | State | Notes |
| :--- | :--- | :--- |
| `assets/pubgen/bps_glossary.json` | **Already committed** (arrived with `1ddc86ef`), 15 terms, `Keterangan → "Note"` | Phase 0's data work is done; `ASSETS.md:211` already lists the file |
| `crates/translate/src/online.rs:183` | Hardcoded system prompt (`:185-191`) | `request_body()` is private and `#[cfg(feature = "groq")]`; the new param must thread through `translate_batch` (`:98`) → `request_once` (`:138`) → `request_body`, not just `request_body` |
| `crates/translate/src/lib.rs:74` | `Backend::translate()` takes `&[String]` | `Backend` is **not used by the UI**: `ai.rs:91` and `ai.rs:317` call `OnlineTranslator::translate` directly. The signature change is smaller than it looks |
| `crates/ui-egui/src/panels/pubgen/ai.rs:525` | `alias_en()` — hardcoded, **case-insensitive**, 10 spellings | Covers `Jumlah`/`Total`, `Provinsi`/`Propinsi`, `ket.`/`kec`, `No` — **none of these are in the JSON** (regression risk, §6 Phase 1) |
| `ai.rs:502` | `translatable_cell()` — skips alias cells | 8 non-test call sites in `ai.rs` (435, 492, 561, 1100, 1103, 1122, 1145) plus tests; 2 `pair_cells` call sites in `pages.rs` (670, 1985) |
| `crates/engine/src/lib.rs:194` | `Prefs` | `#[serde(default, rename_all = "camelCase")]` + generic merge in `prefs.set` (`cmd/prefs.rs:56`) means a new field works once the param doc string (`cmd/prefs.rs:53`) is updated |
| `crates/ui-egui/src/panels/pubgen/ai.rs:27` | `AiSetupDialog` — Groq + Offline panes | No terminology tab |
| `crates/textimport/src/rtf.rs:323,637` | Importer parses `\stylesheet`/`\sN` and returns named `para_styles` | `place_text.rs:57-110` applies those styles to the document, so style-by-name RTF placement works; `ensure_para_style()` (`helpers.rs:29`) covers missing definitions |
| `crates/ui-egui/src/panels/pubgen/helpers.rs:605` | `is_ulasan_type()` lives here, **not** in `pages.rs` | Called from `pages.rs:416, 564, 590, 607, 694` |
| `crates/engine/src/tests_file_guard.rs:64` | `ai.rs` is already allow-listed | The glossary read needs only a reason-string update, no new entry |
| `crates/ui-egui/src/panels/pubgen/state.rs:150` | `card1_records` only | Tables are per-row CSV files resolved from the catalog (`ai.rs:483`); there is **no workbook-matrix store** — Phase 4a needs its own collection pass |
| Branch | `pubgen-0.7.X` | `refactor/pubgen-modularization` does not exist locally; not a dependency |

### Existing alias coverage vs. bundled glossary

| Term | `alias_en()` (current) | `bps_glossary.json` | Gap |
| :--- | :--- | :--- | :--- |
| Kecamatan | Subdistrict | Subdistrict | Covered |
| Kabupaten | Regency | Regency | Covered |
| Desa | — | Village | Added by JSON |
| Kelurahan | — | Urban Village | Added by JSON |
| Luas Wilayah | — | Total Area | Added (whole-cell match only) |
| Kepadatan Penduduk | — | Population Density | Added (whole-cell match only) |
| Jenis Kelamin | — | Sex | Added (whole-cell match only) |
| Laki-laki | — | Male | Added |
| Perempuan | — | Female | Added |
| PDRB | — | GRDP | Added |
| Laju Pertumbuhan | — | Growth Rate | Added (whole-cell match only) |
| Tahun | Year | Year | Covered |
| Sumber | Source | Source | Covered |
| Catatan | Note | Note | Covered |
| Keterangan | Note | Note | **Resolved** — JSON already says `Note` |
| Jumlah / Total | Total | — | **Regression risk** — JSON lacks it; must be merged in |
| Provinsi / Propinsi | Province | — | **Regression risk** — JSON lacks it |
| Ket. / Kec | Note / Subdistrict | — | **Regression risk** — JSON lacks the abbreviations |
| No | No | — | JSON lacks it (`pair_cells` relies on it) |

## 3. Principles

1. **Glossary is deterministic, not advisory.** Mapping applies to **whole cells** in `pair_cells` before/after translation; the system prompt is a secondary net. No substring rewriting of translated prose.
2. **`alias_en()` becomes data-driven without losing ground.** Matching stays case-insensitive, keys are lowercased at load, and the code-only spellings (`jumlah`, `total`, `provinsi`, `propinsi`, `ket.`, `ket`, `kec`, `no`) are merged into the default JSON. The function signature **does change** (it gains `&BpsGlossary`); every call site moves in one commit.
3. **Summarizer is opt-in, never automatic.** A button triggers it; users preview before anything enters the document.
4. **Provider HTTP lives below the UI.** Groq text calls belong in `designcraft-translate` (`ureq` is already optional under `groq`); Cloudflare image calls live in `designcraft-imagegen` (`cloudflare.rs:33`). `ui-egui` gains no HTTP client.
5. **Structured output, not authored RTF.** The model returns JSON blocks; Rust builds the RTF and the styles deterministically. Never trust a model to emit valid RTF (the translation path already needs a salvage path for non-JSON).
6. **One phase = one commit.** `cargo xtask ci` green after every commit.
7. **Stdlib first.** `serde_json`, `std::fs`, existing optional `ureq`. No new crates.

## 4. Architecture Decisions

### 4.1 Where the glossary lives

```
assets/pubgen/bps_glossary.json                     ← bundled default (already committed)
<prefs.pubgen_glossary_path>                        ← optional user override
```

Load from disk at runtime (never `include_str!`: users must edit terms without recompiling). Resolution order, mirroring `resolve_template_path` in `covers.rs:97-107`:

1. `prefs.pubgen_glossary_path` when non-empty;
2. `assets/pubgen/bps_glossary.json` relative to the working directory;
3. the same path under `env!("CARGO_MANIFEST_DIR")/../..` (tests run from the crate dir, the app from the repo root);
4. empty map + one log line (graceful degradation, never an error).

No `ASSETS.md` work remains.

### 4.2 How the glossary reaches the model

1. **Whole-cell mapping (primary):** `alias_en()` consults the merged map; `pair_cells` (`ai.rs:542`) emits `id\u{2028}en`. Works for offline and online alike.
2. **System prompt injection (secondary, online only):** append `"Official terminology you MUST use: …"` in `request_body()`, capped at 200 terms, sorted for a stable prompt.

Rejected: replacing Indonesian terms **inside** translated prose. English output can legitimately contain a proper noun ("Kecamatan Karawang Timur") and a blind pass can double-replace or corrupt it. If verification is wanted later, report leftovers as warnings, never mutate.

Documented limitation: multi-word entries (`Luas Wilayah`, `Jenis Kelamin`, `Laju Pertumbuhan`) match only a whole cell — `"Luas Wilayah (km²)"` falls through to the model. Say so in the terminology tab's hint.

### 4.3 Translate crate API change

```rust
pub fn translate(
    &mut self,
    texts: &[String],
    src: Lang,
    tgt: Lang,
    glossary: Option<&[(String, String)]>,   // NEW
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<Vec<String>, TranslateError>
```

`&[(String, String)]` (not `HashMap`) keeps the L3 public surface minimal; callers pass a sorted vec. Threading:

- `Backend::translate` → `Translator::translate` / `OnlineTranslator::translate` → `translate_batch` → `request_once` → `request_body`;
- `nllb::translate` accepts and ignores it (seq2seq has no prompt);
- `Backend` stays updated for symmetry even though the UI does not use it.

Call sites: `ai.rs:91` (`None`, the setup probe), `ai.rs:317` (`Some`, the job). Tests to update: `online.rs:277` (`request_body_pins_the_contract`) and the round-trip tests at `online.rs:339,349`.

### 4.4 Summarizer architecture (revised)

```
Catalog rows ──► collect table CSVs (resolve_catalog_related_file, parse_csv_content_with_delim)
                        │
                        ▼
             analyze_table()  — pure Rust, unit-testable, no network
                        │
                        ▼
             TableSummary[] ──► prompt (system: BPS analyst + glossary; user: JSON stats)
                        │
                        ▼
             translate::OnlineTranslator::complete()   ← HTTP boundary (ureq, `groq`)
                        │
                        ▼
             {"blocks":[{"role":"lead|heading1|heading2|body","text":"…"}]}
                        │
                        ▼
             build_narrative_rtf()  — Rust writes \stylesheet + \sN
                        │
                        ▼
             textimport::import → place (styles applied by place_text.rs:57-110;
             ensure_para_style only for styles the document lacks)
                        │
                        ▼
             Preview dialog → Apply / Discard (one undo step)
```

Statistics stay testable without network; only the drafting step touches the model. One request per chapter.

## 5. As-Built File Structure (2026-10-27)

```
assets/pubgen/bps_glossary.json      ← 15 roadmap terms + legacy spellings; ASSETS.md row present
docs/pubgen-glossary.example.csv     ← two-column starter sheet for the Terminology pane's Import CSV…

crates/translate/src/
├── lib.rs            ← translate() gains glossary; OnlineTranslator::complete() wrapper
├── online.rs         ← glossary in the prompt (capped 200, None byte-identical);
│                       complete() + shared post_chat() with 429 Retry-After + tests
└── nllb.rs           ← accepts and ignores the glossary param

crates/ui-egui/src/panels/pubgen/
├── ai.rs             ← BpsGlossary (load/merge/terms/save); data-driven alias_en;
│                       translatable_cell/pair_cells take it; Terminology tab; job glossary
├── summarize.rs      ← new: parse_number, analyze_table, collect_table_summaries,
│                       build_narrative_prompt, parse_blocks, draft_blocks,
│                       build_narrative_rtf, apply_narrative, NarrativeJob, preview_ui
├── state.rs          ← glossary cache + glossary_loaded; narrative_job/preview/target
├── mod.rs            ← module registration; one-time glossary load; job poll; preview dialog
└── widgets.rs        ← "Draft Review" beside "Translate this Page" (Groq + RXX page + table rows)

crates/engine/src/
├── lib.rs            ← Prefs.pubgen_glossary_path (camelCase `pubgenGlossaryPath`)
├── cmd/prefs.rs      ← param doc string
└── tests_file_guard.rs ← reason string only

ROADMAP.md            ← progress log; docs/plans/2026-10-25-pubgen-future-roadmap… §5 status
```

`egui_extras` is a workspace dependency but **not** a `ui-egui` dependency — the terminology editor uses `egui::Grid` inside a `ScrollArea`; no crate was added.

## 6. Phases

Branch `feature/pillar4-bps-glossary`. Separate `CARGO_TARGET_DIR`.

---

### Phase 0 — Baseline only (~0.1 d)

**What:** Confirm the data file and record the CI baseline. The original Phase 0 work already landed.

1. Verify `assets/pubgen/bps_glossary.json` contents (15 terms, `Keterangan → "Note"`) and the `ASSETS.md` row — both present.
2. Run `cargo xtask ci`; record the test count for later comparison.

**Deliverable:** Baseline recorded.
**Gate:** `cargo xtask ci` green.

---

### Phase 1 — Glossary struct, Prefs, data-driven alias (~0.75 d)

**What:** Load the glossary, persist the path, and make `alias_en()` data-driven **without regressing** current coverage.

1. **`crates/engine/src/lib.rs`** — add `pub pubgen_glossary_path: String` to `Prefs`, default `""`; document that empty means bundled.
2. **`crates/engine/src/cmd/prefs.rs:53`** — add `pubgenGlossaryPath?: path` to the params doc.
3. **`assets/pubgen/bps_glossary.json`** — merge the code-only spellings so nothing is lost:
   ```json
   "jumlah": "Total", "total": "Total",
   "provinsi": "Province", "propinsi": "Province",
   "ket.": "Note", "ket": "Note",
   "kec": "Subdistrict",
   "no": "No"
   ```
   Keys are lowercased; multi-word entries stay capitalized in the file but are also lowercased at load.
4. **`ai.rs`** — add:
   ```rust
   #[derive(Clone, Debug, Default, serde::Deserialize)]
   pub struct BpsGlossary {
       #[serde(default)]
       pub exact_mappings: HashMap<String, String>,
   }

   impl BpsGlossary {
       /// User path → cwd asset → CARGO_MANIFEST_DIR asset → empty.
       pub fn load(user_path: &str) -> Self;
       pub fn lookup(&self, cell: &str) -> Option<&str>;   // trims + lowercases
   }
   ```
   Lowercase keys at load; `lookup` never allocates for the common miss path (use a lowercased probe only when needed).
5. **Replace `alias_en`** with `pub fn alias_en(cell: &str, glossary: &BpsGlossary) -> Option<&str>` (signature change is intentional — Principle 2).
6. **Thread `&BpsGlossary`** through `translatable_cell` and `pair_cells` at every non-test call site: `ai.rs:435, 492, 561, 1100, 1103, 1122, 1145`, `pages.rs:670, 1985`, and `translate_note_item` (`ai.rs:1052`, used at `:1250`).
7. **`state.rs`** — `#[serde(skip)] pub glossary: Option<ai::BpsGlossary>`; load lazily from `app.session.prefs.pubgen_glossary_path` and invalidate after saves.
8. **`tests_file_guard.rs:64`** — extend the reason string to mention the glossary; no new entry.
9. **Tests:**
   - loader: temp file, malformed JSON → empty, missing file → bundled fallback;
   - **parity test**: for each spelling the old map covered (`jumlah`, `total`, `tahun`, `kabupaten`, `kecamatan`, `kec`, `provinsi`, `propinsi`, `keterangan`, `ket.`, `ket`, `catatan`, `sumber`, `no`), lookup returns the old value;
   - case-insensitivity (`TAHUN`, `Tahun`, `tahun`);
   - multi-word whole-cell behavior (`"Luas Wilayah"` hits, `"Luas Wilayah (km²)"` misses);
   - `alias_en("Kabupaten Karawang") == None` (existing expectation).

**Deliverable:** `alias_en()` is data-driven with identical coverage; glossary path persists.
**Gate:** `cargo xtask ci` green; the parity test is the acceptance criterion.

---

### Phase 2 — System prompt injection in `crates/translate` (~0.5 d)

1. **`lib.rs`** — add the glossary param to `Backend::translate`, `Translator::translate`, `OnlineTranslator::translate`; forward it.
2. **`online.rs`** — thread through `translate` → `translate_batch` → `request_once` → `request_body`; append the term list (sorted, ≤200) to the system prompt only when non-empty. Keep the `None` output byte-identical to today.
3. **`nllb.rs`** — accept and ignore.
4. **`ai.rs`** — `TranslateJob::spawn` takes the glossary vec (sorted) and passes it at `:317`; `verify_and_accept_groq` passes `None` at `:91`.
5. **Tests:** glossary present → prompt contains `"Official terminology"` and the pairs; `None` → byte-identical expected body (`request_body_pins_the_contract`); a 201st term is dropped.

**Deliverable:** Online translations carry BPS terminology constraints.
**Gate:** `cargo xtask ci` green; `cargo xtask layers` green; wasm build green (the glossary type is serde-only and `translate` has no new deps).

---

### Phase 3 — Terminology tab in `AiSetupDialog` (~0.75 d)

1. **`ai.rs`** — extend `AiSetupDialog` with `term_tab: bool`, `term_search: String`, `term_edits: HashMap<String, String>` (keyed by lowercased term).
2. **Third pane** (`"Terminology"`) in `dialog_ui()`: search field; `ScrollArea` + `egui::Grid` with Indonesian term and an inline `TextEdit`; edited rows use the accent token from `theme::Tokens`; "Reset to defaults" and "Save custom glossary…".
3. **Save** — `app.pick_save` (pattern at `dialogs.rs:3692`), write JSON with `std::fs` (already allow-listed), then `prefs.set {pubgenGlossaryPath}` and clear `st.glossary`. Guard the write with `#[cfg(not(target_arch = "wasm32"))]`.
4. **Validation** — reject empty English values; keep the dialog open with an inline message.
5. **Tests:** `BpsGlossary` → JSON → `BpsGlossary` round-trip preserves every edit; saving an empty map is refused.

**Deliverable:** Users can customize terminology without editing JSON.
**Gate:** `cargo xtask ci` green; manual check via `cargo run --release -p designcraft --features pubgen -- --sample`.

---

### Phase 4a — Deterministic summarizer core (~1 d)

**What:** Everything except the model call — data collection, statistics, prompt, RTF building, preview placement. Fully testable offline.

1. **New `crates/ui-egui/src/panels/pubgen/summarize.rs`** (register in `mod.rs`):
   ```rust
   #[derive(Debug, Clone)]
   pub struct ColumnStat { pub name: String, pub sum: f64, pub max: f64, pub min: f64 }

   #[derive(Debug, Clone)]
   pub struct TableSummary {
       pub table_title: String,
       pub column_stats: Vec<ColumnStat>,
       pub top_row: Option<(String, f64)>,
       pub bottom_row: Option<(String, f64)>,
   }

   pub fn parse_number(text: &str) -> Option<f64>;      // "1.234,56", "-", "(1.234)", "45%"
   pub fn analyze_table(matrix: &[Vec<String>], title: &str) -> Option<TableSummary>;
   pub fn build_narrative_prompt(summaries: &[TableSummary], chapter: &str) -> (String, String);
   pub fn build_narrative_rtf(blocks: &[(String, String)]) -> String;  // (role, text)
   ```
   `is_numeric_or_dash` (`csv.rs:380`) only validates; the parser is new and needs its own tests (`1.234,56` → `1234.56`, `1,234.56` → `1234.56`, `(1.234)` → `-1234`, `-` → `None`, `45%` → `0.45`).
2. **Collection pass** — walk `st.card1_records`, resolve each row's CSV with the `resolve_catalog_related_file` logic used at `ai.rs:483`, dedupe by path, parse with `detect_delimiter` + `parse_csv_content_with_delim`. There is no workbook matrix cache to reuse (`state.rs:150`).
3. **RTF writer** — emit `{\stylesheet{\s0 Normal;}{\s1 Heading 1;}{\s2 Heading 2;}{\s3 Lead Paragraph;}}` with `\pard\sN` per block; escape braces/backslashes and non-ASCII exactly as `write_plain_rtf` does (`ai.rs:613`). Reuse/extract that escaper rather than duplicating it.
4. **Preview + apply** — dialog listing the blocks; "Apply" writes a temp RTF (`temp_dir()`, `ai.rs:607`), imports with `textimport::import`, and places it through the existing RXX/template story path; call `ensure_para_style` only for names the document lacks (the importer carries the style definitions, `place_text.rs:57-110` applies them).
5. **Tests:**
   - `analyze_table`: sums/max/min/top row, Indonesian formats, empty and malformed matrices → `None`;
   - prompt includes every table title and stat;
   - `build_narrative_rtf` → `textimport::import` yields `Heading 1`, `Heading 2`, `Lead Paragraph` (pattern: `rtf.rs:800`);
   - braces and `\` in model text survive the round trip.

**Deliverable:** Chapter-review text can be assembled, previewed, and placed from a deterministic input.
**Gate:** `cargo xtask ci` green; all new tests pass without network.

---

### Phase 4b — LLM drafting (~1.5 d)

1. **`crates/translate/src/online.rs`** — `pub fn complete(&self, system: &str, user: &str) -> Result<String, TranslateError>` behind `groq`; non-`groq` builds return `Unavailable`. Share the agent, status mapping and `RateLimited`/`Retry-After` handling with `request_once` (`:138-178`). `temperature: 0`.
2. **`lib.rs`** — `OnlineTranslator::complete` wrapper.
3. **`summarize.rs`** — `draft_blocks(...)` calls `complete` with the Phase 4a prompt, parses `{"blocks":[{"role","text"}]}`, validates roles and non-empty text. Parse failure → one retry with a terser prompt → error; never a panic, never prose salvage.
4. **Job** — a `NarrativeJob` mirroring `TranslateJob` (`ai.rs:295`): background thread, channel, `poll()` on the UI loop, cancel flag, wasm stub.
5. **Button** — "Draft Review" beside "Translate this Page" (`widgets.rs:228`); enabled when the provider is Groq, at least one catalog CSV row resolves, and a document is open.
6. **Tests** — canned local listener (pattern `online.rs:335`) for `complete`; valid/invalid JSON handling; prompt construction; glossary terms present in the system prompt.

**Deliverable:** One-click chapter review from table data.
**Gate:** `cargo xtask ci` green; rate-limit errors surface through the existing status row.

---

### Phase 5 — Closeout (~0.25 d)

1. Update the roadmap's §5 to the implemented shape (Prefs field, button location, JSON-block contract, the 4a/4b split).
2. Add the `ROADMAP.md` progress line (dated when the work lands, not 2026-10-25).
3. Re-run `cargo xtask assets`, `cargo xtask layers`, and the wasm build.
4. Strike the closed items from `docs/gaps.md` if listed.

**Gate:** full CI green.

**Effort:** 0 ≈ 0.1 · 1 ≈ 0.75 · 2 ≈ 0.5 · 3 ≈ 0.75 · 4a ≈ 1 · 4b ≈ 1.5 · 5 ≈ 0.25 → **≈ 4.85 d**.

## 7. Risks & Mitigations

| Risk | Mitigation |
| :--- | :--- |
| Data-driven `alias_en` silently drops `Jumlah`/`Provinsi`/`ket.`/`kec`/`No` (the current map is case-insensitive, the JSON is not) | Merge those spellings into the JSON, lowercase keys at load, and make the parity test the Phase 1 gate |
| Multi-word glossary entries never fire inside longer headers | Document whole-cell semantics; UI hint; optional prefix matching later (out of scope) |
| Glossary grows to hundreds of terms | Cap prompt injection at 200 sorted terms; whole-cell mapping is free |
| Model returns malformed JSON | Strict parse + one shorter retry + error; no RTF parsing, no prose salvage |
| Model-authored RTF hard to guarantee | Removed from the design: Rust builds the RTF from structured blocks |
| Generated style names absent from the document | `place_text` applies imported styles; `ensure_para_style` fills gaps |
| Groq rate limits on long narratives | One request per chapter, `temperature: 0`; `RateLimited` already carries a retry hint |
| `ui-egui` accidentally grows an HTTP client | Provider HTTP stays in `translate`/`imagegen`; no dependency changes in `ui-egui` |
| Packaged build cannot find the bundled asset | Resolution order includes the `CARGO_MANIFEST_DIR` fallback; missing file degrades to an empty map, never an error |
| wasm build breaks on file I/O or threads | Glossary is serde-only; save/apply paths are `cfg(not(target_arch = "wasm32"))`; `NarrativeJob` mirrors the existing wasm stub |
| Text-import crash on bad input | Never `unwrap`; import errors surface in the preview dialog |

## 8. Out of Scope

- Post-translation replacement inside prose (rejected in §4.2 as corrupting; warnings only, if ever).
- Modifying the offline NLLB path to support prompts (seq2seq).
- Multi-language glossaries beyond ID→EN (future: `target_lang`).
- Training/fine-tuning on a BPS corpus.
- Automatic terminology discovery (manual curation for M3).
- PDF/X-4 profiles (Pillar 6), chart vector engine (Pillar 1 / M4), moving `ai.rs` into a subdirectory (Pillar 5).

## 9. Dependency Impact

| Crate | Change | New deps |
| :--- | :--- | :--- |
| `designcraft-translate` | `translate` glossary param; `complete()` behind `groq` | None — `ureq` is already optional under `groq` |
| `designcraft-engine` | `Prefs.pubgen_glossary_path` | None |
| `designcraft-ui-egui` | Glossary loader, terminology tab, summarizer module | None — `serde_json`, `std::fs`, `egui::Grid` |

No new workspace dependencies, no new external crates.

## 10. Definition of Done

- [x] `assets/pubgen/bps_glossary.json` contains the 15 roadmap terms **plus** the legacy spellings; `Keterangan → "Note"`.
- [x] `alias_en()` is backed by `BpsGlossary`; the parity test proves every previously mapped spelling still resolves — and, since 2026-10-27, resolves to the *same* English.
- [x] `Prefs.pubgenGlossaryPath` persists and is honored by the loader (user path → cwd → manifest → empty).
- [x] `crates/translate` `translate()` accepts an optional glossary; the Groq prompt includes it when present; `None` output is byte-identical.
- [ ] `AiSetupDialog` has a searchable, editable Terminology pane with save-to-file and reload.
- [x] `analyze_table` parses Indonesian number formats and has offline unit tests.
- [x] `build_narrative_rtf` produces RTF whose imported paragraph styles match `Heading 1` / `Heading 2` / `Lead Paragraph`.
- [ ] "Draft Review" generates blocks via `OnlineTranslator::complete`, previews them, and applies on confirmation (one undo step).
- [ ] `cargo xtask ci` green (fmt, clippy, tests, assets, layers, wasm).
- [x] `ROADMAP.md` updated; M3 closed.

_State (2026-10-27): a box is ticked where an executed check proves the claim (the checks are listed in §11). The Terminology pane and the Draft Review flow exist, compile and are exercised by the module tests, but their runtime behaviour — pane editing, the enable gating, the Apply path into a real RXX page — still has no automated coverage, so those two stay unticked rather than claimed. The CI box stays unticked because a full `cargo xtask ci` was not run (see §11)._

## 11. Revision Log

**2026-10-27 (implementation).** Phases 0–5 landed on `feature/pillar4-bps-glossary`. Small divergences from the written spec, all recorded in §5:

1. The terminology editor keeps the dialog's `tab` index plus a flat `terms: Vec<(String, String)>` rather than a `term_tab`/`term_edits` pair; the grid edits `terms` directly and saves `BpsGlossary::from_terms`.
2. `draft_blocks` takes the job's cancel flag so a Cancel before/after the first request never reaches the provider twice.
3. Apply uses `file.place` with the selected review page's `en_body_fid`; the imported `\stylesheet` carries the block styles, so `ensure_para_style` was unnecessary. One undo step, as planned.
4. "Draft Review" also requires a detected RXX review page (not just Groq + a document): the blocks must be placed into that page's English body frame.
5. `NarrativeJob` mirrors `TranslateJob` including the wasm stub; `parse_blocks`/`draft_blocks` carry `allow(dead_code)` on wasm only.
6. Post-closeout follow-up (user request): the Terminology pane gained **Import CSV…** — a native-only picker (`pick_open("glossary")`, `OpenFilter` csv/tsv/tab/txt) feeding the existing delimiter-detecting CSV parser; `terms_from_csv` skips a header and incomplete rows, `merge_terms` overrides matching Indonesian terms case-insensitively and appends the rest, and the write still happens only on **Save…**. `docs/pubgen-glossary.example.csv` holds the bundled 23 rows; tests cover header skipping, tab detection, merge counts and the example sheet round-trip.

**2026-10-27** — verified every claim against `pubgen-0.7.X`:

1. **Phase 0 mostly landed.** The glossary asset and `ASSETS.md` row are already committed; `Keterangan` is already `"Note"`. Phase 0 is now baseline-only.
2. **Line references corrected.** `is_ulasan_type` is in `helpers.rs:605` (callers in `pages.rs:416, 564, 590, 607, 694`); `tests_file_guard.rs` already allow-lists `ai.rs`; `refactor/pubgen-modularization` does not exist locally.
3. **Phase 1 regression hazard documented.** The JSON lacks `Jumlah`, `Provinsi`, `ket.`, `kec`, `No`, and its capitalised keys would break the current case-insensitive matching. Added the merge step and the parity test.
4. **Phase 2 scope corrected.** `Backend` is unused by the UI; the param must thread through `translate_batch` and `request_once`, not just `request_body`.
5. **Phase 4 re-scoped.** Split into 4a (deterministic, testable) and 4b (LLM). Moved the HTTP boundary into `designcraft-translate` (mirroring `imagegen` for Cloudflare), replaced model-authored RTF with a JSON block contract built in Rust, called out that there is no workbook matrix store, and raised the estimate from 0.75 d to ~2.5 d.
6. **Misc.** Fixed the `cargo xtest` typo; noted `egui_extras` is not a `ui-egui` dependency; added the wasm and asset-resolution risks.

**2026-10-27 (post-review fixes).** A review of the as-built code (§5) found five defects; all are fixed on this branch, each with a test.

1. **The first Apply of a session could fail.** `apply_narrative` wrote into `ai::temp_dir()` without creating it, and nothing creates that directory before a session translates a page, so Apply reported `cannot write …: the system cannot find the path specified`. The write now goes through `write_narrative_rtf_in`, which creates the directory first; `narrative_rtf_lands_in_a_temp_dir_that_does_not_exist_yet` covers it.
2. **A failed Apply discarded the draft.** Only a successful placement clears `narrative_preview` / `narrative_target` now: a write error keeps the blocks (and the API spend behind them) so the user can retry.
3. **Cancel was unreachable.** The window only rendered once the job had finished, so its Cancel — guarded on `!job.finished` — could never be clicked. The window now also renders while the worker runs (spinner, status line and a working Cancel), and closing it mid-draft cancels the request. A cancel that reaches the worker reports "Chapter review cancelled." rather than a failure.
4. **No way back to the bundled glossary.** `Prefs.pubgenGlossaryPath` outranks the asset while non-empty, but nothing cleared it, so **Reset to bundled** only rewrote the grid. It now also clears the path and reloads the bundled terms.
5. **Save had no validation.** Phase 3 specified rejecting an empty English value inline and a test that an empty map is refused; the code silently dropped incomplete rows instead. `glossary_save_error` now names the offending row — or an empty list — and blocks the write.

Also strengthened the Phase 1 parity gate: it asserted only that the legacy spellings resolve, not to *what*; each is now pinned to the English the hardcoded map returned, which proves the merged spellings kept their values. The web build keeps its stub — **Draft Review** is disabled there instead of arming a job that could never report back.

Verified on this branch, by execution: `cargo fmt --all -- --check`; `cargo test -p designcraft-ui-egui --features pubgen --lib dock::pubgen` (256 tests pass, the four new/strengthened ones included); `cargo test -p designcraft-engine --lib cmd::prefs` (8 pass — `application_and_document_preferences` now round-trips `pubgenGlossaryPath` through `prefs.set`, which is what that DoD box claims); `cargo check --target wasm32-unknown-unknown -p designcraft-ui-egui --features pubgen` (7 panel warnings before these fixes, 5 after: the two that disappear are the `unused variable: app` / `st` hits that existed only because the pane ignored them under `cfg`); `cargo xtask assets` (151 attributed); `cargo xtask layers` (23 crates, no violations).

**2026-10-27 (lint gate).** The panel was compiled by CI but never linted: `cargo clippy --workspace` runs without the feature, so `panels/pubgen` escaped `-D warnings` entirely — and had accumulated 15 clippy errors (`derivable_impls`, `field_reassign_with_default`, five `collapsible_if`, `needless_borrow`, a clone-to-slice in `excel.rs`, `bool_assert_comparison`, `approx_constant`, `err_expect`). All 15 are fixed properly rather than allowed: a derived `Default` for `ChapterSpec`, struct-literal initialization, let-chains mirroring the panel's own `toggle_lang_button` handling, `std::slice::from_ref`, `expect_err`, `assert!(!…)`, and a test decimal that is not π. The `pubgen` CI step is now a lint gate instead of a bare compile:

```
cargo clippy -p designcraft-ui-egui --features pubgen --all-targets -- -D warnings
```

`--all-targets` is the point: eight of the fifteen lived in `#[cfg(test)]` code that a plain `check` never builds. The gate is clean on this branch (exit 0), and `cargo xtask ci`'s step list and `--help` text now name it.

Still open, and recorded rather than assumed: a full `cargo xtask ci` — workspace clippy, the whole nextest suite, doctests and the all-crates wasm check — was not run here, and this machine needs `LIB` pointed at the Windows SDK (`C:/Program Files (x86)/Windows Kits/10/Lib/10.0.26100.0/um/x64` plus the sibling `ucrt/x64`) before anything links at all. Hence the §10 CI box above stays unticked. Also note that the pubgen test suite rewrites three tracked template files under `assets/templates/` on every run; those were reverted so the branch carries only the intended files.
