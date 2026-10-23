# AI Translation for Pubgen — offline NLLB-200 distilled 600M or online Groq — Implementation Plan

> **Status:** PROPOSED
> **Date:** 2026-10-22
> **Branch:** `feature/pubgen-bilingual-id-en`
> **Scope:** new `crates/translate`, `crates/ui-egui/src/panels/pubgen.rs`, engine `Prefs`, feature wiring, docs, online provider alternative (Groq free tier)
> **Reference:** `docs/plans/2026-10-22-bilingual-id-en-pubgen.md` (bilingual phases 1–7)

---

## Context

Pubgen generates Indonesian publication pages from an Excel catalog. Bilingual output today
(`bilingual_output = true`, `pubgen.rs:280`) only works when the source already carries English:
a `title_en` / `file_en` catalog column, or an `" / "` pair inside a title cell. When those are
absent the English half is emitted as an **empty string** (`pubgen.rs:4831-4833`, `:5319-5320`).

The user's sources are pure Indonesian. They want an explicit opt-in — **"Translate with AI"** —
shown only as meaningful alongside the Bilingual toggle, using an offline **NLLB-200 distilled
600M** model loaded from a user-supplied folder. Everything Indonesian on a generated page is in
scope: cover headline, page titles, chapter/divider titles, table and chart titles, TOC entries,
and narrative body paragraphs.

Confirmed decisions (user, 2026-10-22):

| Question | Answer |
|---|---|
| Backend | `ort` (ONNX Runtime), behind an optional cargo feature |
| AI scope | All Indonesian text — paragraphs, page titles, table, TOC, chrome |
| UX | Background worker + progress + cancel |
| Model files | User-supplied folder path, **plus an in-app model picker that downloads** |
| Target hardware | Must work on an **8 GB RAM PC with no GPU/VRAM** |
| Alternative backend | **Online provider — Groq free tier** — offered in the same setup dialog; user logs in with a free account and translates without any local model |

Licence note that must reach the UI: **NLLB-200 is CC-BY-NC-4.0 — non-commercial.** (Applies to the
offline path only; the online path has no NLLB involvement.)

**Why the alternative exists:** the offline model must fit the 8 GB target (§1.1) and a 600M MT
model is mediocre on narrative prose. Groq's free tier removes the RAM budget entirely and an
instruction-tuned LLM translates Indonesian→English noticeably better. Cost, stated honestly in
the dialog: catalog text leaves the machine (privacy), and free-tier rate limits cap throughput.

## Current state that constrains the design

| Fact | Anchor |
|---|---|
| Pubgen is a UI panel, not an engine command; generation is synchronous on the UI thread | `crates/ui-egui/src/panels/pubgen.rs:4727` |
| State is one serde struct in egui temp storage | `pubgen.rs:171`, `:561` |
| Bilingual toggle lives in the header band | `pubgen.rs:530-539` |
| EN text injected as template text slots | `pubgen.rs:4824-4851`, `:4908-4927` |
| EN body comes from a second RTF via `file.place` | `pubgen.rs:4973-5019`, `:5189-5296` |
| Template slot binding sets run language | `crates/engine/src/cmd/template/mod.rs:299-303` |
| `Prefs` is the only persisted settings store | `crates/engine/src/lib.rs:185-219` |
| Layering: L0–L5 + ui-egui must pass `wasm32` check | `xtask/src/main.rs:125-155`, `xtask/src/layers.rs:35-59` |
| No ML dependency anywhere in the workspace | root `Cargo.toml:13-95` |
| Background-thread precedent | `crates/ui-egui/src/render_worker.rs:29-64` |
| File-picker pattern in Pubgen | `pubgen.rs:1511-1512` (`app.services.pick_open`) |

## Architecture

### 1. New crate `crates/translate` — L3 (register in `xtask/src/layers.rs` TABLE)

Pure text-in / text-out. No `doc`, no `egui`. `ort` and `tokenizers` are **optional**, behind
feature `nllb` (off by default), so the default build and the wasm check compile a stub.

```toml
[dependencies]
ort = { version = "2", optional = true, default-features = false, features = ["std", "download-binaries"] }
tokenizers = { version = "0.21", optional = true, default-features = false }
```

