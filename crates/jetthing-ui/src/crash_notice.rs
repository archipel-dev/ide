//! "Jetthing closed unexpectedly" balloon after a crash in a previous session.

use crate::IdeApp;
use archipel_kit::components::{Severity, Toast};
use eframe::egui;
use serde_json::Value;

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if app.state.crash_report.is_none() {
        return;
    }
    // Stack above the update balloon when both are showing.
    let lift = if app.state.update_notification {
        150.0
    } else {
        0.0
    };
    let out = egui::Area::new(egui::Id::new("crash_notice"))
        .order(crate::notifications::BALLOON_ORDER)
        .anchor(
            egui::Align2::RIGHT_BOTTOM,
            egui::vec2(-16.0, -(16.0 + lift + crate::chrome::STATUS_BAR_HEIGHT)),
        )
        .show(ctx, |ui| {
            Toast::new(Severity::Warning, "Jetthing closed unexpectedly")
                .body("A crash report was saved. Attach it to a bug report so it can be fixed.")
                .action("Open Report")
                .action("Show Log in Files")
                .show(ui)
        })
        .inner;
    let action = match out.action {
        Some(0) => Some("app.openCrashReport"),
        Some(_) => Some("app.showLog"),
        None => out.dismissed.then_some("app.dismissCrashReport"),
    };
    if let Some(id) = action {
        app.dispatch(id, Value::Null);
    }
}
