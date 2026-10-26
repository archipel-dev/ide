//! Branch operations behind the branches popup: checkout, create, rename,
//! delete, merge and rebase, plus detection of an interrupted merge/rebase.

use crate::{GitError, checked, run};
use std::path::Path;

/// A multi-step operation that stopped (usually on conflicts) and needs to be
/// continued or aborted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOperation {
    Merge,
    Rebase,
    CherryPick,
    Revert,
}

impl PendingOperation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Merge => "Merge",
            Self::Rebase => "Rebase",
            Self::CherryPick => "Cherry-Pick",
            Self::Revert => "Revert",
        }
    }
}

/// Rejects names `git check-ref-format --branch` would reject, and names that
/// could be mistaken for options.
pub fn validate_branch_name(root: &Path, name: &str) -> Result<(), GitError> {
    if name.trim().is_empty() {
        return Err(GitError::Command("branch name is empty".into()));
    }
    if name.starts_with('-') || name.contains(char::is_whitespace) {
        return Err(GitError::Command(format!(
            "`{name}` is not a valid branch name"
        )));
    }
    if !run(root, &["check-ref-format", "--branch", name])?
        .status
        .success()
    {
        return Err(GitError::Command(format!(
            "`{name}` is not a valid branch name"
        )));
    }
    Ok(())
}

fn validate_revision(revision: &str) -> Result<(), GitError> {
    if revision.trim().is_empty() || revision.starts_with('-') {
        return Err(GitError::Command(format!("invalid revision `{revision}`")));
    }
    Ok(())
}

fn ref_exists(root: &Path, reference: &str) -> Result<bool, GitError> {
    Ok(run(root, &["show-ref", "--verify", "--quiet", reference])?
        .status
        .success())
}

/// Checks out a local branch, a remote branch (creating a local tracking
/// branch of the same name, or switching to it if it exists) or any other
/// revision (detached HEAD).
pub fn checkout(root: &Path, target: &str) -> Result<String, GitError> {
    validate_revision(target)?;
    if ref_exists(root, &format!("refs/heads/{target}"))? {
        checked(root, &["switch", "--quiet", target])?;
        return Ok(format!("Checked out {target}"));
    }
    if ref_exists(root, &format!("refs/remotes/{target}"))? {
        let local = target.split_once('/').map_or(target, |(_, branch)| branch);
        if ref_exists(root, &format!("refs/heads/{local}"))? {
            checked(root, &["switch", "--quiet", local])?;
        } else {
            checked(root, &["switch", "--quiet", "--track", "-c", local, target])?;
        }
        return Ok(format!("Checked out {local} (tracking {target})"));
    }
    checked(root, &["switch", "--quiet", "--detach", target])?;
    Ok(format!("Checked out revision {target} (detached HEAD)"))
}

/// Creates `name` at `start` (default HEAD), optionally checking it out.
pub fn create_branch(
    root: &Path,
    name: &str,
    start: Option<&str>,
    checkout: bool,
) -> Result<String, GitError> {
    validate_branch_name(root, name)?;
    if ref_exists(root, &format!("refs/heads/{name}"))? {
        return Err(GitError::Command(format!("branch `{name}` already exists")));
    }
    let start = start.unwrap_or("HEAD");
    validate_revision(start)?;
    if checkout {
        checked(root, &["switch", "--quiet", "-c", name, start])?;
        Ok(format!("Created and checked out {name}"))
    } else {
        checked(root, &["branch", "--quiet", name, start])?;
        Ok(format!("Created branch {name}"))
    }
}

pub fn rename_branch(root: &Path, old: &str, new: &str) -> Result<String, GitError> {
    validate_revision(old)?;
    validate_branch_name(root, new)?;
    checked(root, &["branch", "--quiet", "-m", old, new])?;
    Ok(format!("Renamed {old} to {new}"))
}

/// Deletes a local branch. Without `force` an unmerged branch is refused.
pub fn delete_branch(root: &Path, name: &str, force: bool) -> Result<String, GitError> {
    validate_revision(name)?;
    let flag = if force { "-D" } else { "-d" };
    checked(root, &["branch", "--quiet", flag, name]).map_err(|error| match error {
        GitError::Command(message) if message.contains("not fully merged") => {
            GitError::Command(format!("Branch {name} is not fully merged"))
        }
        other => other,
    })?;
    Ok(format!("Deleted branch {name}"))
}

