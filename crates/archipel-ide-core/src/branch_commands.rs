//! Typed commands for branch operations (branches popup, Git menu). Every
//! operation saves open documents first and reloads them afterwards, since
//! checkout, merge and rebase rewrite files on disk.

use crate::{CommandError, IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_git::{GitError, PendingOperation};
use serde_json::{Value, json};
use std::path::PathBuf;

impl IdeState {
    fn git_root(&self) -> Result<PathBuf, String> {
        Ok(self
            .git_status()
            .ok_or("the project is not a Git repository")?
            .root
            .clone())
    }

    /// Runs a working-tree-changing Git operation and brings the IDE state up
    /// to date whether it succeeded or stopped half-way (e.g. on conflicts).
    pub(crate) fn run_git_operation(
        &mut self,
        operation: impl FnOnce(&std::path::Path) -> Result<String, GitError>,
    ) -> Result<String, String> {
        let root = self.git_root()?;
        self.save_all_at(crate::SavePoint::Vcs).into_result()?;
        let result = operation(&root).map_err(|error| error.to_string());
        self.after_git_operation();
        let message = result?;
        self.status = message.clone();
        Ok(message)
    }

    fn after_git_operation(&mut self) {
        let _ = self.refresh_git();
        self.invalidate_git_log();
        self.reload_documents_from_disk();
        if let Some(project) = &self.project {
            self.set_files(project.entries(20_000));
        }
    }

    /// Replaces the text of open documents whose file changed on disk. Called
    /// after Git rewrote the working tree; open documents were saved first, so
    /// no unsaved edit is lost. Files that no longer exist keep their buffer.
    pub fn reload_documents_from_disk(&mut self) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let mut reloaded = Vec::new();
        for document in &mut self.documents {
            let Ok(disk) = project.read_text(&document.path) else {
                continue;
            };
            if disk == document.text.text() {
                continue;
            }
            if document.text.replace_from_snapshot(&disk).is_ok() {
                document.text.mark_saved();
                document.selections.clamp(document.text.len_chars());
                document.refresh_analysis(&disk);
                let version = document.text.revision().min(i32::MAX as u64) as i32;
                reloaded.push((document.path.clone(), disk, version));
            }
        }
        for (path, text, version) in reloaded {
            self.sync_document_lsp(&path, &text, version);
        }
        self.refresh_diff();
    }
}

fn optional_string(params: &Value, name: &str) -> Option<String> {
    params
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(Into::into)
}

fn bool_param(params: &Value, name: &str) -> bool {
    params.get(name).and_then(Value::as_bool).unwrap_or(false)
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let has_git = |state: &IdeState| state.has_git();
    let ok = |message: String| Ok(json!({ "message": message }));
    registry
        .register(Command::new(
            spec(
                "git.checkout",
                "Checkout",
                "Git",
                None,
                r#"{"branch":"string"}"#,
            ),
            has_git,
            move |state: &mut IdeState, params| {
                let branch = string_param(&params, "branch")?;
                let message = state
                    .run_git_operation(|root| archipel_ide_git::checkout(root, &branch))
                    .map_err(CommandError::Failed)?;
                ok(message)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.newBranch",
                "New Branch…",
                "Git",
                None,
                r#"{"name":"string","from":"string?","checkout":"bool?"}"#,
            ),
            has_git,
            move |state: &mut IdeState, params| {
                let name = string_param(&params, "name")?;
                let from = optional_string(&params, "from");
                let checkout = params
                    .get("checkout")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let message = state
                    .run_git_operation(|root| {
                        archipel_ide_git::create_branch(root, &name, from.as_deref(), checkout)
                    })
                    .map_err(CommandError::Failed)?;
                ok(message)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.renameBranch",
                "Rename Branch…",
                "Git",
                None,
                r#"{"branch":"string","name":"string"}"#,
            ),
            has_git,
            move |state: &mut IdeState, params| {
                let branch = string_param(&params, "branch")?;
                let name = string_param(&params, "name")?;
                let message = state
                    .run_git_operation(|root| archipel_ide_git::rename_branch(root, &branch, &name))
                    .map_err(CommandError::Failed)?;
                ok(message)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.deleteBranch",
                "Delete Branch",
                "Git",
                None,
                r#"{"branch":"string","force":"bool?"}"#,
            ),
            has_git,
            move |state: &mut IdeState, params| {
                let branch = string_param(&params, "branch")?;
                let force = bool_param(&params, "force");
                let message = state
                    .run_git_operation(|root| archipel_ide_git::delete_branch(root, &branch, force))
                    .map_err(CommandError::Failed)?;
                ok(message)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.merge",
                "Merge into Current",
                "Git",
                None,
                r#"{"branch":"string"}"#,
            ),
            has_git,
            move |state: &mut IdeState, params| {
                let branch = string_param(&params, "branch")?;
                let message = state
                    .run_git_operation(|root| archipel_ide_git::merge(root, &branch))
                    .map_err(CommandError::Failed)?;
                ok(message)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.rebase",
                "Rebase Current onto",
                "Git",
                None,
                r#"{"onto":"string"}"#,
            ),
            has_git,
            move |state: &mut IdeState, params| {
                let onto = string_param(&params, "onto")?;
                let message = state
                    .run_git_operation(|root| archipel_ide_git::rebase(root, &onto))
                    .map_err(CommandError::Failed)?;
                ok(message)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.cherryPick",
                "Cherry-Pick",
                "Git",
                None,
                r#"{"commit":"string"}"#,
            ),
            has_git,
            move |state: &mut IdeState, params| {
                let commit = string_param(&params, "commit")?;
                let message = state
                    .run_git_operation(|root| archipel_ide_git::cherry_pick(root, &commit))
                    .map_err(CommandError::Failed)?;
                ok(message)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.abortOperation",
                "Abort Merge / Rebase",
                "Git",
                None,
                "{}",
            ),
            |state: &IdeState| state.git_pending().is_some(),
            move |state: &mut IdeState, _| {
                let message = state
                    .run_git_operation(archipel_ide_git::abort_operation)
                    .map_err(CommandError::Failed)?;
                ok(message)
            },
        ))
        .expect("unique built-in id");
}

