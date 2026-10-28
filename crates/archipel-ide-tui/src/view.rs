//! One open document as the terminal shows it: the kit's editor model as a renderer, kept equal
//! to the core document.
//!
//! The core owns the document (text, carets and selections, folds, undo history, analysis,
//! language server sync) and does all editing: typed text, Enter, Backspace, Tab, the caret
//! keys and the line operations run through the core's input layer (`editor.type`,
//! `editor.action`, ...), exactly as in the GUI. The kit's `Editor` is only what draws the
//! text and takes the mouse, so [`DocView::pull`] copies the core's text, every caret and
//! selection and the fold regions into it after a change. The other direction is narrow:
//! what the kit widget itself does (a click or drag placing carets, a fold chevron, the Replace
//! of its find bar) is turned into core commands by [`DocView::outgoing`].

use archipel_ide_core::{Document, IdeState, Problem, Severity};
use archipel_ide_text::Selection;
use archipel_kit::editor::model::FoldRegion;
use archipel_kit::editor::model::{Editor, Language};
use archipel_kit::editor::{
    HighlightSeverity, RangeHighlight, Span, SyntaxHighlighter, TextAttributesKey,
};
use archipel_kit_tui::editor::EditorState;
use serde_json::json;
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

/// Kit byte offsets of the core char offsets `chars` (in any order). The kit text has LF line
/// breaks where a CRLF document has `\r\n`.
fn kit_bytes(core_text: &str, chars: &[usize], crlf: bool) -> Vec<usize> {
    let mut order: Vec<usize> = (0..chars.len()).collect();
    order.sort_by_key(|&i| chars[i]);
    let mut out = vec![0; chars.len()];
    let mut next = 0;
    let mut bytes = 0;
    let mut index = 0;
    let mut iter = core_text.chars();
    loop {
        while next < order.len() && chars[order[next]] <= index {
            out[order[next]] = bytes;
            next += 1;
        }
        let Some(ch) = iter.next() else { break };
        if !(crlf && ch == '\r') {
            bytes += ch.len_utf8();
        }
        index += 1;
    }
    while next < order.len() {
        out[order[next]] = bytes;
        next += 1;
    }
    out
}

/// Core char offsets of the kit byte offsets `bytes` (in any order), the inverse of
/// [`kit_bytes`] for `kit_text`.
fn core_chars(kit_text: &str, bytes: &[usize], crlf: bool) -> Vec<usize> {
    let mut order: Vec<usize> = (0..bytes.len()).collect();
    order.sort_by_key(|&i| bytes[i]);
    let mut out = vec![0; bytes.len()];
    let mut next = 0;
    let mut chars = 0;
    for (at, ch) in kit_text.char_indices() {
        while next < order.len() && bytes[order[next]] <= at {
            out[order[next]] = chars;
            next += 1;
        }
        chars += 1;
        if crlf && ch == '\n' {
            chars += 1;
        }
    }
    while next < order.len() {
        out[order[next]] = chars;
        next += 1;
    }
    out
}

/// What the document's folds look like, to notice a change without comparing regions.
fn fold_signature(document: &Document) -> (u64, usize, usize) {
    let collapsed = document.folds.collapsed();
    let hash = collapsed.iter().fold(0usize, |hash, fold| {
        hash.wrapping_mul(31)
            .wrapping_add(fold.start_byte)
            .wrapping_mul(31)
            .wrapping_add(fold.end_byte)
    });
    (document.text.revision(), collapsed.len(), hash)
}

