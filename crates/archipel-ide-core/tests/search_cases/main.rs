//! Executable search and navigation acceptance cases (`docs/specs/search/cases/*.cases.md`).
//!
//! Every `###` block of the case files is parsed and handed to the runner of its spec file
//! (`matching.rs` for 07 matching and ranking, ...). A runner drives the real core path (the name
//! matcher and the Go to rankers, the find model, the Search Everywhere model, ...) and compares
//! the result with the `after` block. Cases whose result is only visible in the window (pixels,
//! focus, popups of the UI toolkit) count as not runnable, with the category as the reason.
//!
//! The report `docs/parity/search-cases.md` is written when `SEARCH_CASES_REPORT=1`
//! (`cargo run -p xtask -- search-cases`). `SEARCH_CASES_ONLY=SRCH-MAT` runs the cases whose id
//! starts with the text; `SEARCH_CASES_VERBOSE=1` prints the failing cases with expected and
//! actual text. The test fails when fewer cases pass than [`PASS_BASELINE`] (raise it when you
//! fix cases, never lower it).
//!
//! Regex flavor: the spec leaves Java regex versus Rust regex open. The runner uses the Rust
//! `regex` crate; cases that fail because of the flavor are reported as `fail (regex)`.

mod find;
mod fip;
mod matching;
mod nav;
mod se;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Number of cases that must pass. Never lower it; raise it with every fix.
const PASS_BASELINE: usize = 308;

// ---------------------------------------------------------------------------------------------
// Parsing

#[derive(Debug, Default, Clone)]
pub struct Case {
    pub file: String,
    pub id: String,
    pub number: u32,
    pub title: String,
    pub settings: String,
    pub language: String,
    pub status: Option<String>,
    pub before: Option<String>,
    pub keys: String,
    /// Other `key: value` lines outside the blocks (`setup:`, `focus:`, ...).
    pub extra: Vec<String>,
    pub after: Option<String>,
}

impl Case {
    pub fn name(&self) -> String {
        format!("{} case {}", self.id, self.number)
    }

    /// The numbered part of the spec file name (`07`).
    pub fn file_no(&self) -> &str {
        self.file.get(..2).unwrap_or("")
    }

    pub fn before(&self) -> &str {
        self.before.as_deref().unwrap_or("")
    }

    pub fn after(&self) -> &str {
        self.after.as_deref().unwrap_or("")
    }

    /// `key=value` pairs and bare words of the `settings:` line.
    pub fn setting(&self, key: &str) -> Option<String> {
        self.settings
            .split(", ")
            .find_map(|part| part.trim().strip_prefix(&format!("{key}=")))
            .map(|v| v.trim().trim_matches('"').to_owned())
    }
}

fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/specs/search/cases")
}

