//! Network operations (fetch, pull, push) run on a background thread with
//! progress lines streamed back, plus the data the push dialog needs.

use crate::{CommitInfo, GitError, LogQuery, Notify, checked, log, run};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteOperation {
    /// Fetch all remotes, pruning deleted remote branches.
    Fetch,
    /// Pull the upstream of the current branch (merge, or rebase).
    Pull { rebase: bool },
    Push {
        remote: String,
        /// Local branch to push.
        branch: String,
        /// Branch name on the remote.
        target: String,
        set_upstream: bool,
        force_with_lease: bool,
    },
    /// `git clone url directory`; the job runs in the existing parent of `directory`.
    Clone { url: String, directory: String },
}

impl RemoteOperation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fetch => "Fetching",
            Self::Pull { .. } => "Updating",
            Self::Push { .. } => "Pushing",
            Self::Clone { .. } => "Cloning",
        }
    }

    fn args(&self) -> Result<Vec<String>, GitError> {
        let reject = |value: &str| value.is_empty() || value.starts_with('-');
        Ok(match self {
            Self::Clone { url, directory } => {
                // `ext::` and `fd::` transports run arbitrary commands.
                if reject(url) || reject(directory) || url.contains("::") {
                    return Err(GitError::Command("invalid repository URL".into()));
                }
                vec![
                    "clone".into(),
                    "--progress".into(),
                    "--".into(),
                    url.clone(),
                    directory.clone(),
                ]
            }
            Self::Fetch => ["fetch", "--all", "--prune", "--progress"]
                .map(String::from)
                .to_vec(),
            Self::Pull { rebase } => vec![
                "pull".into(),
                "--progress".into(),
                "--no-edit".into(),
                if *rebase { "--rebase" } else { "--no-rebase" }.into(),
            ],
            Self::Push {
                remote,
                branch,
                target,
                set_upstream,
                force_with_lease,
            } => {
                if reject(remote) || reject(branch) || reject(target) {
                    return Err(GitError::Command("invalid push target".into()));
                }
                let mut args = vec!["push".to_owned(), "--progress".to_owned()];
                if *set_upstream {
                    args.push("--set-upstream".into());
                }
                if *force_with_lease {
                    args.push("--force-with-lease".into());
                }
                args.push(remote.clone());
                args.push(format!("refs/heads/{branch}:refs/heads/{target}"));
                args
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteEvent {
    /// A progress line, e.g. "Receiving objects:  45% (9/20)".
    Progress(String),
    /// The operation ended: summary on success, Git's message on failure.
    Finished(Result<String, String>),
}

/// A running remote operation. Dropping it does not stop the process; the
/// thread finishes on its own.
pub struct RemoteJob {
    pub operation: RemoteOperation,
    events: Receiver<RemoteEvent>,
}

impl RemoteJob {
    pub fn start(
        root: &Path,
        operation: RemoteOperation,
        notify: Notify,
    ) -> Result<Self, GitError> {
        let args = operation.args()?;
        let mut child = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(&args)
            // Never block on a credential prompt nobody can answer.
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ASKPASS", "")
            .env("SSH_ASKPASS", "")
            .env("GIT_SSH_COMMAND", ssh_command())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let (sender, events) = mpsc::channel();
        let label = operation.label();
        thread::Builder::new()
            .name("archipel-ide-git-remote".into())
            .spawn(move || {
                let mut stdout = child.stdout.take();
                let stdout_thread = thread::spawn(move || {
                    let mut text = String::new();
                    if let Some(out) = stdout.as_mut() {
                        let _ = out.read_to_string(&mut text);
                    }
                    text
                });
                let mut transcript = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    let mut buffer = [0u8; 4096];
                    let mut line = Vec::new();
                    while let Ok(read) = stderr.read(&mut buffer) {
                        if read == 0 {
                            break;
                        }
                        for &byte in &buffer[..read] {
                            // Git redraws progress with carriage returns.
                            if byte == b'\r' || byte == b'\n' {
                                let text = String::from_utf8_lossy(&line).trim().to_owned();
                                if !text.is_empty() {
                                    transcript.push_str(&text);
                                    transcript.push('\n');
                                    let _ = sender.send(RemoteEvent::Progress(text));
                                    notify();
                                }
                                line.clear();
                            } else {
                                line.push(byte);
                            }
                        }
                    }
                }
                let stdout = stdout_thread.join().unwrap_or_default();
                let result = match child.wait() {
                    Ok(status) if status.success() => Ok(summary(label, &stdout, &transcript)),
                    Ok(_) => Err(failure(&stdout, &transcript)),
                    Err(error) => Err(error.to_string()),
                };
                let _ = sender.send(RemoteEvent::Finished(result));
                notify();
            })?;
        Ok(Self { operation, events })
    }

    /// Events received since the last call.
    pub fn poll(&self) -> Vec<RemoteEvent> {
        self.events.try_iter().collect()
    }
}

/// Batch mode for SSH so a missing key fails fast instead of prompting,
/// unless the user configured their own command.
fn ssh_command() -> String {
    std::env::var("GIT_SSH_COMMAND").unwrap_or_else(|_| "ssh -o BatchMode=yes".into())
}

fn summary(label: &str, stdout: &str, transcript: &str) -> String {
    let done = match label {
        "Fetching" => "Fetch finished",
        "Updating" => "Project updated",
        "Cloning" => "Repository cloned",
        _ => "Pushed",
    };
    if label == "Cloning" {
        return done.to_owned();
    }
    let text = format!("{stdout}\n{transcript}");
    if text.contains("Already up to date") || text.contains("Everything up-to-date") {
        return format!("{done}: already up to date");
    }
    // Push reports "   abc..def  main -> main"; pull/fetch similar lines.
    let updates = text
        .lines()
        .filter(|line| line.contains("->") && !line.trim_start().starts_with("Push"))
        .count();
    match updates {
        0 => done.to_owned(),
        1 => format!("{done}: 1 ref updated"),
        n => format!("{done}: {n} refs updated"),
    }
}

/// Git's own explanation of a failure, without progress noise.
fn failure(stdout: &str, transcript: &str) -> String {
    let lines: Vec<&str> = transcript
        .lines()
        .chain(stdout.lines())
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with("error:")
                || line.starts_with("fatal:")
                || line.starts_with("hint:")
                || line.starts_with("CONFLICT")
                || line.starts_with("! [")
        })
        .collect();
    if lines.is_empty() {
        transcript
            .lines()
            .last()
            .unwrap_or("Git command failed")
            .to_owned()
    } else {
        lines.join("\n")
    }
}

/// What the push dialog shows for the current branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushPlan {
    pub branch: String,
    pub remotes: Vec<String>,
    pub remote: String,
    /// Target branch on the remote.
    pub target: String,
    /// Whether the branch has no upstream yet (push sets it).
    pub new_branch: bool,
    /// Commits the push would send, newest first.
    pub commits: Vec<CommitInfo>,
}

