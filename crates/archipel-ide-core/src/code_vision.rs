//! Code vision and inlay hints of the active document: `N usages` and the
//! last author of each class / method declaration (LSP `textDocument/references`
//! plus `git blame`), and parameter-name hints from `textDocument/inlayHint`.
//! Results are cached per document and refreshed (debounced) after edits.

use crate::{CommandError, IdeState, spec};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_language::structure::{StructureItem, SymbolKind};
use archipel_ide_lsp::{
    RequestHandle,
    lsp_types::{
        InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams, Location, Position, Range,
        ReferenceContext, ReferenceParams, TextDocumentIdentifier, TextDocumentPositionParams,
        request::{InlayHintRequest, References},
    },
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Quiet time after an edit before hints and usages are requested again.
const REFRESH_DELAY: Duration = Duration::from_millis(600);
/// Declarations asked for usages per document.
const MAX_LENSES: usize = 60;

/// A grey line above (here: after) a declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeLens {
    pub line: usize,
    /// Character column of the declared name (the Find Usages target).
    pub column: usize,
    pub usages: Option<usize>,
    pub author: Option<String>,
}

impl CodeLens {
    /// `2 usages  sample`, as IntelliJ renders it.
    pub fn text(&self) -> String {
        let mut parts = Vec::new();
        if let Some(count) = self.usages {
            parts.push(match count {
                1 => "1 usage".to_owned(),
                n => format!("{n} usages"),
            });
        }
        if let Some(author) = &self.author {
            parts.push(author.clone());
        }
        parts.join("  ")
    }
}

/// A parameter-name hint shown before the character at `column` of `line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlayLabel {
    pub line: usize,
    pub column: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
struct LensEntry {
    line: usize,
    column: usize,
    /// Declaration text when computed; a changed line drops the lens.
    line_text: String,
    /// Declaration name position (UTF-16) for the references request.
    position: (usize, usize),
    usages: Option<usize>,
    author: Option<String>,
}

#[derive(Debug, Clone)]
struct InlayEntry {
    line: usize,
    column: usize,
    line_text: String,
    text: String,
}

#[derive(Debug, Default)]
struct DocVision {
    revision: Option<u64>,
    /// Revision `lenses` were built for: retries while the server is not ready
    /// must not re-run the structure query and `git blame` of an unchanged text.
    lenses_revision: Option<u64>,
    requested_at: Option<Instant>,
    lenses: Vec<LensEntry>,
    inlays: Vec<InlayEntry>,
    hints: Option<(RequestHandle, u64)>,
    references: Vec<(usize, RequestHandle)>,
}

#[derive(Debug, Default)]
pub struct CodeVisionState {
    docs: HashMap<PathBuf, DocVision>,
    /// Hidden with `view.codeVision`; shown by default.
    pub hidden: bool,
}

/// Declarations that get a usages lens.
fn lens_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Class
            | SymbolKind::Interface
            | SymbolKind::Enum
            | SymbolKind::Method
            | SymbolKind::Function
            | SymbolKind::Constructor
    )
}

fn flatten<'a>(items: &'a [StructureItem], out: &mut Vec<&'a StructureItem>) {
    for item in items {
        if lens_kind(item.kind) {
            out.push(item);
        }
        flatten(&item.children, out);
    }
}

/// Plain text of an LSP inlay label, `None` for non-parameter hints.
fn parameter_label(hint: &InlayHint) -> Option<String> {
    if hint.kind != Some(InlayHintKind::PARAMETER) {
        return None;
    }
    let label = match &hint.label {
        InlayHintLabel::String(text) => text.clone(),
        InlayHintLabel::LabelParts(parts) => parts.iter().map(|p| p.value.as_str()).collect(),
    };
    let label = label.trim();
    (!label.is_empty()).then(|| label.to_owned())
}

