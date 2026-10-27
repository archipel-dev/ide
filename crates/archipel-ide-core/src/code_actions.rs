//! Quick fixes and intentions (Alt+Enter) from `textDocument/codeAction`.
//!
//! The UI asks for actions at the caret when it rests (to show the
//! lightbulb) and on Alt+Enter (to open the popup). Choosing an action
//! applies its edit (resolving it first when the server defers the edit) and
//! runs its command; edits the server pushes back with `workspace/applyEdit`
//! are applied by [`IdeState::poll_code_actions`].
use crate::{IdeState, spec, usize_param};
use archipel_ide_analysis::intention::IntentionGroup;
use archipel_ide_analysis::modcommand::NavigateTo;
use archipel_ide_analysis::{CancelToken, FileId, FileRev, ModCommand};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_daemon::inspection::catalog;
use archipel_ide_daemon::intention::{
    AlertAction, AlertCache, AlertInput, AlertSet, Applied, ApplyInput, Source, apply,
};
use archipel_ide_lsp::{Feature, RequestHandle};
use archipel_ide_text::{Selection, TextChange};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// The client command jdtls uses to hand a `WorkspaceEdit` to the editor.
const APPLY_WORKSPACE_EDIT: &str = "java.apply.workspaceEdit";

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
    /// Native (daemon) fixes and intentions, cached per caret and revision.
    pub(crate) alerts: AlertCache,
    /// Intention ids the user switched off.
    pub disabled_intentions: HashSet<String>,
}

