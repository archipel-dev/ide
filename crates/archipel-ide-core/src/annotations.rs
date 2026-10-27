//! "Annotate with Git Blame" gutter data and "Show History" for a file.

use crate::services::REBLAME_INTERVAL;
use crate::{CommandError, IdeState, ToolWindow, spec};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_git::BlameLine;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Annotation {
    /// Document revision the lines were computed for; `None` forces a rerun.
    pub(crate) revision: Option<u64>,
    pub(crate) lines: Vec<BlameLine>,
    pub(crate) computed_at: Instant,
}

impl Annotation {
    /// Forces a re-blame at the next opportunity (after commits, checkouts, pulls).
    pub(crate) fn invalidate(&mut self, interval: Duration) {
        self.revision = None;
        self.computed_at = Instant::now() - interval;
    }
}

impl IdeState {
    fn repo_path_of(&self, project_path: &std::path::Path) -> Result<PathBuf, String> {
        let git = self
            .git_status()
            .ok_or("the project is not a Git repository")?;
        let project = self.project.as_ref().ok_or("no project is open")?;
        let git_root = git.root.canonicalize().unwrap_or_else(|_| git.root.clone());
        project
            .root()
            .join(project_path)
            .strip_prefix(&git_root)
            .map(std::path::Path::to_path_buf)
            .map_err(|_| format!("{} is outside the Git repository", project_path.display()))
    }

    pub(crate) fn blame_document(&self, index: usize) -> Result<Vec<BlameLine>, String> {
        let document = self.documents.get(index).ok_or("document does not exist")?;
        let git = self
            .git_status()
            .ok_or("the project is not a Git repository")?;
        let path = self.repo_path_of(&document.path)?;
        let text = document.text.text();
        archipel_ide_git::blame(&git.root, &path, Some(&text)).map_err(|error| error.to_string())
    }

    /// Turns blame annotations for the active document on or off.
    pub fn toggle_annotations(&mut self) -> Result<bool, String> {
        let index = self.active_document.ok_or("no active document")?;
        let path = self.documents[index].path.clone();
        let annotations = self.annotations().ok_or("no project is open")?;
        if annotations.write().remove(&path).is_some() {
            return Ok(false);
        }
        let lines = self.blame_document(index)?;
        annotations.write().insert(
            path,
            Annotation {
                revision: Some(self.documents[index].text.revision()),
                lines,
                computed_at: Instant::now(),
            },
        );
        Ok(true)
    }

    /// Blame lines for document `index` if annotations are on, refreshed
    /// (throttled) after edits and repository changes.
    pub fn document_annotations(&self, index: usize) -> Option<Vec<BlameLine>> {
        let document = self.documents.get(index)?;
        let path = document.path.clone();
        let revision = document.text.revision();
        let annotations = self.annotations()?;
        let stale = {
            let files = annotations.read();
            let annotation = files.get(&path)?;
            annotation.revision != Some(revision)
                && annotation.computed_at.elapsed() >= REBLAME_INTERVAL
        };
        if stale {
            let lines = self.blame_document(index);
            if let Some(annotation) = annotations.write().get_mut(&path) {
                annotation.computed_at = Instant::now();
                if let Ok(lines) = lines {
                    annotation.lines = lines;
                    annotation.revision = Some(revision);
                }
            }
        }
        annotations
            .read()
            .get(&path)
            .map(|annotation| annotation.lines.clone())
    }

    /// Shows the Git log limited to the active document's history.
    pub fn show_file_history(&mut self) -> Result<(), String> {
        let index = self.active_document.ok_or("no active document")?;
        let path = self.repo_path_of(&self.documents[index].path.clone())?;
        self.git_log_update(|log| {
            if !log.history_tabs.contains(&path) {
                log.history_tabs.push(path.clone());
            }
            log.path = Some(path);
            log.filter.clear();
        })
        .ok_or("no project is open")?;
        self.tool_windows.show(ToolWindow::Git);
        self.refresh_git_log()
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let tracked = |state: &IdeState| state.active_document.is_some() && state.has_git();
    registry
        .register(Command::new(
            spec("git.annotate", "Annotate with Git Blame", "Git", None, "{}"),
            tracked,
            |state: &mut IdeState, _| {
                let on = state.toggle_annotations().map_err(CommandError::Failed)?;
                Ok(json!({ "annotated": on }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("git.fileHistory", "Show History", "Git", None, "{}"),
            tracked,
            |state: &mut IdeState, _| {
                state.show_file_history().map_err(CommandError::Failed)?;
                Ok(json!({ "commits": state.git_log_snapshot().commits.len() }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.log.clearPath",
                "Show All Files in Log",
                "Git",
                None,
                "{}",
            ),
            |state: &IdeState| state.git_log_snapshot().path.is_some(),
            |state: &mut IdeState, _| {
                state.git_log_update(|log| log.path = None);
                state.refresh_git_log().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, ToolWindow, built_in_commands};
    use serde_json::{Value, json};
    use std::fs;
    use std::path::Path;

    fn git(root: &Path, args: &[&str]) {
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }

    #[test]
    fn annotates_documents_and_shows_file_history() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-blame-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "a@example.invalid"]);
        git(&root, &["config", "user.name", "Alice"]);
        fs::write(root.join("a.txt"), "one\n").unwrap();
        fs::write(root.join("b.txt"), "b\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "First"]);
        fs::write(root.join("b.txt"), "b2\n").unwrap();
        git(&root, &["commit", "-qam", "Only b"]);

        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "a.txt"}))
            .unwrap();
        assert!(state.document_annotations(0).is_none());
        let result = commands
            .execute("git.annotate", &mut state, Value::Null)
            .unwrap();
        assert_eq!(result["annotated"], true);
        let lines = state.document_annotations(0).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].author, "Alice");
        assert_eq!(lines[0].summary, "First");

        let result = commands
            .execute("git.fileHistory", &mut state, Value::Null)
            .unwrap();
        assert_eq!(result["commits"], 1);
        assert!(state.tool_windows.is_visible(ToolWindow::Git));
        assert_eq!(state.git_log_snapshot().commits[0].subject, "First");
        // File history is a sibling tab of `Log`: it can be left and reopened.
        let tab = state.git_log_snapshot().history_tabs[0].clone();
        assert_eq!(state.git_log_snapshot().active_history_tab(), Some(&tab));
        commands
            .execute("git.log.selectTab", &mut state, json!({}))
            .unwrap();
        assert!(state.git_log_snapshot().active_history_tab().is_none());
        assert_eq!(state.git_log_snapshot().commits.len(), 2);
        commands
            .execute("git.log.selectTab", &mut state, json!({"path": tab}))
            .unwrap();
        assert_eq!(state.git_log_snapshot().commits.len(), 1);
        assert!(
            commands
                .execute("git.log.selectTab", &mut state, json!({"path": "nope"}))
                .is_err()
        );
        commands
            .execute("git.fileHistory", &mut state, Value::Null)
            .unwrap();
        assert_eq!(
            state.git_log_snapshot().history_tabs.len(),
            1,
            "no duplicate tabs"
        );
        commands
            .execute("git.log.closeTab", &mut state, json!({"path": tab}))
            .unwrap();
        assert!(state.git_log_snapshot().history_tabs.is_empty());
        assert!(state.git_log_snapshot().path.is_none());
        assert_eq!(state.git_log_snapshot().commits.len(), 2);

        let result = commands
            .execute("git.annotate", &mut state, Value::Null)
            .unwrap();
        assert_eq!(result["annotated"], false);
        assert!(state.document_annotations(0).is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
