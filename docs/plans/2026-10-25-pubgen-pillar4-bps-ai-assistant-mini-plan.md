# PubGen Pillar 4 — BPS Statistical AI Assistant & Official Terminology Memory Mini-Plan

- **Document:** `docs/plans/2026-10-25-pubgen-pillar4-bps-ai-assistant-mini-plan.md`
- **Date:** 2026-10-25
- **Status:** Draft — execution plan for M3 of [`2026-10-25-pubgen-future-roadmap-and-extension-plan.md`](2026-10-25-pubgen-future-roadmap-and-extension-plan.md) (Pillar 4)
- **Target:** `crates/translate/src/`, `crates/ui-egui/src/panels/pubgen/ai.rs`, `crates/engine/src/lib.rs`, `assets/pubgen/`

---

## 1. Goal

Implement the BPS Statistical Glossary Engine and Automated Executive Narrative Summarizer so that AI translation uses official BPS terminology deterministically, and raw table data can generate pre-styled chapter review narratives (`ulasan_bab`) without manual authoring. Two features, five phases, zero regressions.

## 2. Current State (measured 2026-10-25)

| Component | State | Notes |
| :--- | :--- | :--- |
| `crates/translate/src/online.rs:185-191` | Hardcoded system prompt: `"You are a translation engine..."` | No glossary injection point; `request_body()` is `#[cfg(feature = "groq")]` private fn |
| `crates/translate/src/lib.rs` | `Backend::translate()` takes `&[String]`, no context param | Provider-blind API; adding glossary means either a new param or pre/post-processing |
| `crates/ui-egui/src/panels/pubgen/ai.rs:525-538` | `alias_en()` — hardcoded 10-term match map | Covers `Jumlah→Total`, `Kecamatan→Subdistrict`, etc. Deterministic but incomplete and not user-editable |
| `crates/ui-egui/src/panels/pubgen/ai.rs:502-521` | `translatable_cell()` — skips cells matching `alias_en()` | Alias terms never sent to model; replaced locally in `pair_cells()` |
| `crates/engine/src/lib.rs:248-259` | `Prefs` has `translation_provider`, `translation_groq_model`, etc. | No glossary path or inline glossary field yet |
| `crates/ui-egui/src/panels/pubgen/ai.rs:27-39` | `AiSetupDialog` — Groq + Offline tabs only | No glossary editor tab |
| `crates/textimport/src/rtf.rs:229-249` | RTF importer normalizes style names (`heading 1` → `Heading 1`) | Summarizer output must emit RTF with these exact style names |
| `crates/ui-egui/src/panels/pubgen/helpers.rs:29-39` | `ensure_para_style()` creates named paragraph styles on demand | Summarizer can call this for `Heading 1`, `Heading 2`, `Lead Paragraph` before placing text |
| `crates/ui-egui/src/panels/pubgen/pages.rs:271,412,549` | `is_ulasan_type()` gates RXX narrative flow | Summarizer output targets the same code path |
| `assets/pubgen/` | Does not exist yet | Needs creation for `bps_glossary.json` |
| Branch | `refactor/pubgen-modularization` | Pillar 5 modularization in progress; Phase 1–6 landed. Glossary work touches `ai.rs` which is already extracted |

### Existing alias coverage vs. roadmap glossary

| Term | `alias_en()` (current) | Roadmap `bps_glossary.json` | Gap |
| :--- | :--- | :--- | :--- |
| Kecamatan | Subdistrict | Subdistrict | Covered |
| Kabupaten | Regency | Regency | Covered |
| Desa | — | Village | **Missing** |
| Kelurahan | — | Urban Village | **Missing** |
| Luas Wilayah | — | Total Area | **Missing** |
| Kepadatan Penduduk | — | Population Density | **Missing** |
| Jenis Kelamin | — | Sex | **Missing** |
| Laki-laki | — | Male | **Missing** |
| Perempuan | — | Female | **Missing** |
| PDRB | — | GRDP | **Missing** |
| Laju Pertumbuhan | — | Growth Rate | **Missing** |
| Tahun | Year | Year | Covered |
| Sumber | Source | Source | Covered |
| Catatan | Note | Note | Covered |
| Keterangan | Note | Description | **Conflict** (alias says Note, roadmap says Description) |

