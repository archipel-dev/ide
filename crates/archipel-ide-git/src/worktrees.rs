//! Linked worktrees for agent sessions: list, add, remove, prune, lock, lineage, name and path
//! helpers, orphan detection, and the include and shared-directory helpers.
//!
//! Behavior modeled on Orca (src/main/ipc/worktree-logic.ts, src/main/git/worktree-add.ts,
//! src/main/git/worktree-include-file.ts, src/main/git/worktree-shared-directories.ts,
//! src/main/git/worktree-removal-preflight.ts, src/main/worktree-removal-safety.ts,
//! src/main/worktree-orphan-gitdir-proof.ts), MIT; see THIRD_PARTY_NOTICES.md.
//!
//! This module is git operations only, no policy: callers decide names, base refs, which
//! directories to share and when to remove. Every git call passes its arguments as a vector,
//! never through a shell. Not to be confused with `worktree.rs` (amend and stash).

use crate::GitError;
use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{Arc, Mutex, OnceLock},
};
use thiserror::Error;

/// File at the repo root listing literal ignored paths to copy into new worktrees.
pub const INCLUDE_FILE: &str = ".worktreeinclude";
const INCLUDE_MAX_FILE_BYTES: u64 = 256 * 1024;
const INCLUDE_MAX_ENTRIES: usize = 1000;
const COPY_MAX_FILES: usize = 20_000;
const SLUG_MAX_CHARS: usize = 100;

#[derive(Debug, Error)]
pub enum WorktreeError {
    #[error(transparent)]
    Git(#[from] GitError),
    /// Another git process holds a repository lock; trying again shortly usually works.
    #[error("the repository is locked by another Git process, try again: {0}")]
    Locked(String),
    #[error("the worktree has uncommitted or untracked changes:\n{0}")]
    Dirty(String),
    #[error(
        "the worktree has uncommitted changes and contains a populated submodule; forcing would destroy submodule-only commits:\n{0}"
    )]
    SubmoduleDirty(String),
    #[error("refusing to remove protected path {0}")]
    Protected(PathBuf),
    #[error("{0} is not a registered worktree of this repository")]
    Unregistered(PathBuf),
    #[error("the worktree {0} is locked; unlock it first")]
    LockedWorktree(PathBuf),
    #[error("{0} contains another registered worktree")]
    Nested(PathBuf),
    #[error("invalid worktree name")]
    InvalidName,
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("invalid base reference `{0}`")]
    InvalidBase(String),
    #[error("the directory {0} is not provably an orphaned worktree of this repository")]
    NotOrphan(PathBuf),
}

impl WorktreeError {
    /// True when the same call may succeed if repeated after a short wait.
    pub fn is_retryable(&self) -> bool {
        matches!(self, WorktreeError::Locked(_))
    }
}

type Result<T> = std::result::Result<T, WorktreeError>;

// ---------------------------------------------------------------------------------------------
// git plumbing

fn git<I, S>(cwd: &Path, args: I) -> std::io::Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
}

fn git_ok<I, S>(cwd: &Path, args: I) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = git(cwd, args).map_err(GitError::from)?;
    if output.status.success() {
        Ok(output)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(classify(stderr))
    }
}

/// Lock contention is retryable; everything else is a plain command failure.
fn classify(stderr: String) -> WorktreeError {
    let lower = stderr.to_lowercase();
    let lock = lower.contains("index.lock")
        || (lower.contains(".lock") && lower.contains("file exists"))
        || lower.contains("could not lock")
        || lower.contains("unable to lock")
        || lower.contains("another git process seems to be running")
        || lower.contains("cannot lock ref");
    if lock {
        WorktreeError::Locked(stderr)
    } else {
        WorktreeError::Git(GitError::Command(stderr))
    }
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn reject_option_like(value: &str, what: &str) -> Result<()> {
    if value.trim().is_empty() || value.starts_with('-') || value.contains('\0') {
        return Err(WorktreeError::InvalidBase(format!("{what}: {value}")));
    }
    Ok(())
}

/// Absolute git common directory (shared by all worktrees of a repository).
pub fn common_dir(root: &Path) -> Result<PathBuf> {
    let output = git_ok(root, ["rev-parse", "--git-common-dir"])?;
    let raw = PathBuf::from(stdout_text(&output));
    let abs = if raw.is_absolute() {
        raw
    } else {
        root.join(raw)
    };
    Ok(fs::canonicalize(&abs).unwrap_or(abs))
}

fn repo_locks() -> &'static Mutex<HashMap<PathBuf, Arc<Mutex<()>>>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Serialize worktree mutations per repository: git takes repo-wide locks, so parallel adds
/// fail with lock errors.
fn with_repo_queue<T>(root: &Path, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let key = common_dir(root)?;
    let gate = {
        let mut map = repo_locks().lock().unwrap_or_else(|e| e.into_inner());
        map.entry(key).or_default().clone()
    };
    let _guard = gate.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

// ---------------------------------------------------------------------------------------------
// paths

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn resolve(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| {
        // Canonicalize the deepest existing ancestor so symlinked parents are resolved.
        let normalized = normalize(path);
        let mut tail = Vec::new();
        let mut cursor = normalized.as_path();
        while let Some(parent) = cursor.parent() {
            if let Ok(real) = fs::canonicalize(cursor) {
                let mut full = real;
                full.extend(tail.iter().rev());
                return full;
            }
            if let Some(name) = cursor.file_name() {
                tail.push(name.to_owned());
            }
            cursor = parent;
        }
        normalized
    })
}

/// Path equality after normalization and symlink resolution of existing parts.
pub fn same_path(a: &Path, b: &Path) -> bool {
    resolve(a) == resolve(b)
}

fn contains_path(parent: &Path, child: &Path) -> bool {
    resolve(child).starts_with(resolve(parent))
}

/// Home directory from the environment, if any.
pub fn home_dir() -> Option<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}

