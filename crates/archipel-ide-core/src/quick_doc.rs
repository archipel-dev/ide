//! Quick documentation (Ctrl+Q / hover), the documentation panel beside the
//! completion popup, and `completionItem/resolve`.
//!
//! Resolve runs for the selected completion row (documentation, detail) and
//! again when an unresolved item is accepted, because servers such as jdtls
//! only compute auto-import edits (`additionalTextEdits`) during resolve.

use crate::IdeState;
use archipel_ide_lsp::{
    RequestHandle,
    lsp_types::{
        CompletionItem, HoverParams, Position, TextDocumentIdentifier, TextDocumentPositionParams,
        request::{HoverRequest, ResolveCompletionItem},
    },
    markup,
};
use archipel_ide_text::{Selection, TextChange};
use std::path::PathBuf;

/// A documentation popup anchored in a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickDoc {
    pub path: PathBuf,
    /// Char offset the documentation is about.
    pub offset: usize,
    pub markdown: String,
    /// Opened by resting the mouse (closes when the pointer leaves) rather
    /// than by Ctrl+Q (closes when the caret moves).
    pub from_mouse: bool,
}

#[derive(Debug, Clone)]
struct PendingHover {
    handle: RequestHandle,
    path: PathBuf,
    offset: usize,
    from_mouse: bool,
}

#[derive(Debug, Clone)]
struct PendingImport {
    handle: RequestHandle,
    path: PathBuf,
    /// Insertion start; only edits before it are applied.
    start: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DocsState {
    /// The quick documentation popup.
    pub popup: Option<QuickDoc>,
    /// Whether the documentation panel shows beside the completion popup.
    /// Ctrl+Q toggles it and the choice is kept, as in the reference IDE.
    pub completion_panel: bool,
    pending_hover: Option<PendingHover>,
    /// Resolve request for (item index) of the open completion session.
    pending_resolve: Option<(RequestHandle, usize)>,
    pending_import: Option<PendingImport>,
}

impl DocsState {
    /// Whether a documentation request is in flight.
    pub fn is_waiting(&self) -> bool {
        self.pending_hover.is_some()
            || self.pending_resolve.is_some()
            || self.pending_import.is_some()
    }
}

impl IdeState {
    fn active_lsp_target(
        &self,
        offset: usize,
    ) -> Option<(PathBuf, PathBuf, TextDocumentPositionParams)> {
        let document = self.documents.get(self.active_document?)?;
        let absolute = self.project.as_ref()?.root().join(&document.path);
        let (line, character) = document.text.lsp_position(offset).ok()?;
        let uri = archipel_ide_lsp::client::file_uri(&absolute).ok()?;
        Some((
            document.path.clone(),
            absolute,
            TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position::new(line as u32, character as u32),
            },
        ))
    }

    /// Ctrl+Q: toggles the completion documentation panel while the popup is
    /// open; otherwise shows (or hides) documentation for the symbol at the caret.
    pub fn toggle_quick_doc(&mut self) -> Result<(), String> {
        if self
            .completion
            .as_ref()
            .is_some_and(|session| session.visible())
        {
            self.docs.completion_panel = !self.docs.completion_panel;
            return Ok(());
        }
        if self.docs.popup.take().is_some() {
            return Ok(());
        }
        let caret = self.active_caret().ok_or("no active document")?;
        self.request_quick_doc(caret, false)
    }

    /// Requests documentation for the symbol at `offset` of the active document.
    pub fn request_quick_doc(&mut self, offset: usize, from_mouse: bool) -> Result<(), String> {
        let (path, absolute, position) = self
            .active_lsp_target(offset)
            .ok_or("no documentation target at this position")?;
        if let Some(pending) = self.docs.pending_hover.take() {
            self.lsp.cancel_request(&pending.handle);
        }
        let handle = self
            .lsp
            .send_request::<HoverRequest>(
                &absolute,
                HoverParams {
                    text_document_position_params: position,
                    work_done_progress_params: Default::default(),
                },
            )
            .map_err(|error| error.to_string())?;
        self.docs.pending_hover = Some(PendingHover {
            handle,
            path,
            offset,
            from_mouse,
        });
        Ok(())
    }

    pub fn close_quick_doc(&mut self) {
        self.docs.popup = None;
        if let Some(pending) = self.docs.pending_hover.take() {
            self.lsp.cancel_request(&pending.handle);
        }
    }

    /// Markdown for the selected completion row, when the panel is enabled.
    pub fn completion_documentation(&self) -> Option<String> {
        if !self.docs.completion_panel {
            return None;
        }
        let session = self.completion.as_ref()?;
        let item = session.selected_item()?;
        let language = self
            .active_absolute_path()
            .and_then(|path| self.lsp.language_id(&path))
            .unwrap_or("text");
        markup::completion_markdown(item, language)
    }

    fn supports_resolve(&self) -> bool {
        self.active_absolute_path()
            .and_then(|path| self.lsp.capabilities_for(&path).cloned())
            .and_then(|capabilities| capabilities.completion_provider)
            .and_then(|provider| provider.resolve_provider)
            .unwrap_or(false)
    }

    /// Collects hover and resolve responses; keeps the popup in sync with the caret.
    pub(crate) fn poll_docs(&mut self) {
        self.poll_hover();
        self.poll_resolve();
        self.poll_import();
        // A caret-anchored popup closes when the caret moves or the file changes.
        if let Some(popup) = &self.docs.popup {
            let active = self
                .active_document
                .and_then(|index| self.documents.get(index));
            let stale = match active {
                Some(document) => {
                    document.path != popup.path
                        || (!popup.from_mouse && document.selections.primary().head != popup.offset)
                }
                None => true,
            };
            if stale {
                self.docs.popup = None;
            }
        }
    }

