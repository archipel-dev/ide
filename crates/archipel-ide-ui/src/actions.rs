//! The UI's part of the action system: what only the window knows (Find bar text, Project view
//! selection, editor focus), the actions that open workbench popups, and the single place that
//! turns an action's effect into something the app does.
//!
//! Menus, the toolbar, context menus, Find Action and keymap dispatch all ask the registry
//! (`IdeState::action_registry`) for a [`Presentation`] and start actions through it, with a
//! [`DataContext`] made of the IDE's provider (`archipel_ide_core::ide_actions`) and
//! [`UiDataProvider`]. Nothing here reads another subsystem's fields.

use crate::IdeApp;
use crate::context_menus::Target;
use crate::keybindings::{UiAction, ui_action_ids};
use archipel_ide_actions::{
    ActionEffect, ActionPlace, ActionRegistry, AnAction, AnActionEvent, Presentation,
};
use archipel_ide_core::ide_actions::{IdeDataProvider, command_enabled};
use archipel_ide_core::project_view::DirMark;
use archipel_ide_platform::{DataContext, DataKey, DataProvider, data_keys};
use eframe::egui;
use serde_json::{Value, json};
use std::any::Any;
use std::path::Path;
use std::sync::Arc;

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

/// An action that opens a workbench popup or talks to the clipboard: the window owns what it
/// does, so its effect is a [`UiAction`] request.
struct UiActionImpl(UiAction);

impl AnAction for UiActionImpl {
    fn update(&self, event: &AnActionEvent) -> Presentation {
        let data = event.data();
        let document = data.has(&data_keys::ACTIVE_FILE);
        let project = data.has(&data_keys::PROJECT_ROOT);
        // The focused widget handles clipboard keys itself.
        if event.place == ActionPlace::Keymap
            && matches!(self.0, UiAction::Copy | UiAction::Cut | UiAction::Paste)
        {
            return Presentation::hidden();
        }
        Presentation::when(match self.0 {
            UiAction::GotoLine
            | UiAction::FileStructure
            | UiAction::Find
            | UiAction::Replace
            | UiAction::Paste => document,
            UiAction::FindNext | UiAction::FindPrevious => {
                document
                    && data
                        .get(&data_keys::FIND_QUERY)
                        .is_some_and(|query| !query.is_empty())
            }
            UiAction::NewFile
            | UiAction::NewDirectory
            | UiAction::NewJavaClass
            | UiAction::NewKotlinFile
            | UiAction::ManageProjects => project,
            UiAction::Push => data.flag(&data_keys::VCS_ACTIVE),
            UiAction::Copy => data.flag(&command_enabled("editor.copy")),
            UiAction::Cut => data.flag(&command_enabled("editor.cut")),
            UiAction::FindInPath
            | UiAction::SaveAll
            | UiAction::NewProject
            | UiAction::OpenProject
            | UiAction::Exit => true,
        })
    }

    fn perform(&self, _event: &AnActionEvent) -> ActionEffect {
        ActionEffect::Request(Arc::new(self.0))
    }
}

/// What a context menu knows about its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextKind {
    Tree,
    Editor,
    Tab(usize),
}

/// The facts the context menu rules need, supplied next to the usual data keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextFacts {
    pub kind: ContextKind,
    pub open_tabs: usize,
    pub has_closed_tabs: bool,
    pub has_clipboard: bool,
    pub has_document: bool,
}

pub(crate) const CONTEXT: DataKey<ContextFacts> = DataKey::new("ui.contextMenu");

/// Actions whose availability depends on the clicked target (they act on its path or tab).
const TARGET_ACTIONS: [&str; 12] = [
    "RenameElement",
    "$Delete",
    "CopyAbsolutePath",
    "CopyFileName",
    "CopyContentRootPath",
    "ProjectViewEditSource",
    "$Paste",
    "CloseContent",
    "CloseAllEditorsButActive",
    "CloseAllEditors",
    "CloseAllToTheLeft",
    "CloseAllToTheRight",
];

