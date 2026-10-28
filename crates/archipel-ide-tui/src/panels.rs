//! The tool windows below and beside the editor: Run, Problems, Find, Commit and Terminal.
//!
//! The lists are rebuilt from the core's services when their inputs change (`refresh_panels`),
//! so the event handlers act on what is on screen.

use crate::app::{COMMIT, DialogKind, FIND, PROBLEMS, RUN, TERMINAL, TuiApp};
use crate::keys::key_bytes;
use crate::log::{Line, LogEvent, LogView, Tone};
use archipel_ide_core::{FileProblems, Severity};
use archipel_ide_git::ChangeKind;
use archipel_ide_runner::TaskState;
use archipel_kit_tui::Glyph;
use archipel_kit_tui::components::RowIcon;
use archipel_kit_tui::input::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::runtime::Runtime;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where activating a row goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Open {
    Nothing,
    File {
        path: PathBuf,
        line: Option<usize>,
        column: Option<usize>,
        length: Option<usize>,
    },
}

/// A row of the Commit window.
#[derive(Debug, Clone)]
pub struct CommitRow {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

/// The lists and views of the tool windows.
pub struct Panels {
    pub run: LogView,
    pub run_lines: Vec<Line>,
    pub run_targets: Vec<Open>,
    run_key: Option<(u64, usize, u8)>,
    pub running: bool,
    finished_seen: Option<u64>,
    pub problems: LogView,
    pub problem_lines: Vec<Line>,
    pub problem_targets: Vec<Open>,
    problems_key: Option<u64>,
    pub find: LogView,
    pub find_lines: Vec<Line>,
    pub find_targets: Vec<Open>,
    find_key: Option<(usize, String, bool)>,
    pub commit: LogView,
    pub commit_lines: Vec<Line>,
    pub commit_rows: Vec<Option<CommitRow>>,
    pub included: HashMap<PathBuf, bool>,
    commit_source: Option<Arc<archipel_ide_git::RepositoryStatus>>,
}

impl Default for Panels {
    fn default() -> Self {
        Self {
            run: LogView::following(),
            run_lines: Vec::new(),
            run_targets: Vec::new(),
            run_key: None,
            running: false,
            finished_seen: None,
            problems: LogView::default(),
            problem_lines: Vec::new(),
            problem_targets: Vec::new(),
            problems_key: None,
            find: LogView::default(),
            find_lines: Vec::new(),
            find_targets: Vec::new(),
            find_key: None,
            commit: LogView::default(),
            commit_lines: Vec::new(),
            commit_rows: Vec::new(),
            included: HashMap::new(),
            commit_source: None,
        }
    }
}

fn severity_line(severity: Severity) -> (Tone, Glyph) {
    match severity {
        Severity::Error => (Tone::Error, Glyph::Error),
        Severity::Warning => (Tone::Warning, Glyph::Warning),
        Severity::WeakWarning | Severity::Hint => (Tone::Info, Glyph::WeakWarning),
    }
}

/// The rows of the Problems list: a header per file (name, directory, count) followed by its
/// problems. Problems that belong to no file (the build configuration, the JRE; an empty
/// path) are grouped under the project's name and open nothing.
fn problem_rows(files: &[FileProblems], project: &str) -> (Vec<Line>, Vec<Open>) {
    let mut lines = Vec::new();
    let mut targets = Vec::new();
    for file in files {
        let worst = file
            .problems
            .iter()
            .map(|problem| problem.severity)
            .min()
            .unwrap_or(Severity::Hint);
        let (tone, _) = severity_line(worst);
        let project_level = file.path.as_os_str().is_empty();
        let name = if project_level {
            project.to_owned()
        } else {
            file.path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        let mut header = Line::new(name, tone);
        header.icon = Some(if project_level {
            RowIcon::Glyph(Glyph::Project)
        } else {
            crate::layout::file_icon(&file.path)
        });
        let count = format!(
            "{} problem{}",
            file.problems.len(),
            if file.problems.len() == 1 { "" } else { "s" }
        );
        let directory = file
            .path
            .parent()
            .map(|parent| parent.display().to_string())
            .unwrap_or_default();
        header.detail = if directory.is_empty() {
            count
        } else {
            format!("{directory} · {count}")
        };
        lines.push(header);
        let open = |line: Option<usize>, column: Option<usize>| {
            if project_level {
                Open::Nothing
            } else {
                Open::File {
                    path: file.path.clone(),
                    line,
                    column,
                    length: None,
                }
            }
        };
        targets.push(open(None, None));
        for problem in file.problems.iter().filter(|p| p.severity.listed()) {
            let (tone, glyph) = severity_line(problem.severity);
            let mut line = Line::new(problem.message.lines().next().unwrap_or_default(), tone);
            line.icon = Some(RowIcon::Glyph(glyph));
            line.indent = 2;
            line.detail = format!("{}:{}", problem.line + 1, problem.column + 1);
            lines.push(line);
            targets.push(open(Some(problem.line + 1), Some(problem.column + 1)));
        }
    }
    if lines.is_empty() {
        lines.push(Line::new("No problems found in the project", Tone::Muted));
        targets.push(Open::Nothing);
    }
    (lines, targets)
}

impl TuiApp {
    /// Rebuilds the panel lists whose inputs changed.
    pub(crate) fn refresh_panels(&mut self) {
        self.refresh_run();
        self.refresh_problems_list();
        self.refresh_find();
        self.refresh_commit();
    }

    fn refresh_run(&mut self) {
        let state = self.ide.state();
        let execution = state.execution();
        let runner = execution.runner();
        let running = runner.is_running();
        let kind = match runner.state() {
            TaskState::Idle => 0u8,
            TaskState::Running => 1,
            TaskState::Exited(code) => 2 + u8::from(*code != 0),
            TaskState::Cancelled => 4,
            TaskState::Failed(_) => 5,
        };
        let key = (runner.run_id(), runner.console().lines().len(), kind);
        self.panels.running = running;
        if self.panels.run_key == Some(key) {
            return;
        }
        let root = state.project().map(|project| project.root().to_owned());
        let plan = runner.plan().cloned();
        let mut lines = Vec::new();
        let mut targets = Vec::new();
        if let Some(plan) = &plan {
            let mut line = Line::new(format!("$ {}", plan.command_line()), Tone::Muted);
            line.icon = Some(RowIcon::Glyph(Glyph::Run));
            lines.push(line);
            targets.push(Open::Nothing);
        }
        let header = lines.len();
        let messages = runner.events().messages().to_vec();
        for (index, line) in runner.console().lines().iter().enumerate() {
            let text = line.text.trim_end();
            let tone = if text.starts_with("error") || text.contains("error[") {
                Tone::Error
            } else if text.starts_with("warning") {
                Tone::Warning
            } else {
                Tone::Normal
            };
            lines.push(Line::new(text, tone));
            let target = messages
                .iter()
                .find(|message| message.console_line == index)
                .and_then(|message| {
                    let file = message.file.as_ref()?;
                    let absolute = plan
                        .as_ref()
                        .map_or_else(|| file.clone(), |p| p.resolve(file));
                    let relative = root
                        .as_ref()
                        .and_then(|root| absolute.strip_prefix(root).ok())
                        .map(Path::to_path_buf)?;
                    Some(Open::File {
                        path: relative,
                        line: message.line.map(|l| l as usize),
                        column: message.column.map(|c| c as usize),
                        length: None,
                    })
                })
                .or_else(|| {
                    // A location in plain text (`src/main.rs:3:26`): the same links as the GUI.
                    let cwd = plan.as_ref().map(|p| p.cwd.as_path())?;
                    archipel_ide_core::console_links::find(text)
                        .into_iter()
                        .find_map(|link| match link.target {
                            archipel_ide_core::console_links::LinkTarget::File {
                                path,
                                line,
                                column,
                            } => {
                                let path = state.resolve_console_path(cwd, &path)?;
                                Some(Open::File {
                                    path,
                                    line: Some(line as usize),
                                    column: Some(column as usize),
                                    length: None,
                                })
                            }
                            archipel_ide_core::console_links::LinkTarget::Url(_) => None,
                        })
                })
                .unwrap_or(Open::Nothing);
            targets.push(target);
        }
        let _ = header;
        match runner.state() {
            TaskState::Exited(0) => {
                lines.push(Line::new("", Tone::Normal));
                lines.push(Line::new(
                    "Process finished with exit code 0",
                    Tone::Success,
                ));
                targets.extend([Open::Nothing, Open::Nothing]);
            }
            TaskState::Exited(code) => {
                lines.push(Line::new("", Tone::Normal));
                lines.push(Line::new(
                    format!("Process finished with exit code {code}"),
                    Tone::Error,
                ));
                targets.extend([Open::Nothing, Open::Nothing]);
            }
            TaskState::Cancelled => {
                lines.push(Line::new("Process stopped", Tone::Warning));
                targets.push(Open::Nothing);
            }
            TaskState::Failed(message) => {
                lines.push(Line::new(message.clone(), Tone::Error));
                targets.push(Open::Nothing);
            }
            TaskState::Idle | TaskState::Running => {}
        }
        if lines.is_empty() {
            lines.push(Line::new(
                "Nothing has run yet. Shift+F10 runs the selected configuration.",
                Tone::Muted,
            ));
            targets.push(Open::Nothing);
        }
        drop(runner);
        self.panels.run_lines = lines;
        self.panels.run_targets = targets;
        self.panels.run_key = Some(key);
    }

    fn refresh_problems_list(&mut self) {
        self.ide.state_mut().refresh_problems();
        let state = self.ide.state();
        let index = state.problems();
        let signature = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            for file in index.files() {
                file.path.hash(&mut hasher);
                for problem in &file.problems {
                    (problem.line, problem.column, &problem.message).hash(&mut hasher);
                }
            }
            hasher.finish()
        };
        if self.panels.problems_key == Some(signature) {
            return;
        }
        let project = self
            .ide
            .state()
            .project()
            .and_then(|project| project.root().file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".to_owned());
        let (lines, targets) = problem_rows(index.files(), &project);
        self.panels.problem_lines = lines;
        self.panels.problem_targets = targets;
        self.panels.problems_key = Some(signature);
    }

    fn refresh_find(&mut self) {
        let state = self.ide.state();
        let count = state.search_result_count();
        let query = state.search_query();
        let running = state.search_running();
        let key = (count, query.clone(), running);
        if self.panels.find_key.as_ref() == Some(&key) {
            return;
        }
        let results = state.search_results();
        let mut lines = Vec::new();
        let mut targets = Vec::new();
        if query.is_empty() {
            lines.push(Line::new(
                "Find in Files: Ctrl+Shift+F (Ctrl+K then F)",
                Tone::Muted,
            ));
            targets.push(Open::Nothing);
        } else {
            let files = results
                .iter()
                .map(|found| &found.path)
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            let mut summary = Line::new(
                format!(
                    "{count} result{} in {files} file{} for '{query}'{}",
                    if count == 1 { "" } else { "s" },
                    if files == 1 { "" } else { "s" },
                    if running { " (searching…)" } else { "" }
                ),
                Tone::Info,
            );
            summary.icon = Some(RowIcon::Glyph(Glyph::Search));
            lines.push(summary);
            targets.push(Open::Nothing);
            for found in results {
                let mut line = Line::new(found.preview.clone(), Tone::Normal);
                line.icon = Some(crate::layout::file_icon(&found.path));
                line.detail = format!("{}:{}", found.path.display(), found.line);
                targets.push(Open::File {
                    path: found.path.clone(),
                    line: Some(found.line),
                    column: Some(found.column),
                    length: Some(found.length),
                });
                lines.push(line);
            }
        }
        self.panels.find_lines = lines;
        self.panels.find_targets = targets;
        self.panels.find_key = Some(key);
    }

    fn refresh_commit(&mut self) {
        let Some(status) = self.ide.state().git_status() else {
            if self.panels.commit_source.take().is_some() || self.panels.commit_lines.is_empty() {
                self.panels.commit_lines = vec![Line::new(
                    "The project is not a Git repository",
                    Tone::Muted,
                )];
                self.panels.commit_rows = vec![None];
            }
            return;
        };
        if self
            .panels
            .commit_source
            .as_ref()
            .is_some_and(|known| Arc::ptr_eq(known, &status))
            && self.panels.commit_lines.len() == self.commit_len()
        {
            return;
        }
        let root = self.ide.state().project().map(|p| p.root().to_owned());
        let git_root = status
            .root
            .canonicalize()
            .unwrap_or_else(|_| status.root.clone());
        let to_project = |path: &Path| -> PathBuf {
            let absolute = git_root.join(path);
            root.as_ref()
                .and_then(|root| absolute.strip_prefix(root).ok())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| path.to_owned())
        };
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        for (title, untracked) in [("Changes", false), ("Unversioned Files", true)] {
            let files: Vec<_> = status
                .changes
                .iter()
                .filter(|change| (change.kind == ChangeKind::Untracked) == untracked)
                .collect();
            if files.is_empty() {
                continue;
            }
            let mut header = Line::new(format!("{title} ({})", files.len()), Tone::Muted);
            header.icon = Some(RowIcon::Glyph(Glyph::Folder));
            lines.push(header);
            rows.push(None);
            for change in files {
                let path = to_project(&change.path);
                let included = self
                    .panels
                    .included
                    .get(&path)
                    .copied()
                    .unwrap_or(change.kind != ChangeKind::Untracked);
                let tone = match change.kind {
                    ChangeKind::Untracked | ChangeKind::Added => Tone::Success,
                    ChangeKind::Deleted => Tone::Error,
                    ChangeKind::Conflicted => Tone::Error,
                    _ => Tone::Warning,
                };
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut line = Line::new(
                    format!("{} {name}", if included { "[x]" } else { "[ ]" }),
                    tone,
                );
                line.indent = 1;
                line.detail = path
                    .parent()
                    .map(|parent| parent.display().to_string())
                    .unwrap_or_default();
                lines.push(line);
                rows.push(Some(CommitRow {
                    path,
                    kind: change.kind,
                }));
            }
        }
        if lines.is_empty() {
            lines.push(Line::new("No changes", Tone::Muted));
            rows.push(None);
        }
        self.panels.commit_lines = lines;
        self.panels.commit_rows = rows;
        self.panels.commit_source = Some(status);
    }

    fn commit_len(&self) -> usize {
        self.panels.commit_lines.len()
    }

    /// A balloon when a run finishes.
    pub(crate) fn run_finished_notice(&mut self) {
        let state = self.ide.state();
        let execution = state.execution();
        let runner = execution.runner();
        let id = runner.run_id();
        let finished = matches!(
            runner.state(),
            TaskState::Exited(_) | TaskState::Cancelled | TaskState::Failed(_)
        );
        if !finished || self.panels.finished_seen == Some(id) {
            return;
        }
        let (status, title, body) = match runner.state() {
            TaskState::Exited(0) => (
                archipel_kit::ui::Status::Success,
                "Finished",
                runner
                    .plan()
                    .map(|plan| format!("{} finished with exit code 0", plan.name))
                    .unwrap_or_default(),
            ),
            TaskState::Exited(code) => (
                archipel_kit::ui::Status::Warning,
                "Failed",
                runner
                    .plan()
                    .map(|plan| format!("{} finished with exit code {code}", plan.name))
                    .unwrap_or_default(),
            ),
            TaskState::Cancelled => (
                archipel_kit::ui::Status::Info,
                "Stopped",
                "The process was stopped".to_owned(),
            ),
            TaskState::Failed(message) => (
                archipel_kit::ui::Status::Error,
                "Could not start",
                message.clone(),
            ),
            _ => return,
        };
        drop(runner);
        drop(execution);
        self.panels.finished_seen = Some(id);
        if id != 0 {
            self.notify(status, title, &body);
        }
    }

    // ---------------------------------------------------------------- drawing

    pub(crate) fn draw_run(&mut self, rt: &mut Runtime, body: Rect, buf: &mut Buffer) {
        let on = rt.look.palette.darker_black;
        self.panels
            .run
            .show(rt, RUN, body, &self.panels.run_lines, on, buf);
    }

    pub(crate) fn draw_problems(&mut self, rt: &mut Runtime, body: Rect, buf: &mut Buffer) {
        let on = rt.look.palette.darker_black;
        self.panels
            .problems
            .show(rt, PROBLEMS, body, &self.panels.problem_lines, on, buf);
    }

    pub(crate) fn draw_find(&mut self, rt: &mut Runtime, body: Rect, buf: &mut Buffer) {
        let on = rt.look.palette.darker_black;
        self.panels
            .find
            .show(rt, FIND, body, &self.panels.find_lines, on, buf);
    }

    pub(crate) fn draw_commit(&mut self, rt: &mut Runtime, body: Rect, buf: &mut Buffer) {
        let p = rt.look.palette;
        let list = Rect::new(body.x, body.y, body.width, body.height.saturating_sub(1));
        self.panels.commit.show(
            rt,
            COMMIT,
            list,
            &self.panels.commit_lines,
            p.darker_black,
            buf,
        );
        if body.height > 1 {
            archipel_kit_tui::testing::put_text(
                buf,
                body,
                body.x + 1,
                body.bottom() - 1,
                "Space toggle · c commit · r refresh",
                p.grey_fg,
                p.darker_black,
            );
        }
    }

    pub(crate) fn draw_terminal(&mut self, rt: &mut Runtime, body: Rect, buf: &mut Buffer) {
        let p = rt.look.palette;
        rt.widget(TERMINAL, body);
        if !self.ide.state().terminal().has_session() {
            self.ensure_terminal();
        }
        let wanted = (body.height.max(1), body.width.max(1));
        let current = self
            .ide
            .state()
            .terminal()
            .session()
            .as_ref()
            .map(|session| session.screen().size());
        if current.is_some_and(|size| size != wanted) {
            let _ = self.ide.dispatch(
                "terminal.resize",
                json!({ "rows": wanted.0, "cols": wanted.1 }),
            );
        }
        let focused = rt.focused(TERMINAL);
        let state = self.ide.state();
        let terminal = state.terminal();
        let session = terminal.session();
        match session.as_ref() {
            Some(session) => {
                crate::terminal::draw(rt, body, p.darker_black, session.screen(), focused, buf)
            }
            None => archipel_kit_tui::testing::put_text(
                buf,
                body,
                body.x + 1,
                body.y,
                "The terminal could not be started",
                p.red,
                p.darker_black,
            ),
        }
    }

    // ---------------------------------------------------------------- events

    pub(crate) fn route_panels(&mut self, rt: &mut Runtime, event: &Event) {
        if let LogEvent::Activated(index) = self.panels.run.handle(rt, RUN, event)
            && let Some(target) = self.panels.run_targets.get(index).cloned()
        {
            self.open_target(rt, target);
        }
        if let LogEvent::Activated(index) = self.panels.problems.handle(rt, PROBLEMS, event)
            && let Some(target) = self.panels.problem_targets.get(index).cloned()
        {
            self.open_target(rt, target);
        }
        if let LogEvent::Activated(index) = self.panels.find.handle(rt, FIND, event)
            && let Some(target) = self.panels.find_targets.get(index).cloned()
        {
            self.open_target(rt, target);
        }
        self.route_commit(rt, event);
    }

    fn open_target(&mut self, rt: &mut Runtime, target: Open) {
        if let Open::File {
            path,
            line,
            column,
            length,
        } = target
        {
            let mut params = json!({ "path": path });
            if let Some(line) = line {
                params["line"] = json!(line);
            }
            if let Some(column) = column {
                params["column"] = json!(column);
            }
            if let Some(length) = length {
                params["length"] = json!(length);
            }
            self.command(rt, "editor.openFile", params);
            self.focus_editor();
        }
    }

    fn route_commit(&mut self, rt: &mut Runtime, event: &Event) {
        if let LogEvent::Activated(index) = self.panels.commit.handle(rt, COMMIT, event)
            && let Some(Some(row)) = self.panels.commit_rows.get(index).cloned()
        {
            self.open_target(
                rt,
                Open::File {
                    path: row.path,
                    line: None,
                    column: None,
                    length: None,
                },
            );
        }
        let Event::Key(key) = event else { return };
        if key.kind == KeyEventKind::Release
            || !rt.focused(COMMIT)
            || key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }
        match key.code {
            KeyCode::Char(' ') => {
                if let Some(Some(row)) = self.panels.commit_rows.get(self.panels.commit.selected) {
                    let default = row.kind != ChangeKind::Untracked;
                    let now = self
                        .panels
                        .included
                        .get(&row.path)
                        .copied()
                        .unwrap_or(default);
                    self.panels.included.insert(row.path.clone(), !now);
                    self.panels.commit_source = None;
                    self.refresh_commit();
                }
            }
            KeyCode::Char('r') => self.command(rt, "git.refresh", serde_json::Value::Null),
            KeyCode::Char('c') => self.commit_dialog(rt),
            _ => {}
        }
    }

