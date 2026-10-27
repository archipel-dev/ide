//! Commit window integration: the AI "Generate Commit Message" button and
//! streaming of the generated message into the message field.

use crate::icons::{self, Icon};
use crate::{IdeApp, theme};
use eframe::egui;
use serde_json::{Value, json};
use std::path::PathBuf;

/// Shows the generate/stop button and streams a generated message into the
/// commit message field. Call from the Commit window's action row.
pub fn commit_message_button(app: &mut IdeApp, ui: &mut egui::Ui, selected: &[PathBuf]) {
    let mut generating = false;
    if let Some((text, streaming)) = app.state.take_commit_draft() {
        app.commit_message = text;
        generating = streaming;
    }
    if generating {
        ui.add(egui::Spinner::new().size(12.0).color(theme::MODIFIED));
        if icons::button_colored(ui, Icon::Stop, theme::STOP_RED, "Stop generating", false)
            .clicked()
        {
            app.dispatch("ai.vcs.stopCommitMessage", Value::Null);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(30));
        return;
    }
    if !app.action_enabled("ai.vcs.generateCommitMessage") {
        return;
    }
    let tooltip = if selected.is_empty() {
        "Select changes to generate a commit message"
    } else {
        "Generate Commit Message with AI"
    };
    let response = icons::button_colored(ui, Icon::Ai, theme::MODIFIED, tooltip, false);
    if response.clicked() && !selected.is_empty() {
        app.dispatch("ai.vcs.generateCommitMessage", json!({"paths": selected}));
    }
}
