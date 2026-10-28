//! Which actions the frontends implement themselves ([`UiAction`]: popups, dialogs, the
//! clipboard), the context menu applicability rules and the facts they need. Both
//! frontends register these actions; the registry decides whether an action is enabled.

use archipel_ide_actions::{
    ActionEffect, ActionPlace, ActionRegistry, AnAction, AnActionEvent, Presentation,
};
use archipel_ide_core::ide_actions::command_enabled;
use archipel_ide_core::keymap_commands::command_for_action;
use archipel_ide_core::project_view::DirMark;
use archipel_ide_platform::{DataKey, data_keys};
use std::sync::Arc;

/// Actions that open workbench popups rather than run engine commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAction {
    GotoLine,
    FileStructure,
    Find,
    Replace,
    FindNext,
    FindPrevious,
    FindInPath,
    SaveAll,
    NewProject,
    OpenProject,
    ManageProjects,
    NewFile,
    NewDirectory,
    NewJavaClass,
    NewKotlinFile,
    Push,
    Exit,
    Copy,
    Cut,
    Paste,
}

pub fn ui_action(action: &str) -> Option<UiAction> {
    Some(match action {
        "GotoLine" => UiAction::GotoLine,
        "FileStructurePopup" => UiAction::FileStructure,
        "Find" => UiAction::Find,
        "Replace" => UiAction::Replace,
        "FindNext" => UiAction::FindNext,
        "FindPrevious" => UiAction::FindPrevious,
        "FindInPath" | "FindSelectionInPath" => UiAction::FindInPath,
        "SaveAll" => UiAction::SaveAll,
        "NewProject" | "WelcomeScreen.DefaultNewProjectAction" => UiAction::NewProject,
        "OpenFile" => UiAction::OpenProject,
        "ManageRecentProjects" => UiAction::ManageProjects,
        "NewFile" => UiAction::NewFile,
        "NewDir" => UiAction::NewDirectory,
        "NewClass" => UiAction::NewJavaClass,
        "Kotlin.NewFile" => UiAction::NewKotlinFile,
        "Vcs.Push" => UiAction::Push,
        "Exit" => UiAction::Exit,
        "$Copy" => UiAction::Copy,
        "$Cut" => UiAction::Cut,
        "$Paste" => UiAction::Paste,
        _ => return None,
    })
}

/// Every reference action id the window implements itself, with its action.
pub fn ui_action_ids() -> impl Iterator<Item = (&'static str, UiAction)> {
    const IDS: [&str; 22] = [
        "GotoLine",
        "FileStructurePopup",
        "Find",
        "Replace",
        "FindNext",
        "FindPrevious",
        "FindInPath",
        "FindSelectionInPath",
        "SaveAll",
        "NewProject",
        "WelcomeScreen.DefaultNewProjectAction",
        "OpenFile",
        "ManageRecentProjects",
        "NewFile",
        "NewDir",
        "NewClass",
        "Kotlin.NewFile",
        "Vcs.Push",
        "Exit",
        "$Copy",
        "$Cut",
        "$Paste",
    ];
    IDS.into_iter()
        .filter_map(|id| ui_action(id).map(|action| (id, action)))
}

/// Clipboard and selection actions that the focused editor or text field handles
/// itself; the keymap lists them as implemented but does not intercept their keys.
const NATIVE_ACTIONS: [&str; 4] = ["$Copy", "$Cut", "$Paste", "$SelectAll"];

/// Whether Archipel IDE implements `action` (as a command, a popup or natively).
pub fn is_implemented(action: &str) -> bool {
    ui_action(action).is_some()
        || command_for_action(action).is_some()
        || NATIVE_ACTIONS.contains(&action)
}

/// Actions that still work while the terminal has focus; every other key goes to the
/// shell, like the reference terminal's "Override IDE shortcuts" default.
pub const TERMINAL_ACTIONS: [&str; 10] = [
    "GotoAction",
    "ShowSettings",
    "HideAllWindows",
    "ActivateTerminalToolWindow",
    "ActivateProjectToolWindow",
    "ActivateCommitToolWindow",
    "ActivateFindToolWindow",
    "ActivateRunToolWindow",
    "ActivateProblemsViewToolWindow",
    "SearchEverywhere",
];

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
pub enum ContextKind {
    Tree,
    Editor,
    Tab(usize),
}

/// The facts the context menu rules need, supplied next to the usual data keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextFacts {
    pub kind: ContextKind,
    pub open_tabs: usize,
    pub has_closed_tabs: bool,
    pub has_clipboard: bool,
    pub has_document: bool,
}

pub const CONTEXT: DataKey<ContextFacts> = DataKey::new("ui.contextMenu");

/// Actions whose availability depends on the clicked target (they act on its path or tab).
pub const TARGET_ACTIONS: [&str; 12] = [
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

pub fn is_target_action(action: &str) -> bool {
    TARGET_ACTIONS.contains(&action)
}

/// Availability of `action` for a context menu target; `None` means the action's own rule
/// applies. Order matters: the first matching rule wins.
pub fn context_rule(action: &str, facts: &ContextFacts) -> Option<bool> {
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
pub fn register_ui_actions(registry: ActionRegistry<'_>) {
    for (id, action) in ui_action_ids() {
        // The same id never registers twice; a plugin cannot shadow a window action.
        let _ = registry.register(id, Arc::new(UiActionImpl(action)), None);
    }
}

pub fn register_context_rules(registry: ActionRegistry<'_>) {
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

/// Registers the frontends' actions, the command-backed ones are registered by the caller,
/// then call [`register_context_rules`].
pub fn register(registry: ActionRegistry<'_>) {
    register_ui_actions(registry);
}
