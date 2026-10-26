//! Git side of a session: the worktree an agent works in, the changes it made against the
//! merge base, and merging or discarding them.
//!
//! Worktrees come from `archipel_ide_git::worktrees` (under the data dir, outside the
//! repository); every other git call is made with an argument vector, never a shell.

use archipel_ide_git::FileDiff;
use archipel_ide_git::worktrees::{self, AddOptions, RemoveOptions};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Files above this size get statistics but no cached diff.
const MAX_DIFF_BYTES: u64 = 512 * 1024;
/// At most this many files are listed with a diff.
const MAX_FILES: usize = 300;
const AGENT_IDENTITY: [&str; 4] = [
    "-c",
    "user.name=Archipel Agent",
    "-c",
    "user.email=agent@archipel.invalid",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
}

/// One file an agent changed, against the merge base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub added: usize,
    pub removed: usize,
    /// First changed line in the new text, 1-based.
    pub first_line: usize,
}

#[derive(Debug, Default)]
pub struct ChangeSet {
    pub changes: Vec<Change>,
    pub diffs: HashMap<PathBuf, FileDiff>,
}

fn git<I, S>(cwd: &Path, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    git_raw(cwd, args).map(|bytes| String::from_utf8_lossy(&bytes).trim_end().to_owned())
}

fn git_raw<I, S>(cwd: &Path, args: I) -> Result<Vec<u8>, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let mut text = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if text.is_empty() {
            text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        }
        Err(text)
    }
}

/// The branch checked out in `root`; `None` when HEAD is detached or `root` is no repository.
pub fn current_branch(root: &Path) -> Option<String> {
    git(root, ["symbolic-ref", "--quiet", "--short", "HEAD"])
        .ok()
        .filter(|b| !b.is_empty())
}

/// Creates the worktree for a session and copies the `.worktreeinclude` files.
pub fn create_worktree(
    root: &Path,
    path: &Path,
    branch: &str,
    base_branch: &str,
) -> Result<(), String> {
    worktrees::add(
        root,
        &AddOptions {
            path: path.to_path_buf(),
            branch: branch.to_owned(),
            base: Some(base_branch.to_owned()),
            ..AddOptions::default()
        },
    )
    .map_err(|e| format!("could not create the worktree: {e}"))?;
    let include = worktrees::resolve_include_paths(root);
    if !include.is_empty() {
        let _ = worktrees::copy_include_paths(root, path, &include);
    }
    Ok(())
}

/// Changes of `worktree` against the merge base with `base`, including new files.
pub fn compute_changes(worktree: &Path, base: &str) -> Result<ChangeSet, String> {
    // Make untracked files visible to `git diff` (intent to add); ignored files stay out.
    let _ = git(worktree, ["add", "-A", "-N"]);
    let stat = worktrees::diff_stat(worktree, base).map_err(|e| e.to_string())?;
    let anchor = worktrees::merge_base(worktree, base, "HEAD")
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("no common history with `{base}`"))?;
    let short: String = anchor.chars().take(8).collect();
    let mut set = ChangeSet::default();
    for file in stat.files.iter().take(MAX_FILES) {
        let rel = &file.path;
        let spec = format!("{anchor}:{}", rel.to_string_lossy());
        let old = git_raw(worktree, ["show", spec.as_str()])
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
        let absolute = worktree.join(rel);
        let too_big = std::fs::metadata(&absolute).is_ok_and(|m| m.len() > MAX_DIFF_BYTES);
        let binary = file.added.is_none();
        let new = if too_big || binary {
            None
        } else {
            std::fs::read(&absolute)
                .ok()
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        };
        let kind = match (&old, absolute.exists()) {
            (None, _) => ChangeKind::Added,
            (Some(_), false) => ChangeKind::Deleted,
            _ => ChangeKind::Modified,
        };
        let mut first_line = 1;
        if !too_big && !binary {
            let diff = FileDiff::compute(
                rel.clone(),
                (kind != ChangeKind::Added).then(|| short.clone()),
                old.unwrap_or_default(),
                new.unwrap_or_default(),
            );
            if let Some(hunk) = diff.hunks.first() {
                first_line = hunk.new.start + 1;
            }
            set.diffs.insert(rel.clone(), diff);
        }
        set.changes.push(Change {
            path: rel.clone(),
            kind,
            added: file.added.unwrap_or(0) as usize,
            removed: file.deleted.unwrap_or(0) as usize,
            first_line,
        });
    }
    Ok(set)
}