/// Output of a merge or rebase that ran to completion or stopped on conflicts.
fn finish(root: &Path, output: std::process::Output, done: String) -> Result<String, GitError> {
    if output.status.success() {
        return Ok(done);
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if pending_operation(root)?.is_some() {
        let conflicts = text
            .lines()
            .filter(|line| line.starts_with("CONFLICT"))
            .count();
        return Err(GitError::Command(format!(
            "{} stopped with {} conflict{}; resolve them and commit, or abort",
            done.split(' ').next().unwrap_or("Operation"),
            conflicts.max(1),
            if conflicts == 1 { "" } else { "s" }
        )));
    }
    Err(GitError::Command(text.trim().into()))
}

/// Merges `branch` into the current branch.
pub fn merge(root: &Path, branch: &str) -> Result<String, GitError> {
    validate_revision(branch)?;
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["merge", "--no-edit", branch])
        .output()?;
    finish(root, output, format!("Merge of {branch} succeeded"))
}

/// Applies the changes of `commit` onto the current branch.
pub fn cherry_pick(root: &Path, commit: &str) -> Result<String, GitError> {
    validate_revision(commit)?;
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .env("GIT_EDITOR", "true")
        .args(["cherry-pick", commit])
        .output()?;
    finish(root, output, format!("Cherry-picked {commit}"))
}

/// Rebases the current branch onto `onto`.
pub fn rebase(root: &Path, onto: &str) -> Result<String, GitError> {
    validate_revision(onto)?;
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .env("GIT_EDITOR", "true")
        .args(["rebase", onto])
        .output()?;
    finish(root, output, format!("Rebase onto {onto} succeeded"))
}

/// The interrupted operation in progress, if any.
pub fn pending_operation(root: &Path) -> Result<Option<PendingOperation>, GitError> {
    let output = checked(
        root,
        &[
            "rev-parse",
            "--git-path",
            "MERGE_HEAD",
            "--git-path",
            "rebase-merge",
            "--git-path",
            "rebase-apply",
            "--git-path",
            "CHERRY_PICK_HEAD",
            "--git-path",
            "REVERT_HEAD",
        ],
    )?;
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let paths: Vec<&str> = text.lines().collect();
    let exists = |index: usize| {
        paths
            .get(index)
            .is_some_and(|path| root.join(path).exists())
    };
    Ok(if exists(1) || exists(2) {
        Some(PendingOperation::Rebase)
    } else if exists(0) {
        Some(PendingOperation::Merge)
    } else if exists(3) {
        Some(PendingOperation::CherryPick)
    } else if exists(4) {
        Some(PendingOperation::Revert)
    } else {
        None
    })
}

