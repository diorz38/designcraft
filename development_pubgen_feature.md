# Pubgen Feature: Upstream Synchronization & Maintenance Workflow

This document details the architecture, branching model, and step-by-step workflow for keeping the **Pubgen** feature (`feature/pubgen`) synchronized with changes from the original upstream repository (`storytold/designcraft`).

---

## 1. Architectural Seam & Zero-Footprint Isolation

To ensure that DesignCraft's core layout engine and UI remain clean and uncorrupted, Pubgen is architected using a non-invasive feature seam:

1. **Upstream Baseline (`crates/ui-egui/src/dock.rs`)**:
   - Remains **100% untouched** and identical to the original repository.
   - Any upstream changes to panels, dock layouts, or floating windows land directly in `dock.rs` without merge conflicts.

2. **Extended Dock (`crates/ui-egui/src/dock_extended.rs`)**:
   - Active when compiled with the `pubgen` Cargo feature.
   - Incorporates exactly **4 Pubgen injection hooks** into the upstream dock structure:
     - **Hook 1 (Module declaration)**:
       ```rust
       #[path = "panels/pubgen.rs"]
       pub mod pubgen;
       ```
     - **Hook 2 (Tab entry in `DOCK_TABS`)**:
       ```rust
       pub const DOCK_TABS: &[(&str, &str, &str)] = &[
           ("properties", "Properties", "panel-properties"),
           ("pages", "Pages", "panel-pages"),
           ("layers", "Layers", "panel-layers"),
           ("pubgen", "Pubgen", "panel-pages"),
       ];
       ```
     - **Hook 3 (Expanded panel renderer in `dock_expanded`)**:
       ```rust
       "pubgen" => pubgen::show(app, ui),
       ```
     - **Hook 4 (Flyout panel renderer in `panel_body`)**:
       ```rust
       "pubgen" => pubgen::show(app, ui),
       ```

3. **Drift Enforcement Tool (`cargo xtask diff-dock`)**:
   - An automated quality gate implemented in `xtask/src/diff_dock.rs`.
   - Strips the known Pubgen hooks and verifies that `dock_extended.rs` has not drifted from `dock.rs`.

---

## 2. Git Branching Strategy

| Branch | Purpose | Remote Tracking |
| :--- | :--- | :--- |
| `main` | Clean upstream mirror | `upstream/main` / `origin/main` |
| `feature/pubgen` | Active development branch containing Pubgen panels, documentation, tests, and samples | `origin/feature/pubgen` |

> **Rule:** Never develop feature code directly on `main`. Always keep `main` tracking upstream commits cleanly.

---

## 3. Step-by-Step Upstream Sync Workflow

Follow this procedure whenever upstream releases new commits or updates:

### Step 1: Ensure Working Tree is Clean
Make sure all your current work on `feature/pubgen` is saved and committed:
```bash
git status
# If there are changes:
git add .
git commit -m "feat(pubgen): description of local work"
```

### Step 2: Fetch and Sync `main`
Pull the latest commits from the upstream repository into local `main`:
```bash
git checkout main
git fetch upstream main
git merge upstream/main --ff-only
git push origin main
```

### Step 3: Merge `main` into `feature/pubgen`
Switch back to your feature branch and merge the updated `main`:
```bash
git checkout feature/pubgen
git merge main
```

### Step 4: Resolve Merge Conflicts (if any)
Typically, only configuration files with feature declarations (such as `apps/designcraft/Cargo.toml` or `crates/ui-egui/Cargo.toml`) might have conflicts.
- Retain new upstream dependencies.
- Retain the `pubgen = [...]` feature flags.
- Stage resolved files:
  ```bash
  git add apps/designcraft/Cargo.toml
  ```

### Step 5: Check Dock Drift
Run the automated dock verification tool:
```bash
cargo xtask diff-dock
```
- If `dock.rs` received upstream edits, `diff-dock` will report the drifting lines.

### Step 6: Reconcile `dock_extended.rs`
When upstream modifies `crates/ui-egui/src/dock.rs`:
1. Copy `crates/ui-egui/src/dock.rs` into `crates/ui-egui/src/dock_extended.rs`.
2. Re-apply the 4 Pubgen hooks:
   - Add `#[path = "panels/pubgen.rs"] pub mod pubgen;` at the top imports.
   - Add `("pubgen", "Pubgen", "panel-pages")` to `DOCK_TABS`.
   - Add `"pubgen" => pubgen::show(app, ui),` in `dock_expanded` ScrollArea match.
   - Add `"pubgen" => pubgen::show(app, ui),` in `panel_body` match.
3. Verify that `diff-dock` passes:
   ```bash
   cargo xtask diff-dock
   ```
   **Expected output:**
   ```
   ✓ dock.rs and dock_extended.rs are in sync (Pubgen hooks intact).
   ```

### Step 7: Reconcile Engine/Document Model Changes (if any)
If upstream modified engine data structures or document types used by Pubgen (e.g. `CellStroke`, `Table`, `Session` methods):
- Update `crates/ui-egui/src/panels/pubgen.rs` to match the new definitions (e.g., utilizing `..Default::default()` for struct initializers).

### Step 8: Run Validation & Test Suite
Ensure that all unit tests and quality gates pass:
```bash
# 1. Verify dock sync
cargo xtask diff-dock

# 2. Run Pubgen unit tests
cargo test -p designcraft-ui-egui --features pubgen -- pubgen
```
All tests should pass:
```
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 545 filtered out
```

### Step 9: Launch Application for Visual Verification
Run the application locally with the Pubgen feature enabled to test the UI:
```bash
cargo run -p designcraft --features pubgen
```
- Switch to the **Pubgen** tab in the right dock.
- Test generating pages from `plan/pubgen/samples/sample_chapters.csv`.
- Test inserting tables with multi-stack headers from `plan/pubgen/samples/sample_3stack_table.csv`.

### Step 10: Commit and Push
```bash
git add crates/ui-egui/src/dock_extended.rs crates/ui-egui/src/panels/pubgen.rs
git commit -m "fix(pubgen): reconcile dock_extended and pubgen panel with upstream changes"
git push origin feature/pubgen
```

---

## 4. Quick Reference Checklist

| Action | Command |
| :--- | :--- |
| **Check dock drift** | `cargo xtask diff-dock` |
| **Run Pubgen tests** | `cargo test -p designcraft-ui-egui --features pubgen -- pubgen` |
| **Run app with Pubgen** | `cargo run -p designcraft --features pubgen` |
| **Verify branch sync** | `git log -n 5 --oneline --graph` |
