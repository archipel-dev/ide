//! Problems: syntax errors and language-server diagnostics merged into one
//! model for the editor highlighting, the inspection widget, F2/⇧F2
//! navigation, the Problems tool window and error propagation in the
//! Project tree and editor tabs.
//!
//! Positions are 0-based lines and *character* columns (the editor's unit),
//! converted from LSP UTF-16 columns and tree-sitter byte columns. For files
//! that are not open the LSP column is kept as reported.
use crate::{Document, IdeState, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_lsp::LspDiagnostic;
use archipel_ide_text::{Selection, TextDocument};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Highlighting severity, mirroring the IntelliJ levels the editor shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
    WeakWarning,
    /// LSP hints: rendered (e.g. unused code greyed out) but not listed.
    Hint,
}

impl Severity {
    fn from_lsp(severity: Option<u8>) -> Self {
        match severity {
            Some(1) | None => Self::Error,
            Some(2) => Self::Warning,
            Some(3) => Self::WeakWarning,
            _ => Self::Hint,
        }
    }

    /// Whether the Problems tool window lists it and counters include it.
    pub fn listed(self) -> bool {
        self != Self::Hint
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub severity: Severity,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<String>,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    /// Unused code, drawn greyed out.
    pub unnecessary: bool,
    /// Deprecated API, drawn struck through.
    pub deprecated: bool,
}

impl Problem {
    /// Whether the problem covers (or, when empty, touches) `line`.
    pub fn on_line(&self, line: usize) -> bool {
        self.line <= line && line <= self.end_line
    }
}

/// Problem counts for the inspection widget and the tool window tabs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemCounts {
    pub errors: usize,
    pub warnings: usize,
    pub weak_warnings: usize,
}

impl ProblemCounts {
    pub fn of<'a>(problems: impl IntoIterator<Item = &'a Problem>) -> Self {
        let mut counts = Self::default();
        for problem in problems {
            match problem.severity {
                Severity::Error => counts.errors += 1,
                Severity::Warning => counts.warnings += 1,
                Severity::WeakWarning => counts.weak_warnings += 1,
                Severity::Hint => {}
            }
        }
        counts
    }

    pub fn total(self) -> usize {
        self.errors + self.warnings + self.weak_warnings
    }
}

/// Problems of one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileProblems {
    /// Project-relative path.
    pub path: PathBuf,
    pub problems: Vec<Problem>,
}

/// Project-wide problems, rebuilt when diagnostics or documents change.
#[derive(Debug, Clone, Default)]
pub struct ProblemIndex {
    files: Vec<FileProblems>,
    /// Files with errors and all of their ancestor directories.
    error_paths: BTreeSet<PathBuf>,
    key: Option<u64>,
}

impl ProblemIndex {
    /// Files with at least one listed problem, sorted by path.
    pub fn files(&self) -> &[FileProblems] {
        &self.files
    }

    pub fn file(&self, path: &Path) -> Option<&FileProblems> {
        self.files.iter().find(|file| file.path == path)
    }

    pub fn counts(&self) -> ProblemCounts {
        ProblemCounts::of(self.files.iter().flat_map(|file| &file.problems))
    }

    /// Whether `path` (file or directory, project-relative) contains an error;
    /// the tree and tabs underline such entries with a red wave.
    pub fn has_errors(&self, path: &Path) -> bool {
        self.error_paths.contains(path)
    }
}

/// Converts a UTF-16 column on `line` to a character column.
fn utf16_to_char(text: &TextDocument, line: usize, utf16: usize) -> usize {
    let Some(content) = text.line(line) else {
        return utf16;
    };
    let mut units = 0;
    for (index, c) in content.trim_end_matches(['\r', '\n']).chars().enumerate() {
        if units >= utf16 {
            return index;
        }
        units += c.len_utf16();
    }
    content.trim_end_matches(['\r', '\n']).chars().count()
}

