//! LSP-backed refactorings: Rename (Shift+F6), Reformat Code, Optimize
//! Imports, Extract Variable / Method / Constant and Inline.
//!
//! Every refactoring is asynchronous like the other language features: a
//! command sends the request and returns; [`IdeState::poll_refactor`] (called
//! from `poll_lsp`) finishes it. Results are applied through
//! [`IdeState::apply_workspace_edit`], so a rename that touches several files
//! is validated first and is one undo step per file.
//!
//! Rename asks the server for `textDocument/prepareRename`; servers without it
//! get a dialog for the identifier under the caret instead. The occurrences
//! of the symbol in the current file (`textDocument/references`) drive the
//! live preview while the new name is typed. Enter applies, Escape cancels.
use crate::code_actions::{CodeActionItem, CodeActionList, file_uri};
use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_lsp::{Feature, RequestHandle, parse_locations};
use archipel_ide_text::ops;
use serde::Serialize;
use serde_json::{Value, json};
use std::path::PathBuf;

/// Refactorings offered by the server as code actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Refactoring {
    OptimizeImports,
    ExtractVariable,
    ExtractMethod,
    ExtractConstant,
    Inline,
}

impl Refactoring {
    /// The `context.only` kind sent to the server.
    fn only(self) -> &'static str {
        match self {
            Self::OptimizeImports => "source.organizeImports",
            Self::ExtractVariable | Self::ExtractMethod | Self::ExtractConstant => {
                "refactor.extract"
            }
            Self::Inline => "refactor.inline",
        }
    }

    /// Code action kinds (prefixes) that are this refactoring.
    fn kinds(self) -> &'static [&'static str] {
        match self {
            Self::OptimizeImports => &["source.organizeImports"],
            Self::ExtractVariable => &["refactor.extract.variable", "refactor.extract.local"],
            Self::ExtractMethod => &["refactor.extract.function", "refactor.extract.method"],
            Self::ExtractConstant => &["refactor.extract.constant"],
            Self::Inline => &["refactor.inline"],
        }
    }

    /// Words in the title of servers that file every extraction under the
    /// bare `refactor.extract` kind (rust-analyzer).
    fn title_words(self) -> &'static [&'static str] {
        match self {
            Self::OptimizeImports => &[],
            Self::ExtractVariable => &["variable"],
            Self::ExtractMethod => &["function", "method"],
            Self::ExtractConstant => &["constant"],
            Self::Inline => &[],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::OptimizeImports => "Optimize Imports",
            Self::ExtractVariable => "Extract Variable",
            Self::ExtractMethod => "Extract Method",
            Self::ExtractConstant => "Extract Constant",
            Self::Inline => "Inline",
        }
    }

    fn matches(self, item: &CodeActionItem) -> bool {
        let Some(kind) = item.kind.as_deref() else {
            return false;
        };
        if self.kinds().iter().any(|prefix| kind.starts_with(prefix)) {
            return true;
        }
        kind == "refactor.extract" && {
            let title = item.title.to_lowercase();
            self.title_words().iter().any(|word| title.contains(word))
        }
    }

    fn command(self) -> &'static str {
        match self {
            Self::OptimizeImports => "refactor.optimizeImports",
            Self::ExtractVariable => "refactor.extractVariable",
            Self::ExtractMethod => "refactor.extractMethod",
            Self::ExtractConstant => "refactor.extractConstant",
            Self::Inline => "refactor.inline",
        }
    }
}

/// An in-place rename in progress.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameSession {
    pub path: PathBuf,
    /// The renamed identifier as character offsets of the current text.
    pub start: usize,
    pub end: usize,
    pub revision: u64,
    pub original: String,
    /// The name being typed.
    pub name: String,
    /// Occurrences in this file (including the identifier at the caret),
    /// as character ranges; the live preview shows the new name there.
    pub occurrences: Vec<(usize, usize)>,
    /// The server has no `prepareRename`: ask in a dialog instead of in place.
    pub dialog: bool,
    /// The server is applying the rename.
    pub applying: bool,
    pub error: Option<String>,
}

