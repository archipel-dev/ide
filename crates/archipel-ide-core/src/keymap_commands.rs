//! Active keymap, IntelliJ action → Archipel IDE command mapping, and `keymap.*` commands.

use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_keymap::{Keymap, KeymapSet, Overrides, Shortcut};
use archipel_ide_settings::{SettingValue, ids};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Where a keymap-dispatched command applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingScope {
    /// Anywhere in the workbench.
    Global,
    /// Only while the editor has keyboard focus (text fields keep their own keys).
    Editor,
    /// Only from menus: the focused widget handles the key itself (clipboard, Tab).
    Menu,
}

/// An IntelliJ action id that Archipel IDE implements as a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionCommand {
    pub action: &'static str,
    pub command: &'static str,
    /// JSON parameters for the command.
    pub params: &'static str,
    pub scope: BindingScope,
}

const fn global(
    action: &'static str,
    command: &'static str,
    params: &'static str,
) -> ActionCommand {
    ActionCommand {
        action,
        command,
        params,
        scope: BindingScope::Global,
    }
}

const fn menu(action: &'static str, command: &'static str) -> ActionCommand {
    ActionCommand {
        action,
        command,
        params: "{}",
        scope: BindingScope::Menu,
    }
}

const fn editor(action: &'static str, command: &'static str) -> ActionCommand {
    ActionCommand {
        action,
        command,
        params: "{}",
        scope: BindingScope::Editor,
    }
}

