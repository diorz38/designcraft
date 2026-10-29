# Future Plan — Standalone PubGen (Separation Execution Plan)

- **Document:** `docs/plans/2028-09-26-future_plan_standalone_pubgen.md`
- **Horizon / Target Date:** 2028-09-26
- **Status:** Strategic execution plan — not started. Refines the execution phases (§5) of [`2028-06-09-future_plan.md`](2028-06-09-future_plan.md); that document keeps the product rationale, this one is the engineering contract.
- **Target:** new `crates/pubgen` (headless domain core), `apps/pubgen-cli`, `apps/pubgen-studio`; `crates/ui-egui/src/panels/pubgen/` reduced to a thin panel.
- **Effort:** ≈ 12–17 focused days across five phases (spread over 2027–2028, not one sprint).
- **Prerequisites:** Pillars 1–4 of [`2026-10-25-pubgen-future-roadmap-and-extension-plan.md`](2026-10-25-pubgen-future-roadmap-and-extension-plan.md) feature-complete enough that the extraction moves stable code; the vector chart engine (M4) should be **born in `crates/pubgen`**, not in `ui-egui`.

---

## 1. Goal and non-goals

### 1.1 Goal

Make PubGen a product that can ship **without** the DesignCraft editor, while DesignCraft itself is still moving:

1. **One headless core** — `crates/pubgen` owns Excel ingestion, tables, TOC, covers, header/footer, the chart engine, the preflight audit, AI drafting and the project compiler. It depends on the document model and exporters, never on egui.
2. **One shell-neutral contract** — the core reaches the document through a small `PublicationHost` trait (command execution + document read-back). `ui-egui` implements it today; a standalone app implements it tomorrow; tests implement it with no UI at all.
3. **Three delivery surfaces from the same core** — the DesignCraft panel, a headless CLI (batch/CI), and a focused desktop studio.
4. **A split repository only after the contract is proven** — not before. The monorepo is what keeps the two products in lockstep while DesignCraft's APIs are still moving.

### 1.2 Non-goals

- Removing manual design tools from DesignCraft (that is a separate product decision, argued in `2028-06-09-future_plan.md` §2).
- Reimplementing text layout, shaping, PDF writing or the `.designcraft` format inside PubGen. PubGen consumes them; it never forks them.
- A plugin SDK, a cloud service, multi-tenant auth, or a billing layer.
- Any change to the `.designcraft` file format.
- Shipping the studio GUI before the headless CLI is stable.

### 1.3 Why this is safe while DesignCraft is in heavy development

The separation is not "extract everything and hope". It is **narrow the public surface first**:

- DesignCraft exposes everything user-visible as a command (`crates/engine/src/cmd/*`), and `DesignApp::run` (`crates/ui-egui/src/lib.rs:1073`) already dispatches menus, shortcuts, palette, panels and the control channel through one entry point. That is the stable seam.
- PubGen needs a genuinely small slice of DesignCraft: `designcraft-doc` (pages, frames, stories, tables, `Content::Shape(Path)`), the command runner, `designcraft-format` (`.designcraft` read/write) and the exporters (`render`, `pdf`).
- Every dependency crossing the seam goes through **one adapter**, so an upstream API change is a one-file fix, not a 20k-line migration.
- A golden **workbook → `.designcraft` → PDF** test in `cargo xtask ci` turns upstream breakage into a red build on the same day, not a silent runtime regression.

---

## 2. Current State (verified 2026-10-28)