fn byte_to_char(text: &TextDocument, line: usize, byte: usize) -> usize {
    let Some(content) = text.line(line) else {
        return byte;
    };
    content
        .char_indices()
        .take_while(|(index, _)| *index < byte)
        .count()
}

fn from_lsp(diagnostic: &LspDiagnostic, text: Option<&TextDocument>) -> Problem {
    let column = |line, character| match text {
        Some(text) => utf16_to_char(text, line, character),
        None => character,
    };
    Problem {
        severity: Severity::from_lsp(diagnostic.severity),
        message: diagnostic.message.clone(),
        source: diagnostic.source.clone(),
        code: diagnostic.code.clone(),
        line: diagnostic.start_line,
        column: column(diagnostic.start_line, diagnostic.start_character),
        end_line: diagnostic.end_line,
        end_column: column(diagnostic.end_line, diagnostic.end_character),
        unnecessary: diagnostic.unnecessary,
        deprecated: diagnostic.deprecated,
    }
}

/// Syntax errors from the tree-sitter parse, one character wide.
fn syntax_problems(document: &Document) -> Vec<Problem> {
    document
        .analysis
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let line = diagnostic.row.saturating_sub(1);
            let column = byte_to_char(&document.text, line, diagnostic.column.saturating_sub(1));
            Problem {
                severity: Severity::Error,
                message: diagnostic.message.clone(),
                source: Some("syntax".into()),
                code: None,
                line,
                column,
                end_line: line,
                end_column: column + 1,
                unnecessary: false,
                deprecated: false,
            }
        })
        .collect()
}

fn sort(problems: &mut [Problem]) {
    problems.sort_by(|a, b| {
        (a.line, a.column, a.severity, &a.message).cmp(&(b.line, b.column, b.severity, &b.message))
    });
}

/// Problems of an open document. Once its language server has checked the
/// file the server's diagnostics replace the syntax errors (servers report
/// syntax errors themselves); until then the parse errors are shown.
pub fn document_problems(document: &Document, published: Option<&[LspDiagnostic]>) -> Vec<Problem> {
    let mut problems = match published {
        Some(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| from_lsp(diagnostic, Some(&document.text)))
            .collect(),
        None => syntax_problems(document),
    };
    sort(&mut problems);
    problems
}

/// Problems of an open document from the daemon's markup (`native`) and the
/// server's diagnostics. The daemon's own syntax errors give way to the
/// server's once it has published (servers report syntax errors themselves);
/// without daemon data the parse errors of the editor are shown.
pub fn document_problems_with(
    document: &Document,
    published: Option<&[LspDiagnostic]>,
    native: Option<&[Problem]>,
) -> Vec<Problem> {
    let Some(native) = native else {
        return document_problems(document, published);
    };
    let mut problems: Vec<Problem> = match published {
        Some(diagnostics) => {
            let mut all: Vec<Problem> = diagnostics
                .iter()
                .map(|diagnostic| from_lsp(diagnostic, Some(&document.text)))
                .collect();
            all.extend(
                native
                    .iter()
                    .filter(|p| p.source.as_deref() != Some("syntax"))
                    .cloned(),
            );
            all
        }
        None => native.to_vec(),
    };
    sort(&mut problems);
    problems
}

/// Builds the project-wide index from open documents and every published
/// diagnostic. `root` must be canonical, like the paths servers publish.
#[cfg(test)]
pub fn build_index<'a>(
    root: &Path,
    documents: &[Document],
    published: impl IntoIterator<Item = (&'a Path, &'a [LspDiagnostic])>,
) -> ProblemIndex {
    build_index_with(root, documents, published, &BTreeMap::new())
}

