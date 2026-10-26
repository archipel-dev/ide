//! Entry points for AI actions: the editor's "AI Actions" submenu, "Fix with AI"
//! in the Build and Problems tool windows.

use crate::icons::{self, Icon};
use crate::{IdeApp, theme};
use eframe::egui::{self, RichText};
use serde_json::Value;

/// Which AI actions are available, refreshed each frame for menus that have no
/// access to the application state.
#[derive(Debug, Clone, Copy, Default)]
struct Availability {
    enabled: bool,
    fix_problems: bool,
}

fn availability_id() -> egui::Id {
    egui::Id::new("ai_actions_availability")
}

/// Records availability for this frame (call before the editor renders).
pub fn tick(app: &IdeApp, ctx: &egui::Context) {
    let available = Availability {
        enabled: app.commands.is_enabled("ai.action.explain", &app.state),
        fix_problems: app.commands.is_enabled("ai.action.fixProblems", &app.state),
    };
    ctx.data_mut(|data| data.insert_temp(availability_id(), available));
}

/// "AI Actions" submenu for the editor context menu; returns the command to run.
pub fn editor_menu(ui: &mut egui::Ui) -> Option<(&'static str, Value)> {
    let available: Availability = ui
        .ctx()
        .data(|data| data.get_temp(availability_id()))
        .unwrap_or_default();
    let mut chosen = None;
    ui.menu_button("AI Actions", |ui| {
        ui.set_min_width(220.0);
        if !available.enabled {
            if ui.button("Set Up AI Assistant…").clicked() {
                chosen = Some(("toolwindow.activate.aiChat", Value::Null));
            }
            return;
        }
        for (label, id) in [
            ("Explain Code", "ai.action.explain"),
            ("Suggest Refactoring", "ai.action.refactor"),
            ("Find Problems", "ai.action.findProblems"),
            ("Generate Unit Tests", "ai.action.generateTests"),
            ("Generate Documentation", "ai.action.generateDocs"),
        ] {
            if ui.button(label).clicked() {
                chosen = Some((id, Value::Null));
            }
        }
        if available.fix_problems {
            ui.separator();
            if ui.button("Fix with AI").clicked() {
                chosen = Some(("ai.action.fixProblems", Value::Null));
            }
        }
    });
    chosen
}

/// "✦ Fix with AI" link shown next to a failed build/run status.
pub fn build_link(app: &mut IdeApp, ui: &mut egui::Ui) {
    if !app.commands.is_enabled("ai.action.fixBuild", &app.state) {
        return;
    }
    ui.add_space(8.0);
    icons::show(ui, Icon::Ai, 14.0, theme::MODIFIED);
    let link = ui.add(
        egui::Label::new(RichText::new("Fix with AI").color(theme::MODIFIED))
            .sense(egui::Sense::click()),
    );
    if link
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Explain the failure and suggest a fix in AI Chat")
        .clicked()
    {
        app.dispatch("ai.action.fixBuild", Value::Null);
    }
}

/// Header button of the Problems tool window; returns true when clicked.
pub fn problems_button(ui: &mut egui::Ui, enabled: bool) -> bool {
    enabled && icons::button_colored(ui, Icon::Ai, theme::MODIFIED, "Fix with AI", false).clicked()
}
