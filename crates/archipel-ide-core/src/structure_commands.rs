//! Structural editing (WP4): typed handlers, Enter, Complete Current
//! Statement, Move Statement, Auto-Indent Lines, block comments, code block and
//! method navigation, sticky scopes. The logic lives in
//! `archipel_ide_language` (query-driven, over syntax snapshots); this module
//! applies its edit descriptions to documents, registers the commands and the
//! generic services of the analysis registry.

use crate::{Document, IdeState, spec};
use archipel_ide_analysis::editing as contracts;
use archipel_ide_analysis::modcommand::TextEdit;
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_language::edit::{EditSet, TypedDecision};
use archipel_ide_language::indent::IndentOptions;
use archipel_ide_language::lines::{LineIndex, indent_bytes};
use archipel_ide_language::service::{DocView, LanguageServices, QueryServices};
use archipel_ide_language::{
    FileId, FileRev, LANGUAGES, Registration, ServiceRegistry, SyntaxSnapshot, brackets, commenter,
};
use archipel_ide_text::ops::{self, PairOptions};
use archipel_ide_text::{Selection, TextChange};
use serde_json::{Value, json};
use std::ops::Range;
use std::sync::{Arc, OnceLock};

fn services() -> &'static QueryServices {
    static SERVICES: OnceLock<QueryServices> = OnceLock::new();
    SERVICES.get_or_init(QueryServices::new)
}

/// A syntax snapshot of the document's current text (`None` for plain text).
/// Brings the analysis up to date first when edits are pending.
pub fn snapshot(document: &mut Document) -> Option<SyntaxSnapshot> {
    if document.text.revision() != document.analysis_sync.rev {
        let source = document.text.text();
        document.refresh_analysis(&source);
    }
    document
        .analysis
        .snapshot(FileId::default(), FileRev(document.text.revision()))
}

fn view<'a>(document: &Document, snapshot: &'a SyntaxSnapshot) -> DocView<'a> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    document.path.hash(&mut hasher);
    DocView {
        snapshot,
        id: hasher.finish(),
        revision: document.text.revision(),
        path_hint: None,
    }
}

fn byte_range(document: &Document, selection: Selection) -> Range<usize> {
    document.text.char_to_byte(selection.start())..document.text.char_to_byte(selection.end())
}

/// Applies an edit description (byte offsets) to the document.
pub fn apply_edit_set(document: &mut Document, set: &EditSet) -> Result<(), String> {
    let changes: Vec<TextChange> = set
        .edits
        .iter()
        .map(|edit| TextChange {
            start_char: document.text.byte_to_char(edit.start),
            end_char: document.text.byte_to_char(edit.end),
            inserted: edit.text.clone(),
        })
        .collect();
    document
        .text
        .edit_many(changes)
        .map_err(|error| error.to_string())?;
    if let Some(selection) = &set.selection {
        document.selections.set_single(Selection {
            anchor: document.text.byte_to_char(selection.start),
            head: document.text.byte_to_char(selection.end),
        });
    } else if let Some(caret) = set.caret {
        document
            .selections
            .set_single(Selection::caret(document.text.byte_to_char(caret)));
    }
    Ok(())
}

const AFTER_TYPED_TRIGGERS: &[char] = &['}', ')', ']', ':', '>', '"', '\''];

