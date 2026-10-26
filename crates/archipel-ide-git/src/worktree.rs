//! Local-change operations of the Commit tool window: amend, rollback of
//! files, and stash/unstash.

use crate::{GitError, RepositoryStatus, checked, run};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Message of the last commit, for "Amend".
pub fn last_commit_message(root: &Path) -> Result<Option<String>, GitError> {
    let output = run(root, &["log", "-1", "--format=%B"])?;
    if !output.status.success() {
        return Ok(None); // No commits yet.
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned(),
    ))
}

impl RepositoryStatus {
    /// Like [`RepositoryStatus::commit`], optionally amending the last commit.
    /// When amending, `paths` may be empty to only change the message.
    pub fn commit_with(
        &self,
        message: &str,
        paths: &[PathBuf],
        amend: bool,
    ) -> Result<String, GitError> {
        if !amend {
            return self.commit(message, paths);
        }
        if message.trim().is_empty() {
            return Err(GitError::Command("commit message is empty".into()));
        }
        if last_commit_message(&self.root)?.is_none() {
            return Err(GitError::Command("there is no commit to amend".into()));
        }
        if !paths.is_empty() {
            let output = Command::new("git")
                .arg("-C")
                .arg(&self.root)
                .args(["add", "-A", "--"])
                .args(paths)
                .output()?;
            if !output.status.success() {
                return Err(GitError::Command(
                    String::from_utf8_lossy(&output.stderr).trim().into(),
                ));
            }
        }
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(&self.root)
            .args(["commit", "--quiet", "--amend", "-m", message]);
        if paths.is_empty() {
            command.arg("--only");
        } else {
            command.args(["--only", "--"]).args(paths);
        }
        let output = command.output()?;
        if !output.status.success() {
            return Err(GitError::Command(
                format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stderr).trim(),
                    String::from_utf8_lossy(&output.stdout).trim()
                )
                .trim()
                .into(),
            ));
        }
        let head = checked(&self.root, &["rev-parse", "--short", "HEAD"])?;
        Ok(String::from_utf8_lossy(&head.stdout).trim().into())
    }

    /// Discards local changes of repository-relative `paths`: tracked files
    /// are restored from HEAD (index and working tree); newly added files are
    /// unstaged and kept on disk as unversioned files.
    pub fn rollback(&self, paths: &[PathBuf]) -> Result<usize, GitError> {
        if paths.is_empty() {
            return Err(GitError::Command("no files selected for rollback".into()));
        }
        let mut in_head = Vec::new();
        let mut added = Vec::new();
        for path in paths {
            let spec = format!("HEAD:{}", crate::git_path(path));
            if run(&self.root, &["cat-file", "-e", &spec])?
                .status
                .success()
            {
                in_head.push(path.clone());
            } else {
                added.push(path.clone());
            }
        }
        let run_paths = |args: &[&str], paths: &[PathBuf]| -> Result<(), GitError> {
            if paths.is_empty() {
                return Ok(());
            }
            let output = Command::new("git")
                .arg("-C")
                .arg(&self.root)
                .args(args)
                .arg("--")
                .args(paths)
                .output()?;
            if output.status.success() {
                Ok(())
            } else {
                Err(GitError::Command(
                    String::from_utf8_lossy(&output.stderr).trim().into(),
                ))
            }
        };
        run_paths(
            &["restore", "--source=HEAD", "--staged", "--worktree"],
            &in_head,
        )?;
        run_paths(&["rm", "--cached", "--quiet", "-r"], &added)?;
        Ok(paths.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    /// Position in `git stash list` (`stash@{index}`).
    pub index: usize,
    /// e.g. "On main: work in progress".
    pub message: String,
}

pub fn stash_list(root: &Path) -> Result<Vec<StashEntry>, GitError> {
    let output = checked(root, &["stash", "list", "--format=%gs"])?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .enumerate()
        .map(|(index, message)| StashEntry {
            index,
            message: message.to_owned(),
        })
        .collect())
}

/// Stashes local changes. Returns `false` when there was nothing to stash.
pub fn stash_push(
    root: &Path,
    message: &str,
    keep_index: bool,
    include_untracked: bool,
) -> Result<bool, GitError> {
    let before = stash_list(root)?.len();
    let mut args = vec!["stash", "push", "--quiet"];
    if keep_index {
        args.push("--keep-index");
    }
    if include_untracked {
        args.push("--include-untracked");
    }
    if !message.trim().is_empty() {
        args.extend(["-m", message.trim()]);
    }
    checked(root, &args)?;
    Ok(stash_list(root)?.len() > before)
}

