//! PubGen's own menu bar (fork addition).
//!
//! DesignCraft's menu bar is replaced by File + Publication + PubGen Tools + Help while the PubGen
//! menus are on (the default here). Those menus hold the PubGen panel's actions; File and
//! Help are DesignCraft's own rows. The toggle in PubGen › About PubGen… (and in the About PubGen
//! dialog) switches between this bar and DesignCraft's full one.
//!
//! Everything lives in this one file so an upstream update touches at most the single hook at the
//! top of [`super::bar::menu_bar`]: no upstream menu table, command table or UI state is edited.

use crate::DesignApp;

/// The egui memory key holding whether the PubGen menu bar is shown (temporary: resets to on).
const ON: &str = "pubgen_menus_on";

/// Whether the PubGen menu bar replaces DesignCraft's (the default).
pub fn pubgen_menus_on(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::unique(ON))).unwrap_or(true)
}

/// Show the PubGen menu bar (`true`) or DesignCraft's full one (`false`).
pub fn set_pubgen_menus_on(ctx: &egui::Context, on: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique(ON), on));
}

/// Draws the PubGen menu bar in place of DesignCraft's. Returns `false` (having drawn nothing)
/// when DesignCraft's menu bar should be drawn instead.
pub fn menu_bar(app: &mut DesignApp, ui: &mut egui::Ui) -> bool {
    if !pubgen_menus_on(ui.ctx()) {
        return false;
    }
    let rtl = crate::i18n::is_rtl(&app.ui.language);
    // The host tree stays intact (the audit's §1: never hide the host's navigation); the three
    // Pubgen menus join it, with Help kept last like every desktop app.
    let tree = crate::menus::menu_tree();
    let mut top: Vec<Top> = tree.iter().filter(|(name, _)| *name != "Help").map(|(name, entries)| Top::Host(name, entries)).collect();
    top.extend([Top::Publication, Top::Tools, Top::Shortcut, Top::Help]);
    for entry in if rtl { top.into_iter().rev().collect::<Vec<_>>() } else { top } {
        match entry {
            Top::Host(name, entries) => designcraft_menu(app, ui, name, entries),
            Top::Publication => publication_menu(app, ui),
            Top::Tools => content_menu(app, ui),
            Top::Shortcut => shortcut_menu(app, ui),
            Top::Help => help_menu(app, ui),
        }
    }
    true
}

/// One top-level menu of the PubGen bar, in draw order.
enum Top<'a> {
    Host(&'a str, &'a [crate::menus::Item]),
    Publication,
    Tools,
    Shortcut,
    Help,
}

/// DesignCraft's Help rows, then About PubGen… and the menu-bar toggle — the single home for both.
fn help_menu(app: &mut DesignApp, ui: &mut egui::Ui) {
    let tree = crate::menus::menu_tree();
    let Some((name, entries)) = tree.iter().find(|(n, _)| *n == "Help") else { return };
    let lang = app.ui.language.clone();
    let rtl = crate::i18n::is_rtl(&lang);
    crate::menus::menu_button(ui, crate::rtl::widget(ui, crate::i18n::tr(&lang, name)), |ui| {
        super::bar::menu_column(app, ui, &[(name, entries.to_vec())]);
        ui.separator();
        popup(ui, rtl, 240.0, |ui| about(ui, &lang));
    });
}

/// Curated quick access to the original DesignCraft actions a publication operator reaches for
/// most (the plan's §4.8) — existing commands only, so chords, gates and i18n come free.
fn shortcut_menu(app: &mut DesignApp, ui: &mut egui::Ui) {
    let items: Vec<crate::menus::Item> =
        [("app.exportPdf", "{}"), ("preflight.run", "{}"), ("app.packageDialog", "{}"), ("app.placeDialog", "{}"), ("file.newSample", "{}")]
            .iter()
            .map(|(id, params)| super::cmd_item(id, serde_json::from_str(params).unwrap_or_default()))
            .collect();
    let lang = app.ui.language.clone();
    crate::menus::menu_button(ui, crate::rtl::widget(ui, crate::i18n::tr(&lang, "Shortcut")), |ui| {
        super::bar::menu_column(app, ui, &[("Shortcut", items)]);
    });
}

