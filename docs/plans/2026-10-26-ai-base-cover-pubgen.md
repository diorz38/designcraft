# AI Base Cover Generation — Pubgen Publication Cover

**Date:** 2026-10-26
**Status:** Planned
**Feature flags:** `pubgen,ai-cover-online` (mirrors `pubgen,ai-translate-online`)
**Sibling precedent:** `docs/plans/2026-10-22-offline-ai-translation-pubgen.md`, `crates/translate`, `pubgen/ai.rs`

## 1. Goal

Add **AI Base** cover generation to the Pubgen Publication Cover card: the user picks an AI
style preset + color palette, presses the existing **Generate Cover Page** button (or runs the
Excel batch flow), and Pubgen generates a text-free background illustration via Cloudflare
Workers AI (SDXL), then overlays the existing native text/logo frames on top.

## 2. Agreed design decisions

1. **AI Base is styles 100+, not a separate mode.** `cover_style: usize` splits:
   `0..=99` procedural (0–12 today), `100..` AI presets. One discriminator drives UI options
   block, generation dispatch, thumbnails, and Excel name matching. No radio, no ordering rules.
2. **AI paints background only.** Title/year/volume/catalogue/code/agency + logo stay native
   frames (the strict options-only-text rule). AI never renders text. v1 is **text-to-image
   only** (no img2img upload); img2img is a later extension if needed.
3. **Provider: Cloudflare Workers AI direct REST** (`@cf/stabilityai/stable-diffusion-xl-base-1.0`),
   user's own Account ID + API token, free tier (~10k Neurons/day ≈ ~45 images at 18 steps).
   No proxy server, no backend.
4. **No "set AI before Excel" rule.** `apply_excel_cover` only overwrites style when the sheet
   names a known style; panel selection survives otherwise. AI preset names are *added* to the
   Excel `NAMES` table so the workbook can drive AI too.
5. **Batch is a background job** with pre-flight quota estimate, progress, cancel, and
   stop-on-429 keeping partial results (mirrors `TranslateJob` exactly).
6. **Credentials validated at selection time** (verify-before-accept dialog, mirrors
   `AiSetupDialog::verify_and_accept_groq`), never mid-batch.
7. **Hardcoded ML knobs:** `num_steps: 18`, aspect-matched dims (832×1176 / 768×1184 /
   896×1152 mapped from the cover page aspect), hardcoded text-suppressing negative prompt.
   User sees only: preset, palette, optional seed.

## 3. Architecture

```
crates/imagegen              NEW provider crate (mirrors crates/translate)
  └─ CloudflareClient::generate(req) -> Vec<u8> PNG     [feature "cloudflare", ureq, sync]
assets/templates/covers/100_ai_base.designcraft     cover template: #slot:image:background
  └─ template.page.apply     binds PNG path + 7 text slots + logo + palette (existing command)
crates/ui-egui/panels/pubgen UI: presets, style section, setup dialog, job
  └─ CoverJob                worker thread: HTTP only; UI thread applies via template.page.apply
```

Key split: the worker thread does **HTTP only** and ships the PNG path over a channel; the UI
thread mutates the document through `app.run("template.page.apply", …)` (the same call the
procedural covers make) as results arrive. The engine stays synchronous; undo history stays
sane; partial batch results appear live.

---

## Phase 1 — `crates/imagegen` provider crate

**Scope.** Standalone crate, no egui, no engine deps. `#![deny(clippy::unwrap_used, …)]` from
day one (never-crash standard).

- `ImageGenError`: `Unavailable` (feature off), `Quota` (HTTP 429), `Auth` (401/403), `Network`,
  `BadResponse`, `Cancelled`. `thiserror`, same shape as `translate::TranslateError`.
- `GenRequest { prompt, negative_prompt, width, height, num_steps, seed: Option<u32> }` —
  dims come only from the aspect enum (`Aspect::Iso`, `Aspect::Folio`, `Aspect::Letter`,
  `Aspect::Wrap`), never raw user numbers (hostile-input rule).
- `CloudflareClient::new(account_id, token)` + `with_base_url` (test override).
  `generate(&self, req, cancel: &AtomicBool) -> Result<Vec<u8>, ImageGenError>`.