/// A command the terminal editor asks the core to run for what the kit widget did by itself.
pub type Outgoing = (&'static str, serde_json::Value);

/// An open document: the editor model that draws it, its view state and the bookkeeping that
/// tells a change of the core from a change the widget made.
pub struct DocView {
    pub path: PathBuf,
    pub editor: Editor,
    pub state: EditorState,
    pub highlighter: CoreHighlighter,
    pub language: Language,
    pub highlights: Vec<RangeHighlight>,
    /// The primary caret moved because of the core: keep it on screen.
    pub follow: bool,
    crlf: bool,
    core_revision: u64,
    core_selections: Vec<Selection>,
    core_folds: (u64, usize, usize),
    kit_stamp: u64,
    kit_carets: Vec<(usize, usize)>,
    kit_folds: Vec<bool>,
    force_pull: bool,
    problems: Option<Arc<Vec<Problem>>>,
    analysis_key: Option<(u64, usize)>,
}

impl DocView {
    /// A view of `document`.
    pub fn open(document: &Document) -> Self {
        let mut view = Self {
            path: document.path.clone(),
            language: language_for(&document.path),
            state: EditorState::new(),
            highlighter: CoreHighlighter::default(),
            highlights: Vec::new(),
            follow: false,
            crlf: false,
            core_revision: u64::MAX,
            core_selections: Vec::new(),
            core_folds: (u64::MAX, 0, 0),
            kit_stamp: 0,
            kit_carets: Vec::new(),
            kit_folds: Vec::new(),
            force_pull: true,
            problems: None,
            analysis_key: None,
            editor: Editor::new(""),
        };
        view.editor.settings.tab_size = 4;
        view.pull(document);
        view
    }

    /// The caret as (line, column), 1-based.
    pub fn cursor(&self) -> (usize, usize) {
        archipel_kit_tui::editor::cursor_position(&self.editor)
    }

    fn kit_carets_now(&self) -> Vec<(usize, usize)> {
        self.editor
            .carets
            .carets()
            .iter()
            .map(|caret| (caret.anchor, caret.offset))
            .collect()
    }

    fn kit_folds_now(&self) -> Vec<bool> {
        self.editor
            .folding
            .regions()
            .iter()
            .map(|region| region.expanded)
            .collect()
    }

    /// What the kit widget changed since the last sync, as core commands, for the document
    /// the core has active. Nothing when the core changed too: the core wins and
    /// [`DocView::pull`] shows its state.
    pub fn outgoing(&mut self, document: &Document) -> Vec<Outgoing> {
        let core_changed = self.force_pull
            || document.text.revision() != self.core_revision
            || document.selections.selections() != self.core_selections.as_slice()
            || fold_signature(document) != self.core_folds;
        let text_changed = self.editor.document.modification_stamp() != self.kit_stamp;
        let carets_changed = self.kit_carets_now() != self.kit_carets;
        let folds_changed = self.kit_folds_now() != self.kit_folds;
        if core_changed || !(text_changed || carets_changed || folds_changed) {
            return Vec::new();
        }
        let mut commands: Vec<Outgoing> = Vec::new();
        let kit_text = self.editor.document.text().to_owned();
        let wanted: std::borrow::Cow<str> = if self.crlf {
            std::borrow::Cow::Owned(kit_text.replace('\n', "\r\n"))
        } else {
            std::borrow::Cow::Borrowed(&kit_text)
        };
        if text_changed {
            let old: Vec<char> = document.text.text().chars().collect();
            let new: Vec<char> = wanted.chars().collect();
            let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
            let suffix = old[prefix..]
                .iter()
                .rev()
                .zip(new[prefix..].iter().rev())
                .take_while(|(a, b)| a == b)
                .count();
            let (old_end, new_end) = (old.len() - suffix, new.len() - suffix);
            if prefix != old_end || prefix != new_end {
                commands.push((
                    "editor.replaceRange",
                    json!({
                        "start": prefix,
                        "end": old_end,
                        "text": new[prefix..new_end].iter().collect::<String>(),
                    }),
                ));
            }
        }
        if text_changed || carets_changed {
            let carets = self.editor.carets.carets();
            let primary = self.editor.carets.primary_index();
            let mut offsets = Vec::with_capacity(carets.len() * 2);
            for caret in carets {
                offsets.push(caret.anchor);
                offsets.push(caret.offset);
            }
            let chars = core_chars(&kit_text, &offsets, self.crlf);
            let mut selections: Vec<serde_json::Value> = Vec::with_capacity(carets.len());
            for (index, pair) in chars.chunks(2).enumerate() {
                let selection = json!({"anchor": pair[0], "head": pair[1]});
                if index == primary {
                    selections.insert(0, selection);
                } else {
                    selections.push(selection);
                }
            }
            commands.push(("editor.setSelections", json!({ "selections": selections })));
        }
        if folds_changed {
            let before = &self.kit_folds;
            for (index, region) in self.editor.folding.regions().iter().enumerate() {
                if before.get(index).is_some_and(|b| *b != region.expanded) {
                    let line = self.editor.document.line_number(region.start);
                    commands.push(("editor.toggleFold", json!({ "line": line })));
                }
            }
        }
        // Whatever the core makes of it is shown on the next sync.
        self.force_pull = true;
        commands
    }

    /// Shows the core document in the editor model when it changed (or `force`d by
    /// [`DocView::outgoing`]).
    pub fn pull(&mut self, document: &Document) -> bool {
        let selections_changed =
            document.selections.selections() != self.core_selections.as_slice();
        let text_changed = document.text.revision() != self.core_revision;
        let folds = fold_signature(document);
        let folds_changed = folds != self.core_folds;
        let forced = std::mem::take(&mut self.force_pull);
        if !(forced || selections_changed || text_changed || folds_changed) {
            return false;
        }
        let core_text = document.text.text();
        let crlf = core_text.contains("\r\n");
        self.crlf = crlf;
        if forced || text_changed || self.editor.document.modification_stamp() != self.kit_stamp {
            self.pull_text(&core_text, crlf);
        }
        // Carets: the others first, the primary last (the kit's primary is the newest).
        let selections = document.selections.selections();
        let offsets: Vec<usize> = selections.iter().flat_map(|s| [s.anchor, s.head]).collect();
        let bytes = kit_bytes(&core_text, &offsets, crlf);
        let pairs: Vec<(usize, usize)> = bytes.chunks(2).map(|p| (p[0], p[1])).collect();
        let primary_before = self.editor.carets.primary().offset;
        let (primary, others) = (pairs[0], &pairs[1..]);
        match others.split_first() {
            None => self
                .editor
                .carets
                .set_selection(&self.editor.document, primary.0, primary.1),
            Some((first, rest)) => {
                self.editor
                    .carets
                    .set_selection(&self.editor.document, first.0, first.1);
                for (anchor, head) in rest {
                    self.editor.carets.add_caret_with_selection(
                        &self.editor.document,
                        *anchor,
                        *head,
                    );
                }
                self.editor.carets.add_caret_with_selection(
                    &self.editor.document,
                    primary.0,
                    primary.1,
                );
            }
        }
        self.editor.carets.overwrite = document.input.overwrite;
        if self.editor.carets.primary().offset != primary_before || text_changed {
            self.follow = true;
        }
        // Folds.
        if folds_changed || text_changed || self.kit_folds.is_empty() {
            let to_kit = |byte: usize| {
                if crlf {
                    let line = document.text.char_to_line(document.text.byte_to_char(byte));
                    byte.saturating_sub(line)
                } else {
                    byte
                }
            };
            let regions: Vec<FoldRegion> = document
                .fold_regions()
                .into_iter()
                .map(|region| FoldRegion {
                    start: to_kit(region.range.start_byte),
                    end: to_kit(region.range.end_byte),
                    placeholder: region.range.placeholder.clone(),
                    expanded: !region.collapsed,
                })
                .collect();
            let len = self.editor.document.len();
            self.editor
                .folding
                .replace_regions(regions.into_iter().filter(|r| r.end <= len).collect());
        }
        self.core_revision = document.text.revision();
        self.core_selections = selections.to_vec();
        self.core_folds = folds;
        self.kit_stamp = self.editor.document.modification_stamp();
        self.kit_carets = self.kit_carets_now();
        self.kit_folds = self.kit_folds_now();
        true
    }

    /// Replaces the editor text with the core's by one minimal edit (the kit keeps its caches
    /// and scroll position), without a history of its own.
    fn pull_text(&mut self, core_text: &str, crlf: bool) {
        let normalized = if crlf {
            core_text.replace("\r\n", "\n")
        } else {
            core_text.to_owned()
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
        self.editor.undo.clear();
    }

    /// Highlights, diagnostics and the view of the core's state for document `index`.
    pub fn refresh(&mut self, state: &mut IdeState, index: usize) {
        if let Some(document) = state.document(index) {
            self.pull(document);
        }
        self.refresh_decorations(state, index);
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
    use archipel_ide_core::built_in_commands;
    use archipel_ide_text::Selection as CoreSelection;

    fn state_with(path: &str, text: &str) -> IdeState {
        let mut state = IdeState::default();
        state.documents.push(Document::new(path.into(), text));
        state.active_document = Some(0);
        state
    }

    fn run(state: &mut IdeState, commands: Vec<Outgoing>) {
        let registry = built_in_commands();
        for (id, params) in commands {
            registry.execute(id, state, params).unwrap();
        }
    }

    #[test]
    fn offsets_convert_between_core_chars_and_editor_bytes() {
        // `ä` is two bytes, the CRLF of the core is one LF in the editor.
        let core = "äö\r\nß = 1\r\n";
        let chars = [0, 2, 4, 5, 11];
        let bytes = kit_bytes(core, &chars, true);
        assert_eq!(bytes, vec![0, 4, 5, 7, 12]);
        // The `\n` after the `\r` is the same editor byte as the `\r` before it.
        assert_eq!(kit_bytes(core, &[3], true), vec![4]);
        let kit = core.replace("\r\n", "\n");
        assert_eq!(core_chars(&kit, &bytes, true), chars.to_vec());
        // Without CRLF the editor text is the core text.
        let plain = kit_bytes("äö\nß", &[0, 1, 3, 4], false);
        assert_eq!(plain, vec![0, 2, 5, 7]);
        assert_eq!(core_chars("äö\nß", &plain, false), vec![0, 1, 3, 4]);
    }

    #[test]
    fn the_core_selections_are_shown_as_carets_with_the_primary_last() {
        let mut state = state_with("a.txt", "one two three\n");
        let mut view = DocView::open(&state.documents[0]);
        state.documents[0].selections.set_all(vec![
            CoreSelection { anchor: 4, head: 7 },
            CoreSelection::caret(0),
            CoreSelection::caret(13),
        ]);
        assert!(view.pull(&state.documents[0]));
        let carets = view.editor.carets.carets();
        assert_eq!(carets.len(), 3);
        // The core's first selection is the primary one.
        let primary = view.editor.carets.primary();
        assert_eq!((primary.anchor, primary.offset), (0, 0));
        assert!(carets.iter().any(|c| (c.anchor, c.offset) == (4, 7)));
        // Nothing changed: nothing to do, nothing to send.
        assert!(!view.pull(&state.documents[0]));
        assert!(view.outgoing(&state.documents[0]).is_empty());
    }

    #[test]
    fn a_change_of_the_core_text_is_shown_without_an_editor_history() {
        let mut state = state_with("a.txt", "one\ntwo\n");
        let mut view = DocView::open(&state.documents[0]);
        state.documents[0].text.edit(0, 0, "zero\n").unwrap();
        state.documents[0]
            .selections
            .set_single(CoreSelection::caret(5));
        view.refresh(&mut state, 0);
        assert_eq!(view.editor.document.text(), "zero\none\ntwo\n");
        assert_eq!(view.editor.carets.primary().offset, 5);
        assert!(view.follow, "the caret the core moved is kept in view");
        assert!(
            !view.editor.undo.can_undo(&view.editor.carets),
            "the core owns the history"
        );
    }

    #[test]
    fn a_click_in_the_editor_becomes_a_caret_move_of_the_core() {
        let mut state = state_with("a.txt", "äö = 1\nß\n");
        let mut view = DocView::open(&state.documents[0]);
        let at = "äö".len();
        view.editor.carets.set_caret(&view.editor.document, at);
        let commands = view.outgoing(&state.documents[0]);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].0, "editor.setSelections");
        run(&mut state, commands);
        // The columns are chars in the core and bytes in the editor.
        assert_eq!(
            state.documents[0].selections.primary(),
            CoreSelection::caret(2)
        );
        view.refresh(&mut state, 0);
        assert_eq!(view.editor.carets.primary().offset, at);
    }

    #[test]
    fn a_replace_of_the_find_bar_reaches_the_core_with_crlf_kept() {
        let mut state = state_with("a.txt", "a foo\r\nb foo\r\n");
        let mut view = DocView::open(&state.documents[0]);
        assert_eq!(view.editor.document.text(), "a foo\nb foo\n");
        let at = view.editor.document.text().find("foo").unwrap();
        view.editor.replace_range(at, at + 3, "bar");
        view.editor.carets.set_caret(&view.editor.document, at + 3);
        let revision = state.documents[0].text.revision();
        let commands = view.outgoing(&state.documents[0]);
        assert_eq!(commands[0].0, "editor.replaceRange");
        run(&mut state, commands);
        assert_eq!(state.documents[0].text.text(), "a bar\r\nb foo\r\n");
        assert_eq!(state.documents[0].text.revision(), revision + 1, "one edit");
        assert_eq!(state.documents[0].selections.primary().head, 5);
        view.refresh(&mut state, 0);
        assert_eq!(view.editor.document.text(), "a bar\nb foo\n");
    }

    #[test]
    fn the_fold_regions_of_the_core_are_shown_and_a_chevron_click_toggles_them() {
        let source = "class A {\n    void f() {\n        g();\n        h();\n    }\n}\n";
        let mut state = state_with("A.java", source);
        let mut view = DocView::open(&state.documents[0]);
        let regions: Vec<bool> = view
            .editor
            .folding
            .regions()
            .iter()
            .map(|region| region.expanded)
            .collect();
        assert!(
            !regions.is_empty(),
            "the class and method bodies are regions"
        );
        assert!(regions.iter().all(|expanded| *expanded));
        // A click on the chevron of the method toggles the kit's region ...
        let method = view
            .editor
            .folding
            .regions()
            .iter()
            .position(|region| view.editor.document.line_number(region.start) == 1)
            .unwrap();
        view.editor.toggle_fold_region(method, false);
        let commands = view.outgoing(&state.documents[0]);
        assert!(commands.iter().any(|(id, _)| *id == "editor.toggleFold"));
        run(&mut state, commands);
        // ... and the core collapsed its own.
        assert!(
            state.documents[0]
                .fold_regions()
                .iter()
                .any(|region| region.collapsed && region.range.start_line == 1)
        );
        view.refresh(&mut state, 0);
        assert!(!view.editor.folding.regions()[method].expanded);
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