/// Types `text` at the carets: syntax-aware pairing, overtype and surround
/// for one character with one caret, then re-indentation of closers and tag
/// completion; everything else inserts plainly (multi-caret keeps the text
/// engine's pairing).
pub fn type_text(
    document: &mut Document,
    text: &str,
    pairing: PairOptions,
) -> Result<(), String> {
    let text_error = |error: archipel_ide_text::TextError| error.to_string();
    let mut chars = text.chars();
    let single = chars.next().filter(|_| chars.next().is_none());
    let one_caret = document.selections.selections().len() == 1;
    if let Some(typed) = single
        && one_caret
        && let Some(snap) = snapshot(document)
    {
        let selection = document.selections.primary();
        let quote = matches!(typed, '"' | '\'' | '`');
        let enabled = if quote { pairing.quotes } else { pairing.brackets };
        let range = byte_range(document, selection);
        let decision = if enabled {
            let doc_view = view(document, &snap);
            let chosen = (!selection.is_caret()).then(|| range.clone());
            services().before_typed(&doc_view, range.start, chosen, typed)
        } else {
            TypedDecision::Default
        };
        match decision {
            TypedDecision::Overtype => {
                document
                    .selections
                    .set_single(Selection::caret(selection.head + 1));
                return Ok(());
            }
            TypedDecision::InsertPair(close) => {
                document
                    .text
                    .edit(selection.head, selection.head, &format!("{typed}{close}"))
                    .map_err(text_error)?;
                document
                    .selections
                    .set_single(Selection::caret(selection.head + 1));
                return Ok(());
            }
            TypedDecision::Wrap { open, close } => {
                let (start, end) = (selection.start(), selection.end());
                document
                    .text
                    .edit_many([
                        TextChange {
                            start_char: start,
                            end_char: start,
                            inserted: open.to_string(),
                        },
                        TextChange {
                            start_char: end,
                            end_char: end,
                            inserted: close.to_string(),
                        },
                    ])
                    .map_err(text_error)?;
                document.selections.set_single(Selection {
                    anchor: start + 1,
                    head: end + 1,
                });
                return Ok(());
            }
            TypedDecision::Consume => return Ok(()),
            TypedDecision::Default => {}
        }
        document
            .selections
            .insert(&mut document.text, text)
            .map_err(text_error)?;
        if AFTER_TYPED_TRIGGERS.contains(&typed)
            && let Some(fresh) = snapshot(document)
        {
            let caret = document.text.char_to_byte(document.selections.primary().head);
            let doc_view = view(document, &fresh);
            if let Some(set) = services().after_typed(&doc_view, caret, typed) {
                apply_edit_set(document, &set)?;
            }
        }
        return Ok(());
    }
    let handled = match single {
        Some(typed) => {
            ops::type_char_with(&mut document.text, &mut document.selections, typed, pairing)
                .map_err(text_error)?
        }
        None => false,
    };
    if !handled {
        document
            .selections
            .insert(&mut document.text, text)
            .map_err(text_error)?;
    }
    Ok(())
}

/// Enter: smart indentation from the syntax tree, `{|}` splitting and comment
/// continuation; the text engine's newline without a grammar.
pub fn enter(document: &mut Document) -> Result<(), String> {
    if document.selections.selections().len() == 1
        && let Some(snap) = snapshot(document)
    {
        let range = byte_range(document, document.selections.primary());
        let doc_view = view(document, &snap);
        if let Some(set) = services().on_enter(&doc_view, range) {
            return apply_edit_set(document, &set);
        }
    }
    ops::newline(&mut document.text, &mut document.selections).map_err(|e| e.to_string())
}

/// The line comment prefix of the language at the primary caret, falling back
/// to the file extension table for plain text.
pub fn line_comment_prefix_at(document: &mut Document) -> &'static str {
    let offset = document.text.char_to_byte(document.selections.primary().head);
    snapshot(document)
        .and_then(|snap| commenter::line_prefix_at(&snap, offset))
        .unwrap_or_else(|| crate::editor_commands::line_comment_prefix(&document.path))
}

/// Comment with Line Comment: the language at the caret decides the prefix and
/// the tree decides which lines are already comments.
pub fn toggle_line_comment(document: &mut Document) -> Result<(), archipel_ide_text::TextError> {
    let prefix = line_comment_prefix_at(document);
    let snap = snapshot(document);
    let lines = snap.as_ref().map(|snap| LineIndex::new(snap.text()));
    let is_commented = |line: usize, text: &str| match (&snap, &lines) {
        (Some(snap), Some(lines)) => commenter::is_line_commented(snap, lines, line, prefix),
        _ => text.trim_start().starts_with(prefix),
    };
    ops::toggle_line_comment_with(
        &mut document.text,
        &mut document.selections,
        prefix,
        &is_commented,
    )
}

