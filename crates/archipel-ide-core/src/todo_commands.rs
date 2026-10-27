//! TODO tool window model and commands (IJ-TW-030, IJ-SET-060): the rows
//! come from the `Todo` index (comment text only, patterns from Settings >
//! Editor > TODO), for the current file or the whole project, filtered by
//! the settings' filters and grouped by file.

use crate::navigation::NavigationItem;
use crate::{IdeState, ToolWindow, spec, string_param};
use archipel_ide_analysis::resolve::SearchScope;
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_index::ids;
use archipel_ide_index::readers;
use archipel_ide_index::todo::{
    TodoFilter, TodoPattern, TodoSettings, parse_filters, parse_patterns,
};
use archipel_ide_settings::ids as setting_ids;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

/// Which files the tool window lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TodoScope {
    /// The active editor tab.
    CurrentFile,
    #[default]
    Project,
}

impl TodoScope {
    pub fn id(self) -> &'static str {
        match self {
            TodoScope::CurrentFile => "file",
            TodoScope::Project => "project",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        match id {
            "file" => Some(TodoScope::CurrentFile),
            "project" => Some(TodoScope::Project),
            _ => None,
        }
    }
}

/// How rows are arranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TodoGroup {
    #[default]
    File,
    Flat,
}

/// One TODO occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoRow {
    /// Project-relative path.
    pub path: PathBuf,
    /// Zero-based line.
    pub line: usize,
    /// Byte offset of the match in the file as indexed.
    pub start: usize,
    pub text: String,
    /// Index of the matching pattern in the settings.
    pub pattern: usize,
}

/// What the tool window shows right now.
#[derive(Debug, Clone, Default)]
pub struct TodoView {
    pub rows: Arc<Vec<TodoRow>>,
    /// The `Todo` index is not ready (results may be incomplete).
    pub indexing: bool,
    pub patterns: Vec<TodoPattern>,
    pub filters: Vec<TodoFilter>,
}

/// The stored setting lines (patterns, filters) and the multiline flag.
type RawTodoSettings = (Vec<String>, Vec<String>, bool);

#[derive(Debug, Clone, PartialEq, Eq)]
struct CacheKey {
    stamp: u64,
    scope: TodoScope,
    file: Option<PathBuf>,
    filter: Option<usize>,
    settings: Vec<String>,
}

/// State of the TODO tool window.
#[derive(Debug, Default)]
pub struct TodoState {
    pub scope: TodoScope,
    pub group: TodoGroup,
    /// Index into the settings' filters (`None` = all patterns).
    pub filter: Option<usize>,
    /// Selected row of the current view.
    pub selected: Option<usize>,
    /// Files whose rows are collapsed.
    pub collapsed: HashSet<PathBuf>,
    cache: Option<(CacheKey, TodoView)>,
    /// Raw setting lines (patterns, filters, multiline) and what they parse to.
    settings: Option<(RawTodoSettings, TodoSettings)>,
}

impl TodoState {
    /// Rows of the last computed view (for diagnostics and scenes).
    pub fn cached_count(&self) -> Option<usize> {
        self.cache.as_ref().map(|(_, view)| view.rows.len())
    }
}

impl IdeState {
    /// The TODO patterns, filters and options from the settings (parsed
    /// again only when the stored lines change).
    pub fn todo_settings(&mut self) -> TodoSettings {
        let raw = {
            let settings = self.settings();
            let settings = settings.read();
            (
                settings.list(setting_ids::TODO_PATTERNS),
                settings.list(setting_ids::TODO_FILTERS),
                settings.bool(setting_ids::TODO_MULTILINE).unwrap_or(true),
            )
        };
        if let Some((cached, settings)) = &self.analysis.todo.settings
            && *cached == raw
        {
            return settings.clone();
        }
        let settings = TodoSettings {
            patterns: parse_patterns(&raw.0),
            filters: parse_filters(&raw.1),
            multiline: raw.2,
        };
        self.analysis.todo.settings = Some((raw, settings.clone()));
        settings
    }