fn parse_file(path: &Path) -> Vec<Case> {
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let source = std::fs::read_to_string(path).unwrap();
    let mut cases: Vec<Case> = Vec::new();
    let mut fence: Option<(String, String)> = None;
    for line in source.lines() {
        if let Some((kind, body)) = &mut fence {
            if line == "```" {
                let (kind, body) = (kind.clone(), body.clone());
                fence = None;
                let case = cases.last_mut().unwrap();
                let body = body.strip_suffix('\n').unwrap_or(&body).to_owned();
                match kind.as_str() {
                    "before" => case.before = Some(body),
                    "after" => case.after = Some(body),
                    _ => {}
                }
            } else {
                body.push_str(line);
                body.push('\n');
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            let (head, title) = rest.split_once(": ").unwrap_or((rest, ""));
            let (id, number) = head.split_once(" case ").unwrap_or((head, "0"));
            cases.push(Case {
                file: name.clone(),
                id: id.trim().to_owned(),
                number: number.trim().parse().unwrap_or(0),
                title: title.trim().to_owned(),
                ..Case::default()
            });
            continue;
        }
        let Some(case) = cases.last_mut() else {
            continue;
        };
        if let Some(kind) = line.strip_prefix("```") {
            fence = Some((kind.trim().to_owned(), String::new()));
            continue;
        }
        if let Some(value) = line.strip_prefix("settings:") {
            case.settings = value.trim().to_owned();
        } else if let Some(value) = line.strip_prefix("language:") {
            case.language = value.trim().to_owned();
        } else if let Some(value) = line.strip_prefix("status:") {
            case.status = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("keys:") {
            case.keys = value.trim().to_owned();
        } else if !line.trim().is_empty() {
            case.extra.push(line.to_owned());
        }
    }
    cases
}

// ---------------------------------------------------------------------------------------------
// Outcomes

#[derive(Debug, Clone)]
pub enum Outcome {
    Pass,
    Fail(String),
    /// A failure that comes from the regex flavor (Rust `regex` versus Java regex).
    FailRegex(String),
    NotRunnable(&'static str),
}

impl Outcome {
    fn is_fail(&self) -> bool {
        matches!(self, Outcome::Fail(_) | Outcome::FailRegex(_))
    }
}

/// Compares text line by line after collapsing runs of spaces and dropping a trailing
/// parenthesised comment of the case author.
pub fn normalize_line(line: &str) -> String {
    let mut line = line.trim_end().to_owned();
    if line.ends_with(')')
        && let Some(at) = line.rfind("  (")
    {
        line.truncate(at);
    }
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Compares two ordered line lists; the error names the first difference.
pub fn compare_lines(expected: &[String], actual: &[String]) -> Result<(), String> {
    let e: Vec<String> = expected.iter().map(|l| normalize_line(l)).collect();
    let a: Vec<String> = actual.iter().map(|l| normalize_line(l)).collect();
    if e == a {
        return Ok(());
    }
    Err(format!(
        "result differs\n  expected:\n    {}\n  actual:\n    {}",
        expected.join("\n    "),
        actual.join("\n    ")
    ))
}

fn run_case(case: &Case) -> Outcome {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match case.file_no() {
        "07" => matching::run(case),
        "01" => find::run(case),
        "02" => fip::run(case),
        "04" => se::run(case),
        "05" => nav::run(case),
        "03" => Outcome::NotRunnable(
            "needs resolved Java references (usage types, modules, packages) from a language pack or server",
        ),
        "06" => Outcome::NotRunnable("structural search is LATER"),
        _ => Outcome::NotRunnable("no runner yet"),
    }));
    match result {
        Ok(outcome) => outcome,
        Err(panic) => {
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_default();
            Outcome::Fail(format!("panic: {message}"))
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Report

struct Row {
    case: Case,
    outcome: Outcome,
}

fn short(reason: &str) -> String {
    let first = reason.lines().next().unwrap_or_default();
    let first: String = first.chars().take(110).collect();
    first.replace('|', "\\|")
}

/// `[pass, fail, fail (regex), not runnable]`.
fn bucket(outcome: &Outcome) -> usize {
    match outcome {
        Outcome::Pass => 0,
        Outcome::Fail(_) => 1,
        Outcome::FailRegex(_) => 2,
        Outcome::NotRunnable(_) => 3,
    }
}

fn report(rows: &[Row]) -> String {
    let mut files: BTreeMap<&str, [usize; 4]> = BTreeMap::new();
    for row in rows {
        files.entry(row.case.file.as_str()).or_default()[bucket(&row.outcome)] += 1;
    }
    let total = |i: usize| files.values().map(|v| v[i]).sum::<usize>();
    let mut out = String::new();
    out.push_str("# Search and navigation acceptance cases\n\n");
    out.push_str(
        "Generated by `cargo run -p xtask -- search-cases` (the test `crates/archipel-ide-core/tests/search_cases`)\n\
         from `docs/specs/search/cases/*.cases.md`. Do not edit by hand.\n\n",
    );
    let _ = writeln!(
        out,
        "**{} pass, {} fail, {} fail because of the regex flavor, {} not runnable** of {} cases.\n",
        total(0),
        total(1),
        total(2),
        total(3),
        rows.len()
    );
    out.push_str(
        "| Spec file | Pass | Fail | Fail (regex) | Not runnable | Total |\n|---|---:|---:|---:|---:|---:|\n",
    );
    for (file, v) in &files {
        let _ = writeln!(
            out,
            "| {file} | {} | {} | {} | {} | {} |",
            v[0],
            v[1],
            v[2],
            v[3],
            v.iter().sum::<usize>()
        );
    }
    let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows {
        if let Outcome::NotRunnable(reason) = row.outcome {
            *reasons.entry(reason).or_default() += 1;
        }
    }
    if !reasons.is_empty() {
        out.push_str("\nNot runnable (needs window state or a model the core does not have):\n\n");
        for (reason, count) in &reasons {
            let _ = writeln!(out, "- {reason}: {count}");
        }
    }
    let mut groups: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    for row in rows {
        let prefix: Vec<&str> = row.case.id.split('-').take(2).collect();
        groups.entry(prefix.join("-")).or_default()[bucket(&row.outcome)] += 1;
    }
    out.push_str(
        "\n## By requirement group\n\n| Group | Pass | Fail | Fail (regex) | Not runnable |\n|---|---:|---:|---:|---:|\n",
    );
    for (group, v) in &groups {
        let _ = writeln!(
            out,
            "| {group} | {} | {} | {} | {} |",
            v[0], v[1], v[2], v[3]
        );
    }
    let mut current = "";
    for row in rows {
        if row.case.file != current {
            current = row.case.file.as_str();
            let _ = write!(
                out,
                "\n## {current}\n\n| Case | Result | Note |\n|---|---|---|\n"
            );
        }
        let (status, note) = match &row.outcome {
            Outcome::Pass => ("pass", String::new()),
            Outcome::Fail(reason) => ("fail", short(reason)),
            Outcome::FailRegex(reason) => ("fail (regex)", short(reason)),
            Outcome::NotRunnable(reason) => ("not runnable", (*reason).to_owned()),
        };
        let _ = writeln!(out, "| {} | {} | {} |", row.case.name(), status, note);
    }
    out
}

#[test]
fn search_cases_do_not_regress() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(cases_dir())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".cases.md"))
        .collect();
    paths.sort();
    let only = std::env::var("SEARCH_CASES_ONLY").ok();
    let verbose = std::env::var("SEARCH_CASES_VERBOSE").is_ok();
    let mut rows = Vec::new();
    for path in &paths {
        for case in parse_file(path) {
            if only
                .as_ref()
                .is_some_and(|prefixes| !prefixes.split(',').any(|p| case.id.starts_with(p)))
            {
                continue;
            }
            let outcome = run_case(&case);
            if verbose && outcome.is_fail() {
                let (Outcome::Fail(reason) | Outcome::FailRegex(reason)) = &outcome else {
                    unreachable!()
                };
                eprintln!("FAIL {} ({}): {reason}", case.name(), case.title);
            }
            rows.push(Row { case, outcome });
        }
    }
    let passing = rows
        .iter()
        .filter(|r| matches!(r.outcome, Outcome::Pass))
        .count();
    if only.is_none() {
        assert!(
            rows.len() >= 419,
            "cases were dropped: parsed {}",
            rows.len()
        );
        if std::env::var("SEARCH_CASES_REPORT").is_ok() {
            let target =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/parity/search-cases.md");
            std::fs::write(target, report(&rows)).unwrap();
        }
        assert!(
            passing >= PASS_BASELINE,
            "only {passing} search cases pass, the baseline is {PASS_BASELINE}; run \
             `SEARCH_CASES_VERBOSE=1 cargo test -p archipel-ide-core --test search_cases` to see why"
        );
        if passing > PASS_BASELINE {
            eprintln!("{passing} cases pass: raise PASS_BASELINE from {PASS_BASELINE}");
        }
    } else {
        eprintln!("{passing} of {} selected cases pass", rows.len());
    }
}