/// Paths a worktree removal must never touch: empty, the repository itself, a filesystem root,
/// anything containing the repository, the home directory or anything containing it, and the
/// usual parents of homes (`/home`, `/Users`, `/home/<user>`, `/Users/<user>`).
/// An unknown home counts as dangerous: it cannot be checked, so deletion is not authorized.
pub fn is_dangerous_path(path: &Path, repo: &Path, home: Option<&Path>) -> bool {
    if path.as_os_str().is_empty() || path.to_string_lossy().trim().is_empty() {
        return true;
    }
    if !path.is_absolute() {
        return true;
    }
    let resolved = resolve(path);
    if resolved.parent().is_none() {
        return true;
    }
    if resolved == resolve(repo) || resolve(repo).starts_with(&resolved) {
        return true;
    }
    let Some(home) = home else {
        return true;
    };
    let home = resolve(home);
    if home.starts_with(&resolved) {
        return true;
    }
    let shape: Vec<String> = resolved
        .components()
        .filter_map(|c| match c {
            Component::Normal(n) => Some(n.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let top = shape.first().map(String::as_str);
    matches!(
        (top, shape.len()),
        (Some("home" | "Users"), 1 | 2) | (Some("root"), 1)
    )
}

/// Reject a target that escapes `base` (after normalization and symlink resolution).
pub fn ensure_within(target: &Path, base: &Path) -> Result<PathBuf> {
    let resolved = resolve(target);
    if !resolved.starts_with(resolve(base)) || resolved == resolve(base) {
        return Err(WorktreeError::InvalidPath(format!(
            "{} is outside {}",
            target.display(),
            base.display()
        )));
    }
    Ok(resolved)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// `<repo-name>-<hash6>`: readable and stable per repository location.
pub fn repo_dir_name(repo_root: &Path) -> String {
    let real = resolve(repo_root);
    let name = real
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = name.strip_suffix(".git").unwrap_or(&name);
    let name = sanitize_name(name).unwrap_or_else(|_| "repo".into());
    let hash = fnv1a(real.to_string_lossy().as_bytes());
    format!("{name}-{:06x}", hash & 0xff_ffff)
}

/// `<data_dir>/archipel/worktrees/<repo-name>-<hash6>`; never inside the repository.
pub fn managed_base(data_dir: &Path, repo_root: &Path) -> PathBuf {
    data_dir
        .join("archipel")
        .join("worktrees")
        .join(repo_dir_name(repo_root))
}

/// Pick a free directory for `slug` under the managed base, appending `-2`, `-3`, ...
/// The slug is sanitized again, so a hostile name cannot escape the base.
pub fn plan_path(data_dir: &Path, repo_root: &Path, slug: &str) -> Result<PathBuf> {
    plan_path_in(&managed_base(data_dir, repo_root), repo_root, slug)
}

/// Like [`plan_path`] with an explicit base directory (a user-chosen worktree location).
pub fn plan_path_in(base: &Path, repo_root: &Path, slug: &str) -> Result<PathBuf> {
    let slug = sanitize_name(slug)?;
    let base = base.to_path_buf();
    if contains_path(repo_root, &base) {
        return Err(WorktreeError::InvalidPath(
            "the worktree directory must be outside the repository".into(),
        ));
    }
    let mut n = 1u32;
    loop {
        let name = if n == 1 {
            slug.clone()
        } else {
            format!("{slug}-{n}")
        };
        let candidate = base.join(&name);
        if fs::symlink_metadata(&candidate).is_err() {
            ensure_within(&candidate, &base)?;
            return Ok(candidate);
        }
        n += 1;
        if n > 10_000 {
            return Err(WorktreeError::InvalidName);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// names

fn is_emoji(c: char) -> bool {
    matches!(c as u32,
        0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0x2B00..=0x2BFF | 0x20E3 | 0xFE0F | 0x200D)
}

/// Turn free text into a safe directory and branch component. Keeps Unicode letters and digits
/// plus `._-`, turns everything else into `-`, collapses `..`, trims `.` and `-` from the ends,
/// avoids the `.lock` suffix git forbids, and limits the length. Emoji-only input becomes
/// `workspace`. Errors on empty results.
pub fn sanitize_name(input: &str) -> Result<String> {
    let mut out = String::new();
    let mut pending_dash = false;
    for c in input.trim().chars() {
        if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
    }
    if pending_dash {
        out.push('-');
    }
    // Collapse dash runs and dot runs (git rejects `..`).
    let mut collapsed = String::new();
    for c in out.chars() {
        let last = collapsed.chars().last();
        if (c == '-' && last == Some('-')) || (c == '.' && last == Some('.')) {
            continue;
        }
        collapsed.push(c);
    }
    let mut result = collapsed.trim_matches(['.', '-']).to_owned();
    while let Some(stripped) = result.strip_suffix(".lock") {
        result = format!("{}-lock", stripped.trim_end_matches(['.', '-']));
        if result == "-lock" {
            break;
        }
    }
    if result.chars().count() > SLUG_MAX_CHARS {
        result = result
            .chars()
            .take(SLUG_MAX_CHARS)
            .collect::<String>()
            .trim_matches(['.', '-'])
            .to_owned();
    }
    if result.is_empty() {
        if input.chars().any(is_emoji) {
            return Ok("workspace".into());
        }
        return Err(WorktreeError::InvalidName);
    }
    if result == "." || result == ".." {
        return Err(WorktreeError::InvalidName);
    }
    Ok(result)
}

// ---------------------------------------------------------------------------------------------
// list

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub head: Option<String>,
    /// Short branch name when a branch is checked out.
    pub branch: Option<String>,
    pub detached: bool,
    pub bare: bool,
    /// `Some(reason)` when locked (reason may be empty).
    pub locked: Option<String>,
    /// `Some(reason)` when git would prune it (its directory is gone).
    pub prunable: Option<String>,
    /// The main worktree (the first entry git lists).
    pub is_main: bool,
}

pub fn list(root: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = git_ok(root, ["worktree", "list", "--porcelain", "-z"])?;
    Ok(parse_list(&output.stdout))
}

fn parse_list(bytes: &[u8]) -> Vec<WorktreeInfo> {
    let mut out: Vec<WorktreeInfo> = Vec::new();
    let mut current: Option<WorktreeInfo> = None;
    for field in bytes.split(|b| *b == 0) {
        if field.is_empty() {
            if let Some(done) = current.take() {
                out.push(done);
            }
            continue;
        }
        let text = String::from_utf8_lossy(field);
        let (key, value) = text.split_once(' ').unwrap_or((&text, ""));
        if key == "worktree" {
            if let Some(done) = current.take() {
                out.push(done);
            }
            current = Some(WorktreeInfo {
                path: path_from_bytes(&field[key.len() + 1..]),
                head: None,
                branch: None,
                detached: false,
                bare: false,
                locked: None,
                prunable: None,
                is_main: false,
            });
            continue;
        }
        let Some(item) = current.as_mut() else {
            continue;
        };
        match key {
            "HEAD" => item.head = Some(value.to_owned()),
            "branch" => {
                item.branch = Some(
                    value
                        .strip_prefix("refs/heads/")
                        .unwrap_or(value)
                        .to_owned(),
                );
            }
            "detached" => item.detached = true,
            "bare" => item.bare = true,
            "locked" => item.locked = Some(value.to_owned()),
            "prunable" => item.prunable = Some(value.to_owned()),
            _ => {}
        }
    }
    if let Some(done) = current.take() {
        out.push(done);
    }
    if let Some(first) = out.first_mut() {
        first.is_main = true;
    }
    out
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

// ---------------------------------------------------------------------------------------------
// add

#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    /// Absolute target directory (see [`plan_path`]).
    pub path: PathBuf,
    pub branch: String,
    /// Start point for a new branch (a branch, tag or commit). Ignored when
    /// `checkout_existing` is set. `None` starts at the current HEAD.
    pub base: Option<String>,
    /// Check out `branch` as it exists instead of creating it.
    pub checkout_existing: bool,
    pub no_checkout: bool,
    /// Set `push.autoSetupRemote=true` in the shared repo config unless already configured.
    /// Off by default: it changes configuration the user owns.
    pub auto_setup_remote: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Added {
    pub path: PathBuf,
    pub branch: String,
    /// Base recorded as lineage, when a new branch was created from one.
    pub base: Option<String>,
}

/// Create a worktree. Serialized per repository inside this process; a lock held by another
/// process surfaces as [`WorktreeError::Locked`] (retryable).
pub fn add(root: &Path, options: &AddOptions) -> Result<Added> {
    with_repo_queue(root, || add_unserialized(root, options))
}

/// Like [`add`] but without the in-process queue. Concurrent callers may get a retryable
/// lock error; mostly useful for tests and callers that serialize themselves.
pub fn add_unserialized(root: &Path, options: &AddOptions) -> Result<Added> {
    let branch = options.branch.as_str();
    crate::validate_branch_name(root, branch)?;
    if !options.path.is_absolute() {
        return Err(WorktreeError::InvalidPath("path must be absolute".into()));
    }
    if contains_path(root, &options.path) {
        // Inside the repository it would show up as untracked content of the main checkout.
        return Err(WorktreeError::InvalidPath(
            "the worktree directory must be outside the repository".into(),
        ));
    }
    if let Some(base) = &options.base {
        reject_option_like(base, "base")?;
        if !options.checkout_existing {
            let spec = format!("{base}^{{commit}}");
            let resolves = git(root, ["rev-parse", "--verify", "--quiet", &spec])
                .map_err(GitError::from)?
                .status
                .success();
            if !resolves {
                return Err(WorktreeError::InvalidBase(base.clone()));
            }
        }
    }
    if let Some(parent) = options.path.parent() {
        fs::create_dir_all(parent).map_err(GitError::from)?;
    }
    let existed_before = fs::symlink_metadata(&options.path).is_ok();
    let branch_before = branch_exists(root, branch);

    let mut args: Vec<std::ffi::OsString> = vec!["worktree".into(), "add".into()];
    if options.no_checkout {
        args.push("--no-checkout".into());
    }
    if options.checkout_existing {
        args.push(options.path.clone().into_os_string());
        args.push(branch.into());
    } else {
        // No inherited upstream: the base is recorded as lineage instead.
        args.extend(["--no-track".into(), "-b".into(), branch.into()]);
        args.push(options.path.clone().into_os_string());
        if let Some(base) = &options.base {
            args.push(base.into());
        }
    }
    if let Err(error) = git_ok(root, &args) {
        cleanup_failed_add(root, &options.path, branch, branch_before, existed_before);
        return Err(error);
    }

    let mut recorded = None;
    if !options.checkout_existing
        && let Some(base) = &options.base
    {
        // Lineage failures must not fail the creation; the config is cleared instead.
        if set_branch_base(&options.path, branch, base).is_ok() {
            recorded = Some(base.clone());
            track_same_name_remote(&options.path, branch, base);
        } else {
            let _ = clear_branch_base(&options.path, branch);
        }
    }
    if options.auto_setup_remote {
        configure_auto_setup_remote(&options.path);
    }
    Ok(Added {
        path: options.path.clone(),
        branch: branch.to_owned(),
        base: recorded,
    })
}

pub fn branch_exists(root: &Path, branch: &str) -> bool {
    let reference = format!("refs/heads/{branch}");
    git(root, ["show-ref", "--verify", "--quiet", &reference])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// After a failed add: drop what git left behind, but only what this call created.
fn cleanup_failed_add(
    root: &Path,
    path: &Path,
    branch: &str,
    branch_before: bool,
    existed_before: bool,
) {
    if !existed_before && orphan_proof(path, root) {
        let _ = fs::remove_dir_all(path);
    }
    let _ = git(root, ["worktree", "prune"]);
    if !branch_before && branch_exists(root, branch) {
        let in_use = list(root)
            .map(|all| all.iter().any(|w| w.branch.as_deref() == Some(branch)))
            .unwrap_or(true);
        if !in_use {
            let _ = git(root, ["branch", "-D", branch]);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// lineage and config

fn base_key(branch: &str) -> String {
    format!("branch.{branch}.base")
}

/// Record the branch this one was created from (`branch.<name>.base`, local config).
pub fn set_branch_base(worktree: &Path, branch: &str, base: &str) -> Result<()> {
    reject_option_like(base, "base")?;
    git_ok(
        worktree,
        [
            "config",
            "--local",
            "--replace-all",
            &base_key(branch),
            base,
        ],
    )?;
    Ok(())
}

pub fn branch_base(root: &Path, branch: &str) -> Option<String> {
    let out = git(root, ["config", "--local", "--get", &base_key(branch)]).ok()?;
    out.status
        .success()
        .then(|| stdout_text(&out))
        .filter(|s| !s.is_empty())
}

pub fn clear_branch_base(root: &Path, branch: &str) -> Result<()> {
    // Exit 5 means the key was not set; that is already the desired state.
    let output = git(
        root,
        ["config", "--local", "--unset-all", &base_key(branch)],
    )
    .map_err(GitError::from)?;
    if output.status.success() || output.status.code() == Some(5) {
        Ok(())
    } else {
        Err(classify(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

/// Set the upstream when the base is the same-name remote-tracking branch.
fn track_same_name_remote(worktree: &Path, branch: &str, base: &str) {
    let Some(rest) = base.strip_prefix("refs/remotes/") else {
        return;
    };
    let Ok(remotes) = git_ok(worktree, ["remote"]) else {
        return;
    };
    let text = String::from_utf8_lossy(&remotes.stdout).into_owned();
    // Remote names may contain slashes: take the longest remote that prefixes the ref.
    let name = text
        .lines()
        .filter(|r| rest.strip_prefix(*r).is_some_and(|t| t.starts_with('/')))
        .max_by_key(|r| r.len());
    if let Some(remote) = name
        && &rest[remote.len() + 1..] == branch
    {
        let _ = git(worktree, ["branch", "--set-upstream-to", base, branch]);
    }
}

fn configure_auto_setup_remote(worktree: &Path) {
    // Honor explicit config at any scope; exit 1 means unset, other codes are real failures.
    match git(worktree, ["config", "--get", "push.autoSetupRemote"]) {
        Ok(out) if out.status.code() == Some(1) => {
            let _ = git(
                worktree,
                ["config", "--local", "push.autoSetupRemote", "true"],
            );
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------
// remove, prune, lock

#[derive(Debug, Clone, Default)]
pub struct RemoveOptions {
    /// Remove even with uncommitted changes (the dangerous-path rules still apply).
    pub force: bool,
    /// Shared-directory links created by [`link_shared_directories`]: tolerated by the dirty
    /// check and unlinked (never followed) before removal.
    pub shared_links: Vec<String>,
    /// Home directory for the dangerous-path guard; defaults to the environment.
    pub home: Option<PathBuf>,
}

/// Remove a registered linked worktree after the safety checks: refuses the main worktree,
/// dangerous paths, nested worktrees, locked worktrees, and (unless forced) dirty ones.
pub fn remove(root: &Path, path: &Path, options: &RemoveOptions) -> Result<()> {
    with_repo_queue(root, || {
        let all = list(root)?;
        let target = all
            .iter()
            .find(|w| same_path(&w.path, path))
            .ok_or_else(|| WorktreeError::Unregistered(path.to_owned()))?;
        let home = options.home.clone().or_else(home_dir);
        let main = all
            .first()
            .map(|w| w.path.clone())
            .unwrap_or_else(|| root.to_owned());
        if target.is_main || is_dangerous_path(&target.path, &main, home.as_deref()) {
            return Err(WorktreeError::Protected(target.path.clone()));
        }
        if all
            .iter()
            .any(|w| !same_path(&w.path, &target.path) && contains_path(&target.path, &w.path))
        {
            return Err(WorktreeError::Nested(target.path.clone()));
        }
        if target.locked.is_some() {
            return Err(WorktreeError::LockedWorktree(target.path.clone()));
        }
        if target.prunable.is_some() || !target.path.exists() {
            // Directory already gone: only the registration is left.
            git_ok(root, ["worktree", "prune"])?;
            return Ok(());
        }
        preflight_clean(&target.path, options.force, &options.shared_links)?;
        for link in &options.shared_links {
            if is_safe_relative(link) {
                let link_path = target.path.join(link);
                if fs::symlink_metadata(&link_path).is_ok_and(|m| m.file_type().is_symlink()) {
                    unlink(&link_path);
                }
            }
        }
        let mut args: Vec<std::ffi::OsString> = vec!["worktree".into(), "remove".into()];
        if options.force {
            args.push("--force".into());
        }
        args.push(target.path.clone().into_os_string());
        git_ok(root, &args)?;
        Ok(())
    })
}

fn unlink(path: &Path) {
    // A directory symlink on Windows needs remove_dir; remove_file covers the rest.
    if fs::remove_file(path).is_err() {
        let _ = fs::remove_dir(path);
    }
}

/// Delete a branch only when git agrees it is fully merged (`branch -d`): branches with
/// possibly unmerged commits are kept. Returns whether the branch was deleted.
pub fn delete_branch_if_merged(root: &Path, branch: &str) -> Result<bool> {
    crate::validate_branch_name(root, branch)?;
    let output = git(root, ["branch", "-d", branch]).map_err(GitError::from)?;
    if output.status.success() {
        let _ = clear_branch_base(root, branch);
        return Ok(true);
    }
    Ok(false)
}

/// Fail when the worktree has uncommitted or untracked changes. `shared_links` are untracked
/// shared-directory links the caller tolerates. A forced check always passes.
pub fn preflight_clean(path: &Path, force: bool, shared_links: &[String]) -> Result<()> {
    if force {
        return Ok(());
    }
    let output = git_ok(
        path,
        [
            "status",
            "--porcelain",
            "-z",
            "--untracked-files=all",
            // User config must not hide edits from the dirty check.
            "--ignore-submodules=none",
        ],
    )?;
    let tolerated: HashSet<String> = shared_links
        .iter()
        .map(|s| s.trim().trim_start_matches(['/', '\\']).replace('\\', "/"))
        .filter(|s| !s.is_empty() && !s.split('/').any(|seg| seg == ".."))
        .collect();
    let blocking: Vec<String> = output
        .stdout
        .split(|b| *b == 0)
        .filter(|r| !r.is_empty())
        .map(|r| String::from_utf8_lossy(r).into_owned())
        .filter(|entry| {
            let Some(p) = entry.strip_prefix("?? ") else {
                return true;
            };
            let p = p.trim_end_matches('/');
            !tolerated.contains(p)
        })
        .collect();
    if blocking.is_empty() {
        return Ok(());
    }
    let status = blocking.join("\n");
    if has_populated_submodule(path) {
        return Err(WorktreeError::SubmoduleDirty(status));
    }
    Err(WorktreeError::Dirty(status))
}

/// Git's own submodule refusal test: a `.gitmodules` path whose directory holds a `.git`.
fn has_populated_submodule(worktree: &Path) -> bool {
    let Ok(out) = git(
        worktree,
        [
            "config",
            "-z",
            "--file",
            ".gitmodules",
            "--get-regexp",
            "^submodule\\..*\\.path$",
        ],
    ) else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    out.stdout
        .split(|b| *b == 0)
        .filter_map(|entry| {
            let text = String::from_utf8_lossy(entry).into_owned();
            text.split_once('\n').map(|(_, p)| p.to_owned())
        })
        .filter(|p| is_safe_relative(p))
        .any(|p| worktree.join(p).join(".git").exists())
}

pub fn prune(root: &Path) -> Result<()> {
    with_repo_queue(root, || {
        git_ok(root, ["worktree", "prune"])?;
        Ok(())
    })
}

pub fn lock(root: &Path, path: &Path, reason: Option<&str>) -> Result<()> {
    let mut args: Vec<std::ffi::OsString> = vec!["worktree".into(), "lock".into()];
    if let Some(reason) = reason {
        args.push("--reason".into());
        args.push(reason.into());
    }
    args.push(path.as_os_str().to_owned());
    git_ok(root, &args)?;
    Ok(())
}

pub fn unlock(root: &Path, path: &Path) -> Result<()> {
    git_ok(
        root,
        [
            std::ffi::OsString::from("worktree"),
            "unlock".into(),
            path.as_os_str().to_owned(),
        ],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// merge base and diff stat

/// Best common ancestor of two revisions, `None` when they share no history.
pub fn merge_base(root: &Path, a: &str, b: &str) -> Result<Option<String>> {
    reject_option_like(a, "revision")?;
    reject_option_like(b, "revision")?;
    let out = git(root, ["merge-base", a, b]).map_err(GitError::from)?;
    if out.status.success() {
        return Ok(Some(stdout_text(&out)));
    }
    if out.status.code() == Some(1) {
        return Ok(None);
    }
    Err(classify(
        String::from_utf8_lossy(&out.stderr).trim().to_owned(),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStat {
    pub path: PathBuf,
    /// `None` for binary files.
    pub added: Option<u32>,
    pub deleted: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffStat {
    pub files: Vec<FileStat>,
    pub added: u32,
    pub deleted: u32,
}

/// Changes of a worktree against the merge base with `base` (committed and uncommitted
/// changes of tracked files; renames are reported as delete plus add).
pub fn diff_stat(worktree: &Path, base: &str) -> Result<DiffStat> {
    let anchor = merge_base(worktree, base, "HEAD")?.ok_or_else(|| {
        WorktreeError::Git(GitError::Command(format!(
            "no common history with `{base}`"
        )))
    })?;
    let out = git_ok(
        worktree,
        ["diff", "--numstat", "-z", "--no-renames", &anchor, "--"],
    )?;
    let mut stat = DiffStat::default();
    for record in out.stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let text = String::from_utf8_lossy(record);
        let mut parts = text.splitn(3, '\t');
        let (Some(a), Some(d), Some(p)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let added = a.parse::<u32>().ok();
        let deleted = d.parse::<u32>().ok();
        stat.added += added.unwrap_or(0);
        stat.deleted += deleted.unwrap_or(0);
        stat.files.push(FileStat {
            path: PathBuf::from(p),
            added,
            deleted,
        });
    }
    Ok(stat)
}

// ---------------------------------------------------------------------------------------------
// orphans

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Orphan {
    /// Registered with git but the directory is gone (`git worktree prune` clears it).
    MissingDirectory(PathBuf),
    /// A directory under the managed base whose `.git` file proves it was a worktree of this
    /// repository, but git no longer lists it.
    Unregistered(PathBuf),
}

/// Find orphans of this repository. `managed_dirs` are the directories to scan for
/// unregistered leftovers (usually [`managed_base`]); only direct children are examined.
pub fn detect_orphans(root: &Path, managed_dirs: &[PathBuf]) -> Result<Vec<Orphan>> {
    let all = list(root)?;
    let mut found = Vec::new();
    for w in all.iter().filter(|w| !w.is_main) {
        if w.prunable.is_some() || !w.path.exists() {
            found.push(Orphan::MissingDirectory(w.path.clone()));
        }
    }
    for dir in managed_dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        let mut names: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        names.sort();
        for candidate in names {
            let is_dir = fs::symlink_metadata(&candidate).is_ok_and(|m| m.is_dir());
            if is_dir
                && !all.iter().any(|w| same_path(&w.path, &candidate))
                && orphan_proof(&candidate, root)
            {
                found.push(Orphan::Unregistered(candidate));
            }
        }
    }
    Ok(found)
}

/// Delete an unregistered leftover directory, only when its `.git` file provably points at an
/// administrative entry of this repository that names the directory back (or is gone).
pub fn remove_orphan_dir(root: &Path, path: &Path, home: Option<&Path>) -> Result<()> {
    let home = home.map(Path::to_owned).or_else(home_dir);
    let main = list(root)?
        .first()
        .map(|w| w.path.clone())
        .unwrap_or_else(|| root.to_owned());
    if is_dangerous_path(path, &main, home.as_deref()) {
        return Err(WorktreeError::Protected(path.to_owned()));
    }
    if list(root)?.iter().any(|w| same_path(&w.path, path)) || !orphan_proof(path, root) {
        return Err(WorktreeError::NotOrphan(path.to_owned()));
    }
    fs::remove_dir_all(path).map_err(GitError::from)?;
    Ok(())
}

fn first_line(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let line = text.lines().next()?.trim().to_owned();
    (!line.is_empty()).then_some(line)
}

fn resolve_against(raw: &str, base: &Path) -> PathBuf {
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        normalize(&p)
    } else {
        normalize(&base.join(p))
    }
}

/// True when `<dir>/.git` is a file pointing into `<common>/worktrees/<name>` of this repository
/// and that entry names `<dir>/.git` back, or the entry is gone.
fn orphan_proof(dir: &Path, root: &Path) -> bool {
    let git_file = dir.join(".git");
    if !fs::symlink_metadata(&git_file).is_ok_and(|m| m.is_file()) {
        return false;
    }
    let Some(line) = first_line(&git_file) else {
        return false;
    };
    let Some(target) = line
        .strip_prefix("gitdir:")
        .or_else(|| line.strip_prefix("GITDIR:"))
        .map(str::trim)
    else {
        return false;
    };
    if target.is_empty() {
        return false;
    }
    let Ok(common) = common_dir(root) else {
        return false;
    };
    let admin = resolve_against(target, dir);
    let worktrees = common.join("worktrees");
    let admin_resolved = resolve(&admin);
    // Exactly one component below <common>/worktrees.
    let Ok(rel) = admin_resolved.strip_prefix(resolve(&worktrees)) else {
        return false;
    };
    if rel.components().count() != 1 {
        return false;
    }
    match first_line(&admin.join("gitdir")) {
        Some(back) => same_path(&resolve_against(&back, &admin), &dir.join(".git")),
        None => fs::symlink_metadata(&admin).is_err(),
    }
}

// ---------------------------------------------------------------------------------------------
// include and shared helpers

fn is_safe_relative(rel: &str) -> bool {
    if rel.is_empty() || Path::new(rel).is_absolute() || rel.contains('\0') {
        return false;
    }
    let mut segments = rel.split('/');
    let first = segments.next().unwrap_or("");
    if first == ".git" || first.is_empty() {
        return false;
    }
    !rel.split('/').any(|s| s.is_empty() || s == "..")
        && !rel.contains(':')
        && !rel.starts_with('\\')
}

fn is_literal(entry: &str) -> bool {
    !(entry.starts_with('!') || entry.contains('*') || entry.contains('?') || entry.contains('['))
}

/// Parse the include file: one literal path per line, `#` comments, anchored at the repo root.
pub fn parse_include_file(content: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut normalized = line.replace('\\', "/");
        if let Some(rest) = normalized.strip_prefix("./") {
            normalized = rest.to_owned();
        }
        let normalized = normalized.trim_end_matches('/').to_owned();
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        out.push(normalized);
    }
    out
}

/// Which of `paths` git ignores (relative to `root`). Tracked files never count as ignored.
pub fn check_ignored(root: &Path, paths: &[String]) -> Result<HashSet<String>> {
    if paths.is_empty() {
        return Ok(HashSet::new());
    }
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["check-ignore", "-z", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(GitError::from)?;
    let mut stdin = child.stdin.take();
    let mut input = Vec::new();
    for p in paths {
        input.extend_from_slice(p.as_bytes());
        input.push(0);
    }
    let writer = std::thread::spawn(move || {
        if let Some(mut s) = stdin.take() {
            let _ = s.write_all(&input);
        }
    });
    let output = child.wait_with_output().map_err(GitError::from)?;
    let _ = writer.join();
    // Exit 1 means nothing was ignored.
    if !output.status.success() && output.status.code() != Some(1) {
        return Err(classify(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(output
        .stdout
        .split(|b| *b == 0)
        .filter(|r| !r.is_empty())
        .map(|r| String::from_utf8_lossy(r).trim_end_matches('/').to_owned())
        .collect())
}

/// Existing, ignored, literal and safe entries of the include file. Missing or oversized file,
/// and any failure, yield an empty list: inclusion must never block creating a worktree.
pub fn resolve_include_paths(root: &Path) -> Vec<String> {
    let file = root.join(INCLUDE_FILE);
    let Ok(meta) = fs::symlink_metadata(&file) else {
        return Vec::new();
    };
    if !meta.is_file() || meta.len() > INCLUDE_MAX_FILE_BYTES {
        return Vec::new();
    }
    let Ok(content) = fs::read_to_string(&file) else {
        return Vec::new();
    };
    let candidates: Vec<String> = parse_include_file(&content)
        .into_iter()
        .filter(|e| is_literal(e) && is_safe_relative(e))
        .take(INCLUDE_MAX_ENTRIES)
        .filter(|e| fs::symlink_metadata(root.join(e)).is_ok())
        .collect();
    let Ok(ignored) = check_ignored(root, &candidates) else {
        return Vec::new();
    };
    let mut out: Vec<String> = candidates
        .into_iter()
        .filter(|c| ignored.contains(c))
        .collect();
    out.sort();
    out
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetupReport {
    pub done: Vec<String>,
    /// Entries that were left alone, with the reason.
    pub skipped: Vec<(String, String)>,
}

/// True when any existing component of `base/rel` (after `base`) is a symlink.
fn crosses_symlink(base: &Path, rel: &str) -> bool {
    let mut cursor = base.to_owned();
    for seg in rel.split('/') {
        cursor.push(seg);
        match fs::symlink_metadata(&cursor) {
            Ok(m) if m.file_type().is_symlink() => return true,
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    false
}

/// Copy the listed paths from the repo into the worktree, literally: symlinks (as the entry or
/// anywhere inside it) are skipped rather than followed, existing destination files are never
/// overwritten, and nothing leaves the two roots.
pub fn copy_include_paths(root: &Path, worktree: &Path, paths: &[String]) -> SetupReport {
    let mut report = SetupReport::default();
    let mut budget = COPY_MAX_FILES;
    for rel in paths {
        if !is_safe_relative(rel) || !is_literal(rel) {
            report.skipped.push((rel.clone(), "unsafe path".into()));
            continue;
        }
        if crosses_symlink(root, rel) || crosses_symlink(worktree, rel) {
            report.skipped.push((rel.clone(), "symbolic link".into()));
            continue;
        }
        let src = root.join(rel);
        let dst = worktree.join(rel);
        match copy_tree(&src, &dst, &mut budget) {
            Ok(()) => report.done.push(rel.clone()),
            Err(reason) => report.skipped.push((rel.clone(), reason)),
        }
    }
    report
}

fn copy_tree(src: &Path, dst: &Path, budget: &mut usize) -> std::result::Result<(), String> {
    let meta = fs::symlink_metadata(src).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Ok(()); // never follow
    }
    if meta.is_dir() {
        fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            copy_tree(&entry.path(), &dst.join(entry.file_name()), budget)?;
        }
        return Ok(());
    }
    if !meta.is_file() {
        return Ok(()); // sockets, fifos, devices
    }
    if *budget == 0 {
        return Err("too many files".into());
    }
    *budget -= 1;
    if fs::symlink_metadata(dst).is_ok() {
        return Ok(()); // never overwrite
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(src, dst).map_err(|e| e.to_string())?;
    Ok(())
}

/// Entries of `configured` that exist as real (non-symlink) directories in the repo and are
/// ignored by git; sorted. Others are dropped. Files, unsafe names and globs never qualify.
pub fn resolve_shared_directories(root: &Path, configured: &[String]) -> Vec<String> {
    let existing: Vec<String> = configured
        .iter()
        .map(|c| c.trim().trim_end_matches('/').replace('\\', "/"))
        .filter(|c| is_literal(c) && is_safe_relative(c))
        .filter(|c| {
            !crosses_symlink(root, c)
                && fs::symlink_metadata(root.join(c)).is_ok_and(|m| m.is_dir())
        })
        .collect();
    let Ok(ignored) = check_ignored(root, &existing) else {
        return Vec::new();
    };
    let mut out: Vec<String> = existing
        .into_iter()
        .filter(|c| ignored.contains(c))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Symlink each shared directory of the repo into the worktree. Existing destinations (files,
/// directories or links) are left alone, and destinations below a symlink are refused.
pub fn link_shared_directories(root: &Path, worktree: &Path, dirs: &[String]) -> SetupReport {
    let mut report = SetupReport::default();
    for rel in dirs {
        if !is_safe_relative(rel) {
            report.skipped.push((rel.clone(), "unsafe path".into()));
            continue;
        }
        let src = root.join(rel);
        let dst = worktree.join(rel);
        if crosses_symlink(root, rel) || !fs::symlink_metadata(&src).is_ok_and(|m| m.is_dir()) {
            report
                .skipped
                .push((rel.clone(), "not a real directory".into()));
            continue;
        }
        if fs::symlink_metadata(&dst).is_ok() {
            report.skipped.push((rel.clone(), "already exists".into()));
            continue;
        }
        if let Some(parent) = dst.parent()
            && fs::create_dir_all(parent).is_err()
        {
            report
                .skipped
                .push((rel.clone(), "cannot create parent".into()));
            continue;
        }
        if crosses_symlink(worktree, rel) {
            report.skipped.push((rel.clone(), "symbolic link".into()));
            continue;
        }
        match symlink_dir(&resolve(&src), &dst) {
            Ok(()) => report.done.push(rel.clone()),
            Err(e) => report.skipped.push((rel.clone(), e.to_string())),
        }
    }
    report
}

#[cfg(unix)]
fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn symlink_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn rev(root: &Path, spec: &str) -> Option<String> {
        let out = git(root, ["rev-parse", "--verify", "--quiet", spec]).ok()?;
        out.status.success().then(|| stdout_text(&out))
    }

    struct Sandbox {
        dir: PathBuf,
        repo: PathBuf,
        data: PathBuf,
        home: PathBuf,
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn g(cwd: &Path, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn sandbox(name: &str) -> Sandbox {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "archipel-ide-worktrees-{name}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        let dir = {
            fs::create_dir_all(&dir).unwrap();
            fs::canonicalize(&dir).unwrap()
        };
        let repo = dir.join("project");
        let data = dir.join("data");
        let home = dir.join("home");
        for d in [&repo, &data, &home] {
            fs::create_dir_all(d).unwrap();
        }
        g(&repo, &["init", "-q", "-b", "main"]);
        g(&repo, &["config", "user.email", "test@example.invalid"]);
        g(&repo, &["config", "user.name", "Archipel Test"]);
        fs::write(repo.join("a.txt"), "one\n").unwrap();
        fs::write(repo.join(".gitignore"), "node_modules/\ntarget/\n.env\n").unwrap();
        g(&repo, &["add", "."]);
        g(&repo, &["commit", "-qm", "initial"]);
        Sandbox {
            dir,
            repo,
            data,
            home,
        }
    }

    fn add_new(sb: &Sandbox, slug: &str) -> Added {
        let path = plan_path(&sb.data, &sb.repo, slug).unwrap();
        add(
            &sb.repo,
            &AddOptions {
                path,
                branch: sanitize_name(slug).unwrap(),
                base: Some("main".into()),
                ..Default::default()
            },
        )
        .unwrap()
    }

    fn opts(sb: &Sandbox, force: bool) -> RemoveOptions {
        RemoveOptions {
            force,
            home: Some(sb.home.clone()),
            ..Default::default()
        }
    }

    #[test]
    fn add_list_remove_round_trip() {
        let sb = sandbox("roundtrip");
        let added = add_new(&sb, "feature one");
        assert_eq!(added.branch, "feature-one");
        assert!(added.path.join("a.txt").exists());
        assert!(contains_path(
            &managed_base(&sb.data, &sb.repo),
            &added.path
        ));
        assert!(!contains_path(&sb.repo, &added.path));

        let all = list(&sb.repo).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all[0].is_main && same_path(&all[0].path, &sb.repo));
        let linked = &all[1];
        assert_eq!(linked.branch.as_deref(), Some("feature-one"));
        assert!(!linked.is_main && linked.locked.is_none() && linked.prunable.is_none());

        remove(&sb.repo, &added.path, &opts(&sb, false)).unwrap();
        assert_eq!(list(&sb.repo).unwrap().len(), 1);
        assert!(!added.path.exists());
        // The branch is kept by remove; deleting it is a separate, merge-aware step.
        assert!(branch_exists(&sb.repo, "feature-one"));
        assert!(delete_branch_if_merged(&sb.repo, "feature-one").unwrap());
    }

    #[test]
    fn add_existing_branch_and_bad_base() {
        let sb = sandbox("existing");
        g(&sb.repo, &["branch", "topic"]);
        let path = plan_path(&sb.data, &sb.repo, "topic").unwrap();
        let added = add(
            &sb.repo,
            &AddOptions {
                path: path.clone(),
                branch: "topic".into(),
                checkout_existing: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(added.base, None);
        assert_eq!(branch_base(&sb.repo, "topic"), None);

        let bad = AddOptions {
            path: plan_path(&sb.data, &sb.repo, "x").unwrap(),
            branch: "x".into(),
            base: Some("--upload-pack=evil".into()),
            ..Default::default()
        };
        assert!(matches!(
            add(&sb.repo, &bad),
            Err(WorktreeError::InvalidBase(_))
        ));
        let missing = AddOptions {
            base: Some("no-such-ref".into()),
            ..bad
        };
        assert!(matches!(
            add(&sb.repo, &missing),
            Err(WorktreeError::InvalidBase(_))
        ));
        // Inside the repository is refused.
        let inside = AddOptions {
            path: sb.repo.join("inner"),
            branch: "inner".into(),
            ..Default::default()
        };
        assert!(matches!(
            add(&sb.repo, &inside),
            Err(WorktreeError::InvalidPath(_))
        ));
        assert!(!branch_exists(&sb.repo, "x"));
    }

    #[test]
    fn dirty_refuses_unless_forced() {
        let sb = sandbox("dirty");
        let added = add_new(&sb, "dirty");
        fs::write(added.path.join("scratch.txt"), "x").unwrap();
        let err = remove(&sb.repo, &added.path, &opts(&sb, false)).unwrap_err();
        assert!(matches!(err, WorktreeError::Dirty(ref s) if s.contains("scratch.txt")));
        assert!(added.path.exists());
        fs::write(added.path.join("a.txt"), "changed\n").unwrap();
        remove(&sb.repo, &added.path, &opts(&sb, true)).unwrap();
        assert!(!added.path.exists());
        assert_eq!(list(&sb.repo).unwrap().len(), 1);
    }

    #[test]
    fn refuses_main_unregistered_locked_and_dangerous() {
        let sb = sandbox("protect");
        let o = opts(&sb, true);
        assert!(matches!(
            remove(&sb.repo, &sb.repo, &o),
            Err(WorktreeError::Protected(_))
        ));
        assert!(matches!(
            remove(&sb.repo, &sb.dir, &o),
            Err(WorktreeError::Unregistered(_))
        ));
        let added = add_new(&sb, "locked");
        lock(&sb.repo, &added.path, Some("in use")).unwrap();
        assert_eq!(list(&sb.repo).unwrap()[1].locked.as_deref(), Some("in use"));
        assert!(matches!(
            remove(&sb.repo, &added.path, &o),
            Err(WorktreeError::LockedWorktree(_))
        ));
        unlock(&sb.repo, &added.path).unwrap();
        assert!(list(&sb.repo).unwrap()[1].locked.is_none());
        // A home directory that contains the worktree makes it dangerous.
        let in_home = RemoveOptions {
            force: true,
            home: Some(added.path.join("deeper")),
            ..Default::default()
        };
        assert!(matches!(
            remove(&sb.repo, &added.path, &in_home),
            Err(WorktreeError::Protected(_))
        ));
        remove(&sb.repo, &added.path, &o).unwrap();
    }

    #[test]
    fn dangerous_path_table() {
        let sb = sandbox("danger");
        let home = Some(sb.home.as_path());
        let r = sb.repo.as_path();
        assert!(is_dangerous_path(Path::new(""), r, home));
        assert!(is_dangerous_path(Path::new("   "), r, home));
        assert!(is_dangerous_path(Path::new("relative/dir"), r, home));
        assert!(is_dangerous_path(Path::new("/"), r, home));
        assert!(is_dangerous_path(r, r, home));
        assert!(is_dangerous_path(&sb.dir, r, home), "contains the repo");
        assert!(is_dangerous_path(&sb.home, r, home));
        assert!(is_dangerous_path(&sb.dir, &sb.dir.join("elsewhere"), home));
        assert!(is_dangerous_path(Path::new("/home"), r, home));
        assert!(is_dangerous_path(Path::new("/home/someone"), r, home));
        assert!(is_dangerous_path(Path::new("/Users/someone"), r, home));
        assert!(
            is_dangerous_path(&sb.data.join("x"), r, None),
            "unknown home"
        );
        assert!(!is_dangerous_path(&sb.data.join("x"), r, home));
        // Lexical traversal cannot sneak past the guard.
        let sneaky = sb.data.join("x").join("..").join("..").join("project");
        assert!(is_dangerous_path(&sneaky, r, home));
    }

    #[test]
    fn sanitize_table() {
        let ok = |i: &str| sanitize_name(i).unwrap();
        assert_eq!(ok("Fix login bug"), "Fix-login-bug");
        assert_eq!(ok("  spaced   out  "), "spaced-out");
        assert_eq!(ok("../../etc/passwd"), "etc-passwd");
        assert_eq!(ok("a..b"), "a.b");
        assert_eq!(ok(".hidden-"), "hidden");
        assert_eq!(ok("feat/ui:thing?*"), "feat-ui-thing");
        assert_eq!(ok("Überprüfung Größe"), "Überprüfung-Größe");
        assert_eq!(ok("修复 登录"), "修复-登录");
        assert_eq!(ok("release.lock"), "release-lock");
        assert_eq!(ok("🚀"), "workspace");
        assert_eq!(ok("🚀 ship"), "ship");
        assert_eq!(ok("a@{b}"), "a-b");
        assert!(sanitize_name("").is_err());
        assert!(sanitize_name("   ").is_err());
        assert!(sanitize_name("..").is_err());
        assert!(sanitize_name("/// ...").is_err());
        assert!(sanitize_name("-").is_err());
        assert_eq!(ok(&"x".repeat(300)).chars().count(), SLUG_MAX_CHARS);
    }

    #[test]
    fn sanitized_names_are_valid_refs_and_stay_inside_base() {
        let sb = sandbox("names");
        for input in [
            "../../escape",
            "..\\..\\win",
            "über uns",
            "a b/c",
            "x.lock",
            "-rf",
            "名前",
            "$(touch pwned)",
            "~tilde^caret",
        ] {
            let slug = sanitize_name(input).unwrap();
            crate::validate_branch_name(&sb.repo, &slug)
                .unwrap_or_else(|e| panic!("{input:?} -> {slug:?}: {e}"));
            let path = plan_path(&sb.data, &sb.repo, input).unwrap();
            assert!(contains_path(&managed_base(&sb.data, &sb.repo), &path));
            assert_eq!(path.parent().unwrap(), managed_base(&sb.data, &sb.repo));
        }
        assert!(ensure_within(&sb.dir.join("project/../outside"), &sb.data).is_err());
        assert!(!sb.dir.join("pwned").exists());
    }

    #[test]
    fn plan_path_uniqueness_and_layout() {
        let sb = sandbox("layout");
        let first = add_new(&sb, "same");
        let second = plan_path(&sb.data, &sb.repo, "same").unwrap();
        assert_eq!(first.path.file_name().unwrap(), "same");
        assert_eq!(second.file_name().unwrap(), "same-2");
        let base = managed_base(&sb.data, &sb.repo);
        assert!(base.starts_with(sb.data.join("archipel").join("worktrees")));
        let dir_name = base.file_name().unwrap().to_string_lossy().into_owned();
        assert!(dir_name.starts_with("project-") && dir_name.len() == "project-".len() + 6);
        // Stable per location, different per repository.
        assert_eq!(repo_dir_name(&sb.repo), repo_dir_name(&sb.repo));
        assert_ne!(repo_dir_name(&sb.repo), repo_dir_name(&sb.dir));
        // A data dir inside the repository is refused.
        assert!(plan_path(&sb.repo.join("data"), &sb.repo, "x").is_err());
    }

    #[test]
    fn lineage_config() {
        let sb = sandbox("lineage");
        let added = add_new(&sb, "lin");
        assert_eq!(added.base.as_deref(), Some("main"));
        assert_eq!(branch_base(&sb.repo, "lin").as_deref(), Some("main"));
        // Readable and writable from the linked worktree too (shared config).
        set_branch_base(&added.path, "lin", "develop").unwrap();
        assert_eq!(branch_base(&added.path, "lin").as_deref(), Some("develop"));
        assert!(set_branch_base(&added.path, "lin", "--evil").is_err());
        clear_branch_base(&sb.repo, "lin").unwrap();
        clear_branch_base(&sb.repo, "lin").unwrap();
        assert_eq!(branch_base(&sb.repo, "lin"), None);
        // Off by default, opt-in only.
        assert!(
            Command::new("git")
                .args(["-C"])
                .arg(&sb.repo)
                .args(["config", "--local", "push.autoSetupRemote"])
                .output()
                .unwrap()
                .stdout
                .is_empty()
        );
        let path = plan_path(&sb.data, &sb.repo, "auto").unwrap();
        add(
            &sb.repo,
            &AddOptions {
                path,
                branch: "auto".into(),
                base: Some("main".into()),
                auto_setup_remote: true,
                ..Default::default()
            },
        )
        .unwrap();
        let out = Command::new("git")
            .args(["-C"])
            .arg(&sb.repo)
            .args(["config", "--local", "push.autoSetupRemote"])
            .output()
            .unwrap();
        assert_eq!(stdout_text(&out), "true");
    }

    #[test]
    fn merge_base_and_diff_stat() {
        let sb = sandbox("diffstat");
        let added = add_new(&sb, "work");
        fs::write(added.path.join("b.txt"), "x\ny\n").unwrap();
        g(&added.path, &["add", "b.txt"]);
        g(&added.path, &["commit", "-qm", "work"]);
        fs::write(added.path.join("a.txt"), "one\ntwo\n").unwrap(); // uncommitted
        // main moves on; the merge base stays at the fork point.
        fs::write(sb.repo.join("c.txt"), "c\n").unwrap();
        g(&sb.repo, &["add", "c.txt"]);
        g(&sb.repo, &["commit", "-qm", "main moves"]);
        let fork = rev(&sb.repo, "main~1").unwrap();
        assert_eq!(
            merge_base(&added.path, "main", "HEAD").unwrap().unwrap(),
            fork
        );
        let stat = diff_stat(&added.path, "main").unwrap();
        assert_eq!(stat.files.len(), 2);
        assert_eq!((stat.added, stat.deleted), (3, 0));
        let b = stat
            .files
            .iter()
            .find(|f| f.path == Path::new("b.txt"))
            .unwrap();
        assert_eq!(b.added, Some(2));
        assert!(merge_base(&sb.repo, "--bad", "main").is_err());
    }

    #[test]
    fn include_files_copy_literally_and_safely() {
        let sb = sandbox("include");
        fs::write(sb.repo.join(".env"), "SECRET=1\n").unwrap();
        fs::create_dir_all(sb.repo.join("target/debug")).unwrap();
        fs::write(sb.repo.join("target/debug/out.bin"), "bin").unwrap();
        fs::write(sb.repo.join("notignored.txt"), "tracked-ish").unwrap();
        let outside = sb.dir.join("outside-secret");
        fs::write(&outside, "top secret").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, sb.repo.join("target/debug/link")).unwrap();
        fs::write(
            sb.repo.join(INCLUDE_FILE),
            "# comment\n.env\n./target/\n*.txt\n!.env\n../outside-secret\n.git/config\n/abs\nmissing.cfg\nnotignored.txt\n.env\n",
        )
        .unwrap();
        assert_eq!(
            parse_include_file("a\n\n# c\n./b/\nb\n"),
            vec!["a".to_owned(), "b".to_owned()]
        );
        let paths = resolve_include_paths(&sb.repo);
        assert_eq!(paths, vec![".env".to_owned(), "target".to_owned()]);

        let added = add_new(&sb, "inc");
        let report = copy_include_paths(&sb.repo, &added.path, &paths);
        assert_eq!(report.done.len(), 2, "{report:?}");
        assert_eq!(
            fs::read_to_string(added.path.join(".env")).unwrap(),
            "SECRET=1\n"
        );
        assert!(added.path.join("target/debug/out.bin").exists());
        assert!(
            fs::symlink_metadata(added.path.join("target/debug/link")).is_err(),
            "symlinks inside entries are not followed or copied"
        );
        // Existing files are not overwritten.
        fs::write(added.path.join(".env"), "MINE\n").unwrap();
        copy_include_paths(&sb.repo, &added.path, &paths);
        assert_eq!(
            fs::read_to_string(added.path.join(".env")).unwrap(),
            "MINE\n"
        );
        // Hostile entries handed to the copier directly are refused.
        let bad = copy_include_paths(
            &sb.repo,
            &added.path,
            &[
                "../outside-secret".into(),
                ".git/config".into(),
                "*.txt".into(),
            ],
        );
        assert!(bad.done.is_empty() && bad.skipped.len() == 3);
        assert!(!added.path.join("../outside-secret").exists());
        // Oversized include file is ignored.
        fs::write(
            sb.repo.join(INCLUDE_FILE),
            "x".repeat(INCLUDE_MAX_FILE_BYTES as usize + 1),
        )
        .unwrap();
        assert!(resolve_include_paths(&sb.repo).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn include_refuses_symlinked_parent() {
        let sb = sandbox("include-link");
        let outside = sb.dir.join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("key"), "k").unwrap();
        std::os::unix::fs::symlink(&outside, sb.repo.join("target")).unwrap();
        let added = add_new(&sb, "inc-link");
        let report = copy_include_paths(&sb.repo, &added.path, &["target/key".into()]);
        assert!(report.done.is_empty());
        // And a destination reached through a symlink in the worktree is refused.
        std::os::unix::fs::symlink(&outside, added.path.join("escape")).unwrap();
        fs::create_dir_all(sb.repo.join("escape")).unwrap();
        fs::write(sb.repo.join("escape/.env"), "x").unwrap();
        let report = copy_include_paths(&sb.repo, &added.path, &["escape/.env".into()]);
        assert!(report.done.is_empty());
        assert!(!outside.join(".env").exists());
    }

    #[cfg(unix)]
    #[test]
    fn shared_directories_link_only_ignored_real_dirs() {
        let sb = sandbox("shared");
        fs::create_dir_all(sb.repo.join("node_modules/pkg")).unwrap();
        fs::write(sb.repo.join("node_modules/pkg/i.js"), "1").unwrap();
        fs::create_dir_all(sb.repo.join("src")).unwrap(); // not ignored
        fs::write(sb.repo.join("src/m.rs"), "").unwrap();
        fs::write(sb.repo.join(".env"), "").unwrap(); // ignored, but a file
        let configured: Vec<String> = [
            "node_modules/",
            "src",
            ".env",
            "missing",
            "../x",
            "*",
            "target",
        ]
        .map(String::from)
        .to_vec();
        fs::create_dir_all(sb.repo.join("target")).unwrap();
        let shared = resolve_shared_directories(&sb.repo, &configured);
        assert_eq!(shared, vec!["node_modules".to_owned(), "target".to_owned()]);

        let added = add_new(&sb, "share");
        let report = link_shared_directories(&sb.repo, &added.path, &shared);
        assert_eq!(report.done.len(), 2, "{report:?}");
        let link = added.path.join("node_modules");
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(link.join("pkg/i.js").exists());
        // Second run leaves existing destinations alone.
        let again = link_shared_directories(&sb.repo, &added.path, &shared);
        assert!(again.done.is_empty() && again.skipped.len() == 2);
        // `node_modules/` does not match a symlink, so git sees it as untracked: the dirty check
        // must tolerate the configured links, and still catch real changes.
        assert!(preflight_clean(&added.path, false, &[]).is_err());
        preflight_clean(&added.path, false, &shared).unwrap();
        fs::write(added.path.join("junk"), "x").unwrap();
        assert!(preflight_clean(&added.path, false, &shared).is_err());
        fs::remove_file(added.path.join("junk")).unwrap();

        // Removal unlinks the links and never touches the shared targets.
        let o = RemoveOptions {
            shared_links: shared.clone(),
            ..opts(&sb, false)
        };
        remove(&sb.repo, &added.path, &o).unwrap();
        assert!(!added.path.exists());
        assert!(sb.repo.join("node_modules/pkg/i.js").exists());
    }

    #[test]
    fn orphans_missing_and_unregistered() {
        let sb = sandbox("orphan");
        let gone = add_new(&sb, "gone");
        let leftover = add_new(&sb, "leftover");
        let healthy = add_new(&sb, "healthy");
        fs::remove_dir_all(&gone.path).unwrap();
        // Unregistered: administrative entry removed, directory remains.
        let admin = common_dir(&sb.repo).unwrap().join("worktrees");
        let mut removed = 0;
        for e in fs::read_dir(&admin).unwrap().flatten() {
            let back = fs::read_to_string(e.path().join("gitdir")).unwrap_or_default();
            if back.contains("leftover") {
                fs::remove_dir_all(e.path()).unwrap();
                removed += 1;
            }
        }
        assert_eq!(removed, 1);
        // A stranger directory with a copied .git file naming another checkout is not provable.
        let stranger = managed_base(&sb.data, &sb.repo).join("stranger");
        fs::create_dir_all(&stranger).unwrap();
        fs::copy(healthy.path.join(".git"), stranger.join(".git")).unwrap();
        let plain = managed_base(&sb.data, &sb.repo).join("plain");
        fs::create_dir_all(&plain).unwrap();

        let base = managed_base(&sb.data, &sb.repo);
        let orphans = detect_orphans(&sb.repo, std::slice::from_ref(&base)).unwrap();
        assert!(
            orphans.contains(&Orphan::MissingDirectory(gone.path.clone())),
            "{orphans:?}"
        );
        assert!(
            orphans.contains(&Orphan::Unregistered(leftover.path.clone())),
            "{orphans:?}"
        );
        assert_eq!(orphans.len(), 2, "{orphans:?}");

        assert!(matches!(
            remove_orphan_dir(&sb.repo, &stranger, Some(&sb.home)),
            Err(WorktreeError::NotOrphan(_))
        ));
        assert!(matches!(
            remove_orphan_dir(&sb.repo, &healthy.path, Some(&sb.home)),
            Err(WorktreeError::NotOrphan(_))
        ));
        assert!(matches!(
            remove_orphan_dir(&sb.repo, &sb.repo, Some(&sb.home)),
            Err(WorktreeError::Protected(_))
        ));
        remove_orphan_dir(&sb.repo, &leftover.path, Some(&sb.home)).unwrap();
        assert!(!leftover.path.exists());
        prune(&sb.repo).unwrap();
        let listed = list(&sb.repo).unwrap();
        assert!(listed.iter().all(|w| w.prunable.is_none()));
        assert!(!listed.iter().any(|w| same_path(&w.path, &gone.path)));
        // Removing a registered-but-gone worktree just prunes it.
        let again = add_new(&sb, "again");
        fs::remove_dir_all(&again.path).unwrap();
        remove(&sb.repo, &again.path, &opts(&sb, false)).unwrap();
        assert!(
            !list(&sb.repo)
                .unwrap()
                .iter()
                .any(|w| same_path(&w.path, &again.path))
        );
    }

    #[test]
    fn lock_errors_are_retryable() {
        assert!(
            classify("fatal: Unable to create '/r/.git/index.lock': File exists.".into())
                .is_retryable()
        );
        assert!(
            classify("error: could not lock config file /r/.git/config: File exists".into())
                .is_retryable()
        );
        assert!(!classify("fatal: 'x' is already checked out".into()).is_retryable());
    }

    #[test]
    fn concurrent_adds_serialize_or_report_retryable() {
        let sb = sandbox("concurrent");
        // Serialized adds all succeed.
        let handles: Vec<_> = (0..6)
            .map(|i| {
                let repo = sb.repo.clone();
                let data = sb.data.clone();
                std::thread::spawn(move || {
                    let slug = format!("par-{i}");
                    let path = plan_path(&data, &repo, &slug).unwrap();
                    add(
                        &repo,
                        &AddOptions {
                            path,
                            branch: slug,
                            base: Some("main".into()),
                            ..Default::default()
                        },
                    )
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap().unwrap();
        }
        assert_eq!(list(&sb.repo).unwrap().len(), 7);

        // Unserialized adds either succeed or fail with a retryable lock error, and leave a
        // consistent repository; retrying the failures succeeds.
        let handles: Vec<_> = (0..6)
            .map(|i| {
                let repo = sb.repo.clone();
                let data = sb.data.clone();
                std::thread::spawn(move || {
                    let slug = format!("raw-{i}");
                    let path = plan_path(&data, &repo, &slug).unwrap();
                    let o = AddOptions {
                        path,
                        branch: slug,
                        base: Some("main".into()),
                        ..Default::default()
                    };
                    let first = add_unserialized(&repo, &o);
                    (o, first)
                })
            })
            .collect();
        for h in handles {
            let (o, first) = h.join().unwrap();
            match first {
                Ok(_) => {}
                Err(e) if e.is_retryable() => {
                    add(&sb.repo, &o).unwrap();
                }
                Err(e) => panic!("non-retryable failure: {e}"),
            }
        }
        assert_eq!(list(&sb.repo).unwrap().len(), 13);
    }

    #[test]
    fn three_simultaneous_worktrees() {
        let sb = sandbox("three");
        let a = add_new(&sb, "alpha");
        let b = add_new(&sb, "beta");
        let c = add_new(&sb, "gamma");
        let all = list(&sb.repo).unwrap();
        assert_eq!(all.len(), 4);
        for w in [&a, &b, &c] {
            assert!(all.iter().any(|i| same_path(&i.path, &w.path)));
            assert_eq!(branch_base(&sb.repo, &w.branch).as_deref(), Some("main"));
        }
        // Independent work in each.
        for (w, file) in [(&a, "a1"), (&b, "b1"), (&c, "c1")] {
            fs::write(w.path.join(file), file).unwrap();
            g(&w.path, &["add", file]);
            g(&w.path, &["commit", "-qm", file]);
            assert_eq!(diff_stat(&w.path, "main").unwrap().files.len(), 1);
        }
        lock(&sb.repo, &b.path, None).unwrap();
        assert!(matches!(
            remove(&sb.repo, &b.path, &opts(&sb, true)),
            Err(WorktreeError::LockedWorktree(_))
        ));
        remove(&sb.repo, &a.path, &opts(&sb, false)).unwrap();
        remove(&sb.repo, &c.path, &opts(&sb, false)).unwrap();
        // Unmerged work keeps its branch.
        assert!(!delete_branch_if_merged(&sb.repo, "alpha").unwrap());
        assert!(branch_exists(&sb.repo, "alpha"));
        unlock(&sb.repo, &b.path).unwrap();
        remove(&sb.repo, &b.path, &opts(&sb, false)).unwrap();
        assert_eq!(list(&sb.repo).unwrap().len(), 1);
        prune(&sb.repo).unwrap();
    }

    #[test]
    fn works_from_inside_a_linked_worktree() {
        let sb = sandbox("from-linked");
        let a = add_new(&sb, "one");
        let b = add_new(&sb, "two");
        // A linked worktree can be used as the entry point.
        let all = list(&a.path).unwrap();
        assert_eq!(all.len(), 3);
        assert!(all[0].is_main && same_path(&all[0].path, &sb.repo));
        remove(&a.path, &b.path, &opts(&sb, false)).unwrap();
        assert!(matches!(
            remove(&a.path, &sb.repo, &opts(&sb, true)),
            Err(WorktreeError::Protected(_))
        ));
    }
}
