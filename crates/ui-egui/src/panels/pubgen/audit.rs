//! The "Audit Publication" dialog: the `preflight.pubgen` checklist (Pillar 3). One row per
//! issue; clicking a row selects the offending item and jumps to its page, like the
//! Preflight panel does.

use egui::{RichText, vec2};
use serde::Deserialize;
use serde_json::{Value, json};

use super::state::PubgenState;
use crate::DesignApp;
use crate::theme::Tokens;

/// The distribution profile: which rules the audit applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditProfile {
    /// Online PDF / digital product: a 150 ppi floor, no spine or bleed guards.
    #[default]
    Digital,
    /// Press printing: a 300 ppi floor plus the spine and bleed guards.
    Print,
}

impl AuditProfile {
    pub fn param(self) -> &'static str {
        match self {
            Self::Print => "print",
            Self::Digital => "digital",
        }
    }

    pub fn default_min_ppi(self) -> f64 {
        match self {
            Self::Print => 300.0,
            Self::Digital => 150.0,
        }
    }
}

/// One row of the checklist: an issue from `preflight.pubgen`.
#[derive(Debug, Clone, Deserialize)]
pub struct AuditIssue {
    pub severity: String,
    pub kind: String,
    pub message: String,
    pub item: Option<u64>,
    pub page: Option<usize>,
}

/// The triage tags an unresolved row can carry, in cycle order (empty = untagged).
const TAGS: [&str; 3] = ["", "false-positive", "ignored"];

fn tag_label(tag: &str) -> &'static str {
    match tag {
        "false-positive" => "FP",
        "ignored" => "IG",
        _ => "—",
    }
}

/// Tags survive re-runs and regeneration through this identity: rule + defect text.
fn tag_key(issue: &AuditIssue) -> String {
    format!("{}|{}", issue.kind, issue.message)
}

/// Run the audit and keep its issues for the dialog.
pub(crate) fn run_audit(app: &mut DesignApp, st: &mut PubgenState) {
    st.audit_revision = app.session.active().map(|d| d.revision).unwrap_or(0);
    let r = app.run("preflight.pubgen", json!({"profile": st.audit_profile.param(), "minPpi": st.audit_min_ppi}));
    st.audit_issues = match r {
        Ok(v) => Some(parse_issues(&v)),
        Err(e) => {
            st.header_status = Some((format!("Audit failed: {e}"), false));
            Some(vec![]) // shown as an empty checklist; the status band carries the error
        }
    };
}

fn parse_issues(v: &Value) -> Vec<AuditIssue> {
    serde_json::from_value(v.get("issues").cloned().unwrap_or(Value::Null)).unwrap_or_default()
}

/// Click-to-remediate: select the offending item and jump to its page.
fn open_issue(app: &mut DesignApp, issue: &AuditIssue) {
    if let Some(id) = issue.item {
        let _ = app.run("selection.set", json!({"ids": [id]}));
    }
    if let Some(p) = issue.page {
        crate::canvas::go_to_page(app, p);
    }
}

