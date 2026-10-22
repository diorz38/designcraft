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
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 545 filtered out
```

### Step 9: Launch Application for Visual Verification
Run the application locally with the Pubgen feature enabled to test the UI:
```bash
cargo run -p designcraft --features pubgen
```
or
```bash
cargo run --release -p designcraft --features pubgen
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

## 4. Generating Portable Windows x64 Demo (`.exe` & `.zip`)

This workflow packages a self-contained, portable Windows x64 demonstration build that can be run from any folder, USB drive, or virtual machine without installation or administrator privileges.

### Why This Build is Portable
- **Static C Runtime (`+crt-static`)**: Binaries link the Microsoft C Runtime statically. Users do not need the Visual C++ Redistributable installed.
- **Local Isolation Marker (`portable.txt`)**: When `portable.txt` sits next to `designcraft.exe`, DesignCraft automatically creates and keeps all configuration, logs, and crash recovery in `.\DesignCraftData\` rather than writing to `%APPDATA%\DesignCraft`.
- **Pure GUI Subsystem**: `designcraft.exe` is linked with PE subsystem 2 (Windows GUI), so no console window flashes or stays open when launched.
- **Embedded Icon & Version Metadata**: Icons and version information are compiled directly into the binary.

---

### Step-by-Step Packaging Procedure

Open PowerShell in the repository root (`f:\GitHub\designcraft`):

#### Step 1: Compile Release Binaries with Static CRT & Embedded Icon
Ensure the Windows Resource Compiler (`rc.exe`) is on `$env:PATH`, require resource embedding, and set target rustflags to statically link the CRT, then compile in optimized `--release` mode with the `pubgen` feature:
```powershell
# Ensure rc.exe (Windows SDK) is in PATH so the application icon is embedded
if (-not (Get-Command rc.exe -ErrorAction SilentlyContinue)) {
    $rc = Get-ChildItem -Path "C:\Program Files (x86)\Windows Kits", "C:\Program Files\Microsoft Visual Studio" -Filter "rc.exe" -Recurse -ErrorAction SilentlyContinue | Where-Object { $_.FullName -like "*\x64\rc.exe" } | Select-Object -First 1
    if ($rc) { $env:PATH = "$($rc.DirectoryName);$env:PATH" }
}
$env:DESIGNCRAFT_REQUIRE_WINRES = "1"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release -p designcraft -p designcraft-cli --features designcraft/pubgen --target x86_64-pc-windows-msvc
```

#### Step 2: Prepare the Portable Directory Structure
```powershell
$DistDir = "dist\designcraft-portable-x64"
if (Test-Path $DistDir) { Remove-Item -Recurse -Force $DistDir }
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $DistDir "samples\csv") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $DistDir "samples\xlsx") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $DistDir "samples\images") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $DistDir "samples\style") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $DistDir "samples\thumb") | Out-Null
```

#### Step 3: Copy Binaries and Assets
```powershell
# Copy GUI and CLI executables
Copy-Item "target\x86_64-pc-windows-msvc\release\designcraft.exe" $DistDir
Copy-Item "target\x86_64-pc-windows-msvc\release\designcraft-cli.exe" $DistDir

# Copy sample CSV and XLSX datasets into dedicated subfolders (no files in root samples folder)
Copy-Item "plan\pubgen\samples\*.csv" (Join-Path $DistDir "samples\csv")
Copy-Item "plan\pubgen\samples\BAB_02_2025.xlsx" (Join-Path $DistDir "samples\xlsx")
if (Test-Path "plan\pubgen\samples\images") { Copy-Item -Recurse "plan\pubgen\samples\images\*" (Join-Path $DistDir "samples\images") }
if (Test-Path "plan\pubgen\samples\style") { Copy-Item -Recurse "plan\pubgen\samples\style\*" (Join-Path $DistDir "samples\style") }
if (Test-Path "plan\pubgen\samples\thumb") { Copy-Item -Recurse "plan\pubgen\samples\thumb\*" (Join-Path $DistDir "samples\thumb") }
```

#### Step 4: Create the `portable.txt` Marker
```powershell
Set-Content -Encoding ascii -Path (Join-Path $DistDir "portable.txt") -Value @"
DesignCraft portable: preferences, logs and crash recovery are kept in the DesignCraftData folder
next to designcraft.exe. Delete this file to use %APPDATA%\DesignCraft instead.
"@
```

#### Step 5: Add a User Guide (`README-PORTABLE.txt`)
```powershell
Set-Content -Encoding utf8 -Path (Join-Path $DistDir "README-PORTABLE.txt") -Value @"
========================================================================
DesignCraft - Standalone Portable Demo (Windows x64)
========================================================================

