//! "Annotate with Git Blame" gutter data and "Show History" for a file.

use crate::{CommandError, IdeState, ToolWindow, spec};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_git::BlameLine;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Minimum time between blame runs while a document is being edited.
const REBLAME_INTERVAL: Duration = Duration::from_millis(800);

#[derive(Debug, Clone)]
pub(crate) struct Annotation {
    /// Document revision the lines were computed for; `None` forces a rerun.
    revision: Option<u64>,
    lines: Vec<BlameLine>,
    computed_at: Instant,
}

impl IdeState {
    fn repo_path_of(&self, project_path: &std::path::Path) -> Result<PathBuf, String> {
        let git = self
            .git
            .as_ref()
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
            .git
            .as_ref()
            .ok_or("the project is not a Git repository")?;
        let path = self.repo_path_of(&document.path)?;
        let text = document.text.text();
        archipel_ide_git::blame(&git.root, &path, Some(&text)).map_err(|error| error.to_string())
    }

    /// Turns blame annotations for the active document on or off.
    pub fn toggle_annotations(&mut self) -> Result<bool, String> {
        let index = self.active_document.ok_or("no active document")?;
        let path = self.documents[index].path.clone();
        if self.annotations.remove(&path).is_some() {
            return Ok(false);
        }
        let lines = self.blame_document(index)?;
        self.annotations.insert(
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
    pub fn document_annotations(&mut self, index: usize) -> Option<&[BlameLine]> {
        let document = self.documents.get(index)?;
        let path = document.path.clone();
        let revision = document.text.revision();
        let stale = {
            let annotation = self.annotations.get(&path)?;
            annotation.revision != Some(revision)
                && annotation.computed_at.elapsed() >= REBLAME_INTERVAL
        };
        if stale {
            let lines = self.blame_document(index);
            if let Some(annotation) = self.annotations.get_mut(&path) {
                annotation.computed_at = Instant::now();
                if let Ok(lines) = lines {
                    annotation.lines = lines;
                    annotation.revision = Some(revision);
                }
            }
        }
        self.annotations
            .get(&path)
            .map(|annotation| annotation.lines.as_slice())
    }

    /// Re-blames annotated documents (after commits, checkouts, pulls).
    pub(crate) fn invalidate_annotations(&mut self) {
        for annotation in self.annotations.values_mut() {
            annotation.revision = None;
            annotation.computed_at = Instant::now() - REBLAME_INTERVAL;
        }
    }

    /// Shows the Git log limited to the active document's history.
    pub fn show_file_history(&mut self) -> Result<(), String> {
        let index = self.active_document.ok_or("no active document")?;
        let path = self.repo_path_of(&self.documents[index].path.clone())?;
        if !self.git_log.history_tabs.contains(&path) {
            self.git_log.history_tabs.push(path.clone());
        }
        self.git_log.path = Some(path);
        self.git_log.filter.clear();
        self.tool_windows.show(ToolWindow::Git);
        self.refresh_git_log()
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let tracked = |state: &IdeState| state.active_document.is_some() && state.git.is_some();
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
                Ok(json!({ "commits": state.git_log.commits.len() }))
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
            |state: &IdeState| state.git_log.path.is_some(),
            |state: &mut IdeState, _| {
                state.git_log.path = None;
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
        assert_eq!(state.git_log.commits[0].subject, "First");
        // File history is a sibling tab of `Log`: it can be left and reopened.
        let tab = state.git_log.history_tabs[0].clone();
        assert_eq!(state.git_log.active_history_tab(), Some(&tab));
        commands
            .execute("git.log.selectTab", &mut state, json!({}))
            .unwrap();
        assert!(state.git_log.active_history_tab().is_none());
        assert_eq!(state.git_log.commits.len(), 2);
        commands
            .execute("git.log.selectTab", &mut state, json!({"path": tab}))
            .unwrap();
        assert_eq!(state.git_log.commits.len(), 1);
        assert!(
            commands
                .execute("git.log.selectTab", &mut state, json!({"path": "nope"}))
                .is_err()
        );
        commands
            .execute("git.fileHistory", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.git_log.history_tabs.len(), 1, "no duplicate tabs");
        commands
            .execute("git.log.closeTab", &mut state, json!({"path": tab}))
            .unwrap();
        assert!(state.git_log.history_tabs.is_empty());
        assert!(state.git_log.path.is_none());
        assert_eq!(state.git_log.commits.len(), 2);

        let result = commands
            .execute("git.annotate", &mut state, Value::Null)
            .unwrap();
        assert_eq!(result["annotated"], false);
        assert!(state.document_annotations(0).is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
