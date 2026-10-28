//! Native completion sources in the completion session (spec 06, CI-CMP-020, CI-CMP-030).
//!
//! A language pack registers a [`CompletionSource`] on `COMPLETION_SOURCE`; the session asks
//! the source of the active document's language before the language server. At a position
//! where only class names are legal (`@|`, `new |`, `extends |`) the native answer is the
//! whole list and no server is asked; elsewhere it is merged ahead of the server's items and
//! the server's duplicates (same class name) are dropped. The answer is recomputed from the
//! live buffer whenever the prefix changes, so a keystroke never waits for a process.
//! Nothing here names a language.

use crate::IdeState;
use crate::completion::CompletionSession;
use archipel_ide_analysis::langext::{
    self, COMPLETION_SOURCE, CompletionSource, SourceCompletion, SourceItem, SourceItemKind,
    SourceRequest,
};
use archipel_ide_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionItemTag,
};
use archipel_ide_text::TextChange;
use serde_json::{Value, json};
use std::sync::Arc;

/// The key of a native row inside `CompletionItem::data`.
const KEY: &str = "archipelNativeKey";

/// The native part of a completion session.
#[derive(Debug, Clone)]
pub struct NativeSession {
    /// Id of the [`CompletionSource`] that answered.
    pub source: &'static str,
    /// The native list is complete: the language server was not asked.
    pub exclusive: bool,
    /// The current native rows.
    pub rows: Vec<CompletionItem>,
    /// The language server's items (and live templates) of the last response; the session's
    /// `items` are the native rows followed by these.
    pub server_items: Vec<CompletionItem>,
    /// `(caret, characters in the document, prefix)` the native rows were computed for.
    pub computed_for: (usize, usize, String),
}

fn kind_of(kind: SourceItemKind) -> CompletionItemKind {
    match kind {
        SourceItemKind::Class => CompletionItemKind::CLASS,
        SourceItemKind::Interface | SourceItemKind::Annotation => CompletionItemKind::INTERFACE,
        SourceItemKind::Enum => CompletionItemKind::ENUM,
        SourceItemKind::Record => CompletionItemKind::STRUCT,
    }
}

fn to_item(item: &SourceItem) -> CompletionItem {
    CompletionItem {
        label: item.label.clone(),
        kind: Some(kind_of(item.kind)),
        filter_text: Some(item.filter_text.clone()),
        sort_text: Some(item.sort_text.clone()),
        label_details: item.detail.as_ref().map(|d| CompletionItemLabelDetails {
            detail: None,
            description: Some(d.clone()),
        }),
        tags: item.deprecated.then(|| vec![CompletionItemTag::DEPRECATED]),
        data: Some(json!({ KEY: item.key })),
        ..CompletionItem::default()
    }
}

fn is_type_kind(kind: Option<CompletionItemKind>) -> bool {
    matches!(
        kind,
        Some(
            CompletionItemKind::CLASS
                | CompletionItemKind::INTERFACE
                | CompletionItemKind::ENUM
                | CompletionItemKind::STRUCT
        )
    )
}

/// Native rows first; the server's type rows with the same name are duplicates of them.
pub(crate) fn merge_items(
    native: &[CompletionItem],
    server: &[CompletionItem],
) -> Vec<CompletionItem> {
    let names: std::collections::HashSet<&str> = native.iter().map(|i| i.label.as_str()).collect();
    let mut out: Vec<CompletionItem> = native.to_vec();
    out.extend(
        server
            .iter()
            .filter(|i| {
                !(is_type_kind(i.kind)
                    && names.contains(
                        i.label
                            .split([' ', '<', '(', ':'])
                            .next()
                            .unwrap_or(&i.label),
                    ))
            })
            .cloned(),
    );
    out
}

impl IdeState {
    /// The native completion source serving the active document's language.
    pub(crate) fn completion_source_for_active(&self) -> Option<Arc<dyn CompletionSource>> {
        let document = self.documents.get(self.active_document?)?;
        let extensions = self.application().extensions();
        let language = langext::language_for_path(extensions, &document.path)?;
        extensions
            .extension_list(COMPLETION_SOURCE)
            .into_iter()
            .find(|source| source.languages().contains(&language.as_str()))
    }

    /// Whether a native source can complete in the active document (enables Ctrl+Space
    /// without a language server).
    pub fn active_has_completion_source(&self) -> bool {
        self.completion_source_for_active().is_some()
    }