## 3. Principles

1. **Glossary is deterministic, not advisory.** Terms in `exact_mappings` are replaced by string match before/after translation, not left to LLM compliance. The system prompt injection is a secondary safety net, not the primary mechanism.
2. **`alias_en()` becomes data-driven.** The hardcoded match in `ai.rs:525` is replaced by loading `bps_glossary.json`. The function signature stays identical; the backing store changes.
3. **Summarizer is opt-in, never automatic.** A button triggers it. It does not run during normal page generation. Users review output before it enters the document.
4. **Layering preserved.** `crates/translate` (L3) knows nothing about BPS or PubGen. Glossary injection happens at L6 (`ai.rs`) for the UI path, and via a new parameter for the translate crate's system prompt.
5. **One phase = one commit.** `cargo xtask ci` green after every commit.
6. **Stdlib first.** JSON parsing uses `serde_json` (already a dependency). File I/O uses `std::fs`. No new crates.

## 4. Architecture Decisions

### 4.1 Where glossary lives

```
assets/pubgen/bps_glossary.json    ← bundled default (include_str! or load at runtime)
~/.config/designcraft/pubgen/bps_glossary.json  ← user override (prefs-resolved path)
```

Decision: **load from disk at runtime**, not `include_str!`. Reason: users must be able to edit terms without recompiling. The bundled file under `assets/pubgen/` is the fallback when no user override exists. ASSETS.md needs a row for the new JSON file.

### 4.2 How glossary reaches the LLM

Two mechanisms, both active simultaneously:

1. **Deterministic pre-replacement (primary):** Before sending text to the model, scan source strings for exact Indonesian terms and replace them in the output directly. This is what `alias_en()` already does. Expand it to use the full glossary map.
2. **System prompt injection (secondary):** Append `"Official terminology: Kecamatan=Subdistrict, Desa=Village, …"` to the Groq system prompt in `online.rs:request_body()`. This catches compound phrases and contextual usage where exact string replacement would miss.

For the offline NLLB path, only mechanism 1 applies (NLLB has no system prompt). This is correct: NLLB is a pure seq2seq model.

### 4.3 Translate crate API change

Add an optional `glossary` parameter to `Backend::translate()` and `OnlineTranslator::translate()`:

```rust
pub fn translate(
    &mut self,
    texts: &[String],
    src: Lang,
    tgt: Lang,
    glossary: Option<&HashMap<String, String>>,  // NEW
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<Vec<String>, TranslateError>
```

When `glossary` is `Some`, `request_body()` appends the mapping to the system prompt. When `None`, behavior is unchanged. This is backward-compatible at the call site level (every caller adds one argument).

**Alternative considered:** Pre/post-process entirely in `ai.rs` without touching `crates/translate`. Rejected because the system prompt injection requires modifying `request_body()` inside `online.rs`, and keeping the translate crate provider-blind means the glossary should pass through its API rather than being smuggled via thread-local state.

### 4.4 Summarizer architecture

```
┌──────────────────────────────────────────────┐
│ Table CSV matrices (already parsed)          │
│ - Column headers                             │
│ - Numeric data rows                          │
│ - Summary rows (Jumlah, etc.)                │
└──────────────────┬───────────────────────────┘
                   │ Aggregate stats in Rust
                   ▼
┌──────────────────────────────────────────────┐
│ Statistical Analysis (pure Rust, no LLM)     │
│ - Max/min values per column                  │
│ - Row with highest/lowest total              │
│ - Percentage distributions                   │
│ - Year-over-year deltas if multi-year        │
└──────────────────┬───────────────────────────┘
                   │ Structured summary as text
                   ▼
┌──────────────────────────────────────────────┐
│ LLM Narrative Drafting (Groq only)           │
│ System prompt: BPS analytical phrasing       │
│ + glossary terms                             │
│ User prompt: structured stats → prose        │
│ Output: RTF with \sN style references        │
└──────────────────┬───────────────────────────┘
                   │ RTF bytes
                   ▼
┌──────────────────────────────────────────────┐
│ Place into RXX template slots                │
│ ensure_para_style("Heading 1")               │
│ ensure_para_style("Heading 2")               │
│ ensure_para_style("Lead Paragraph")          │
│ template.page.apply with story slot          │
└──────────────────────────────────────────────┘
```

