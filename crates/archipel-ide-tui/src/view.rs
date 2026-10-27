//! One open document as the terminal shows it: the kit's editor model, the view state and the
//! mirror between the editor buffer and the core's document.
//!
//! The core owns the document (text, undo history seen by commands, analysis, language server
//! sync). The terminal editor works on the kit's `Editor` model, so every document has both,
//! and [`DocView::sync`] keeps them equal: an edit made in the terminal editor is pushed to the
//! core document as one minimal edit (and the analysis, the language server and completion are
//! told, like the GUI editor does after a snapshot edit); a change the core made (a quick fix,
//! a refactoring, a reload from disk, Go to Declaration moving the caret) is pulled into the
//! editor model as one undoable edit.

use archipel_ide_core::{Document, IdeState, Problem, Severity};
use archipel_ide_text::Selection;
use archipel_kit::editor::model::{Editor, Language};
use archipel_kit::editor::{
    HighlightSeverity, RangeHighlight, Span, SyntaxHighlighter, TextAttributesKey,
};
use archipel_kit_tui::editor::EditorState;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Colors the text from the core's analysis (tree-sitter highlights with IntelliJ attribute
/// keys), mapped to the kit's coarse kinds.
#[derive(Default)]
pub struct CoreHighlighter {
    spans: Vec<(usize, usize, TextAttributesKey)>,
    line_starts: Vec<usize>,
    /// Extra bytes each earlier line has in the core text (1 for CRLF files).
    line_extra: usize,
    generation: bool,
}

impl CoreHighlighter {
    fn rebuild(&mut self, document: &Document, kit_text: &str, crlf: bool) {
        self.line_extra = usize::from(crlf);
        self.line_starts.clear();
        self.line_starts.push(0);
        for (at, byte) in kit_text.bytes().enumerate() {
            if byte == b'\n' {
                self.line_starts.push(at + 1);
            }
        }
        self.spans = document
            .analysis
            .highlights
            .spans()
            .iter()
            .map(|span| (span.start_byte, span.end_byte, kind_of(span.key)))
            .filter(|(_, _, kind)| *kind != TextAttributesKey::Plain)
            .collect();
        // The span cache of the editor is keyed by name: flip it so a new analysis result
        // is never shown with the old colors.
        self.generation = !self.generation;
    }
}

impl SyntaxHighlighter for CoreHighlighter {
    fn highlight_line(&self, line: &str, state: &mut u32, out: &mut Vec<Span>) {
        let index = *state as usize;
        *state += 1;
        let Some(&kit_start) = self.line_starts.get(index) else {
            return;
        };
        let start = kit_start + index * self.line_extra;
        let end = start + line.len();
        let first = self
            .spans
            .partition_point(|(_, span_end, _)| *span_end <= start);
        for (span_start, span_end, kind) in &self.spans[first..] {
            if *span_start >= end {
                break;
            }
            let (from, to) = (
                (*span_start).max(start) - start,
                (*span_end).min(end) - start,
            );
            if from < to
                && line.is_char_boundary(from)
                && line.is_char_boundary(to)
                && out.last().is_none_or(|last| last.end <= from)
            {
                out.push(Span {
                    start: from,
                    end: to,
                    kind: *kind,
                });
            }
        }
    }

    fn name(&self) -> &'static str {
        if self.generation {
            "archipel-ide-core-a"
        } else {
            "archipel-ide-core-b"
        }
    }
}

/// The kit kind of an IntelliJ text attribute key.
pub fn kind_of(key: &str) -> TextAttributesKey {
    use TextAttributesKey as K;
    if key.contains("COMMENT") {
        K::Comment
    } else if key.contains("KEYWORD") {
        K::Keyword
    } else if key.contains("STRING") {
        K::String
    } else if key.contains("NUMBER") {
        K::Number
    } else if key.starts_with("ERRORS") || key.contains("BAD_CHARACTER") {
        K::Error
    } else if key.contains("OPERATION_SIGN")
        || key.contains("COMMA")
        || key.contains("BRACES")
        || key.contains("PARENTHS")
        || key.contains("BRACKETS")
        || key.contains("SEMICOLON")
        || key.contains("DOT")
    {
        K::Punctuation
    } else if key.contains("FUNCTION") || key.contains("FIELD") || key.contains("PROPERTY") {
        K::Property
    } else if key.contains("CLASS") || key.contains("TAG") || key.contains("TYPE") {
        K::Tag
    } else if key.contains("ATTRIBUTE") || key.contains("METADATA") {
        K::Attribute
    } else if key.contains("CONSTANT")
        || key.contains("VARIABLE")
        || key.contains("PARAMETER")
        || key.contains("LABEL")
    {
        K::Variable
    } else {
        K::Plain
    }
}

