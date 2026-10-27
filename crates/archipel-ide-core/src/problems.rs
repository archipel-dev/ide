//! Problems: the model behind the editor squiggles, the inspection widget,
//! F2/⇧F2 navigation, the Problems tool window and error propagation in the
//! Project tree and editor tabs.
//!
//! The daemon is the only producer of editor markup: parse errors, native
//! inspections and the language servers' diagnostics (its external pass,
//! deduplicated against the native results) all arrive as highlights in the
//! daemon's markup, and this module only converts that markup into lines and
//! columns. Files that are not open have no markup; their entries come from
//! the daemon's external feed with the columns as the server reported them.
//!
//! Positions are 0-based lines and *character* columns (the editor's unit).
use crate::{Document, IdeState, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_daemon::{ExternalDiagnostic, ExternalFile};
use archipel_ide_text::Selection;
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
    fn from_lsp(severity: u8) -> Self {
        match severity {
            1 => Self::Error,
            2 => Self::Warning,
            3 => Self::WeakWarning,
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

fn sort(problems: &mut [Problem]) {
    problems.sort_by(|a, b| {
        (a.line, a.column, a.severity, &a.message).cmp(&(b.line, b.column, b.severity, &b.message))
    });
}

/// A published diagnostic of a file that is not open (columns as reported).
fn from_external(diagnostic: &ExternalDiagnostic) -> Problem {
    Problem {
        severity: Severity::from_lsp(diagnostic.severity),
        message: diagnostic.message.clone(),
        source: diagnostic.source.clone(),
        code: diagnostic.code.clone(),
        line: diagnostic.start.0 as usize,
        column: diagnostic.start.1 as usize,
        end_line: diagnostic.end.0 as usize,
        end_column: diagnostic.end.1 as usize,
        unnecessary: diagnostic.unnecessary,
        deprecated: diagnostic.deprecated,
    }
}

/// Builds the project-wide index from the daemon's problems of the open
/// documents (by path) and the external feed's entries of the other files.
/// `root` must be canonical.
pub fn build_index(
    documents: &[Document],
    markup: &BTreeMap<PathBuf, Vec<Problem>>,
    external: &[(PathBuf, ExternalFile)],
) -> ProblemIndex {
    let mut by_path: BTreeMap<PathBuf, Vec<Problem>> = BTreeMap::new();
    for document in documents {
        by_path.insert(
            document.path.clone(),
            markup.get(&document.path).cloned().unwrap_or_default(),
        );
    }
    for (path, file) in external {
        if by_path.contains_key(path) {
            continue;
        }
        by_path.insert(
            path.clone(),
            file.diagnostics.iter().map(from_external).collect(),
        );
    }
    let mut files = Vec::new();
    let mut error_paths = BTreeSet::new();
    for (path, mut problems) in by_path {
        sort(&mut problems);
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
    let tool = tool
        .strip_prefix("native:")
        .or_else(|| tool.strip_prefix("lsp:"))
        .unwrap_or(tool);
    Some(Problem {
        severity,
        message: info.message.as_deref().unwrap_or("").to_owned(),
        source: Some(info.source.as_deref().unwrap_or(tool).to_owned()),
        code: info.code.as_deref().map(str::to_owned),
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
    /// columns. Empty until the daemon has analyzed the file; while it
    /// catches up with an edit the markup (shifted through the edit journal at
    /// every tick) is used as it is.
    pub(crate) fn markup_problems(&self, document: &Document) -> Vec<Problem> {
        let Some(service) = self.daemon_service() else {
            return Vec::new();
        };
        let Some(file) = self.analysis.host.lookup(&document.path) else {
            return Vec::new();
        };
        let daemon = service.lock();
        let Some(markup) = daemon.markup(file) else {
            return Vec::new();
        };
        let mut problems: Vec<Problem> = markup
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
            .collect();
        sort(&mut problems);
        problems
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
            self.daemon_version(),
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
        self.documents
            .get(index)
            .map(|document| self.markup_problems(document))
            .unwrap_or_default()
    }

    pub fn active_problems(&self) -> Vec<Problem> {
        self.active_document
            .map(|index| self.problems_for_document(index))
            .unwrap_or_default()
    }

    /// Rebuilds [`IdeState::problems`] when the daemon's markup, the external
    /// feed or the documents changed.
    pub fn refresh_problems(&mut self) {
        let (Some(project), Some(service)) = (&self.project, self.daemon_service()) else {
            self.problems = ProblemIndex::default();
            return;
        };
        let feed = service.shared().external.clone();
        let key = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            feed.version().hash(&mut hasher);
            self.daemon_version().hash(&mut hasher);
            for document in &self.documents {
                document.path.hash(&mut hasher);
                document.text.revision().hash(&mut hasher);
            }
            hasher.finish()
        };
        let _ = project;
        if self.problems.key == Some(key) {
            return;
        }
        let markup: BTreeMap<PathBuf, Vec<Problem>> = self
            .documents
            .iter()
            .map(|document| (document.path.clone(), self.markup_problems(document)))
            .collect();
        self.problems = build_index(&self.documents, &markup, &feed.all());
        self.problems.key = Some(key);
    }

    /// Moves the caret to the next (or previous) problem in the active file,
    /// wrapping around. Errors are visited first: warnings are only reached
    /// when the file has no errors, like the reference IDE's default.
    pub fn goto_problem(&mut self, forward: bool) -> Result<Problem, String> {
        self.poll_analysis();
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
                state.poll_analysis();
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

    fn external(severity: u8, line: u32, start: u32, end: u32) -> ExternalDiagnostic {
        ExternalDiagnostic {
            range: 0..0,
            start: (line, start),
            end: (line, end),
            severity,
            message: format!("severity {severity}"),
            code: None,
            source: Some("test".into()),
            unnecessary: false,
            deprecated: false,
        }
    }

    fn file(diagnostics: Vec<ExternalDiagnostic>) -> ExternalFile {
        ExternalFile {
            server: "test".into(),
            rev: None,
            diagnostics: std::sync::Arc::new(diagnostics),
        }
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
        state.poll_analysis();
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
        state.poll_analysis();
        let second = state.cached_active_problems();
        assert!(!std::sync::Arc::ptr_eq(&first, &second));
        assert_eq!(*second, state.active_problems());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn index_groups_files_counts_and_propagates_errors_to_ancestors() {
        let open = Document::new("src/main.rs".into(), "fn main() {}\n");
        // Open documents come from the daemon's markup; the other files from
        // the external feed.
        let markup = BTreeMap::from([(PathBuf::from("src/main.rs"), Vec::new())]);
        let feed = [
            (
                PathBuf::from("src/app/App.java"),
                file(vec![external(1, 3, 0, 4), external(4, 3, 0, 4)]),
            ),
            (
                PathBuf::from("build.rs"),
                file(vec![external(2, 0, 0, 1), external(3, 1, 0, 1)]),
            ),
        ];
        let index = build_index(std::slice::from_ref(&open), &markup, &feed);
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
    fn the_daemons_problems_win_over_stale_feed_entries_of_open_files() {
        let open = Document::new("a.rs".into(), "fn a() {}\n");
        let markup = BTreeMap::from([(PathBuf::from("a.rs"), Vec::new())]);
        let feed = [(PathBuf::from("a.rs"), file(vec![external(1, 0, 0, 1)]))];
        let index = build_index(std::slice::from_ref(&open), &markup, &feed);
        assert!(index.files().is_empty(), "the markup is the one source");
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
