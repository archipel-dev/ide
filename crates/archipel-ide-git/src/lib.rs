use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
use thiserror::Error;

mod blame;
mod branch;
mod diff;
mod log;
mod remote;
mod watcher;
mod worktree;
pub use blame::{BlameLine, blame};
pub use branch::{
    PendingOperation, abort_operation, checkout, cherry_pick, create_branch, delete_branch, merge,
    pending_operation, rebase, rename_branch, validate_branch_name,
};
pub use diff::{FileDiff, Hunk, HunkKind, LineRange, WhitespaceMode, split_lines};
pub use log::{
    Branches, CommitDetails, CommitFile, CommitInfo, GraphEdge, GraphRow, LocalBranch, LogQuery,
    branches, commit_details, graph, log,
};
pub use remote::{PushPlan, RemoteEvent, RemoteJob, RemoteOperation, push_plan, remotes};
pub use watcher::{Notify, StatusWatcher};
pub use worktree::{
    StashEntry, last_commit_message, stash_apply, stash_drop, stash_list, stash_push,
};

#[derive(Debug, Error)]
pub enum GitError {
    #[error("failed to execute Git: {0}")]
    Io(#[from] std::io::Error),
    #[error("Git command failed: {0}")]
    Command(String),
    #[error("path is not inside a Git repository: {0}")]
    NotRepository(PathBuf),
    #[error("{0} is a binary file and cannot be compared as text")]
    Binary(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Untracked,
    Modified,
    Added,
    Deleted,
    Renamed,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub staged: bool,
    pub unstaged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryStatus {
    pub root: PathBuf,
    pub branch: String,
    pub changes: Vec<FileChange>,
}

impl RepositoryStatus {
    pub fn load(path: &Path) -> Result<Self, GitError> {
        let root_output = run(path, &["rev-parse", "--show-toplevel"])?;
        if !root_output.status.success() {
            return Err(GitError::NotRepository(path.to_owned()));
        }
        let root = PathBuf::from(String::from_utf8_lossy(&root_output.stdout).trim());
        let branch = current_branch(&root)?;
        let output = checked(
            &root,
            &[
                // Never take index.lock in the background: it would make the
                // user's own git commands fail.
                "--no-optional-locks",
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
            ],
        )?;
        let mut changes = parse_porcelain(&output.stdout);
        changes.retain(|change| !is_ide_artefact(&root, change));
        Ok(Self {
            root,
            branch,
            changes,
        })
    }

    /// Content of repository-relative `path` at `HEAD`, or `None` when the
    /// file is not in `HEAD` (new file or no commits yet).
    pub fn head_text(&self, path: &Path) -> Result<Option<String>, GitError> {
        self.text_at("HEAD", path)
    }

    /// Content of repository-relative `path` at `revision`, or `None` when the
    /// file does not exist there.
    pub fn text_at(&self, revision: &str, path: &Path) -> Result<Option<String>, GitError> {
        if revision.is_empty() || revision.starts_with('-') || revision.contains(':') {
            return Err(GitError::Command(format!("invalid revision `{revision}`")));
        }
        let spec = format!("{revision}:{}", git_path(path));
        if !run(&self.root, &["cat-file", "-e", &spec])?
            .status
            .success()
        {
            return Ok(None);
        }
        let output = checked(&self.root, &["show", &spec])?;
        decode_text(path, output.stdout).map(Some)
    }

    /// Diff of repository-relative `path` between two revisions (`old` may be
    /// `None` for a root commit).
    pub fn diff_revisions(
        &self,
        path: &Path,
        old: Option<&str>,
        new: &str,
    ) -> Result<FileDiff, GitError> {
        let old_text = match old {
            Some(old) => self.text_at(old, path)?,
            None => None,
        };
        let new_text = self.text_at(new, path)?;
        Ok(FileDiff::compute(
            path.to_owned(),
            old.map(|old| old.chars().take(7).collect()),
            old_text.unwrap_or_default(),
            new_text.unwrap_or_default(),
        ))
    }

    /// Short hash of `HEAD`, or `None` before the first commit.
    pub fn head_revision(&self) -> Option<String> {
        let output = run(&self.root, &["rev-parse", "--short", "HEAD"]).ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    /// Diff of `HEAD` against `working` (the current content of repository-
    /// relative `path`; `None` when the file was deleted).
    pub fn diff_against_head(
        &self,
        path: &Path,
        working: Option<String>,
    ) -> Result<FileDiff, GitError> {
        let base = self.head_text(path)?;
        if base.is_none() && working.is_none() {
            return Err(GitError::Command(format!(
                "{} exists neither in HEAD nor in the working tree",
                path.display()
            )));
        }
        let revision = base.as_ref().and(self.head_revision());
        Ok(FileDiff::compute(
            path.to_owned(),
            revision,
            base.unwrap_or_default(),
            working.unwrap_or_default(),
        ))
    }

    /// Reads repository-relative `path` from the working tree as text.
    pub fn working_text(&self, path: &Path) -> Result<Option<String>, GitError> {
        match std::fs::read(self.root.join(path)) {
            Ok(bytes) => decode_text(path, bytes).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn stage(&self, path: &Path) -> Result<(), GitError> {
        checked_with_path(&self.root, &["add", "--"], path).map(|_| ())
    }

    /// Stages `content` as the new index version of repository-relative `path`
    /// (partial staging of chunks); the working tree is left alone.
    pub fn stage_text(&self, path: &Path, content: &str) -> Result<(), GitError> {
        use std::io::Write;
        let mut child = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["hash-object", "-w", "--stdin"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())?;
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(GitError::Command(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        let sha = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        // Keep the file mode of an already tracked file.
        let listing = checked_with_path(&self.root, &["ls-files", "-s", "--"], path)?;
        let mode = String::from_utf8_lossy(&listing.stdout)
            .split_whitespace()
            .next()
            .unwrap_or("100644")
            .to_owned();
        checked(
            &self.root,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("{mode},{sha},{}", git_path(path)),
            ],
        )
        .map(|_| ())
    }

    pub fn unstage(&self, path: &Path) -> Result<(), GitError> {
        checked_with_path(&self.root, &["restore", "--staged", "--"], path).map(|_| ())
    }

    /// Commits exactly `paths` (repository-relative), including new and deleted files,
    /// leaving other staged changes untouched. Returns the new short commit hash.
    pub fn commit(&self, message: &str, paths: &[PathBuf]) -> Result<String, GitError> {
        if message.trim().is_empty() {
            return Err(GitError::Command("commit message is empty".into()));
        }
        if paths.is_empty() {
            return Err(GitError::Command("no changes selected for commit".into()));
        }
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
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["commit", "--quiet", "-m", message, "--only", "--"])
            .args(paths)
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(GitError::Command(
                format!("{}{}", stderr.trim(), stdout.trim()).trim().into(),
            ));
        }
        let head = checked(&self.root, &["rev-parse", "--short", "HEAD"])?;
        Ok(String::from_utf8_lossy(&head.stdout).trim().into())
    }
}

/// Git tree paths always use `/`.
fn git_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn decode_text(path: &Path, bytes: Vec<u8>) -> Result<String, GitError> {
    if bytes.contains(&0) {
        return Err(GitError::Binary(path.to_owned()));
    }
    Ok(String::from_utf8(bytes)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned()))
}

impl RepositoryStatus {
    /// Unified diff of `paths` (repository-relative) as they would be committed:
    /// working-tree state against `HEAD`, with untracked files shown as additions.
    /// Read-only; binary and non-UTF-8 untracked files are summarized.
    pub fn diff(&self, paths: &[PathBuf]) -> Result<String, GitError> {
        let untracked: Vec<&PathBuf> = paths
            .iter()
            .filter(|path| {
                self.changes
                    .iter()
                    .any(|c| &c.path == *path && c.kind == ChangeKind::Untracked)
            })
            .collect();
        let tracked: Vec<&PathBuf> = paths.iter().filter(|p| !untracked.contains(p)).collect();
        let mut diff = String::new();
        if !tracked.is_empty() {
            let has_head = run(&self.root, &["rev-parse", "--verify", "-q", "HEAD"])?
                .status
                .success();
            // Without commits, everything staged is new.
            let base: &[&str] = if has_head {
                &["diff", "HEAD", "--no-color", "--no-ext-diff", "--"]
            } else {
                &["diff", "--cached", "--no-color", "--no-ext-diff", "--"]
            };
            let output = Command::new("git")
                .arg("-C")
                .arg(&self.root)
                .args(base)
                .args(&tracked)
                .output()?;
            if !output.status.success() {
                return Err(GitError::Command(
                    String::from_utf8_lossy(&output.stderr).trim().into(),
                ));
            }
            diff.push_str(&String::from_utf8_lossy(&output.stdout));
        }
        for path in untracked {
            let shown = path.display();
            diff.push_str(&format!(
                "diff --git a/{shown} b/{shown}\nnew file\n--- /dev/null\n+++ b/{shown}\n"
            ));
            match std::fs::read(self.root.join(path)) {
                Ok(bytes) if !bytes.contains(&0) => match String::from_utf8(bytes) {
                    Ok(text) => {
                        for line in text.lines() {
                            diff.push('+');
                            diff.push_str(line);
                            diff.push('\n');
                        }
                    }
                    Err(_) => diff.push_str("(non-UTF-8 file)\n"),
                },
                Ok(_) => diff.push_str("(binary file)\n"),
                Err(error) => diff.push_str(&format!("(unreadable: {error})\n")),
            }
        }
        Ok(diff)
    }

    /// Subjects of the last `count` commits, newest first (empty without commits).
    pub fn recent_subjects(&self, count: usize) -> Result<Vec<String>, GitError> {
        let output = run(
            &self.root,
            &[
                "log",
                &format!("-n{count}"),
                "--no-color",
                "--pretty=format:%s",
            ],
        )?;
        if !output.status.success() {
            return Ok(Vec::new());
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_owned)
            .collect())
    }
}

fn current_branch(root: &Path) -> Result<String, GitError> {
    let symbolic = run(root, &["symbolic-ref", "--short", "-q", "HEAD"])?;
    if symbolic.status.success() {
        return Ok(String::from_utf8_lossy(&symbolic.stdout).trim().into());
    }
    let detached = checked(root, &["rev-parse", "--short", "HEAD"])?;
    Ok(format!(
        "detached@{}",
        String::from_utf8_lossy(&detached.stdout).trim()
    ))
}

fn run(root: &Path, args: &[&str]) -> Result<Output, GitError> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(GitError::from)
}

fn checked(root: &Path, args: &[&str]) -> Result<Output, GitError> {
    let output = run(root, args)?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(GitError::Command(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ))
    }
}

fn checked_with_path(root: &Path, args: &[&str], path: &Path) -> Result<Output, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .arg(path)
        .output()?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(GitError::Command(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ))
    }
}

