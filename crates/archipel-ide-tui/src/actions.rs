//! The terminal's part of the action system: the actions that need a dialog or a picker of
//! this frontend. They register first, so they own their reference ids; everything else is a
//! `CommandAction` of the core.

use archipel_ide_actions::{ActionEffect, ActionRegistry, AnAction, AnActionEvent, Presentation};
use archipel_ide_platform::data_keys;
use std::sync::Arc;

/// What the frontend does itself when one of these actions starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiAction {
    GotoLine,
    FindInPath,
    SaveAll,
    NewFile,
    NewDirectory,
    OpenProject,
    Exit,
    NextTheme,
}

impl TuiAction {
    /// The reference action ids this frontend implements itself.
    pub const IDS: [(&'static str, TuiAction); 9] = [
        ("GotoLine", TuiAction::GotoLine),
        ("FindInPath", TuiAction::FindInPath),
        ("FindSelectionInPath", TuiAction::FindInPath),
        ("SaveAll", TuiAction::SaveAll),
        ("NewFile", TuiAction::NewFile),
        ("NewDir", TuiAction::NewDirectory),
        ("OpenFile", TuiAction::OpenProject),
        ("Exit", TuiAction::Exit),
        ("Archipel.NextTheme", TuiAction::NextTheme),
    ];
}

struct TuiActionImpl(TuiAction);

impl AnAction for TuiActionImpl {
    fn update(&self, event: &AnActionEvent) -> Presentation {
        let data = event.data();
        let document = data.has(&data_keys::ACTIVE_FILE);
        let project = data.has(&data_keys::PROJECT_ROOT);
        Presentation::when(match self.0 {
            TuiAction::GotoLine => document,
            TuiAction::NewFile | TuiAction::NewDirectory => project,
            TuiAction::SaveAll => project,
            TuiAction::FindInPath => project,
            TuiAction::OpenProject | TuiAction::Exit | TuiAction::NextTheme => true,
        })
    }

    fn perform(&self, _event: &AnActionEvent) -> ActionEffect {
        ActionEffect::Request(Arc::new(self.0))
    }
}

/// Registers the frontend's actions. Call before the core's command actions are synced.
pub fn register(registry: ActionRegistry<'_>) {
    for (id, action) in TuiAction::IDS {
        // A duplicate id means another frontend piece owns it already.
        let _ = registry.register(id, Arc::new(TuiActionImpl(action)), None);
    }
}