impl IdeState {
    /// Lenses and inlays of document `index` that still match its text.
    pub fn code_vision(&self, index: usize) -> (Vec<CodeLens>, Vec<InlayLabel>) {
        let (Some(document), false) = (self.documents.get(index), self.code_vision.hidden) else {
            return Default::default();
        };
        let Some(vision) = self.code_vision.docs.get(&document.path) else {
            return Default::default();
        };
        let current = |line: usize, expected: &str| {
            document
                .text
                .line(line)
                .is_some_and(|text| text.trim_end_matches(['\r', '\n']) == expected)
        };
        let lenses = vision
            .lenses
            .iter()
            .filter(|lens| lens.usages.is_some() || lens.author.is_some())
            .filter(|lens| current(lens.line, &lens.line_text))
            .map(|lens| CodeLens {
                line: lens.line,
                column: lens.column,
                usages: lens.usages,
                author: lens.author.clone(),
            })
            .collect();
        let inlays = vision
            .inlays
            .iter()
            .filter(|inlay| current(inlay.line, &inlay.line_text))
            .map(|inlay| InlayLabel {
                line: inlay.line,
                column: inlay.column,
                text: inlay.text.clone(),
            })
            .collect();
        (lenses, inlays)
    }

    /// Collects finished requests and, once the document has been quiet,
    /// asks the server again. Called from [`IdeState::poll_lsp`].
    pub(crate) fn poll_code_vision(&mut self) {
        let Some(index) = self.active_document else {
            return;
        };
        let Some(document) = self.documents.get(index) else {
            return;
        };
        let (path, revision) = (document.path.clone(), document.text.revision());
        let Some(mut vision) = self.code_vision.docs.remove(&path) else {
            return self.start_code_vision(index);
        };
        self.collect_code_vision(index, &mut vision);
        let due = vision.revision != Some(revision)
            && vision.hints.is_none()
            && vision.references.is_empty()
            && vision
                .requested_at
                .is_none_or(|at| at.elapsed() >= REFRESH_DELAY);
        self.code_vision.docs.insert(path, vision);
        if due && !self.code_vision.hidden {
            self.start_code_vision(index);
        }
    }

    fn collect_code_vision(&mut self, index: usize, vision: &mut DocVision) {
        let Some(document) = self.documents.get(index) else {
            return;
        };
        if let Some((handle, revision)) = vision.hints.clone()
            && let Some(result) = self.lsp.take_typed_response::<InlayHintRequest>(&handle)
        {
            vision.hints = None;
            let hints = result.ok().flatten().unwrap_or_default();
            if revision == document.text.revision() {
                vision.inlays = hints
                    .iter()
                    .filter_map(|hint| {
                        let text = parameter_label(hint)?;
                        let offset = document.text.char_from_lsp_position(
                            hint.position.line as usize,
                            hint.position.character as usize,
                        );
                        let line = document.text.char_to_line(offset);
                        let start = document.text.line_to_char(line)?;
                        let line_text = document.text.line(line)?;
                        Some(InlayEntry {
                            line,
                            column: offset - start,
                            line_text: line_text.trim_end_matches(['\r', '\n']).to_owned(),
                            text,
                        })
                    })
                    .collect();
            }
        }
        let mut pending = Vec::new();
        for (slot, handle) in std::mem::take(&mut vision.references) {
            match self.lsp.take_typed_response::<References>(&handle) {
                None => pending.push((slot, handle)),
                Some(result) => {
                    if let Some(lens) = vision.lenses.get_mut(slot) {
                        lens.usages = result
                            .ok()
                            .flatten()
                            .map(|locations: Vec<Location>| locations.len());
                    }
                }
            }
        }
        vision.references = pending;
    }

