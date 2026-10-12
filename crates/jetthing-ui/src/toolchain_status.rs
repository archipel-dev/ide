//! Status bar entry for running toolchain setups (download progress, cancel).

use crate::IdeApp;
use archipel_kit::components::{ProgressBar, Spinner, StatusBarItem, Tooltip};
use archipel_kit::tokens;
use eframe::egui;
use serde_json::json;

/// "Downloading JDK 21.0.4… 45%" with a progress bar; click to cancel.
pub fn status_widget(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some((text, fraction)) = app.state.toolchains.progress() else {
        return;
    };
    // Right to left: the item first, the progress after it.
    let item = StatusBarItem::new(text).with(Tooltip::new(
        "Toolchain setup in progress. Click to cancel; the download resumes next time.",
    ));
    if ui.add(item).clicked() {
        app.dispatch("toolchain.cancel", json!({}));
    }
    ui.add_space(tokens::size::ICON_TEXT_GAP);
    match fraction {
        Some(fraction) => {
            ui.add(ProgressBar::new(Some(fraction)).width(80.0));
        }
        None => {
            ui.add(Spinner::new().size(tokens::size::ICON));
        }
    }
    ui.add_space(tokens::size::ICON_TEXT_GAP);
}
