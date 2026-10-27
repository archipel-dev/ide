//! Status bar entry of a running project import ("Gradle: resolving dependencies").

use crate::{IdeApp, theme, widgets};
use eframe::egui;
use serde_json::json;

/// Spinner and the importer's progress line; click to cancel. A queued auto-import shows
/// "Build files changed" until it starts.
pub fn status_widget(app: &mut IdeApp, ui: &mut egui::Ui, font: egui::FontId) {
    let Some(model) = app.state.project_model() else {
        return;
    };
    let (progress, busy) = {
        let state = model.read();
        (state.progress_text(), state.is_busy())
    };
    let Some(text) = progress.or_else(|| busy.then(|| "Build files changed…".to_owned())) else {
        return;
    };
    widgets::busy_spinner(ui, 12.0, *theme::TEXT_INFO);
    ui.add_space(4.0);
    let response = widgets::flat_widget(ui, None, &text, font, theme::TEXT_SECONDARY, false, 22.0)
        .on_hover_text("Importing the project model from the build files. Click to cancel.");
    if response.clicked() {
        app.dispatch("externalSystem.cancel", json!({}));
    }
    ui.add_space(8.0);
}