    /// The rows for the current scope and filter; cached until the index,
    /// the scope, the active file or the settings change.
    pub fn todo_view(&mut self) -> TodoView {
        let settings = self.todo_settings();
        let active = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .map(|document| document.path.clone());
        let snapshot = self.analysis.host.snapshot();
        let key = CacheKey {
            stamp: snapshot.stamps().index(ids::TODO).0 ^ snapshot.generation().rotate_left(40),
            scope: self.analysis.todo.scope,
            file: if self.analysis.todo.scope == TodoScope::CurrentFile {
                active.clone()
            } else {
                None
            },
            filter: self.analysis.todo.filter,
            settings: archipel_ide_index::todo::format_patterns(&settings.patterns)
                .into_iter()
                .chain(archipel_ide_index::todo::format_filters(&settings.filters))
                .collect(),
        };
        if let Some((cached, view)) = &self.analysis.todo.cache
            && *cached == key
        {
            return view.clone();
        }
        let scope = match (self.analysis.todo.scope, &active) {
            (TodoScope::CurrentFile, Some(path)) => snapshot.file_id(path).map_or(
                SearchScope::Files(Default::default()),
                SearchScope::CurrentFile,
            ),
            (TodoScope::CurrentFile, None) => SearchScope::Files(Default::default()),
            (TodoScope::Project, _) => SearchScope::Project,
        };
        let allowed: Option<Vec<u32>> = self
            .analysis
            .todo
            .filter
            .and_then(|index| settings.filters.get(index))
            .map(|filter| filter.patterns.clone());
        let (rows, indexing) = match readers::todo_hits(&snapshot, &scope) {
            Ok(hits) => (
                hits.into_iter()
                    .filter(|hit| allowed.as_ref().is_none_or(|a| a.contains(&hit.pattern)))
                    .map(|hit| TodoRow {
                        path: hit.path,
                        line: hit.item.line as usize,
                        start: hit.item.start as usize,
                        text: hit.item.text,
                        pattern: hit.pattern as usize,
                    })
                    .collect(),
                false,
            ),
            Err(_) => (Vec::new(), true),
        };
        let view = TodoView {
            rows: Arc::new(rows),
            indexing,
            patterns: settings.patterns,
            filters: settings.filters,
        };
        // Only cache complete answers: while indexing the key would not
        // change when the index becomes ready within the same stamp.
        self.analysis.todo.cache = (!indexing).then(|| (key, view.clone()));
        view
    }

