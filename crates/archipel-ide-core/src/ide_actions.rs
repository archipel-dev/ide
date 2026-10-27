//! The IDE's side of `DataContext` and `AnAction` (Phase 5 of the platform refactor).
//!
//! [`IdeDataProvider`] answers the common data keys (`archipel_ide_platform::data_keys`) from
//! the IDE state, so a menu, a toolbar or a shortcut asks "is a file open, is there a
//! selection, does this command apply" through a [`DataContext`] instead of reading the
//! features' fields. [`CommandAction`] is the `AnAction` behind every typed command and every
//! reference action id the keymap maps to one (`keymap_commands::ACTION_COMMANDS`): its
//! presentation comes from the context, its effect is "dispatch this command".
//!
//! The UI adds what only it knows (editor focus, Find bar text, tree selection) as a second
//! provider, and its own popups as further actions on the same extension point.

use crate::keymap_commands::{ACTION_COMMANDS, ARCHIPEL_ACTIONS, ActionCommand, BindingScope};
use crate::{IdeState, platform::Platform};
use archipel_ide_actions::{
    ActionEffect, ActionPlace, ActionRegistry, AnAction, AnActionEvent, Presentation,
};
use archipel_ide_command::CommandRegistry;
use archipel_ide_platform::{Caret, DataKey, DataProvider, ExtensionError, ExtensionRegistry};
use serde_json::Value;
use std::any::Any;
use std::collections::BTreeSet;
use std::sync::Arc;

const COMMAND_ENABLED_PREFIX: &str = "command.enabled:";

/// Whether the typed command `id` is enabled for the current state. One key per command; the
/// provider evaluates the command's own predicate only when something asks.
pub fn command_enabled(id: &str) -> DataKey<bool> {
    DataKey::dynamic(format!("{COMMAND_ENABLED_PREFIX}{id}"))
}

/// Whether the caret of the active editor is on an Android resource reference (`R.string.x`).
pub const RESOURCE_AT_CARET: DataKey<bool> = DataKey::new("android.resourceAtCaret");

/// Answers the common data keys from the IDE state.
pub struct IdeDataProvider<'a> {
    state: &'a IdeState,
    commands: &'a CommandRegistry<IdeState>,
}

impl<'a> IdeDataProvider<'a> {
    pub fn new(state: &'a IdeState, commands: &'a CommandRegistry<IdeState>) -> Self {
        Self { state, commands }
    }
}

fn boxed<T: 'static>(value: T) -> Option<Box<dyn Any>> {
    Some(Box::new(value))
}

impl DataProvider for IdeDataProvider<'_> {
    fn data(&self, key: &str) -> Option<Box<dyn Any>> {
        use archipel_ide_platform::data_keys as k;
        let state = self.state;
        if let Some(id) = key.strip_prefix(COMMAND_ENABLED_PREFIX) {
            return boxed(self.commands.is_enabled(id, state));
        }
        match key {
            _ if key == k::PROJECT_ROOT.name() => boxed(state.project.as_ref()?.root().to_owned()),
            _ if key == k::ACTIVE_FILE.name() => boxed(state.active_file()?.to_owned()),
            _ if key == k::OPEN_EDITORS.name() => boxed(state.documents.len()),
            _ if key == k::HAS_UNSAVED.name() => {
                boxed(state.documents.iter().any(|d| d.text.is_dirty()))
            }
            _ if key == k::CARET.name() => {
                let document = state.documents.get(state.active_document?)?;
                let head = document.selections.primary().head;
                let (line, column) = document.text.char_to_line_column(head).ok()?;
                boxed(Caret { line, column })
            }
            _ if key == k::SELECTED_TEXT.name() => {
                let text = state.selected_text()?;
                (!text.is_empty()).then(|| Box::new(text) as Box<dyn Any>)
            }
            _ if key == k::DIFF_ACTIVE.name() => boxed(state.diff_tab_active()),
            _ if key == k::VCS_ACTIVE.name() => boxed(state.has_git()),
            _ if key == k::DEBUG_SESSION.name() => boxed(state.is_debugging()),
            _ if key == k::ANDROID_PROJECT.name() => boxed(state.is_android_project()),
            _ if key == RESOURCE_AT_CARET.name() => boxed(state.resource_at_caret().is_some()),
            _ => None,
        }
    }
}

/// The action behind a typed command.
pub struct CommandAction {
    command: String,
    params: Value,
    scope: Option<BindingScope>,
}