Public API (stable in both feature states):

```rust
pub enum Lang { Indonesian, English }
impl Lang { pub const NLLB_CODE: &'static str; }   // "ind_Latn" / "eng_Latn"

pub struct Translator { /* ort sessions + tokenizer */ }

impl Translator {
    pub fn load(dir: &Path, prof: &Profile) -> Result<Self, TranslateError>; // Unavailable when feature off
    pub fn translate(
        &mut self,
        texts: &[String],
        src: Lang,
        tgt: Lang,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(usize, usize),      // (done, total)
    ) -> Result<Vec<String>, TranslateError>;
}

pub fn is_available() -> bool;                     // cfg!(feature = "nllb")
pub fn model_files_present(dir: &Path) -> bool;

// --- hardware probe and model catalogue (compiled in both feature states) ---
pub struct MemoryInfo { pub total: u64, pub available: u64 }          // bytes
pub fn memory_info() -> Option<MemoryInfo>;                            // None on wasm / probe failure
pub fn cpu_cores() -> usize;                                           // std::thread::available_parallelism

pub struct ModelOption {
    pub id: &'static str,                    // "nllb600-int8"
    pub label: &'static str,
    pub approx_bytes: u64,                   // on-disk download size
    pub peak_ram_bytes: u64,                 // weights + arena + KV headroom
    pub url: &'static str,                   // pinned Hugging Face resolve URL
    pub sha256: &'static str,
    pub files: &'static [(&'static str, &'static str)],  // (file name, path within repo)
}
pub const MODEL_OPTIONS: &[ModelOption];
pub fn fit(opt: &ModelOption, mem: &MemoryInfo) -> Fit;   // Fits / Tight / TooSmall
pub struct SpeedEstimate { pub tokens_per_sec: f32, pub per_paragraph_secs: f32, pub per_page_secs: f32 }
pub fn estimate_speed(opt: &ModelOption, cores: usize) -> SpeedEstimate;
```

Inference (`#[cfg(feature = "nllb")]`), greedy decode, no beams (`src/nllb.rs`):

1. Tokenize with `tokenizer.json`; `src_lang` = source code.
2. Encoder graph: `input_ids`, `attention_mask` → `last_hidden_state`.
3. **Prefer a merged decoder** (`decoder_model_merged.onnx`): one session with a
   `use_cache_branch` bool input, ~700 MB resident instead of ~1.4 GB for the two-graph pair.
   If the merged graph is absent, fall back to `decoder_model.onnx` →
   `decoder_with_past_model.onnx`. First decoder step passes `input_ids = [eos(2), <tgt_lang_id>]`
   and the encoder outputs; later steps pass the previous token + `past_key_values.N.*`.
4. **Bind each graph's inputs by name from `session.inputs()`**, not by position — exports differ
   in which encoder tensors they request. Missing name → `TranslateError::ModelMismatch`.
5. Stop on EOS; cap at 400 new tokens; decode with the tokenizer, skip specials.

Long input is split on paragraph / sentence boundaries (≤ ~200 tokens per segment) before
encoding, then rejoined with newlines. One segment in flight at a time; output tensors are dropped
before the next segment starts. `cancel` is checked between segments and between decode steps, so
Cancel is responsive.

`Translator` is cached in a `OnceLock<Mutex<Translator>>` keyed by model dir, so a second run does
not reload the weights.

### 1.1 Low-memory profile (the 8 GB / no-VRAM target)

Sessions are created with options chosen for a small machine, not ORT defaults
(`SessionOptions` in `src/nllb.rs`):

| Option | Value | Why |
|---|---|---|
| `enable_cpu_mem_arena` | `false` | the arena never returns memory on Windows; RSS creeps per paragraph |
| `disable_mem_pattern` | `true` | allocation pattern memory is another fixed overhead |
| `intra_op_num_threads` | `min(4, cores)` | ORT otherwise spawns one thread per core, each with its own stack + arena |
| `inter_op_num_threads` | `1` | seq2seq runs one op at a time anyway |
| execution provider | CPU only | no GPU path is built |

Measured budget (must hold on an 8 GB machine):