HOW TO RUN:
Double-click designcraft.exe to start the app.

TESTING PUBGEN:
1. File > New Document (Ctrl+N) > Create.
2. Open the 'Pubgen' tab in the right dock panel.
3. Test 'Page from CSV' with samples\csv\KBDA2026_chapters.csv.
4. Test 'Table from CSV' with samples\csv\kbda_2005.csv (Style: Horizontal Rules).
5. Test 'Excel to CSV' with samples\xlsx\BAB_02_2025.xlsx.
6. Test 'Quick Actions' to switch Double-sided / Single-sided or add blank pages.
========================================================================
"@
```

#### Step 6: Create the Portable ZIP Archive
```powershell
$ZipPath = "dist\designcraft-portable-x64.zip"
if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
Compress-Archive -Path "$DistDir\*" -DestinationPath $ZipPath
```

---

### Automated One-Shot Script

To run all packaging steps in a single automated command:
```powershell
powershell -Command "
`$ErrorActionPreference = 'Stop'
`$DistDir = 'dist\designcraft-portable-x64'
`$ZipPath = 'dist\designcraft-portable-x64.zip'

Write-Host '==> 1. Building release binaries (static CRT & embedded icon)...'
if (-not (Get-Command rc.exe -ErrorAction SilentlyContinue)) {
    `$rc = Get-ChildItem -Path 'C:\Program Files (x86)\Windows Kits', 'C:\Program Files\Microsoft Visual Studio' -Filter 'rc.exe' -Recurse -ErrorAction SilentlyContinue | Where-Object { `$_.FullName -like '*\x64\rc.exe' } | Select-Object -First 1
    if (`$rc) { `$env:PATH = "`$(`$rc.DirectoryName);`$env:PATH" }
}
`$env:DESIGNCRAFT_REQUIRE_WINRES = '1'
`$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release -p designcraft -p designcraft-cli --features designcraft/pubgen --target x86_64-pc-windows-msvc

Write-Host '==> 2. Assembling portable folder...'
Remove-Item -Recurse -Force `$DistDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path (Join-Path `$DistDir 'samples\csv') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path `$DistDir 'samples\xlsx') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path `$DistDir 'samples\images') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path `$DistDir 'samples\style') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path `$DistDir 'samples\thumb') | Out-Null

Copy-Item 'target\x86_64-pc-windows-msvc\release\designcraft.exe' `$DistDir
Copy-Item 'target\x86_64-pc-windows-msvc\release\designcraft-cli.exe' `$DistDir