impl CommandAction {
    /// A command started by its own id (toolbar buttons, Find Action entries of commands that no
    /// reference action covers).
    pub fn for_command(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            params: Value::Null,
            scope: None,
        }
    }

    /// A reference action id that Archipel IDE implements as a command.
    pub fn for_binding(binding: &ActionCommand) -> Self {
        Self {
            command: binding.command.to_owned(),
            params: serde_json::from_str(binding.params).unwrap_or(Value::Null),
            scope: Some(binding.scope),
        }
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    /// Whether a key press may run this action here. Keys the focused widget owns stay with it
    /// (menu-only and editor-only bindings), diff keys apply to the diff tab, and debugger keys
    /// stay free outside a debug session. Everything else runs, and a command that refuses
    /// reports why in the status bar.
    fn keymap_presentation(&self, event: &AnActionEvent, enabled: bool) -> Presentation {
        use archipel_ide_platform::data_keys as k;
        let data = event.data();
        let applies = match self.scope {
            Some(BindingScope::Menu) => false,
            Some(BindingScope::Editor) if !data.flag(&k::EDITOR_FOCUSED) => false,
            _ if self.command.starts_with("diff.") && !data.flag(&k::DIFF_ACTIVE) => false,
            _ if self.command.starts_with("debug.") && self.command != "debug.start" => enabled,
            _ => true,
        };
        if applies {
            Presentation::enabled()
        } else {
            Presentation::hidden()
        }
    }

    /// Commands a key press runs instead, because another feature owns the situation.
    fn keymap_redirect(&self, event: &AnActionEvent) -> Option<&'static str> {
        use archipel_ide_platform::data_keys as k;
        let data = event.data();
        let android = data.flag(&k::ANDROID_PROJECT);
        match self.command.as_str() {
            // `R.x.y` references resolve against the merged Android resources.
            "navigate.declaration" if android && data.flag(&RESOURCE_AT_CARET) => {
                Some("android.resources.gotoDeclaration")
            }
            // Android apps run by deploying 'app' (like the toolbar Run button).
            "run.project" if android => Some("android.app.deploy"),
            "editor.close" if data.flag(&k::DIFF_ACTIVE) => Some("diff.close"),
            _ => None,
        }
    }
}

impl AnAction for CommandAction {
    fn update(&self, event: &AnActionEvent) -> Presentation {
        let enabled = event.data().flag(&command_enabled(&self.command));
        if event.place == ActionPlace::Keymap {
            self.keymap_presentation(event, enabled)
        } else {
            Presentation::when(enabled)
        }
    }

    fn perform(&self, event: &AnActionEvent) -> ActionEffect {
        if event.place == ActionPlace::Keymap
            && let Some(command) = self.keymap_redirect(event)
        {
            return ActionEffect::command(command, Value::Null);
        }
        ActionEffect::command(self.command.clone(), self.params.clone())
    }
}

/// Keeps the command-backed actions of an application in step with a command registry.
#[derive(Default)]
pub struct CommandActions {
    registered: BTreeSet<String>,
    bindings_registered: bool,
}

impl CommandActions {
    /// Registers an action for every reference id that maps to a command (once) and one for
    /// every command id, and removes the actions of commands that were unregistered (a plugin
    /// that was disabled). Cheap when nothing changed.
    pub fn sync(
        &mut self,
        extensions: &ExtensionRegistry,
        commands: &CommandRegistry<IdeState>,
    ) -> Result<(), ExtensionError> {
        let registry = ActionRegistry::new(extensions);
        if !self.bindings_registered {
            for binding in ACTION_COMMANDS.iter().chain(ARCHIPEL_ACTIONS) {
                // The window may own the action (popups, clipboard): it registered first.
                if registry.contains(binding.action) {
                    continue;
                }
                registry.register(
                    binding.action,
                    Arc::new(CommandAction::for_binding(binding)),
                    None,
                )?;
            }
            self.bindings_registered = true;
        }
        let ids: BTreeSet<String> = commands.specs().into_iter().map(|spec| spec.id).collect();
        for id in ids.difference(&self.registered.clone()) {
            registry.register(id.clone(), Arc::new(CommandAction::for_command(id)), None)?;
        }
        for gone in self.registered.difference(&ids.clone()) {
            registry.unregister(gone);
        }
        self.registered = ids;
        Ok(())
    }
}

impl Platform {
    /// The extension registry actions register on.
    pub fn action_registry(&self) -> ActionRegistry<'_> {
        ActionRegistry::new(self.application().extensions())
    }
}

impl IdeState {
    /// The project-relative path of the selected editor tab's document.
    pub fn active_file(&self) -> Option<&std::path::Path> {
        let document = self.documents.get(self.active_document?)?;
        Some(&document.path)
    }

    /// Text of the primary selection of the active editor.
    pub fn selected_text(&self) -> Option<String> {
        let document = self.documents.get(self.active_document?)?;
        let primary = document.selections.primary();
        document.text.slice(primary.start(), primary.end()).ok()
    }

    /// Whether the diff viewer, not a document, is the selected editor tab.
    pub fn diff_tab_active(&self) -> bool {
        self.diff_active && self.diff.is_some()
    }

    /// Whether the open project is an Android project.
    pub fn is_android_project(&self) -> bool {
        self.android_tooling().read().project.is_some()
    }