/// [`build_index`] with the daemon's problems of the open documents (by path).
pub fn build_index_with<'a>(
    root: &Path,
    documents: &[Document],
    published: impl IntoIterator<Item = (&'a Path, &'a [LspDiagnostic])>,
    native: &BTreeMap<PathBuf, Vec<Problem>>,
) -> ProblemIndex {
    let mut by_path: BTreeMap<PathBuf, Option<&'a [LspDiagnostic]>> = documents
        .iter()
        .map(|document| (document.path.clone(), None))
        .collect();
    for (path, diagnostics) in published {
        if let Ok(relative) = path.strip_prefix(root) {
            by_path.insert(relative.to_owned(), Some(diagnostics));
        }
    }
    let mut files = Vec::new();
    let mut error_paths = BTreeSet::new();
    for (path, published) in by_path {
        let mut problems = match documents.iter().find(|document| document.path == path) {
            Some(document) => {
                document_problems_with(document, published, native.get(&path).map(Vec::as_slice))
            }
            None => {
                let mut problems: Vec<Problem> = published
                    .unwrap_or_default()
                    .iter()
                    .map(|diagnostic| from_lsp(diagnostic, None))
                    .collect();
                sort(&mut problems);
                problems
            }
        };
        problems.retain(|problem| problem.severity.listed());
        if problems.is_empty() {
            continue;
        }
        if problems
            .iter()
            .any(|problem| problem.severity == Severity::Error)
        {
            error_paths.extend(path.ancestors().map(Path::to_path_buf));
        }
        files.push(FileProblems { path, problems });
    }
    error_paths.remove(Path::new(""));
    ProblemIndex {
        files,
        error_paths,
        key: None,
    }
}

/// Problems of the active document and the state they were computed for.
pub(crate) struct ActiveProblemsCache {
    key: (usize, PathBuf, u64, u64),
    problems: std::sync::Arc<Vec<Problem>>,
}

/// Converts a highlight into a problem of `document`.
fn from_highlight(
    document: &Document,
    info: &archipel_ide_analysis::HighlightInfo,
) -> Option<Problem> {
    use archipel_ide_analysis::HighlightKind;
    let severity = match info.kind {
        HighlightKind::Unused | HighlightKind::Deprecated | HighlightKind::MarkedForRemoval
            if !info.severity.is_problem() =>
        {
            Severity::Hint
        }
        _ if info.severity >= archipel_ide_analysis::Severity::ERROR => Severity::Error,
        _ if info.severity >= archipel_ide_analysis::Severity::WARNING => Severity::Warning,
        _ if info.severity >= archipel_ide_analysis::Severity::WEAK_WARNING => {
            Severity::WeakWarning
        }
        _ => return None,
    };
    let text = &document.text;
    let to_char =
        |byte: u32| text.byte_to_char((byte as usize).min(text.len_chars().saturating_mul(4)));
    let (start, end) = (
        to_char(info.range.start).min(text.len_chars()),
        to_char(info.range.end).min(text.len_chars()),
    );
    let (line, column) = text.char_to_line_column(start).unwrap_or((0, 0));
    let (end_line, end_column) = text.char_to_line_column(end).unwrap_or((line, column));
    let tool = info.tool.as_str();
    let tool = tool.strip_prefix("native:").unwrap_or(tool);
    Some(Problem {
        severity,
        message: info.message.as_deref().unwrap_or("").to_owned(),
        source: Some(tool.to_owned()),
        code: None,
        line,
        column,
        end_line,
        end_column,
        unnecessary: matches!(info.kind, HighlightKind::Unused),
        deprecated: matches!(
            info.kind,
            HighlightKind::Deprecated | HighlightKind::MarkedForRemoval
        ),
    })
}

