//! Editor notification for a file that changed on disk while it has unsaved
//! edits, offering the two resolutions as typed commands.

use crate::IdeApp;
use archipel_kit::components::{Banner, Severity};
use eframe::egui;
use jetthing_core::DiskConflict;
use serde_json::Value;

/// Message and actions for a conflict: (text, [(label, command id)]).
pub fn content(name: &str, conflict: DiskConflict) -> (String, [(&'static str, &'static str); 2]) {
    match conflict {
        DiskConflict::Modified => (
            format!("{name} changed on disk. Your unsaved changes are not saved until you choose."),
            [
                ("Load Disk Version", "editor.reloadFromDisk"),
                ("Keep My Changes", "editor.keepMemoryVersion"),
            ],
        ),
        DiskConflict::Deleted => (
            format!("{name} was deleted on disk."),
            [
                ("Close", "editor.reloadFromDisk"),
                ("Keep and Recreate on Save", "editor.keepMemoryVersion"),
            ],
        ),
    }
}

/// Draws the banner above the active editor when its file conflicts.
pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(document) = app
        .state
        .active_document
        .and_then(|index| app.state.documents.get(index))
    else {
        return;
    };
    let Some(conflict) = document.conflict else {
        return;
    };
    let (message, actions) = content(&IdeApp::file_name(&document.path), conflict);
    let out = Banner::new(Severity::Warning, message)
        .action(actions[0].0)
        .action(actions[1].0)
        .show(ui);
    let clicked = out.action.map(|index| actions[index].1);
    ui.add_space(4.0);
    if let Some(command) = clicked {
        app.dispatch(command, Value::Null);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_offers_both_resolutions_as_commands() {
        let (text, actions) = content("App.java", DiskConflict::Modified);
        assert!(text.contains("App.java changed on disk"));
        assert_eq!(actions[0].1, "editor.reloadFromDisk");
        assert_eq!(actions[1].1, "editor.keepMemoryVersion");
        let commands = jetthing_core::built_in_commands().specs();
        for (_, id) in actions {
            assert!(
                commands.iter().any(|spec| spec.id == id),
                "{id} is registered"
            );
        }
        let (text, _) = content("App.java", DiskConflict::Deleted);
        assert!(text.contains("deleted"));
    }
}