/// Label for the abort action of an interrupted operation.
pub fn abort_label(operation: PendingOperation) -> String {
    format!("Abort {}", operation.label())
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use archipel_ide_git::PendingOperation;
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
    fn branch_commands_switch_branches_and_reload_open_files() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-branch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "dev@example.invalid"]);
        git(&root, &["config", "user.name", "Dev"]);
        fs::write(root.join("a.txt"), "main\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "main"]);

        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "a.txt"}))
            .unwrap();
        commands
            .execute("git.newBranch", &mut state, json!({"name": "topic"}))
            .unwrap();
        assert_eq!(state.git_status().unwrap().branch, "topic");
        // Unsaved edits are saved before the operation and committed on topic.
        commands
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 0, "text": "topic "}),
            )
            .unwrap();
        state.save_all().unwrap();
        git(&root, &["commit", "-qam", "topic"]);
        commands
            .execute("git.checkout", &mut state, json!({"branch": "main"}))
            .unwrap();
        assert_eq!(state.git_status().unwrap().branch, "main");
        // The open editor shows main's content after the checkout.
        assert_eq!(state.documents[0].text.text(), "main\n");
        assert!(!state.documents[0].text.is_dirty());

        // Conflicting merge leaves a pending operation that can be aborted.
        fs::write(root.join("a.txt"), "other\n").unwrap();
        git(&root, &["commit", "-qam", "other"]);
        state.reload_documents_from_disk();
        assert!(
            commands
                .execute("git.merge", &mut state, json!({"branch": "topic"}))
                .is_err()
        );
        assert_eq!(state.git_pending(), Some(PendingOperation::Merge));
        assert!(state.documents[0].text.text().contains("<<<<<<<"));
        commands
            .execute("git.abortOperation", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.git_pending(), None);
        assert_eq!(state.documents[0].text.text(), "other\n");
        assert!(!commands.is_enabled("git.abortOperation", &state));

        commands
            .execute(
                "git.renameBranch",
                &mut state,
                json!({"branch": "topic", "name": "feature"}),
            )
            .unwrap();
        assert!(
            commands
                .execute("git.deleteBranch", &mut state, json!({"branch": "feature"}))
                .is_err()
        );
        commands
            .execute(
                "git.deleteBranch",
                &mut state,
                json!({"branch": "feature", "force": true}),
            )
            .unwrap();
        assert!(
            commands
                .execute("git.newBranch", &mut state, json!({"name": "bad name"}))
                .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