/// The editing rules of the language of `path`.
pub fn language_for(path: &Path) -> Language {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "rs" | "java" | "kt" | "kts" | "js" | "mjs" | "ts" | "tsx" | "jsx" | "c" | "h" | "cc"
        | "cpp" | "hpp" | "go" | "cs" | "swift" | "scala" | "groovy" | "gradle" | "css"
        | "json" => Language::c_like(),
        "py" | "sh" | "bash" | "toml" | "yaml" | "yml" | "rb" | "properties" => {
            let mut language = Language::c_like().with_line_comment("#");
            language.block_comment = None;
            language
        }
        _ => Language::plain_text(),
    }
}

/// Char index in the core text of byte `at` of the (LF-normalized) editor text.
fn core_char(kit_text: &str, at: usize, crlf: bool) -> usize {
    let at = at.min(kit_text.len());
    let before = &kit_text[..at];
    let chars = before.chars().count();
    if crlf {
        chars + before.bytes().filter(|byte| *byte == b'\n').count()
    } else {
        chars
    }
}

/// Byte offset in the editor text of char index `at` of the core text.
fn kit_byte(core_text: &str, at: usize, crlf: bool) -> usize {
    let mut bytes = 0;
    for (index, ch) in core_text.chars().enumerate() {
        if index >= at {
            break;
        }
        if crlf && ch == '\r' {
            continue;
        }
        bytes += ch.len_utf8();
    }
    bytes
}

/// What [`DocView::sync`] found out.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SyncResult {
    /// The terminal editor changed the text and the core document was updated.
    pub pushed: bool,
    /// The core changed the text and the editor model was updated.
    pub pulled: bool,
}

/// An open document: the editor model, its view state and the mirror bookkeeping.
pub struct DocView {
    pub path: PathBuf,
    pub editor: Editor,
    pub state: EditorState,
    pub highlighter: CoreHighlighter,
    pub language: Language,
    pub highlights: Vec<RangeHighlight>,
    crlf: bool,
    core_revision: u64,
    kit_stamp: u64,
    core_selection: Selection,
    kit_selection: (usize, usize),
    problems: Option<Arc<Vec<Problem>>>,
    analysis_key: Option<(u64, usize)>,
}

impl DocView {
    /// A view of `document`.
    pub fn open(document: &Document) -> Self {
        let core_text = document.text.text();
        let crlf = core_text.contains("\r\n");
        let mut editor = Editor::new(&core_text);
        let mut view = Self {
            path: document.path.clone(),
            language: language_for(&document.path),
            state: EditorState::new(),
            highlighter: CoreHighlighter::default(),
            highlights: Vec::new(),
            crlf,
            core_revision: document.text.revision(),
            kit_stamp: 0,
            core_selection: document.selections.primary(),
            kit_selection: (0, 0),
            problems: None,
            analysis_key: None,
            editor: Editor::new(""),
        };
        let selection = document.selections.primary();
        let anchor = kit_byte(&core_text, selection.anchor, crlf);
        let head = kit_byte(&core_text, selection.head, crlf);
        editor.carets.set_selection(&editor.document, anchor, head);
        view.editor = editor;
        view.kit_stamp = view.editor.document.modification_stamp();
        view.kit_selection = view.selection_bytes();
        view
    }

    fn selection_bytes(&self) -> (usize, usize) {
        let caret = self.editor.carets.primary();
        (caret.anchor, caret.offset)
    }

    /// The caret as (line, column), 1-based.
    pub fn cursor(&self) -> (usize, usize) {
        archipel_kit_tui::editor::cursor_position(&self.editor)
    }

    /// Whether the editor text differs from the core's (before the next sync).
    pub fn changed_in_editor(&self) -> bool {
        self.editor.document.modification_stamp() != self.kit_stamp
    }