/// Aborts the interrupted operation, restoring the state before it started.
pub fn abort_operation(root: &Path) -> Result<String, GitError> {
    let operation = pending_operation(root)?
        .ok_or_else(|| GitError::Command("no merge, rebase or cherry-pick in progress".into()))?;
    let command = match operation {
        PendingOperation::Merge => "merge",
        PendingOperation::Rebase => "rebase",
        PendingOperation::CherryPick => "cherry-pick",
        PendingOperation::Revert => "revert",
    };
    checked(root, &[command, "--abort"])?;
    Ok(format!("{} aborted", operation.label()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RepositoryStatus, branches};
    use std::fs;

    fn git(root: &Path, args: &[&str]) {
        let output = run(root, args).unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn repo(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-branch-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "dev@example.invalid"]);
        git(&root, &["config", "user.name", "Dev"]);
        fs::write(root.join("a.txt"), "base\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "base"]);
        root
    }

    fn current(root: &Path) -> String {
        RepositoryStatus::load(root).unwrap().branch
    }

    #[test]
    fn creates_checks_out_renames_and_deletes_branches() {
        let root = repo("lifecycle");
        assert!(validate_branch_name(&root, "feature/x").is_ok());
        for bad in ["", "-x", "a b", "a..b", "x.lock"] {
            assert!(validate_branch_name(&root, bad).is_err(), "{bad}");
        }
        create_branch(&root, "feature/x", None, true).unwrap();
        assert_eq!(current(&root), "feature/x");
        assert!(create_branch(&root, "feature/x", None, false).is_err());
        create_branch(&root, "other", Some("main"), false).unwrap();
        assert_eq!(current(&root), "feature/x");
        checkout(&root, "main").unwrap();
        assert_eq!(current(&root), "main");
        rename_branch(&root, "other", "renamed").unwrap();
        let names: Vec<_> = branches(&root)
            .unwrap()
            .local
            .into_iter()
            .map(|b| b.name)
            .collect();
        assert_eq!(names, vec!["feature/x", "main", "renamed"]);
        // An unmerged branch needs force.
        checkout(&root, "feature/x").unwrap();
        fs::write(root.join("b.txt"), "b\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "b"]);
        checkout(&root, "main").unwrap();
        let error = delete_branch(&root, "feature/x", false).unwrap_err();
        assert!(error.to_string().contains("not fully merged"), "{error}");
        delete_branch(&root, "feature/x", true).unwrap();
        delete_branch(&root, "renamed", false).unwrap();
        assert!(checkout(&root, "--orphan").is_err());
        let head = run(&root, &["rev-parse", "HEAD"]).unwrap();
        let head = String::from_utf8_lossy(&head.stdout).trim().to_owned();
        checkout(&root, &head).unwrap();
        assert!(current(&root).starts_with("detached@"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn checks_out_remote_branches_as_tracking_branches() {
        let upstream = repo("upstream");
        git(&upstream, &["branch", "topic"]);
        let clone =
            std::env::temp_dir().join(format!("archipel-ide-branch-clone-{}", std::process::id()));
        let _ = fs::remove_dir_all(&clone);
        let output = std::process::Command::new("git")
            .args(["clone", "-q"])
            .arg(&upstream)
            .arg(&clone)
            .output()
            .unwrap();
        assert!(output.status.success());
        let message = checkout(&clone, "origin/topic").unwrap();
        assert!(message.contains("tracking origin/topic"), "{message}");
        assert_eq!(current(&clone), "topic");
        let local = branches(&clone).unwrap().local;
        let topic = local.iter().find(|b| b.name == "topic").unwrap();
        assert_eq!(topic.upstream.as_deref(), Some("origin/topic"));
        fs::remove_dir_all(upstream).unwrap();
        fs::remove_dir_all(clone).unwrap();
    }

    #[test]
    fn merges_rebases_and_aborts_conflicts() {
        let root = repo("merge");
        git(&root, &["checkout", "-qb", "side"]);
        fs::write(root.join("side.txt"), "side\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "side"]);
        git(&root, &["checkout", "-q", "main"]);
        fs::write(root.join("main.txt"), "main\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "main"]);
        merge(&root, "side").unwrap();
        assert!(root.join("side.txt").exists());
        assert_eq!(pending_operation(&root).unwrap(), None);

        // Conflicting change on both branches.
        git(&root, &["checkout", "-qb", "conflict"]);
        fs::write(root.join("a.txt"), "theirs\n").unwrap();
        git(&root, &["commit", "-qam", "theirs"]);
        git(&root, &["checkout", "-q", "main"]);
        fs::write(root.join("a.txt"), "ours\n").unwrap();
        git(&root, &["commit", "-qam", "ours"]);
        let error = merge(&root, "conflict").unwrap_err().to_string();
        assert!(error.contains("1 conflict"), "{error}");
        assert_eq!(
            pending_operation(&root).unwrap(),
            Some(PendingOperation::Merge)
        );
        abort_operation(&root).unwrap();
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "ours\n");

        let error = rebase(&root, "conflict").unwrap_err().to_string();
        assert!(error.contains("Rebase stopped"), "{error}");
        assert_eq!(
            pending_operation(&root).unwrap(),
            Some(PendingOperation::Rebase)
        );
        assert_eq!(abort_operation(&root).unwrap(), "Rebase aborted");
        assert!(abort_operation(&root).is_err());

        git(&root, &["checkout", "-qb", "linear", "side"]);
        rebase(&root, "main").unwrap();
        assert!(root.join("main.txt").exists());
        assert!(merge(&root, "-n").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