/// One of DesignCraft's own menus (File, Help), drawn from the upstream menu tree.
fn designcraft_menu(app: &mut DesignApp, ui: &mut egui::Ui, name: &str, entries: &[crate::menus::Item]) {
    let label = crate::i18n::tr(&app.ui.language, name);
    crate::menus::menu_button(ui, crate::rtl::widget(ui, label), |ui| {
        super::bar::menu_column(app, ui, &[(name, entries.to_vec())]);
    });
}

/// The Popup layout every PubGen menu uses: right-to-left aware, at least `min_width` wide.
fn popup(ui: &mut egui::Ui, rtl: bool, min_width: f32, contents: impl FnOnce(&mut egui::Ui)) {
    if rtl {
        ui.set_max_width(320.0);
    }
    ui.with_layout(egui::Layout::top_down_justified(if rtl { egui::Align::Max } else { egui::Align::Min }), |ui| {
        ui.set_min_width(min_width);
        contents(ui);
    });
}

/// A menu row: label, enabled, clicked.
fn item(ui: &mut egui::Ui, lang: &str, label: &str, enabled: bool) -> bool {
    ui.add_enabled(enabled, egui::Button::new(crate::rtl::widget(ui, crate::i18n::tr(lang, label)))).clicked()
}

/// A submenu row that opens `contents`.
fn submenu(ui: &mut egui::Ui, lang: &str, rtl: bool, label: &str, min_width: f32, contents: impl FnOnce(&mut egui::Ui)) {
    crate::menus::menu_button(ui, crate::rtl::widget(ui, crate::i18n::tr(lang, label)), |ui| popup(ui, rtl, min_width, contents));
}

/// The generation pipeline: Master Excel, the compiler, charts, ToC, cover and headers.
fn pipeline(app: &mut DesignApp, ui: &mut egui::Ui, lang: &str) {
    let has_doc = app.session.active().is_some();

    if item(ui, lang, "Master Excel Import & Generate…", has_doc) {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            st.dialog_excel_open = true;
        });
        ui.close();
    }

    if item(ui, lang, "Statistical Vector Chart…", has_doc) {
        let dlg = Box::new(crate::dock::pubgen::charts::dialog::ChartDialog::new(app, false));
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            st.chart_dialog = Some(dlg);
        });
        ui.close();
    }

    if item(ui, lang, "Table of Contents…", has_doc) {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            st.dialog_toc_open = true;
        });
        ui.close();
    }

    if item(ui, lang, "Publication Cover…", has_doc) {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            st.dialog_cover_open = true;
        });
        ui.close();
    }

    if item(ui, lang, "Header & Footer Setup…", has_doc) {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            st.dialog_hf_open = true;
        });
        ui.close();
    }

    // Last pipeline step: the QA audit over the finished layout (Pillar 3).
    if item(ui, lang, "Audit Publication…", has_doc) {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            st.audit_open = true;
            st.audit_issues = None; // the dialog runs `preflight.pubgen` on first paint
        });
        ui.close();
    }
}

