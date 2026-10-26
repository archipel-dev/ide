//! Commit tool window operations on local changes: rollback of files,
//! amend support, and stash/unstash.

use crate::{CommandError, IdeState, spec, string_array_param, usize_param};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_git::StashEntry;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

impl IdeState {
    fn to_repo_paths(&self, paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
        let git = self
            .git
            .as_ref()
            .ok_or("the project is not a Git repository")?;
        let project = self.project.as_ref().ok_or("no project is open")?;
        let git_root = git.root.canonicalize().unwrap_or_else(|_| git.root.clone());
        paths
            .iter()
            .map(|path| {
                project
                    .root()
                    .join(path)
                    .strip_prefix(&git_root)
                    .map(Path::to_path_buf)
                    .map_err(|_| format!("{} is outside the Git repository", path.display()))
            })
            .collect()
    }

    /// Discards local changes of project-relative `paths`.
    pub fn rollback_paths(&mut self, paths: &[PathBuf]) -> Result<String, String> {
        let repo_paths = self.to_repo_paths(paths)?;
        self.run_git_operation(|root| {
            let status = archipel_ide_git::RepositoryStatus::load(root)?;
            let count = status.rollback(&repo_paths)?;
            Ok(format!(
                "Rolled back {count} file{}",
                if count == 1 { "" } else { "s" }
            ))
        })
    }

    /// Message of the last commit, for the Amend toggle.
    pub fn last_commit_message(&self) -> Option<String> {
        let git = self.git.as_ref()?;
        archipel_ide_git::last_commit_message(&git.root)
            .ok()
            .flatten()
    }

    pub fn stashes(&self) -> Vec<StashEntry> {
        self.git
            .as_ref()
            .and_then(|git| archipel_ide_git::stash_list(&git.root).ok())
            .unwrap_or_default()
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let has_git = |state: &IdeState| state.git.is_some();
    registry
        .register(Command::new(
            spec(
                "git.rollback",
                "Rollback…",
                "Git",
                Some("⌥⌘Z"),
                r#"{"paths":["string"]}"#,
            ),
            |state: &IdeState| {
                state
                    .git
                    .as_ref()
                    .is_some_and(|git| !git.changes.is_empty())
            },
            |state: &mut IdeState, params| {
                let paths: Vec<PathBuf> = string_array_param(&params, "paths")?
                    .into_iter()
                    .map(PathBuf::from)
                    .collect();
                let message = state.rollback_paths(&paths).map_err(CommandError::Failed)?;
                Ok(json!({ "message": message }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.stash",
                "Stash Changes…",
                "Git",
                None,
                r#"{"message":"string?","keepIndex":"bool?","includeUntracked":"bool?"}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                let message = params
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let flag = |name: &str| params.get(name).and_then(Value::as_bool).unwrap_or(false);
                let (keep_index, untracked) = (flag("keepIndex"), flag("includeUntracked"));
                let message = state
                    .run_git_operation(|root| {
                        if archipel_ide_git::stash_push(root, &message, keep_index, untracked)? {
                            Ok("Changes stashed".into())
                        } else {
                            Err(archipel_ide_git::GitError::Command(
                                "No local changes to stash".into(),
                            ))
                        }
                    })
                    .map_err(CommandError::Failed)?;
                Ok(json!({ "message": message }))
            },
        ))
        .expect("unique built-in id");
    for (id, label, pop) in [
        ("git.stash.apply", "Apply Stash", false),
        ("git.stash.pop", "Pop Stash", true),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Git", None, r#"{"index":0}"#),
                has_git,
                move |state: &mut IdeState, params| {
                    let index = usize_param(&params, "index")?;
                    let message = state
                        .run_git_operation(|root| {
                            archipel_ide_git::stash_apply(root, index, pop)?;
                            Ok(if pop {
                                "Stash applied and dropped".into()
                            } else {
                                "Stash applied".into()
                            })
                        })
                        .map_err(CommandError::Failed)?;
                    Ok(json!({ "message": message }))
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "git.stash.drop",
                "Drop Stash",
                "Git",
                None,
                r#"{"index":0}"#,
            ),
            has_git,
            |state: &mut IdeState, params| {
                let index = usize_param(&params, "index")?;
                let root = state
                    .git
                    .as_ref()
                    .ok_or_else(|| CommandError::Failed("not a Git repository".into()))?
                    .root
                    .clone();
                archipel_ide_git::stash_drop(&root, index)
                    .map_err(|error| CommandError::Failed(error.to_string()))?;
                state.status = "Stash dropped".into();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;
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
    fn amends_rolls_back_and_stashes_through_commands() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-wt-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "dev@example.invalid"]);
        git(&root, &["config", "user.name", "Dev"]);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "Initail"]);

        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        assert_eq!(state.last_commit_message().as_deref(), Some("Initail"));
        commands
            .execute(
                "git.commit",
                &mut state,
                json!({"message": "Initial", "paths": [], "amend": true}),
            )
            .unwrap();
        assert_eq!(state.last_commit_message().as_deref(), Some("Initial"));

        // Rollback reloads the open editor.
        commands
            .execute("editor.openFile", &mut state, json!({"path": "a.txt"}))
            .unwrap();
        commands
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 0, "text": "x"}),
            )
            .unwrap();
        state.save_all().unwrap();
        state.refresh_git().unwrap();
        commands
            .execute("git.rollback", &mut state, json!({"paths": ["a.txt"]}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "a\n");
        assert!(state.git.as_ref().unwrap().changes.is_empty());
        assert!(!commands.is_enabled("git.rollback", &state));

        fs::write(root.join("a.txt"), "wip\n").unwrap();
        state.refresh_git().unwrap();
        commands
            .execute("git.stash", &mut state, json!({"message": "WIP"}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "a\n");
        assert_eq!(state.stashes().len(), 1);
        assert!(
            commands
                .execute("git.stash", &mut state, json!({}))
                .is_err()
        );
        commands
            .execute("git.stash.pop", &mut state, json!({"index": 0}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "wip\n");
        assert!(state.stashes().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
