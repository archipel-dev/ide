//! Quick fixes and intentions (Alt+Enter) from `textDocument/codeAction`.
//!
//! The UI asks for actions at the caret when it rests (to show the
//! lightbulb) and on Alt+Enter (to open the popup). Choosing an action
//! applies its edit (resolving it first when the server defers the edit) and
//! runs its command; edits the server pushes back with `workspace/applyEdit`
//! are applied by [`IdeState::poll_code_actions`].
use crate::{IdeState, spec, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_lsp::{Feature, RequestHandle};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::PathBuf;

/// One entry of the intention popup.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeActionItem {
    pub title: String,
    pub kind: Option<String>,
    /// Fixes a diagnostic (red bulb) rather than an intention (yellow bulb).
    pub quick_fix: bool,
    pub preferred: bool,
    /// Why the action cannot run now, shown greyed out.
    pub disabled: Option<String>,
    #[serde(skip)]
    pub(crate) raw: Value,
}

impl CodeActionItem {
    pub(crate) fn parse(raw: &Value) -> Option<Self> {
        let title = raw.get("title")?.as_str()?.to_owned();
        let kind = raw.get("kind").and_then(Value::as_str).map(String::from);
        let has_diagnostics = raw
            .get("diagnostics")
            .and_then(Value::as_array)
            .is_some_and(|diagnostics| !diagnostics.is_empty());
        Some(Self {
            quick_fix: kind
                .as_deref()
                .is_some_and(|kind| kind.starts_with("quickfix"))
                || has_diagnostics,
            preferred: raw.get("isPreferred").and_then(Value::as_bool) == Some(true),
            disabled: raw
                .get("disabled")
                .and_then(|disabled| disabled.get("reason"))
                .and_then(Value::as_str)
                .map(String::from),
            title,
            kind,
            raw: raw.clone(),
        })
    }
}

/// Actions for one caret position of one document revision.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeActionList {
    pub path: PathBuf,
    pub offset: usize,
    pub line: usize,
    pub revision: u64,
    /// Diagnostics generation the actions were computed for: quick fixes go
    /// stale when the diagnostics change.
    pub diagnostics: u64,
    pub items: Vec<CodeActionItem>,
}

impl CodeActionList {
    /// Whether any action fixes a problem (the bulb is red then).
    pub fn has_quick_fix(&self) -> bool {
        self.items.iter().any(|item| item.quick_fix)
    }
}

#[derive(Debug)]
struct PendingList {
    handle: RequestHandle,
    path: PathBuf,
    offset: usize,
    line: usize,
    revision: u64,
    diagnostics: u64,
    popup: bool,
}

#[derive(Debug, Default)]
pub struct CodeActionState {
    pending: Option<PendingList>,
    resolving: Option<(RequestHandle, PathBuf)>,
    executing: Vec<RequestHandle>,
    /// The latest actions; stale once the caret or the text moves.
    pub list: Option<CodeActionList>,
    /// Whether the intention popup is open.
    pub popup: bool,
}

impl IdeState {
    /// The actions for the current caret and revision, if known.
    pub fn current_code_actions(&self) -> Option<&CodeActionList> {
        let list = self.code_actions.list.as_ref()?;
        let document = self.documents.get(self.active_document?)?;
        (document.path == list.path
            && document.text.revision() == list.revision
            && document.selections.primary().head == list.offset
            && self.lsp.diagnostics_generation() == list.diagnostics)
            .then_some(list)
    }

    /// Whether the active document's server offers code actions.
    pub fn active_supports_code_actions(&self) -> bool {
        self.active_absolute_path()
            .is_some_and(|path| self.lsp.supports(&path, Feature::CodeAction))
    }

    /// Requests code actions at the caret; `popup` opens the intention popup
    /// when they arrive (Alt+Enter), otherwise only the lightbulb updates.
    pub fn request_code_actions(&mut self, popup: bool) -> Result<(), String> {
        if popup && self.current_code_actions().is_some() {
            self.code_actions.popup = true;
            return Ok(());
        }
        let index = self.active_document.ok_or("no active document")?;
        let absolute = self.active_absolute_path().ok_or("no project is open")?;
        let document = &self.documents[index];
        let selection = document.selections.primary();
        let to_lsp = |offset| {
            document
                .text
                .lsp_position(offset)
                .map_err(|error| error.to_string())
        };
        let (start_line, start_character) = to_lsp(selection.start())?;
        let (end_line, end_character) = to_lsp(selection.end())?;
        let caret_line = document.text.char_to_line(selection.head);
        // Diagnostics touching the caret line, as the server published them.
        let diagnostics: Vec<Value> = self
            .lsp
            .published_diagnostics(&absolute)
            .unwrap_or_default()
            .into_iter()
            .filter(|d| d.start_line <= caret_line && caret_line <= d.end_line)
            .map(|d| d.raw)
            .filter(|raw| !raw.is_null())
            .collect();
        let params = json!({
            "textDocument": {"uri": file_uri(&absolute)?},
            "range": {
                "start": {"line": start_line, "character": start_character},
                "end": {"line": end_line, "character": end_character}
            },
            "context": {"diagnostics": diagnostics, "triggerKind": if popup { 1 } else { 2 }}
        });
        let (path, offset, revision) = (
            document.path.clone(),
            selection.head,
            document.text.revision(),
        );
        if let Some(pending) = self.code_actions.pending.take() {
            self.lsp.cancel_request(&pending.handle);
        }
        let handle = self
            .lsp
            .request_with_params(&absolute, Feature::CodeAction, params)
            .map_err(|error| error.to_string())?;
        self.code_actions.pending = Some(PendingList {
            handle,
            path,
            offset,
            line: caret_line,
            revision,
            diagnostics: self.lsp.diagnostics_generation(),
            popup,
        });
        Ok(())
    }