impl IdeState {
    /// The daemon's problems of an open document, converted to lines and
    /// columns; `None` while the daemon has nothing for the current text.
    pub(crate) fn native_problems(&self, document: &Document) -> Option<Vec<Problem>> {
        let service = self.daemon_service()?;
        let file = self.analysis.host.lookup(&document.path)?;
        let daemon = service.lock();
        let markup = daemon.markup(file)?;
        if markup.rev() != archipel_ide_analysis::FileRev(document.text.revision()) {
            return None;
        }
        Some(
            markup
                .iter()
                .filter(|info| !info.file_level || info.severity.is_problem())
                .filter(|info| {
                    !matches!(
                        info.kind,
                        archipel_ide_analysis::HighlightKind::Semantic(_)
                            | archipel_ide_analysis::HighlightKind::TextAttributes(_)
                    )
                })
                .filter_map(|info| from_highlight(document, info))
                .collect(),
        )
    }

    /// [`IdeState::active_problems`], recomputed only when the document revision
    /// or the diagnostics generation changed.
    pub fn cached_active_problems(&mut self) -> std::sync::Arc<Vec<Problem>> {
        let Some(index) = self.active_document else {
            return Default::default();
        };
        let Some(document) = self.documents.get(index) else {
            return Default::default();
        };
        let key = (
            index,
            document.path.clone(),
            document.text.revision(),
            self.lsp.diagnostics_generation() ^ self.daemon_version().rotate_left(32),
        );
        // One entry per document, so split groups showing different files
        // don't evict each other.
        if let Some(cache) = self.problems_cache.get(&key.1)
            && cache.key == key
        {
            return cache.problems.clone();
        }
        let problems = std::sync::Arc::new(self.problems_for_document(index));
        if self.problems_cache.len() >= 16 {
            self.problems_cache.clear();
        }
        self.problems_cache.insert(
            key.1.clone(),
            ActiveProblemsCache {
                key,
                problems: problems.clone(),
            },
        );
        problems
    }

    /// All problems (including hints) of an open document, sorted by position.
    pub fn problems_for_document(&self, index: usize) -> Vec<Problem> {
        let (Some(document), Some(project)) = (self.documents.get(index), &self.project) else {
            return Vec::new();
        };
        let published = self
            .lsp
            .published_diagnostics(&project.root().join(&document.path));
        let native = self.native_problems(document);
        document_problems_with(document, published.as_deref(), native.as_deref())
    }

    pub fn active_problems(&self) -> Vec<Problem> {
        self.active_document
            .map(|index| self.problems_for_document(index))
            .unwrap_or_default()
    }