/// One pinned line for the editor: header text and the scope's line span
/// (zero-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickyLine {
    pub text: String,
    pub line: usize,
    pub end_line: usize,
}

/// Sticky scopes of the document for the first visible line (outermost first).
pub fn sticky_lines(document: &mut Document, first_visible_line: usize, max: usize) -> Vec<StickyLine> {
    let Some(snap) = snapshot(document) else {
        return Vec::new();
    };
    let doc_view = view(document, &snap);
    services()
        .sticky_scopes(&doc_view, first_visible_line, max)
        .into_iter()
        .map(|scope| StickyLine {
            text: snap.text()[scope.header.clone()].to_owned(),
            line: scope.start_line,
            end_line: scope.end_line,
        })
        .collect()
}

fn move_caret(document: &mut Document, byte: usize, select: bool) {
    let target = document.text.byte_to_char(byte);
    let selection = document.selections.primary();
    document.selections.set_single(if select {
        Selection {
            anchor: selection.anchor,
            head: target,
        }
    } else {
        Selection::caret(target)
    });
}

fn complete_statement(document: &mut Document) -> Result<(), String> {
    if let Some(snap) = snapshot(document) {
        let caret = document.text.char_to_byte(document.selections.primary().head);
        let doc_view = view(document, &snap);
        if let Some(set) = services().smart_enter(&doc_view, caret) {
            return apply_edit_set(document, &set);
        }
    }
    // Start New Line.
    let line = document.text.char_to_line(document.selections.primary().head);
    let (_, content_end, _) = ops::line_bounds(&document.text, line);
    document.selections.set_single(Selection::caret(content_end));
    enter(document)
}

fn move_statement(document: &mut Document, up: bool) -> Result<(), String> {
    if let Some(snap) = snapshot(document)
        && document.selections.selections().len() == 1
    {
        let range = byte_range(document, document.selections.primary());
        let doc_view = view(document, &snap);
        if let Some(set) = services().move_statement(&doc_view, range, up) {
            return apply_edit_set(document, &set);
        }
    }
    ops::move_lines(&mut document.text, &mut document.selections, up).map_err(|e| e.to_string())
}

fn comment_block(document: &mut Document) -> Result<(), String> {
    if let Some(snap) = snapshot(document)
        && document.selections.selections().len() == 1
    {
        let range = byte_range(document, document.selections.primary());
        let doc_view = view(document, &snap);
        if let Some(set) = services().toggle_block_comment(&doc_view, range) {
            return apply_edit_set(document, &set);
        }
    }
    toggle_line_comment(document).map_err(|e| e.to_string())
}

/// Auto-Indent Lines: reindents the selected lines (the caret's line without
/// a selection). Returns how many lines changed.
fn auto_indent(document: &mut Document) -> Result<usize, String> {
    let Some(snap) = snapshot(document) else {
        return Ok(0);
    };
    let selection = document.selections.primary();
    let (first, last) = ops::selection_lines(&document.text, selection);
    let doc_view = view(document, &snap);
    let options = services().indent_options(&doc_view);
    let fixes = services().reindent_lines(&doc_view, first..last + 1);
    if fixes.is_empty() {
        return Ok(0);
    }
    let text = snap.text();
    let lines = LineIndex::new(text);
    let mut changes = Vec::new();
    for (line, columns) in &fixes {
        let range = lines.line_range(*line, text);
        let current = indent_bytes(&text[range.clone()]);
        changes.push(TextChange {
            start_char: document.text.byte_to_char(range.start),
            end_char: document.text.byte_to_char(range.start + current),
            inserted: options.indent_string(*columns),
        });
    }
    let count = changes.len();
    document.text.edit_many(changes).map_err(|e| e.to_string())?;
    Ok(count)
}

fn next_method(document: &mut Document, next: bool) -> bool {
    let Some(snap) = snapshot(document) else {
        return false;
    };
    let caret = document.text.char_to_byte(document.selections.primary().head);
    let doc_view = view(document, &snap);
    match services().method_boundary(&doc_view, caret, next) {
        Some(target) => {
            move_caret(document, target, false);
            true
        }
        None => false,
    }
}