#[derive(Debug)]
enum Kind {
    Prepare,
    Occurrences,
    Rename { new_name: String },
    Format,
    Actions(Refactoring),
}

#[derive(Debug)]
struct Job {
    handle: RequestHandle,
    kind: Kind,
    path: PathBuf,
    offset: usize,
    revision: u64,
}

#[derive(Debug, Default)]
pub struct RefactorState {
    pub rename: Option<RenameSession>,
    jobs: Vec<Job>,
}

impl IdeState {
    fn active_supports(&self, feature: Feature) -> bool {
        self.active_absolute_path()
            .is_some_and(|path| self.lsp.supports(&path, feature))
    }

    fn lsp_range(&self, start: usize, end: usize) -> Result<Value, String> {
        let document = &self.documents[self.active_document.ok_or("no active document")?];
        let position = |offset| {
            document
                .text
                .lsp_position(offset)
                .map(|(line, character)| json!({"line": line, "character": character}))
                .map_err(|error| error.to_string())
        };
        Ok(json!({"start": position(start)?, "end": position(end)?}))
    }

    /// The `(path, absolute path, revision, selection start, selection end, caret)` of the editor.
    fn refactor_target(&self) -> Result<(PathBuf, PathBuf, u64, usize, usize, usize), String> {
        let index = self.active_document.ok_or("no active document")?;
        let absolute = self.active_absolute_path().ok_or("no project is open")?;
        let document = &self.documents[index];
        let selection = document.selections.primary();
        Ok((
            document.path.clone(),
            absolute,
            document.text.revision(),
            selection.start(),
            selection.end(),
            selection.head,
        ))
    }

    /// Starts Rename at the caret.
    pub fn start_rename(&mut self) -> Result<(), String> {
        let (path, absolute, revision, _, _, caret) = self.refactor_target()?;
        self.refactor.rename = None;
        if !self.lsp.supports(&absolute, Feature::Rename) {
            return Err("The language server does not support Rename".into());
        }
        if self.lsp.supports(&absolute, Feature::PrepareRename) {
            let (line, character) = self.caret_lsp(caret)?;
            let handle = self
                .lsp
                .request(
                    &absolute,
                    Feature::PrepareRename,
                    line,
                    character,
                    json!({}),
                )
                .map_err(|error| error.to_string())?;
            self.refactor.jobs.push(Job {
                handle,
                kind: Kind::Prepare,
                path,
                offset: caret,
                revision,
            });
            return Ok(());
        }
        // No prepareRename: the identifier under the caret, asked in a dialog.
        let document = &self.documents[self.active_document.ok_or("no active document")?];
        let (start, end) = ops::word_at(&document.text, caret);
        let session = self.new_rename_session(&path, revision, start, end, None, true)?;
        self.begin_rename_session(session)
    }

    fn caret_lsp(&self, offset: usize) -> Result<(usize, usize), String> {
        let document = &self.documents[self.active_document.ok_or("no active document")?];
        document
            .text
            .lsp_position(offset)
            .map_err(|error| error.to_string())
    }

    fn new_rename_session(
        &self,
        path: &std::path::Path,
        revision: u64,
        start: usize,
        end: usize,
        placeholder: Option<String>,
        dialog: bool,
    ) -> Result<RenameSession, String> {
        let document = self
            .documents
            .iter()
            .find(|document| document.path == path)
            .ok_or("the file is no longer open")?;
        let original = document
            .text
            .slice(start, end)
            .map_err(|error| error.to_string())?;
        if original.trim().is_empty() || original.chars().any(char::is_whitespace) {
            return Err("Place the caret on the name to rename".into());
        }
        Ok(RenameSession {
            path: path.to_owned(),
            start,
            end,
            revision,
            name: placeholder.unwrap_or_else(|| original.clone()),
            original,
            occurrences: vec![(start, end)],
            dialog,
            applying: false,
            error: None,
        })
    }

