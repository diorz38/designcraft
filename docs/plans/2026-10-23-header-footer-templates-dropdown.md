# Header & Footer Templates: Predefined Styles, Dropdown & Custom Template Persistence — Implementation Plan

DesignCraft currently provides the engine commands and dialog for basic Header/Footer presets (Minimal, Book, Editorial). This plan upgrades the system to support rich **Header & Footer Templates** bundling **headers, footers, decorative objects, and colors** into unified templates, featuring:
1. **3 Built-in Predefined Templates**:
   - **Modern Minimal**: Clean single-line header with hairline rule, section title, and page folio.
   - **Editorial Book**: Literary spread layout with running headers, chapter numbers, inner/outer margin alignment, and elegant book rules.
   - **Corporate Report**: Institutional publication styling featuring a colored top banner bar, pipe delimiters, and agency branding footer.
2. **Dedicated Pubgen Panel Section ("Header & Footer Template")**:
   - Placed directly between "Publication Cover" and "Quick Actions" in `pubgen.rs`.
   - Style dropdown selector (matching the "Publication Cover" UX pattern).
   - Color palette controls (Primary, Accent, Dark/Text).
   - Decorative object switches (Top accent bar, Header rule, Footer rule).
   - Editable header and footer token templates.
   - Action buttons: `Apply to Document`, `Save as New Template…`, and `Options…`.
3. **Template Customization & Persistence**:
   - Ability to modify any template's text, objects, and colors.
   - Save modified configurations as new named templates stored in persistent engine preferences (`s.prefs.header_footer_templates`).
   - Saved templates automatically appear in the Style dropdown across sessions.
4. **Dialog Integration**:
   - Sync the `Layout ▸ Header/Footer…` dialog with the template registry and allow saving custom templates directly from the dialog.

---

## Phase 1: Data Model (`crates/doc`)
**Goal:** Define `HeaderFooterTemplate`, `HeaderFooterObjects`, and `HeaderFooterColors` data structures with default factory functions and serde support.

- **Files:**
  - Modify: `crates/doc/src/header_footer.rs`
  - Modify: `crates/doc/src/lib.rs`
  - Test: `crates/doc/src/header_footer.rs`

- **Acceptance Criteria:**
  - [ ] `HeaderFooterTemplate` encapsulates `name`, `description`, `sided`, `header_offset`, `footer_offset`, `header`, `footer`, `style`, `objects`, and `colors`.
  - [ ] `HeaderFooterObjects` supports `header_rule`, `header_rule_weight`, `footer_rule`, `footer_rule_weight`, `top_accent_bar`, `top_accent_bar_height`, and `top_accent_bar_bleed`.
  - [ ] `HeaderFooterColors` supports `primary`, `accent`, `dark`, `rule_color`, and `text_color`.
  - [ ] `HeaderFooterTemplate::built_in_templates()` returns the 3 predefined templates: `"Modern Minimal"`, `"Editorial Book"`, and `"Corporate Report"`.
  - [ ] Serde roundtrip tests verify clean JSON serialization and deserialization.

---

## Phase 2: Engine Preference Persistence & Template Commands (`crates/engine`)
**Goal:** Add template storage to `s.prefs` and implement engine commands for listing, saving, deleting, and applying full templates.

- **Files:**
  - Modify: `crates/engine/src/lib.rs` (`Prefs`)
  - Modify: `crates/engine/src/cmd/header_footer.rs`
  - Test: `crates/engine/src/cmd/header_footer.rs`

- **Acceptance Criteria:**
  - [ ] `s.prefs.header_footer_templates` persists custom user templates across app launches.
  - [ ] `layout.headerFooter.listTemplates` returns all 3 built-in templates combined with any user-saved templates.
  - [ ] `layout.headerFooter.saveTemplate` saves/updates custom templates by name (validating name length and capping at maximum presets).
  - [ ] `layout.headerFooter.deleteTemplate` removes custom templates (protecting built-in defaults from deletion).
  - [ ] `layout.headerFooter.apply` accepts full template parameters including `objects` (creating top accent colored bars, header rules, footer rules) and `colors` (registering swatches and styling frames).

---

## Phase 3: Pubgen Panel "Header & Footer Template" Section (`crates/ui-egui`)
**Goal:** Add a dedicated card in `pubgen.rs` with Style dropdown, color palette controls, decorative object toggles, and template saving.

- **Files:**
  - Modify: `crates/ui-egui/src/panels/pubgen.rs`
  - Test: `crates/ui-egui/src/panels/pubgen.rs`

- **Acceptance Criteria:**
  - [ ] Dedicated section header `"Header & Footer Template"` rendered between Publication Cover and Quick Actions.
  - [ ] Style dropdown displaying built-in and user-saved templates with current template selection.
  - [ ] Selecting a style populates live fields (header tokens, footer tokens, colors, objects, offsets).
  - [ ] Color editors for Primary, Accent, and Dark/Text swatches.
  - [ ] Toggles for Top Accent Bar, Header Rule, and Footer Rule.
  - [ ] `Apply to Document` executes `layout.headerFooter.apply` with all template settings.
  - [ ] `Save as New Template…` opens an inline prompt to name and persist the modified template.

---

## Phase 4: Header/Footer Dialog Integration (`crates/ui-egui`)
**Goal:** Upgrade `Layout ▸ Header/Footer…` dialog to load from template registry and support saving custom templates.

- **Files:**
  - Modify: `crates/ui-egui/src/header_footer.rs`
  - Modify: `crates/ui-egui/src/dialogs.rs`
  - Test: `crates/ui-egui/src/dialogs.rs`

- **Acceptance Criteria:**
  - [ ] Preset dropdown queries `layout.headerFooter.listTemplates` and displays all available templates.
  - [ ] Selecting a template updates dialog fields and preview.
  - [ ] "Save as Template…" action allows saving current dialog configuration into the template registry.

---

## Phase 5: Localization & Internationalization (`crates/ui-egui`)
**Goal:** Register all new UI strings in the `i18n` translation catalog.

- **Files:**
  - Modify: `crates/ui-egui/src/i18n.rs`
  - Test: `crates/ui-egui/src/i18n.rs`

- **Acceptance Criteria:**
  - [ ] All new labels (`"Header & Footer Template"`, `"Modern Minimal"`, `"Editorial Book"`, `"Corporate Report"`, `"Top Accent Bar"`, `"Save as New Template…"`, `"Template Name"`, etc.) registered in `TABLE` and `interface_strings()`.
  - [ ] All 17 `i18n` tests pass without collision or untracked strings.

---

## Phase 6: End-to-End Verification & Quality Gates
**Goal:** Verify all quality gates pass without warnings, test rendering visually, and update docs.

- **Files:**
  - Modify: `ROADMAP.md`
  - Modify: `docs/plans/2026-10-23-header-footer-templates-dropdown.md`

- **Acceptance Criteria:**
  - [ ] `cargo xtask layers` confirms 0 layering violations.
  - [ ] `cargo xtask assets` passes attribution check.
  - [ ] `cargo fmt --check` passes cleanly.
  - [ ] `cargo clippy --workspace --all-targets -- -D warnings` reports 0 errors.
  - [ ] `cargo xtask wasm` compiles for WebAssembly cleanly.
  - [ ] Document PNG export confirms rendering of top accent bar, header/footer text, and rules.