/// The Audit Publication window, shown while `st.audit_open`.
pub(crate) fn audit_dialog_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.audit_open {
        return;
    }
    // Opened from the menu (the handler only sets flags) or the document changed since the last
    // run: refresh, so every row's item and page name objects that still exist.
    let revision = app.session.active().map(|d| d.revision).unwrap_or(0);
    if st.audit_issues.is_none() || revision != st.audit_revision {
        run_audit(app, st);
    }
    let mut open = st.audit_open;
    egui::Window::new("Audit Publication").open(&mut open).collapsible(false).resizable(true).default_width(440.0).show(ctx, |ui| {
        let t = Tokens::get(ui.ctx());
        let issues = st.audit_issues.clone().unwrap_or_default();
        let tag_of = |tags: &std::collections::HashMap<String, String>, issue: &AuditIssue| tags.get(&tag_key(issue)).cloned().unwrap_or_default();
        let live = issues.iter().filter(|i| tag_of(&st.audit_tags, i).is_empty()).count();
        let errors = issues.iter().filter(|i| i.severity == "error" && tag_of(&st.audit_tags, i).is_empty()).count();
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter().circle_filled(
                r.center(),
                4.5,
                if errors == 0 { egui::Color32::from_rgb(60, 200, 90) } else { egui::Color32::from_rgb(235, 50, 50) },
            );
            crate::rtl::label(
                ui,
                if live == 0 {
                    RichText::new(crate::i18n::tr(&app.ui.language, "No errors").to_string()).size(11.0)
                } else {
                    RichText::new(format!(
                        "{}, {}",
                        crate::i18n::count_label(&app.ui.language, "errors", errors),
                        crate::i18n::count_label(&app.ui.language, "warnings", live - errors)
                    ))
                    .size(11.0)
                },
            );
            if ui.button("Re-run").clicked() {
                run_audit(app, st);
            }
        });
        ui.horizontal(|ui| {
            crate::rtl::label(ui, RichText::new("Distribution").size(10.0).color(t.text_dim));
            for (profile, label) in [(AuditProfile::Digital, "Digital (online PDF)"), (AuditProfile::Print, "Print (press)")] {
                if ui
                    .radio(st.audit_profile == profile, label)
                    .on_hover_text(match profile {
                        AuditProfile::Digital => "150 ppi floor; no spine or bleed guards.",
                        AuditProfile::Print => "300 ppi floor plus spine and bleed guards.",
                    })
                    .clicked()
                {
                    st.audit_profile = profile;
                    st.audit_min_ppi = profile.default_min_ppi();
                    run_audit(app, st);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.checkbox(&mut st.audit_hide_tagged, "hide tagged").changed() {
                    ui.ctx().request_repaint();
                }
                crate::rtl::label(ui, RichText::new("min ppi").size(10.0).color(t.text_dim));
                ui.add(egui::DragValue::new(&mut st.audit_min_ppi).range(50.0..=600.0).speed(10.0));
            });
        });
        ui.separator();
        egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
            for issue in &issues {
                let key = tag_key(issue);
                let tag = tag_of(&st.audit_tags, issue);
                let tagged = !tag.is_empty();
                if tagged && st.audit_hide_tagged {
                    continue;
                }
                let page = issue.page.filter(|_| app.session.active().is_some()).map(|p| app.session.page_label(p));
                ui.horizontal(|ui| {
                    let text = format!("{}  {}", if issue.severity == "error" { "⛔" } else { "⚠" }, issue.message);
                    let mut text = RichText::new(text).size(11.0);
                    if tagged {
                        text = text.weak();
                    }
                    let resp = ui.add(egui::Button::new(text).frame(false));
                    if let Some(pg) = &page {
                        crate::rtl::label(ui, RichText::new(format!("   page {pg}")).size(10.0).color(t.text_dim));
                    }
                    if resp.clicked() {
                        open_issue(app, issue);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Cycle: untagged → false positive → ignored → untagged.
                        if ui.button(tag_label(&tag)).on_hover_text("Tag this finding: false positive, or temporarily ignored").clicked() {
                            let next = TAGS[(TAGS.iter().position(|t| *t == tag).unwrap_or(0) + 1) % TAGS.len()];
                            st.audit_tags.insert(key.clone(), next.to_string());
                        }
                    });
                });
            }
            if !issues.is_empty() && issues.iter().all(|i| !tag_of(&st.audit_tags, i).is_empty()) {
                crate::rtl::label(ui, RichText::new("Every finding is tagged.").size(10.0).color(t.text_dim));
            }
        });
    });
    st.audit_open = open;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `preflight.pubgen` result parses into checklist rows, item and page included.
    #[test]
    fn a_result_parses_into_issues() {
        let v: Value = serde_json::from_str(
            r#"{"errors": 1, "warnings": 1, "issues": [
                {"severity": "error", "kind": "cellOverset", "message": "Overset table cell: 4 characters", "item": 17, "page": 2},
                {"severity": "warning", "kind": "missingSourceNote", "message": "Empty `Sumber` note line"}
            ]}"#,
        )
        .unwrap();
        let issues = parse_issues(&v);
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].item, Some(17));
        assert_eq!(issues[0].page, Some(2));
        assert_eq!(issues[1].item, None);
    }

    /// Tags survive re-runs through `kind|message`; the button cycles —, FP, IG, back to —.
    #[test]
    fn tag_keys_survive_and_the_cycle_returns_to_untagged() {
        let issue = AuditIssue {
            severity: "error".into(),
            kind: "cellOverset".into(),
            message: "Overset table cell: 4 characters".into(),
            item: Some(17),
            page: Some(2),
        };
        assert_eq!(tag_key(&issue), "cellOverset|Overset table cell: 4 characters");
        let cycle = |tag: &str| TAGS[(TAGS.iter().position(|t| *t == tag).unwrap_or(0) + 1) % TAGS.len()];
        assert_eq!(tag_label(cycle("")), "FP");
        assert_eq!(tag_label(cycle("false-positive")), "IG");
        assert_eq!(tag_label(cycle("ignored")), "—");
    }

    /// Clicking the overset row selects the offending frame and jumps to its page — through the
    /// real UI, from the Publication menu to the checklist row.
    #[test]
    fn clicking_the_overset_row_selects_the_frame_and_jumps_to_its_page() {
        use egui_kittest::{Harness, kittest::Queryable};

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"pages": 2})).unwrap();
        // 64×14 of frame: the sentence cannot fit, exactly like the engine's overset regression.
        let id = app
            .run(
                "frame.create",
                json!({"spread": 1, "rect": [40, 40, 104, 54], "content": "text", "text": "This text will not fit in such a tiny frame at all."}),
            )
            .unwrap()["id"]
            .as_u64()
            .unwrap();
        let mut h = Harness::builder().with_size(egui::vec2(1440.0, 900.0)).with_accessibility_check(false).build_ui_state(
            |ui, app: &mut DesignApp| {
                // The menu handler can only set this flag; the dialog itself runs the audit.
                crate::theme::install_fonts(ui.ctx(), "en");
                crate::dock::pubgen::with_state(ui.ctx(), |st| {
                    st.audit_open = true;
                });
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        let before = h.state().view().unwrap().origin;
        // Hover first, then press and release in separate passes: a same-pass pair is one raw
        // input, and the window only senses the press when it has seen the pointer hover.
        let row = h.get_by_label_contains("Overset text");
        let pos = row.rect().center();
        h.hover_at(pos);
        h.step();
        h.event(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.step();
        h.event(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(2);
        assert_eq!(h.state().session.doc().unwrap().selection.items.first().map(|i| i.0), Some(id));
        assert_ne!(h.state().view().unwrap().origin, before, "the canvas jumped to the overset frame's page");
    }
}
