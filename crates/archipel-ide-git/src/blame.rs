//! `git blame` for gutter annotations.

use crate::{GitError, git_path};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// The commit that last changed one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameLine {
    /// Full hash; all zeros for lines not committed yet.
    pub hash: String,
    pub author: String,
    /// Author time, seconds since the Unix epoch.
    pub time: i64,
    pub summary: String,
}

impl BlameLine {
    pub fn is_uncommitted(&self) -> bool {
        self.hash.chars().all(|c| c == '0')
    }

    pub fn short(&self) -> &str {
        &self.hash[..self.hash.len().min(8)]
    }
}

/// Blames repository-relative `path`. With `contents`, the given text (an
/// editor buffer) is blamed instead of the file on disk, so unsaved lines
/// show as not committed.
pub fn blame(root: &Path, path: &Path, contents: Option<&str>) -> Result<Vec<BlameLine>, GitError> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args(["--no-optional-locks", "blame", "--porcelain"]);
    if contents.is_some() {
        command.args(["--contents", "-"]);
    }
    command
        .arg("--")
        .arg(git_path(path))
        .stdin(if contents.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    if let (Some(text), Some(mut stdin)) = (contents, child.stdin.take()) {
        // A write error surfaces as a failed exit status below.
        let _ = stdin.write_all(text.as_bytes());
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(GitError::Command(if message.contains("no such path") {
            format!("{} is not under version control", path.display())
        } else {
            message
        }));
    }
    Ok(parse_porcelain(&String::from_utf8_lossy(&output.stdout)))
}

/// Parses `git blame --porcelain`: commit headers are given in full only the
/// first time a commit appears.
fn parse_porcelain(text: &str) -> Vec<BlameLine> {
    #[derive(Default, Clone)]
    struct Info {
        author: String,
        time: i64,
        summary: String,
    }
    let mut commits: HashMap<String, Info> = HashMap::new();
    let mut lines = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if line.starts_with('\t') {
            if let Some(hash) = current.take() {
                let info = commits.get(&hash).cloned().unwrap_or_default();
                lines.push(BlameLine {
                    hash,
                    author: info.author,
                    time: info.time,
                    summary: info.summary,
                });
            }
            continue;
        }
        let mut words = line.splitn(2, ' ');
        let key = words.next().unwrap_or_default();
        let value = words.next().unwrap_or_default();
        if key.len() == 40 && key.chars().all(|c| c.is_ascii_hexdigit()) {
            commits.entry(key.to_owned()).or_default();
            current = Some(key.to_owned());
            continue;
        }
        let Some(info) = current.as_ref().and_then(|hash| commits.get_mut(hash)) else {
            continue;
        };
        match key {
            "author" => info.author = value.to_owned(),
            "author-time" => info.time = value.parse().unwrap_or(0),
            "summary" => info.summary = value.to_owned(),
            _ => {}
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run;
    use std::fs;

    fn git(root: &Path, args: &[&str]) {
        let output = run(root, args).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn blames_committed_and_unsaved_lines() {
        let root = std::env::temp_dir().join(format!("archipel-ide-blame-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "a@example.invalid"]);
        git(&root, &["config", "user.name", "Alice"]);
        fs::write(root.join("src/f.txt"), "one\ntwo\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "Add f"]);
        git(&root, &["config", "user.name", "Bob"]);
        fs::write(root.join("src/f.txt"), "one\nTWO\n").unwrap();
        git(&root, &["commit", "-qam", "Shout two"]);

        let path = Path::new("src/f.txt");
        let lines = blame(&root, path, None).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].author, "Alice");
        assert_eq!(lines[0].summary, "Add f");
        assert_eq!(lines[1].author, "Bob");
        assert!(lines[1].time > 0);
        assert!(!lines[0].is_uncommitted());

        let edited = blame(&root, path, Some("zero\none\nTWO\n")).unwrap();
        assert_eq!(edited.len(), 3);
        assert!(edited[0].is_uncommitted());
        assert_eq!(edited[1].author, "Alice");

        fs::write(root.join("new.txt"), "x\n").unwrap();
        let error = blame(&root, Path::new("new.txt"), None).unwrap_err();
        assert!(
            error.to_string().contains("not under version control"),
            "{error}"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