fn code_block(document: &mut Document, start: bool, select: bool) -> bool {
    let Some(snap) = snapshot(document) else {
        return false;
    };
    let caret = document.text.char_to_byte(document.selections.primary().head);
    let doc_view = view(document, &snap);
    let mut at = caret;
    // Walk outward until the target lies on the requested side of the caret.
    for _ in 0..64 {
        let Some(block) = services().code_block(&doc_view, at) else {
            return false;
        };
        let target = if start { block.start } else { block.end + 1 };
        let beyond = if start { target < caret } else { target > caret };
        if beyond {
            move_caret(document, target, select);
            return true;
        }
        if block.start == 0 {
            return false;
        }
        at = block.start;
    }
    false
}

fn match_brace(document: &mut Document, select: bool) -> bool {
    let Some(snap) = snapshot(document) else {
        return false;
    };
    let head = document.text.char_to_byte(document.selections.primary().head);
    let (pair, after) = match brackets::pair_at(&snap, head) {
        Some(pair) => (pair, false),
        None => match head.checked_sub(1).and_then(|at| brackets::pair_at(&snap, at)) {
            Some(pair) => (pair, true),
            None => return false,
        },
    };
    let at_open = if after { pair.open == head - 1 } else { pair.open == head };
    let target = if at_open {
        let closer = &snap.text()[pair.close..];
        pair.close + closer.chars().next().map_or(1, char::len_utf8)
    } else {
        pair.open
    };
    move_caret(document, target, select);
    true
}

fn register_document_command(
    registry: &mut CommandRegistry<IdeState>,
    id: &str,
    label: &str,
    shortcut: Option<&str>,
    run: fn(&mut Document) -> Result<Value, String>,
) {
    registry
        .register(Command::new(
            spec(id, label, "Editor", shortcut, "{}"),
            |state: &IdeState| state.active_document.is_some(),
            move |state: &mut IdeState, _| {
                state
                    .edit_active(|document| run(document))
                    .map_err(CommandError::Failed)
            },
        ))
        .expect("unique built-in id");
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    register_document_command(
        registry,
        "editor.completeStatement",
        "Complete Current Statement",
        Some("⇧⌘↩"),
        |d| complete_statement(d).map(|()| Value::Null),
    );
    register_document_command(
        registry,
        "editor.moveStatementUp",
        "Move Statement Up",
        Some("⇧⌘↑"),
        |d| move_statement(d, true).map(|()| Value::Null),
    );
    register_document_command(
        registry,
        "editor.moveStatementDown",
        "Move Statement Down",
        Some("⇧⌘↓"),
        |d| move_statement(d, false).map(|()| Value::Null),
    );
    register_document_command(
        registry,
        "editor.commentBlock",
        "Comment with Block Comment",
        Some("⌥⌘/"),
        |d| comment_block(d).map(|()| Value::Null),
    );
    register_document_command(
        registry,
        "editor.autoIndent",
        "Auto-Indent Lines",
        Some("⌃⌥I"),
        |d| auto_indent(d).map(|changed| json!({"changed": changed})),
    );
    register_document_command(registry, "editor.nextMethod", "Move Caret to Next Method", Some("⌃↓"), |d| {
        Ok(json!({"moved": next_method(d, true)}))
    });
    register_document_command(
        registry,
        "editor.previousMethod",
        "Move Caret to Previous Method",
        Some("⌃↑"),
        |d| Ok(json!({"moved": next_method(d, false)})),
    );
    register_document_command(
        registry,
        "editor.codeBlockStart",
        "Move Caret to Code Block Start",
        Some("⌃⌘["),
        |d| Ok(json!({"moved": code_block(d, true, false)})),
    );
    register_document_command(
        registry,
        "editor.codeBlockEnd",
        "Move Caret to Code Block End",
        Some("⌃⌘]"),
        |d| Ok(json!({"moved": code_block(d, false, false)})),
    );
    register_document_command(
        registry,
        "editor.codeBlockStartSelect",
        "Move Caret to Code Block Start with Selection",
        None,
        |d| Ok(json!({"moved": code_block(d, true, true)})),
    );
    register_document_command(
        registry,
        "editor.codeBlockEndSelect",
        "Move Caret to Code Block End with Selection",
        None,
        |d| Ok(json!({"moved": code_block(d, false, true)})),
    );
    register_document_command(
        registry,
        "editor.matchBrace",
        "Move Caret to Matching Brace",
        Some("⇧⌘M"),
        |d| Ok(json!({"moved": match_brace(d, false)})),
    );
    registry
        .register(Command::new(
            spec(
                "editor.stickyScopes",
                "Sticky Scopes at Line",
                "Editor",
                None,
                r#"{"firstLine":"number","max":"number?"}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let first = crate::usize_param(&params, "firstLine")?;
                let max = params
                    .get("max")
                    .and_then(Value::as_u64)
                    .map_or(archipel_ide_language::sticky::DEFAULT_MAX_STICKY, |m| m as usize);
                let document = state
                    .active_document
                    .and_then(|index| state.documents.get_mut(index))
                    .ok_or_else(|| CommandError::Disabled("no active document".into()))?;
                let lines: Vec<Value> = sticky_lines(document, first, max)
                    .into_iter()
                    .map(|s| json!({"text": s.text, "line": s.line, "endLine": s.end_line}))
                    .collect();
                Ok(json!({"lines": lines}))
            },
        ))
        .expect("unique built-in id");
}