- Timeout, response-size cap, no `unsafe`, no panics on malformed JSON.
- Cargo: `cloudflare = ["dep:ureq", "dep:serde_json"]`; every type exists without the feature,
  calls return `Unavailable` (translate pattern, lib.rs:48).

**Tests** (canned TCP listener, mirror `translate/src/online.rs:290`): success returns PNG
bytes; 429 → `Quota`; 403 → `Auth`; malformed JSON → `BadResponse`; cancel between calls.
No network in tests.

**Acceptance:** `cargo test -p designcraft-imagegen` green with and without `cloudflare`;
`cargo xtask layers` clean; wasm target untouched (feature never enabled there).

## Phase 2 — Cover template + engine prefs  *(done; scope evolved — better path)*

**Deviation note.** The procedural styles are `.designcraft` template files applied with
`template.page.apply` (`assets/templates/covers/*.designcraft`, `#slot:`-tagged frames). That
existing command already inserts the page, binds arbitrary text/image slots by label, places
pictures and applies palette swatches — so **no new engine command is needed**: applying an AI
cover is the same `template.page.apply` call the procedural path makes, with
`slots.image.background` pointing at the generated PNG. Stays reachable from CLI / MCP /
control channel for free; `pubgen.coverApply` from the original sketch is dropped.

**What landed.**

- `assets/templates/covers/100_ai_base.designcraft`: A4, full-bleed
  `#slot:image:background`, an accent band bound to the palette's Accent swatch, house
  typography slots (title block lifted; over-art text near-white; agency dark on the band).
  Regenerable via `plan/pubgen/build_ai_base_template.ps1` (local; derives from 012).
- Engine template tests: 13 → 14 covers; the photo-slot assertion accepts `background`.
- Prefs: `coverAiCloudflareAccountId`, `coverAiCloudflareApiToken` (engine `Prefs`,
  documented in the `prefs.set` schema line; accepted keys derive from the struct).

**Proof.** CLI script (`prefs.set` + `file.new` + `template.page.apply` with all 7 text slots
+ background/logo images + palette) binds 10 slots and renders the composed cover
(`outputs/ai_cover_applied.png`). Engine template tests green; assets/fmt/clippy clean.

## Phase 3 — Presets, state, UI selection

**Scope.**

- `pubgen/ai_cover_presets.rs`: `AI_STYLE_BASE = 100`; preset table
  `{ id, name, prompt, palettes: Vec<(name, palette_words)>, thumbnail: Option<Png> }`.
  Launch set: 3 presets × 4 palettes (e.g. Retro Waves, Dark Fantasy, Minimal Sci-Fi ×
  Warm Terracotta, Pastel, Monochrome Slate, Forest). Prompt templates from the
  brainstorming conclusions: style + palette + "negative space for typography" +
  hardcoded negative prompt (text/letters/typography/watermark/…).
- State: reuse `cover_style`; add `cover_ai_palette: usize` (default 0, meaningful only for
  styles ≥ 100).
- `widgets.rs`: ComboBox gains "── AI Base (online) ──" section; per-style options block
  (existing `match st.cover_style`, widgets.rs:833) shows palette pills + seed toggle for AI
  styles instead of skyline/logo pickers.
- Setup: `AiCoverSetupDialog` mirrors `pubgen/ai.rs` — Account ID + Token (password field),
  **Verify** button does one tiny generation (256×256, 1 step, a few Neurons) and accepts only
  on success; disabled/hint state on Generate when style ≥ 100 and no verified key.
- Thumbnails: one rendered PNG per preset in `plan/pubgen/samples/thumb/` + `ASSETS.md` rows +
  vector fallback (`paint_fallback_cover_thumb` pattern).

**Tests:** Excel name round-trip ("AI Base: Retro Waves" → 100) extended in
`cover_style_index` NAMES table (excel.rs:3142); AI style options-only-text; ComboBox
section renders (kittest snapshot if panel tests exist).

**Acceptance:** user can select AI style without key (button disabled with hint), set up key
via dialog, palette pills switch.

## Phase 4 — Single-cover generation flow *(done)*

**Scope.**