    /// Number of open editor tabs.
    pub fn open_editor_count(&self) -> usize {
        self.documents.len()
    }

    /// Whether Reopen Closed Tab has something to reopen.
    pub fn has_closed_tabs(&self) -> bool {
        !self.tabs.closed.is_empty()
    }

    /// Whether the Project view's file clipboard holds paths to paste.
    pub fn project_view_clipboard_present(&self) -> bool {
        self.project_view.clipboard.is_some()
    }

    /// The registry actions are looked up in.
    pub fn action_registry(&self) -> ActionRegistry<'_> {
        self.platform.action_registry()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;
    use archipel_ide_actions::ActionPlace;
    use archipel_ide_platform::{DataContext, data_keys};

    fn registered() -> (IdeState, CommandRegistry<IdeState>, CommandActions) {
        let state = IdeState::default();
        let commands = built_in_commands();
        let mut actions = CommandActions::default();
        actions
            .sync(state.platform.application().extensions(), &commands)
            .unwrap();
        (state, commands, actions)
    }

    #[test]
    fn every_command_and_mapped_reference_action_is_registered() {
        let (state, commands, _) = registered();
        let registry = state.action_registry();
        for spec in commands.specs() {
            assert!(registry.contains(&spec.id), "{}", spec.id);
        }
        for binding in ACTION_COMMANDS.iter().chain(ARCHIPEL_ACTIONS) {
            assert!(registry.contains(binding.action), "{}", binding.action);
        }
    }

    #[test]
    fn sync_follows_commands_that_come_and_go() {
        let (state, mut commands, mut actions) = registered();
        let registry = state.action_registry();
        assert!(commands.unregister("settings.open"));
        actions
            .sync(state.platform.application().extensions(), &commands)
            .unwrap();
        assert!(!registry.contains("settings.open"));
        // The reference id still exists; it just reports disabled.
        assert!(registry.contains("ShowSettings"));
    }

    #[test]
    fn presentation_follows_the_command_predicate() {
        let (state, commands, _) = registered();
        let provider = IdeDataProvider::new(&state, &commands);
        let data = DataContext::new().with_provider(&provider);
        let registry = state.action_registry();
        // Closing a project needs one.
        let closed = registry
            .update("CloseProject", ActionPlace::MainMenu, &data)
            .unwrap();
        assert!(!closed.enabled && closed.visible);
        // Settings always open.
        assert!(
            registry
                .update("ShowSettings", ActionPlace::MainMenu, &data)
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn keys_of_an_empty_ide_are_missing_or_false() {
        let (state, commands, _) = registered();
        let provider = IdeDataProvider::new(&state, &commands);
        let data = DataContext::new().with_provider(&provider);
        assert!(!data.has(&data_keys::PROJECT_ROOT));
        assert!(!data.has(&data_keys::ACTIVE_FILE));
        assert!(!data.has(&data_keys::CARET));
        assert!(!data.has(&data_keys::SELECTED_TEXT));
        assert!(!data.flag(&data_keys::VCS_ACTIVE));
        assert_eq!(data.get(&data_keys::OPEN_EDITORS), Some(0));
        assert!(!data.flag(&command_enabled("no.such.command")));
    }

    #[test]
    fn keymap_rules_hide_what_the_focused_widget_owns() {
        let (state, commands, _) = registered();
        let provider = IdeDataProvider::new(&state, &commands);
        let registry = state.action_registry();
        let unfocused = DataContext::new().with_provider(&provider);
        let focused = DataContext::new()
            .with_provider(&provider)
            .with_value(&data_keys::EDITOR_FOCUSED, true);
        // Clipboard keys belong to the focused widget: menu-only bindings never run from keys.
        let copy = registry
            .update("$Copy", ActionPlace::Keymap, &focused)
            .unwrap();
        assert!(!copy.visible);
        // Diff keys only apply on the diff tab.
        let next = registry
            .update("Diff.NextChange", ActionPlace::Keymap, &focused)
            .map(|p| p.visible);
        assert_ne!(next, Some(true));
        // Global bindings run anywhere.
        assert!(
            registry
                .update("ShowSettings", ActionPlace::Keymap, &unfocused)
                .unwrap()
                .visible
        );
    }

    #[test]
    fn android_redirects_apply_to_keys_only() {
        let (state, commands, _) = registered();
        let provider = IdeDataProvider::new(&state, &commands);
        let registry = state.action_registry();
        let android = DataContext::new()
            .with_provider(&provider)
            .with_value(&data_keys::ANDROID_PROJECT, true);
        let key = registry.perform("Run", ActionPlace::Keymap, &android);
        let menu = registry.perform("Run", ActionPlace::MainMenu, &android);
        // Without a project the command is disabled for the menu; the key still runs.
        assert!(menu.is_none());
        match key {
            Some(ActionEffect::Command { id, .. }) => assert_eq!(id, "android.app.deploy"),
            other => panic!("unexpected {other:?}"),
        }
    }
}