    fn begin_rename_session(&mut self, session: RenameSession) -> Result<(), String> {
        let absolute = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .join(&session.path);
        if self.lsp.supports(&absolute, Feature::References)
            && let Ok((line, character)) = self.caret_lsp(session.start)
            && let Ok(handle) = self.lsp.request(
                &absolute,
                Feature::References,
                line,
                character,
                json!({"context": {"includeDeclaration": true}}),
            )
        {
            self.refactor.jobs.push(Job {
                handle,
                kind: Kind::Occurrences,
                path: session.path.clone(),
                offset: session.start,
                revision: session.revision,
            });
        }
        self.status = format!("Rename {}", session.original);
        self.refactor.rename = Some(session);
        Ok(())
    }

    /// Updates the name typed in the in-place editor or dialog.
    pub fn set_rename_name(&mut self, name: &str) -> Result<(), String> {
        let session = self
            .refactor
            .rename
            .as_mut()
            .ok_or("no rename in progress")?;
        session.name = name.to_owned();
        session.error = None;
        Ok(())
    }

    /// Escape: abandons the rename without changing anything.
    pub fn cancel_rename(&mut self) {
        self.refactor.rename = None;
    }

    /// Enter: renames across the project through the server.
    pub fn apply_rename(&mut self) -> Result<(), String> {
        let session = self
            .refactor
            .rename
            .as_mut()
            .ok_or("no rename in progress")?;
        if session.applying {
            return Ok(());
        }
        let name = session.name.trim().to_owned();
        if name.is_empty() || name.chars().any(char::is_whitespace) {
            let message = format!("'{}' is not a valid name", session.name);
            session.error = Some(message.clone());
            return Err(message);
        }
        if name == session.original {
            self.refactor.rename = None;
            return Ok(());
        }
        let (path, start, revision) = (session.path.clone(), session.start, session.revision);
        let document = self
            .documents
            .iter()
            .find(|document| document.path == path)
            .ok_or("the file is no longer open")?;
        if document.text.revision() != revision {
            self.refactor.rename = None;
            return Err("The text changed during the rename; start it again".into());
        }
        let (line, character) = document
            .text
            .lsp_position(start)
            .map_err(|error| error.to_string())?;
        let absolute = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .join(&path);
        let handle = self
            .lsp
            .request(
                &absolute,
                Feature::Rename,
                line,
                character,
                json!({"newName": name}),
            )
            .map_err(|error| error.to_string())?;
        if let Some(session) = self.refactor.rename.as_mut() {
            session.applying = true;
        }
        self.refactor.jobs.push(Job {
            handle,
            kind: Kind::Rename { new_name: name },
            path,
            offset: start,
            revision,
        });
        Ok(())
    }

    /// Reformat Code: the selection through `rangeFormatting`, otherwise the file.
    pub fn reformat_code(&mut self) -> Result<(), String> {
        let (path, absolute, revision, start, end, caret) = self.refactor_target()?;
        let options = json!({"tabSize": 4, "insertSpaces": true});
        let uri = file_uri(&absolute)?;
        let (feature, params) = if start != end
            && self.lsp.supports(&absolute, Feature::RangeFormatting)
        {
            (
                Feature::RangeFormatting,
                json!({"textDocument": {"uri": uri}, "range": self.lsp_range(start, end)?, "options": options}),
            )
        } else if self.lsp.supports(&absolute, Feature::Formatting) {
            (
                Feature::Formatting,
                json!({"textDocument": {"uri": uri}, "options": options}),
            )
        } else {
            return Err("The language server does not support Reformat Code".into());
        };
        let handle = self
            .lsp
            .request_with_params(&absolute, feature, params)
            .map_err(|error| error.to_string())?;
        self.refactor.jobs.push(Job {
            handle,
            kind: Kind::Format,
            path,
            offset: caret,
            revision,
        });
        Ok(())
    }