- `pubgen/ai_cover_job.rs`: `CoverJob` — clone of `TranslateJob` (ai.rs:291–343):
  `std::thread` + mpsc (`Progress/Done/Failed`), `Arc<AtomicBool>` cancel,
  `ctx.request_repaint()`, `poll()` drains into `PubgenState` statuses. Native-only;
  wasm stub returns finished-empty (ai.rs:345 pattern).
- Generate Cover Page branch (covers.rs:990): style < 100 → today's path; style ≥ 100 →
  spawn `CoverJob` with 1 item; on result, `app.run("template.page.apply", …)` on the UI
  thread; status line "Generating AI cover…", then done/failed.
- Error surfacing: `Quota` → "Daily Cloudflare free quota reached; resets 00:00 UTC";
  `Auth`/`Network` → setup dialog offer. Cancel button while running.
- Button enabled-state: AI style without verified key → disabled + hint.

**Tests:** job poll transitions with injected fake client; branch dispatch; cancel keeps
document unchanged.

**Acceptance:** manual run (`cargo run --release -p designcraft --features pubgen,ai-cover-online`):
pick Retro Waves + Warm Terracotta → Generate → cover on page 1, native text crisp, undo
clean, cancel works.

## Phase 5 — Excel batch flow

**Scope.**

- Excel integration: `NAMES` + `apply_excel_cover` accept AI preset names; palette column
  optional (falls back to palette 0).
- **Pre-flight** on batch start: N × ~220 Neurons estimate vs 10,000/day; if N > ~45,
  confirm dialog: "45 of 50 fit today's free quota; the rest will fail after ~45 — continue?"
- Batch: `CoverJob::spawn(items)` — one item per publication, seed derived from
  `hash(title)` (cohesive series, varied backgrounds); worker does HTTP only; UI applies each
  cover as it arrives; progress "cover 12/50" on both card statuses (ai.rs:368 pattern);
  cancel keeps completed covers.
- **Stop-on-429:** remaining items marked skipped; final report lists publications without
  covers ("3 missing: KCDAs Rotasi 2024, …"). No silent procedural fallback, no crash.
- Re-run only missing: batch skips publications whose cover page already exists with the
  matching AI preset tag (frame label), so "generate the rest tomorrow" is one click.

**Tests:** fake client sequence (item 2 fails) → partial kept + missing reported; cancel
mid-batch → document consistent; Excel name matching for AI styles.

**Acceptance:** `plan/pubgen/samples` workbook with AI style column generates N covers;
cancel and 429 paths leave a valid, undoable document.

## Phase 6 — Docs, guides, release wiring

**Scope.**

- `docs/pubgen-guide.md` + `docs/pubgen-guide_id.md`: Card 1 "AI Base" section — presets,
  palettes, Cloudflare setup (account id + token, token scope Workers AI Edit only),
  quota expectations, batch behavior, privacy note (cover prompts leave the machine).
- `ROADMAP.md` progress log; `docs/control-protocol.md` note that AI covers apply through the
  existing `template.page.apply`;
  `AGENT_MCP_GUIDE.md` example.
- Build wiring: nothing new required (feature flag composes:
  `--features pubgen,ai-cover-online`); confirm portable-release script still builds without
  the flag; `cargo xtask pubgen` + `cargo xtask ci` + wasm check green.
- Verify neuron-per-image estimate against current Cloudflare docs; adjust the pre-flight
  constant if drifted.

**Acceptance:** both guides updated; full `cargo xtask ci` green; feature-flag-off build
unchanged.

---

## 4. Out of scope (v2+)

- img2img (user collage as `image_b64` input), wraparound spread preset, seed lock UI,
  local Neuron usage tracker persisted across sessions, other providers (ComfyUI local),
  per-row palette column with validation UI.

## 5. Risks

| Risk | Mitigation |
|---|---|
| Neuron-cost estimates drift | Verify in Phase 6; constants in one place (`imagegen` crate) |
| Cloudflare response format differs from brainstorming notes (binary vs JSON) | Phase 1 contract isolated behind `CloudflareClient`; canned-listener tests make adaptation one file |
| Batch freezes UI | Worker thread + poll pattern already proven by `TranslateJob` |
| Quota death mid-batch | Pre-flight estimate + stop-on-429 + missing-report + resume |
| Key leakage | Token in prefs (local), password field, scoped token instructions in guide; never compiled in |
