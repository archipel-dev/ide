//! `cargo xtask changelog`: release notes from conventional commits.
//!
//! ```text
//! cargo xtask changelog [--from <ref>] [--to <ref>] [--version <x.y.z>] [--output <file>]
//! ```
//!
//! The range defaults to the newest `v*` tag reachable from `--to` (default
//! `HEAD`) up to `--to`; without a tag it covers the whole history. Merge
//! commits are skipped (their branches' commits are listed instead). Subjects
//! follow <https://www.conventionalcommits.org>: `type(scope)!: description`;
//! anything else lands under "Other changes".

use anyhow::{Context, Result, bail};
use std::fmt::Write as _;
use std::process::Command;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub from: Option<String>,
    pub to: Option<String>,
    pub version: Option<String>,
    pub output: Option<String>,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Options> {
        let mut options = Options::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let mut value = || args.next().with_context(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--from" => options.from = Some(value()?),
                "--to" => options.to = Some(value()?),
                "--version" => options.version = Some(value()?),
                "--output" => options.output = Some(value()?),
                other => bail!("unknown changelog option `{other}`"),
            }
        }
        Ok(options)
    }
}

/// One commit as `git log` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub subject: String,
    pub body: String,
}

/// A parsed conventional-commit subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: Option<String>,
    pub scope: Option<String>,
    pub breaking: bool,
    pub description: String,
    pub hash: String,
}

/// Sections in display order: (heading, commit types).
const SECTIONS: [(&str, &[&str]); 7] = [
    ("Features", &["feat"]),
    ("Bug fixes", &["fix"]),
    ("Performance", &["perf"]),
    ("Documentation", &["docs"]),
    ("Refactoring and tests", &["refactor", "test", "style"]),
    ("Build, CI and tooling", &["build", "ci", "chore"]),
    ("Other changes", &[]),
];

pub fn parse(commit: &Commit) -> Entry {
    let subject = commit.subject.trim();
    let hash = commit.hash.chars().take(7).collect();
    let breaking_note = commit
        .body
        .lines()
        .any(|line| line.starts_with("BREAKING CHANGE:") || line.starts_with("BREAKING-CHANGE:"));
    let conventional = subject.split_once(": ").and_then(|(head, description)| {
        let (head, bang) = match head.strip_suffix('!') {
            Some(head) => (head, true),
            None => (head, false),
        };
        let (kind, scope) = match head.split_once('(') {
            Some((kind, rest)) => (kind, Some(rest.strip_suffix(')')?.to_owned())),
            None => (head, None),
        };
        let valid = !kind.is_empty() && kind.chars().all(|c| c.is_ascii_lowercase());
        valid.then(|| (kind.to_owned(), scope, bang, description.trim().to_owned()))
    });
    match conventional {
        Some((kind, scope, bang, description)) => Entry {
            kind: Some(kind),
            scope: scope.filter(|scope| !scope.is_empty()),
            breaking: bang || breaking_note,
            description,
            hash,
        },
        None => Entry {
            kind: None,
            scope: None,
            breaking: breaking_note,
            description: subject.to_owned(),
            hash,
        },
    }
}

fn section_of(entry: &Entry) -> usize {
    let kind = entry.kind.as_deref().unwrap_or_default();
    SECTIONS
        .iter()
        .position(|(_, kinds)| kinds.contains(&kind))
        .unwrap_or(SECTIONS.len() - 1)
}

fn line(entry: &Entry) -> String {
    match &entry.scope {
        Some(scope) => format!("- **{scope}:** {} ({})\n", entry.description, entry.hash),
        None => format!("- {} ({})\n", entry.description, entry.hash),
    }
}

/// Markdown release notes. `commits` are newest first, as `git log` lists them.
pub fn render(title: &str, commits: &[Commit]) -> String {
    let mut entries: Vec<Entry> = Vec::new();
    for entry in commits.iter().map(parse) {
        // Rebased duplicates and reverted-then-reapplied commits repeat subjects.
        if !entries
            .iter()
            .any(|seen| seen.description == entry.description && seen.kind == entry.kind)
        {
            entries.push(entry);
        }
    }
    let mut out = format!("## {title}\n");
    if entries.is_empty() {
        out.push_str("\nNo changes.\n");
        return out;
    }
    let breaking: Vec<&Entry> = entries.iter().filter(|entry| entry.breaking).collect();
    if !breaking.is_empty() {
        out.push_str("\n### Breaking changes\n\n");
        for entry in breaking {
            out.push_str(&line(entry));
        }
    }
    for (index, (heading, _)) in SECTIONS.iter().enumerate() {
        let section: Vec<&Entry> = entries
            .iter()
            .filter(|entry| section_of(entry) == index)
            .collect();
        if section.is_empty() {
            continue;
        }
        let _ = write!(out, "\n### {heading}\n\n");
        for entry in section {
            out.push_str(&line(entry));
        }
    }
    out
}

fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("run git (is it installed?)")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Splits `git log --format=%H%x1f%s%x1f%b%x1e` output.
pub fn parse_log(text: &str) -> Vec<Commit> {
    text.split('\x1e')
        .filter_map(|record| {
            let mut fields = record.trim_start_matches('\n').splitn(3, '\x1f');
            let hash = fields.next()?.trim();
            let subject = fields.next()?;
            (!hash.is_empty()).then(|| Commit {
                hash: hash.to_owned(),
                subject: subject.to_owned(),
                body: fields.next().unwrap_or_default().to_owned(),
            })
        })
        .collect()
}

pub fn run(options: Options) -> Result<()> {
    let to = options.to.as_deref().unwrap_or("HEAD");
    let from = match &options.from {
        Some(from) => Some(from.clone()),
        None => git(&["describe", "--tags", "--abbrev=0", "--match", "v*", to])
            .ok()
            .map(|tag| tag.trim().to_owned()),
    };
    let range = match &from {
        Some(from) => format!("{from}..{to}"),
        None => to.to_owned(),
    };
    let log = git(&["log", "--no-merges", "--format=%H%x1f%s%x1f%b%x1e", &range])?;
    let version = options
        .version
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());
    let date = git(&["log", "-1", "--format=%cs", to])?;
    let title = format!("Archipel IDE {version} ({})", date.trim());
    let mut notes = render(&title, &parse_log(&log));
    if let Some(from) = &from {
        let _ = write!(notes, "\nChanges since `{from}`.\n");
    }
    match options.output {
        Some(path) => {
            std::fs::write(&path, &notes).with_context(|| format!("write {path}"))?;
            println!("wrote {path}");
        }
        None => print!("{notes}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(hash: &str, subject: &str, body: &str) -> Commit {
        Commit {
            hash: hash.into(),
            subject: subject.into(),
            body: body.into(),
        }
    }

    #[test]
    fn parses_conventional_subjects() {
        let entry = parse(&commit("abcdef123", "feat(ui): New UI workbench", ""));
        assert_eq!(entry.kind.as_deref(), Some("feat"));
        assert_eq!(entry.scope.as_deref(), Some("ui"));
        assert_eq!(entry.description, "New UI workbench");
        assert_eq!(entry.hash, "abcdef1");
        assert!(!entry.breaking);
        assert!(parse(&commit("1", "feat!: drop old config", "")).breaking);
        assert!(parse(&commit("1", "fix(lsp)!: rename", "")).breaking);
        assert!(parse(&commit("1", "refactor: x", "text\nBREAKING CHANGE: gone")).breaking);
        let plain = parse(&commit("1", "Update README", ""));
        assert_eq!(plain.kind, None);
        assert_eq!(plain.description, "Update README");
        // Not a type: uppercase, spaces, unclosed scope.
        assert_eq!(parse(&commit("1", "WIP: stuff", "")).kind, None);
        assert_eq!(parse(&commit("1", "fix(ui: stuff", "")).kind, None);
        assert_eq!(parse(&commit("1", "fix stuff: more", "")).kind, None);
    }

    #[test]
    fn renders_sections_in_order_with_breaking_first() {
        let commits = [
            commit("1111111a", "ci: lean workflows", ""),
            commit("2222222b", "fix(updates): save before restart", ""),
            commit("3333333c", "feat(ui)!: new toolbar", ""),
            commit("4444444d", "feat: terminal grid", ""),
            commit("5555555e", "Tweak things", ""),
            commit("6666666f", "feat: terminal grid", ""),
        ];
        let notes = render("Archipel IDE 0.0.3 (2026-10-10)", &commits);
        assert_eq!(
            notes,
            "## Archipel IDE 0.0.3 (2026-10-10)\n\
             \n### Breaking changes\n\n\
             - **ui:** new toolbar (3333333)\n\
             \n### Features\n\n\
             - **ui:** new toolbar (3333333)\n\
             - terminal grid (4444444)\n\
             \n### Bug fixes\n\n\
             - **updates:** save before restart (2222222)\n\
             \n### Build, CI and tooling\n\n\
             - lean workflows (1111111)\n\
             \n### Other changes\n\n\
             - Tweak things (5555555)\n"
        );
        assert!(render("x", &[]).ends_with("No changes.\n"));
    }

    #[test]
    fn splits_git_log_records() {
        let log = "aaa\x1ffeat: one\x1fbody line\n\x1e\nbbb\x1ffix: two\x1f\x1e\n";
        let commits = parse_log(log);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "feat: one");
        assert_eq!(commits[0].body, "body line\n");
        assert_eq!(commits[1].hash, "bbb");
        assert!(parse_log("").is_empty());
    }

    #[test]
    fn parses_options() {
        let parse = |args: &[&str]| Options::parse(args.iter().map(|a| a.to_string()));
        assert_eq!(
            parse(&["--from", "v0.0.1", "--version", "0.0.3"]).unwrap(),
            Options {
                from: Some("v0.0.1".into()),
                version: Some("0.0.3".into()),
                ..Options::default()
            }
        );
        assert!(parse(&["--from"]).is_err());
        assert!(parse(&["--nope"]).is_err());
    }
}