pub(crate) fn is_target_action(action: &str) -> bool {
    TARGET_ACTIONS.contains(&action)
}

/// Availability of `action` for a context menu target; `None` means the action's own rule
/// applies. Order matters: the first matching rule wins.
fn context_rule(action: &str, facts: &ContextFacts) -> Option<bool> {
    use ContextKind::{Editor, Tab, Tree};
    Some(match (action, facts.kind) {
        ("$Paste", Tree) => facts.has_clipboard,
        ("LocalHistory", _) => false,
        (id, Tree) if id.starts_with("jt.") => true,
        ("$Paste", Editor) => facts.has_document,
        ("CloseAllEditorsButActive", Tab(_)) => facts.open_tabs > 1,
        ("CloseAllToTheLeft", Tab(index)) => index > 0,
        ("CloseAllToTheRight", Tab(index)) => index + 1 < facts.open_tabs,
        (
            "CloseAllEditors"
            | "CloseAllUnmodifiedEditors"
            | "PinActiveEditorTab"
            | "PinActiveTabToggle",
            Tab(_),
        ) => true,
        ("ReopenClosedTab", Tab(_)) => facts.has_closed_tabs,
        (_, Tree | Tab(_)) if TARGET_ACTIONS.contains(&action) => true,
        ("$Cut" | "$Copy" | "SafeDelete" | "Move" | "RenameFile", Tree) => true,
        ("RevealIn" | "Terminal.OpenInTerminal" | "SynchronizeCurrentFile", Tree) => true,
        ("FindInPath", Tree) => true,
        ("ReformatCode" | "OptimizeImports", Tree) => true,
        _ => return None,
    })
}

/// Puts the context menu rules in front of an action: with a [`CONTEXT`] in the data the rule
/// decides, otherwise (and for everything the rules do not cover) the wrapped action does.
struct ContextMenuAction {
    id: String,
    inner: Option<Arc<dyn AnAction>>,
}

impl AnAction for ContextMenuAction {
    fn update(&self, event: &AnActionEvent) -> Presentation {
        if let Some(facts) = event.data().get(&CONTEXT)
            && let Some(enabled) = context_rule(&self.id, &facts)
        {
            return Presentation::when(enabled);
        }
        self.inner
            .as_ref()
            .map_or_else(Presentation::disabled, |inner| inner.update(event))
    }

    fn perform(&self, event: &AnActionEvent) -> ActionEffect {
        self.inner
            .as_ref()
            .map_or(ActionEffect::None, |inner| inner.perform(event))
    }
}

/// Ids the context menus add themselves (not in any reference group).
fn context_only_ids() -> Vec<String> {
    let mut ids: Vec<String> = [
        "LocalHistory",
        "jt.runFile",
        "jt.git.diff",
        "jt.git.stage",
        "jt.git.unstage",
        "jt.mark.none",
    ]
    .map(str::to_owned)
    .into();
    ids.extend(
        DirMark::ALL
            .iter()
            .map(|mark| format!("jt.mark.{}", mark.id())),
    );
    ids
}

/// Registers the window's actions, then the command-backed ones, then the context menu rules
/// in front of both. Idempotent apart from the first call.
fn register_ui_actions(registry: ActionRegistry<'_>) {
    for (id, action) in ui_action_ids() {
        // The same id never registers twice; a plugin cannot shadow a window action.
        let _ = registry.register(id, Arc::new(UiActionImpl(action)), None);
    }
}

