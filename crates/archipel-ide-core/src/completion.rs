//! Code completion session: requests, client-side filtering as you type,
//! auto-popup rules and acceptance (Enter inserts, Tab replaces).
//!
//! The server is asked once per session; typing more identifier characters
//! only re-filters locally unless the server marked its list `isIncomplete`.

use crate::IdeState;
use crate::completion_native::{NativeSession, merge_items};
use archipel_ide_lsp::{
    CompletionResult, CompletionTrigger,
    completion::{self as ranking, RankedItem, is_identifier_char},
    lsp_types::{CompletionItem, CompletionTextEdit, InsertTextFormat, TextEdit},
    snippet::{self, Snippet},
};
use archipel_ide_text::{Selection, TextChange, TextDocument};
use std::collections::HashSet;
use std::path::PathBuf;

/// Trigger characters that are better served by parameter info or that
/// would pop up while typing ordinary text.
const IGNORED_TRIGGERS: &[&str] = &["(", ",", " ", "'", "\"", "=", "\n", "\t"];

#[derive(Debug, Clone)]
pub struct CompletionSession {
    /// Project-relative path of the document being completed.
    pub path: PathBuf,
    /// Char offset where the identifier being completed starts.
    pub anchor: usize,
    /// Caret offset the last request was made at.
    pub request_caret: usize,
    /// Items from the last response, in server order.
    pub items: Vec<CompletionItem>,
    /// Visible rows for the current prefix, best first.
    pub ranked: Vec<RankedItem>,
    /// Index into `ranked`.
    pub selected: usize,
    /// Typed text between `anchor` and the caret.
    pub prefix: String,
    /// The server said more items exist; typing re-queries.
    pub incomplete: bool,
    /// A request is in flight.
    pub waiting: bool,
    /// Opened with Ctrl+Space: an empty result is reported, not hidden.
    pub explicit: bool,
    /// Item indices already passed through `completionItem/resolve`.
    pub resolved: HashSet<usize>,
    /// What the language server was still doing (importing, indexing) when
    /// it answered: results may be incomplete, and the popup says so.
    pub indexing: Option<String>,
    /// Ask again once the server has finished `indexing`.
    pub requery_when_ready: bool,
    /// The native source of the language answered too (class names with import on insert).
    pub native: Option<NativeSession>,
    /// Why the native answer may be incomplete (indexing, a classpath that failed to import).
    pub note: Option<String>,
    /// Consecutive explicit invocations on this prefix (1: first press, CI-CMP-100).
    pub invocation: u32,
}

impl CompletionSession {
    pub fn selected_item(&self) -> Option<&CompletionItem> {
        self.items.get(self.ranked.get(self.selected)?.index)
    }

    /// Whether the popup has rows to show.
    pub fn visible(&self) -> bool {
        !self.ranked.is_empty()
    }

    /// Whether the popup shows only the "Indexing…" notice (no rows yet).
    pub fn indexing_only(&self) -> bool {
        self.ranked.is_empty() && self.indexing.is_some()
    }
}

/// Start of the identifier that ends at `caret`.
fn identifier_start(text: &TextDocument, caret: usize) -> usize {
    let mut start = caret;
    while start > 0 && text.char_at(start - 1).is_some_and(is_identifier_char) {
        start -= 1;
    }
    start
}

/// End of the identifier that continues after `caret` (Tab replaces it).
fn identifier_end(text: &TextDocument, caret: usize) -> usize {
    let mut end = caret;
    while text.char_at(end).is_some_and(is_identifier_char) {
        end += 1;
    }
    end
}