| Fact | Evidence | Consequence |
| :--- | :--- | :--- |
| PubGen is UI-layer code | `crates/ui-egui/src/panels/pubgen/` (23 files) | Nothing below L6 can use it; it cannot ship without egui |
| Its modules are entangled with the UI | e.g. `mod.rs` imports `egui`, `crate::DesignApp`, `crate::theme::Tokens`, `crate::widgets`; `divider_art.rs` imports `crate::DesignApp`; most files `use super::*` | Extraction is real work, not a folder move |
| `mod.rs` is already modular | Pillar 5 executed: `mod.rs` is a ≈198-line entrypoint, domains are flat files | The file split is done; only the **crate** split remains |
| A `pubgen` feature already exists | `crates/ui-egui/Cargo.toml` `[features] pubgen = []`; `apps/designcraft` `pubgen = ["designcraft-ui-egui/pubgen", "ai-translate-online", "ai-cover-online"]` | The panel is already optional; the feature is the natural extraction boundary |
| AI helpers live at L3 | `designcraft-translate`, `designcraft-imagegen` are Layer(3) | A headless core can use them without the engine |
| The engine already depends on the exporters | `designcraft-engine` deps: `doc`, `compose`, `render`, `format`, `pdf`, `tools`, … | A core that avoids the engine stays at L4 and is portable |
| Layering is enforced | `xtask/src/layers.rs` `TABLE` (line 35), `INTRA_LAYER_ORDER` (line 67), UI crates forbidden below L6 (`UI_MIN_LAYER = 6`) | The new crate must be registered and must have zero UI dependencies, checked by CI |
| Apps are exempt | `apps/*` are `Class::Exempt`; workspace members are `crates/*`, `apps/*`, `xtask` | Standalone binaries can depend on core + engine freely |
| File I/O is centrally guarded | `crates/engine/src/tests_file_guard.rs` allow-lists each `ui-egui/src/panels/pubgen/*.rs` that touches the filesystem | Moving `excel.rs`/`compile.rs`/`csv.rs` requires updating those paths and reasons |
| Project compile exists | `crates/ui-egui/src/panels/pubgen/compile.rs` (`pubgen.compileProject`) | It is the first thing that must run headless |
| Stage | `ROADMAP.md`: pre-alpha | Beta-grade API stability is not there yet; a separate repo now would be version-bump churn |

---

## 3. Principles

1. **Separate the product, not the codebase — yet.** Same monorepo, new crates, new binaries. Split the repository only after the seam has survived real releases.
2. **Mechanical moves first.** No renaming, no signature changes, no opportunistic cleanup in extraction commits. Behaviour-preserving moves keep the test suite meaningful.
3. **One extraction = one commit.** `cargo xtask ci` green after every commit (fmt, clippy, tests, assets, layers, wasm).
4. **The core never knows about a window.** No `egui`, `eframe`, `winit`, `rfd`, `craft-ui`, `DesignApp`, `Tokens`, or `i18n::tr` in `crates/pubgen`.
5. **The host is injected, not imported.** The core calls `host.run(command, params)` and reads `host.document()`; it does not own a `Session`.
6. **Messages, not strings.** The core returns structured message keys + parameters; each shell translates. This is what lets the CLI print English while the panel prints Indonesian.
7. **The core is hostile-input safe.** Workbooks, manifests, CSVs, profiles and AI responses are untrusted: bounded counts, `get()` over indexing, checked arithmetic, no panics, explicit errors.
8. **Golden files are the contract.** A synthetic workbook compiled to `.designcraft` and PDF, byte-compared after normalization, is the regression gate for both products.
9. **No new crates for the sake of it.** External deps stay at what `ui-egui` already uses (`serde`, `serde_json`, `zip`, `quick-xml`, `image`).
10. **The chart engine is born headless.** Pillar 1 must land in `crates/pubgen` (§7), never in `ui-egui`; otherwise it will be extracted twice.

---

## 4. Architecture Decisions

### 4.1 Target topology

```text
┌──────────────────────────────┐   ┌──────────────────────────────┐   ┌──────────────────────────────┐
│ apps/designcraft             │   │ apps/pubgen-cli              │   │ apps/pubgen-studio           │
│ (editor + PubGen panel)      │   │ (headless batch compiler)    │   │ (focused publishing GUI)     │
│ L7 exempt                    │   │ L7 exempt                    │   │ L7 exempt                    │
└──────────────┬───────────────┘   └──────────────┬───────────────┘   └──────────────┬───────────────┘
               │                                  │                                  │
      ┌────────┴────────┐                ┌────────┴────────┐                ┌────────┴────────┐
      │ crates/ui-egui  │ L6             │ (implements     │                │ (implements     │
      │ thin PubGen     │                │  PublicationHost│                │  PublicationHost│
      │ panel           │                │  over Session)  │                │  over Session)  │
      └────────┬────────┘                └────────┬────────┘                └────────┬────────┘
               │                                  │                                  │
               └──────────────────┬───────────────┴──────────────────┬───────────────┘
                                  ▼                                  ▼
                        ┌──────────────────┐               ┌──────────────────┐
                        │ crates/pubgen    │ L4            │ crates/engine    │ L5
                        │ domain core      │               │ Session, cmds,   │
                        │ no egui, no engine│              │ Prefs, undo      │
                        └────────┬─────────┘               └────────┬─────────┘
                                 │                                  │
        ┌────────────────────────┼────────────────────────┐         │
        ▼                        ▼                        ▼         ▼
 designcraft-doc        designcraft-format        designcraft-pdf / render / compose / translate / imagegen
 (L1)                   (L3)                      (L3)
```