impl IdeState {
    /// The LSP code actions of the current list as popup rows, sorted into
    /// IntelliJ's groups like the native ones: a quick fix is attached to the
    /// language server's highlight it fixes (matched by the diagnostic the
    /// action names, else the one at the caret) and joins the Error group
    /// (error at the caret), Inspection (lesser problem at the caret) or
    /// RemoteError (elsewhere on the line); refactorings, source actions and
    /// quick assists are intentions.
    fn lsp_alert_rows(
        &self,
        document: &crate::Document,
        markup: &archipel_ide_daemon::FileMarkup,
        caret: u32,
        selection: Option<&std::ops::Range<u32>>,
    ) -> Vec<AlertAction> {
        let Some(list) = self.current_code_actions() else {
            return Vec::new();
        };
        let byte = |line: u64, character: u64| {
            let char_index = document
                .text
                .char_from_lsp_position(line as usize, character as usize);
            document.text.char_to_byte(char_index) as u32
        };
        let lsp_range = |value: &Value| -> Option<std::ops::Range<u32>> {
            let position = |name: &str| {
                let p = value.get("range")?.get(name)?;
                Some(byte(
                    p.get("line")?.as_u64()?,
                    p.get("character")?.as_u64()?,
                ))
            };
            Some(position("start")?..position("end")?)
        };
        let at_caret = |range: &std::ops::Range<u32>| {
            (range.start <= caret && caret <= range.end)
                || selection.is_some_and(|s| range.start < s.end && s.start < range.end)
        };
        let lsp_highlights: Vec<_> = markup
            .iter()
            .filter(|info| info.tool.is_lsp() && info.severity.is_problem())
            .collect();
        list.items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.disabled.is_none())
            .map(|(index, item)| {
                let named: Vec<(std::ops::Range<u32>, Option<&str>)> = item
                    .raw
                    .get("diagnostics")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|d| Some((lsp_range(d)?, d.get("message").and_then(Value::as_str))))
                    .collect();
                // The highlights the action names, else the one at the caret.
                let mut attached: Vec<&archipel_ide_analysis::HighlightInfo> = lsp_highlights
                    .iter()
                    .copied()
                    .filter(|info| {
                        named.iter().any(|(range, message)| {
                            info.range == *range
                                && message.is_none_or(|m| info.message.as_deref() == Some(m))
                        })
                    })
                    .collect();
                if attached.is_empty() && item.quick_fix && named.is_empty() {
                    attached = lsp_highlights
                        .iter()
                        .copied()
                        .filter(|info| at_caret(&info.range))
                        .collect();
                }
                // The nearest to the caret decides the group.
                attached.sort_by_key(|info| !at_caret(&info.range));
                let anchor = attached.first().copied();
                let group = match (item.quick_fix, anchor) {
                    (false, _) => IntentionGroup::Other,
                    (true, None) => IntentionGroup::Inspection,
                    (true, Some(info)) if !at_caret(&info.range) => IntentionGroup::RemoteError,
                    (true, Some(info))
                        if info.severity >= archipel_ide_analysis::Severity::ERROR =>
                    {
                        IntentionGroup::Error
                    }
                    (true, Some(_)) => IntentionGroup::Inspection,
                };
                let priority = if item.preferred {
                    archipel_ide_analysis::intention::Priority::High
                } else {
                    archipel_ide_analysis::intention::Priority::Normal
                };
                AlertAction::external(
                    item.title.clone(),
                    group,
                    priority,
                    Source::Lsp,
                    format!("lsp:{index}"),
                )
                .attached_to(anchor.map(|info| info.range.clone()))
            })
            .collect()
    }

    /// Caret byte offset and selection (bytes) of the active document.
    fn active_caret_bytes(&self) -> Option<(FileId, u32, Option<std::ops::Range<u32>>)> {
        let document = self.documents.get(self.active_document?)?;
        let file = self.analysis.host.lookup(&document.path)?;
        let selection = document.selections.primary();
        let byte = |c: usize| document.text.char_to_byte(c.min(document.text.len_chars())) as u32;
        let range = (!selection.is_caret()).then(|| byte(selection.start())..byte(selection.end()));
        Some((file, byte(selection.head), range))
    }

    /// The Alt+Enter rows at the caret: native quick fixes and intentions from
    /// the daemon plus the language server's code actions, deduplicated and
    /// ordered (group priority + priority weight, then text). `None` while the
    /// analysis host has not caught up with the editor text.
    pub fn current_alerts(&mut self) -> Option<Arc<AlertSet>> {
        let service = self.daemon_service()?;
        let (file, caret, selection) = self.active_caret_bytes()?;
        let document = self.documents.get(self.active_document?)?;
        let snap = self.analysis.host.snapshot();
        if snap.file_rev(file) != Some(FileRev(document.text.revision())) {
            return None;
        }
        let syntax = snap.syntax(file)?;
        let names: HashMap<String, String> = catalog::inspections(snap.registry())
            .into_iter()
            .map(|t| (t.id.to_owned(), t.name.to_owned()))
            .collect();
        let daemon = service.lock();
        let markup = daemon.markup(file)?;
        let extra = self.lsp_alert_rows(document, markup, caret, selection.as_ref());
        let cancel = CancelToken::new();
        let input = AlertInput {
            snap: &snap,
            file,
            syntax: &syntax,
            markup,
            caret,
            selection,
            cancel: &cancel,
            extra,
            disabled: &self.code_actions.disabled_intentions,
            tool_names: &names,
        };
        let version = daemon.store().version();
        let mut cache = std::mem::take(&mut self.code_actions.alerts);
        let set = cache.get_or_compute(&input, version).ok();
        drop(daemon);
        self.code_actions.alerts = cache;
        set
    }

    /// Chooses row `index` of [`IdeState::current_alerts`].
    pub fn apply_alert(&mut self, index: usize) -> Result<(), String> {
        let set = self
            .current_alerts()
            .ok_or("the actions are out of date; press Alt+Enter again")?;
        let action = set.actions.get(index).ok_or("no such action")?.clone();
        let (file, caret, selection) = self.active_caret_bytes().ok_or("no active document")?;
        let service = self
            .daemon_service()
            .ok_or("the analysis daemon is not available")?;
        let applied = {
            let snap = self.analysis.host.snapshot();
            let syntax = snap.syntax(file).ok_or("the file has no syntax tree")?;
            let daemon = service.lock();
            let markup = daemon.markup(file).ok_or("no highlights for this file")?;
            let cancel = CancelToken::new();
            apply(
                &action,
                &ApplyInput {
                    snap: &snap,
                    file,
                    syntax: &syntax,
                    markup,
                    caret,
                    selection,
                    cancel: &cancel,
                },
            )
            .map_err(|_| "cancelled".to_owned())?
        };
        match applied {
            Applied::External { token } => {
                let index = token
                    .strip_prefix("lsp:")
                    .and_then(|n| n.parse().ok())
                    .ok_or("unknown action owner")?;
                self.apply_code_action(index)
            }
            other => {
                self.code_actions.popup = false;
                self.code_actions.list = None;
                match other {
                    Applied::Command(command) => {
                        let message = self.execute_mod_command(&command)?;
                        self.status = if message.is_empty() {
                            format!("Applied: {}", action.text)
                        } else {
                            message
                        };
                        Ok(())
                    }
                    Applied::DisableInspection { tool } => {
                        let root = self.project.as_ref().map(|p| p.root().to_owned());
                        service
                            .edit_profile(
                                root.as_deref(),
                                crate::inspection_commands::config_dir_for_actions(),
                                |file| file.set_enabled(&tool, Some(false)),
                            )
                            .map_err(|e| e.to_string())?;
                        self.analysis
                            .host
                            .apply(archipel_ide_analysis::Change::SettingsChanged);
                        self.status = format!("Inspection '{tool}' disabled in the active profile");
                        Ok(())
                    }
                    Applied::EditProfile { .. } => {
                        self.settings_request = Some(crate::INSPECTIONS_PAGE.to_owned());
                        Ok(())
                    }
                    Applied::External { .. } => Ok(()),
                }
            }
        }
    }

    /// Executes a [`ModCommand`] against the open documents: every document
    /// edit is one undo step; commands the editor cannot run yet are refused.
    /// Returns a message to show (empty = the default).
    pub fn execute_mod_command(&mut self, command: &ModCommand) -> Result<String, String> {
        match command {
            ModCommand::Nothing => Ok(String::new()),
            ModCommand::Composite(parts) => {
                let mut message = String::new();
                for part in parts {
                    let next = self.execute_mod_command(part)?;
                    if !next.is_empty() {
                        message = next;
                    }
                }
                Ok(message)
            }
            ModCommand::UpdateFile {
                file,
                edits,
                expected_rev,
            } => {
                let snap = self.analysis.host.snapshot();
                let path = snap.path(*file).ok_or("unknown file")?.to_owned();
                let index = self
                    .documents
                    .iter()
                    .position(|d| d.path == path)
                    .ok_or_else(|| format!("{} is not open", path.display()))?;
                let document = &mut self.documents[index];
                if FileRev(document.text.revision()) != *expected_rev {
                    return Err(format!(
                        "{} changed since the fix was computed; try again",
                        path.display()
                    ));
                }
                let mut changes = Vec::with_capacity(edits.len());
                let text = document.text.text();
                for edit in edits {
                    let (start, end) = (edit.range.start as usize, edit.range.end as usize);
                    if start > end
                        || end > text.len()
                        || !text.is_char_boundary(start)
                        || !text.is_char_boundary(end)
                    {
                        return Err(format!("{}: invalid edit range", path.display()));
                    }
                    changes.push(TextChange {
                        start_char: document.text.byte_to_char(start),
                        end_char: document.text.byte_to_char(end),
                        inserted: edit.new_text.replace("\r\n", "\n"),
                    });
                }
                let caret = document.selections.primary().head;
                let shift: isize = changes
                    .iter()
                    .filter(|c| c.end_char <= caret)
                    .map(|c| {
                        c.inserted.chars().count() as isize - (c.end_char - c.start_char) as isize
                    })
                    .sum();
                let revision = document.text.revision();
                document
                    .text
                    .edit_many(changes)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                if document.text.revision() != revision {
                    let caret = caret
                        .saturating_add_signed(shift)
                        .min(document.text.len_chars());
                    document.selections.set_single(Selection::caret(caret));
                    let source = document.text.text();
                    document.refresh_analysis(&source);
                    let version = document.text.revision().min(i32::MAX as u64) as i32;
                    self.sync_document_lsp(&path, &source, version);
                    self.close_completion();
                }
                Ok(String::new())
            }
            ModCommand::Navigate {
                to: NavigateTo::Range { file, range },
                select,
            } => {
                let snap = self.analysis.host.snapshot();
                let path = snap.path(*file).ok_or("unknown file")?.to_owned();
                let index = self
                    .documents
                    .iter()
                    .position(|d| d.path == path)
                    .ok_or("file is not open")?;
                let document = &mut self.documents[index];
                let target = select.clone().unwrap_or(range.clone());
                let (a, b) = (
                    document.text.byte_to_char(target.start as usize),
                    document.text.byte_to_char(target.end as usize),
                );
                document
                    .selections
                    .set_single(Selection { anchor: a, head: b });
                self.active_document = Some(index);
                Ok(String::new())
            }
            ModCommand::DisplayMessage { message, .. } => Ok(message.clone()),
            other => Err(format!(
                "{} is not supported by the editor yet",
                command_kind(other)
            )),
        }
    }

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
        if !self.active_supports_code_actions() {
            // Native fixes and intentions only.
            let native = self
                .current_alerts()
                .is_some_and(|set| !set.actions.is_empty());
            if popup {
                if native {
                    self.code_actions.popup = true;
                } else {
                    self.status = "No quick fixes or intentions here".into();
                }
            }
            return Ok(());
        }
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
        // jdtls hands some edits over as a client command; there is nothing
        // for the server to execute.
        if command.get("command").and_then(Value::as_str) == Some(APPLY_WORKSPACE_EDIT) {
            let mut files = 0;
            for edit in command
                .get("arguments")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                files += self.apply_workspace_edit(edit)?.files.len();
            }
            self.status = format!(
                "Applied: {}",
                command
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("code action")
            );
            if files == 0 {
                self.status = "Nothing to change".into();
            }
            return Ok(());
        }
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
                    let native = self
                        .current_alerts()
                        .is_some_and(|set| !set.actions.is_empty());
                    if pending.popup && items.is_empty() && !native {
                        self.status = "No quick fixes or intentions here".into();
                    }
                    self.code_actions.popup = pending.popup && (!items.is_empty() || native);
                    self.code_actions.list = Some(CodeActionList {
                        path: pending.path,
                        offset: pending.offset,
                        line: pending.line,
                        revision: pending.revision,
                        // What the server answered matches the diagnostics it
                        // has now: a publish that raced the request must not
                        // make the answer stale on arrival.
                        diagnostics: self.lsp.diagnostics_generation(),
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

fn command_kind(command: &ModCommand) -> &'static str {
    match command {
        ModCommand::CreateFile { .. } => "creating files",
        ModCommand::DeleteFile { .. } => "deleting files",
        ModCommand::MoveFile { .. } => "moving files",
        ModCommand::StartRename { .. } => "in-place rename",
        ModCommand::StartTemplate(_) => "live templates from fixes",
        ModCommand::ShowConflicts(_) => "conflict dialogs",
        ModCommand::Choose(_) => "choice menus",
        ModCommand::OpenUrl(_) => "opening URLs",
        ModCommand::CopyToClipboard(_) => "clipboard commands",
        ModCommand::Highlight { .. } => "highlight commands",
        ModCommand::RegisterTabOut { .. } => "tab-out",
        ModCommand::Navigate { .. } => "navigation to targets",
        _ => "this command",
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
            |state: &IdeState| state.active_document.is_some(),
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
            |state: &IdeState| state.active_document.is_some(),
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
            spec(
                "code.applyAlert",
                "Apply Context Action (merged list)",
                "Code",
                None,
                r#"{"index":0}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let index = usize_param(&params, "index")?;
                state.apply_alert(index).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "code.alerts",
                "List Quick Fixes and Intentions",
                "Code",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                state.poll_analysis();
                let set = state.current_alerts();
                Ok(json!({
                    "actions": set.as_ref().map(|s| s.actions.iter().map(|a| json!({
                        "text": a.text, "group": format!("{:?}", a.group), "weight": a.weight(),
                        "submenu": a.submenu, "source": format!("{:?}", a.source),
                    })).collect::<Vec<_>>()),
                    "bulb": set.as_ref().map(|s| format!("{:?}", s.bulb())),
                }))
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