impl IdeState {
    /// Requests completion at the caret of the active document.
    pub fn start_completion(
        &mut self,
        trigger: CompletionTrigger,
        explicit: bool,
    ) -> Result<u64, String> {
        let index = self.active_document.ok_or("no active document")?;
        let root = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .to_owned();
        let document = self
            .documents
            .get(index)
            .ok_or("active document does not exist")?;
        let caret = document.selections.primary().head;
        let anchor = identifier_start(&document.text, caret);
        let (line, character) = document
            .text
            .lsp_position(caret)
            .map_err(|error| error.to_string())?;
        let path = document.path.clone();
        let prefix = document
            .text
            .slice(anchor, caret)
            .map_err(|error| error.to_string())?;
        let absolute = root.join(&path);
        let invocation = self
            .completion
            .as_ref()
            .filter(|session| explicit && session.path == path && session.anchor == anchor)
            .map_or(1, |session| session.invocation + 1);
        // The language's own source first (class names from the indexes): at a position where
        // only class names are legal the server is not asked at all.
        let source = self.completion_source_for_active();
        let answer = source
            .as_ref()
            .and_then(|source| self.native_answer(source.as_ref(), invocation));
        let exclusive = answer.as_ref().is_some_and(|answer| answer.exclusive);
        let (id, indexing) = if exclusive {
            (None, None)
        } else {
            match self
                .lsp
                .request_completion(&absolute, line, character, trigger)
            {
                Ok(id) => (Some(id), self.lsp.indexing_status(&absolute)),
                // The server is still starting: say so, and ask once it is ready.
                Err(archipel_ide_lsp::LspError::Starting { server, detail }) => (
                    None,
                    Some(detail.unwrap_or_else(|| format!("{server}: starting"))),
                ),
                // No language server for this file: native rows and live templates still apply.
                Err(error) => {
                    // Ctrl+Space on nothing explains the missing server; a typed
                    // abbreviation still expands.
                    if answer.is_none()
                        && (self.template_completion_items(anchor).is_empty()
                            || (explicit && anchor == caret))
                    {
                        return Err(error.to_string());
                    }
                    if explicit && answer.is_none() {
                        self.status = error.to_string();
                    }
                    (None, None)
                }
            }
        };
        let offline = id.is_none() && indexing.is_none();
        // Keep the visible rows while an `isIncomplete` re-query is running.
        let previous = self
            .completion
            .take()
            .filter(|session| session.path == path && session.anchor == anchor);
        let native = source
            .as_ref()
            .zip(answer.as_ref())
            .map(|(source, answer)| {
                let rows = Self::native_rows(answer);
                NativeSession {
                    source: source.id(),
                    exclusive: answer.exclusive,
                    server_items: previous
                        .as_ref()
                        .map(|session| match &session.native {
                            Some(native) => native.server_items.clone(),
                            None => session.items.clone(),
                        })
                        .unwrap_or_default(),
                    rows,
                    computed_for: (
                        caret,
                        self.documents
                            .get(index)
                            .map_or(0, |document| document.text.len_chars()),
                        prefix.clone(),
                    ),
                }
            });
        let items = match &native {
            Some(native) => merge_items(&native.rows, &native.server_items),
            None => previous
                .as_ref()
                .map(|session| session.items.clone())
                .unwrap_or_default(),
        };
        let note = answer.as_ref().and_then(|answer| answer.note.clone());
        self.completion = Some(CompletionSession {
            path,
            anchor,
            request_caret: caret,
            items,
            ranked: if native.is_some() {
                Vec::new()
            } else {
                previous
                    .as_ref()
                    .map(|session| session.ranked.clone())
                    .unwrap_or_default()
            },
            selected: 0,
            prefix,
            incomplete: false,
            waiting: id.is_some(),
            explicit: explicit || previous.is_some_and(|session| session.explicit),
            resolved: HashSet::new(),
            requery_when_ready: id.is_none() && !offline,
            indexing,
            native,
            note,
            invocation,
        });
        if self
            .completion
            .as_ref()
            .is_some_and(|session| session.native.is_some() && !offline)
        {
            self.refresh_completion();
        }
        if offline {
            self.receive_completion(CompletionResult::default());
        }
        Ok(id.unwrap_or(0))
    }

    /// Closes the popup and cancels its request.
    pub fn close_completion(&mut self) {
        if self.completion.take().is_some() {
            self.lsp.cancel_completion();
        }
    }

    /// Collects a completion response and keeps the session in sync with the caret.
    pub(crate) fn poll_completion(&mut self) {
        if self.completion.is_none() {
            // Drop a result that arrived for a session that was closed.
            let _ = self.lsp.take_completion();
            return;
        }
        if let Some(result) = self.lsp.take_completion() {
            self.receive_completion(result);
        } else {
            self.refresh_completion();
        }
        self.requery_completion_when_ready();
    }

    /// Re-queries a popup that was answered (or not sent) while the server was
    /// still importing or indexing, once the server has finished.
    fn requery_completion_when_ready(&mut self) {
        let Some(root) = self.project.as_ref().map(|p| p.root().to_owned()) else {
            return;
        };
        let Some(session) = self.completion.as_mut() else {
            return;
        };
        if session.indexing.is_none() || session.waiting {
            return;
        }
        let absolute = root.join(&session.path);
        // The footer follows the server's current activity.
        let status = self.lsp.indexing_status(&absolute);
        if status.is_some() || !session.requery_when_ready {
            session.indexing = status;
            if session.indexing_only() && session.indexing.is_none() && !session.explicit {
                self.close_completion();
            }
            return;
        }
        if !self.lsp.is_idle_for(&absolute) {
            return;
        }
        let explicit = session.explicit;
        if let Err(error) = self.start_completion(CompletionTrigger::Invoked, explicit) {
            self.status = error;
            self.close_completion();
        }
    }