`crates/pubgen` sits at **Layer(4)** because its highest workspace dependency is L3 (`format`, `pdf`, `render`, `compose`, `translate`, `imagegen`). It must not depend on `engine` (L5) or `tools` (L4). The engine-backed and UI-backed `PublicationHost` implementations live in the apps/UI, which may depend on both.

### 4.2 What moves, what stays, what splits

`crates/ui-egui/src/panels/pubgen/` today (23 files). Destination is decided by **dependency**, not by topic:

| File | Destination | Notes |
| :--- | :--- | :--- |
| `csv.rs` | `pubgen` | Pure parsing/predicates |
| `locale.rs` | `pubgen` | Locale definitions, no UI |
| `vocab.rs` | `pubgen` | Terminology data |
| `themes.rs` | `pubgen` | `TableColorTheme` / palette data; UI swatch widgets stay |
| `excel.rs` | `pubgen` | XLSX → CSV package; file I/O; file-guard allow-list moves |
| `tables.rs` | `pubgen` | Matrix → document items; needs `doc` only |
| `pages.rs` | `pubgen` | Page emission through the host |
| `page_utils.rs` | `pubgen` | Document-level utilities |
| `toc.rs` | `pubgen` | TOC plan + generation through the host |
| `covers.rs` | `pubgen` | Cover specs + execution through the host |
| `header_footer.rs` | **split** | Template/spec/preview geometry → `pubgen`; dialog + presets UI → `ui-egui` |
| `helpers.rs` | **split** | Doc/frame/story/style helpers → `pubgen`; egui helpers → `ui-egui` |
| `cover_art.rs` | **split** | Cover templates as data/geometry → `pubgen`; egui preview drawing → `ui-egui` |
| `divider_art.rs` | **split** | Imports `crate::DesignApp` today; vector spec → `pubgen`, rendering → `ui-egui` |
| `ai.rs` | **split** | Provider, prompts, jobs → `pubgen` (translate is L3); dialogs/status rows → `ui-egui` |
| `ai_cover.rs` | **split** | Prompt/palette model → `pubgen`; preview panel → `ui-egui` |
| `ai_cover_job.rs` | `pubgen` | Background job loop; no egui |
| `compile.rs` | `pubgen` | Manifest, planner, job, report, TOC fixed point |
| `state.rs` | **split** | Persistent project/profile settings → `pubgen::Settings`; transient UI state → `ui-egui` |
| `testutil.rs` | **split** | Document fixtures → `pubgen` test support; grid/UI fixtures stay |
| `mod.rs` | `ui-egui` | Panel entrypoint only |
| `widgets.rs` | `ui-egui` | Panel chrome |
| `thumbnails.rs` | `ui-egui` | egui image widgets |
| `charts/` (new, Pillar 1) | `pubgen` | Must never be created under `ui-egui` |
| `audit/` (new, Pillar 3) | `pubgen` | Same rule |

Anything that satisfies both tests — "touches no egui type" and "returns data or runs commands through the host" — belongs to the core.

### 4.3 The seam: `PublicationHost`

The core needs exactly four capabilities from its shell. Keep the trait this small:

```text
trait PublicationHost {
    fn run(&mut self, command: &str, params: Value) -> Result<Value, String>;
    fn document(&self) -> &Document;
    fn doc_uid(&self) -> u64;                 // detect a tab switch mid-compile
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, String>;   // optional, wasm-friendly
}
```

- `DesignApp` already has `run` (`crates/ui-egui/src/lib.rs:1073`), so the panel implementation is a thin wrapper.
- `apps/pubgen-cli` implements it over a headless `Session` (exempt app, may use L5).
- Tests implement it over a `Document` + a scripted command map, so no UI and no windowing are needed.
- Reading the document through a trait, not through `Session`, is what keeps `crates/pubgen` at L4 and lets the same core run in WASM later.

`doc_uid` mirrors the guard already added to `compile.rs` after Pillar 2: a compile must abort rather than emit chapters into the wrong document.

### 4.4 Settings, i18n, assets, file I/O