| Item | RSS |
|---|---|
| merged decoder 700 MB + encoder 400 MB | 1.1 GB |
| ORT activations + tokenizer | 0.3–0.5 GB |
| KV cache, 400 tokens, 12 layers × 2 × 1024 × f32 | ~38 MB |
| DesignCraft with a publication open | 1–1.5 GB |
| **Peak** | **~3.5–4 GB** |

Windows on 8 GB leaves ~5.5–6.5 GB usable, so this fits with headroom. **The fp32 export
(~7 GB on disk, >4 GB resident) is not offered at all** — it is not in `MODEL_OPTIONS`.

`MODEL_OPTIONS` therefore ships exactly one entry:

```rust
MODEL_OPTIONS = &[ModelOption {
    id: "nllb600-int8",
    label: "NLLB-200 distilled 600M · int8 (recommended)",
    approx_bytes: 1_800_000_000,
    peak_ram_bytes: 4_200_000_000,
    url: "https://huggingface.co/TigreGotico/nllb-200-distilled-600M-onnx/resolve/main/int8/…",
    sha256: "<pinned at implementation time>",
    files: &[("encoder_model.onnx", "int8/encoder_model.onnx"),
             ("decoder_model.onnx", "int8/decoder_model.onnx"),
             ("decoder_model.onnx_data", "int8/decoder_model.onnx_data"),
             ("decoder_with_past_model.onnx", "int8/decoder_with_past_model.onnx"),
             ("decoder_with_past_model.onnx_data", "int8/decoder_with_past_model.onnx_data"),
             ("tokenizer.json", "tokenizer.json")],
}];
```

The struct is a slice so a smaller or merged-decoder variant can be added without touching the UI.

Errors are `Result`; no `unwrap`/`expect` (workspace `never-crash` rule). The crate carries the
standard `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, ...)]` header.

### 1.2 Hardware probe

`memory_info()` is platform-specific and **must not add a C dependency to the default build**:

- **Windows:** `GlobalMemoryStatusEx` through `winsafe`'s `user` feature is not enough (that is
  User32); use the safe `windows-sys` binding **only under `#[cfg(windows)]`** with
  `Win32_System_SystemInformation`. It is already a transitive dependency of winit.
- **macOS/Linux:** `sysinfo` is overkill. Linux reads `/proc/meminfo` (`MemTotal`, `MemAvailable`);
  macOS shells out once to `sysctl -n hw.memsize` on a background thread with a timeout — both are
  plain `std` reads and a `Command`, no unsafe.
- **wasm:** returns `None`; the UI then says "hardware check unavailable in the browser".

All three return `Option<MemoryInfo>`: a probe failure is a normal state, not an error. No
`unsafe` anywhere in our code (`unsafe_code = "forbid"`); `windows-sys` is the safe wrapper path.

`estimate_speed` uses a documented, deliberately crude model — cores × a per-core int8
throughput constant, clamped — and the UI labels the result as an **estimate**, because it is one:

```rust
tokens_per_sec ≈ (0.9 + 0.55 * cores.min(8) as f32) * 4.2   // ~5 tok/s at 2 cores, ~20 at 8
per_paragraph_secs ≈ 60.0 / tokens_per_sec * 1.6            // ~90 tokens average paragraph
per_page_secs ≈ per_paragraph_secs * 1.8                    // title + chrome + ~0.8 body
```

### 1.3 Alternative backend — online provider (Groq free tier)

Same crate, same `translate` shape, no ML deps. Feature `groq` adds only `serde_json` (already a
workspace dependency) and shares `ureq` with the `download` feature
(`groq = ["dep:ureq", "dep:serde_json"]`). Compiled as a stub in the default build exactly like
`nllb`; the wasm target keeps the feature off so `cargo xtask wasm` stays green.

```rust
pub enum Provider { Offline, Groq }

pub struct OnlineTranslator { agent: ureq::Agent, key: String, model: String }

impl OnlineTranslator {
    pub fn new(key: &str, model: &str) -> Self;
    pub fn list_models(agent: &ureq::Agent, key: &str) -> Result<Vec<String>, TranslateError>;
    pub fn translate(&mut self, texts: &[String], src: Lang, tgt: Lang, cancel: &AtomicBool,
                     progress: &mut dyn FnMut(usize, usize)) -> Result<Vec<String>, TranslateError>;
}

pub enum Backend { Offline(Translator), Online(OnlineTranslator) }  // one `translate`; panel + job code provider-blind
pub fn online_available() -> bool;                                  // cfg!(feature = "groq")
```