    /// Rebuilds [`IdeState::problems`] when diagnostics or documents changed.
    pub fn refresh_problems(&mut self) {
        let Some(project) = &self.project else {
            self.problems = ProblemIndex::default();
            return;
        };
        let key = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.lsp.diagnostics_generation().hash(&mut hasher);
            self.daemon_version().hash(&mut hasher);
            for document in &self.documents {
                document.path.hash(&mut hasher);
                document.text.revision().hash(&mut hasher);
            }
            hasher.finish()
        };
        if self.problems.key == Some(key) {
            return;
        }
        let root = project
            .root()
            .canonicalize()
            .unwrap_or_else(|_| project.root().to_owned());
        let native: BTreeMap<PathBuf, Vec<Problem>> = self
            .documents
            .iter()
            .filter_map(|document| Some((document.path.clone(), self.native_problems(document)?)))
            .collect();
        self.problems =
            build_index_with(&root, &self.documents, self.lsp.all_diagnostics(), &native);
        self.problems.key = Some(key);
    }

    /// Moves the caret to the next (or previous) problem in the active file,
    /// wrapping around. Errors are visited first: warnings are only reached
    /// when the file has no errors, like the reference IDE's default.
    pub fn goto_problem(&mut self, forward: bool) -> Result<Problem, String> {
        let index = self.active_document.ok_or("no active document")?;
        let problems: Vec<Problem> = self
            .problems_for_document(index)
            .into_iter()
            .filter(|problem| problem.severity.listed())
            .collect();
        let has_errors = problems.iter().any(|p| p.severity == Severity::Error);
        let targets: Vec<&Problem> = problems
            .iter()
            .filter(|p| !has_errors || p.severity == Severity::Error)
            .collect();
        let document = &mut self.documents[index];
        let offset = |problem: &Problem| {
            document
                .text
                .line_to_char(problem.line)
                .map(|start| start + problem.column)
                .unwrap_or(0)
                .min(document.text.len_chars())
        };
        let caret = document.selections.primary().head;
        let target = if forward {
            targets
                .iter()
                .find(|problem| offset(problem) > caret)
                .or_else(|| targets.first())
        } else {
            targets
                .iter()
                .rev()
                .find(|problem| offset(problem) < caret)
                .or_else(|| targets.last())
        }
        .copied()
        .cloned()
        .ok_or("No problems found in this file")?;
        let position = offset(&target);
        document.selections.set_single(Selection::caret(position));
        self.status = target.message.lines().next().unwrap_or_default().into();
        Ok(target)
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    for (id, label, shortcut, forward) in [
        ("editor.nextProblem", "Next Highlighted Error", "F2", true),
        (
            "editor.previousProblem",
            "Previous Highlighted Error",
            "⇧F2",
            false,
        ),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Navigate", Some(shortcut), "{}"),
                |state: &IdeState| state.active_document.is_some(),
                move |state: &mut IdeState, _| {
                    let problem = state.goto_problem(forward).map_err(CommandError::Failed)?;
                    Ok(json!(problem))
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "problems.list",
                "List Problems",
                "Code",
                None,
                r#"{"scope":"file|project?"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let scope = params
                    .get("scope")
                    .and_then(Value::as_str)
                    .unwrap_or("project");
                match scope {
                    "file" => {
                        let index = state
                            .active_document
                            .ok_or_else(|| CommandError::Failed("no active document".into()))?;
                        let problems: Vec<Problem> = state
                            .problems_for_document(index)
                            .into_iter()
                            .filter(|problem| problem.severity.listed())
                            .collect();
                        Ok(json!({
                            "path": state.documents[index].path,
                            "counts": ProblemCounts::of(&problems),
                            "problems": problems,
                        }))
                    }
                    "project" => {
                        state.refresh_problems();
                        Ok(json!({
                            "counts": state.problems.counts(),
                            "files": state.problems.files(),
                        }))
                    }
                    other => Err(CommandError::InvalidParams(format!(
                        "unknown scope `{other}`; expected `file` or `project`"
                    ))),
                }
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn diagnostic(severity: u8, line: usize, start: usize, end: usize) -> LspDiagnostic {
        LspDiagnostic {
            message: format!("severity {severity}"),
            severity: Some(severity),
            start_line: line,
            start_character: start,
            end_line: line,
            end_character: end,
            code: None,
            source: Some("test".into()),
            unnecessary: false,
            deprecated: false,
            raw: serde_json::Value::Null,
        }
    }

    fn document(path: &str, text: &str) -> Document {
        Document::new(path.into(), text)
    }

    #[test]
    fn active_problem_cache_follows_the_document_revision() {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-problems-cache-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("main.rs"), "fn main() { let = ; }\n").unwrap();
        let mut state = IdeState::default();
        state.open_project(&root).unwrap();
        state.open_file(Path::new("main.rs")).unwrap();
        let first = state.cached_active_problems();
        assert!(!first.is_empty());
        assert!(std::sync::Arc::ptr_eq(
            &first,
            &state.cached_active_problems()
        ));
        built_in_commands()
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 0, "text": "// fixed\nfn ok() {}\n"}),
            )
            .unwrap();
        let second = state.cached_active_problems();
        assert!(!std::sync::Arc::ptr_eq(&first, &second));
        assert_eq!(*second, state.active_problems());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lsp_columns_are_converted_from_utf16_to_characters() {
        // "𝔸" is two UTF-16 units but one character.
        let doc = document("a.txt", "let 𝔸 = geet();\n");
        let problems = document_problems(&doc, Some(&[diagnostic(1, 0, 9, 13)]));
        assert_eq!((problems[0].column, problems[0].end_column), (8, 12));
    }

    #[test]
    fn syntax_errors_are_shown_until_the_server_has_checked_the_file() {
        let doc = document("main.rs", "fn main() { let = ; }\n");
        let syntax = document_problems(&doc, None);
        assert!(!syntax.is_empty());
        assert!(syntax.iter().all(|p| p.source.as_deref() == Some("syntax")));
        let lsp = document_problems(&doc, Some(&[diagnostic(2, 0, 0, 2)]));
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].severity, Severity::Warning);
        assert!(document_problems(&doc, Some(&[])).is_empty());
    }

    #[test]
    fn index_groups_files_counts_and_propagates_errors_to_ancestors() {
        let root = Path::new("/project");
        let open = document("src/main.rs", "fn main() {}\n");
        let error_file = root.join("src/app/App.java");
        let warning_file = root.join("build.rs");
        let outside = Path::new("/elsewhere/lib.rs");
        let error = [diagnostic(1, 3, 0, 4), diagnostic(4, 3, 0, 4)];
        let warning = [diagnostic(2, 0, 0, 1), diagnostic(3, 1, 0, 1)];
        let other = [diagnostic(1, 0, 0, 1)];
        let index = build_index(
            root,
            std::slice::from_ref(&open),
            [
                (error_file.as_path(), &error[..]),
                (warning_file.as_path(), &warning[..]),
                (outside, &other[..]),
            ],
        );
        let paths: Vec<_> = index.files().iter().map(|f| f.path.clone()).collect();
        assert_eq!(
            paths,
            [PathBuf::from("build.rs"), PathBuf::from("src/app/App.java")]
        );
        // Hints are rendered but not listed.
        assert_eq!(
            index
                .file(Path::new("src/app/App.java"))
                .unwrap()
                .problems
                .len(),
            1
        );
        assert_eq!(
            index.counts(),
            ProblemCounts {
                errors: 1,
                warnings: 1,
                weak_warnings: 1
            }
        );
        for path in ["src/app/App.java", "src/app", "src"] {
            assert!(index.has_errors(Path::new(path)), "{path}");
        }
        assert!(!index.has_errors(Path::new("build.rs")));
        assert!(!index.has_errors(Path::new("")));
    }

    #[test]
    fn f2_visits_errors_first_and_wraps() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-problems-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("main.rs"),
            "fn a() {}\nfn b( {}\nfn c() {}\nfn d( {}\n",
        )
        .unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "main.rs"}))
            .unwrap();
        let line = |state: &IdeState| {
            let document = &state.documents[0];
            document
                .text
                .char_to_line(document.selections.primary().head)
        };
        let first = commands
            .execute("editor.nextProblem", &mut state, json!({}))
            .unwrap();
        assert_eq!(first["severity"], "error");
        assert_eq!(line(&state), 1);
        commands
            .execute("editor.nextProblem", &mut state, json!({}))
            .unwrap();
        assert_eq!(line(&state), 3);
        commands
            .execute("editor.nextProblem", &mut state, json!({}))
            .unwrap();
        assert_eq!(line(&state), 1, "wraps to the first problem");
        commands
            .execute("editor.previousProblem", &mut state, json!({}))
            .unwrap();
        assert_eq!(line(&state), 3, "wraps backwards");
        let listed = commands
            .execute("problems.list", &mut state, json!({"scope": "project"}))
            .unwrap();
        assert_eq!(listed["files"][0]["path"], "main.rs");
        assert_eq!(listed["counts"]["errors"], 2);
        assert!(state.problems.has_errors(Path::new("main.rs")));
        let file = commands
            .execute("problems.list", &mut state, json!({"scope": "file"}))
            .unwrap();
        assert_eq!(file["counts"]["errors"], 2);
        assert!(
            commands
                .execute("problems.list", &mut state, json!({"scope": "module"}))
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
