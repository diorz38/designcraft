**Yes — doable.** The plan's assumptions is verified against the codebase; most hold, with four caveats worth fixing before execution.

**Verified accurate:**
- Seam (Phase 1): `dock.rs` matches the PRD exactly — `DOCK_TABS` (dock.rs:10), the scroll-area dispatcher (dock.rs:109), `pub mod dock;` (lib.rs:16). The `#[cfg]`/`#[path]` trick plus a `pubgen = []` feature (none exists yet in ui-egui's Cargo.toml) is standard and works.
- UI parity (Phase 2): all claimed helpers exist — `selection_band`, `section`, `rule`, `divider`, `dropdown`, `outline_button`, `FIELD_H = 21.0` — in `widgets.rs` (not `properties.rs`, same effect).
- Engines (Phase 3/4): `layout.pages.insert`, `frame.create`, `text.insert`, `table.insert` (with `headerRows`/`width`), `style.table.create`, and `margin_rect()` (doc/src/page.rs:240) all exist.
- xtask (Phase 5): main.rs is a simple match dispatcher; adding `diff-dock` is trivial. Drop the "AST comparison" wording — line-diff after stripping the known hooks is what's actually specified and sufficient.

**Caveats:**
1. **Card 1 duplicates existing Data Merge.** The engine already has `data.merge` / `data.source.*` / `data.fields` (crates/engine/src/cmd/datamerge/) accepting inline CSV with record-range/limit selection and generating pages per record. Build Card 1 on that instead of re-implementing CSV ingest and page loops in the UI — the PRD doesn't mention this overlap at all.
2. **Task 3.4 (single-step undo) is not off-the-shelf.** Undo is one `HistoryEntry` per command execution; there is no public transaction/batch API to coalesce a UI-side loop of `execute()` calls. Cleanest fix, which also matches the repo's "everything is a command" rule: implement generation as one engine command (like `data.merge` does) so it's one undo step by construction.
3. **Native file picker can't live in ui-egui**: `rfd` is banned below the app crates (layering rule); route through the same mechanism datamerge's `select_source` uses.
4. **The "only two insertion points" claim misses a third**: `panel_body` (dock.rs:247) dispatches flyout/floating panel bodies — without it, a floated Pubgen panel falls through to `info_panel`. Also add an i18n entry for the "Pubgen" label, and note `cargo xtask ci` only checks default features, so the `pubgen` feature path needs an explicit CI check or it will rot silently.