//! Commit history, branches and the commit graph for the Git log tool window.

use crate::{ChangeKind, GitError, checked, run};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Field and record separators for `git log --format`.
const FIELD: char = '\u{1f}';
const RECORD: char = '\u{1e}';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    pub hash: String,
    pub short: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    /// Author time, seconds since the Unix epoch.
    pub time: i64,
    /// Ref names pointing at the commit (`HEAD -> master`, `origin/main`, `tag: v1`).
    pub refs: Vec<String>,
    pub subject: String,
}

impl CommitInfo {
    /// Case-insensitive match on subject, author or hash prefix.
    pub fn matches(&self, filter: &str) -> bool {
        let filter = filter.trim();
        if filter.is_empty() {
            return true;
        }
        let needle = filter.to_lowercase();
        self.hash.starts_with(&needle)
            || self.subject.to_lowercase().contains(&needle)
            || self.author.to_lowercase().contains(&needle)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBranch {
    pub name: String,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Branches {
    /// Checked-out branch, or `None` when HEAD is detached.
    pub current: Option<String>,
    pub local: Vec<LocalBranch>,
    pub remote: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFile {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDetails {
    pub hash: String,
    /// Full commit message.
    pub message: String,
    pub committer: String,
    pub commit_time: i64,
    /// Changes against the first parent (or the empty tree for a root commit).
    pub files: Vec<CommitFile>,
}

/// Which history to list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogQuery {
    /// A branch or other revision; `None` lists all branches, remotes and tags.
    pub revision: Option<String>,
    /// Only commits touching this repository-relative path.
    pub path: Option<PathBuf>,
    pub limit: usize,
}

pub fn log(root: &Path, query: &LogQuery) -> Result<Vec<CommitInfo>, GitError> {
    if !run(root, &["rev-parse", "--verify", "-q", "HEAD"])?
        .status
        .success()
    {
        return Ok(Vec::new()); // No commits yet.
    }
    let format = format!(
        "--format=%H{FIELD}%h{FIELD}%P{FIELD}%an{FIELD}%ae{FIELD}%at{FIELD}%D{FIELD}%s{RECORD}"
    );
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args([
            "--no-optional-locks",
            "log",
            "--date-order",
            "--decorate=full",
        ])
        .arg(format)
        .arg(format!("--max-count={}", query.limit.max(1)));
    match &query.revision {
        Some(revision) => {
            if revision.starts_with('-') {
                return Err(GitError::Command(format!("invalid revision `{revision}`")));
            }
            command.arg(revision);
        }
        None => {
            command.args(["--branches", "--remotes", "--tags", "HEAD"]);
        }
    }
    if query.path.is_some() {
        // Rewrite parents to the previous commit touching the path, so the
        // graph of a file history stays connected.
        command.arg("--parents");
    }
    // `--` keeps a revision that is also a file name from being read as a path.
    command.arg("--");
    if let Some(path) = &query.path {
        command.arg(path);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(GitError::Command(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ));
    }
    Ok(parse_log(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_log(text: &str) -> Vec<CommitInfo> {
    text.split(RECORD)
        .filter_map(|record| {
            let record = record.trim_start_matches('\n');
            let fields: Vec<&str> = record.split(FIELD).collect();
            let [hash, short, parents, author, email, time, refs, subject] = fields[..] else {
                return None;
            };
            Some(CommitInfo {
                hash: hash.into(),
                short: short.into(),
                parents: parents.split_whitespace().map(Into::into).collect(),
                author: author.into(),
                email: email.into(),
                time: time.trim().parse().unwrap_or(0),
                refs: refs
                    .split(", ")
                    .filter(|name| !name.is_empty())
                    .map(short_ref)
                    .collect(),
                subject: subject.into(),
            })
        })
        .collect()
}

/// `HEAD -> refs/heads/main` → `HEAD -> main`; `refs/remotes/o/x` → `o/x`;
/// `tag: refs/tags/v1` → `tag: v1`.
fn short_ref(name: &str) -> String {
    name.replace("refs/heads/", "")
        .replace("refs/remotes/", "")
        .replace("refs/tags/", "")
}

pub fn branches(root: &Path) -> Result<Branches, GitError> {
    let output = checked(
        root,
        &[
            "for-each-ref",
            "--format=%(refname)%1f%(upstream:short)%1f%(upstream:track,nobracket)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ],
    )?;
    let mut branches = Branches::default();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let fields: Vec<&str> = line.split(FIELD).collect();
        let Some(name) = fields.first() else { continue };
        if let Some(local) = name.strip_prefix("refs/heads/") {
            let track = fields.get(2).copied().unwrap_or("");
            let count = |key: &str| {
                track
                    .split(", ")
                    .find_map(|part| part.strip_prefix(key))
                    .and_then(|n| n.trim().parse().ok())
                    .unwrap_or(0)
            };
            branches.local.push(LocalBranch {
                name: local.into(),
                upstream: fields
                    .get(1)
                    .filter(|upstream| !upstream.is_empty())
                    .map(|upstream| upstream.to_string()),
                ahead: count("ahead "),
                behind: count("behind "),
            });
        } else if let Some(remote) = name.strip_prefix("refs/remotes/") {
            if !remote.ends_with("/HEAD") {
                branches.remote.push(remote.into());
            }
        } else if let Some(tag) = name.strip_prefix("refs/tags/") {
            branches.tags.push(tag.into());
        }
    }
    let head = run(root, &["symbolic-ref", "--short", "-q", "HEAD"])?;
    if head.status.success() {
        branches.current = Some(String::from_utf8_lossy(&head.stdout).trim().into());
    }
    Ok(branches)
}

pub fn commit_details(root: &Path, hash: &str) -> Result<CommitDetails, GitError> {
    if hash.is_empty() || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(GitError::Command(format!("invalid commit hash `{hash}`")));
    }
    let header = checked(
        root,
        &[
            "show",
            "-s",
            &format!("--format=%P{FIELD}%cn{FIELD}%ct{FIELD}%B"),
            hash,
        ],
    )?;
    let header = String::from_utf8_lossy(&header.stdout).into_owned();
    let mut fields = header.splitn(4, FIELD);
    let parents: Vec<&str> = fields.next().unwrap_or("").split_whitespace().collect();
    let committer = fields.next().unwrap_or("").to_owned();
    let commit_time = fields.next().unwrap_or("0").trim().parse().unwrap_or(0);
    let message = fields.next().unwrap_or("").trim_end().to_owned();
    let names = match parents.first() {
        Some(parent) => checked(
            root,
            &["diff", "--name-status", "-z", "-M", parent, hash, "--"],
        )?,
        None => checked(
            root,
            &[
                "diff-tree",
                "--root",
                "-r",
                "--name-status",
                "-z",
                "-M",
                "--no-commit-id",
                hash,
            ],
        )?,
    };
    Ok(CommitDetails {
        hash: hash.into(),
        message,
        committer,
        commit_time,
        files: parse_name_status(&names.stdout),
    })
}

fn parse_name_status(bytes: &[u8]) -> Vec<CommitFile> {
    let fields: Vec<String> = bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| String::from_utf8_lossy(field).into_owned())
        .collect();
    let mut files = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let status = &fields[index];
        let kind = match status.chars().next() {
            Some('A') => ChangeKind::Added,
            Some('D') => ChangeKind::Deleted,
            Some('R') | Some('C') => ChangeKind::Renamed,
            Some('U') => ChangeKind::Conflicted,
            _ => ChangeKind::Modified,
        };
        // Renames and copies list the old and the new path.
        let skip = if matches!(kind, ChangeKind::Renamed) {
            2
        } else {
            1
        };
        if let Some(path) = fields.get(index + skip) {
            files.push(CommitFile {
                path: PathBuf::from(path),
                kind,
            });
        }
        index += skip + 1;
    }
    files
}

/// One line segment of a graph row. Columns are lane indices; the row is
/// split into a top half (above the commit dot) and a bottom half.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphEdge {
    /// A lane passing by the commit, from the top to the bottom of the row.
    Through { column: usize },
    /// From a lane at the top of the row into the commit dot.
    Into { column: usize },
    /// From the commit dot to a parent lane at the bottom of the row.
    OutOf { column: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRow {
    /// Lane of the commit dot.
    pub column: usize,
    pub edges: Vec<GraphEdge>,
}

/// Assigns commits (newest first, each parent listed after its children) to
/// lanes so every parent link can be drawn as straight segments.
pub fn graph(commits: &[CommitInfo]) -> Vec<GraphRow> {
    let mut lanes: Vec<Option<&str>> = Vec::new();
    let mut rows = Vec::with_capacity(commits.len());
    for commit in commits {
        let hash = commit.hash.as_str();
        let column = lanes
            .iter()
            .position(|lane| *lane == Some(hash))
            .or_else(|| lanes.iter().position(Option::is_none))
            .unwrap_or_else(|| {
                lanes.push(None);
                lanes.len() - 1
            });
        let mut edges = Vec::new();
        for (index, lane) in lanes.iter().enumerate() {
            match lane {
                Some(expected) if *expected == hash => {
                    edges.push(GraphEdge::Into { column: index })
                }
                Some(_) => edges.push(GraphEdge::Through { column: index }),
                None => {}
            }
        }
        for lane in lanes.iter_mut() {
            if *lane == Some(hash) {
                *lane = None;
            }
        }
        for (position, parent) in commit.parents.iter().enumerate() {
            let parent = parent.as_str();
            let target = if position == 0 && lanes[column].is_none() {
                Some(column)
            } else {
                lanes.iter().position(|lane| *lane == Some(parent))
            };
            let target = target.unwrap_or_else(|| {
                lanes.iter().position(Option::is_none).unwrap_or_else(|| {
                    lanes.push(None);
                    lanes.len() - 1
                })
            });
            if lanes[target].is_none() {
                lanes[target] = Some(parent);
            }
            edges.push(GraphEdge::OutOf { column: target });
        }
        while lanes.last() == Some(&None) {
            lanes.pop();
        }
        rows.push(GraphRow { column, edges });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn git(root: &Path, args: &[&str]) {
        let output = run(root, args).unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn commit(hash: &str, parents: &[&str]) -> CommitInfo {
        CommitInfo {
            hash: hash.into(),
            short: hash.into(),
            parents: parents.iter().map(|p| p.to_string()).collect(),
            author: "A".into(),
            email: String::new(),
            time: 0,
            refs: Vec::new(),
            subject: hash.into(),
        }
    }

    #[test]
    fn linear_history_stays_in_one_lane() {
        let rows = graph(&[commit("c", &["b"]), commit("b", &["a"]), commit("a", &[])]);
        assert!(rows.iter().all(|row| row.column == 0));
        assert_eq!(rows[0].edges, vec![GraphEdge::OutOf { column: 0 }]);
        assert_eq!(
            rows[1].edges,
            vec![
                GraphEdge::Into { column: 0 },
                GraphEdge::OutOf { column: 0 }
            ]
        );
        assert_eq!(rows[2].edges, vec![GraphEdge::Into { column: 0 }]);
    }

    #[test]
    fn merge_opens_and_closes_a_second_lane() {
        // m merges side branch s into main line b; both come from a.
        let rows = graph(&[
            commit("m", &["b", "s"]),
            commit("s", &["a"]),
            commit("b", &["a"]),
            commit("a", &[]),
        ]);
        assert_eq!(rows[0].column, 0);
        assert_eq!(
            rows[0].edges,
            vec![
                GraphEdge::OutOf { column: 0 },
                GraphEdge::OutOf { column: 1 }
            ]
        );
        assert_eq!(rows[1].column, 1);
        assert!(rows[1].edges.contains(&GraphEdge::Through { column: 0 }));
        assert_eq!(rows[2].column, 0);
        // Both lanes converge into the root.
        assert_eq!(rows[3].column, 0);
        assert!(rows[3].edges.contains(&GraphEdge::Into { column: 0 }));
        assert!(rows[3].edges.contains(&GraphEdge::Into { column: 1 }));
    }

    #[test]
    fn filters_by_subject_author_or_hash() {
        let mut c = commit("abc123", &[]);
        c.subject = "Fix Parser".into();
        assert!(c.matches("parser"));
        assert!(c.matches("ABC"));
        assert!(c.matches("  "));
        assert!(!c.matches("lexer"));
    }

    #[test]
    fn reads_log_branches_and_commit_files_from_a_repository() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-git-log-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "dev@example.invalid"]);
        git(&root, &["config", "user.name", "Dev"]);
        assert!(log(&root, &LogQuery::default()).unwrap().is_empty());
        fs::write(root.join("a.txt"), "a\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "Initial commit\n\nWith body"]);
        git(&root, &["checkout", "-qb", "feature"]);
        fs::write(root.join("b.txt"), "b\n").unwrap();
        fs::remove_file(root.join("a.txt")).unwrap();
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-qm", "Add b"]);
        git(&root, &["tag", "v1"]);
        git(&root, &["checkout", "-q", "main"]);

        let commits = log(
            &root,
            &LogQuery {
                limit: 100,
                ..LogQuery::default()
            },
        )
        .unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "Add b");
        assert!(commits[0].refs.iter().any(|r| r == "feature"));
        assert!(commits[0].refs.iter().any(|r| r == "tag: v1"));
        assert!(commits[1].refs.iter().any(|r| r == "HEAD -> main"));
        assert_eq!(commits[0].parents, vec![commits[1].hash.clone()]);
        let main_only = log(
            &root,
            &LogQuery {
                revision: Some("main".into()),
                limit: 100,
                ..LogQuery::default()
            },
        )
        .unwrap();
        assert_eq!(main_only.len(), 1);
        // File history of b.txt: only the commit that added it, with no
        // parent that touches b.txt.
        let history = log(
            &root,
            &LogQuery {
                path: Some(PathBuf::from("b.txt")),
                limit: 100,
                ..LogQuery::default()
            },
        )
        .unwrap();
        assert_eq!(history.len(), 1);
        assert!(history[0].parents.is_empty(), "{:?}", history[0].parents);
        assert!(
            log(
                &root,
                &LogQuery {
                    revision: Some("--all".into()),
                    limit: 1,
                    ..LogQuery::default()
                }
            )
            .is_err()
        );

        let branches = branches(&root).unwrap();
        assert_eq!(branches.current.as_deref(), Some("main"));
        let names: Vec<_> = branches.local.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, vec!["feature", "main"]);
        assert_eq!(branches.tags, vec!["v1"]);

        let details = commit_details(&root, &commits[0].hash).unwrap();
        assert_eq!(details.message, "Add b");
        let mut files: Vec<_> = details
            .files
            .iter()
            .map(|f| (f.path.display().to_string(), f.kind))
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            files,
            vec![
                ("a.txt".into(), ChangeKind::Deleted),
                ("b.txt".into(), ChangeKind::Added)
            ]
        );
        let root_details = commit_details(&root, &commits[1].hash).unwrap();
        assert_eq!(root_details.message, "Initial commit\n\nWith body");
        assert_eq!(root_details.files.len(), 1);
        assert!(commit_details(&root, "--oops").is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parses_renames_in_name_status() {
        let files = parse_name_status(b"R100\0old.txt\0new.txt\0M\0x.rs\0");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, PathBuf::from("new.txt"));
        assert_eq!(files[0].kind, ChangeKind::Renamed);
        assert_eq!(files[1].kind, ChangeKind::Modified);
    }
}