fn register_context_rules(registry: ActionRegistry<'_>) {
    let mut ids = context_only_ids();
    ids.extend(TARGET_ACTIONS.iter().map(|id| (*id).to_owned()));
    ids.extend(
        [
            "CloseAllUnmodifiedEditors",
            "PinActiveEditorTab",
            "PinActiveTabToggle",
            "ReopenClosedTab",
            "SafeDelete",
            "Move",
            "RenameFile",
            "RevealIn",
            "Terminal.OpenInTerminal",
            "SynchronizeCurrentFile",
            "FindInPath",
            "ReformatCode",
            "OptimizeImports",
            "$Cut",
            "$Copy",
        ]
        .map(str::to_owned),
    );
    ids.sort();
    ids.dedup();
    for id in ids {
        let inner = registry.find(&id);
        if inner.is_some() {
            registry.unregister(&id);
        }
        let _ = registry.register(id.clone(), Arc::new(ContextMenuAction { id, inner }), None);
    }
}

impl IdeApp {
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

    /// Context for a context menu on `target`.
    pub(crate) fn context_facts(&self, target: &Target) -> ContextFacts {
        ContextFacts {
            kind: match target {
                Target::Tree { .. } => ContextKind::Tree,
                Target::Editor => ContextKind::Editor,
                Target::Tab(index) => ContextKind::Tab(*index),
            },
            open_tabs: self.state.open_editor_count(),
            has_closed_tabs: self.state.has_closed_tabs(),
            has_clipboard: self.state.project_view_clipboard_present(),
            has_document: self.state.active_file().is_some(),
        }
    }

    /// Whether the context menu entry `action` is enabled for `target`.
    pub(crate) fn context_action_enabled(&self, action: &str, target: &Target) -> bool {
        let facts = self.context_facts(target);
        self.with_data_values(
            |data| data.with_value(&CONTEXT, facts),
            |registry, data| registry.update(action, ActionPlace::ContextMenu, data),
        )
        .is_some_and(|presentation| presentation.enabled)
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

    fn facts(kind: ContextKind) -> ContextFacts {
        ContextFacts {
            kind,
            open_tabs: 3,
            has_closed_tabs: false,
            has_clipboard: false,
            has_document: true,
        }
    }

    #[test]
    fn tab_rules_follow_the_tab_strip() {
        let rule = |action, kind| context_rule(action, &facts(kind));
        assert_eq!(rule("CloseAllToTheLeft", ContextKind::Tab(0)), Some(false));
        assert_eq!(rule("CloseAllToTheLeft", ContextKind::Tab(1)), Some(true));
        assert_eq!(rule("CloseAllToTheRight", ContextKind::Tab(2)), Some(false));
        assert_eq!(rule("CloseAllToTheRight", ContextKind::Tab(1)), Some(true));
        assert_eq!(rule("ReopenClosedTab", ContextKind::Tab(0)), Some(false));
        assert_eq!(rule("CloseAllEditors", ContextKind::Tab(0)), Some(true));
        let single = ContextFacts {
            open_tabs: 1,
            ..facts(ContextKind::Tab(0))
        };
        assert_eq!(
            context_rule("CloseAllEditorsButActive", &single),
            Some(false)
        );
    }

    #[test]
    fn paste_depends_on_the_target() {
        assert_eq!(
            context_rule("$Paste", &facts(ContextKind::Tree)),
            Some(false)
        );
        let with_clipboard = ContextFacts {
            has_clipboard: true,
            ..facts(ContextKind::Tree)
        };
        assert_eq!(context_rule("$Paste", &with_clipboard), Some(true));
        assert_eq!(
            context_rule("$Paste", &facts(ContextKind::Editor)),
            Some(true)
        );
    }

    #[test]
    fn tree_entries_are_available_and_history_is_not() {
        let tree = facts(ContextKind::Tree);
        assert_eq!(context_rule("jt.git.stage", &tree), Some(true));
        assert_eq!(context_rule("LocalHistory", &tree), Some(false));
        assert_eq!(context_rule("SafeDelete", &tree), Some(true));
        // Anything else is the action's own business.
        assert_eq!(context_rule("GotoLine", &tree), None);
        assert_eq!(
            context_rule("SafeDelete", &facts(ContextKind::Editor)),
            None
        );
    }

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