/// Action ids from `docs/spec/data/actions.json` mapped to typed commands. Actions not
/// listed here are shown in the keymap as unbound ("not available in Archipel IDE").
pub const ACTION_COMMANDS: &[ActionCommand] = &[
    global("ShowSettings", "settings.open", "{}"),
    menu("ShowPlugins", "plugin.manager.open"),
    menu("CloseProject", "project.close"),
    menu("InvalidateCaches", "index.rebuild"),
    menu("LocalHistory.ShowHistory", "history.show"),
    menu("LocalHistory.ShowProjectHistory", "history.project"),
    menu("LocalHistory.PutLabel", "history.label"),
    menu("RecentChanges", "history.recent"),
    global("SearchEverywhere", "navigate.searchEverywhere", "{}"),
    global("GotoAction", "navigate.findAction", "{}"),
    global("GotoClass", "navigate.gotoClass", "{}"),
    global("GotoFile", "navigate.gotoFile", "{}"),
    global("GotoSymbol", "navigate.gotoSymbol", "{}"),
    global("RecentFiles", "navigate.recentFiles", "{}"),
    global("RecentLocations", "navigate.recentLocations", "{}"),
    global("Switcher", "navigate.switcher", "{}"),
    global("CloseContent", "editor.close", "{}"),
    global("CompileDirty", "build.project", "{}"),
    global("Run", "run.project", "{}"),
    // Run context configuration: the gutter target at the caret.
    editor("RunClass", "run.gutter"),
    global("Rerun", "run.rerun", "{}"),
    global("Stop", "run.cancel", "{}"),
    global("Debug", "debug.start", "{}"),
    global("ToggleLineBreakpoint", "debug.toggleBreakpoint", "{}"),
    global("StepOver", "debug.stepOver", "{}"),
    global("StepInto", "debug.stepInto", "{}"),
    global("StepOut", "debug.stepOut", "{}"),
    global("Resume", "debug.resume", "{}"),
    global("ForceStepInto", "debug.forceStepInto", "{}"),
    global("RunToCursor", "debug.runToCursor", "{}"),
    global("EvaluateExpression", "debug.evaluateDialog", "{}"),
    editor("EditBreakpoint", "debug.editBreakpoint"),
    global("CheckinProject", "toolwindow.activate.commit", "{}"),
    global("HideAllWindows", "toolwindow.hideAll", "{}"),
    global("Synchronize", "file.synchronize", "{}"),
    global("Vcs.UpdateProject", "git.pull", "{}"),
    menu("ReopenClosedTab", "editor.reopenClosed"),
    global("NextTab", "editor.nextTab", "{}"),
    global("PreviousTab", "editor.previousTab", "{}"),
    menu("PinActiveTabToggle", "editor.togglePin"),
    menu("SplitVertically", "editor.splitRight"),
    menu("SplitHorizontally", "editor.splitDown"),
    menu("MoveEditorToOppositeTabGroup", "editor.moveToOppositeGroup"),
    menu("OpenInRightSplit", "editor.openInRightSplit"),
    menu("ChangeSplitOrientation", "editor.changeSplitOrientation"),
    menu("Unsplit", "editor.unsplit"),
    menu("UnsplitAll", "editor.unsplitAll"),
    menu("KeepTabOpen", "editor.keepTabOpen"),
    global("NextSplitter", "editor.nextSplitter", "{}"),
    global("PrevSplitter", "editor.previousSplitter", "{}"),
    menu("CloseAllEditors", "editor.closeAll"),
    menu("CloseAllEditorsButActive", "editor.closeOthers"),
    menu("CloseAllUnmodifiedEditors", "editor.closeUnmodified"),
    global("NextDiff", "diff.nextChange", "{}"),
    global("PreviousDiff", "diff.previousChange", "{}"),
    global(
        "ActivateProjectToolWindow",
        "toolwindow.toggle",
        r#"{"id":"project"}"#,
    ),
    global(
        "ActivateCommitToolWindow",
        "toolwindow.toggle",
        r#"{"id":"commit"}"#,
    ),
    global(
        "ActivateStructureToolWindow",
        "toolwindow.toggle",
        r#"{"id":"structure"}"#,
    ),
    global(
        "ActivateFindToolWindow",
        "toolwindow.toggle",
        r#"{"id":"find"}"#,
    ),
    global(
        "ActivateRunToolWindow",
        "toolwindow.toggle",
        r#"{"id":"build"}"#,
    ),
    global(
        "ActivateProblemsViewToolWindow",
        "toolwindow.toggle",
        r#"{"id":"problems"}"#,
    ),
    global(
        "ActivateVersionControlToolWindow",
        "toolwindow.toggle",
        r#"{"id":"git"}"#,
    ),
    global(
        "ActivateTerminalToolWindow",
        "toolwindow.toggle",
        r#"{"id":"terminal"}"#,
    ),
    editor("$Undo", "editor.undo"),
    editor("$Redo", "editor.redo"),
    editor("CodeCompletion", "language.complete"),
    editor("InsertLiveTemplate", "template.insert"),
    editor("SurroundWithLiveTemplate", "template.surround"),
    editor("GotoNextError", "editor.nextProblem"),
    // In Android projects the UI resolves `R.x.y` references at the caret through
    // `android.resources.gotoDeclaration` first; everything else asks the language server.
    editor("GotoDeclaration", "navigate.declaration"),
    editor("GotoTypeDeclaration", "navigate.typeDeclaration"),
    editor("GotoImplementation", "navigate.implementation"),
    editor("FindUsages", "navigate.findUsages"),
    editor("ShowUsages", "navigate.showUsages"),
    global("Back", "navigate.back", "{}"),
    global("Forward", "navigate.forward", "{}"),
    editor("ShowIntentionActions", "code.showIntentions"),
    editor("Generate", "code.generate"),
    editor("GotoSuperMethod", "navigate.superMethod"),
    editor("GotoTest", "navigate.test"),
    editor("TypeHierarchy", "hierarchy.type"),
    editor("CallHierarchy", "hierarchy.call"),
    editor("RenameElement", "refactor.rename"),
    editor("ReformatCode", "refactor.reformatCode"),
    editor("OptimizeImports", "refactor.optimizeImports"),
    editor("IntroduceVariable", "refactor.extractVariable"),
    editor("ExtractMethod", "refactor.extractMethod"),
    editor("IntroduceConstant", "refactor.extractConstant"),
    editor("Inline", "refactor.inline"),
    editor("QuickJavaDoc", "language.quickDoc"),
    editor("ParameterInfo", "language.parameterInfo"),
    editor("VcsShowNextChangeMarker", "vcs.nextChange"),
    editor("VcsShowPrevChangeMarker", "vcs.previousChange"),
    editor("Vcs.RollbackChangedLines", "vcs.rollbackChange"),
    editor("GotoPreviousError", "editor.previousProblem"),
    editor("EditorSelectWord", "editor.extendSelection"),
    editor("EditorUnSelectWord", "editor.shrinkSelection"),
    editor("CollapseRegion", "editor.collapseRegion"),
    editor("ExpandRegion", "editor.expandRegion"),
    editor("CollapseSelection", "editor.foldSelection"),
    editor("CollapseAllRegions", "editor.collapseAllRegions"),
    editor("ExpandAllRegions", "editor.expandAllRegions"),
    editor("EditorDuplicate", "editor.duplicate"),
    editor("EditorDeleteLine", "editor.deleteLine"),
    editor("CommentByLineComment", "editor.commentLine"),
    editor("MoveLineUp", "editor.moveLineUp"),
    editor("MoveLineDown", "editor.moveLineDown"),
    editor("SelectNextOccurrence", "editor.selectNextOccurrence"),
    editor("UnselectPreviousOccurrence", "editor.unselectOccurrence"),
    editor("SelectAllOccurrences", "editor.selectAllOccurrences"),
    editor(
        "EditorAddCaretPerSelectedLine",
        "editor.addCaretsToLineEnds",
    ),
    editor("EditorToggleColumnMode", "editor.toggleColumnMode"),
    editor("EditorJoinLines", "editor.joinLines"),
    editor("EditorToggleCase", "editor.toggleCase"),
    editor("Markdown.Styling.CreateLink", "markdown.link"),
    menu("Markdown.InsertEmptyTable", "markdown.insertTable"),
    editor("EditorSplitLine", "editor.splitLine"),
    editor("EditorStartNewLineBefore", "editor.startNewLineBefore"),
    menu("$Copy", "editor.copy"),
    menu("$Cut", "editor.cut"),
    menu("$SelectAll", "editor.selectAll"),
    menu("EditorIndentSelection", "editor.indent"),
    menu("EditorUnindentSelection", "editor.unindent"),
    menu("About", "app.about"),
    menu("CheckForUpdate", "app.checkForUpdates"),
];