| Concern | Today | Target |
| :--- | :--- | :--- |
| Settings | `Prefs.pubgenGlossaryPath`, `Prefs.pubgen_project_path` in `crates/engine` | `pubgen::Settings` owns the schema; the engine mirrors it during the transition, then the mirror is deprecated. The standalone app reads a `pubgen.json` beside the manifest or in the OS config dir |
| i18n | `crate::i18n::tr` (ui-egui) | Core returns `Message { key, params }`; each shell translates. `ui-egui` keeps its catalog; the CLI gets a small English (and optional Indonesian) catalog |
| Assets | Resolved via `CARGO_MANIFEST_DIR/../..` (e.g. `assets/pubgen/bps_glossary.json`) | `pubgen::assets::root()` resolves from the workspace root in the monorepo and from the install dir in a packaged app; keep the same search order (`Settings` path → cwd → workspace root) |
| File I/O | Allow-listed in `crates/engine/src/tests_file_guard.rs` | Update the allow-list to `pubgen/src/...` with new reasons; the CLI's own I/O is in an exempt app |
| wasm | Panel modules are `#[cfg]`-gated; compile has a wasm stub | `pubgen` must stay wasm-clean for the read-only paths; file-system modules are `#[cfg(not(target_arch = "wasm32"))]`, `cargo xtask wasm` is the gate |
| Fonts | `designcraft-fonts`, optional `CRAFT_FONTS_DIR` | Unchanged; the standalone build documents the same optional input and must build without it |

### 4.5 Why not split the repository now

| Concern | Monorepo (chosen now) | Separate repo (later) |
| :--- | :--- | :--- |
| DesignCraft API churn | One `cargo update`, one commit, atomic fix | Every upstream change becomes a cross-repo version bump and a PR in each |
| Golden tests | Run in the same CI as the thing they test | Need a pinned dependency + a scheduled cross-repo job |
| Refactors | `cargo xtask` sees both sides of the seam | You cannot change both sides in one commit |
| Release cadence | One workspace version, PubGen pinned to a commit | Independent, but only meaningful once the contract is stable |
| Contributor friction | One clone, one build | Two clones and a local path override to work on both |

The repository split is a **release-engineering decision**, not an architecture prerequisite. It becomes correct when all of the following hold:

1. The `PublicationHost` trait and `pubgen::Settings` schema have been unchanged for two release cycles.
2. The `.designcraft` format is at a stable major (or PubGen only consumes a documented subset).
3. At least one external consumer (an agency build or a downstream fork) depends on the CLI.
4. The golden suite runs green from a pinned git tag in a clean checkout.

### 4.6 Alternatives considered and rejected

| Alternative | Why rejected |
| :--- | :--- |
| **Drive DesignCraft over the control channel / MCP** | Requires a running app, adds a socket hop to every command, cannot run in WASM, and cannot be embedded. Acceptable as a *cloud batch stopgap* (already documented in `docs/AGENTS.md`), not as the architecture |
| **Fork the layout engine into PubGen** | Duplicates the most expensive, fastest-moving code (`compose`, `pdf`, `doc`). Guaranteed divergence and double maintenance |
| **Immediate repository split** | Constant cross-repo version bumps while both are pre-1.0; no atomic seam changes; CI cannot test both sides together |
| **Keep everything in `ui-egui` and add a second binary that links `ui-egui`** | The binary still contains the editor, the panels and the UI toolkit; no headless path; WASM and server builds stay blocked |
| **A `pubgen` engine command per feature** | Moves Excel/domain rules into `engine`, violating the layering and the "engine is generic" rule. The existing precedent is UI-layer orchestration via commands; keep it |

### 4.7 Versioning and the contract

- `pubgen` exposes one small public API: `Settings`, `ProjectManifest`, `CompileJob`/`CompileReport`, `PublicationHost`, the message type, and the domain entry points (`convert_excel`, `generate_pages`, `generate_toc`, `emit_chart`, `audit`).
- Everything else is `pub(crate)`.
- `pubgen::API_VERSION: u32` is bumped on any breaking change to that list; the CLI prints it in `--version`; the panel asserts it at startup in debug builds.
- The workspace `version` stays single-source (`Cargo.toml` `[workspace.package] version`) until the split; afterwards PubGen gets its own line and pins `designcraft-*` by tag + `sha256` where it consumes binary assets.

---

## 5. Target Workspace Layout