    /// Mirrors editor and core for document `index` of the state.
    pub fn sync(&mut self, state: &mut IdeState, index: usize) -> SyncResult {
        let mut result = SyncResult::default();
        let Some(document) = state.document(index) else {
            return result;
        };
        let core_changed = document.text.revision() != self.core_revision;
        let kit_changed = self.changed_in_editor();
        if kit_changed {
            result.pushed = self.push(state, index);
        } else if core_changed {
            self.pull(state, index);
            result.pulled = true;
        } else {
            self.sync_selection(state, index);
        }
        self.refresh_decorations(state, index);
        result
    }

    /// Pushes the editor text and caret to the core document.
    fn push(&mut self, state: &mut IdeState, index: usize) -> bool {
        let kit_text = self.editor.document.text().to_owned();
        let caret = self.editor.carets.primary().clone();
        let crlf = self.crlf;
        let Some(document) = state.document_mut(index) else {
            return false;
        };
        let core_text = document.text.text();
        let wanted: std::borrow::Cow<str> = if crlf {
            std::borrow::Cow::Owned(kit_text.replace('\n', "\r\n"))
        } else {
            std::borrow::Cow::Borrowed(&kit_text)
        };
        let old: Vec<char> = core_text.chars().collect();
        let new: Vec<char> = wanted.chars().collect();
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(new[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let (old_end, new_end) = (old.len() - suffix, new.len() - suffix);
        let inserted: String = new[prefix..new_end].iter().collect();
        let pure_insertion = old_end == prefix;
        if prefix != old_end || !inserted.is_empty() {
            let _ = document.text.edit(prefix, old_end, &inserted);
        }
        let anchor = core_char(&kit_text, caret.anchor, crlf);
        let head = core_char(&kit_text, caret.offset, crlf);
        let selection = Selection { anchor, head };
        document.selections.set_single(selection);
        document.refresh_analysis(&wanted);
        let path = document.path.clone();
        let revision = document.text.revision();
        self.core_revision = revision;
        self.core_selection = selection;
        self.kit_stamp = self.editor.document.modification_stamp();
        self.kit_selection = self.selection_bytes();
        // The GUI editor does the same after a snapshot edit.
        state.sync_document_lsp(&path, &wanted, revision.min(i32::MAX as u64) as i32);
        if state.active_index() == Some(index) {
            state.snippet_after_change();
            let typed = if pure_insertion {
                inserted.as_str()
            } else {
                ""
            };
            state.completion_after_typing(typed);
            state.signature_after_typing(typed);
        }
        true
    }

    /// Replaces the editor text with the core's, as one undoable edit.
    fn pull(&mut self, state: &IdeState, index: usize) {
        let Some(document) = state.document(index) else {
            return;
        };
        let core_text = document.text.text();
        let crlf = core_text.contains("\r\n");
        self.crlf = crlf;
        let normalized = if crlf {
            core_text.replace("\r\n", "\n")
        } else {
            core_text.clone()
        };
        let old = self.editor.document.text().to_owned();
        if old != normalized {
            let prefix = old
                .bytes()
                .zip(normalized.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            let prefix = floor_boundary(&old, floor_boundary(&normalized, prefix));
            let suffix = old.as_bytes()[prefix..]
                .iter()
                .rev()
                .zip(normalized.as_bytes()[prefix..].iter().rev())
                .take_while(|(a, b)| a == b)
                .count();
            let (mut old_end, mut new_end) = (old.len() - suffix, normalized.len() - suffix);
            while !old.is_char_boundary(old_end) || !normalized.is_char_boundary(new_end) {
                old_end += 1;
                new_end += 1;
            }
            self.editor
                .replace_range(prefix, old_end, &normalized[prefix..new_end]);
        }
        let selection = document.selections.primary();
        let anchor = kit_byte(&core_text, selection.anchor, crlf);
        let head = kit_byte(&core_text, selection.head, crlf);
        self.editor
            .carets
            .set_selection(&self.editor.document, anchor, head);
        // The first Undo takes the pulled change back, not the caret the core moved.
        self.editor.undo.set_baseline(&self.editor.carets);
        self.core_revision = document.text.revision();
        self.core_selection = selection;
        self.kit_stamp = self.editor.document.modification_stamp();
        self.kit_selection = self.selection_bytes();
    }

    /// Carries a caret move across: whichever side moved since the last sync wins.
    fn sync_selection(&mut self, state: &mut IdeState, index: usize) {
        let kit_now = self.selection_bytes();
        let Some(document) = state.document(index) else {
            return;
        };
        let core_now = document.selections.primary();
        if kit_now != self.kit_selection {
            let kit_text = self.editor.document.text();
            let selection = Selection {
                anchor: core_char(kit_text, kit_now.0, self.crlf),
                head: core_char(kit_text, kit_now.1, self.crlf),
            };
            if let Some(document) = state.document_mut(index) {
                document.selections.set_single(selection);
            }
            self.core_selection = selection;
            self.kit_selection = kit_now;
            // Like the GUI editor after a caret move: snippets, parameter info and the
            // completion list follow the caret.
            if state.active_index() == Some(index) {
                state.snippet_after_caret_move();
                state.signature_after_caret_move();
                if state.completion().is_some() {
                    state.refresh_completion();
                }
            }
        } else if core_now != self.core_selection {
            let core_text = document.text.text();
            let anchor = kit_byte(&core_text, core_now.anchor, self.crlf);
            let head = kit_byte(&core_text, core_now.head, self.crlf);
            self.editor
                .carets
                .set_selection(&self.editor.document, anchor, head);
            self.core_selection = core_now;
            self.kit_selection = self.selection_bytes();
            // Navigation (Go to Declaration, Back) moved the caret: show it.
            self.state
                .scroll_to_line(self.editor.document.line_number(head));
        }
    }

    /// Highlights and diagnostics for the editor.
    fn refresh_decorations(&mut self, state: &mut IdeState, index: usize) {
        let kit_text_len = self.editor.document.text().len();
        let active = state.active_index() == Some(index);
        // Large files are highlighted lazily by range; the terminal editor asks for it all.
        if let Some(document) = state.document_mut(index)
            && !document.analysis.fully_highlighted()
        {
            let end = document.text.char_to_byte(document.text.len_chars());
            document.analysis.ensure_highlighted(0..end);
        }
        let Some(document) = state.document(index) else {
            return;
        };
        let key = (
            document.text.revision(),
            document.analysis.highlights.spans().len(),
        );
        if self.analysis_key != Some(key) {
            self.analysis_key = Some(key);
            self.highlighter
                .rebuild(document, self.editor.document.text(), self.crlf);
        }
        let _ = kit_text_len;
        if !active {
            return;
        }
        let problems = state.cached_active_problems();
        if self
            .problems
            .as_ref()
            .is_some_and(|known| Arc::ptr_eq(known, &problems))
        {
            return;
        }
        self.highlights = problems
            .iter()
            .filter(|problem| problem.severity != Severity::Hint)
            .map(|problem| self.highlight(problem))
            .collect();
        self.problems = Some(problems);
    }

    fn highlight(&self, problem: &Problem) -> RangeHighlight {
        let document = &self.editor.document;
        let offset = |line: usize, column: usize| {
            let line = line.min(document.line_count().saturating_sub(1));
            let start = document.line_start_offset(line);
            let text = document.line_text(line);
            let within = text
                .char_indices()
                .nth(column)
                .map_or(text.len(), |(at, _)| at);
            start + within
        };
        let start = offset(problem.line, problem.column);
        let mut end = offset(problem.end_line, problem.end_column);
        if end <= start {
            end = document.next_offset(start).max(start);
        }
        let severity = match problem.severity {
            Severity::Error => HighlightSeverity::Error,
            Severity::Warning => HighlightSeverity::Warning,
            Severity::WeakWarning => HighlightSeverity::WeakWarning,
            Severity::Hint => HighlightSeverity::Info,
        };
        RangeHighlight::new(start..end, severity).message(problem.message.clone())
    }

    /// The problems of the document as the daemon last reported them (the active document).
    pub fn problems(&self) -> Option<&[Problem]> {
        self.problems.as_ref().map(|problems| problems.as_slice())
    }

    /// Forces the diagnostics to be rebuilt (a document that became active).
    pub fn invalidate_problems(&mut self) {
        self.problems = None;
    }
}

fn floor_boundary(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_text::Selection as CoreSelection;

    fn state_with(text: &str) -> IdeState {
        let mut state = IdeState::default();
        state.documents.push(Document::new("a.txt".into(), text));
        state.active_document = Some(0);
        state
    }

    fn core_text(state: &IdeState) -> String {
        state.documents[0].text.text()
    }

    #[test]
    fn an_edit_in_the_editor_reaches_the_core_as_one_edit() {
        let mut state = state_with("hello world\n");
        let mut view = DocView::open(&state.documents[0]);
        let revision = state.documents[0].text.revision();
        let at = view.editor.document.text().find("world").unwrap();
        view.editor.replace_range(at, at + 5, "there");
        view.editor.carets.set_caret(&view.editor.document, at + 5);
        let result = view.sync(&mut state, 0);
        assert!(result.pushed && !result.pulled);
        assert_eq!(core_text(&state), "hello there\n");
        assert_eq!(state.documents[0].text.revision(), revision + 1, "one edit");
        assert_eq!(state.documents[0].selections.primary().head, 11);
        // Nothing left to do.
        assert_eq!(view.sync(&mut state, 0), SyncResult::default());
    }

    #[test]
    fn a_change_the_core_made_is_pulled_into_the_editor_and_can_be_undone_there() {
        let mut state = state_with("one\ntwo\n");
        let mut view = DocView::open(&state.documents[0]);
        state.documents[0].text.edit(0, 0, "zero\n").unwrap();
        state.documents[0]
            .selections
            .set_single(CoreSelection::caret(5));
        let result = view.sync(&mut state, 0);
        assert!(result.pulled && !result.pushed);
        assert_eq!(view.editor.document.text(), "zero\none\ntwo\n");
        assert_eq!(view.editor.carets.primary().offset, 5);
        // The pulled change is a step of the editor's own history.
        view.editor.execute(
            &archipel_kit::editor::model::EditorAction::Undo,
            &archipel_kit::editor::model::ActionContext::plain_text(),
        );
        assert_eq!(view.editor.document.text(), "one\ntwo\n");
    }

    #[test]
    fn columns_are_chars_in_the_core_and_bytes_in_the_editor() {
        let mut state = state_with("äö = 1\nß\n");
        let mut view = DocView::open(&state.documents[0]);
        // The core puts the caret after `ß` (char 8); the editor counts bytes.
        state.documents[0]
            .selections
            .set_single(CoreSelection::caret(8));
        view.sync(&mut state, 0);
        assert_eq!(view.editor.carets.primary().offset, "äö = 1\nß".len());
        // And back: an editor caret after `ö` is char 2.
        view.editor
            .carets
            .set_caret(&view.editor.document, "äö".len());
        view.sync(&mut state, 0);
        assert_eq!(state.documents[0].selections.primary().head, 2);
    }

    #[test]
    fn crlf_files_keep_their_line_endings_in_the_core() {
        let mut state = state_with("a\r\nb\r\n");
        let mut view = DocView::open(&state.documents[0]);
        assert_eq!(view.editor.document.text(), "a\nb\n");
        let end = view.editor.document.text().len();
        view.editor.replace_range(end, end, "c");
        view.sync(&mut state, 0);
        assert_eq!(core_text(&state), "a\r\nb\r\nc");
        // A selection in the second line maps across the CR.
        state.documents[0]
            .selections
            .set_single(CoreSelection { anchor: 3, head: 4 });
        view.sync(&mut state, 0);
        let caret = view.editor.carets.primary();
        assert_eq!((caret.anchor, caret.offset), (2, 3));
    }

    #[test]
    fn highlights_follow_the_analysis_keys() {
        assert_eq!(kind_of("DEFAULT_KEYWORD"), TextAttributesKey::Keyword);
        assert_eq!(kind_of("DEFAULT_DOC_COMMENT"), TextAttributesKey::Comment);
        assert_eq!(
            kind_of("DEFAULT_VALID_STRING_ESCAPE"),
            TextAttributesKey::String
        );
        assert_eq!(
            kind_of("DEFAULT_OPERATION_SIGN"),
            TextAttributesKey::Punctuation
        );
        assert_eq!(kind_of("DEFAULT_IDENTIFIER"), TextAttributesKey::Plain);
        let mut state = IdeState::default();
        state
            .documents
            .push(Document::new("A.java".into(), "class A { int x = 12; }\n"));
        let document = state.documents[0].clone();
        let mut highlighter = CoreHighlighter::default();
        highlighter.rebuild(&document, "class A { int x = 12; }\n", false);
        let mut spans = Vec::new();
        highlighter.highlight_line("class A { int x = 12; }", &mut 0, &mut spans);
        assert!(
            spans
                .iter()
                .any(|s| s.kind == TextAttributesKey::Keyword && s.start == 0),
            "{spans:?}"
        );
    }
}
