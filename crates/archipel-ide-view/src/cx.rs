//! The read-only context every builder takes.

use archipel_ide_actions::{ActionPlace, Presentation};
use archipel_ide_command::CommandRegistry;
use archipel_ide_core::IdeState;
use archipel_ide_core::ide_actions::IdeDataProvider;
use archipel_ide_platform::{DataContext, DataProvider, data_keys};
use archipel_kit_model::Notation;
use std::any::Any;
use std::path::PathBuf;

/// What the frontend's keyboard can send (TUI-GUI-155): the GUI sends everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    /// The kitty keyboard protocol is on (or the window is a GUI): every shortcut arrives.
    pub kitty: bool,
    /// Shortcuts are written with macOS symbols.
    pub mac: bool,
    /// A key event has carried the Super modifier.
    pub super_reported: bool,
}

impl Caps {
    /// A window: every shortcut arrives.
    pub fn gui() -> Self {
        Self {
            kitty: true,
            mac: cfg!(target_os = "macos"),
            super_reported: true,
        }
    }

    /// A terminal with the kitty keyboard protocol.
    pub fn kitty() -> Self {
        Self {
            kitty: true,
            mac: cfg!(target_os = "macos"),
            super_reported: false,
        }
    }

    /// A legacy terminal.
    pub fn legacy() -> Self {
        Self {
            kitty: false,
            mac: cfg!(target_os = "macos"),
            super_reported: false,
        }
    }

    /// The notation shortcuts are written in.
    pub fn notation(&self) -> Notation {
        if self.mac {
            Notation::Mac
        } else {
            Notation::Text
        }
    }
}

/// Facts only the frontend knows, as data keys for actions (the Find bar text, the Project
/// tree selection).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiData {
    /// The text of the Find bar of the focused editor.
    pub find_query: String,
    /// The project-relative path selected in the Project tree.
    pub tree_selection: Option<PathBuf>,
    /// The tool window that has keyboard focus (the terminal frontend; the GUI leaves it
    /// `None`).
    pub focused_window: Option<archipel_ide_core::ToolWindow>,
    /// Whether the active file is read-only on disk (probed by the frontend, at most once a
    /// second: the builders do no I/O).
    pub read_only: bool,
    /// The declarations around the caret, outermost first, for the breadcrumbs (the frontend
    /// owns the structure cache; see [`crate::status::code_crumbs`]).
    pub code_crumbs: Vec<crate::status::CodeCrumb>,
}

impl DataProvider for UiData {
    fn data(&self, key: &str) -> Option<Box<dyn Any>> {
        if key == data_keys::FIND_QUERY.name() {
            Some(Box::new(self.find_query.clone()))
        } else if key == data_keys::TREE_SELECTION.name() {
            Some(Box::new(self.tree_selection.clone()?))
        } else {
            None
        }
    }
}

/// A read-only view of the IDE for the builders.
pub struct ViewCx<'a> {
    /// The IDE state.
    pub state: &'a IdeState,
    /// The registered commands.
    pub commands: &'a CommandRegistry<IdeState>,
    /// What only the frontend knows.
    pub ui: &'a UiData,
    /// What the frontend's keyboard can send.
    pub caps: Caps,
}

impl<'a> ViewCx<'a> {
    /// A context over `state`.
    pub fn new(
        state: &'a IdeState,
        commands: &'a CommandRegistry<IdeState>,
        ui: &'a UiData,
        caps: Caps,
    ) -> Self {
        Self {
            state,
            commands,
            ui,
            caps,
        }
    }

    /// Runs `f` with the action registry and the data context, with `add` putting extra
    /// values in first.
    pub fn with_data<R>(
        &self,
        add: impl for<'d> FnOnce(DataContext<'d>) -> DataContext<'d>,
        f: impl FnOnce(archipel_ide_actions::ActionRegistry<'_>, &DataContext<'_>) -> R,
    ) -> R {
        let ide = IdeDataProvider::new(self.state, self.commands);
        let data = add(DataContext::new()
            .with_provider(&ide)
            .with_provider(self.ui));
        f(self.state.action_registry(), &data)
    }

    /// What action `id` looks like in `place`; an unregistered id is disabled.
    pub fn presentation(&self, id: &str, place: ActionPlace) -> Presentation {
        self.with_data(|d| d, |registry, data| registry.update(id, place, data))
            .unwrap_or_else(Presentation::disabled)
    }

    /// Whether the button of command or action `id` is enabled.
    pub fn action_enabled(&self, id: &str) -> bool {
        self.presentation(id, ActionPlace::Toolbar).enabled
    }

    /// The shortcut of `action` for this frontend's keyboard.
    pub fn shortcut(&self, action: &str) -> Option<archipel_kit_model::Shortcut> {
        crate::keys::display_shortcut(&self.state.keymap_state().keymap, action, &self.caps)
    }
}
