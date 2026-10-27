//! Editor notification for a file that changed on disk while it has unsaved
//! edits, offering the two resolutions as typed commands.

use crate::{IdeApp, theme};
use archipel_ide_core::DiskConflict;
use eframe::egui::{self, RichText, vec2};
use serde_json::Value;

pub const HEIGHT: f32 = 34.0;

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
    let Some(document) = app.state.active_doc() else {
        return;
    };
    let Some(conflict) = document.conflict else {
        return;
    };
    let (message, actions) = content(&IdeApp::file_name(&document.path), conflict);
    let mut clicked = None;
    egui::Frame::new()
        .fill(*theme::BANNER_WARNING)
        .stroke(egui::Stroke::new(1.0_f32, theme::BANNER_WARNING_BORDER))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 0))
        .show(ui, |ui| {
            let size = vec2(ui.available_width(), HEIGHT);
            ui.allocate_ui_with_layout(
                size,
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_size(size);
                    ui.label(RichText::new(message).color(theme::TEXT));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for (label, command) in actions.iter().rev() {
                            let link = ui.add(
                                egui::Label::new(RichText::new(*label).color(theme::MODIFIED))
                                    .sense(egui::Sense::click()),
                            );
                            if link
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                clicked = Some(*command);
                            }
                            ui.add_space(8.0);
                        }
                    });
                },
            );
        });
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
        let commands = archipel_ide_core::built_in_commands().specs();
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