Protocol — Groq is OpenAI-compatible: `POST https://api.groq.com/openai/v1/chat/completions`,
`Authorization: Bearer <key>`. Batch 10–20 collected units per request, `temperature: 0`, system
prompt pins the contract: *translate each array item from Indonesian to English; return a JSON
array of the same length and order; keep numbers, proper nouns and formatting unchanged.* Parse
`choices[0].message.content` (strip a ```` ```json ```` fence if present) with `serde_json`. A
length mismatch is an error naming the batch — never silent reordering. Non-JSON output retries
that batch once item-by-item; still bad → `TranslateError::Provider`.

Free-tier realities, all handled in code:

| Concern | Handling |
|---|---|
| per-minute / per-day request caps | small batches (≤ 20 units), one progress tick per request |
| HTTP 429 | sleep `Retry-After` (clamped 1–30 s), retry twice, then error; `cancel` checked before each sleep |
| network hang | 30 s agent timeout per request, so Cancel and failure stay responsive |
| model ids churn | never hardcoded: the dialog fills the model picker from `GET /openai/v1/models` (first `llama-3.3` entry preselected); the chosen id persists in `Prefs` |
| key secrecy | key lives only in the `Authorization` header; never logged, never in error strings, never written into the document |

Secrets note: the key is stored in `Prefs`, which is plaintext on disk
(`translation_groq_api_key`). The dialog says so in one line ("Key is stored locally in
DesignCraft settings"). `ponytail:` OS keychain (DPAPI / Keychain) is the upgrade path if keys
beyond a hobby account ever matter.

### 2. Preferences — five fields

`crates/engine/src/lib.rs` `Prefs`: add `pub translation_model_dir: String` (default `""`),
`pub translation_model_id: String` (default `"nllb600-int8"`), `pub translation_provider: String`
(default `"offline"`; `"groq"` selects the online backend), `pub translation_groq_api_key: String`
(default `""`), `pub translation_groq_model: String` (default `""`, resolved via `list_models` on
first use), and extend the `prefs.set` params doc in `crates/engine/src/cmd/prefs.rs:52-55` with
`translationModelDir?: path, translationModelId?: id, translationProvider?: "offline"|"groq",
translationGroqApiKey?: string, translationGroqModel?: id`. `merge` already accepts new keys;
`tests_params.rs` polices the doc.

### 3. Panel state and UI

`PubgenState` (`pubgen.rs:278` region):

```rust
#[serde(default)] pub translate_with_ai: bool,                     // default false
#[serde(skip)]    pub ai_setup_done: bool,                         // dialog accepted this session
#[serde(skip)]    pub ai_dialog: Option<AiSetupDialog>,            // Some(..) = modal open
#[serde(skip)]    pub translate_job: Option<TranslateJob>,
#[serde(skip)]    pub translations: HashMap<String, String>,       // ID text -> EN text
#[serde(skip)]    pub translated_bodies: HashMap<String, PathBuf>, // source rtf -> temp rtf
```

`TranslateJob` (mirrors `render_worker.rs:29-64`):

```rust
struct TranslateJob {
    rx: std::sync::mpsc::Receiver<JobMsg>,
    cancel: Arc<AtomicBool>,
    done: usize, total: usize,
}
enum JobMsg { Progress(usize, usize), Done(Vec<(String, String)>), Failed(String) }
```

Header band (`pubgen.rs:513-547`), directly under the Single/Dual Lang button:

```rust
let can_ai = st.bilingual_output && (translate::is_available() || translate::online_available());
let mut on = st.translate_with_ai;
let resp = ui.add_enabled(can_ai, egui::Checkbox::new(&mut on, "Translate with AI"));
if resp.changed() && on && !st.ai_setup_done {
    st.ai_dialog = Some(AiSetupDialog::open(app));   // probe hardware, do NOT turn AI on yet
}
```

`resp.on_hover_text(...)` states that it fills the English half when no `title_en`/`file_en`
exists, plus the CC-BY-NC-4.0 note.

**Ticking the box does not enable AI by itself — it opens the setup dialog.** `translate_with_ai`
is only set to `true` when the user presses **Use AI** inside the dialog. Cancel leaves it `false`
and unticks the box. Once `ai_setup_done && translate_with_ai`, the header band shows a compact
status row instead of re-opening the dialog: model name, "Ready" / "Folder missing", a
"Configure…" link (reopens the dialog), and the live progress bar while a job runs. The check is
re-run whenever the model folder or model id in `Prefs` changes.

### 3.1 Setup dialog — `AiSetupDialog`

A `dialogs::Dialog`-style modal owned by the panel (`ai_dialog: Option<AiSetupDialog>` in
`PubgenState`, rendered from `show()` like the existing pubgen modals). Five states in one window,
top to bottom:

**0. Provider** — two radio cards; everything below switches on the choice:

- **Offline on this computer** → states 1–4 below (hardware, verdict, model list, actions).
- **Online — Groq (free account)** → a compact pane instead of 1–2: a `Password` field for the API
  key with a **Get a free key** hyperlink to `console.groq.com/keys`, a model picker filled by
  `OnlineTranslator::list_models` with a Refresh button (id remembered in `Prefs`), and two honest
  lines — *"Catalog text is sent to Groq's cloud"* and *"Free-tier limits: big jobs run slower,
  not blocked"*. The hardware probe, fit banners and model download never run on this path.
  `Use AI` enables on a non-empty key and persists provider + key + model through
  `prefs.set` (§2). `TranslateJob` (§4) then runs a `Backend::Online`; the collect/generate flow
  and all EN injection sites are unchanged.

**1. Hardware readout** — filled synchronously when the dialog opens (the probe is a few
microseconds; no thread needed):

```
This computer
  RAM      6.9 GB available of 15.8 GB
  CPU      8 logical cores
  GPU      none detected (CPU inference)
```

Both numbers are shown, because *available* is the one that decides fit and it changes minute to
minute. Refreshed by a "Re-check" button.

**2. Verdict banner** — colour-coded, plain language:

| `fit()` | Banner |
|---|---|
| `Fits` | green — "This computer can run offline translation." |
| `Tight` | amber — "This computer can run offline translation, but close other apps first. Peak use is about 4.2 GB." |
| `TooSmall` | red — "Not enough memory for the offline model. Close other apps, or use Single Lang / add `title_en` columns." |

Thresholds: `available >= peak_ram_bytes * 1.25` → `Fits`; `available >= peak_ram_bytes` → `Tight`;
else `TooSmall`. On `TooSmall` the model list and **Use AI** are disabled; only Cancel remains.

**3. Model options** — a radio list built from `MODEL_OPTIONS`, each row showing:
name, download size, peak RAM, a fit badge from `fit()`, and the speed estimate for this machine:

```
(•) NLLB-200 distilled 600M · int8        1.8 GB download · 4.2 GB peak
    Est. 4.8 tokens/s — about 20 s per paragraph, 35 s per page   [fits]
    Licence: CC-BY-NC-4.0 (non-commercial)
```

An option whose `fit()` is `TooSmall` is listed but radio-disabled with a "needs 4.2 GB" note, so
the user sees why. The estimate line repeats the word "Est." because it is derived from core count,
not measured; once a real run finishes, the dialog's last line shows the **measured** rate from
that run so the estimate can be corrected by the user's own experience.

**4. Actions** — `[Use AI]` `[Download model]` `[Browse…]` `[Cancel]`

- If `model_files_present(dir)` is already true → `Use AI` enabled, `Download model` hidden.
- Otherwise both are shown; `Download model` runs the downloader (§3.2) and turns into a progress
  row with bytes-done / total and a Cancel. On success it flips to `Use AI`-enabled.
- `Browse…` uses the existing `app.services.pick_open` pattern (`pubgen.rs:1511-1512`) to point at a
  folder the user already has; it re-checks `model_files_present` immediately.
- `Use AI` writes `translate_with_ai = true`, `ai_setup_done = true`, persists path + id through
  `app.run("prefs.set", json!({"translationModelDir": dir, "translationModelId": id}))`.

The downloader is optional and gated the same way as inference: `download = []` feature adding
`ureq` (pure-Rust TLS via `rustls`, wasm-safe to compile though unusable there). Without the
feature the `Download model` button is hidden and only `Browse…` is offered. Downloads go to
`profile::config_dir()/models/<model id>/` via a `.part` file renamed on completion, each file
checked against the pinned `sha256`; a mismatch deletes the file and reports, never keeps it.

### 4. Two-phase generation (the core trick)

`execute_generate_pages` keeps its shape. Translation happens **before** it, in the Generate
button handler (`pubgen.rs:666-674`):

```
if bilingual_output && translate_with_ai {
    if translations not collected for this slice -> collect + spawn worker, return
    // on worker Done: store maps, then fall through
}
execute_generate_pages(app, st)
```

**Collect pass** `collect_translation_units(st, slice) -> Vec<String>` walks the same rows the
generator will walk and gathers, deduped and order-stable:

- row title (`title` column) — dividers, chapters, tables, charts
- `cover_title`, `cover_title2`
- note titles
- for rows whose `file` is an RTF with no `file_en`: the imported plain text, split per paragraph

TOC entries are page titles, so they are covered by the translated titles — no separate pass.

**Lookup helper** used at every EN injection point instead of `title_en_val` alone:

```rust
fn en_text<'a>(st: &'a PubgenState, source: &str, existing: &'a str) -> &'a str {
    if !existing.is_empty() { existing } else { st.translations.get(source).map(String::as_str).unwrap_or("") }
}
```

Rewrite sites (mechanical):

| Site | Change |
|---|---|
| `pubgen.rs:4824-4843` divider title (template) | fall back to `en_text(st, &final_title, title_en_val)` before `split_pair` |
| `pubgen.rs:5315-5334` divider title (procedural) | same |
| `pubgen.rs:4906-4927` table title slots | `table_title_en` = `en_text(st, &id_title, en_title_opt)` |
| `pubgen.rs:5421-5448` chart title (procedural) | no `" / "` pair → translate `title_val`; render `ID⏎EN` with the EN line italic |
| `pubgen.rs:5592`, `:5779` table presets | same fallback string for the second line |
| `pubgen.rs:1873-1875` cover | `title_en` slot = `en_text(st, cover_title, "")` |
| `pubgen.rs:4973-5019`, `:5189-5296`, `:4888`, `:4994` bodies/notes | when `file_en` is empty, use `st.translated_bodies.get(source_rtf)` |

**Temp RTF for bodies**: `write_plain_rtf(text, path) -> Result<(), String>` (~20 lines: escape
`\ { }`, newlines → `\par`) writing into a per-run temp dir (`std::env::temp_dir()` + process id).
Paths land in `translated_bodies`, so the existing `file.place` story-slot path works unchanged.
The temp dir is removed when the panel closes or the document changes; generation must tolerate a
missing temp file (skip the slot, report a warning) rather than fail the page.

Results stay in state for the session, so a second Generate run on the same rows does no inference.

### 4.1 Styled tables — translate the matrix, never the styled cells

Table styling in Pubgen is **procedural**: `execute_insert_table_matrix` (`pubgen.rs:6087`) builds
every cell and applies `card2_style` (Horizontal Rules, …) from code. In-place editing of a built
table would need cell → story → run-offset mapping and would wipe char styles on replace. Instead:

```
ID matrix ─► dedupe + skip rules ─► MT batch ─► EN matrix ─► execute_insert_table_matrix
                                                                (identical style, same code)
```

**Why this is the cheap path:** the same function renders both tables, so the styling is identical
by construction rather than by copying style state.

Insertion sites: template auto-insert (`pubgen.rs:~4930`) and procedural (`pubgen.rs:~5790`), both
already hold `tbl_matrix` from `parse_csv_content_with_delim`:

```rust
let tbl_matrix = parse_csv_content_with_delim(&content, delim);
let en_matrix = if st.bilingual_output && st.translate_with_ai {
    Some(translate_matrix(&mut st.translations, &tbl_matrix))   // same rows × cols
} else { None };
```

`translate_matrix` flattens → dedupes → filters → one batched `Translator::translate` → rebuilds a
same-shape `Vec<Vec<String>>`. Shape is preserved by construction.

**Cells that are never sent to the model** (a guard, not a prompt instruction — NLLB invents digits):

| Cell kind | Rule |
|---|---|
| numeric / decimal / thousands-separated | `is_numeric_cell`, pass through |
| currency, `%`, `Rp`, unit suffixes | pass through |
| dates and bare years | pass through |
| one token, ≤ 2 chars | pass through |
| `total_words` aliases (`jumlah`, `tahun`, `kabupaten`, `kec`, …) | map via `locale::words(Locale::En)` (`pubgen_locale.rs:89`) — deterministic, no MT |
| empty | pass through |

**One table, two lines per cell.** Both languages share the *same* table and the *same* rows. A
cell's text is:

```rust
format!("{id}\u{2028}{en}")          // designcraft_doc::story::FORCED_LINE_BREAK
```

This works because `table.convertFromText` (`crates/engine/src/cmd/table.rs:615`) splits rows on
`\n` and columns on `\t` **only** — a U+2028 inside a cell is not a row separator, so the matrix
shape is untouched and the pair renders as two lines in that one cell. `FORCED_LINE_BREAK` is
already the line-break character used by every other bilingual renderer in this feature.

So the matrix is **not** reshaped and **not** duplicated:

```rust
let tbl_matrix = parse_csv_content_with_delim(&content, delim);
let display_matrix: Vec<Vec<String>> = if st.bilingual_output && st.translate_with_ai {
    pair_cells(&tbl_matrix, &mut st.translations)   // "id\u{2028}en" per cell, same rows × cols
} else {
    tbl_matrix
};
// one execute_insert_table_matrix(app, st, Some(&display_matrix), page)
```

`pair_cells` returns the original cell unchanged for every skipped kind (numbers, currency, dates,
aliases, empty) — those cells stay single-line, which is correct: a translated `2024` or `3.14`
would be an error, and the alias map already yields English.

**Italicising the EN line** is the one real cost. `execute_insert_table_matrix` ends by calling
`table.convertFromText` on the whole story, so per-cell styling must run after conversion. Use the
existing post-pass hook (`post_process_template_applied_page`'s pattern): for each cell whose text
contains U+2028, compute the char offset of the second line and apply italic over it

```rust
text.select { story, anchor: <char offset of EN>, focus: <end of cell> }
type.char  { fontStyle: "Italic" }
```

Do this as one pass after insertion, not per cell during it. If the first cut ships without the
italic pass, the table is still correct and readable — italics are polish, not structure.

**Overflow:** EN runs ~20–30 % longer, and now it is on a second line in the same column, so the
column must fit `max(id_len, en_len)`, not their sum. Reuse the existing column-fit post-pass and
add one warning-only preflight — never auto-shrink the font:

```rust
if en_cell.chars().count() > id_cell.chars().count() * 3 / 2 {
    warn("Table column may overflow after translation")
}
```

Report through `card1_status`.

**Proper nouns** (district and agency names such as `Karawang Timur`): tier 2. If a first real run
mangles them, build a per-catalog preserve list (words in the `title` column, or repeated ≥ 3×
across cells), substitute `⟦1⟧`-style tokens before MT and restore after. Not in the first cut.

**Tests:** pure-function tests for `pair_cells` — same shape, `"id\u{2028}en"` for translatable
cells, untouched single-line for every skip rule. One end-to-end test feeds a paired matrix through
`execute_insert_table_matrix` and asserts the resulting table has exactly the input row and column
counts (i.e. U+2028 did not split a row) and that each paired cell's text contains one U+2028.

### 5. Feature wiring

- `crates/ui-egui/Cargo.toml`: `designcraft-translate = { workspace = true }`;
  feature `ai-translate = ["designcraft-translate/nllb"]`,
  feature `ai-translate-online = ["designcraft-translate/groq"]`.
- `apps/designcraft/Cargo.toml`: `ai-translate = ["designcraft-ui-egui/ai-translate"]`,
  `ai-translate-online = ["designcraft-ui-egui/ai-translate-online"]`.
- Root `Cargo.toml`: add the `designcraft-translate` path dependency.
- Default build stays a stub and wasm-safe; `cargo xtask ci` unchanged and green.
- Document `cargo run --release -p designcraft --features pubgen,ai-translate` in
  `docs/pubgen-guide.md` and `docs/pubgen-guide_id.md`, including folder layout and licence.

## Files

| File | Work |
|---|---|
| `crates/translate/Cargo.toml`, `src/lib.rs`, `src/nllb.rs`, `src/online.rs`, `src/system.rs`, `src/models.rs`, `src/download.rs` | new crate: errors, stub, ort inference, tokenizer, segmentation, memory probe, model catalogue + speed estimate, downloader, Groq client (chat completions, 429 backoff, model list) |
| `crates/translate/tests/` | segmentation, dedupe, RTF escaping, `fit()` thresholds, `estimate_speed` monotonicity, online prompt/parse/429 tests (model and network not needed) |
| `crates/engine/src/lib.rs`, `crates/engine/src/cmd/prefs.rs` | `translation_model_dir`, `translation_model_id`, `translation_provider`, `translation_groq_api_key`, `translation_groq_model` + params doc |
| `crates/ui-egui/src/panels/pubgen.rs` | state, setup dialog (provider radio + Groq pane), job, collect pass, `en_text`, 8 lookup sites, RTF writer |
| `crates/ui-egui/Cargo.toml`, `apps/designcraft/Cargo.toml`, root `Cargo.toml` | deps + features |
| `xtask/src/layers.rs` | register `translate` at L3 |
| `docs/pubgen-guide.md`, `docs/pubgen-guide_id.md`, `ROADMAP.md` | usage, licence, progress line |

## Verification

1. `cargo xtask ci` — fmt, clippy, tests, assets, layers, wasm (default build: stub only).
2. `cargo check -p designcraft-translate --features nllb`, then
   `cargo run -p designcraft-translate --features nllb --example translate -- "teks uji"`.
3. Unit tests: paragraph segmentation, dedupe, RTF escaping, `en_text` fallback order (existing EN
   wins over AI), cancel flag aborts mid-batch.
4. Headless UI run:
   `cargo run --release -p designcraft --features pubgen,ai-translate -- --sample --control 7979`,
   `ui.menu.invoke` to open Pubgen, set bilingual + translate + model folder; `ui.screenshot` and
   inspect that the EN half is populated and the italic table/chart line is right. Compare with
   `plan/indesign/02-ui-ux.md`.
5. Missing-model path: tick the box, dialogs opens with no model → `Use AI` stays disabled,
   `Download model` fails cleanly with a clear message, no panic, no partial document.
6. Cancel mid-run → no pages generated, document unchanged.
7. **8 GB machine (the target):** open a real publication, tick Translate with AI, and confirm the
   dialog reports `fit() = Fits` or `Tight`, then run a 5-row range while watching peak RSS in
   Task Manager — it must stay under ~4.5 GB and must not climb across rows (the
   `enable_cpu_mem_arena(false)` check). Repeat a second Generate without restarting: peak must
   not grow (loader cache).
8. `TooSmall` path is testable without a small machine: a `#[cfg(test)]` override of
   `memory_info()` (test hook) feeds a 2 GB `MemoryInfo` and asserts the red banner, disabled
   radio options and disabled `Use AI`.
9. Online path without network flake in CI: unit tests for prompt build, fence strip, JSON parse,
   count-mismatch error, and 429 + `Retry-After` backoff against a local canned TCP listener;
   `groq` feature stays off on the wasm target (`cargo xtask wasm` green).
10. Online smoke with a real free key (manual, not CI): tick Online, paste key, pick a model, run a
    5-row range — EN halves populated, key absent from logs/errors/undo, Cancel between batches
    aborts with the document untouched.

## Non-goals

No GPU, no other language pair (the `Lang` enum makes adding one a one-line mapping), no online
provider besides Groq (the OpenAI-compatible call means a second provider is config, not code),
no streaming/chat UX, no translation outside Pubgen, no changes to `template.page.apply`.