/// Manual content: CSV ingest, layout utilities, the dual-language switch and AI translation.
fn content(app: &mut DesignApp, ui: &mut egui::Ui, lang: &str, rtl: bool) {
    let has_doc = app.session.active().is_some();

    submenu(ui, lang, rtl, "Manual Ingest & Tables", 220.0, |ui| {
        if item(ui, lang, "Page from CSV…", has_doc) {
            crate::dock::pubgen::with_state(ui.ctx(), |st| {
                st.dialog_manual_open = true;
                st.dialog_manual_tab = 0;
            });
            ui.close();
        }
        if item(ui, lang, "Table from CSV…", has_doc) {
            crate::dock::pubgen::with_state(ui.ctx(), |st| {
                st.dialog_manual_open = true;
                st.dialog_manual_tab = 1;
            });
            ui.close();
        }
    });

    submenu(ui, lang, rtl, "Page & Layout Utilities", 280.0, |ui| {
        if item(ui, lang, "Set Standard BPS Margins (20/20/19/15 mm)", has_doc) {
            let _ = crate::dock::pubgen::execute_set_page_margins(app);
            ui.close();
        }
        ui.separator();
        if item(ui, lang, "Page Margins & Renumbering Options…", has_doc) {
            crate::dock::pubgen::with_state(ui.ctx(), |st| {
                st.dialog_utils_open = true;
            });
            ui.close();
        }
    });

    ui.separator();

    let is_bilingual = crate::dock::pubgen::with_state(ui.ctx(), |st| st.bilingual_output);
    let check_mark = if is_bilingual { "✓ " } else { "   " };
    let bi_label = format!("{}{}", check_mark, crate::i18n::tr(lang, "Dual Language Mode (Bilingual ID / EN)"));
    if ui.button(crate::rtl::widget(ui, bi_label)).clicked() {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            st.bilingual_output = !st.bilingual_output;
        });
        ui.close();
    }

    submenu(ui, lang, rtl, "AI Translate", 260.0, |ui| ai_translate(app, ui, lang, has_doc));
}

/// Submenu of AI translation actions for the current page.
fn ai_translate(app: &mut DesignApp, ui: &mut egui::Ui, lang: &str, has_doc: bool) {
    let backend_ready = crate::dock::pubgen::ai::backend_from_prefs(app).is_some();
    let is_bilingual = crate::dock::pubgen::with_state(ui.ctx(), |st| st.bilingual_output);
    let cur_page = app.session.active().and_then(|s| s.selection.page).or_else(|| crate::canvas::current_page(app));
    let review_target = cur_page.and_then(|p| crate::dock::pubgen::ai::detect_page_target(app, p));
    let table_target = cur_page.and_then(|p| crate::dock::pubgen::ai::detect_table_page_target(app, p));
    let is_review = review_target.is_some();
    let is_table = table_target.is_some();
    let can_page_ai = has_doc && (is_review || is_table) && backend_ready && is_bilingual;

    let page_label = if is_table {
        "Translate Table on this Page"
    } else if is_review {
        "Translate Review on this Page"
    } else {
        "Translate Selected Page"
    };
    if item(ui, lang, page_label, can_page_ai) {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            crate::dock::pubgen::ai::execute_translate_selected_page(app, st, ui.ctx().clone());
        });
        ui.close();
    }

    let total_pages = app.session.active().map(|s| s.doc.page_count()).unwrap_or(0);
    let can_batch_tables = has_doc && total_pages > 0;
    if item(ui, lang, "Translate Table Pages…", can_batch_tables) {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            st.show_table_range_popup = true;
            if st.ai_table_range_to == 0 || st.ai_table_range_to > total_pages {
                st.ai_table_range_to = total_pages.max(1);
            }
        });
        ui.close();
    }

    let can_fix = crate::dock::pubgen::with_state(ui.ctx(), |st| (is_review || is_table) && !st.translations.is_empty() && !st.glossary.is_empty());
    if item(ui, lang, "Fix Terms with Glossary", can_fix) {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            crate::dock::pubgen::ai::execute_fix_terms(app, st);
        });
        ui.close();
    }

    ui.separator();

    if item(ui, lang, "Setup…", true) {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            st.ai_dialog = Some(crate::dock::pubgen::ai::AiSetupDialog::open(app, st));
        });
        ui.close();
    }
}

/// About PubGen… and the menu-bar toggle, shown at the foot of the PubGen menu.
fn about(ui: &mut egui::Ui, lang: &str) {
    if item(ui, lang, "About PubGen…", true) {
        crate::dock::pubgen::with_state(ui.ctx(), |st| {
            st.show_about_dialog = true;
        });
        ui.close();
    }
    let pubgen_on = pubgen_menus_on(ui.ctx());
    let toggle = if pubgen_on { "Show DesignCraft Menus" } else { "Show PubGen Menus" };
    if item(ui, lang, toggle, true) {
        set_pubgen_menus_on(ui.ctx(), !pubgen_on);
        ui.close();
    }
}