pub fn remotes(root: &Path) -> Result<Vec<String>, GitError> {
    let output = checked(root, &["remote"])?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .filter(|name| !name.is_empty())
        .collect())
}

pub fn push_plan(root: &Path) -> Result<PushPlan, GitError> {
    let head = run(root, &["symbolic-ref", "--short", "-q", "HEAD"])?;
    if !head.status.success() {
        return Err(GitError::Command(
            "HEAD is detached; check out a branch to push".into(),
        ));
    }
    let branch = String::from_utf8_lossy(&head.stdout).trim().to_owned();
    let remotes = remotes(root)?;
    if remotes.is_empty() {
        return Err(GitError::Command(
            "the repository has no remotes; add one with `git remote add`".into(),
        ));
    }
    let upstream = run(
        root,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )?;
    let upstream = upstream
        .status
        .success()
        .then(|| String::from_utf8_lossy(&upstream.stdout).trim().to_owned());
    let (remote, target, new_branch) = match upstream.as_deref().and_then(|u| u.split_once('/')) {
        Some((remote, target)) => (remote.to_owned(), target.to_owned(), false),
        None => {
            let remote = if remotes.iter().any(|r| r == "origin") {
                "origin".to_owned()
            } else {
                remotes[0].clone()
            };
            (remote, branch.clone(), true)
        }
    };
    let commits = if new_branch {
        commits_not_on_remotes(root, &branch)?
    } else {
        log(
            root,
            &LogQuery {
                revision: Some(format!("{remote}/{target}..{branch}")),
                path: None,
                limit: 500,
            },
        )?
    };
    Ok(PushPlan {
        branch,
        remotes,
        remote,
        target,
        new_branch,
        commits,
    })
}

