//! The UI's part of the action system: what only the window knows (Find bar text, Project view
//! selection, editor focus), the actions that open workbench popups, and the single place that
//! turns an action's effect into something the app does.
//!
//! Menus, the toolbar, context menus, Find Action and keymap dispatch all ask the registry
//! (`IdeState::action_registry`) for a [`Presentation`] and start actions through it, with a
//! [`DataContext`] made of the IDE's provider (`archipel_ide_core::ide_actions`) and
//! [`UiDataProvider`]. Nothing here reads another subsystem's fields.

use crate::IdeApp;
use crate::keybindings::UiAction;
use archipel_ide_actions::{ActionEffect, ActionPlace, ActionRegistry, Presentation};
use archipel_ide_core::ide_actions::IdeDataProvider;
use archipel_ide_platform::{DataContext, DataProvider, data_keys};
#[cfg(test)]
use archipel_ide_view::actions::ui_action_ids;
use eframe::egui;
use serde_json::{Value, json};
use std::any::Any;
use std::path::Path;

pub(crate) use archipel_ide_view::actions::{register_context_rules, register_ui_actions};

/// Answers the keys that only the window knows.
pub(crate) struct UiDataProvider<'a> {
    pub find_query: &'a str,
    pub tree_selection: Option<&'a Path>,
}

impl DataProvider for UiDataProvider<'_> {
    fn data(&self, key: &str) -> Option<Box<dyn Any>> {
        if key == data_keys::FIND_QUERY.name() {
            Some(Box::new(self.find_query.to_owned()))
        } else if key == data_keys::TREE_SELECTION.name() {
            Some(Box::new(self.tree_selection?.to_path_buf()))
        } else {
            None
        }
    }
}

impl IdeApp {
    /// The facts only the window knows, for the view builders.
    pub(crate) fn view_data(&self) -> archipel_ide_view::UiData {
        archipel_ide_view::UiData {
            find_query: self.find.query.clone(),
            tree_selection: self.tree_selection.clone(),
            ..archipel_ide_view::UiData::default()
        }
    }

    /// Runs `f` with the view context of this window: the builders of `archipel-ide-view`
    /// (main menu, context menus, chrome) read the state through it.
    pub(crate) fn with_view<R>(&self, f: impl FnOnce(&archipel_ide_view::ViewCx<'_>) -> R) -> R {
        let data = self.view_data();
        let cx = archipel_ide_view::ViewCx::new(
            &self.state,
            &self.commands,
            &data,
            archipel_ide_view::Caps::gui(),
        );
        f(&cx)
    }

    /// Does what the view crate's `dispatch` asked for.
    pub(crate) fn apply_view_effects(
        &mut self,
        effects: Vec<archipel_ide_view::Effect>,
        ctx: &egui::Context,
    ) {
        use archipel_ide_view::{Effect, Request};
        for effect in effects {
            match effect {
                Effect::Command { id, params } => self.dispatch(&id, params),
                Effect::Action { id, place } => {
                    self.run_action(&id, place, ctx);
                }
                Effect::ToggleToolWindow(window) => self.toggle_tool_window(window),
                Effect::HideToolWindow(window) => {
                    if self.with_view(|cx| cx.tool_window_visible(window)) {
                        self.toggle_tool_window(window);
                    }
                }
                Effect::Request(Request::Ui(action)) => self.run_ui_action(action, ctx),
                Effect::Request(Request::Paste) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                }
                Effect::Request(Request::FindInPathIn(path)) => {
                    // Scoped to the clicked directory (or file); the project root is everything.
                    self.state.project_view_mut().search_scope =
                        (!path.as_os_str().is_empty()).then(|| path.clone());
                    self.state.set_search_scope_kind(
                        if self.state.project_view().search_scope.is_some() {
                            archipel_ide_core::SearchScopeKind::Directory
                        } else {
                            archipel_ide_core::SearchScopeKind::Project
                        },
                    );
                    self.run_action("FindInPath", ActionPlace::ContextMenu, ctx);
                }
                Effect::Request(Request::SafeDelete(path)) => {
                    let usages = self.state.safe_delete_usages(&path).unwrap_or_default();
                    self.open_dialog(crate::popups::Dialog::SafeDelete { path, usages });
                }
                Effect::Request(Request::Move(path)) => {
                    let destination = path
                        .parent()
                        .map(|parent| parent.display().to_string())
                        .unwrap_or_default();
                    self.open_dialog(crate::popups::Dialog::Move { path, destination });
                }
                Effect::Request(Request::Rename(path)) => {
                    let name = IdeApp::file_name(&path);
                    self.open_dialog(crate::popups::Dialog::Rename { path, name });
                }
                Effect::Request(Request::Delete(path)) => {
                    self.open_dialog(crate::popups::Dialog::Delete { path });
                }
                Effect::Request(Request::OpenProject(path)) => {
                    if let Err(error) = self.open_project_path(&path, ctx) {
                        self.state.set_status(error);
                    }
                }
                Effect::Request(Request::GitStash { pop }) => {
                    self.git_dialog = Some(if pop {
                        crate::branches_popup::GitDialog::Unstash {
                            selected: 0,
                            pop: true,
                            error: None,
                        }
                    } else {
                        crate::branches_popup::GitDialog::Stash {
                            message: String::new(),
                            keep_index: false,
                            include_untracked: false,
                            error: None,
                        }
                    });
                    self.dialog_focus = true;
                }
                Effect::OpenMenuBar(_) => {
                    self.menu_bar_open = true;
                    self.menu_bar_popup_seen = false;
                }
                Effect::CopyText(text) => ctx.copy_text(text),
                Effect::Status(text) => self.state.set_status(text),
                Effect::SelectTreeNode(path) => self.tree_selection = path,
                // Popups, dialogs of the context menus and the terminal's own rows have
                // their own handling in the window.
                _ => {}
            }
        }
    }