The statistical analysis step is pure Rust (no LLM needed). Only the prose drafting calls the model. This keeps token costs low and makes the analysis testable without network access.

## 5. Target File Structure

```
assets/pubgen/
└── bps_glossary.json              ← new: default BPS terminology

crates/translate/src/
├── lib.rs                         ← modified: Backend::translate() gains glossary param
├── online.rs                      ← modified: request_body() appends glossary to system prompt
└── nllb.rs                        ← modified: stub accepts glossary param (ignored)

crates/ui-egui/src/panels/pubgen/
├── ai.rs                          ← modified: load glossary, replace alias_en(), add summarizer UI + logic
├── mod.rs                         ← modified: wire summarizer button into header band
└── state.rs                       ← modified: add glossary_path, summarizer state fields

crates/engine/src/
├── lib.rs                         ← modified: Prefs gains pubgen_glossary_path field
└── tests_file_guard.rs            ← modified: allow-list ai.rs glossary file read

ASSETS.md                          ← modified: row for bps_glossary.json
ROADMAP.md                         ← modified: progress log line after M3 close
```

## 6. Phases

Branch: `feature/pillar4-bps-glossary` off `pubgen-0.7.X` (or off `refactor/pubgen-modularization` after it lands). Separate `CARGO_TARGET_DIR`.

---

### Phase 0 — Baseline & glossary data file (~0.25 d)

**What:** Create the glossary JSON. Record baseline CI state.

1. Create `assets/pubgen/bps_glossary.json` with the schema from the roadmap (§5.2):
   ```json
   {
     "exact_mappings": {
       "Kecamatan": "Subdistrict",
       "Kabupaten": "Regency",
       "Desa": "Village",
       "Kelurahan": "Urban Village",
       "Luas Wilayah": "Total Area",
       "Kepadatan Penduduk": "Population Density",
       "Jenis Kelamin": "Sex",
       "Laki-laki": "Male",
       "Perempuan": "Female",
       "PDRB": "GRDP",
       "Laju Pertumbuhan": "Growth Rate",
       "Tahun": "Year",
       "Sumber": "Source",
       "Catatan": "Note",
       "Keterangan": "Description"
     }
   }
   ```
2. Resolve the `Keterangan` conflict: current `alias_en()` maps it to `"Note"`, roadmap says `"Description"`. Decision: keep `"Note"` (matches BPS bilingual publications observed in `plan/indesign/11-observed-ui.md`). Update roadmap glossary to say `"Note"`.
3. Add row to `ASSETS.md`:
   ```
   | `assets/pubgen/bps_glossary.json` | DesignCraft contributors / BPS IPDS Karawang | original compilation from BPS bilingual publication standards | MIT OR Apache-2.0 | Official BPS statistical terminology mapping for AI translation |
   ```
4. Run `cargo xtask ci` — confirm green. Record test count.

**Deliverable:** `bps_glossary.json` committed, ASSETS.md updated, CI green.
**Gate:** `cargo xtask assets` passes. `cargo xtest ci` green.

---

### Phase 1 — Glossary struct & Prefs wiring (~0.5 d)

**What:** Define the Rust type, load it, persist the path.

1. **`crates/engine/src/lib.rs`** — add to `Prefs`:
   ```rust
   /// Path to custom BPS glossary JSON. Empty = use bundled default.
   pub pubgen_glossary_path: String,
   ```
   Default in `impl Default for Prefs`: empty string.