    /// Optimize Imports, Extract Variable / Method / Constant and Inline:
    /// the matching code action of the server, applied directly when it is
    /// the only one and chosen from the intention popup otherwise.
    pub fn request_refactoring(&mut self, refactoring: Refactoring) -> Result<(), String> {
        let (path, absolute, revision, start, end, caret) = self.refactor_target()?;
        if !self.lsp.supports(&absolute, Feature::CodeAction) {
            return Err(format!(
                "The language server does not support {}",
                refactoring.label()
            ));
        }
        let params = json!({
            "textDocument": {"uri": file_uri(&absolute)?},
            "range": self.lsp_range(start, end)?,
            "context": {"diagnostics": [], "only": [refactoring.only()], "triggerKind": 1}
        });
        let handle = self
            .lsp
            .request_with_params(&absolute, Feature::CodeAction, params)
            .map_err(|error| error.to_string())?;
        self.refactor.jobs.push(Job {
            handle,
            kind: Kind::Actions(refactoring),
            path,
            offset: caret,
            revision,
        });
        Ok(())
    }

    /// Finishes refactoring requests whose answers arrived.
    pub fn poll_refactor(&mut self) {
        let mut jobs = std::mem::take(&mut self.refactor.jobs);
        let mut waiting = Vec::new();
        for job in jobs.drain(..) {
            match self.lsp.take_response(&job.handle) {
                None => waiting.push(job),
                Some(result) => {
                    let result = result.map_err(|error| error.to_string());
                    self.finish_job(job, result);
                }
            }
        }
        // Jobs queued while finishing others (e.g. none today) stay too.
        waiting.append(&mut self.refactor.jobs);
        self.refactor.jobs = waiting;
        // A rename is void once its text is gone or another file is shown.
        if let Some(session) = &self.refactor.rename {
            let valid = !session.applying
                && self
                    .active_document
                    .and_then(|index| self.documents.get(index))
                    .is_some_and(|document| {
                        document.path == session.path
                            && document.text.revision() == session.revision
                    });
            if !valid && !session.applying {
                self.refactor.rename = None;
            }
        }
    }

    fn finish_job(&mut self, job: Job, result: Result<Value, String>) {
        let current = self.documents.iter().find(|d| d.path == job.path);
        let fresh = current.is_some_and(|d| d.text.revision() == job.revision);
        match (job.kind, result) {
            (Kind::Prepare, Ok(value)) => {
                if fresh {
                    self.finish_prepare(&job.path, job.revision, job.offset, &value);
                }
            }
            (Kind::Prepare, Err(error)) => self.status = error,
            (Kind::Occurrences, Ok(value)) => {
                if fresh {
                    self.finish_occurrences(&value);
                }
            }
            (Kind::Occurrences, Err(_)) => {}
            (Kind::Rename { new_name }, result) => self.finish_rename(&new_name, result),
            (Kind::Format, Ok(value)) => self.finish_format(&job.path, &value),
            (Kind::Format, Err(error)) => self.status = error,
            (Kind::Actions(refactoring), Ok(value)) => {
                if fresh {
                    self.finish_refactoring(
                        refactoring,
                        &job.path,
                        job.offset,
                        job.revision,
                        &value,
                    );
                } else {
                    self.status = format!("{}: the text changed; try again", refactoring.label());
                }
            }
            (Kind::Actions(_), Err(error)) => self.status = error,
        }
    }

    /// Opens the rename session for a `prepareRename` answer.
    pub(crate) fn finish_prepare(
        &mut self,
        path: &std::path::Path,
        revision: u64,
        caret: usize,
        value: &Value,
    ) {
        let Some(document) = self.documents.iter().find(|d| d.path == path) else {
            return;
        };
        let range = value.get("range").unwrap_or(value);
        let located = match (range.get("start"), range.get("end")) {
            (Some(start), Some(end)) => {
                let offset = |position: &Value| {
                    let line = position.get("line")?.as_u64()? as usize;
                    let character = position.get("character")?.as_u64()? as usize;
                    Some(document.text.char_from_lsp_position(line, character))
                };
                offset(start).zip(offset(end))
            }
            _ if value.get("defaultBehavior").and_then(Value::as_bool) == Some(true) => {
                Some(ops::word_at(&document.text, caret))
            }
            _ => None,
        };
        let Some((start, end)) = located.filter(|(start, end)| start < end) else {
            self.status = "Cannot rename the element at the caret".into();
            return;
        };
        let placeholder = value
            .get("placeholder")
            .and_then(Value::as_str)
            .map(String::from);
        match self
            .new_rename_session(path, revision, start, end, placeholder, false)
            .and_then(|session| self.begin_rename_session(session))
        {
            Ok(()) => {}
            Err(error) => self.status = error,
        }
    }

