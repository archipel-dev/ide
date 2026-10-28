//! The "File Cache Conflict" dialog (PLAT-VFS-101): a file changed on disk while its document
//! has unsaved edits. Texts and choices come from `archipel_ide_view::conflict`, the same as
//! in the terminal frontend; Escape postpones the question (the editor banner stays and
//! saving or Reload All from Disk ask again).

use crate::{IdeApp, popups, theme};
use archipel_ide_core::{DiffLine, DiffTag};
use archipel_ide_view::conflict::{Choice, command, prompt};
use eframe::egui::{self, RichText};
use std::path::PathBuf;

/// Rows of the difference listing before it scrolls.
const DIFF_HEIGHT: f32 = 220.0;

#[derive(Default)]
pub struct State {
    /// Documents whose dialog was closed with Escape.
    snoozed: Vec<PathBuf>,
    /// The difference of the open dialog (Show Difference).
    diff: Option<Vec<DiffLine>>,
}

/// The document the dialog asks about now.
pub fn active(app: &IdeApp) -> Option<usize> {
    app.state
        .conflicts()
        .into_iter()
        .find(|(_, path, _)| !app.conflict_ui.snoozed.contains(path))
        .map(|(index, _, _)| index)
}

/// Escape: postpones the question.
pub fn snooze(app: &mut IdeApp) {
    if let Some(index) = active(app)
        && let Some((_, path, _)) = app.state.conflicts().into_iter().find(|c| c.0 == index)
    {
        app.conflict_ui.snoozed.push(path);
        app.conflict_ui.diff = None;
    }
}

/// Saving and Reload All from Disk ask again.
pub fn ask_again(app: &mut IdeApp) {
    app.conflict_ui.snoozed.clear();
}

/// Applies a button of the dialog.
pub fn answer(app: &mut IdeApp, index: usize, choice: Choice) {
    match command(choice, index) {
        Some((id, params)) => {
            app.conflict_ui.diff = None;
            app.dispatch(id, params);
        }
        None => {
            app.conflict_ui.diff = app.state.conflict_diff(index, 300).ok();
        }
    }
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    // A resolved conflict is forgotten.
    let conflicts = app.state.conflicts();
    app.conflict_ui
        .snoozed
        .retain(|path| conflicts.iter().any(|(_, open, _)| open == path));
    let Some((index, path, conflict)) = conflicts
        .into_iter()
        .find(|(_, path, _)| !app.conflict_ui.snoozed.contains(path))
    else {
        app.conflict_ui.diff = None;
        return;
    };
    let name = IdeApp::file_name(&path);
    let prompt = prompt(&name, conflict);
    let mut chosen = None;
    popups::popup_frame(ctx, "dialog_file_cache_conflict", 520.0, |ui| {
        popups::title(ui, prompt.title);
        ui.label(RichText::new(&prompt.message).color(theme::TEXT));
        if let Some(lines) = &app.conflict_ui.diff {
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "- {}   + {}",
                    prompt.pane_titles.0, prompt.pane_titles.1
                ))
                .color(theme::TEXT_SECONDARY),
            );
            egui::ScrollArea::vertical()
                .max_height(DIFF_HEIGHT)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for line in lines {
                        let (mark, color) = match line.tag {
                            DiffTag::Same => (' ', theme::TEXT),
                            DiffTag::Disk => ('-', theme::ERROR),
                            DiffTag::Memory => ('+', theme::ADDED),
                            DiffTag::Gap => ('~', theme::TEXT_SECONDARY),
                        };
                        let text = if line.tag == DiffTag::Gap {
                            "...".to_owned()
                        } else {
                            line.text.clone()
                        };
                        ui.label(
                            RichText::new(format!("{mark} {text}"))
                                .monospace()
                                .color(color),
                        );
                    }
                });
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            for (label, choice) in &prompt.buttons {
                if ui.button(*label).clicked() {
                    chosen = Some(*choice);
                }
            }
        });
    });
    if let Some(choice) = chosen {
        answer(app, index, choice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{frame_input, headless_app};

    fn conflicted(name: &str) -> (IdeApp, egui::Context) {
        let (mut app, ctx) = headless_app(name, "a.txt", "alpha\n");
        app.state.documents[0].text.edit(0, 0, "mine ").unwrap();
        let root = app.state.project.as_ref().unwrap().root().to_path_buf();
        std::fs::write(root.join("a.txt"), "theirs\n").unwrap();
        app.state.synchronize_all();
        (app, ctx)
    }

    #[test]
    fn plat_vfs_101_the_dialog_opens_and_load_replaces_the_text() {
        let (mut app, ctx) = conflicted("conflict-load");
        let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        assert_eq!(active(&app), Some(0));
        answer(&mut app, 0, Choice::LoadDisk);
        assert_eq!(app.state.documents[0].text.text(), "theirs\n");
        assert!(!app.state.documents[0].text.is_dirty());
        assert_eq!(active(&app), None);
    }

    #[test]
    fn keep_memory_and_escape_and_difference() {
        let (mut app, ctx) = conflicted("conflict-keep");
        answer(&mut app, 0, Choice::ShowDifference);
        assert!(app.conflict_ui.diff.as_ref().is_some_and(|d| {
            d.iter()
                .any(|l| l.tag == DiffTag::Memory && l.text == "mine alpha")
        }));
        let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        snooze(&mut app);
        assert_eq!(active(&app), None, "Escape postpones");
        ask_again(&mut app);
        assert_eq!(active(&app), Some(0));
        answer(&mut app, 0, Choice::KeepMemory);
        assert_eq!(active(&app), None);
        assert!(app.state.documents[0].conflict.is_none());
        assert!(app.state.documents[0].text.is_dirty());
    }
}
