//! The "Audit Publication" dialog: the `preflight.pubgen` checklist (Pillar 3). One row per
//! issue; clicking a row selects the offending item and jumps to its page, like the
//! Preflight panel does.

use egui::{RichText, vec2};
use serde::Deserialize;
use serde_json::{Value, json};

use super::state::PubgenState;
use crate::DesignApp;
use crate::theme::Tokens;

/// One row of the checklist: an issue from `preflight.pubgen`.
#[derive(Debug, Clone, Deserialize)]
pub struct AuditIssue {
    pub severity: String,
    pub kind: String,
    pub message: String,
    pub item: Option<u64>,
    pub page: Option<usize>,
}

/// Run the audit and keep its issues for the dialog.
pub(crate) fn run_audit(app: &mut DesignApp, st: &mut PubgenState) {
    let r = app.run("preflight.pubgen", json!({"minPpi": st.audit_min_ppi}));
    st.audit_issues = match r {
        Ok(v) => Some(parse_issues(&v)),
        Err(e) => {
            st.header_status = Some((format!("Audit failed: {e}"), false));
            None
        }
    };
}

fn parse_issues(v: &Value) -> Vec<AuditIssue> {
    serde_json::from_value(v.get("issues").cloned().unwrap_or(Value::Null)).unwrap_or_default()
}

/// The Audit Publication window, shown while `st.audit_open`.
pub(crate) fn audit_dialog_ui(app: &mut DesignApp, ctx: &egui::Context, st: &mut PubgenState) {
    if !st.audit_open {
        return;
    }
    let mut open = st.audit_open;
    egui::Window::new("Audit Publication").open(&mut open).collapsible(false).resizable(true).default_width(440.0).show(ctx, |ui| {
        let t = Tokens::get(ui.ctx());
        let issues = st.audit_issues.clone().unwrap_or_default();
        let errors = issues.iter().filter(|i| i.severity == "error").count();
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter().circle_filled(
                r.center(),
                4.5,
                if errors == 0 { egui::Color32::from_rgb(60, 200, 90) } else { egui::Color32::from_rgb(235, 50, 50) },
            );
            crate::rtl::label(
                ui,
                if issues.is_empty() {
                    RichText::new(crate::i18n::tr(&app.ui.language, "No errors").to_string()).size(11.0)
                } else {
                    RichText::new(format!(
                        "{}, {}",
                        crate::i18n::count_label(&app.ui.language, "errors", errors),
                        crate::i18n::count_label(&app.ui.language, "warnings", issues.len() - errors)
                    ))
                    .size(11.0)
                },
            );
            if ui.button("Re-run").clicked() {
                run_audit(app, st);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                crate::rtl::label(ui, RichText::new("min ppi").size(10.0).color(t.text_dim));
                ui.add(egui::DragValue::new(&mut st.audit_min_ppi).range(50.0..=600.0).speed(10.0));
            });
        });
        ui.separator();
        egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
            for issue in &issues {
                let page = issue.page.filter(|_| app.session.active().is_some()).map(|p| app.session.page_label(p));
                let text = format!("{}  {}", if issue.severity == "error" { "⛔" } else { "⚠" }, issue.message);
                let resp = ui.add(egui::Button::new(RichText::new(text).size(11.0)).frame(false));
                if let Some(pg) = &page {
                    crate::rtl::label(ui, RichText::new(format!("   page {pg}")).size(10.0).color(t.text_dim));
                }
                if resp.clicked() {
                    if let Some(id) = issue.item {
                        let _ = app.run("selection.set", json!({"ids": [id]}));
                    }
                    if let Some(p) = issue.page {
                        crate::canvas::go_to_page(app, p);
                    }
                }
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
}