2. **`crates/ui-egui/src/panels/pubgen/ai.rs`** — new types and loader:
   ```rust
   #[derive(Clone, Debug, serde::Deserialize)]
   pub struct BpsGlossary {
       pub exact_mappings: HashMap<String, String>,
   }

   impl BpsGlossary {
       /// Load from user path, falling back to bundled asset.
       pub fn load(user_path: &str) -> Self { ... }
   }
   ```
   - Bundled fallback: read `assets/pubgen/bps_glossary.json` relative to working directory (same pattern as `cover_art.rs` reads templates). If missing, return empty glossary (not an error — graceful degradation).
   - User path: if non-empty, read that file instead. Parse errors return empty glossary + log warning.

3. **Replace `alias_en()`** — change from hardcoded match to glossary lookup:
   ```rust
   pub fn alias_en(cell: &str, glossary: &BpsGlossary) -> Option<&str> {
       glossary.exact_mappings.get(cell.trim()).map(String::as_str)
   }
   ```
   All callers (`translatable_cell`, `pair_cells`, `translate_note_item`) gain a `&BpsGlossary` parameter. Thread it through from `PubgenState` or pass as arg.

4. **`crates/ui-egui/src/panels/pubgen/state.rs`** — add cached glossary:
   ```rust
   #[serde(skip)]
   pub glossary: Option<ai::BpsGlossary>,
   ```
   Loaded lazily on first use or when prefs change.

5. **`crates/engine/src/tests_file_guard.rs`** — add `ai.rs` glossary file read to ALLOWED list (it already has an entry for ai.rs reading RTF files; update the reason string to include glossary).

6. Tests: unit test `BpsGlossary::load` with a temp JSON file. Test that `alias_en` with glossary returns correct mappings. Test fallback to bundled file.

**Deliverable:** `alias_en()` is fully data-driven. No functional change visible to users yet (same terms, different source).
**Gate:** `cargo xtask ci` green. All existing `ai.rs` tests pass. New glossary loader tests pass.

---

### Phase 2 — System prompt injection in `crates/translate` (~0.5 d)

**What:** Pass glossary to the translate crate so Groq's system prompt includes official terminology.

1. **`crates/translate/src/lib.rs`** — modify `Backend::translate()` and `OnlineTranslator::translate()`:
   ```rust
   pub fn translate(
       &mut self,
       texts: &[String],
       src: Lang,
       tgt: Lang,
       glossary: Option<&[(String, String)]>,
       cancel: &AtomicBool,
       progress: &mut dyn FnMut(usize, usize),
   ) -> Result<Vec<String>, TranslateError>
   ```
   Use `&[(String, String)]` instead of `HashMap` to avoid adding `std::collections` import to the public API surface and to keep the L3 crate minimal. Callers convert their map to a sorted vec.

2. **`crates/translate/src/online.rs`** — modify `request_body()`:
   ```rust
   fn request_body(model: &str, batch: &[String], src: Lang, tgt: Lang, glossary: Option<&[(String, String)]>) -> String {
       let mut system = format!(
           "You are a translation engine. Translate each array item from {} to {}. Return a JSON \
            array of the same length and order. Keep numbers, proper nouns and formatting unchanged. \
            Output only the JSON array.",
           src.label(), tgt.label()
       );
       if let Some(terms) = glossary {
           if !terms.is_empty() {
               let pairs: Vec<String> = terms.iter().map(|(id, en)| format!("{id}={en}")).collect();
               system.push_str(&format!(" Official terminology you MUST use: {}.", pairs.join(", ")));
           }
       }
       // ... rest unchanged
   }
   ```

3. **`crates/translate/src/nllb.rs`** — accept the new param, ignore it (NLLB has no system prompt):
   ```rust
   pub fn translate(
       _texts: &[String], _src: Lang, _tgt: Lang,
       _glossary: Option<&[(String, String)]>,
       _cancel: &AtomicBool, _progress: &mut dyn FnMut(usize, usize),
   ) -> Result<Vec<String>, TranslateError> { Err(unavailable()) }
   ```