    /// Installs a server response into the open session and filters it.
    pub fn receive_completion(&mut self, result: CompletionResult) {
        // Where the native list is complete the server's notices and the templates do not apply.
        let exclusive = self
            .completion
            .as_ref()
            .and_then(|session| session.native.as_ref())
            .is_some_and(|native| native.exclusive);
        let indexing = self.completion.as_ref().and_then(|session| {
            let root = self.project.as_ref()?.root();
            self.lsp
                .indexing_status(&root.join(&session.path))
                .filter(|_| !exclusive)
        });
        // Live templates and postfix completion join the server's items; a
        // server snippet with the same label (`sout`) yields to ours.
        let templates = self
            .completion
            .as_ref()
            .filter(|_| !exclusive)
            .map(|session| self.template_completion_items(session.anchor))
            .unwrap_or_default();
        let server_empty = result.items.is_empty();
        let mut result = result;
        result.items.retain(|item| {
            item.kind != Some(archipel_ide_lsp::lsp_types::CompletionItemKind::SNIPPET)
                || !templates
                    .iter()
                    .any(|template| template.label == item.label)
        });
        result.items.extend(templates);
        if let Some(session) = self.completion.as_mut() {
            session.items = match session.native.as_mut() {
                Some(native) => {
                    native.server_items = result.items;
                    merge_items(&native.rows, &native.server_items)
                }
                None => result.items,
            };
            session.incomplete = result.incomplete;
            session.waiting = false;
            session.selected = 0;
            session.ranked.clear();
            session.resolved.clear();
            // Answers given while the server imports the project can be empty:
            // keep the popup with a notice and ask again when it is done.
            session.requery_when_ready = indexing.is_some() && session.items.is_empty();
            session.indexing = indexing;
        }
        self.refresh_completion();
        if server_empty
            && let Some(session) = self.completion.as_ref()
            && session.explicit
            && session.indexing.is_none()
            && session.ranked.is_empty()
        {
            self.status = "No suggestions".into();
        }
        // The server may ask for an item to be selected first (`preselect`).
        if let Some(session) = self.completion.as_mut()
            && let Some(row) = session
                .ranked
                .iter()
                .position(|row| session.items[row.index].preselect == Some(true))
        {
            session.selected = row;
        }
    }

    /// Re-filters after the document or caret changed; closes the session
    /// when the caret left the identifier being completed.
    pub fn refresh_completion(&mut self) {
        self.refresh_native_items();
        let Some(session) = self.completion.as_ref() else {
            return;
        };
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .filter(|document| document.path == session.path)
        else {
            self.close_completion();
            return;
        };
        let caret = document.selections.primary().head;
        let prefix = if caret >= session.anchor && document.selections.primary().is_caret() {
            document.text.slice(session.anchor, caret).ok()
        } else {
            None
        };
        let Some(prefix) = prefix.filter(|prefix| prefix.chars().all(is_identifier_char)) else {
            self.close_completion();
            return;
        };
        let Some(root) = self
            .project
            .as_ref()
            .map(|project| project.root().to_owned())
        else {
            self.close_completion();
            return;
        };
        let absolute = root.join(&session.path);
        let previous = session
            .selected_item()
            .map(|item| item.label.clone())
            .filter(|_| prefix == session.prefix);
        let language = self.lsp.language_id(&absolute).unwrap_or("text");
        let stats = &self.completion_stats;
        let ranked = self
            .lsp
            .rank_completions(&absolute, &session.items, &prefix, |item| {
                stats.weight(language, item)
            });
        let close = !session.waiting
            && ranked.is_empty()
            && !session.explicit
            && session.indexing.is_none();
        let Some(session) = self.completion.as_mut() else {
            return;
        };
        session.selected = previous
            .and_then(|label| {
                ranked
                    .iter()
                    .position(|row| session.items[row.index].label == label)
            })
            .unwrap_or(0);
        session.ranked = ranked;
        session.prefix = prefix;
        if close {
            self.close_completion();
        }
    }