/// Applies stash `index`; `pop` also drops it when it applied cleanly.
pub fn stash_apply(root: &Path, index: usize, pop: bool) -> Result<(), GitError> {
    let name = format!("stash@{{{index}}}");
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["stash", if pop { "pop" } else { "apply" }, "--quiet", &name])
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Err(GitError::Command(if text.contains("CONFLICT") {
        "Unstash produced conflicts; resolve them in the Commit window (the stash was kept)".into()
    } else {
        text.trim().into()
    }))
}

pub fn stash_drop(root: &Path, index: usize) -> Result<(), GitError> {
    checked(
        root,
        &["stash", "drop", "--quiet", &format!("stash@{{{index}}}")],
    )
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn git(root: &Path, args: &[&str]) {
        let output = run(root, args).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn repo(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-worktree-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "dev@example.invalid"]);
        git(&root, &["config", "user.name", "Dev"]);
        root
    }

    #[test]
    fn amends_message_and_files() {
        let root = repo("amend");
        let status = RepositoryStatus::load(&root).unwrap();
        assert_eq!(last_commit_message(&root).unwrap(), None);
        assert!(status.commit_with("x", &[], true).is_err());
        fs::write(root.join("a.txt"), "a\n").unwrap();
        status.commit("Frist", &[PathBuf::from("a.txt")]).unwrap();
        assert_eq!(
            last_commit_message(&root).unwrap().as_deref(),
            Some("Frist")
        );
        status.commit_with("First", &[], true).unwrap();
        fs::write(root.join("b.txt"), "b\n").unwrap();
        status
            .commit_with("First\n\nwith b", &[PathBuf::from("b.txt")], true)
            .unwrap();
        let count = run(&root, &["rev-list", "--count", "HEAD"]).unwrap();
        assert_eq!(String::from_utf8_lossy(&count.stdout).trim(), "1");
        assert_eq!(
            last_commit_message(&root).unwrap().as_deref(),
            Some("First\n\nwith b")
        );
        assert!(RepositoryStatus::load(&root).unwrap().changes.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rolls_back_modified_deleted_and_added_files() {
        let root = repo("rollback");
        fs::write(root.join("m.txt"), "m\n").unwrap();
        fs::write(root.join("d.txt"), "d\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "base"]);
        fs::write(root.join("m.txt"), "changed\n").unwrap();
        git(&root, &["add", "m.txt"]);
        fs::remove_file(root.join("d.txt")).unwrap();
        fs::write(root.join("n.txt"), "new\n").unwrap();
        git(&root, &["add", "n.txt"]);
        let status = RepositoryStatus::load(&root).unwrap();
        let count = status
            .rollback(&[
                PathBuf::from("m.txt"),
                PathBuf::from("d.txt"),
                PathBuf::from("n.txt"),
            ])
            .unwrap();
        assert_eq!(count, 3);
        assert_eq!(fs::read_to_string(root.join("m.txt")).unwrap(), "m\n");
        assert!(root.join("d.txt").exists());
        // The added file stays on disk as unversioned.
        assert!(root.join("n.txt").exists());
        let after = RepositoryStatus::load(&root).unwrap();
        assert_eq!(after.changes.len(), 1);
        assert_eq!(after.changes[0].kind, crate::ChangeKind::Untracked);
        assert!(status.rollback(&[]).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stashes_and_unstashes_changes() {
        let root = repo("stash");
        fs::write(root.join("a.txt"), "a\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "base"]);
        assert!(!stash_push(&root, "nothing", false, false).unwrap());
        fs::write(root.join("a.txt"), "wip\n").unwrap();
        fs::write(root.join("u.txt"), "untracked\n").unwrap();
        assert!(stash_push(&root, "Work in progress", false, true).unwrap());
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "a\n");
        assert!(!root.join("u.txt").exists());
        let list = stash_list(&root).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].message.contains("Work in progress"), "{:?}", list);
        stash_apply(&root, 0, false).unwrap();
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "wip\n");
        assert_eq!(stash_list(&root).unwrap().len(), 1);
        git(&root, &["checkout", "--", "a.txt"]);
        fs::remove_file(root.join("u.txt")).unwrap();
        stash_apply(&root, 0, true).unwrap();
        assert!(stash_list(&root).unwrap().is_empty());
        assert!(root.join("u.txt").exists());
        assert!(stash_drop(&root, 0).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
