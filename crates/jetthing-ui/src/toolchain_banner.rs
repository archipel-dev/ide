//! "Missing toolchain" editor notification: a bar above the editor when the
//! open file's language lacks its SDK, build tool or language server (or,
//! with no editor open, when the project does), with Set up.

use crate::IdeApp;
use archipel_kit::components::{Banner, Severity};
use eframe::egui;
use serde_json::json;

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let path = app
        .state
        .active_document
        .and_then(|index| app.state.documents.get(index))
        .map(|document| document.path.clone());
    let banner = match &path {
        Some(path) => app.state.toolchains.banner_for(path),
        None => app.state.toolchains.project_banner(),
    };
    let Some(banner) = banner else {
        return;
    };
    // The kit banner has no progress slot: the text says what is happening instead.
    let text = if banner.busy {
        format!("{} Setting up…", banner.text)
    } else {
        banner.text.clone()
    };
    let mut view = Banner::new(Severity::Warning, text).dismissible(true);
    if !banner.busy {
        view = view.action("Set up");
    }
    let out = view.show(ui);
    if out.dismissed {
        app.dispatch(
            "toolchain.dismissBanner",
            json!({ "language": banner.language }),
        );
    } else if out.action.is_some() {
        app.dispatch(
            "toolchain.setup",
            json!({ "components": banner.components }),
        );
    }
}