    /// Runs the action at `index` of the current list.
    pub fn apply_code_action(&mut self, index: usize) -> Result<(), String> {
        let list = self
            .current_code_actions()
            .ok_or("the code actions are out of date; press Alt+Enter again")?;
        let item = list.items.get(index).ok_or("no such code action")?.clone();
        let path = list.path.clone();
        self.code_actions.popup = false;
        self.code_actions.list = None;
        if let Some(reason) = &item.disabled {
            return Err(format!("{}: {reason}", item.title));
        }
        self.run_code_action(&item.raw, &path)
    }

    pub(crate) fn run_code_action(
        &mut self,
        raw: &Value,
        path: &std::path::Path,
    ) -> Result<(), String> {
        // A bare `Command`.
        if raw.get("command").is_some_and(Value::is_string) {
            return self.execute_lsp_command(raw, path);
        }
        if let Some(edit) = raw.get("edit") {
            let summary = self.apply_workspace_edit(edit)?;
            self.status = match summary.files.len() {
                0 => "Nothing to change".into(),
                1 => format!(
                    "Applied: {}",
                    raw["title"].as_str().unwrap_or("code action")
                ),
                count => format!(
                    "Applied: {} ({count} files)",
                    raw["title"].as_str().unwrap_or("code action")
                ),
            };
        } else {
            let absolute = self
                .project
                .as_ref()
                .ok_or("no project is open")?
                .root()
                .join(path);
            if self.lsp.supports(&absolute, Feature::CodeActionResolve) && raw.get("data").is_some()
            {
                let handle = self
                    .lsp
                    .request_with_params(&absolute, Feature::CodeActionResolve, raw.clone())
                    .map_err(|error| error.to_string())?;
                self.code_actions.resolving = Some((handle, path.to_owned()));
                return Ok(());
            }
        }
        if let Some(command) = raw.get("command").filter(|command| command.is_object()) {
            self.execute_lsp_command(command, path)?;
        }
        Ok(())
    }

    fn execute_lsp_command(
        &mut self,
        command: &Value,
        path: &std::path::Path,
    ) -> Result<(), String> {
        let absolute = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .join(path);
        let params = json!({
            "command": command.get("command").cloned().unwrap_or(Value::Null),
            "arguments": command.get("arguments").cloned().unwrap_or(json!([])),
        });
        let handle = self
            .lsp
            .request_with_params(&absolute, Feature::ExecuteCommand, params)
            .map_err(|error| error.to_string())?;
        self.code_actions.executing.push(handle);
        Ok(())
    }

    /// Applies code action answers and server-requested edits.
    pub fn poll_code_actions(&mut self) {
        if let Some(pending) = &self.code_actions.pending
            && let Some(result) = self.lsp.take_response(&pending.handle)
            && let Some(pending) = self.code_actions.pending.take()
        {
            match result {
                Ok(value) => {
                    let items: Vec<CodeActionItem> = value
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(CodeActionItem::parse)
                        .collect();
                    if pending.popup && items.is_empty() {
                        self.status = "No quick fixes or intentions here".into();
                    }
                    self.code_actions.popup = pending.popup && !items.is_empty();
                    self.code_actions.list = Some(CodeActionList {
                        path: pending.path,
                        offset: pending.offset,
                        line: pending.line,
                        revision: pending.revision,
                        diagnostics: pending.diagnostics,
                        items,
                    });
                }
                Err(error) if pending.popup => self.status = error.to_string(),
                Err(_) => {}
            }
        }
        if let Some((handle, _)) = &self.code_actions.resolving
            && let Some(result) = self.lsp.take_response(handle)
            && let Some((_, path)) = self.code_actions.resolving.take()
        {
            let outcome = result
                .map_err(|error| error.to_string())
                .and_then(|resolved| {
                    if resolved.get("edit").is_none() && resolved.get("command").is_none() {
                        return Err("the server returned no change for this action".into());
                    }
                    self.run_code_action(&resolved, &path)
                });
            if let Err(error) = outcome {
                self.status = error;
            }
        }
        let mut executing = std::mem::take(&mut self.code_actions.executing);
        executing.retain(|handle| match self.lsp.take_response(handle) {
            None => true,
            Some(Ok(_)) => false,
            Some(Err(error)) => {
                self.status = error.to_string();
                false
            }
        });
        self.code_actions.executing = executing;
        for (server, request) in self.lsp.take_apply_edits() {
            let result = self.apply_workspace_edit(&request.edit).map(|summary| {
                if let Some(label) = &request.label {
                    self.status = format!("Applied: {label}");
                } else if !summary.files.is_empty() {
                    self.status = format!("Changed {} file(s)", summary.files.len());
                }
            });
            if let Err(error) = &result {
                self.status = error.clone();
            }
            self.lsp.respond_apply_edit(&server, request.id, result);
        }
    }
}