    /// Fills the live preview from `textDocument/references`.
    pub(crate) fn finish_occurrences(&mut self, value: &Value) {
        let Some(session) = self.refactor.rename.as_ref() else {
            return;
        };
        let Some(document) = self.documents.iter().find(|d| d.path == session.path) else {
            return;
        };
        let Some(root) = self.project.as_ref().map(|p| p.root().to_owned()) else {
            return;
        };
        let absolute = root.join(&session.path);
        let canonical = absolute.canonicalize().unwrap_or(absolute);
        let mut ranges: Vec<(usize, usize)> = parse_locations(value)
            .into_iter()
            .filter(|location| {
                location.path == canonical
                    || location.path.canonicalize().is_ok_and(|p| p == canonical)
            })
            .filter_map(|location| {
                let start = document
                    .text
                    .char_from_lsp_position(location.start_line, location.start_character);
                let end = document
                    .text
                    .char_from_lsp_position(location.end_line, location.end_character);
                (start < end).then_some((start, end))
            })
            .collect();
        ranges.push((session.start, session.end));
        ranges.sort_unstable();
        ranges.dedup();
        if let Some(session) = self.refactor.rename.as_mut() {
            session.occurrences = ranges;
        }
    }

    pub(crate) fn finish_rename(&mut self, new_name: &str, result: Result<Value, String>) {
        let outcome = result.and_then(|edit| self.apply_workspace_edit(&edit));
        match outcome {
            Ok(summary) => {
                let original = self
                    .refactor
                    .rename
                    .take()
                    .map(|session| session.original)
                    .unwrap_or_default();
                self.status = match summary.files.len() {
                    0 => "Nothing to rename".into(),
                    1 => format!("Renamed {original} to {new_name}"),
                    count => format!("Renamed {original} to {new_name} in {count} files"),
                };
            }
            Err(error) => {
                // Keep the editor open so the name can be corrected.
                if let Some(session) = self.refactor.rename.as_mut() {
                    session.applying = false;
                    session.error = Some(error.clone());
                }
                self.status = error;
            }
        }
    }