Copy-Item 'plan\pubgen\samples\*.csv' (Join-Path `$DistDir 'samples\csv')
Copy-Item 'plan\pubgen\samples\BAB_02_2025.xlsx' (Join-Path `$DistDir 'samples\xlsx')
if (Test-Path 'plan\pubgen\samples\images') { Copy-Item -Recurse 'plan\pubgen\samples\images\*' (Join-Path `$DistDir 'samples\images') }
if (Test-Path 'plan\pubgen\samples\style') { Copy-Item -Recurse 'plan\pubgen\samples\style\*' (Join-Path `$DistDir 'samples\style') }
if (Test-Path 'plan\pubgen\samples\thumb') { Copy-Item -Recurse 'plan\pubgen\samples\thumb\*' (Join-Path `$DistDir 'samples\thumb') }

Set-Content -Encoding ascii -Path (Join-Path `$DistDir 'portable.txt') -Value 'DesignCraft portable: preferences, logs and crash recovery are kept in the DesignCraftData folder next to designcraft.exe.'

Set-Content -Encoding utf8 -Path (Join-Path `$DistDir 'README-PORTABLE.txt') -Value @'
========================================================================
DesignCraft - Standalone Portable Demo (Windows x64)
========================================================================

HOW TO RUN:
Double-click designcraft.exe to start the app.

TESTING PUBGEN:
1. File > New Document (Ctrl+N) > Create.
2. Open the 'Pubgen' tab in the right dock panel.
3. Test 'Page from CSV' with samples\csv\KBDA2026_chapters.csv.
4. Test 'Table from CSV' with samples\csv\kbda_2005.csv (Style: Horizontal Rules).
5. Test 'Excel to CSV' with samples\xlsx\BAB_02_2025.xlsx.
6. Test 'Quick Actions' to switch Double-sided / Single-sided or add blank pages.
========================================================================
'@

Write-Host '==> 3. Creating portable zip...'
Remove-Item -Force `$ZipPath -ErrorAction SilentlyContinue
Compress-Archive -Path `"`$DistDir\*`" -DestinationPath `$ZipPath

Write-Host '==> Finished packaging:'
Get-Item (Join-Path `$DistDir 'designcraft.exe'), `$ZipPath | Format-Table Name, Length, LastWriteTime
"
```

---

## 5. Quick Reference Checklist

| Action | Command |
| :--- | :--- |
| **Check dock drift** | `cargo xtask diff-dock` |
| **Run Pubgen tests** | `cargo test -p designcraft-ui-egui --features pubgen -- pubgen` |
| **Run app with Pubgen (Debug)** | `cargo run -p designcraft --features pubgen` |
| **Build Portable Release (x64)** | `$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = "-C target-feature=+crt-static"; cargo build --release -p designcraft -p designcraft-cli --features designcraft/pubgen --target x86_64-pc-windows-msvc` |
| **Verify branch sync** | `git log -n 5 --oneline --graph` |

---

## 6. Room for Improvement (What's Left) & Next Steps

While Pubgen provides high-value automated layout and advanced tabular workflows out of the box, several architectural and functional opportunities remain for future development:

### Main Weaknesses & Architectural Debt

1. **Generation Logic in UI Layer (Violates "Everything is a Command"):**
   - **Current State:** Batch page loops (`execute_generate_pages`) and table synthesis (`execute_insert_table`) reside directly within `crates/ui-egui/src/panels/pubgen.rs`.
   - **Impact:** Headless environments (`designcraft-cli`), scripts (`.dcs`), and MCP automation agents cannot execute Pubgen workflows without launching the GUI.
   - **Next Step:** Push core generator pipelines down into engine commands (`crates/engine/src/cmd/pubgen.rs` or `data.rs`), leaving `pubgen.rs` as a thin UI coordinator calling `app.run("pubgen.pages", ...)` and `app.run("pubgen.table", ...)`.

2. **Non-Atomic Batch Undo:**
   - **Current State:** Generating 50 catalog pages executes individual engine commands sequentially, resulting in 50+ entries in the undo stack.
   - **Impact:** Undoing a generation batch requires repeated `Ctrl+Z` operations rather than a single revert.
   - **Next Step:** Implement an engine-level command or atomic transaction grouping to guarantee a 1-step undo revert.

3. **Duplication of `data.merge` Infrastructure:**
   - **Current State:** Card 1 implements its own CSV parsing and record iteration routines.
   - **Impact:** Duplicates existing engine-level CSV ingestion and record-range mapping already maintained in `crates/engine/src/cmd/datamerge/`.
   - **Next Step:** Refactor Card 1 to act as an ergonomic preset-driven front-end over `data.source` and `data.merge`.

4. **Hardcoded Geometric Layout Presets:**
   - **Current State:** The 6 layout recipes (1 Row · 3 Columns, Editorial Chapter, TOC Line, etc.) are compiled Rust code with static arithmetic offsets.
   - **Impact:** Users cannot customize column widths, margins, or typography without editing Rust source code and recompiling.
   - **Next Step:** Externalize layout recipes into data-driven configuration templates (JSON/TOML) located in `assets/templates/` or user preferences.

5. **CI Feature Flag Testing:**
   - **Current State:** `cargo xtask ci` validates the default workspace features only.
   - **Impact:** Code rot or upstream signature changes can silently break the `pubgen` compilation target unless developers remember to run `--features pubgen` manually.
   - **Next Step:** Add `cargo check -p designcraft-ui-egui --features pubgen` and `cargo xtask diff-dock` directly to the `xtask ci` pipeline.