    /// Registers the UI actions and the command-backed ones, once at start.
    pub(crate) fn register_actions(&mut self) {
        register_ui_actions(self.state.action_registry());
        self.sync_command_actions();
        register_context_rules(self.state.action_registry());
    }

    /// Keeps the command actions in step with the registered commands (plugins add and remove
    /// commands).
    pub(crate) fn sync_command_actions(&mut self) {
        let mut actions = std::mem::take(&mut self.command_actions);
        let result = actions.sync(self.state.application().extensions(), &self.commands);
        self.command_actions = actions;
        if let Err(error) = result {
            crate::diagnostics::log(
                crate::diagnostics::Level::Warn,
                &format!("action registration: {error}"),
            );
        }
    }

    /// Runs `f` with the registry and the current [`DataContext`] (the IDE's facts plus the
    /// window's).
    pub(crate) fn with_data<R>(
        &self,
        f: impl FnOnce(ActionRegistry<'_>, &DataContext<'_>) -> R,
    ) -> R {
        self.with_data_values(|data| data, f)
    }

    /// Like [`Self::with_data`], with `add` putting extra values into the context first (editor
    /// focus for a key press, the target of a context menu).
    pub(crate) fn with_data_values<R>(
        &self,
        add: impl for<'a> FnOnce(DataContext<'a>) -> DataContext<'a>,
        f: impl FnOnce(ActionRegistry<'_>, &DataContext<'_>) -> R,
    ) -> R {
        let ide = IdeDataProvider::new(&self.state, &self.commands);
        let ui = UiDataProvider {
            find_query: &self.find.query,
            tree_selection: self.tree_selection.as_deref(),
        };
        let data = add(DataContext::new().with_provider(&ide).with_provider(&ui));
        f(self.state.action_registry(), &data)
    }

    /// What `id` looks like in `place` right now; an id nothing registered is disabled.
    pub(crate) fn action_presentation(&self, id: &str, place: ActionPlace) -> Presentation {
        self.with_data(|registry, data| registry.update(id, place, data))
            .unwrap_or_else(Presentation::disabled)
    }

    /// Whether the button of command or action `id` is enabled.
    pub(crate) fn action_enabled(&self, id: &str) -> bool {
        self.action_presentation(id, ActionPlace::Toolbar).enabled
    }

    /// Starts `id` as the user would from `place` and runs the effect. Returns whether the
    /// action was there and enabled.
    pub(crate) fn run_action(&mut self, id: &str, place: ActionPlace, ctx: &egui::Context) -> bool {
        let effect = self.with_data(|registry, data| registry.perform(id, place, data));
        match effect {
            Some(effect) => {
                self.apply_effect(effect, place, ctx);
                true
            }
            None => false,
        }
    }

    /// Does what an action asked for.
    pub(crate) fn apply_effect(
        &mut self,
        effect: ActionEffect,
        place: ActionPlace,
        ctx: &egui::Context,
    ) {
        match effect {
            ActionEffect::None => {}
            ActionEffect::Request(request) => {
                if let Some(action) = request.downcast_ref::<UiAction>() {
                    self.run_ui_action(*action, ctx);
                }
            }
            ActionEffect::Command { id, params } => {
                if place == ActionPlace::Keymap {
                    self.dispatch_from_key(&id, params, ctx);
                } else {
                    self.dispatch(&id, params);
                }
            }
        }
    }
}

/// The parameters of the Switcher depend on the Shift key.
pub(crate) fn switcher_params(ctx: &egui::Context) -> Value {
    json!({"backward": ctx.input(|input| input.modifiers.shift)})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings;

    #[test]
    fn every_implemented_action_is_registered() {
        let app = IdeApp::new(None, Vec::new());
        let registry = app.state.action_registry();
        for action in archipel_ide_actions::Catalog::shared().actions() {
            if keybindings::is_implemented(&action.id) {
                assert!(
                    registry.contains(&action.id) || action.id == "$SelectAll",
                    "{} is implemented but not registered",
                    action.id
                );
            }
        }
    }

    #[test]
    fn menu_presentation_follows_the_open_project() {
        let app = IdeApp::new(None, Vec::new());
        // No project: creating a file and closing the project are unavailable, opening one is not.
        assert!(
            !app.action_presentation("NewFile", ActionPlace::MainMenu)
                .enabled
        );
        assert!(
            !app.action_presentation("CloseProject", ActionPlace::MainMenu)
                .enabled
        );
        assert!(
            app.action_presentation("OpenFile", ActionPlace::MainMenu)
                .enabled
        );
        assert!(
            !app.action_presentation("NoSuchAction", ActionPlace::MainMenu)
                .enabled
        );
    }

    #[test]
    fn every_ui_action_id_maps_back_to_its_action() {
        for (id, action) in ui_action_ids() {
            assert_eq!(keybindings::ui_action(id), Some(action), "{id}");
        }
    }
}