    pub(crate) fn finish_format(&mut self, path: &std::path::Path, value: &Value) {
        let edits = value.as_array().cloned().unwrap_or_default();
        if edits.is_empty() {
            self.status = "Nothing to reformat".into();
            return;
        }
        let Some(root) = self.project.as_ref().map(|p| p.root().to_owned()) else {
            return;
        };
        let uri = match file_uri(&root.join(path)) {
            Ok(uri) => uri,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        self.status = match self.apply_workspace_edit(&json!({"changes": {uri: edits}})) {
            Ok(_) => "Reformatted the code".into(),
            Err(error) => error,
        };
    }

    pub(crate) fn finish_refactoring(
        &mut self,
        refactoring: Refactoring,
        path: &std::path::Path,
        caret: usize,
        revision: u64,
        value: &Value,
    ) {
        let items: Vec<CodeActionItem> = value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(CodeActionItem::parse)
            .filter(|item| refactoring.matches(item))
            .collect();
        let enabled: Vec<&CodeActionItem> = items
            .iter()
            .filter(|item| item.disabled.is_none())
            .collect();
        let message = match (enabled.len(), items.first()) {
            (0, Some(disabled)) => Some(format!(
                "{}: {}",
                refactoring.label(),
                disabled.disabled.as_deref().unwrap_or("not available here")
            )),
            (0, None) => Some(match refactoring {
                Refactoring::OptimizeImports => "No unused imports to optimize".into(),
                _ => format!("{} is not available here", refactoring.label()),
            }),
            _ => None,
        };
        if let Some(message) = message {
            self.status = message;
            return;
        }
        if enabled.len() == 1 || refactoring == Refactoring::OptimizeImports {
            let raw = enabled[0].raw.clone();
            if let Err(error) = self.run_code_action(&raw, path) {
                self.status = error;
            }
            return;
        }
        // Several variants (e.g. "replace all occurrences"): choose in the popup.
        self.code_actions.list = Some(CodeActionList {
            path: path.to_owned(),
            offset: caret,
            line: self
                .documents
                .iter()
                .find(|d| d.path == path)
                .map_or(0, |d| d.text.char_to_line(caret)),
            revision,
            diagnostics: self.lsp.diagnostics_generation(),
            items,
        });
        self.code_actions.popup = true;
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec("refactor.rename", "Rename…", "Refactor", Some("⇧F6"), "{}"),
            |state: &IdeState| state.active_supports(Feature::Rename),
            |state: &mut IdeState, _| {
                state.start_rename().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "refactor.rename.setName",
                "Set Rename Name",
                "Refactor",
                None,
                r#"{"name":"newName"}"#,
            ),
            |state: &IdeState| state.refactor.rename.is_some(),
            |state: &mut IdeState, params| {
                state
                    .set_rename_name(&string_param(&params, "name")?)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "refactor.rename.apply",
                "Apply Rename",
                "Refactor",
                None,
                "{}",
            ),
            |state: &IdeState| state.refactor.rename.is_some(),
            |state: &mut IdeState, _| {
                state.apply_rename().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "refactor.rename.cancel",
                "Cancel Rename",
                "Refactor",
                None,
                "{}",
            ),
            |state: &IdeState| state.refactor.rename.is_some(),
            |state: &mut IdeState, _| {
                state.cancel_rename();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "refactor.reformatCode",
                "Reformat Code",
                "Code",
                Some("⌥⌘L"),
                "{}",
            ),
            |state: &IdeState| {
                state.active_supports(Feature::Formatting)
                    || state.active_supports(Feature::RangeFormatting)
            },
            |state: &mut IdeState, _| {
                state.reformat_code().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "refactor.state",
                "Refactoring State",
                "Refactor",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| Ok(json!({"rename": state.refactor.rename})),
        ))
        .expect("unique built-in id");
    for (refactoring, label, category, shortcut) in [
        (
            Refactoring::OptimizeImports,
            "Optimize Imports",
            "Code",
            "⌃⌥O",
        ),
        (
            Refactoring::ExtractVariable,
            "Extract Variable…",
            "Refactor",
            "⌥⌘V",
        ),
        (
            Refactoring::ExtractMethod,
            "Extract Method…",
            "Refactor",
            "⌥⌘M",
        ),
        (
            Refactoring::ExtractConstant,
            "Extract Constant…",
            "Refactor",
            "⌥⌘C",
        ),
        (Refactoring::Inline, "Inline…", "Refactor", "⌥⌘N"),
    ] {
        registry
            .register(Command::new(
                spec(refactoring.command(), label, category, Some(shortcut), "{}"),
                |state: &IdeState| state.active_supports(Feature::CodeAction),
                move |state: &mut IdeState, _| {
                    state
                        .request_refactoring(refactoring)
                        .map_err(CommandError::Failed)?;
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn project(name: &str) -> (IdeState, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-refactor-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "let 𝔸 = old;\nold();\n").unwrap();
        std::fs::write(root.join("b.txt"), "use old;\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "a.txt"}))
            .unwrap();
        (state, root.canonicalize().unwrap())
    }

    fn range(line: u64, start: u64, end: u64) -> Value {
        json!({"start": {"line": line, "character": start}, "end": {"line": line, "character": end}})
    }

    fn uri(root: &std::path::Path, name: &str) -> String {
        format!("file://{}", root.join(name).display())
    }

    #[test]
    fn rename_previews_occurrences_and_edits_several_files_with_undo() {
        let (mut state, root) = project("rename");
        let revision = state.documents[0].text.revision();
        // "old" of the declaration: UTF-16 column 9 (𝔸 is two units), char 8.
        state.finish_prepare(
            "a.txt".as_ref(),
            revision,
            9,
            &json!({"range": range(0, 9, 12), "placeholder": "old"}),
        );
        let session = state.refactor.rename.clone().expect("session opens");
        assert_eq!((session.start, session.end), (8, 11));
        assert_eq!(session.name, "old");
        assert!(!session.dialog);
        // Live preview: both occurrences in this file; the reference in b.txt is not shown.
        let location = |name: &str, line, start, end| json!({"uri": uri(&root, name), "range": range(line, start, end)});
        state.finish_occurrences(&json!([
            location("a.txt", 0, 9, 12),
            location("a.txt", 1, 0, 3),
            location("b.txt", 0, 4, 7),
        ]));
        assert_eq!(
            state.refactor.rename.as_ref().unwrap().occurrences,
            vec![(8, 11), (13, 16)]
        );
        let commands = built_in_commands();
        commands
            .execute(
                "refactor.rename.setName",
                &mut state,
                json!({"name": "has space"}),
            )
            .unwrap();
        let error = commands
            .execute("refactor.rename.apply", &mut state, json!({}))
            .unwrap_err()
            .to_string();
        assert!(error.contains("not a valid name"), "{error}");
        assert!(state.refactor.rename.is_some(), "the editor stays open");
        // The server's answer renames in both files.
        state.refactor.rename.as_mut().unwrap().applying = true;
        state.finish_rename(
            "newer",
            Ok(json!({"changes": {
                uri(&root, "a.txt"): [{"range": range(0, 9, 12), "newText": "newer"},
                                      {"range": range(1, 0, 3), "newText": "newer"}],
                uri(&root, "b.txt"): [{"range": range(0, 4, 7), "newText": "newer"}]}})),
        );
        assert!(state.refactor.rename.is_none());
        assert_eq!(state.status, "Renamed old to newer in 2 files");
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = newer;\nnewer();\n");
        assert_eq!(state.documents[1].text.text(), "use newer;\n");
        // One undo step per file restores everything.
        for document in &mut state.documents {
            assert!(document.text.undo());
        }
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = old;\nold();\n");
        assert_eq!(state.documents[1].text.text(), "use old;\n");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rename_that_the_server_declines_or_fails_changes_nothing() {
        let (mut state, root) = project("decline");
        let revision = state.documents[0].text.revision();
        state.finish_prepare("a.txt".as_ref(), revision, 0, &Value::Null);
        assert!(state.refactor.rename.is_none());
        assert_eq!(state.status, "Cannot rename the element at the caret");
        // `defaultBehavior` uses the word at the caret.
        state.finish_prepare(
            "a.txt".as_ref(),
            revision,
            9,
            &json!({"defaultBehavior": true}),
        );
        assert_eq!(state.refactor.rename.as_ref().unwrap().original, "old");
        // A failing rename keeps the session open with the error shown.
        state.refactor.rename.as_mut().unwrap().applying = true;
        state.finish_rename("x", Err("cannot rename here".into()));
        let session = state.refactor.rename.as_ref().unwrap();
        assert!(!session.applying);
        assert_eq!(session.error.as_deref(), Some("cannot rename here"));
        // Escape cancels.
        built_in_commands()
            .execute("refactor.rename.cancel", &mut state, json!({}))
            .unwrap();
        assert!(state.refactor.rename.is_none());
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = old;\nold();\n");
        // Without a server the command is disabled (no dialog to fall back on).
        assert!(!built_in_commands().is_enabled("refactor.rename", &state));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn formatting_edits_apply_and_report() {
        let (mut state, root) = project("format");
        state.finish_format(
            "a.txt".as_ref(),
            &json!([{"range": range(1, 0, 3), "newText": "NEW"}]),
        );
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = old;\nNEW();\n");
        assert_eq!(state.status, "Reformatted the code");
        state.finish_format("a.txt".as_ref(), &json!([]));
        assert_eq!(state.status, "Nothing to reformat");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refactoring_actions_apply_alone_and_open_the_popup_when_ambiguous() {
        let (mut state, root) = project("extract");
        let revision = state.documents[0].text.revision();
        let a = uri(&root, "a.txt");
        let edit = |text: &str| json!({"changes": {a.clone(): [{"range": range(1, 0, 3), "newText": text}]}});
        // Other kinds and quick fixes are ignored; one match applies at once.
        let single = json!([
            {"title": "Extract to method", "kind": "refactor.extract.function", "edit": edit("mmm")},
            {"title": "Fix it", "kind": "quickfix", "edit": edit("fix")},
        ]);
        state.finish_refactoring(
            Refactoring::ExtractVariable,
            "a.txt".as_ref(),
            0,
            revision,
            &single,
        );
        assert_eq!(state.status, "Extract Variable is not available here");
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = old;\nold();\n");
        state.finish_refactoring(
            Refactoring::ExtractMethod,
            "a.txt".as_ref(),
            0,
            revision,
            &single,
        );
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = old;\nmmm();\n");
        // Servers that file extractions under the bare kind are told apart by title.
        let revision = state.documents[0].text.revision();
        let bare = json!([
            {"title": "Extract into variable", "kind": "refactor.extract", "edit": edit("vvv")},
            {"title": "Extract into function", "kind": "refactor.extract", "edit": edit("fff")},
        ]);
        state.finish_refactoring(
            Refactoring::ExtractConstant,
            "a.txt".as_ref(),
            0,
            revision,
            &bare,
        );
        assert_eq!(state.status, "Extract Constant is not available here");
        state.finish_refactoring(
            Refactoring::ExtractVariable,
            "a.txt".as_ref(),
            0,
            revision,
            &bare,
        );
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = old;\nvvv();\n");
        // Several variants: the intention popup lets the user choose.
        let revision = state.documents[0].text.revision();
        let several = json!([
            {"title": "Extract to local variable (replace all)", "kind": "refactor.extract.variable", "edit": edit("aaa")},
            {"title": "Extract to local variable", "kind": "refactor.extract.variable", "edit": edit("bbb")},
        ]);
        state.finish_refactoring(
            Refactoring::ExtractVariable,
            "a.txt".as_ref(),
            0,
            revision,
            &several,
        );
        assert!(state.code_actions.popup);
        assert_eq!(state.current_code_actions().unwrap().items.len(), 2);
        built_in_commands()
            .execute("code.applyAction", &mut state, json!({"index": 1}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = old;\nbbb();\n");
        // A disabled action explains itself; Optimize Imports needs no choice.
        let revision = state.documents[0].text.revision();
        state.finish_refactoring(
            Refactoring::Inline,
            "a.txt".as_ref(),
            0,
            revision,
            &json!([{"title": "Inline", "kind": "refactor.inline", "disabled": {"reason": "Select a variable"}}]),
        );
        assert_eq!(state.status, "Inline: Select a variable");
        state.finish_refactoring(
            Refactoring::OptimizeImports,
            "a.txt".as_ref(),
            0,
            revision,
            &json!([{"title": "Organize Imports", "kind": "source.organizeImports", "edit": edit("ooo")}]),
        );
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = old;\nooo();\n");
        state.finish_refactoring(
            Refactoring::OptimizeImports,
            "a.txt".as_ref(),
            0,
            revision,
            &json!([]),
        );
        assert_eq!(state.status, "No unused imports to optimize");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refactor_commands_are_registered_with_shortcuts() {
        let commands = built_in_commands();
        let specs = commands.specs();
        for (id, shortcut) in [
            ("refactor.rename", "⇧F6"),
            ("refactor.reformatCode", "⌥⌘L"),
            ("refactor.optimizeImports", "⌃⌥O"),
            ("refactor.extractVariable", "⌥⌘V"),
            ("refactor.extractMethod", "⌥⌘M"),
            ("refactor.extractConstant", "⌥⌘C"),
            ("refactor.inline", "⌥⌘N"),
        ] {
            let spec = specs.iter().find(|spec| spec.id == id).expect(id);
            assert_eq!(spec.shortcut.as_deref(), Some(shortcut), "{id}");
        }
    }
}