fn has_identity(cwd: &Path) -> bool {
    git(cwd, ["config", "user.email"]).is_ok_and(|e| !e.is_empty())
}

fn identity_args(cwd: &Path) -> Vec<&'static str> {
    if has_identity(cwd) {
        Vec::new()
    } else {
        AGENT_IDENTITY.to_vec()
    }
}

/// Commits everything in the worktree. `Ok(false)` when there was nothing to commit.
pub fn commit_all(worktree: &Path, message: &str) -> Result<bool, String> {
    git(worktree, ["add", "-A"])?;
    if git(worktree, ["diff", "--cached", "--quiet"]).is_ok() {
        return Ok(false);
    }
    let mut args: Vec<&str> = identity_args(worktree);
    args.extend([
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--no-verify",
        "-m",
        message,
    ]);
    git(worktree, args)?;
    Ok(true)
}

/// Whether `branch` has commits that `base` lacks.
pub fn has_new_commits(root: &Path, base: &str, branch: &str) -> bool {
    git(root, ["rev-list", "--count", &format!("{base}..{branch}")])
        .ok()
        .and_then(|n| n.parse::<u64>().ok())
        .is_some_and(|n| n > 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integration {
    Squash,
    Merge,
}

/// Merges `branch` into `base_branch` in the project checkout, which must have the base
/// branch checked out and no uncommitted changes to tracked files. On conflicts or a failing
/// commit the checkout is put back as it was.
pub fn integrate(
    root: &Path,
    branch: &str,
    base_branch: &str,
    how: Integration,
    message: &str,
) -> Result<(), String> {
    match current_branch(root) {
        Some(current) if current == base_branch => {}
        Some(current) => {
            return Err(format!(
                "the project has `{current}` checked out, but this agent branched from \
                 `{base_branch}`; check out `{base_branch}` first"
            ));
        }
        None => {
            return Err(format!(
                "the project has no branch checked out; check out `{base_branch}` first"
            ));
        }
    }
    let dirty = git(root, ["status", "--porcelain", "--untracked-files=no"])?;
    if !dirty.is_empty() {
        return Err(format!(
            "`{base_branch}` has uncommitted changes; commit or stash them first"
        ));
    }
    let identity = identity_args(root);
    match how {
        Integration::Squash => {
            if let Err(error) = git(root, ["merge", "--squash", branch]) {
                let _ = git(root, ["reset", "--hard", "HEAD"]);
                return Err(format!(
                    "squash merge of `{branch}` into `{base_branch}` failed (conflicts?): {error}"
                ));
            }
            let mut args: Vec<&str> = identity;
            args.extend(["-c", "commit.gpgsign=false", "commit", "-m", message]);
            if let Err(error) = git(root, args) {
                let _ = git(root, ["reset", "--hard", "HEAD"]);
                return Err(format!("could not commit the squashed changes: {error}"));
            }
        }
        Integration::Merge => {
            let mut args: Vec<&str> = identity;
            args.extend([
                "-c",
                "commit.gpgsign=false",
                "merge",
                "--no-ff",
                "-m",
                message,
                branch,
            ]);
            if let Err(error) = git(root, args) {
                let _ = git(root, ["merge", "--abort"]);
                return Err(format!(
                    "merge of `{branch}` into `{base_branch}` failed (conflicts?): {error}"
                ));
            }
        }
    }
    Ok(())
}

/// Removes the worktree and its branch. The branch is only deleted when `delete_branch`.
pub fn remove_worktree(
    root: &Path,
    path: &Path,
    branch: &str,
    delete_branch: bool,
) -> Result<(), String> {
    if path.exists() {
        worktrees::remove(
            root,
            path,
            &RemoveOptions {
                force: true,
                ..RemoveOptions::default()
            },
        )
        .map_err(|e| format!("could not remove the worktree: {e}"))?;
    } else {
        let _ = worktrees::prune(root);
    }
    if delete_branch && worktrees::branch_exists(root, branch) {
        git(root, ["branch", "-D", branch])?;
        let _ = worktrees::clear_branch_base(root, branch);
    }
    Ok(())
}
