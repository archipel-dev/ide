//! Fetch, Update Project (pull) and Push as background operations with
//! progress in the status bar.

use crate::services::RemoteOutcome;
use crate::{CommandError, IdeState, spec};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_git::{PushPlan, RemoteOperation};
use serde_json::{Value, json};

impl IdeState {
    pub fn remote_busy(&self) -> bool {
        self.remote_ops().is_some_and(|ops| ops.is_busy())
    }

    /// Latest progress line of the running fetch, pull or push.
    pub fn remote_progress(&self) -> Option<String> {
        self.remote_ops()?.progress()
    }

    /// Message of the last failed fetch, pull or push.
    pub fn remote_error(&self) -> Option<String> {
        self.remote_ops()?.error()
    }

    fn start_remote(&mut self, operation: RemoteOperation) -> Result<(), String> {
        let ops = self.remote_ops().ok_or("no project is open")?;
        if ops.is_busy() {
            return Err("Another fetch, pull or push is still running".into());
        }
        let root = self
            .git_status()
            .ok_or("the project is not a Git repository")?
            .root
            .clone();
        if matches!(operation, RemoteOperation::Pull { .. }) {
            // The pull rewrites files; editors are reloaded when it ends.
            self.save_all_at(crate::SavePoint::Vcs).into_result()?;
        }
        let label = operation.label();
        ops.start(&root, operation, self.repaint.notifier())?;
        self.status = format!("{label}…");
        Ok(())
    }

    /// What Push would send; used by the push dialog.
    pub fn push_plan(&self) -> Result<PushPlan, String> {
        let git = self
            .git_status()
            .ok_or("the project is not a Git repository")?;
        archipel_ide_git::push_plan(&git.root).map_err(|error| error.to_string())
    }

    /// Applies progress and completion of the running remote operation. The end of the
    /// operation arrives as a [`crate::services::REMOTE_TOPIC`] event.
    pub fn poll_remote(&mut self) {
        let Some(ops) = self.remote_ops() else {
            return;
        };
        if let Some(progress) = ops.poll() {
            self.status = progress;
        }
        self.dispatch_reactions();
    }

    /// Brings the IDE up to date after a fetch, pull or push ended.
    pub(crate) fn remote_operation_finished(&mut self, outcome: RemoteOutcome) {
        let _ = self.refresh_git();
        self.invalidate_git_log();
        if outcome.pull {
            self.reload_documents_from_disk();
            if let Some(project) = &self.project {
                self.set_files(project.entries(20_000));
            }
        }
        self.status = match outcome.result {
            Ok(summary) => summary,
            Err(error) => format!(
                "{} failed: {}",
                match outcome.label {
                    "Fetching" => "Fetch",
                    "Pushing" => "Push",
                    _ => "Update",
                },
                error.lines().next().unwrap_or_default()
            ),
        };
        if let Some(ops) = self.remote_ops() {
            ops.set_error(
                self.status
                    .contains(" failed: ")
                    .then(|| self.status.clone()),
            );
        }
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let idle = |state: &IdeState| state.has_git() && !state.remote_busy();
    registry
        .register(Command::new(
            spec("git.fetch", "Fetch", "Git", None, "{}"),
            idle,
            |state: &mut IdeState, _| {
                state
                    .start_remote(RemoteOperation::Fetch)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.pull",
                "Update Project…",
                "Git",
                Some("⌘T"),
                r#"{"rebase":"bool?"}"#,
            ),
            idle,
            |state: &mut IdeState, params| {
                let rebase = params
                    .get("rebase")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                state
                    .start_remote(RemoteOperation::Pull { rebase })
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.push",
                "Push…",
                "Git",
                Some("⇧⌘K"),
                r#"{"remote":"string?","target":"string?","force":"bool?"}"#,
            ),
            idle,
            |state: &mut IdeState, params| {
                let plan = state.push_plan().map_err(CommandError::Failed)?;
                let text = |name: &str, default: &str| {
                    params
                        .get(name)
                        .and_then(Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or(default)
                        .trim()
                        .to_owned()
                };
                let remote = text("remote", &plan.remote);
                let target = text("target", &plan.target);
                let set_upstream =
                    plan.new_branch || target != plan.target || remote != plan.remote;
                state
                    .start_remote(RemoteOperation::Push {
                        remote,
                        branch: plan.branch.clone(),
                        target,
                        set_upstream,
                        force_with_lease: params
                            .get("force")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                    .map_err(CommandError::Failed)?;
                Ok(json!({"commits": plan.commits.len()}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::{Value, json};
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, Instant};

    fn git(root: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn wait(state: &mut IdeState) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while state.remote_busy() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
            state.poll_remote();
        }
        assert!(!state.remote_busy());
    }

    #[test]
    fn pushes_and_pulls_in_the_background() {
        let base =
            std::env::temp_dir().join(format!("archipel-ide-core-remote-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        git(&base, &["init", "-q", "--bare", "-b", "main", "origin.git"]);
        for dir in ["a", "b"] {
            git(&base, &["clone", "-q", "origin.git", dir]);
            git(
                &base.join(dir),
                &["config", "user.email", "dev@example.invalid"],
            );
            git(&base.join(dir), &["config", "user.name", "Dev"]);
        }
        let a = base.join("a");
        fs::write(a.join("f.txt"), "1\n").unwrap();
        git(&a, &["add", "."]);
        git(&a, &["commit", "-qm", "first"]);

        let commands = built_in_commands();
        let mut a_state = IdeState::default();
        commands
            .execute("project.open", &mut a_state, json!({"path": a}))
            .unwrap();
        assert_eq!(a_state.push_plan().unwrap().commits.len(), 1);
        let result = commands
            .execute("git.push", &mut a_state, Value::Null)
            .unwrap();
        assert_eq!(result["commits"], 1);
        assert!(!commands.is_enabled("git.fetch", &a_state));
        wait(&mut a_state);
        assert!(a_state.status.starts_with("Pushed"), "{}", a_state.status);

        let b = base.join("b");
        let mut b_state = IdeState::default();
        commands
            .execute("project.open", &mut b_state, json!({"path": b}))
            .unwrap();
        commands
            .execute("git.pull", &mut b_state, Value::Null)
            .unwrap();
        wait(&mut b_state);
        assert!(
            b_state
                .files
                .iter()
                .any(|f| f.relative_path == Path::new("f.txt"))
        );
        assert!(
            b_state.remote_error().is_none(),
            "{:?}",
            b_state.remote_error()
        );

        // Failure is reported, not panicked.
        git(
            &a,
            &["remote", "set-url", "origin", "/nonexistent/repo.git"],
        );
        commands
            .execute("git.fetch", &mut a_state, Value::Null)
            .unwrap();
        wait(&mut a_state);
        assert!(
            a_state.status.starts_with("Fetch failed"),
            "{}",
            a_state.status
        );
        assert!(a_state.remote_error().is_some());
        fs::remove_dir_all(base).unwrap();
    }
}