    fn poll_hover(&mut self) {
        let Some(pending) = self.docs.pending_hover.clone() else {
            return;
        };
        let Some(result) = self
            .lsp
            .take_typed_response::<HoverRequest>(&pending.handle)
        else {
            return;
        };
        self.docs.pending_hover = None;
        let markdown = match result {
            Ok(Some(hover)) => markup::hover_markdown(&hover),
            Ok(None) => String::new(),
            Err(error) => {
                if !pending.from_mouse {
                    self.status = format!("Quick documentation failed: {error}");
                }
                return;
            }
        };
        if markdown.is_empty() {
            if !pending.from_mouse {
                self.status = "No documentation found".into();
            }
            return;
        }
        self.docs.popup = Some(QuickDoc {
            path: pending.path,
            offset: pending.offset,
            markdown,
            from_mouse: pending.from_mouse,
        });
    }

    /// Resolves the selected completion row once.
    fn poll_resolve(&mut self) {
        if let Some((handle, index)) = self.docs.pending_resolve.clone()
            && let Some(result) = self
                .lsp
                .take_typed_response::<ResolveCompletionItem>(&handle)
        {
            self.docs.pending_resolve = None;
            if let (Ok(resolved), Some(session)) = (result, self.completion.as_mut())
                && let Some(item) = session.items.get_mut(index)
                && item.label == resolved.label
            {
                merge_resolved(item, resolved);
                session.resolved.insert(index);
            }
        }
        let Some(session) = self.completion.as_ref() else {
            if let Some((handle, _)) = self.docs.pending_resolve.take() {
                self.lsp.cancel_request(&handle);
            }
            return;
        };
        let Some(index) = session.ranked.get(session.selected).map(|row| row.index) else {
            return;
        };
        if session.resolved.contains(&index) || session.waiting {
            return;
        }
        match &self.docs.pending_resolve {
            Some((_, pending)) if *pending == index => return,
            Some((handle, _)) => {
                let handle = handle.clone();
                self.lsp.cancel_request(&handle);
                self.docs.pending_resolve = None;
            }
            None => {}
        }
        if !self.supports_resolve() {
            return;
        }
        let item = session.items[index].clone();
        let Some(path) = self.active_absolute_path() else {
            return;
        };
        if let Ok(handle) = self.lsp.send_request::<ResolveCompletionItem>(&path, item) {
            self.docs.pending_resolve = Some((handle, index));
        }
    }

    /// After accepting an unresolved item, resolve it and apply its
    /// auto-import edits.
    pub(crate) fn resolve_accepted(&mut self, item: CompletionItem, start: usize) {
        if !self.supports_resolve() {
            return;
        }
        let (Some(path), Some(relative)) = (
            self.active_absolute_path(),
            self.active_document
                .and_then(|index| self.documents.get(index))
                .map(|document| document.path.clone()),
        ) else {
            return;
        };
        if let Ok(handle) = self.lsp.send_request::<ResolveCompletionItem>(&path, item) {
            self.docs.pending_import = Some(PendingImport {
                handle,
                path: relative,
                start,
            });
        }
    }

    fn poll_import(&mut self) {
        let Some(pending) = self.docs.pending_import.clone() else {
            return;
        };
        let Some(result) = self
            .lsp
            .take_typed_response::<ResolveCompletionItem>(&pending.handle)
        else {
            return;
        };
        self.docs.pending_import = None;
        let Ok(item) = result else {
            return;
        };
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get_mut(index))
            .filter(|document| document.path == pending.path)
        else {
            return;
        };
        let changes: Vec<TextChange> = item
            .additional_text_edits
            .iter()
            .flatten()
            .map(|edit| crate::completion::lsp_change(&document.text, edit))
            .filter(|change| change.end_char <= pending.start)
            .collect();
        if changes.is_empty() {
            return;
        }
        let shift: isize = changes
            .iter()
            .map(|change| {
                change.inserted.chars().count() as isize
                    - (change.end_char - change.start_char) as isize
            })
            .sum();
        if document.text.edit_many(changes).is_err() {
            return;
        }
        let selections = document
            .selections
            .selections()
            .iter()
            .map(|selection| Selection {
                anchor: selection.anchor.saturating_add_signed(shift),
                head: selection.head.saturating_add_signed(shift),
            })
            .collect();
        document.selections.set_all(selections);
        document.analysis = archipel_ide_language::analyze(&document.path, &document.text.text());
        if let Some(snippet) = self.snippet.as_mut() {
            snippet.shift(shift);
        }
        self.sync_active_lsp();
    }
}

/// Copies the fields a resolve response fills in, keeping the rest.
fn merge_resolved(item: &mut CompletionItem, resolved: CompletionItem) {
    if resolved.documentation.is_some() {
        item.documentation = resolved.documentation;
    }
    if resolved.detail.is_some() {
        item.detail = resolved.detail;
    }
    if resolved.additional_text_edits.is_some() {
        item.additional_text_edits = resolved.additional_text_edits;
    }
    if resolved.text_edit.is_some() {
        item.text_edit = resolved.text_edit;
    }
    if resolved.command.is_some() {
        item.command = resolved.command;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_lsp::lsp_types::{Documentation, MarkupContent, MarkupKind};

    #[test]
    fn resolve_merges_documentation_without_dropping_fields() {
        let mut item = CompletionItem {
            label: "greeting".into(),
            insert_text: Some("greeting".into()),
            ..Default::default()
        };
        merge_resolved(
            &mut item,
            CompletionItem {
                label: "greeting".into(),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Greets".into(),
                })),
                ..Default::default()
            },
        );
        assert_eq!(item.insert_text.as_deref(), Some("greeting"));
        assert!(item.documentation.is_some());
    }
}