4. **Update all callers** in `ai.rs`:
   - `TranslateJob::spawn()` — accept glossary vec, pass to `t.translate()`
   - `verify_and_accept_groq()` — pass `None` (test translation doesn't need glossary)

5. **Tests:**
   - `online.rs` test: verify `request_body` with glossary contains `"Official terminology"` and the term pairs.
   - `online.rs` test: verify `request_body` with `None` glossary matches existing expected output (no regression).
   - Update `request_body_pins_the_contract` test to pass `None` for glossary.

**Deliverable:** Groq translations now receive BPS terminology constraints in the system prompt.
**Gate:** `cargo xtask ci` green. `cargo xtask layers` green (translate is L3, no new deps). Wasm check green.

---

### Phase 3 — Glossary editor UI in AiSetupDialog (~0.5 d)

**What:** Let users view, search, and override glossary terms from the AI setup dialog.

1. **`crates/ui-egui/src/panels/pubgen/ai.rs`** — extend `AiSetupDialog`:
   ```rust
   pub struct AiSetupDialog {
       // ... existing fields
       pub glossary_tab: bool,
       pub glossary_search: String,
       pub glossary_edits: HashMap<String, String>,  // pending overrides
   }
   ```

2. **New tab in `dialog_ui()`:** Add a third radio button `"Terminology"` alongside `"Online — Groq"` and `"Offline"`. When selected, render:
   - Search/filter text input
   - Scrollable table: two columns (Indonesian term, English translation)
   - Each English cell is editable (inline `TextEdit`)
   - Modified rows highlighted with accent color
   - "Reset to defaults" button (clears `glossary_edits`)
   - "Save custom glossary" button → writes to `prefs.pubgen_glossary_path` (prompts for save location via `app.services.pick_save`)

3. **Persistence:** On "Use AI" or "Save", write the edited glossary as JSON to the user-specified path. Update `Prefs.pubgen_glossary_path` via `prefs.set`.

4. **Reload trigger:** After saving, invalidate `st.glossary` so next translation uses updated terms.

5. **Layout:** Keep dialog width at 460px. Glossary table uses `egui_extras::TableBuilder` (already available as workspace dep) or a simple scroll area with grid rows (prefer simple grid — fewer deps, shorter diff).

6. Tests: UI rendering tests are not practical here (egui snapshot tests require kittest harness). Verify serialization round-trip: save glossary edits to JSON, reload, compare.

**Deliverable:** Users can customize BPS terminology without editing JSON files manually.
**Gate:** `cargo xtask ci` green. Manual visual verification via `cargo run --release -p designcraft --features pubgen -- --sample`.

---

### Phase 4 — Automated Executive Narrative Summarizer (~0.75 d)

**What:** Generate chapter review text (`ulasan_bab`) from table data using statistical analysis + LLM drafting.

#### 4.1 Statistical analysis engine (pure Rust, no LLM)

New functions in `ai.rs` (or a new `summarize.rs` if >200 lines):

```rust
#[derive(Debug, Clone)]
pub struct TableSummary {
    pub table_title: String,
    pub column_stats: Vec<ColumnStat>,
    pub max_row: Option<(String, f64)>,  // (row label, value)
    pub min_row: Option<(String, f64)>,
    pub total: f64,
}

#[derive(Debug, Clone)]
pub struct ColumnStat {
    pub name: String,
    pub sum: f64,
    pub max: f64,
    pub min: f64,
}

/// Analyze a CSV matrix into structured statistics.
/// Skips non-numeric cells, handles Indonesian decimal formats (dots as thousands sep).
pub fn analyze_table(matrix: &[Vec<String>], title: &str) -> Option<TableSummary> { ... }
```

- Parse numeric cells using the same logic as `is_numeric_or_dash()` in `csv.rs` (reuse or extract to shared helper).
- Identify the label column (first column, typically subdistrict/village names).
- Compute sum, max, min per numeric column.
- Find the row with the highest total across all numeric columns.
- Return `None` if no numeric data found.

#### 4.2 LLM narrative drafting (Groq only)

```rust
/// Build the prompt for narrative generation.
fn build_narrative_prompt(summaries: &[TableSummary], chapter_title: &str) -> String { ... }

/// Call Groq to draft the ulasan_bab narrative.
/// Returns RTF-formatted text with paragraph style markers.
pub fn draft_narrative(
    summaries: &[TableSummary],
    chapter_title: &str,
    provider: &str, key: &str, model: &str,
    glossary: &BpsGlossary,
    cancel: &AtomicBool,
) -> Result<String, String> { ... }
```

System prompt for summarizer (distinct from translation prompt):
```
You are a BPS statistical analyst writing chapter reviews for official publications.
Write in formal Indonesian following BPS analytical conventions.
Structure your response as RTF with these paragraph styles:
- {\*\stylesheet{\s0 Normal;}{\s1 Heading 1;}{\s2 Heading 2;}{\s3 Lead Paragraph;}}
- Use \s1 for the chapter opening statement
- Use \s2 for section transitions
- Use \s3 for the lead/key finding paragraph
- Use \s0 for body paragraphs
Include specific numbers, percentages, and comparisons from the data provided.
Official terminology: [glossary terms injected here].
Output ONLY valid RTF.
```

User prompt: structured JSON of `TableSummary` data.

#### 4.3 RTF output & placement

The LLM returns RTF text. Write it to a temp file (reuse `temp_dir()` + `write_plain_rtf()` pattern from `ai.rs:607-627`), then feed it through the existing RXX generation pipeline:

1. User clicks **"Generate Chapter Review"** button (new button in header band, visible when Excel is loaded and tables are present).
2. Background job collects all table matrices from the loaded workbook sheets.
3. Runs `analyze_table()` on each.
4. Calls `draft_narrative()` with the summaries.
5. On completion, writes the RTF to a temp file.
6. Injects the temp file path into the catalog records as the `file` column for the Ulasan row.
7. Triggers normal page generation (`execute_generate_pages`), which places the RTF into the RXX template via `template.page.apply`.

Alternatively (simpler, recommended for v1): skip the catalog injection. Instead:
1. Generate the RTF.
2. Create a new page using `template.page.apply` with the divider/RXX template.
3. Place the RTF into the narrative story slot directly.
4. Apply paragraph styles via `ensure_para_style()` before placement.

#### 4.4 UI integration

- Button: `"Draft Review"` in the header band (next to `"Translate this Page"`), enabled when:
  - Backend is Groq (offline NLLB cannot do generative tasks reliably)
  - At least one table sheet is loaded
  - Document is open
- Progress: reuse `TranslateJob` pattern (background thread + channel + poll in `show()`).
- Result: opens a preview dialog showing the generated RTF text, with "Apply" and "Discard" buttons.

#### 4.5 Tests

- `analyze_table()`: unit tests with synthetic CSV matrices. Verify max/min/sum calculations. Test with Indonesian number formats (`1.234,56`). Test with empty/malformed data.
- `build_narrative_prompt()`: verify it includes all table titles and stats.
- RTF output validation: verify the returned string starts with `{\rtf1` and contains `\s1`, `\s2` markers.
- Integration test: mock the Groq response (canned server pattern from `online.rs:291-327`), verify end-to-end flow produces valid RTF.

**Deliverable:** One-click chapter review generation from table data.
**Gate:** `cargo xtask ci` green. Unit tests for statistical analysis pass without network.

---

### Phase 5 — Closeout & documentation (~0.25 d)

1. Update `docs/plans/2026-10-25-pubgen-future-roadmap-and-extension-plan.md` §5 to reflect implemented structure (glossary path in Prefs, summarizer button location, any deviations from original spec).
2. Add progress-log line to `ROADMAP.md`:
   ```
   - **2026-10-25** · PubGen Pillar 4 (M3): BPS statistical glossary engine injects official terminology
     into AI translation system prompts; user-editable terminology table in AI setup dialog; automated
     chapter review drafter generates styled narratives from table data via Groq.
   ```
3. Verify `cargo xtask assets` passes (the new `bps_glossary.json` is listed in ASSETS.md).
4. Verify `cargo xtask layers` passes (no layer violations introduced).
5. Verify wasm build passes (glossary types are serde-only, no platform deps).

**Deliverable:** Docs consistent, M3 closed.
**Gate:** Full CI green.

## 7. Risks & Mitigations

| Risk | Mitigation |
| :--- | :--- |
| Glossary grows large (500+ terms), system prompt exceeds token budget | Cap prompt injection at 200 terms (most frequent first); pre-replacement handles the rest deterministically without prompt cost |
| LLM ignores system prompt terminology instructions | Deterministic post-pass: after translation, scan output for Indonesian terms still present and replace with glossary English equivalents. Belt-and-suspenders approach |
| `alias_en()` signature change breaks callers across `mod.rs`/`pages.rs`/`tables.rs` | Phase 1 updates all callers in one commit. The function is `pub(crate)` within pubgen module only |
| Summarizer produces malformed RTF that crashes the text importer | Wrap `textimport::import()` call in error handling; on parse failure, show raw text in a dialog instead of crashing. Never unwrap |
| Groq free tier rate limits hit during summarization (large token counts) | Summarizer sends one request per chapter (not per table). Token count stays well under free-tier limits. Show rate-limit error gracefully |
| Pillar 5 modularization moves `ai.rs` concurrently | Branch off after Pillar 5 lands, or coordinate: glossary changes are additive to `ai.rs`, no structural refactors |
| `Keterangan` → `"Description"` vs `"Note"` conflict | Resolved in Phase 0: use `"Note"` to match existing BPS bilingual convention. Document decision in this plan |

## 8. Out of Scope

- Modifying `crates/translate`'s offline NLLB path to support system prompts (NLLB is seq2seq, no prompt concept).
- Multi-language glossaries beyond ID→EN (future: add `target_lang` field to `BpsGlossary`).
- Training or fine-tuning models on BPS corpus.
- Automatic detection of new terminology from translated documents (manual curation only for M3).
- PDF/X-4 export profiles (Pillar 6).
- Chart vector engine (Pillar 1 / M4).
- Moving `ai.rs` into a subdirectory (Pillar 5 concern).

## 9. Dependency Impact

| Crate | Change | New deps |
| :--- | :--- | :--- |
| `designcraft-translate` | API addition (`glossary` param) | None |
| `designcraft-engine` | `Prefs` field addition | None |
| `designcraft-ui-egui` | Glossary loader, summarizer UI | None (uses existing `serde_json`, `std::fs`) |

No new workspace dependencies. No new external crates. Stdlib and existing serde_json handle everything.

## 10. Definition of Done

- [ ] `assets/pubgen/bps_glossary.json` exists with ≥15 official BPS term mappings.
- [ ] `alias_en()` is backed by `BpsGlossary` data, not hardcoded matches.
- [ ] `crates/translate` `Backend::translate()` accepts optional glossary; Groq system prompt includes terminology when provided.
- [ ] `AiSetupDialog` has a Terminology tab with searchable, editable term list and save-to-file.
- [ ] `Prefs.pubgen_glossary_path` persists custom glossary location.
- [ ] "Draft Review" button generates chapter narrative from loaded table data via Groq.
- [ ] Generated narrative uses correct paragraph styles (`Heading 1`, `Heading 2`, `Lead Paragraph`).
- [ ] Statistical analysis (`analyze_table`) has unit tests covering Indonesian number formats.
- [ ] `cargo xtask ci` green (fmt, clippy, tests, assets, layers, wasm).
- [ ] ROADMAP.md and ASSETS.md updated. M3 closed.

</content>