fn parse_porcelain(bytes: &[u8]) -> Vec<FileChange> {
    let records: Vec<&[u8]> = bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .collect();
    let mut changes = Vec::new();
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        if record.len() < 4 {
            index += 1;
            continue;
        }
        let x = record[0] as char;
        let y = record[1] as char;
        let path = PathBuf::from(String::from_utf8_lossy(&record[3..]).into_owned());
        let kind = if x == '?' && y == '?' {
            ChangeKind::Untracked
        } else if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
            ChangeKind::Conflicted
        } else if x == 'R' || y == 'R' {
            ChangeKind::Renamed
        } else if x == 'D' || y == 'D' {
            ChangeKind::Deleted
        } else if x == 'A' {
            ChangeKind::Added
        } else {
            ChangeKind::Modified
        };
        changes.push(FileChange {
            path,
            kind,
            staged: x != ' ' && x != '?',
            unstaged: y != ' ' || (x == '?' && y == '?'),
        });
        index += if matches!(x, 'R' | 'C') { 2 } else { 1 };
    }
    changes
}

/// Untracked files the IDE itself generates: everything under `.archipel/` plus the
/// jdtls metadata (`.classpath`, `.project`, `.settings/`, class files in `bin/`) and
/// `archipel-model.json` at the root of a Gradle/Maven project. Tracked files are
/// never filtered.
fn is_ide_artefact(root: &Path, change: &FileChange) -> bool {
    if change.kind != ChangeKind::Untracked {
        return false;
    }
    let mut parts = change.path.components();
    let Some(first) = parts.next().and_then(|c| c.as_os_str().to_str()) else {
        return false;
    };
    // `.jetthing` / `jetthing-model.json`: what versions before the rename generated.
    if first == ".archipel" || first == ".jetthing" {
        return true;
    }
    let is_class = change.path.extension().is_some_and(|ext| ext == "class");
    let nested = parts.next().is_some();
    let generated = match first {
        ".classpath" | ".project" | "archipel-model.json" | "jetthing-model.json" => !nested,
        ".settings" => nested,
        "bin" => is_class,
        _ => false,
    };
    generated
        && [
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
            "pom.xml",
        ]
        .iter()
        .any(|file| root.join(file).is_file())
}