```text
crates/
├── pubgen/                       ← new, Layer(4)
│   ├── src/
│   │   ├── lib.rs                // public API, API_VERSION, error type
│   │   ├── host.rs               // PublicationHost trait, Message
│   │   ├── settings.rs           // pubgen::Settings (serde, defaults)
│   │   ├── assets.rs             // workspace/install-root resolution
│   │   ├── excel.rs              // XLSX → CSV package
│   │   ├── csv.rs
│   │   ├── locale.rs
│   │   ├── vocab.rs
│   │   ├── themes.rs
│   │   ├── tables.rs
│   │   ├── pages.rs
│   │   ├── page_utils.rs
│   │   ├── toc.rs
│   │   ├── covers.rs
│   │   ├── header_footer.rs
│   │   ├── compile.rs            // manifest, planner, job, report
│   │   ├── ai.rs                 // prompts/providers/jobs
│   │   ├── ai_cover.rs
│   │   ├── ai_cover_job.rs
│   │   ├── charts/
│   │   │   ├── mod.rs            // Pillar 1 engine (born here)
│   │   │   └── vector.rs         // bar/line/donut/area emitters
│   │   ├── audit/                // Pillar 3 rules
│   │   └── testutil.rs
│   └── Cargo.toml                // doc, compose, render, format, pdf, translate, imagegen
│
├── ui-egui/src/panels/pubgen/    ← thin panel, L6
│   ├── mod.rs, state.rs, widgets.rs, thumbnails.rs
│   ├── host_impl.rs              // impl PublicationHost for DesignApp
│   └── dialogs/                  // AI dialogs, H&F dialog, cover preview
│
apps/
├── designcraft/                  ← unchanged editor + panel
├── designcraft-cli/              ← unchanged
├── pubgen-cli/                   ← new, exempt: compile / batch / convert / audit
└── pubgen-studio/                ← new, exempt: focused GUI shell
```

No new `designcraft-*` crate is required for the studio: it composes `pubgen` + `engine` + a small egui shell. If binary size becomes a hard requirement, add `crates/pubgen-ui` for shared widgets later; do not pull the whole `ui-egui` into the studio.

---

## 6. Phases

Effort is for one engineer already familiar with the PubGen panel. Every phase ends with `cargo xtask ci` green and a commit.

---

### Phase 0 — Freeze the seam (~0.5 d)

1. Record the baseline: `cargo xtask ci` result, test count, `cargo build --release -p designcraft` time, binary size, cold start.
2. Add the golden corridor: two synthetic chapter catalogs (no committed `.xlsx`; generated in the test, as `compile.rs` already does) → one `.designcraft` → PDF. Normalize the document JSON (timestamps/ids) and compare against a checked-in expectation or a structural assertion set.
3. Add a CI assertion that `crates/pubgen` has **zero** UI-crate dependencies once it exists (a unit test in `xtask/src/layers.rs`, not a convention).
4. Document the seam in this file and link it from `2028-06-09-future_plan.md`.

**Gate:** baseline recorded; golden test green; no source moved.

---

### Phase 1 — Create `crates/pubgen` and move the leaves (~2–3 d)

1. Add `crates/pubgen/Cargo.toml` (deps: `designcraft-doc`, `-compose`, `-render`, `-format`, `-pdf`, `-translate`, `-imagegen`; `serde`, `serde_json`, `zip`, `quick-xml`, `image` as needed). No `engine`, no `tools`, no UI crates.
2. Register `("pubgen", Class::Layer(4))` in `xtask/src/layers.rs` `TABLE`. No `INTRA_LAYER_ORDER` entry is needed at L4.
3. Move the dependency-free leaves: `locale`, `vocab`, `csv`, `themes`, `excel`, `page_utils`, `ai_cover_job`, and the non-UI halves of `helpers`/`ai`/`testutil`.
4. Add `host.rs` (`PublicationHost`, `Message`), `settings.rs`, `assets.rs`, `error.rs`.
5. `ui-egui` re-exports the moved items during the transition so the panel compiles unchanged; the panel imports from `designcraft_pubgen` instead of `super::`.
6. Update `crates/engine/src/tests_file_guard.rs` allow-list paths and reasons for every moved file that performs I/O.

**Deliverable:** `crates/pubgen` builds standalone; the panel still works through re-exports.
**Gate:** `cargo xtask ci` green (including `layers` and `wasm`); `pubgen` has no egui dependency and no `engine` dependency; the golden corridor still passes.

---

### Phase 2 — Move the engines and the compiler (~3–4 d)