    /// Updates completion after the user typed `typed` into the active
    /// document: re-filters, re-queries incomplete lists and opens the popup
    /// automatically on identifier starts and trigger characters.
    pub fn completion_after_typing(&mut self, typed: &str) {
        if typed.is_empty() {
            self.refresh_completion();
            return;
        }
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            return;
        };
        let caret = document.selections.primary().head;
        if self.commits_on_typed(typed) {
            // The commit character is typed after the item: take it out,
            // accept the selection and put it back behind the completion.
            let index = self.active_document.unwrap_or_default();
            if let Some(document) = self.documents.get_mut(index)
                && caret >= typed.chars().count()
                && let start = caret - typed.chars().count()
                && document.text.edit(start, caret, "").is_ok()
            {
                document.selections.set_single(Selection::caret(start));
                if self.accept_completion(false).is_ok() {
                    if let Some(document) = self.documents.get_mut(index) {
                        let at = document.selections.primary().head;
                        let _ = document.text.edit(at, at, typed);
                        document
                            .selections
                            .set_single(Selection::caret(at + typed.chars().count()));
                    }
                    self.sync_active_lsp();
                    return self.completion_after_typing_resumed(typed);
                }
                // Not accepted: restore the typed character.
                if let Some(document) = self.documents.get_mut(index) {
                    let _ = document.text.edit(start, start, typed);
                    document
                        .selections
                        .set_single(Selection::caret(start + typed.chars().count()));
                }
            }
        }
        self.completion_after_typing_resumed(typed);
    }

    /// Whether `typed` is a commit character of the selected completion.
    fn commits_on_typed(&self, typed: &str) -> bool {
        let mut chars = typed.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return false;
        };
        if is_identifier_char(c) || c.is_whitespace() {
            return false;
        }
        let Some(session) = self.completion.as_ref() else {
            return false;
        };
        let Some(item) = session.selected_item().filter(|_| !session.waiting) else {
            return false;
        };
        let typed = typed.to_owned();
        match &item.commit_characters {
            Some(chars) => chars.contains(&typed),
            None => {
                let Some(root) = self
                    .project
                    .as_ref()
                    .map(|project| project.root().to_owned())
                else {
                    return false;
                };
                self.lsp
                    .all_commit_characters(&root.join(&session.path))
                    .contains(&typed)
            }
        }
    }

    fn completion_after_typing_resumed(&mut self, typed: &str) {
        let Some(last) = typed.chars().last() else {
            return;
        };
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            return;
        };
        let caret = document.selections.primary().head;
        if self.completion.is_some() {
            self.refresh_completion();
            if let Some(session) = &self.completion {
                if session.incomplete && !session.waiting {
                    let _ = self.start_completion(CompletionTrigger::Incomplete, false);
                }
                return;
            }
        }
        if self.caret_in_comment_or_string(caret) {
            return;
        }
        // The language opens the popup by itself after `@` and after `new ` (CI-CMP-030).
        if self.native_auto_popup(caret) {
            let _ = self.start_completion(CompletionTrigger::Invoked, false);
            return;
        }
        let trigger = self.trigger_before_caret(caret);
        if let Some(trigger) = trigger {
            let _ = self.start_completion(CompletionTrigger::Character(trigger), false);
            return;
        }
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            return;
        };
        // Inside a template variable the popup only opens on request or on
        // trigger characters, so Tab keeps moving between variables.
        let starts_word = self.snippet.is_none()
            && is_identifier_char(last)
            && !last.is_ascii_digit()
            && identifier_start(&document.text, caret) + 1 == caret;
        if starts_word {
            let _ = self.start_completion(CompletionTrigger::Invoked, false);
        }
    }

    /// The server trigger sequence that ends at `caret`, if any (`.`, `::`).
    fn trigger_before_caret(&self, caret: usize) -> Option<String> {
        let document = self.documents.get(self.active_document?)?;
        let root = self.project.as_ref()?.root();
        let mut triggers = self.lsp.trigger_characters(&root.join(&document.path));
        if triggers.is_empty() {
            return None;
        }
        // rust-analyzer registers ":"; the popup should open on "::" only.
        if triggers.iter().any(|t| t == ":") {
            triggers.retain(|t| t != ":");
            triggers.push("::".into());
        }
        triggers
            .into_iter()
            .filter(|trigger| !IGNORED_TRIGGERS.contains(&trigger.as_str()))
            .find(|trigger| {
                let count = trigger.chars().count();
                caret >= count
                    && document
                        .text
                        .slice(caret - count, caret)
                        .is_ok_and(|text| &text == trigger)
            })
    }

    fn caret_in_comment_or_string(&self, caret: usize) -> bool {
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            return false;
        };
        let text = document.text.text();
        let byte = text
            .char_indices()
            .nth(caret)
            .map_or(text.len(), |(byte, _)| byte);
        // Highlight keys are IntelliJ text attribute keys
        // (`DEFAULT_LINE_COMMENT`, `DEFAULT_STRING`, ...).
        document
            .analysis
            .highlights
            .in_range(byte.saturating_sub(1)..byte + 1)
            .any(|span| {
                (span.key.contains("COMMENT") || span.key.contains("STRING"))
                    && span.start_byte < byte
                    && byte < span.end_byte
            })
    }

    /// Moves the popup selection by `delta` rows, wrapping around.
    pub fn move_completion_selection(&mut self, delta: isize) {
        if let Some(session) = self.completion.as_mut()
            && !session.ranked.is_empty()
        {
            let count = session.ranked.len() as isize;
            session.selected = (session.selected as isize + delta).rem_euclid(count) as usize;
        }
    }

    /// Selects a row; out-of-range indices are ignored.
    pub fn select_completion(&mut self, row: usize) {
        if let Some(session) = self.completion.as_mut()
            && row < session.ranked.len()
        {
            session.selected = row;
        }
    }

    /// Accepts the selected item. `replace` (Tab) also overwrites the rest of
    /// the identifier after the caret; otherwise (Enter) the text is inserted.
    pub fn accept_completion(&mut self, replace: bool) -> Result<(), String> {
        let session = self.completion.take().ok_or("no completion is showing")?;
        self.lsp.cancel_completion();
        let item = session
            .selected_item()
            .cloned()
            .ok_or("no completion item is selected")?;
        let resolved = session
            .ranked
            .get(session.selected)
            .is_some_and(|row| session.resolved.contains(&row.index));
        if let Some(language) = self
            .active_absolute_path()
            .and_then(|path| self.lsp.language_id(&path))
        {
            self.completion_stats.record(language, &item);
        }
        let index = self.active_document.ok_or("no active document")?;
        // A native row: the import and the text (or the qualified name on a clash) are
        // computed now, against the text as it is.
        let native_insertion = self.native_insertion(&session, &item);
        let document = self
            .documents
            .get_mut(index)
            .ok_or("active document does not exist")?;
        if document.path != session.path {
            return Err("the completed document is no longer active".into());
        }
        let caret = document.selections.primary().head;
        let snippet_format = item.insert_text_format == Some(InsertTextFormat::SNIPPET)
            || native_insertion
                .as_ref()
                .is_some_and(|(_, _, snippet)| *snippet);
        let (start, new_text) = match &item.text_edit {
            Some(CompletionTextEdit::Edit(edit)) => (
                document.text.char_from_lsp_position(
                    edit.range.start.line as usize,
                    edit.range.start.character as usize,
                ),
                edit.new_text.clone(),
            ),
            Some(CompletionTextEdit::InsertAndReplace(edit)) => (
                document.text.char_from_lsp_position(
                    edit.insert.start.line as usize,
                    edit.insert.start.character as usize,
                ),
                edit.new_text.clone(),
            ),
            None => (session.anchor, ranking::insert_text(&item).to_owned()),
        };
        let new_text = match &native_insertion {
            Some((text, _, _)) => text.clone(),
            None => new_text,
        };
        let start = start.min(caret);
        let file_name = document
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        // Multi-line insertions follow the indentation of the current line.
        let new_text = if new_text.contains('\n') {
            let line = document.text.char_to_line(start);
            let indent: String = document
                .text
                .line(line)
                .unwrap_or_default()
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect();
            adjust_indentation(&new_text, &indent)
        } else {
            new_text
        };
        // A call without arguments for a method that takes parameters gets
        // one tab stop per parameter, like the reference IDE's argument
        // placeholders (`greeting()` -> `greeting(name)` with `name` selected).
        // (A native row says itself where the caret goes: its snippet is used as it is.)
        let (new_text, snippet_format) = match parameter_snippet(&item, &new_text, snippet_format)
            .filter(|_| native_insertion.is_none())
        {
            Some(snippet) => (snippet, true),
            None => (new_text, snippet_format),
        };
        let snippet = snippet_format.then(|| {
            let variables = |name: &str| match name {
                "TM_FILENAME" => file_name.clone(),
                _ => None,
            };
            snippet::parse(&new_text, &variables)
        });
        let new_text = snippet
            .as_ref()
            .map_or(new_text, |snippet| snippet.text.clone());
        let end = if replace && !crate::live_templates::is_template_item(&item) {
            identifier_end(&document.text, caret)
        } else {
            caret
        };
        let mut changes = vec![TextChange {
            start_char: start,
            end_char: end,
            inserted: new_text.clone(),
        }];
        let mut shift: isize = 0;
        for edit in item.additional_text_edits.iter().flatten() {
            let change = lsp_change(&document.text, edit);
            if change.end_char <= start {
                shift += change.inserted.chars().count() as isize
                    - (change.end_char - change.start_char) as isize;
                changes.push(change);
            } else if change.start_char >= end {
                changes.push(change);
            }
        }
        for change in native_insertion.iter().flat_map(|(_, changes, _)| changes) {
            if change.end_char <= start {
                shift += change.inserted.chars().count() as isize
                    - (change.end_char - change.start_char) as isize;
                changes.push(change.clone());
            } else if change.start_char >= end {
                changes.push(change.clone());
            }
        }
        if document.text.edit_many(changes.clone()).is_err() {
            // Overlapping auxiliary edits: apply the completion itself.
            shift = 0;
            document
                .text
                .edit(start, end, &new_text)
                .map_err(|error| error.to_string())?;
        }
        let inserted_at = start.saturating_add_signed(shift);
        let caret = inserted_at
            + snippet
                .as_ref()
                .map_or(new_text.chars().count(), Snippet::final_caret);
        document.selections.set_single(Selection::caret(caret));
        let source = document.text.text();
        document.refresh_analysis(&source);
        match snippet.filter(Snippet::is_interactive) {
            Some(snippet) => self.start_snippet(&snippet, inserted_at),
            // Completing inside a template variable keeps the template.
            None => self.snippet_after_change(),
        }
        if !resolved && item.additional_text_edits.is_none() && native_insertion.is_none() {
            self.resolve_accepted(item, inserted_at);
        }
        // Completing a call leaves the caret inside `(`: show parameter info,
        // as the reference IDE does.
        let after_paren = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .and_then(|document| {
                let start = document.selections.primary().start();
                start
                    .checked_sub(1)
                    .and_then(|before| document.text.char_at(before))
            })
            == Some('(');
        if after_paren {
            let _ = self.request_parameter_info(None, false);
        }
        self.sync_active_lsp();
        Ok(())
    }
}