    /// The paths of the checked changes.
    pub(crate) fn commit_paths(&self) -> Vec<PathBuf> {
        self.panels
            .commit_rows
            .iter()
            .flatten()
            .filter(|row| {
                self.panels
                    .included
                    .get(&row.path)
                    .copied()
                    .unwrap_or(row.kind != ChangeKind::Untracked)
            })
            .map(|row| row.path.clone())
            .collect()
    }

    /// Opens the Commit dialog for the checked files.
    pub(crate) fn commit_dialog(&mut self, rt: &mut Runtime) {
        let paths = self.commit_paths();
        if paths.is_empty() {
            self.ide
                .state_mut()
                .set_status("Nothing is checked for the commit".to_owned());
            return;
        }
        let count = paths.len();
        self.input_dialog(
            rt,
            "Commit",
            "Message",
            "",
            DialogKind::Commit { paths },
            |content| content.placeholder(&format!("Commit {count} file(s)")),
        );
    }

    /// Sends a key to the shell. Returns false when the key belongs to the workbench or the
    /// IDE (switching windows, Find Action).
    pub(crate) fn terminal_key(&mut self, rt: &mut Runtime, key: &KeyEvent) -> bool {
        const PASS: [&str; 12] = [
            "HideActiveWindow",
            "HideAllWindows",
            "JumpToLastWindow",
            "MaximizeToolWindow",
            "GotoAction",
            "SearchEverywhere",
            "ShowSettings",
            "ActivateTerminalToolWindow",
            "ActivateProjectToolWindow",
            "ActivateRunToolWindow",
            "ActivateFindToolWindow",
            "ActivateProblemsViewToolWindow",
        ];
        if rt.awaiting_second_stroke() || rt.chord_completed() {
            return false;
        }
        if rt
            .actions()
            .iter()
            .any(|id| PASS.contains(&id.as_str()) || id.starts_with("ActivateToolWindow"))
        {
            return false;
        }
        match key_bytes(key) {
            Some(bytes) => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let _ = self
                    .ide
                    .dispatch("terminal.sendText", json!({ "text": text }));
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_core::Problem;

    fn problem(severity: Severity, message: &str) -> Problem {
        Problem {
            severity,
            message: message.into(),
            source: None,
            code: None,
            line: 0,
            column: 0,
            end_line: 0,
            end_column: 1,
            unnecessary: false,
            deprecated: false,
        }
    }

    #[test]
    fn problems_without_a_file_are_grouped_under_the_project_name() {
        let files = [
            FileProblems {
                path: PathBuf::new(),
                problems: vec![
                    problem(Severity::Warning, "Build path specifies JavaSE-1.8"),
                    problem(Severity::Warning, "The compiler compliance is 1.8"),
                ],
            },
            FileProblems {
                path: PathBuf::from("src/App.java"),
                problems: vec![problem(Severity::Error, "greet() is undefined")],
            },
        ];
        let (lines, targets) = problem_rows(&files, "demo");
        assert_eq!(lines[0].text, "demo");
        assert_eq!(
            lines[0].detail, "2 problems",
            "no empty directory before the count"
        );
        assert!(matches!(
            lines[0].icon,
            Some(RowIcon::Glyph(Glyph::Project))
        ));
        assert_eq!(lines[3].text, "App.java");
        assert_eq!(lines[3].detail, "src · 1 problem");
        // A group without a file opens nothing; a file's rows open it.
        assert!(targets[..3].iter().all(|open| *open == Open::Nothing));
        assert!(matches!(&targets[4], Open::File { line: Some(1), .. }));
    }
}