    fn start_code_vision(&mut self, index: usize) {
        let Some(project) = &self.project else {
            return;
        };
        let Some(document) = self.documents.get(index) else {
            return;
        };
        let absolute = project.root().join(&document.path);
        let (path, revision) = (document.path.clone(), document.text.revision());
        let mut vision = self.code_vision.docs.remove(&path).unwrap_or_default();
        vision.requested_at = Some(Instant::now());
        if !self.lsp.handles(&absolute) {
            self.code_vision.docs.insert(path, vision);
            return;
        }
        let Ok(uri) = archipel_ide_lsp::client::file_uri(&absolute) else {
            self.code_vision.docs.insert(path, vision);
            return;
        };
        let identifier = TextDocumentIdentifier { uri };

        if vision.lenses_revision != Some(revision) {
            let structure = document.analysis.structure();
            let mut declarations = Vec::new();
            flatten(&structure, &mut declarations);
            declarations.truncate(MAX_LENSES);
            let blame = self.blame_document(index).ok();
            let document = &self.documents[index];
            let lenses: Vec<LensEntry> = declarations
                .into_iter()
                .filter_map(|item| {
                    let limit = document.text.char_to_byte(document.text.len_chars());
                    let start = document.text.byte_to_char(item.name_range.start.min(limit));
                    let (line, character) = document.text.lsp_position(start).ok()?;
                    let line_text = document.text.line(item.line)?;
                    let author =
                        blame
                            .as_ref()
                            .and_then(|lines| lines.get(item.line))
                            .map(|line| {
                                if line.is_uncommitted() {
                                    "You".to_owned()
                                } else {
                                    line.author.clone()
                                }
                            });
                    let line_start = document.text.line_to_char(item.line)?;
                    Some(LensEntry {
                        line: item.line,
                        column: start.saturating_sub(line_start),
                        line_text: line_text.trim_end_matches(['\r', '\n']).to_owned(),
                        position: (line, character),
                        usages: vision
                            .lenses
                            .iter()
                            .find(|old| old.line == item.line)
                            .and_then(|old| old.usages),
                        author,
                    })
                })
                .collect();
            vision.lenses = lenses;
            vision.lenses_revision = Some(revision);
        }

        if let Some((handle, _)) = vision.hints.take() {
            self.lsp.cancel_request(&handle);
        }
        for (_, handle) in std::mem::take(&mut vision.references) {
            self.lsp.cancel_request(&handle);
        }
        let supports_hints = self
            .lsp
            .capabilities_for(&absolute)
            .and_then(|capabilities| capabilities.inlay_hint_provider.as_ref())
            .is_some_and(|provider| {
                !matches!(provider, archipel_ide_lsp::lsp_types::OneOf::Left(false))
            });
        if supports_hints {
            let end = document.text.len_chars();
            let end = document.text.lsp_position(end).unwrap_or((0, 0));
            let params = InlayHintParams {
                work_done_progress_params: Default::default(),
                text_document: identifier.clone(),
                range: Range::new(
                    Position::new(0, 0),
                    Position::new(end.0 as u32, end.1 as u32),
                ),
            };
            if let Ok(handle) = self.lsp.send_request::<InlayHintRequest>(&absolute, params) {
                vision.hints = Some((handle, revision));
            }
        }
        if self
            .lsp
            .supports(&absolute, archipel_ide_lsp::Feature::References)
        {
            for (slot, lens) in vision.lenses.iter().enumerate() {
                let params = ReferenceParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: identifier.clone(),
                        position: Position::new(lens.position.0 as u32, lens.position.1 as u32),
                    },
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                    context: ReferenceContext {
                        include_declaration: false,
                    },
                };
                if let Ok(handle) = self.lsp.send_request::<References>(&absolute, params) {
                    vision.references.push((slot, handle));
                }
            }
        }
        // Retry later when the server was not ready to take any request.
        if vision.hints.is_some() || !vision.references.is_empty() {
            vision.revision = Some(revision);
        }
        self.code_vision.docs.insert(path, vision);
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "view.codeVision",
                "Toggle Code Vision and Inlay Hints",
                "View",
                None,
                "{}",
            ),
            |_: &IdeState| true,
            |state: &mut IdeState, _| {
                state.code_vision.hidden = !state.code_vision.hidden;
                Ok::<Value, CommandError>(json!({ "shown": !state.code_vision.hidden }))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lens_text_matches_the_reference() {
        let lens = |usages, author: Option<&str>| CodeLens {
            line: 0,
            column: 0,
            usages,
            author: author.map(str::to_owned),
        };
        assert_eq!(lens(Some(2), Some("sample")).text(), "2 usages  sample");
        assert_eq!(lens(Some(1), None).text(), "1 usage");
        assert_eq!(lens(None, Some("sample")).text(), "sample");
    }

    #[test]
    fn only_parameter_hints_are_shown() {
        let hint = |kind, label: &str| InlayHint {
            position: Position::new(0, 0),
            label: InlayHintLabel::String(label.into()),
            kind,
            text_edits: None,
            tooltip: None,
            padding_left: None,
            padding_right: None,
            data: None,
        };
        assert_eq!(
            parameter_label(&hint(Some(InlayHintKind::PARAMETER), "name:")),
            Some("name:".into())
        );
        assert_eq!(
            parameter_label(&hint(Some(InlayHintKind::TYPE), ": int")),
            None
        );
        assert_eq!(parameter_label(&hint(None, "x")), None);
    }
}
