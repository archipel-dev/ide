//! Floating "Load Gradle Changes" notification shown when build scripts changed.

use crate::IdeApp;
use archipel_kit::components::{Banner, Severity};
use eframe::egui;

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    if !app.state.gradle_changes_pending || !app.state.is_gradle_project() {
        return;
    }
    let busy = app.state.runner.is_running();
    let mut banner =
        Banner::new(Severity::Warning, "Gradle build scripts have changed").dismissible(true);
    if !busy {
        banner = banner.action("Load Gradle Changes");
    }
    let out = banner.show(ui);
    let id = if out.action.is_some() {
        "gradle.loadChanges"
    } else if out.dismissed {
        "gradle.dismissChanges"
    } else {
        return;
    };
    app.dispatch(id, serde_json::Value::Null);
}