    /// Opens the file of row `index` of the current view at the match.
    pub fn todo_open(&mut self, index: usize) -> Result<(), String> {
        let view = self.todo_view();
        let row = view
            .rows
            .get(index)
            .ok_or_else(|| format!("no TODO row {index}"))?
            .clone();
        self.analysis.todo.selected = Some(index);
        self.open_file(&row.path)?;
        let column = self
            .active_document
            .and_then(|i| self.documents.get(i))
            .and_then(|document| {
                let char_index = document.text.byte_to_char(row.start);
                document
                    .text
                    .char_to_line_column(char_index)
                    .ok()
                    .map(|(_, column)| column)
            })
            .unwrap_or(0);
        self.navigate_to(&NavigationItem {
            path: row.path.clone(),
            external: false,
            line: row.line,
            column,
            end_column: column,
            line_text: row.text,
        })
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec("todo.show", "TODO", "Window", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                state.tool_windows.show(ToolWindow::Todo);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "todo.setScope",
                "TODO Scope",
                "View",
                None,
                r#"{"scope":"file|project"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = string_param(&params, "scope")?;
                state.analysis.todo.scope = TodoScope::parse(&id)
                    .ok_or_else(|| CommandError::InvalidParams(format!("unknown scope `{id}`")))?;
                state.analysis.todo.selected = None;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "todo.setGroup",
                "TODO Grouping",
                "View",
                None,
                r#"{"group":"file|flat"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                state.analysis.todo.group = match string_param(&params, "group")?.as_str() {
                    "file" => TodoGroup::File,
                    "flat" => TodoGroup::Flat,
                    other => {
                        return Err(CommandError::InvalidParams(format!(
                            "unknown grouping `{other}`"
                        )));
                    }
                };
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "todo.setFilter",
                "TODO Filter",
                "View",
                None,
                r#"{"filter":"index of a filter of the settings, null for all"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                state.analysis.todo.filter = match params.get("filter") {
                    None | Some(Value::Null) => None,
                    Some(value) => {
                        let index = value
                            .as_u64()
                            .and_then(|n| usize::try_from(n).ok())
                            .ok_or_else(|| {
                                CommandError::InvalidParams("expected a filter index".into())
                            })?;
                        if index >= state.todo_settings().filters.len() {
                            return Err(CommandError::InvalidParams(format!("no filter {index}")));
                        }
                        Some(index)
                    }
                };
                state.analysis.todo.selected = None;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "todo.toggleFile",
                "Collapse or Expand TODO File",
                "View",
                None,
                r#"{"path":"relative path"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                let collapsed = &mut state.analysis.todo.collapsed;
                if !collapsed.remove(&path) {
                    collapsed.insert(path);
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("todo.open", "Open TODO", "Navigate", None, r#"{"index":0}"#),
            |_| true,
            |state: &mut IdeState, params| {
                let index = params
                    .get("index")
                    .and_then(Value::as_u64)
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| CommandError::InvalidParams("expected `index`".into()))?;
                state.todo_open(index).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("todo.list", "List TODO Items", "View", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                state.poll_analysis();
                let view = state.todo_view();
                Ok(json!({
                    "indexing": view.indexing,
                    "scope": state.analysis.todo.scope.id(),
                    "filter": state.analysis.todo.filter,
                    "rows": view.rows.iter().map(|row| json!({
                        "path": row.path,
                        "line": row.line + 1,
                        "text": row.text,
                        "pattern": row.pattern,
                    })).collect::<Vec<_>>(),
                }))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, ToolWindow};
    use archipel_ide_settings::{SettingValue, ids};
    use serde_json::json;
    use std::time::Duration;

    fn state(dir: &std::path::Path) -> IdeState {
        let mut state = IdeState {
            analysis: crate::AnalysisServices::new(archipel_ide_analysis::HostConfig::testing()),
            ..IdeState::default()
        };
        state.open_project(dir).expect("open");
        settle(&mut state);
        state
    }

    fn settle(state: &mut IdeState) {
        for _ in 0..2000 {
            state.poll_analysis();
            if state.analysis.index.is_idle() {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("indexing did not finish");
    }

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("dir");
        std::fs::create_dir_all(dir.path().join("src")).expect("dir");
        std::fs::write(
            dir.path().join("src/Main.java"),
            "class Main {\n  // TODO ship it\n  String s = \"TODO nope\";\n  // FIXME é broken\n}\n",
        )
        .expect("write");
        std::fs::write(dir.path().join("notes.txt"), "a\nTODO call\n").expect("write");
        dir
    }

    fn rows(state: &mut IdeState) -> Vec<(String, u64, String)> {
        let list = crate::built_in_commands()
            .execute("todo.list", state, json!({}))
            .expect("list");
        list["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| {
                (
                    r["path"].as_str().expect("path").to_string(),
                    r["line"].as_u64().expect("line"),
                    r["text"].as_str().expect("text").to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn rows_follow_scope_filter_and_settings() {
        let dir = project();
        let mut state = state(dir.path());
        let commands = crate::built_in_commands();
        let all = rows(&mut state);
        assert_eq!(
            all,
            vec![
                ("notes.txt".into(), 2, "TODO call".into()),
                ("src/Main.java".into(), 2, "TODO ship it".into()),
                ("src/Main.java".into(), 4, "FIXME é broken".into()),
            ],
            "the string literal is not a TODO"
        );

        // Scope: current file only.
        state
            .open_file(std::path::Path::new("src/Main.java"))
            .expect("open");
        commands
            .execute("todo.setScope", &mut state, json!({"scope": "file"}))
            .expect("scope");
        assert_eq!(rows(&mut state).len(), 2);
        commands
            .execute("todo.setScope", &mut state, json!({"scope": "project"}))
            .expect("scope");
        assert!(
            commands
                .execute("todo.setScope", &mut state, json!({"scope": "x"}))
                .is_err()
        );

        // Settings: a filter and new patterns (only the Todo index rebuilds).
        state
            .settings()
            .write()
            .set(
                ids::TODO_FILTERS,
                SettingValue::List(vec!["Fixes: 1".into()]),
            )
            .expect("filters");
        commands
            .execute("todo.setFilter", &mut state, json!({"filter": 0}))
            .expect("filter");
        let fixes = rows(&mut state);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].2, "FIXME é broken");
        assert!(
            commands
                .execute("todo.setFilter", &mut state, json!({"filter": 5}))
                .is_err()
        );
        commands
            .execute("todo.setFilter", &mut state, json!({"filter": null}))
            .expect("all");

        let before = state.analysis.index.stats().indexed;
        state
            .settings()
            .write()
            .set(
                ids::TODO_PATTERNS,
                SettingValue::List(vec![r"i todo \bcall\b.*".into()]),
            )
            .expect("patterns");
        settle(&mut state);
        assert_eq!(
            rows(&mut state),
            vec![("notes.txt".into(), 2, "call".into())]
        );
        assert_eq!(
            state.analysis.index.stats().indexed,
            before + 2,
            "both files were reread for the Todo index only"
        );
    }

    #[test]
    fn opening_a_row_jumps_to_the_match_and_the_window_shows() {
        let dir = project();
        let mut state = state(dir.path());
        let commands = crate::built_in_commands();
        commands
            .execute("todo.show", &mut state, json!({}))
            .expect("show");
        assert!(state.tool_windows.is_visible(ToolWindow::Todo));
        commands
            .execute("todo.open", &mut state, json!({"index": 2}))
            .expect("open");
        let document = &state.documents[state.active_document.expect("active")];
        assert_eq!(document.path, std::path::Path::new("src/Main.java"));
        let caret = document.selections.primary().head;
        assert_eq!(
            document.text.char_to_line_column(caret).expect("pos"),
            (3, 5),
            "line 4, after the comment marker (char columns, é-safe)"
        );
        assert!(
            commands
                .execute("todo.open", &mut state, json!({"index": 99}))
                .is_err()
        );
        commands
            .execute("todo.toggleFile", &mut state, json!({"path": "notes.txt"}))
            .expect("toggle");
        assert!(
            state
                .analysis
                .todo
                .collapsed
                .contains(std::path::Path::new("notes.txt"))
        );
    }
}
