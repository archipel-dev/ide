//! Status bar entry for running toolchain setups (download progress, cancel).

use crate::{IdeApp, theme, widgets};
use eframe::egui;
use serde_json::json;

/// "Downloading JDK 21.0.4… 45%" with a progress bar; click to cancel.
pub fn status_widget(app: &mut IdeApp, ui: &mut egui::Ui, font: egui::FontId) {
    let Some((text, fraction)) = app.state.toolchains.progress() else {
        return;
    };
    match fraction {
        Some(fraction) => {
            ui.add(
                egui::ProgressBar::new(fraction)
                    .desired_width(80.0)
                    .desired_height(4.0)
                    .fill(*theme::TEXT_INFO),
            );
        }
        None => {
            widgets::busy_spinner(ui, 12.0, *theme::TEXT_INFO);
        }
    }
    ui.add_space(4.0);
    let response = widgets::flat_widget(ui, None, &text, font, theme::TEXT_SECONDARY, false, 22.0)
        .on_hover_text(
            "Toolchain setup in progress. Click to cancel; the download resumes next time.",
        );
    if response.clicked() {
        app.dispatch("toolchain.cancel", json!({}));
    }
    ui.add_space(8.0);
}