#[cfg(test)]
mod tests {
    #[test]
    fn ide_artefacts_are_not_reported_as_unversioned() {
        let dir = std::env::temp_dir().join(format!("jt-git-artefacts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let change = |path: &str| FileChange {
            path: PathBuf::from(path),
            kind: ChangeKind::Untracked,
            staged: false,
            unstaged: true,
        };
        assert!(is_ide_artefact(&dir, &change(".archipel/model.json")));
        assert!(
            !is_ide_artefact(&dir, &change(".classpath")),
            "no build file"
        );
        std::fs::write(dir.join("build.gradle"), "").unwrap();
        for path in [
            ".classpath",
            ".project",
            ".settings/a.prefs",
            "bin/main/A.class",
        ] {
            assert!(is_ide_artefact(&dir, &change(path)), "{path}");
        }
        assert!(!is_ide_artefact(&dir, &change("bin/run.sh")));
        assert!(!is_ide_artefact(&dir, &change("src/App.java")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    use super::*;
    use std::fs;

    fn git(root: &Path, args: &[&str]) {
        assert!(run(root, args).unwrap().status.success());
    }

    #[test]
    fn diffs_selected_paths_including_untracked_files() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-git-ai-diff-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        git(&root, &["config", "user.name", "Archipel IDE Test"]);
        fs::write(root.join("a.txt"), "one\n").unwrap();
        fs::write(root.join("b.txt"), "keep\n").unwrap();
        git(&root, &["add", "."]);
        let status = RepositoryStatus::load(&root).unwrap();
        // No commits yet: staged files diff against the empty tree.
        assert!(status.diff(&["a.txt".into()]).unwrap().contains("+one"));
        assert!(status.recent_subjects(5).unwrap().is_empty());
        git(&root, &["commit", "-qm", "feat: initial"]);
        fs::write(root.join("a.txt"), "two\n").unwrap();
        fs::write(root.join("b.txt"), "changed\n").unwrap();
        fs::write(root.join("new.txt"), "fresh\n").unwrap();
        fs::write(root.join("blob.bin"), [0u8, 1, 2]).unwrap();
        let status = RepositoryStatus::load(&root).unwrap();
        let diff = status
            .diff(&["a.txt".into(), "new.txt".into(), "blob.bin".into()])
            .unwrap();
        assert!(diff.contains("-one\n+two"));
        assert!(!diff.contains("changed"), "unselected files are excluded");
        assert!(diff.contains("+++ b/new.txt\n+fresh"));
        assert!(diff.contains("(binary file)"));
        assert_eq!(status.recent_subjects(5).unwrap(), vec!["feat: initial"]);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn reads_real_repository_status_and_stages_safely() {
        let root = std::env::temp_dir().join(format!("archipel-ide-git-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        git(&root, &["config", "user.name", "Archipel IDE Test"]);
        fs::write(root.join("tracked.txt"), "first\n").unwrap();
        git(&root, &["add", "tracked.txt"]);
        git(&root, &["commit", "-qm", "initial"]);
        fs::write(root.join("tracked.txt"), "changed\n").unwrap();
        fs::write(root.join("space name.txt"), "new\n").unwrap();

        let status = RepositoryStatus::load(&root).unwrap();
        assert!(
            status
                .changes
                .iter()
                .any(|change| change.path == Path::new("tracked.txt") && change.unstaged)
        );
        let untracked = status
            .changes
            .iter()
            .find(|change| change.path == Path::new("space name.txt"))
            .unwrap();
        assert_eq!(untracked.kind, ChangeKind::Untracked);
        status.stage(&untracked.path).unwrap();
        let staged = RepositoryStatus::load(&root).unwrap();
        assert!(
            staged
                .changes
                .iter()
                .any(|change| change.path == Path::new("space name.txt") && change.staged)
        );
        staged.unstage(Path::new("space name.txt")).unwrap();
        assert!(
            RepositoryStatus::load(&root)
                .unwrap()
                .changes
                .iter()
                .any(|change| change.path == Path::new("space name.txt") && change.unstaged)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn diffs_working_files_against_head() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-git-diff-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        git(&root, &["config", "user.name", "Archipel IDE Test"]);
        let status = RepositoryStatus::load(&root).unwrap();
        assert_eq!(status.head_revision(), None);
        fs::write(root.join("src/a b.txt"), "one\ntwo\n").unwrap();
        fs::write(root.join("bin.dat"), [0u8, 1, 2]).unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "initial"]);
        fs::write(root.join("src/a b.txt"), "one\nTWO\nthree\n").unwrap();
        fs::write(root.join("new.txt"), "fresh\n").unwrap();
        let status = RepositoryStatus::load(&root).unwrap();
        let path = Path::new("src/a b.txt");
        let diff = status
            .diff_against_head(path, status.working_text(path).unwrap())
            .unwrap();
        assert_eq!(diff.old_text, "one\ntwo\n");
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.hunks[0].kind(), HunkKind::Modified);
        assert_eq!(diff.base_revision, status.head_revision());
        let added = status
            .diff_against_head(Path::new("new.txt"), Some("fresh\n".into()))
            .unwrap();
        assert_eq!(added.base_revision, None);
        assert_eq!(added.hunks[0].kind(), HunkKind::Inserted);
        let deleted = status.diff_against_head(path, None).unwrap();
        assert_eq!(deleted.hunks[0].kind(), HunkKind::Deleted);
        assert!(matches!(
            status.head_text(Path::new("bin.dat")),
            Err(GitError::Binary(_))
        ));
        assert!(
            status
                .diff_against_head(Path::new("missing"), None)
                .is_err()
        );
        assert_eq!(status.working_text(Path::new("missing")).unwrap(), None);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_non_repository() {
        let root = std::env::temp_dir();
        let result = RepositoryStatus::load(&root);
        assert!(matches!(result, Err(GitError::NotRepository(_))));
    }

    #[test]
    fn commits_only_selected_paths_including_new_files() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-git-commit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        git(&root, &["config", "user.name", "Archipel IDE Test"]);
        fs::write(root.join("a.txt"), "a\n").unwrap();
        git(&root, &["add", "a.txt"]);
        git(&root, &["commit", "-qm", "initial"]);
        fs::write(root.join("a.txt"), "changed\n").unwrap();
        fs::write(root.join("new file.txt"), "new\n").unwrap();
        fs::write(root.join("other.txt"), "keep\n").unwrap();
        let status = RepositoryStatus::load(&root).unwrap();
        assert!(status.commit("  ", &[PathBuf::from("a.txt")]).is_err());
        assert!(status.commit("msg", &[]).is_err());
        let hash = status
            .commit(
                "Update a\n\nwith body",
                &[PathBuf::from("a.txt"), PathBuf::from("new file.txt")],
            )
            .unwrap();
        assert!(!hash.is_empty());
        let after = RepositoryStatus::load(&root).unwrap();
        let remaining: Vec<_> = after.changes.iter().map(|c| c.path.clone()).collect();
        assert_eq!(remaining, vec![PathBuf::from("other.txt")]);
        fs::remove_dir_all(root).unwrap();
    }
}