    /// The source's answer at the caret of the active document.
    pub(crate) fn native_answer(
        &self,
        source: &dyn CompletionSource,
        count: u32,
    ) -> Option<SourceCompletion> {
        let document = self.documents.get(self.active_document?)?;
        let root = self.project.as_ref()?.root();
        let path = root.join(&document.path);
        let text = document.text.text();
        let caret = document.selections.primary().head;
        let offset = document.text.char_to_byte(caret);
        source.complete(&SourceRequest {
            path: &path,
            text: &text,
            offset,
            count,
        })
    }

    /// The native rows as completion items.
    pub(crate) fn native_rows(answer: &SourceCompletion) -> Vec<CompletionItem> {
        answer.items.iter().map(to_item).collect()
    }

    /// Recomputes the native rows of the open session when the prefix or the text changed.
    pub(crate) fn refresh_native_items(&mut self) {
        let Some(session) = self.completion.as_ref() else {
            return;
        };
        let Some(native) = session.native.as_ref() else {
            return;
        };
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .filter(|document| document.path == session.path)
        else {
            return;
        };
        let caret = document.selections.primary().head;
        let key = (caret, document.text.len_chars(), {
            document
                .text
                .slice(session.anchor, caret)
                .unwrap_or_default()
        });
        if native.computed_for == key {
            return;
        }
        let Some(source) = self
            .application()
            .extensions()
            .find_by_id(COMPLETION_SOURCE, native.source)
        else {
            return;
        };
        let count = session.invocation;
        let answer = self.native_answer(source.as_ref(), count);
        let Some(session) = self.completion.as_mut() else {
            return;
        };
        let Some(native) = session.native.as_mut() else {
            return;
        };
        native.computed_for = key;
        match answer {
            Some(answer) => {
                native.rows = Self::native_rows(&answer);
                session.items = merge_items(&native.rows, &native.server_items);
                session.note = answer.note;
            }
            None => {
                native.rows.clear();
                session.items = native.server_items.clone();
                session.note = None;
            }
        }
        session.resolved.clear();
    }

    /// The insertion of the accepted native row: the text for the identifier and the import
    /// edits as character-based changes (one undo step with the insertion).
    pub(crate) fn native_insertion(
        &self,
        session: &CompletionSession,
        item: &CompletionItem,
    ) -> Option<(String, Vec<TextChange>)> {
        let native = session.native.as_ref()?;
        let key = item.data.as_ref()?.get(KEY).and_then(Value::as_str)?;
        let source = self
            .application()
            .extensions()
            .find_by_id(COMPLETION_SOURCE, native.source)?;
        let document = self.documents.get(self.active_document?)?;
        let root = self.project.as_ref()?.root();
        let path = root.join(&document.path);
        let text = document.text.text();
        let offset = document
            .text
            .char_to_byte(document.selections.primary().head);
        let insertion = source.insertion(
            &SourceRequest {
                path: &path,
                text: &text,
                offset,
                count: 1,
            },
            key,
        )?;
        let changes = insertion
            .edits
            .iter()
            .map(|edit| TextChange {
                start_char: document.text.byte_to_char(edit.range.start as usize),
                end_char: document.text.byte_to_char(edit.range.end as usize),
                inserted: edit.new_text.clone(),
            })
            .collect();
        Some((insertion.text, changes))
    }

    /// Whether the text typed just now opens the popup by itself (after `@`, after `new `).
    pub(crate) fn native_auto_popup(&self, caret: usize) -> bool {
        let Some(source) = self.completion_source_for_active() else {
            return false;
        };
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            return false;
        };
        let before = document.text.slice(0, caret).unwrap_or_default();
        source.auto_popup(&before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(label: &str, kind: CompletionItemKind) -> CompletionItem {
        CompletionItem {
            label: label.into(),
            kind: Some(kind),
            ..CompletionItem::default()
        }
    }

    #[test]
    fn the_servers_duplicate_class_rows_yield_to_the_native_ones() {
        let native = vec![class("ArrayList", CompletionItemKind::CLASS)];
        let server = vec![
            class("ArrayList", CompletionItemKind::CLASS),
            class("ArrayList<>", CompletionItemKind::CLASS),
            class("arrayList", CompletionItemKind::VARIABLE),
            class("Arrays", CompletionItemKind::CLASS),
        ];
        let labels: Vec<String> = merge_items(&native, &server)
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert_eq!(labels, ["ArrayList", "arrayList", "Arrays"]);
    }
}