pub(crate) fn file_uri(path: &std::path::Path) -> Result<String, String> {
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    url::Url::from_file_path(&absolute)
        .map(|url| url.to_string())
        .map_err(|_| format!("invalid path {}", path.display()))
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "code.showIntentions",
                "Show Context Actions",
                "Code",
                Some("⌥↩"),
                "{}",
            ),
            |state: &IdeState| state.active_has_language_server(),
            |state: &mut IdeState, _| {
                state
                    .request_code_actions(true)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "code.requestActions",
                "Refresh Context Actions",
                "Code",
                None,
                "{}",
            ),
            |state: &IdeState| state.active_supports_code_actions(),
            |state: &mut IdeState, _| {
                state
                    .request_code_actions(false)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "code.applyAction",
                "Apply Context Action",
                "Code",
                None,
                r#"{"index":0}"#,
            ),
            |state: &IdeState| state.current_code_actions().is_some(),
            |state: &mut IdeState, params| {
                let index = usize_param(&params, "index")?;
                state
                    .apply_code_action(index)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("code.actions", "List Context Actions", "Code", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                Ok(json!({
                    "current": state.current_code_actions(),
                    "popup": state.code_actions.popup,
                    "pending": state.code_actions.pending.is_some(),
                }))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    #[test]
    fn actions_are_parsed_and_go_stale_when_the_caret_moves() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-actions-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "old\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "a.txt"}))
            .unwrap();
        let uri = file_uri(&root.join("a.txt")).unwrap();
        let raw = [
            json!({"title": "Replace with new", "kind": "quickfix", "isPreferred": true,
                "edit": {"changes": {uri.clone(): [{"range": {"start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 3}}, "newText": "new"}]}}}),
            json!({"title": "Extract", "kind": "refactor.extract",
                "disabled": {"reason": "Select an expression"}}),
            json!({"title": "Organize", "command": "x.organize"}),
            json!({"no": "title"}),
        ];
        let items: Vec<_> = raw.iter().filter_map(CodeActionItem::parse).collect();
        assert_eq!(items.len(), 3);
        assert!(items[0].quick_fix && items[0].preferred);
        assert_eq!(items[1].disabled.as_deref(), Some("Select an expression"));
        assert!(!items[2].quick_fix);
        state.code_actions.list = Some(CodeActionList {
            path: "a.txt".into(),
            offset: 0,
            line: 0,
            revision: state.documents[0].text.revision(),
            diagnostics: state.lsp.diagnostics_generation(),
            items,
        });
        assert!(state.current_code_actions().unwrap().has_quick_fix());
        let listed = commands
            .execute("code.actions", &mut state, json!({}))
            .unwrap();
        assert_eq!(listed["current"]["items"][1]["title"], "Extract");
        let error = commands
            .execute("code.applyAction", &mut state, json!({"index": 1}))
            .unwrap_err()
            .to_string();
        assert!(error.contains("Select an expression"), "{error}");
        // Applying consumed the list; restore it and apply the quick fix.
        state.code_actions.list = Some(CodeActionList {
            path: "a.txt".into(),
            offset: 0,
            line: 0,
            revision: state.documents[0].text.revision(),
            diagnostics: state.lsp.diagnostics_generation(),
            items: raw.iter().filter_map(CodeActionItem::parse).collect(),
        });
        commands
            .execute("code.applyAction", &mut state, json!({"index": 0}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "new\n");
        assert_eq!(state.status, "Applied: Replace with new");
        // Moving the caret makes any list stale.
        state.code_actions.list = Some(CodeActionList {
            path: "a.txt".into(),
            offset: 0,
            line: 0,
            revision: state.documents[0].text.revision(),
            diagnostics: state.lsp.diagnostics_generation(),
            items: Vec::new(),
        });
        state.documents[0]
            .selections
            .set_single(archipel_ide_text::Selection::caret(2));
        assert!(state.current_code_actions().is_none());
        assert!(!commands.is_enabled("code.applyAction", &state));
        std::fs::remove_dir_all(root).unwrap();
    }
}