/// Archipel-only actions: no reference id, so no bundled shortcut, but a keymap override
/// can bind them like any other action.
pub const ARCHIPEL_ACTIONS: &[ActionCommand] = &[global(
    "Archipel.ToggleAgentsMode",
    "workbench.toggleMode",
    "{}",
)];

pub fn command_for_action(action: &str) -> Option<&'static ActionCommand> {
    ACTION_COMMANDS
        .iter()
        .chain(ARCHIPEL_ACTIONS)
        .find(|binding| binding.action == action)
}

/// IntelliJ actions that run `command` (used for shortcut labels in menus).
pub fn actions_for_command(command: &str) -> impl Iterator<Item = &'static str> + '_ {
    ACTION_COMMANDS
        .iter()
        .chain(ARCHIPEL_ACTIONS)
        .filter(move |binding| binding.command == command)
        .map(|binding| binding.action)
}

/// The effective keymap: the selected bundled keymap plus the user's overrides for it.
#[derive(Debug, Clone)]
pub struct KeymapState {
    pub keymap: Keymap,
    /// Overrides of all keymaps, keyed by keymap id (persisted in `keymap.overrides`).
    pub overrides: BTreeMap<String, Overrides>,
    pub error: Option<String>,
}

impl Default for KeymapState {
    fn default() -> Self {
        let mut state = Self {
            keymap: Keymap::new("", BTreeMap::new()),
            overrides: BTreeMap::new(),
            error: None,
        };
        state.rebuild(archipel_ide_keymap::Desktop::detect().default_keymap());
        state
    }
}