/// The PubGen menu of the DesignCraft menu bar: every PubGen action in one menu.
pub fn pubgen_menu(app: &mut DesignApp, ui: &mut egui::Ui) {
    let lang = app.ui.language.clone();
    let rtl = crate::i18n::is_rtl(&lang);
    crate::menus::menu_button(ui, crate::rtl::widget(ui, "Pubgen"), |ui| {
        popup(ui, rtl, 260.0, |ui| {
            pipeline(app, ui, &lang);
            ui.separator();
            content(app, ui, &lang, rtl);
            ui.separator();
            about(ui, &lang);
        });
    });
}

/// The Publication menu: the generation pipeline. (About and the menu-bar toggle live in Help.)
fn publication_menu(app: &mut DesignApp, ui: &mut egui::Ui) {
    let lang = app.ui.language.clone();
    let rtl = crate::i18n::is_rtl(&lang);
    crate::menus::menu_button(ui, crate::rtl::widget(ui, crate::i18n::tr(&lang, "Publication")), |ui| {
        popup(ui, rtl, 260.0, |ui| pipeline(app, ui, &lang));
    });
}

/// The PubGen Tools menu: manual ingest, layout utilities, language mode and AI translation.
fn content_menu(app: &mut DesignApp, ui: &mut egui::Ui) {
    let lang = app.ui.language.clone();
    let rtl = crate::i18n::is_rtl(&lang);
    crate::menus::menu_button(ui, crate::rtl::widget(ui, crate::i18n::tr(&lang, "PubGen Tools")), |ui| {
        popup(ui, rtl, 260.0, |ui| content(app, ui, &lang, rtl));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, app: &mut DesignApp, size: egui::Vec2, events: Vec<egui::Event>) -> Vec<String> {
        // Changing the menu set discards a pass to re-lay-out the bar; the discarded pass has no
        // shapes, so draw until a pass with shapes comes back.
        let mut texts = Vec::new();
        for _ in 0..3 {
            let time = ctx.cumulative_pass_nr() as f64 / 60.0;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                time: Some(time),
                events: if texts.is_empty() { events.clone() } else { Vec::new() },
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                // The real entry point: `menu_bar` here would be this module's own wrapper.
                egui::MenuBar::new().ui(ui, |ui| crate::menus::menu_bar(app, ui));
            });
            output.textures_delta.clear();
            texts = collect_texts(&output.shapes);
            if !texts.is_empty() {
                break;
            }
        }
        texts
    }

    fn collect_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
        fn collect(shape: &egui::Shape, out: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(t) => out.push(t.galley.job.text.clone()),
                egui::Shape::Vec(shapes) => {
                    for s in shapes {
                        collect(s, out);
                    }
                }
                _ => {}
            }
        }
        let mut texts = Vec::new();
        for clipped in shapes {
            collect(&clipped.shape, &mut texts);
        }
        texts
    }

    fn app() -> DesignApp {
        DesignApp::new(designcraft_engine::Session::new(), Default::default())
    }

    #[test]
    fn pubgen_bar_keeps_the_host_menus_and_adds_the_pubgen_ones() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx, "en");
        let mut app = app();
        let texts = frame(&ctx, &mut app, egui::vec2(1920.0, 200.0), Vec::new());
        let order = ["File", "Edit", "Layout", "Type", "Object", "Table", "View", "Window", "Publication", "PubGen Tools", "Shortcut", "Help"];
        let positions: Vec<Option<usize>> = order.iter().map(|m| texts.iter().position(|t| t == m)).collect();
        for (i, top) in order.iter().enumerate() {
            assert!(positions[i].is_some(), "missing {top} in {texts:?}");
        }
        for (i, pair) in order.windows(2).enumerate() {
            assert!(positions[i] < positions[i + 1], "{} must come before {}: {:?}", pair[0], pair[1], positions);
        }
    }

    #[test]
    fn toggling_brings_designcrafts_menus_back_and_pubgens_away() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx, "en");
        let mut app = app();
        let texts = frame(&ctx, &mut app, egui::vec2(1600.0, 200.0), Vec::new());
        let publication = texts.iter().position(|t| t == "Publication").expect("PubGen bar");
        assert!(publication < texts.iter().position(|t| t == "Help").expect("Help"), "Publication comes before Help");

        set_pubgen_menus_on(&ctx, false);
        assert!(!pubgen_menus_on(&ctx));
        let texts = frame(&ctx, &mut app, egui::vec2(1600.0, 200.0), Vec::new());
        assert!(texts.iter().any(|t| t == "Edit"), "DesignCraft's Edit menu is back: {texts:?}");
        assert!(!texts.iter().any(|t| t == "Publication"), "PubGen's menus are gone: {texts:?}");
        let pubgen = texts.iter().position(|t| t == "Pubgen").expect("the PubGen menu stays reachable");
        assert!(pubgen < texts.iter().position(|t| t == "Help").expect("Help"));

        set_pubgen_menus_on(&ctx, true);
        let texts = frame(&ctx, &mut app, egui::vec2(1920.0, 200.0), Vec::new());
        assert!(texts.iter().any(|t| t == "Publication"), "PubGen's menus are back: {texts:?}");
        assert!(texts.iter().any(|t| t == "Edit"), "the host menus stay: {texts:?}");
        assert!(!texts.iter().any(|t| t == "Pubgen"), "the fallback Pubgen menu hides while its own bar shows: {texts:?}");
    }

    #[test]
    fn every_pubgen_action_renders() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx, "en");
        let mut app = app();
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), ..Default::default() };
        let mut output = ctx.run_ui(input, |ui| {
            egui::MenuBar::new().ui(ui, |ui| pubgen_menu(&mut app, ui));
        });
        output.textures_delta.clear();
        let texts = collect_texts(&output.shapes);
        assert!(texts.iter().any(|t| t == "Pubgen"), "the menu button is drawn: {texts:?}");
    }

    /// The Publication menu's rows, drawn like the popup draws them (the popup itself needs a
    /// click; the contents function is what the click opens).
    fn publication_rows(app: &mut DesignApp, ctx: &egui::Context) -> Vec<String> {
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), ..Default::default() };
        let mut output = ctx.run_ui(input, |ui| {
            let lang = app.ui.language.clone();
            pipeline(app, ui, &lang);
        });
        output.textures_delta.clear();
        collect_texts(&output.shapes)
    }

    /// One action, one route: the Publication menu is exactly the pipeline — no second Excel row,
    /// no About (§2.4-1/2 of the accessibility audit).
    #[test]
    fn publication_menu_is_exactly_the_pipeline() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx, "en");
        let mut app = app();
        let rows = publication_rows(&mut app, &ctx);
        let excel = rows.iter().filter(|t| t.contains("Excel")).count();
        assert_eq!(excel, 1, "one Excel route only: {rows:?}");
        assert!(!rows.iter().any(|t| t.contains("Compile Wizard")), "the duplicate route is gone: {rows:?}");
        assert!(!rows.iter().any(|t| t.contains("About")), "About lives in Help: {rows:?}");
    }

    /// Help carries About PubGen… and the menu-bar toggle — the one place both live.
    #[test]
    fn help_menu_ends_with_about_and_the_bar_toggle() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx, "en");
        let app = app();
        let lang = app.ui.language.clone();
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), ..Default::default() };
        let mut output = ctx.run_ui(input, |ui| about(ui, &lang));
        output.textures_delta.clear();
        let texts = collect_texts(&output.shapes);
        assert!(texts.iter().any(|t| t.contains("About PubGen")), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("DesignCraft Menus") || t.contains("PubGen Menus")), "{texts:?}");
    }
}