1. Move `tables`, `pages`, `toc`, `covers`, `compile` and the split halves of `header_footer`, `cover_art`, `divider_art`.
2. Replace every `app.run(...)` with `host.run(...)`; replace every `crate::i18n::tr(...)` with a `Message` return; replace `Tokens` colours with explicit palette values from `Settings`/themes.
3. Implement `PublicationHost for DesignApp` in `ui-egui/panels/pubgen/host_impl.rs` (`run` → `DesignApp::run`, `document` → active document, `doc_uid` → the existing tab guard).
4. Split `PubgenState`: persistent fields move to `pubgen::Settings` (serde-compatible with today's saved state); transient UI fields stay. Dual-read during the transition.
5. Keep the chart engine (Pillar 1) and audit (Pillar 3) landing **in `crates/pubgen` only**.

**Deliverable:** the whole panel is a view over the core; no domain logic remains in `ui-egui`.
**Gate:** full CI green; UI snapshot tests unchanged (or intentionally re-baselined); golden corridor green; `cargo xtask layers` green.

---

### Phase 3 — Headless CLI (~2–3 d)

1. `apps/pubgen-cli`: `pubgen compile <manifest> [--pdf] [--json] [--work-dir]`, `pubgen batch <dir> --profile <id> --out <dir>`, `pubgen convert <xlsx>`, `pubgen audit <manifest>`.
2. Implement `PublicationHost` over a headless `Session`; reuse `crate::cmd` through the registry; no windowing.
3. Deterministic output: same manifest → same `.designcraft` bytes after normalization and the same PDF page count; exit `0` on clean, non-zero with a JSON diagnostic on preflight failure.
4. `--api-version` prints `pubgen::API_VERSION`.
5. CI job runs the CLI on the golden fixtures and compares against the panel result.

**Deliverable:** agencies can compile publications on a server with no GUI.
**Gate:** CLI and panel produce equivalent documents for both fixtures; `cargo xtask ci` green; CLI build in a clean container.

---

### Phase 4 — Studio shell (~3–4 d)

1. `apps/pubgen-studio`: minimal eframe window — manifest picker, profile dropdown, spread preview (via `designcraft-render`), Compile/Cancel, report, export.
2. Do **not** link the editor panels. Reuse `theme`/widget helpers only if they are small; otherwise copy the handful of widgets into the studio.
3. Wire the existing job protocol (one step per frame, cancel flag) through `pubgen::CompileJob`.
4. Package Windows/macOS/Linux builds; document the optional `CRAFT_FONTS_DIR` build input.

**Deliverable:** a non-designer can generate a publication in three clicks.
**Gate:** binary < 25 MB on the first cut (target for a later pass: < 20 MB), cold start < 150 ms, compile/cancel/report parity with the CLI.

---

### Phase 5 — Contract pinning and the optional split (~1–2 d)

1. Freeze `pubgen::API_VERSION`; publish the host trait, the settings schema and the manifest schema as the documented contract.
2. Decide the repository split against §4.5's four conditions. If they are not all true, **stop here** — the monorepo is the correct home.
3. If splitting: create the repository, move `crates/pubgen` + `apps/pubgen-*`, pin `designcraft-*` by tag, and add a scheduled cross-repo golden job that builds both sides.
4. Add a `docs/pubgen-standalone.md` (install, batch, CLI flags, font input, licence) and a release workflow.

**Deliverable:** either a documented, stable embedded product, or a clean standalone repository.
**Gate:** cross-repo (or monorepo) CI green; a release artifact compiles the golden publication outside the DesignCraft tree.

---

## 7. Layering Registration (exact change)

`xtask/src/layers.rs` gains one table row:

```text
("pubgen", Class::Layer(4)),
```

Why L4 is correct:

- Highest workspace deps are L3: `format`, `pdf`, `render`, `compose`, `translate`, `imagegen`.
- `doc`/`geom`/`color` are L1/L0.
- It must **not** depend on `engine` (L5) or `tools` (L4). The host trait is what removes the engine dependency; if a future feature genuinely needs an engine command, it stays a command string sent through `host.run`, not a crate dependency.
- `UI_MIN_LAYER = 6` already rejects any UI crate below L6, so `pubgen` cannot accidentally pull egui. Add the explicit "zero UI deps" assertion anyway, so a future exception is a conscious change.

`ui-egui` (L6) and both new apps (exempt) may depend on `pubgen`. `engine` may not.

---

## 8. CI and Test Strategy

| Layer | Test | Gate |
| :--- | :--- | :--- |
| Unit | Parsers, matrix → items, TOC plans, chart geometry, audit rules | `cargo test -p designcraft-pubgen` |
| Host-contract | A fake `PublicationHost` over a `Document` + recorded command results | No UI required; proves the core is shell-neutral |
| Golden corridor | Synthetic workbooks → `.designcraft` → PDF (normalized) | Same result from panel and CLI |
| Layering | `cargo xtask layers` + "pubgen has no UI deps" unit test | CI |
| wasm | `cargo xtask wasm` | `pubgen` and the read-only paths compile |
| UI parity | Existing egui snapshot tests for the panel | Unchanged after Phase 2 |
| CLI | `pubgen compile` on the fixtures, exit codes, `--json` diagnostics | CI job |
| Studio | Window opens, picks a manifest, compiles, cancels, exports | Manual + one offscreen smoke test |
| Never-crash | Hostile manifest/workbook/CSV/profile/AI-response cases | `cargo xtask ci` (workspace lints deny `unwrap`/`expect`/`panic`) |

---

## 9. Risks & Mitigations

| Risk | Mitigation |
| :--- | :--- |
| DesignCraft's `doc`/`engine` API changes mid-extraction | One `PublicationHost` + one adapter; golden corridor catches breakage the same day; workspace pinning keeps both sides at one commit |
| `.designcraft` format changes | PubGen consumes `designcraft-format` and never writes its own serializer; format version bumps are tested by the golden corridor; regenerate fixtures instead of migrating |
| Behaviour drift during the move | Mechanical moves only; one extraction per commit; tests travel with their domain; UI snapshots unchanged unless intentionally re-baselined |
| Hidden egui coupling in "pure" modules | Compile-time proof: `crates/pubgen` cannot list an egui/eframe/winit/rfd dependency; a layers unit test asserts it |
| `i18n` accidentally imported into the core | The core returns `Message`; a grep/CI test rejects `i18n::tr` in `crates/pubgen` |
| File-guard allow-list breaks after the move | Update `crates/engine/src/tests_file_guard.rs` paths/reasons in the same commit that moves the file |
| Asset paths break (`CARGO_MANIFEST_DIR/../..`) | `pubgen::assets::root()` with the existing search order and one test per root (workspace, packaged, cwd override) |
| Prefs split (engine ↔ pubgen settings) | Dual-read from both during transition; one migration function; remove the engine mirror after two releases |
| Compile blocks the studio UI | Keep the frame-stepped `CompileJob`; the CLI uses `sync`; cancel checked between steps |
| Two binaries double CI time | Separate `CARGO_TARGET_DIR` for parallel agents; `cargo xtask ci` builds the workspace once; add the CLI job only in Phase 3 |
| Studio links the editor and stays huge | Dedicated shell; no `ui-egui` dependency in `apps/pubgen-studio` except optional theme/widgets behind a feature |
| WASM breaks on `std::fs` | File-system modules gated; `pubgen` keeps read-only paths wasm-clean; `cargo xtask wasm` is a required gate |
| Fonts missing in a standalone install | Same optional `CRAFT_FONTS_DIR` contract; the app falls back to bundled/system fonts; release builds set it |
| Asset/licence regression | Every new art asset gets an `ASSETS.md` row (`cargo xtask assets`); no Adobe iconography; PubGen branding gets its own licence row if added |
| Repo split before the contract is stable | §4.5's four conditions are mandatory; default is to stay in the monorepo |

---

## 10. Out of Scope

- Removing editor tools from DesignCraft or renaming the editor.
- New PubGen features beyond Pillars 1–4 (the chart engine and audit are in scope only as "born in the core").
- A cloud service, multi-tenant hosting, authentication, billing.
- A plugin/extension SDK for third parties.
- Changing the `.designcraft` file format or the command IDs/params.
- Incremental chapter recompiles, `.dcbook` per-chapter output, EPUB.
- Shipping the studio before the CLI.
- A separate repository before Phase 5's conditions are met.

---

## 11. Dependency Impact

| Crate / app | Change | New dependencies |
| :--- | :--- | :--- |
| `crates/pubgen` | New at L4 | `designcraft-doc`, `-compose`, `-render`, `-format`, `-pdf`, `-translate`, `-imagegen`; `serde`, `serde_json`, `zip`, `quick-xml`, `image` |
| `crates/ui-egui` | Panel shrinks to a view; `host_impl.rs`; modules moved | Gains `designcraft-pubgen`; loses no external deps |
| `crates/engine` | `tests_file_guard.rs` allow-list paths; `Prefs` mirror during transition | None |
| `xtask` | `pubgen` row + zero-UI-dep assertion | None |
| `apps/designcraft` | `pubgen` feature points at `ui-egui` → `pubgen` | None |
| `apps/pubgen-cli` | New, exempt | `designcraft-pubgen`, `designcraft-engine`, `serde_json` |
| `apps/pubgen-studio` | New, exempt | `designcraft-pubgen`, `designcraft-engine`, `designcraft-render`, `eframe`/`egui` |
| `designcraft-web` | Optional future: link `pubgen` for in-browser compile | `designcraft-pubgen` (wasm-clean) |

---

## 12. Definition of Done

- [ ] `crates/pubgen` exists at L4, has zero UI and zero `engine` dependencies, and passes `cargo xtask layers`.
- [ ] Every domain module from `ui-egui/src/panels/pubgen/` lives in `pubgen` or is a documented split; the panel contains only state, widgets, dialogs and `host_impl`.
- [ ] The panel's domain behaviour is unchanged (UI snapshots + golden corridor).
- [ ] `pubgen compile` runs the golden manifest headless and produces a `.designcraft`/PDF equivalent to the panel's.
- [ ] The chart engine (Pillar 1) and preflight audit (Pillar 3) were written in `pubgen`, not in `ui-egui`.
- [ ] `pubgen::Settings` round-trips today's saved PubGen state; the engine `Prefs` mirror is read-compatible.
- [ ] `PublicationHost` is the only way the core touches a document; a fake host drives a full compile in tests.
- [ ] `cargo xtask ci` green (fmt, clippy, tests, assets, layers, wasm); CLI and studio build.
- [ ] `2028-06-09-future_plan.md` §5 links here for execution; `ROADMAP.md` gets a progress line per landed phase.
- [ ] The repository split is either completed against §4.5's conditions or explicitly deferred with a recorded reason.

---

## 13. Decisions Taken

1. **Monorepo first, separate binary first, separate repository last.** The split is a release decision gated on a stable contract.
2. **`crates/pubgen` at L4, no `engine` dependency.** The host trait is the seam; commands are strings, not types.
3. **Headless CLI before the GUI.** It is the strongest use case (batch/batch-in-CI) and the cheapest way to prove the core is shell-neutral.
4. **The panel stays as a thin view.** No domain logic remains in `ui-egui`.
5. **The core returns message keys, not translated strings.** Every shell translates.
6. **`pubgen::Settings` owns the schema; engine `Prefs` mirrors it during the transition.**
7. **The chart engine and audit are born in the core.** No second extraction later.
8. **Golden workbook → `.designcraft` → PDF is the contract test.** Byte-comparison after normalization.
9. **No layout/PDF/format reimplementation.** PubGen consumes DesignCraft's lower crates.
10. **One extraction per commit; mechanical moves only.**

## 14. Open Questions (non-blocking)

These do not block Phases 0–3; they are decided before Phase 4–5.

1. **Product name and branding** — PubGen vs PubCraft, and whether a distinct mark/asset is added (needs an `ASSETS.md` row and a licence note).
2. **Studio UI source** — a dedicated shell vs a shared `crates/pubgen-ui`; decide with the first binary-size measurement.
3. **Split repository owner/name** — `storytold/pubgen` vs staying in the DesignCraft mono-repo for the foreseeable future.
4. **WASM scope** — read-only preview only, or full in-browser compile (the doc `2028-06-09-future_plan.md` §3.3 argues for full compile; that needs the CLI path proven first).

---

## 15. Revision Log

**2026-10-28** — created. Verified against the workspace at `0.13.0-pubgen.0.7.8`:

1. Confirmed PubGen is L6 UI code (23 files), that Pillar 5 already flattened `mod.rs`, and that the `pubgen` cargo feature is the natural extraction boundary.
2. Confirmed the layering rules (`xtask/src/layers.rs` `TABLE`/`INTRA_LAYER_ORDER`, `UI_MIN_LAYER = 6`) and derived L4 for the new crate from its actual dependency set.
3. Confirmed `DesignApp::run` (`crates/ui-egui/src/lib.rs:1073`) is the existing command seam, and `tests_file_guard.rs` must move its allow-list paths with the I/O modules.
4. Chose the injected `PublicationHost` over a `Session` dependency so the core stays portable and testable without a UI.
5. Chose the headless CLI before the studio so the shell-neutral claim is proven before any GUI work.
6. Effort spread: Phase 0 ≈ 0.5 d · 1 ≈ 2–3 d · 2 ≈ 3–4 d · 3 ≈ 2–3 d · 4 ≈ 3–4 d · 5 ≈ 1–2 d → **≈ 12–17 d**.