/// Parameter names of a method/function/constructor item, from its label
/// details (`(String name, int count)`) or its label
/// (`greeting(String name) : String`). Each name is the last identifier of
/// the parameter (`String name` -> `name`, `name: &str` -> `name`).
fn parameter_names(item: &CompletionItem) -> Vec<String> {
    use archipel_ide_lsp::lsp_types::CompletionItemKind;
    if !matches!(
        item.kind,
        Some(
            CompletionItemKind::METHOD
                | CompletionItemKind::FUNCTION
                | CompletionItemKind::CONSTRUCTOR
        )
    ) {
        return Vec::new();
    }
    let source = item
        .label_details
        .as_ref()
        .and_then(|details| details.detail.as_deref())
        .filter(|detail| detail.contains('('))
        .unwrap_or(&item.label);
    let Some(open) = source.find('(') else {
        return Vec::new();
    };
    let mut depth = 0usize;
    let mut params = Vec::new();
    let mut current = String::new();
    for c in source[open + 1..].chars() {
        match c {
            '(' | '<' | '[' | '{' => {
                depth += 1;
                current.push(c);
            }
            ')' if depth == 0 => {
                params.push(std::mem::take(&mut current));
                break;
            }
            ')' | '>' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            ',' if depth == 0 => params.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    params
        .iter()
        .filter_map(|param| {
            let param = param.trim();
            // `name: Type` (Rust, Kotlin, TS) names come first; `Type name` last.
            let name = match param.split_once(':') {
                Some((name, _)) if !name.contains(' ') || name.starts_with("mut ") => {
                    name.trim_start_matches("mut ").trim()
                }
                _ => param
                    .trim_end_matches("...")
                    .rsplit(|c: char| c.is_whitespace() || c == '*' || c == '&')
                    .next()
                    .unwrap_or_default(),
            };
            let name: String = name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (!name.is_empty() && name != "self").then_some(name)
        })
        .collect()
}

/// A snippet with one tab stop per parameter for a call inserted without
/// arguments (`name()`, `name($0)`), or `None` when the item already has
/// placeholders, is not a call, or takes no parameters.
fn parameter_snippet(item: &CompletionItem, text: &str, snippet: bool) -> Option<String> {
    let call = if snippet {
        // Only bare calls; real snippets (`${1:...}`, `$1`) are kept.
        let stripped = text.strip_suffix("$0").unwrap_or(text);
        let stripped = stripped
            .strip_suffix("($0)")
            .or_else(|| stripped.strip_suffix("(${0})"))
            .map(|name| format!("{name}()"))
            .unwrap_or_else(|| stripped.to_owned());
        if stripped.contains('$') {
            return None;
        }
        stripped
    } else {
        text.to_owned()
    };
    let name = call.strip_suffix("()")?;
    if name.is_empty() || name.contains(['\n', '(', ')']) {
        return None;
    }
    let params = parameter_names(item);
    if params.is_empty() {
        return None;
    }
    let escape = |text: &str| {
        text.replace('\\', "\\\\")
            .replace('$', "\\$")
            .replace('}', "\\}")
    };
    let stops: Vec<String> = params
        .iter()
        .enumerate()
        .map(|(index, param)| format!("${{{}:{}}}", index + 1, escape(param)))
        .collect();
    let name = if snippet {
        name.to_owned()
    } else {
        escape(name)
    };
    Some(format!("{name}({})$0", stops.join(", ")))
}

/// Indents every line after the first with `indent`, and expands tabs to the
/// indentation unit of the line (a tab if it is tab-indented, else 4 spaces).
fn adjust_indentation(text: &str, indent: &str) -> String {
    let unit = if indent.starts_with('\t') {
        "\t"
    } else {
        "    "
    };
    let mut lines = text.split('\n');
    let mut out = lines.next().unwrap_or_default().replace('\t', unit);
    for line in lines {
        out.push('\n');
        out.push_str(indent);
        out.push_str(&line.replace('\t', unit));
    }
    out
}

pub(crate) fn lsp_change(text: &TextDocument, edit: &TextEdit) -> TextChange {
    let start = text.char_from_lsp_position(
        edit.range.start.line as usize,
        edit.range.start.character as usize,
    );
    let end = text
        .char_from_lsp_position(
            edit.range.end.line as usize,
            edit.range.end.character as usize,
        )
        .max(start);
    TextChange {
        start_char: start,
        end_char: end,
        inserted: edit.new_text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;
    use archipel_ide_lsp::lsp_types::{
        CompletionItem, CompletionTextEdit, InsertReplaceEdit, InsertTextFormat, Position, Range,
        TextEdit,
    };
    use serde_json::json;

    fn item(label: &str) -> CompletionItem {
        CompletionItem {
            label: label.into(),
            ..Default::default()
        }
    }

    fn range(line: u32, start: u32, end: u32) -> Range {
        Range::new(Position::new(line, start), Position::new(line, end))
    }

    /// A project with one open document and the caret at `caret`.
    fn state_with(text: &str, caret: usize) -> (IdeState, PathBuf) {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-completion-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("notes.txt"), text).unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "notes.txt"}))
            .unwrap();
        state.documents[0]
            .selections
            .set_single(Selection::caret(caret));
        (state, root)
    }

    fn open_session(state: &mut IdeState, anchor: usize, explicit: bool) {
        state.completion = Some(CompletionSession {
            path: PathBuf::from("notes.txt"),
            anchor,
            request_caret: state.documents[0].selections.primary().head,
            items: Vec::new(),
            ranked: Vec::new(),
            selected: 0,
            prefix: String::new(),
            incomplete: false,
            waiting: true,
            explicit,
            resolved: Default::default(),
            indexing: None,
            requery_when_ready: false,
            native: None,
            note: None,
            invocation: 1,
        });
    }

    fn visible_labels(state: &IdeState) -> Vec<String> {
        let session = state.completion.as_ref().unwrap();
        session
            .ranked
            .iter()
            .map(|row| session.items[row.index].label.clone())
            .collect()
    }

    fn type_text(state: &mut IdeState, text: &str) {
        let document = &mut state.documents[0];
        let caret = document.selections.primary().head;
        document.text.edit(caret, caret, text).unwrap();
        document
            .selections
            .set_single(Selection::caret(caret + text.chars().count()));
        state.completion_after_typing(text);
    }

    #[test]
    fn filters_as_you_type_and_closes_when_nothing_matches() {
        let (mut state, root) = state_with("app.", 4);
        open_session(&mut state, 4, false);
        state.receive_completion(CompletionResult {
            items: ["greeting", "getRequiredField", "getClass", "hashCode"]
                .into_iter()
                .map(item)
                .collect(),
            incomplete: false,
        });
        assert_eq!(visible_labels(&state).len(), 4);
        type_text(&mut state, "g");
        assert_eq!(
            visible_labels(&state),
            ["getClass", "getRequiredField", "greeting"]
        );
        type_text(&mut state, "RF");
        assert_eq!(visible_labels(&state), ["getRequiredField"]);
        assert_eq!(state.completion.as_ref().unwrap().prefix, "gRF");
        type_text(&mut state, "x");
        assert!(state.completion.is_none(), "no match closes the popup");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn leaving_the_identifier_closes_the_session() {
        let (mut state, root) = state_with("app.ge", 6);
        open_session(&mut state, 4, false);
        state.receive_completion(CompletionResult {
            items: vec![item("getClass")],
            incomplete: false,
        });
        assert!(state.completion.is_some());
        state.documents[0]
            .selections
            .set_single(Selection::caret(2));
        state.refresh_completion();
        assert!(state.completion.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_empty_results_report_no_suggestions() {
        let (mut state, root) = state_with("zz", 2);
        open_session(&mut state, 0, true);
        state.receive_completion(CompletionResult::default());
        assert_eq!(state.status, "No suggestions");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enter_inserts_and_tab_replaces_the_identifier() {
        for (replace, expected) in [(false, "app.greetingName();"), (true, "app.greeting();")] {
            let (mut state, root) = state_with("app.gName();", 5);
            open_session(&mut state, 4, false);
            let mut greeting = item("greeting");
            greeting.text_edit = Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
                new_text: "greeting".into(),
                insert: range(0, 4, 5),
                replace: range(0, 4, 9),
            }));
            state.receive_completion(CompletionResult {
                items: vec![greeting],
                incomplete: false,
            });
            state.accept_completion(replace).unwrap();
            let document = &state.documents[0];
            assert_eq!(document.text.text(), expected);
            assert_eq!(document.selections.primary().head, 12);
            assert!(state.completion.is_none());
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn additional_text_edits_add_imports_in_one_undo_step() {
        let (mut state, root) = state_with("class A {\n  Li\n}\n", 14);
        open_session(&mut state, 12, false);
        let mut list = item("List");
        list.text_edit = Some(CompletionTextEdit::Edit(TextEdit::new(
            range(1, 2, 4),
            "List".into(),
        )));
        list.additional_text_edits = Some(vec![TextEdit::new(
            range(0, 0, 0),
            "import java.util.List;\n\n".into(),
        )]);
        state.receive_completion(CompletionResult {
            items: vec![list],
            incomplete: false,
        });
        state.accept_completion(false).unwrap();
        let document = &mut state.documents[0];
        assert_eq!(
            document.text.text(),
            "import java.util.List;\n\nclass A {\n  List\n}\n"
        );
        let caret = document.selections.primary().head;
        assert_eq!(document.text.slice(caret - 4, caret).unwrap(), "List");
        assert!(document.text.undo());
        assert_eq!(document.text.text(), "class A {\n  Li\n}\n");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plain_items_replace_the_typed_prefix_and_selection_wraps() {
        let (mut state, root) = state_with("x = has", 7);
        open_session(&mut state, 4, false);
        state.receive_completion(CompletionResult {
            items: vec![item("hashCode"), item("hasNext")],
            incomplete: false,
        });
        state.move_completion_selection(-1);
        assert_eq!(state.completion.as_ref().unwrap().selected, 1);
        state.move_completion_selection(1);
        assert_eq!(state.completion.as_ref().unwrap().selected, 0);
        state.accept_completion(false).unwrap();
        assert_eq!(state.documents[0].text.text(), "x = hashCode");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn snippet_items_expand_and_start_a_tab_stop_session() {
        let (mut state, root) = state_with("app.gr;", 6);
        open_session(&mut state, 4, false);
        let mut greeting = item("greeting");
        greeting.insert_text_format = Some(InsertTextFormat::SNIPPET);
        greeting.text_edit = Some(CompletionTextEdit::Edit(TextEdit::new(
            range(0, 4, 6),
            "greeting(${1:name})$0".into(),
        )));
        let mut plain_call = item("toString");
        plain_call.insert_text_format = Some(InsertTextFormat::SNIPPET);
        plain_call.insert_text = Some("toString()$0".into());
        state.receive_completion(CompletionResult {
            items: vec![greeting, plain_call],
            incomplete: false,
        });
        state.accept_completion(false).unwrap();
        assert_eq!(state.documents[0].text.text(), "app.greeting(name);");
        let selection = state.documents[0].selections.primary();
        assert_eq!((selection.start(), selection.end()), (13, 17));
        assert!(state.snippet.is_some());
        state.snippet_next();
        assert!(state.snippet.is_none());
        assert_eq!(state.documents[0].selections.primary().head, 18);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn calls_without_arguments_get_parameter_tab_stops() {
        use archipel_ide_lsp::lsp_types::{CompletionItemKind, CompletionItemLabelDetails};
        let mut greeting = item("greeting(String name) : String");
        greeting.kind = Some(CompletionItemKind::METHOD);
        assert_eq!(parameter_names(&greeting), ["name"]);
        assert_eq!(
            parameter_snippet(&greeting, "greeting()", false).as_deref(),
            Some("greeting(${1:name})$0")
        );
        assert_eq!(
            parameter_snippet(&greeting, "greeting($0)", true).as_deref(),
            Some("greeting(${1:name})$0")
        );
        // Real snippets and parameterless methods are left alone.
        assert_eq!(
            parameter_snippet(&greeting, "greeting(${1:x})$0", true),
            None
        );
        let mut to_string = item("toString() : String");
        to_string.kind = Some(CompletionItemKind::METHOD);
        assert_eq!(parameter_snippet(&to_string, "toString()", false), None);
        // Label details (jdtls with labelDetailsSupport), generics and Rust/Kotlin style.
        let mut put = item("put");
        put.kind = Some(CompletionItemKind::METHOD);
        put.label_details = Some(CompletionItemLabelDetails {
            detail: Some("(Map<K, V> map, int... counts)".into()),
            description: None,
        });
        assert_eq!(parameter_names(&put), ["map", "counts"]);
        let mut rust = item("push(&mut self, value: T)");
        rust.kind = Some(CompletionItemKind::FUNCTION);
        assert_eq!(parameter_names(&rust), ["value"]);
        let mut field = item("name");
        field.kind = Some(CompletionItemKind::FIELD);
        assert_eq!(parameter_snippet(&field, "name()", false), None);

        // Accepting `greeting()` selects the `name` placeholder inside the parentheses.
        let (mut state, root) = state_with("app.gr;", 6);
        open_session(&mut state, 4, false);
        greeting.insert_text = Some("greeting()".into());
        state.receive_completion(CompletionResult {
            items: vec![greeting],
            incomplete: false,
        });
        state.accept_completion(false).unwrap();
        assert_eq!(state.documents[0].text.text(), "app.greeting(name);");
        let selection = state.documents[0].selections.primary();
        assert_eq!((selection.start(), selection.end()), (13, 17));
        assert!(state.snippet.is_some());
        state.snippet_next();
        assert_eq!(state.documents[0].selections.primary().head, 18);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn multi_line_snippets_follow_the_line_indentation() {
        assert_eq!(
            adjust_indentation("for (;;) {\n\t$0\n}", "        "),
            "for (;;) {\n            $0\n        }"
        );
        assert_eq!(adjust_indentation("a\n\tb", "\t"), "a\n\t\tb");
        let (mut state, root) = state_with("    fo", 6);
        open_session(&mut state, 4, false);
        let mut fori = item("fori");
        fori.insert_text_format = Some(InsertTextFormat::SNIPPET);
        fori.insert_text = Some("for (${1:i}) {\n\t$0\n}".into());
        state.receive_completion(CompletionResult {
            items: vec![fori],
            incomplete: false,
        });
        state.accept_completion(false).unwrap();
        assert_eq!(
            state.documents[0].text.text(),
            "    for (i) {\n        \n    }"
        );
        state.snippet_next();
        assert_eq!(state.documents[0].selections.primary().head, 22);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn snippets_without_placeholders_just_place_the_caret() {
        let (mut state, root) = state_with("s.to", 4);
        open_session(&mut state, 2, false);
        let mut call = item("toString");
        call.insert_text_format = Some(InsertTextFormat::SNIPPET);
        call.insert_text = Some("toString($0)".into());
        state.receive_completion(CompletionResult {
            items: vec![call],
            incomplete: false,
        });
        state.accept_completion(false).unwrap();
        assert_eq!(state.documents[0].text.text(), "s.toString()");
        assert_eq!(state.documents[0].selections.primary().head, 11);
        assert!(state.snippet.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_auto_popup_inside_comments_and_strings() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-completion-rs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let text = "fn f() { let s = \"ab\"; // cd\n}\n";
        std::fs::write(root.join("lib.rs"), text).unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "lib.rs"}))
            .unwrap();
        let at = |needle: &str| text.find(needle).unwrap();
        assert!(state.caret_in_comment_or_string(at("ab") + 1));
        assert!(state.caret_in_comment_or_string(at("cd") + 1));
        assert!(!state.caret_in_comment_or_string(at("let") + 1));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn completion_commands_drive_the_session() {
        let (mut state, root) = state_with("has", 3);
        let commands = built_in_commands();
        assert!(!commands.is_enabled("completion.accept", &state));
        open_session(&mut state, 0, false);
        state.receive_completion(CompletionResult {
            items: vec![item("hashCode"), item("hasNext")],
            incomplete: false,
        });
        let moved = commands
            .execute("completion.move", &mut state, json!({"delta": 1}))
            .unwrap();
        assert_eq!(moved["selected"], 1);
        commands
            .execute("completion.accept", &mut state, json!({"row": 0}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "hashCode");
        std::fs::remove_dir_all(root).unwrap();
    }
}