// ---- generic services of the analysis registry (other packages and
// language crates look these up per language) --------------------------------

fn edit_set(set: EditSet) -> contracts::EditSet {
    contracts::EditSet {
        edits: set
            .edits
            .into_iter()
            .map(|e| TextEdit::new(e.start as u32..e.end as u32, e.text))
            .collect(),
        caret: set.caret.map(|c| c as u32),
        selection: set.selection.map(|s| s.start as u32..s.end as u32),
    }
}

fn indent_options(options: &contracts::IndentOptions) -> IndentOptions {
    IndentOptions {
        indent_size: options.indent_size as usize,
        tab_size: options.tab_size as usize,
        use_tabs: options.use_tabs,
    }
}

struct LanguageCommenter(&'static archipel_ide_language::syntax_data::LanguageData);

impl contracts::Commenter for LanguageCommenter {
    fn line_comment_prefix(&self) -> Option<&str> {
        self.0.line_prefix()
    }
    fn block_comment(&self) -> Option<(&str, &str)> {
        self.0.block_comment
    }
    fn doc_comment_prefix(&self) -> Option<&str> {
        self.0.doc_line_prefix.or(self.0.doc_block_open)
    }
}

struct QueryIndenter;

impl contracts::Indenter for QueryIndenter {
    fn indent_for_line(
        &self,
        syntax: &SyntaxSnapshot,
        line: u32,
        options: &contracts::IndentOptions,
    ) -> Option<u32> {
        let engine = archipel_ide_language::indent::IndentEngine::new(syntax, indent_options(options));
        engine
            .for_line(line as usize, &|row| engine.indent_of(row))
            .map(|columns| columns as u32)
    }
}

struct QueryTyping;

impl contracts::TypedHandler for QueryTyping {
    fn before_typed(&self, syntax: &SyntaxSnapshot, caret: u32, typed: char) -> contracts::TypedDecision {
        match archipel_ide_language::typing::before_typed(syntax, caret as usize, None, typed) {
            TypedDecision::Default => contracts::TypedDecision::Default,
            TypedDecision::Overtype => contracts::TypedDecision::Overtype,
            TypedDecision::InsertPair(c) => contracts::TypedDecision::InsertPair(c),
            TypedDecision::Wrap { open, close } => contracts::TypedDecision::Wrap { open, close },
            TypedDecision::Consume => contracts::TypedDecision::Consume,
        }
    }

    fn after_typed(&self, syntax: &SyntaxSnapshot, caret: u32, typed: char) -> Option<contracts::EditSet> {
        let options = IndentOptions::detect(syntax.text());
        archipel_ide_language::typing::after_typed(syntax, caret as usize, typed, options).map(edit_set)
    }

    fn on_enter(
        &self,
        syntax: &SyntaxSnapshot,
        caret: u32,
        options: &contracts::IndentOptions,
    ) -> Option<contracts::EditSet> {
        let caret = caret as usize;
        archipel_ide_language::typing::on_enter(syntax, caret..caret, indent_options(options)).map(edit_set)
    }
}

struct QuerySmartEnter;

impl contracts::SmartEnterProcessor for QuerySmartEnter {
    fn smart_enter(&self, syntax: &SyntaxSnapshot, caret: u32) -> Option<contracts::EditSet> {
        let options = IndentOptions::detect(syntax.text());
        archipel_ide_language::smart_enter::smart_enter(syntax, caret as usize, options).map(edit_set)
    }
}

struct QueryMoveStatement;

impl contracts::MoveStatementHandler for QueryMoveStatement {
    fn move_statement(
        &self,
        syntax: &SyntaxSnapshot,
        range: Range<u32>,
        up: bool,
    ) -> Option<contracts::EditSet> {
        archipel_ide_language::move_statement::move_statement(
            syntax,
            range.start as usize..range.end as usize,
            up,
        )
        .map(edit_set)
    }
}

struct QuerySticky;

impl contracts::StickyScopesProvider for QuerySticky {
    fn sticky_scopes(
        &self,
        syntax: &SyntaxSnapshot,
        first_visible_line: u32,
        max: usize,
    ) -> Vec<contracts::StickyScope> {
        archipel_ide_language::sticky::sticky_scopes(syntax, first_visible_line as usize, max)
            .into_iter()
            .map(|scope| contracts::StickyScope {
                header: scope.header.start as u32..scope.header.end as u32,
                start_line: scope.start_line as u32,
                end_line: scope.end_line as u32,
            })
            .collect()
    }
}

struct QueryBreadcrumbs;

impl contracts::BreadcrumbsProvider for QueryBreadcrumbs {
    fn breadcrumbs(&self, syntax: &SyntaxSnapshot, offset: u32) -> Vec<contracts::Crumb> {
        let items = syntax.structure();
        archipel_ide_language::structure::path_at(&items, offset as usize)
            .into_iter()
            .map(|item| contracts::Crumb {
                label: item.name.clone(),
                kind: Some(item.kind),
                range: item.range.start as u32..item.range.end as u32,
            })
            .collect()
    }
}

/// Registers the query-driven services for every language (dumb-aware: they
/// need no index) and each language's commenter.
pub(crate) fn register_services(registry: &mut ServiceRegistry) {
    registry.register::<dyn contracts::Indenter>(
        Registration::any("query-indenter").dumb_aware(true),
        Arc::new(QueryIndenter),
    );
    registry.register::<dyn contracts::TypedHandler>(
        Registration::any("query-typing").dumb_aware(true),
        Arc::new(QueryTyping),
    );
    registry.register::<dyn contracts::SmartEnterProcessor>(
        Registration::any("query-smart-enter").dumb_aware(true),
        Arc::new(QuerySmartEnter),
    );
    registry.register::<dyn contracts::MoveStatementHandler>(
        Registration::any("query-move-statement").dumb_aware(true),
        Arc::new(QueryMoveStatement),
    );
    registry.register::<dyn contracts::StickyScopesProvider>(
        Registration::any("query-sticky").dumb_aware(true),
        Arc::new(QuerySticky),
    );
    registry.register::<dyn contracts::BreadcrumbsProvider>(
        Registration::any("query-breadcrumbs").dumb_aware(true),
        Arc::new(QueryBreadcrumbs),
    );
    for spec in LANGUAGES {
        if let Some(data) = archipel_ide_language::syntax_data::language_data(spec.id) {
            registry.register::<dyn contracts::Commenter>(
                Registration::new("language-commenter", spec.language_id()).dumb_aware(true),
                Arc::new(LanguageCommenter(data)),
            );
        }
    }
}