impl KeymapState {
    fn rebuild(&mut self, id: &str) {
        let overrides = self.overrides.get(id).cloned().unwrap_or_default();
        match KeymapSet::shared().and_then(|set| set.keymap(id, &overrides)) {
            Ok(keymap) => {
                self.keymap = keymap;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    /// Parses `keymap<TAB>action<TAB>shortcut…` lines; malformed lines are skipped and
    /// reported.
    fn parse_overrides(lines: &[String]) -> (BTreeMap<String, Overrides>, Option<String>) {
        let mut overrides: BTreeMap<String, Overrides> = BTreeMap::new();
        let mut error = None;
        for line in lines {
            let Some((keymap, rest)) = line.split_once('\t') else {
                continue;
            };
            match Overrides::from_lines(&[rest.to_owned()]) {
                Ok(parsed) => overrides
                    .entry(keymap.to_owned())
                    .or_default()
                    .actions
                    .extend(parsed.actions),
                Err(parse) => error = Some(format!("ignored keymap override: {parse}")),
            }
        }
        (overrides, error)
    }

    fn override_lines(&self) -> Vec<String> {
        self.overrides
            .iter()
            .flat_map(|(keymap, overrides)| {
                overrides
                    .to_lines()
                    .into_iter()
                    .map(move |line| format!("{keymap}\t{line}"))
            })
            .collect()
    }
}

impl IdeState {
    /// Rebuilds the keymap from the `keymap.active` and `keymap.overrides` settings.
    pub fn refresh_keymap(&mut self) {
        let id = self.settings().read().text(ids::KEYMAP).unwrap_or_else(|| {
            archipel_ide_keymap::Desktop::detect()
                .default_keymap()
                .into()
        });
        let lines = self.settings().read().list(ids::KEYMAP_OVERRIDES);
        let (overrides, error) = KeymapState::parse_overrides(&lines);
        self.keymap.overrides = overrides;
        self.keymap.rebuild(&id);
        if let Some(error) = error.or_else(|| self.keymap.error.clone()) {
            self.status = error;
        }
    }

    fn save_keymap_overrides(&mut self) -> Result<(), CommandError> {
        let lines = self.keymap.override_lines();
        {
            let settings = self.settings();
            let mut settings = settings.write();
            settings
                .set(ids::KEYMAP_OVERRIDES, SettingValue::List(lines))
                .and_then(|_| settings.store.save())
                .map_err(|error| CommandError::Failed(error.to_string()))?;
        }
        self.refresh_keymap();
        Ok(())
    }

    /// Replaces the shortcuts of `action` in the active keymap (user override).
    pub fn set_action_shortcuts(
        &mut self,
        action: &str,
        shortcuts: Vec<Shortcut>,
    ) -> Result<(), CommandError> {
        let id = self.keymap.keymap.id.clone();
        let bundled = KeymapSet::shared()
            .ok()
            .and_then(|set| set.get(&id))
            .and_then(|keymap| keymap.bindings.get(action))
            .cloned()
            .unwrap_or_default();
        let overrides = self.keymap.overrides.entry(id).or_default();
        if shortcuts == bundled {
            overrides.actions.remove(action);
        } else {
            overrides.actions.insert(action.to_owned(), shortcuts);
        }
        self.save_keymap_overrides()
    }
}

/// Same shortcut, or one is a single stroke that starts the other's chord.
fn overlaps(a: &Shortcut, b: &Shortcut) -> bool {
    match (a, b) {
        (Shortcut::Keys(a), Shortcut::Keys(b)) if a.len() != b.len() => {
            // An empty key list can come from a hand-edited override file.
            a.first().is_some_and(|first| b.first() == Some(first))
        }
        _ => a == b,
    }
}

fn shortcut_param(params: &Value) -> Result<Shortcut, CommandError> {
    string_param(params, "shortcut")?
        .parse::<Shortcut>()
        .map_err(|error| CommandError::InvalidParams(error.to_string()))
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "keymap.lookup",
                "Find Actions by Shortcut",
                "Keymap",
                None,
                r#"{"shortcut":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let shortcut = shortcut_param(&params)?;
                let actions: Vec<_> = state
                    .keymap
                    .keymap
                    .actions_for(&shortcut)
                    .iter()
                    .map(|action| {
                        json!({
                            "action": action,
                            "command": command_for_action(action).map(|binding| binding.command),
                        })
                    })
                    .collect();
                Ok(json!({"keymap": state.keymap.keymap.id, "actions": actions}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "keymap.bind",
                "Add Shortcut",
                "Keymap",
                None,
                r#"{"action":"string","shortcut":"string","removeConflicts":"bool?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let action = string_param(&params, "action")?;
                let shortcut = shortcut_param(&params)?;
                let remove = params
                    .get("removeConflicts")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let conflicts = state.keymap.keymap.conflicts_with(&action, &shortcut);
                if remove {
                    for other in &conflicts {
                        let kept: Vec<_> = state
                            .keymap
                            .keymap
                            .shortcuts(other)
                            .iter()
                            .filter(|existing| !overlaps(existing, &shortcut))
                            .cloned()
                            .collect();
                        state.set_action_shortcuts(other, kept)?;
                    }
                }
                let mut shortcuts = state.keymap.keymap.shortcuts(&action).to_vec();
                if !shortcuts.contains(&shortcut) {
                    shortcuts.push(shortcut);
                }
                state.set_action_shortcuts(&action, shortcuts)?;
                Ok(json!({"conflicts": if remove { Vec::new() } else { conflicts }}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "keymap.unbind",
                "Remove Shortcut",
                "Keymap",
                None,
                r#"{"action":"string","shortcut":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let action = string_param(&params, "action")?;
                let shortcut = shortcut_param(&params)?;
                let shortcuts: Vec<_> = state
                    .keymap
                    .keymap
                    .shortcuts(&action)
                    .iter()
                    .filter(|existing| **existing != shortcut)
                    .cloned()
                    .collect();
                state.set_action_shortcuts(&action, shortcuts)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "keymap.reset",
                "Reset Shortcuts",
                "Keymap",
                None,
                r#"{"action":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let action = string_param(&params, "action")?;
                let id = state.keymap.keymap.id.clone();
                if let Some(overrides) = state.keymap.overrides.get_mut(&id) {
                    overrides.actions.remove(&action);
                }
                state.save_keymap_overrides()?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn state_with(keymap: &str) -> IdeState {
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute(
                "settings.set",
                &mut state,
                json!({"id": ids::KEYMAP, "value": keymap}),
            )
            .unwrap();
        state
    }

    #[test]
    fn every_mapped_action_and_command_exists() {
        let commands = built_in_commands();
        let specs = commands.specs();
        let set = KeymapSet::shared().unwrap();
        let windows = set.get("$default").unwrap();
        let mac = set.get("Mac OS X 10.5+").unwrap();
        for binding in ACTION_COMMANDS {
            assert!(
                specs.iter().any(|spec| spec.id == binding.command),
                "{} maps to missing command {}",
                binding.action,
                binding.command
            );
            if binding.scope != BindingScope::Menu {
                assert!(
                    windows.bindings.contains_key(binding.action)
                        || mac.bindings.contains_key(binding.action),
                    "{} has no default shortcut",
                    binding.action
                );
            }
            // Executor actions (Run, Debug) are registered at runtime and only
            // appear in the keymaps.
            assert!(
                archipel_ide_actions::Catalog::shared()
                    .get(binding.action)
                    .is_some()
                    || windows.bindings.contains_key(binding.action),
                "{} is not a reference action id",
                binding.action
            );
            assert!(serde_json::from_str::<Value>(binding.params).is_ok());
        }
    }

    #[test]
    fn keymap_follows_the_setting() {
        let state = state_with("Default for KDE");
        assert_eq!(state.keymap.keymap.id, "Default for KDE");
        assert_eq!(
            state.keymap.keymap.shortcuts("ReformatCode"),
            &["Alt+Shift+L".parse::<Shortcut>().unwrap()]
        );
        let state = state_with("Mac OS X 10.5+");
        assert_eq!(
            state.keymap.keymap.shortcuts("ShowSettings"),
            &["Cmd+,".parse::<Shortcut>().unwrap()]
        );
    }

    #[test]
    fn bind_unbind_reset_and_conflicts() {
        let commands = built_in_commands();
        let mut state = state_with("$default");
        // Ctrl+D is Duplicate; binding it to Settings reports the conflict.
        let result = commands
            .execute(
                "keymap.bind",
                &mut state,
                json!({"action": "ShowSettings", "shortcut": "Ctrl+D"}),
            )
            .unwrap();
        assert!(
            result["conflicts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a == "EditorDuplicate")
        );
        assert!(
            state
                .keymap
                .keymap
                .shortcuts("ShowSettings")
                .contains(&"Ctrl+D".parse().unwrap())
        );
        // Persisted as an override line for the active keymap.
        let lines = state.settings().read().get(ids::KEYMAP_OVERRIDES).unwrap();
        assert_eq!(
            lines,
            SettingValue::List(vec!["$default\tShowSettings\tCtrl+Alt+S\tCtrl+D".into()])
        );
        // Remove the conflicting assignment.
        commands
            .execute(
                "keymap.bind",
                &mut state,
                json!({"action": "ShowSettings", "shortcut": "Ctrl+D", "removeConflicts": true}),
            )
            .unwrap();
        assert!(state.keymap.keymap.shortcuts("EditorDuplicate").is_empty());
        let found = commands
            .execute("keymap.lookup", &mut state, json!({"shortcut": "Ctrl+D"}))
            .unwrap();
        assert_eq!(found["actions"][0]["action"], "ShowSettings");
        assert_eq!(found["actions"][0]["command"], "settings.open");
        commands
            .execute(
                "keymap.reset",
                &mut state,
                json!({"action": "EditorDuplicate"}),
            )
            .unwrap();
        commands
            .execute(
                "keymap.unbind",
                &mut state,
                json!({"action": "ShowSettings", "shortcut": "Ctrl+D"}),
            )
            .unwrap();
        assert_eq!(
            state.keymap.keymap.shortcuts("EditorDuplicate"),
            &["Ctrl+D".parse::<Shortcut>().unwrap()]
        );
        // Reset and unbind removed their overrides; the other actions that lost Ctrl+D
        // when conflicts were removed keep theirs.
        let lines = state.settings().read().list(ids::KEYMAP_OVERRIDES);
        assert!(!lines.iter().any(|line| line.contains("EditorDuplicate")));
        assert!(!lines.iter().any(|line| line.contains("ShowSettings")));
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("$default\tSendEOF"))
        );
        assert!(
            commands
                .execute(
                    "keymap.bind",
                    &mut state,
                    json!({"action": "X", "shortcut": "Ctrl+"})
                )
                .is_err()
        );
    }
}