/// Commits of `branch` that no remote-tracking branch contains yet.
fn commits_not_on_remotes(root: &Path, branch: &str) -> Result<Vec<CommitInfo>, GitError> {
    let output = checked(
        root,
        &[
            "rev-list",
            "--max-count=500",
            branch,
            "--not",
            "--remotes",
            "--",
        ],
    )?;
    let hashes: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    if hashes.is_empty() {
        return Ok(Vec::new());
    }
    let history = log(
        root,
        &LogQuery {
            revision: Some(branch.to_owned()),
            path: None,
            limit: hashes.len(),
        },
    )?;
    Ok(history
        .into_iter()
        .filter(|commit| hashes.contains(&commit.hash))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn git(root: &Path, args: &[&str]) {
        let output = run(root, args).unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn wait(job: &RemoteJob) -> (Vec<String>, Result<String, String>) {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut progress = Vec::new();
        while Instant::now() < deadline {
            for event in job.poll() {
                match event {
                    RemoteEvent::Progress(line) => progress.push(line),
                    RemoteEvent::Finished(result) => return (progress, result),
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("remote operation did not finish");
    }

    fn setup(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let base =
            std::env::temp_dir().join(format!("archipel-ide-remote-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let bare = base.join("origin.git");
        git(&base, &["init", "-q", "--bare", "-b", "main", "origin.git"]);
        let clone = |dir: &str| {
            let path = base.join(dir);
            let output = Command::new("git")
                .args(["clone", "-q"])
                .arg(&bare)
                .arg(&path)
                .output()
                .unwrap();
            assert!(output.status.success());
            git(&path, &["config", "user.email", "dev@example.invalid"]);
            git(&path, &["config", "user.name", "Dev"]);
            path
        };
        let a = clone("a");
        let b = clone("b");
        (base, a, b)
    }

    #[test]
    fn clones_with_progress_and_rejects_command_transports() {
        let (base, _a, _b) = setup("clone");
        let noop: Notify = Arc::new(|| {});
        let target = base.join("cloned");
        let job = RemoteJob::start(
            &base,
            RemoteOperation::Clone {
                url: base.join("origin.git").display().to_string(),
                directory: target.display().to_string(),
            },
            noop.clone(),
        )
        .unwrap();
        let (_, result) = wait(&job);
        assert_eq!(result.unwrap(), "Repository cloned");
        assert!(target.join(".git").exists());
        for url in ["ext::sh -c id", "--upload-pack=x", ""] {
            let bad = RemoteOperation::Clone {
                url: url.into(),
                directory: "/tmp/x".into(),
            };
            assert!(RemoteJob::start(&base, bad, noop.clone()).is_err(), "{url}");
        }
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn pushes_fetches_and_pulls_through_a_bare_remote() {
        let (base, a, b) = setup("cycle");
        let noop: Notify = Arc::new(|| {});
        fs::write(a.join("f.txt"), "1\n").unwrap();
        git(&a, &["add", "."]);
        git(&a, &["commit", "-qm", "first"]);
        let plan = push_plan(&a).unwrap();
        assert_eq!(plan.remote, "origin");
        assert!(plan.new_branch);
        assert_eq!(plan.commits.len(), 1);
        let push = RemoteJob::start(
            &a,
            RemoteOperation::Push {
                remote: plan.remote.clone(),
                branch: plan.branch.clone(),
                target: plan.target.clone(),
                set_upstream: true,
                force_with_lease: false,
            },
            noop.clone(),
        )
        .unwrap();
        let (_, result) = wait(&push);
        assert!(result.unwrap().starts_with("Pushed"));
        let plan = push_plan(&a).unwrap();
        assert!(!plan.new_branch);
        assert!(plan.commits.is_empty());

        let (_, result) = wait(
            &RemoteJob::start(&b, RemoteOperation::Pull { rebase: false }, noop.clone()).unwrap(),
        );
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(fs::read_to_string(b.join("f.txt")).unwrap(), "1\n");

        fs::write(a.join("f.txt"), "2\n").unwrap();
        git(&a, &["commit", "-qam", "second"]);
        assert_eq!(push_plan(&a).unwrap().commits.len(), 1);
        let push_again = RemoteOperation::Push {
            remote: "origin".into(),
            branch: "main".into(),
            target: "main".into(),
            set_upstream: false,
            force_with_lease: false,
        };
        assert!(
            wait(&RemoteJob::start(&a, push_again, noop.clone()).unwrap())
                .1
                .is_ok()
        );
        let (_, fetched) =
            wait(&RemoteJob::start(&b, RemoteOperation::Fetch, noop.clone()).unwrap());
        assert!(fetched.is_ok(), "{fetched:?}");
        let branches = crate::branches(&b).unwrap();
        assert_eq!(branches.local[0].behind, 1);

        // A diverged push is rejected with Git's explanation.
        fs::write(b.join("g.txt"), "b\n").unwrap();
        git(&b, &["add", "."]);
        git(&b, &["commit", "-qm", "from b"]);
        let rejected = RemoteOperation::Push {
            remote: "origin".into(),
            branch: "main".into(),
            target: "main".into(),
            set_upstream: false,
            force_with_lease: false,
        };
        let (_, result) = wait(&RemoteJob::start(&b, rejected, noop.clone()).unwrap());
        let error = result.unwrap_err();
        assert!(
            error.contains("rejected") || error.contains("error:"),
            "{error}"
        );
        let (_, result) =
            wait(&RemoteJob::start(&b, RemoteOperation::Pull { rebase: true }, noop).unwrap());
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(fs::read_to_string(b.join("f.txt")).unwrap(), "2\n");
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn rejects_option_like_targets_and_explains_missing_remotes() {
        let invalid = RemoteOperation::Push {
            remote: "--exec=x".into(),
            branch: "main".into(),
            target: "main".into(),
            set_upstream: false,
            force_with_lease: false,
        };
        assert!(invalid.args().is_err());
        let root =
            std::env::temp_dir().join(format!("archipel-ide-remote-none-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        let error = push_plan(&root).unwrap_err().to_string();
        assert!(error.contains("no remotes"), "{error}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn summarizes_and_explains_output() {
        assert_eq!(
            summary("Pushing", "", "To x\n   a..b  main -> main\n"),
            "Pushed: 1 ref updated"
        );
        assert_eq!(
            summary("Updating", "Already up to date.\n", ""),
            "Project updated: already up to date"
        );
        assert_eq!(
            failure(
                "",
                "Counting objects\nerror: failed to push\nhint: pull first\n"
            ),
            "error: failed to push\nhint: pull first"
        );
    }
}
