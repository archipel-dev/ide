//! Spec coverage: how many clean-room requirements (`docs/specs/**`) are implemented.
//!
//! * Requirements are parsed from three row formats: table rows (`| ID | Pri | ... |`),
//!   bullets (`- **ID** `MUST` text`) and bare lines (`` `ID` MUST: text ``).
//! * Evidence is found by scanning `crates/`, `plugins/` and `xtask/` for IDs (`JAVA-IMP-012`,
//!   ranges `JAVA-IMP-070..076`, lists `JAVA-ERR-012 / 144`, and test names such as
//!   `plat_psi_051`) and `docs/parity/` (scenes, scenarios, evidence) for IDs.
//! * Status: `implemented` = referenced from code and from a test, scenario or scene;
//!   `partial` = any other reference; `missing` = none. `docs/specs/coverage.toml` holds
//!   explicit overrides with evidence (a code path and a test path, both verified to exist).
//!
//! `--check` validates IDs and override paths only; missing coverage never fails it.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

const SPECS_DIR: &str = "docs/specs";
const MAPPING_FILE: &str = "docs/specs/coverage.toml";
const REPORT_FILE: &str = "docs/specs/coverage.md";
const SETS: &[(&str, &str)] = &[
    ("JAVA", "Java"),
    ("PLAT", "Platform"),
    ("CI", "Code insight"),
    ("BLD", "Build"),
    ("EXE", "Execution"),
    ("VCS", "Version control"),
    ("ED", "Editor"),
    ("WB", "Workbench"),
    ("UI", "UI look and feel"),
];
/// Pre-spec ID areas from `docs/spec/features/` that still appear in evidence files.
const LEGACY_AREAS: [&str; 2] = ["JAVA-NAV", "JAVA-GEN"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Priority {
    Must,
    Should,
    Later,
}

impl Priority {
    const ALL: [Priority; 3] = [Priority::Must, Priority::Should, Priority::Later];

    fn parse(text: &str) -> Option<Priority> {
        match text.trim().trim_matches(['*', '`']).trim() {
            "MUST" => Some(Priority::Must),
            "SHOULD" => Some(Priority::Should),
            "LATER" => Some(Priority::Later),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Priority::Must => "MUST",
            Priority::Should => "SHOULD",
            Priority::Later => "LATER",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Status {
    Implemented,
    Partial,
    Missing,
}

impl Status {
    fn parse(text: &str) -> Option<Status> {
        match text {
            "implemented" => Some(Status::Implemented),
            "partial" => Some(Status::Partial),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
struct Requirement {
    id: String,
    /// `JAVA-IMP`, `PLAT-DUMB`, ...
    area: String,
    num: u32,
    priority: Priority,
    title: String,
    accept: String,
    file: String,
    line: usize,
}

impl Requirement {
    fn set(&self) -> &str {
        self.area.split('-').next().unwrap_or("")
    }
}

// ---------------------------------------------------------------------------------------------
// Parsing the specs
// ---------------------------------------------------------------------------------------------

/// A parsed ID: `(area, number, suffix)`, for example `("JAVA-IMP", 19, "a")`.
fn parse_id(text: &str) -> Option<(String, u32, String)> {
    let text = text.trim().trim_matches(['*', '`']).trim();
    let mut parts = text.splitn(3, '-');
    let set = parts.next()?;
    let area = parts.next()?;
    let rest = parts.next()?;
    if !SETS.iter().any(|(s, _)| *s == set)
        || !(2..=5).contains(&area.len())
        || !area.bytes().all(|b| b.is_ascii_uppercase())
    {
        return None;
    }
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let suffix = &rest[digits..];
    if digits == 0
        || digits > 4
        || suffix.len() > 1
        || !suffix.bytes().all(|b| b.is_ascii_lowercase())
    {
        return None;
    }
    Some((
        format!("{set}-{area}"),
        rest[..digits].parse().ok()?,
        suffix.to_string(),
    ))
}

fn canonical(area: &str, num: u32, suffix: &str) -> String {
    format!("{area}-{num:03}{suffix}")
}

fn split_cells(line: &str) -> Vec<String> {
    let inner = line.trim();
    let inner = inner.strip_prefix('|').unwrap_or(inner);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let mut cells = vec![String::new()];
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cells.last_mut().expect("non-empty").push('|');
                chars.next();
            }
            '|' => cells.push(String::new()),
            _ => cells.last_mut().expect("non-empty").push(c),
        }
    }
    cells.iter().map(|c| c.trim().to_string()).collect()
}

fn clean(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate(text: &str, max: usize) -> String {
    let text = clean(text);
    if text.chars().count() <= max {
        return text;
    }
    let cut: String = text.chars().take(max).collect();
    format!("{}…", cut.trim_end())
}

/// Splits `text` into (requirement, acceptance) at the first `Accept:` / `Acceptance:` marker.
fn split_accept(text: &str) -> (String, String) {
    for marker in ["*Accept:*", "Acceptance:", "*Acceptance:*", "Accept:"] {
        if let Some(at) = text.find(marker) {
            return (
                text[..at].trim().to_string(),
                text[at + marker.len()..].trim().to_string(),
            );
        }
    }
    (text.trim().to_string(), String::new())
}

fn parse_spec(file: &str, text: &str, out: &mut Vec<Requirement>) {
    let lines: Vec<&str> = text.lines().collect();
    let mut header: Option<Vec<String>> = None;
    let mut i = 0;
    let push = |out: &mut Vec<Requirement>,
                id: &str,
                pri: Priority,
                title: &str,
                acc: &str,
                line: usize| {
        if let Some((area, num, suffix)) = parse_id(id) {
            out.push(Requirement {
                id: canonical(&area, num, &suffix),
                area,
                num,
                priority: pri,
                title: truncate(title, 140),
                accept: truncate(acc, 240),
                file: file.to_string(),
                line,
            });
        }
    };
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();
        if line.starts_with('|') {
            let cells = split_cells(line);
            if cells.first().map(String::as_str) == Some("ID")
                && cells.get(1).map(String::as_str) == Some("Pri")
            {
                header = Some(cells);
            } else if let Some(head) = &header
                && cells.len() > 2
                && parse_id(&cells[0]).is_some()
                && let Some(pri) = Priority::parse(&cells[1])
            {
                let col = |names: &[&str]| {
                    head.iter()
                        .position(|h| names.contains(&h.as_str()))
                        .and_then(|p| cells.get(p))
                        .map(String::as_str)
                };
                let title = col(&[
                    "Requirement",
                    "Message",
                    "Name",
                    "Behavior",
                    "Feature",
                    "Scope",
                    "Scope constructor",
                    "Tracker",
                    "Short name",
                ])
                .or(cells.get(2).map(String::as_str))
                .unwrap_or("");
                let accept = col(&[
                    "Acceptance",
                    "Trigger",
                    "Meaning",
                    "Increments when",
                    "Contains",
                ])
                .unwrap_or("");
                push(out, &cells[0], pri, title, accept, i + 1);
            }
            i += 1;
            continue;
        }
        header = None;
        // Bullet: `- **ID** `PRI` text` with indented continuation lines.
        let bullet = trimmed
            .strip_prefix("- **")
            .or_else(|| trimmed.strip_prefix("* **"))
            .filter(|_| trimmed.len() == line.len());
        let bare = trimmed
            .strip_prefix('`')
            .filter(|_| trimmed.len() == line.len());
        let (id, rest) = if let Some(rest) = bullet {
            match rest.split_once("**") {
                Some((id, rest)) => (id, rest),
                None => ("", ""),
            }
        } else if let Some(rest) = bare {
            match rest.split_once('`') {
                Some((id, rest)) => (id, rest),
                None => ("", ""),
            }
        } else {
            ("", "")
        };
        if parse_id(id).is_some() {
            let rest = rest.trim_start();
            let (pri, rest) = if bullet.is_some() {
                match rest.strip_prefix('`').and_then(|r| r.split_once('`')) {
                    Some((p, r)) => (Priority::parse(p), r),
                    None => (None, rest),
                }
            } else {
                match rest.split_once(':') {
                    Some((p, r)) => (Priority::parse(p), r),
                    None => (None, rest),
                }
            };
            // Bullets without a priority (the unnumbered Spring feature notes) are LATER.
            let pri = pri.or((bullet.is_some() && !rest.is_empty()).then_some(Priority::Later));
            if let Some(pri) = pri {
                let mut body = rest.trim().to_string();
                let start = i;
                while i + 1 < lines.len() {
                    let next = lines[i + 1];
                    let continues = if bullet.is_some() {
                        next.starts_with("  ") && !next.trim().is_empty()
                    } else {
                        !next.trim().is_empty()
                            && !next.starts_with('#')
                            && !next.starts_with('`')
                            && !next.starts_with('|')
                            && !next.starts_with("- ")
                    };
                    if !continues {
                        break;
                    }
                    body.push(' ');
                    body.push_str(next.trim());
                    i += 1;
                }
                let (req, acc) = split_accept(&body);
                let title = req.split(". ").next().unwrap_or(&req).to_string();
                push(out, id, pri, &title, &acc, start + 1);
            }
        }
        i += 1;
    }
}

fn collect_files(dir: &Path, ext: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name != "target" && !name.starts_with('.') && name != "node_modules" {
                collect_files(&path, ext, out);
            }
        } else if ext.is_empty()
            || path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| ext.contains(&e))
        {
            out.push(path);
        }
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

// ---------------------------------------------------------------------------------------------
// Scanning for references
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
struct RefToken {
    area: String,
    first: u32,
    /// Inclusive end of a range (`..`), or extra numbers of a `/` list.
    last: Option<u32>,
    extra: Vec<u32>,
    suffix: String,
    /// Written as a lowercase test name; unknown areas are then not an error.
    lowercase: bool,
    line: usize,
}

fn digits_at(b: &[u8], mut i: usize, max: usize) -> (u32, usize) {
    let start = i;
    while i < b.len() && b[i].is_ascii_digit() && i - start < max {
        i += 1;
    }
    (
        std::str::from_utf8(&b[start..i])
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        i,
    )
}

fn scan_line(text: &str, line: usize, out: &mut Vec<RefToken>) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let boundary_ok = |i: usize, allow_us: bool| {
            i == 0 || {
                let p = b[i - 1];
                !(p.is_ascii_alphanumeric() || p == b'-' || (p == b'_' && !allow_us))
            }
        };
        let upper = [("PLAT-", 5), ("CI-", 3), ("JAVA-", 5)]
            .iter()
            .find(|(p, _)| b[i..].starts_with(p.as_bytes()))
            .map(|(_, n)| *n);
        if let (Some(n), true) = (upper, boundary_ok(i, false)) {
            let set = &text[i..i + n - 1];
            let a0 = i + n;
            let mut a1 = a0;
            while a1 < b.len() && b[a1].is_ascii_uppercase() {
                a1 += 1;
            }
            if (2..=5).contains(&(a1 - a0)) && b.get(a1) == Some(&b'-') {
                let (first, mut j) = digits_at(b, a1 + 1, 5);
                if j > a1 + 1 && j - (a1 + 1) <= 4 {
                    let area = format!("{set}-{}", &text[a0..a1]);
                    let mut suffix = String::new();
                    if j < b.len()
                        && b[j].is_ascii_lowercase()
                        && !b.get(j + 1).is_some_and(|c| c.is_ascii_alphanumeric())
                    {
                        suffix = (b[j] as char).to_string();
                        j += 1;
                    }
                    let mut tok = RefToken {
                        area,
                        first,
                        last: None,
                        extra: Vec::new(),
                        suffix,
                        lowercase: false,
                        line,
                    };
                    if b[j..].starts_with(b"..") {
                        let mut k = j + 2;
                        // `JAVA-IMP-070..JAVA-IMP-076` repeats the prefix.
                        let prefix = format!("{}-", tok.area);
                        if text[k..].starts_with(&prefix) {
                            k += prefix.len();
                        }
                        let (last, k2) = digits_at(b, k, 4);
                        if k2 > k {
                            tok.last = Some(last);
                            j = k2;
                        }
                    } else {
                        // `JAVA-ERR-012 / 144 / 150`
                        loop {
                            let mut k = j;
                            while k < b.len() && b[k] == b' ' {
                                k += 1;
                            }
                            if b.get(k) != Some(&b'/') {
                                break;
                            }
                            k += 1;
                            while k < b.len() && b[k] == b' ' {
                                k += 1;
                            }
                            let (n, k2) = digits_at(b, k, 4);
                            if k2 - k < 3 || b.get(k2).is_some_and(|c| c.is_ascii_alphanumeric()) {
                                break;
                            }
                            tok.extra.push(n);
                            j = k2;
                        }
                    }
                    out.push(tok);
                    i = j.max(i + 1);
                    continue;
                }
            }
        }
        // Lowercase test names: plat_psi_051, ci_alt_007, java_imp_012.
        let lower = [("plat_", "PLAT"), ("ci_", "CI"), ("java_", "JAVA")]
            .iter()
            .find(|(p, _)| b[i..].starts_with(p.as_bytes()));
        if let Some((p, set)) = lower
            && boundary_ok(i, true)
        {
            let a0 = i + p.len();
            let mut a1 = a0;
            while a1 < b.len() && b[a1].is_ascii_lowercase() {
                a1 += 1;
            }
            if (2..=5).contains(&(a1 - a0)) && b.get(a1) == Some(&b'_') {
                let (first, mut j) = digits_at(b, a1 + 1, 4);
                if j - (a1 + 1) == 3 {
                    let mut suffix = String::new();
                    if j < b.len()
                        && b[j].is_ascii_lowercase()
                        && !b.get(j + 1).is_some_and(|c| c.is_ascii_alphanumeric())
                    {
                        suffix = (b[j] as char).to_string();
                        j += 1;
                    }
                    out.push(RefToken {
                        area: format!("{set}-{}", text[a0..a1].to_ascii_uppercase()),
                        first,
                        last: None,
                        extra: Vec::new(),
                        suffix,
                        lowercase: true,
                        line,
                    });
                    i = j.max(i + 1);
                    continue;
                }
            }
        }
        i += 1;
    }
}

fn scan_text(text: &str) -> Vec<RefToken> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        scan_line(line, n + 1, &mut out);
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Code,
    Test,
    Scenario,
}

#[derive(Default, Debug)]
struct Refs {
    code: BTreeSet<String>,
    test: BTreeSet<String>,
    scenario: BTreeSet<String>,
}

impl Refs {
    fn bucket(&mut self, kind: Kind) -> &mut BTreeSet<String> {
        match kind {
            Kind::Code => &mut self.code,
            Kind::Test => &mut self.test,
            Kind::Scenario => &mut self.scenario,
        }
    }
}

fn is_test_path(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    path.split('/').any(|c| c == "tests" || c == "benches")
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name.ends_with("_test.rs")
}

/// First line (1-based) of an inline `#[cfg(test)]` module, if any.
fn test_region_start(text: &str) -> Option<usize> {
    text.lines()
        .position(|l| l.trim_start().starts_with("#[cfg(test)]"))
        .map(|n| n + 1)
}

struct Index {
    /// All defined IDs.
    ids: BTreeMap<String, Requirement>,
    areas: BTreeSet<String>,
}

impl Index {
    /// Resolves a token to the defined IDs it names; unknown ones become problems.
    fn resolve(&self, tok: &RefToken, where_: &str, problems: &mut Vec<String>) -> Vec<String> {
        let mut found = Vec::new();
        let known_area = self.areas.contains(&tok.area);
        if !known_area {
            if !tok.lowercase && !LEGACY_AREAS.contains(&tok.area.as_str()) {
                problems.push(format!(
                    "{where_}:{}: unknown ID area `{}` (ID `{}`)",
                    tok.line,
                    tok.area,
                    canonical(&tok.area, tok.first, &tok.suffix)
                ));
            }
            return found;
        }
        let mut want = |num: u32, suffix: &str| {
            let id = canonical(&tok.area, num, suffix);
            if self.ids.contains_key(&id) {
                found.push(id);
            } else {
                problems.push(format!(
                    "{where_}:{}: unknown requirement ID `{id}`",
                    tok.line
                ));
            }
        };
        want(tok.first, &tok.suffix);
        for n in &tok.extra {
            want(*n, "");
        }
        if let Some(last) = tok.last {
            if last <= tok.first {
                problems.push(format!(
                    "{where_}:{}: empty range `{}-{:03}..{last:03}`",
                    tok.line, tok.area, tok.first
                ));
            } else {
                want(last, "");
                for req in self.ids.values() {
                    if req.area == tok.area && req.num > tok.first && req.num < last {
                        found.push(req.id.clone());
                    }
                }
            }
        }
        found
    }
}

fn scan_references(
    root: &Path,
    index: &Index,
    problems: &mut Vec<String>,
) -> BTreeMap<String, Refs> {
    let mut refs: BTreeMap<String, Refs> = BTreeMap::new();
    let mut add = |ids: Vec<String>, kind: Kind, path: &str| {
        for id in ids {
            refs.entry(id)
                .or_default()
                .bucket(kind)
                .insert(path.to_string());
        }
    };
    // Code and tests.
    let mut files = Vec::new();
    for dir in ["crates", "plugins", "xtask"] {
        collect_files(&root.join(dir), &[], &mut files);
    }
    for path in files {
        let rel_path = rel(root, &path);
        // This module's own examples are not evidence.
        if rel_path == "xtask/src/spec_coverage.rs" {
            continue;
        }
        let is_rs = rel_path.ends_with(".rs");
        let test_path = is_test_path(&rel_path);
        // Other files only count when they are test data (fixtures, scenarios).
        if !is_rs && !test_path {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let region = if is_rs && !test_path {
            test_region_start(&text)
        } else {
            None
        };
        for tok in scan_text(&text) {
            let kind = if test_path || region.is_some_and(|r| tok.line >= r) {
                Kind::Test
            } else {
                Kind::Code
            };
            let ids = index.resolve(&tok, &rel_path, problems);
            add(ids, kind, &rel_path);
        }
    }
    // Parity scenes, scenarios and evidence.
    let mut files = Vec::new();
    collect_files(
        &root.join("docs/parity"),
        &["json", "md", "toml", "txt"],
        &mut files,
    );
    for path in files {
        let rel_path = rel(root, &path);
        if rel_path.ends_with("/dashboard.md") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for tok in scan_text(&text) {
            let ids = index.resolve(&tok, &rel_path, problems);
            add(ids, Kind::Scenario, &rel_path);
        }
    }
    refs
}

// ---------------------------------------------------------------------------------------------
// Overrides
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct MappingFile {
    #[serde(default, rename = "override")]
    overrides: Vec<Override>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Override {
    /// IDs; `A-B-001..004` and `A-B-001..A-B-004` expand to the defined IDs in between.
    ids: Vec<String>,
    status: String,
    /// Paths of the implementing code (files or directories).
    code: Vec<String>,
    /// Paths of the tests that exercise it.
    test: Vec<String>,
    #[serde(default)]
    note: String,
}

#[derive(Debug)]
struct Credit {
    status: Status,
    code: Vec<String>,
    test: Vec<String>,
    note: String,
}

fn load_overrides(
    root: &Path,
    index: &Index,
    problems: &mut Vec<String>,
) -> BTreeMap<String, Credit> {
    let mut credits = BTreeMap::new();
    let path = root.join(MAPPING_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return credits;
    };
    let file: MappingFile = match toml::from_str(&text) {
        Ok(f) => f,
        Err(e) => {
            problems.push(format!("{MAPPING_FILE}: {e}"));
            return credits;
        }
    };
    for (n, ov) in file.overrides.iter().enumerate() {
        let label = format!(
            "{MAPPING_FILE}: override #{} ({})",
            n + 1,
            ov.ids.first().map_or("", String::as_str)
        );
        let Some(status) = Status::parse(&ov.status) else {
            problems.push(format!(
                "{label}: status must be `implemented` or `partial`, got `{}`",
                ov.status
            ));
            continue;
        };
        if ov.code.is_empty() || ov.test.is_empty() {
            problems.push(format!(
                "{label}: needs at least one `code` path and one `test` path"
            ));
        }
        for p in ov.code.iter().chain(&ov.test) {
            if !root.join(p).exists() {
                problems.push(format!("{label}: path `{p}` does not exist"));
            }
        }
        let mut ids = Vec::new();
        for spec in &ov.ids {
            match expand_id_spec(spec, index) {
                Ok(v) => ids.extend(v),
                Err(e) => problems.push(format!("{label}: {e}")),
            }
        }
        for id in ids {
            let credit = Credit {
                status,
                code: ov.code.clone(),
                test: ov.test.clone(),
                note: ov.note.clone(),
            };
            if credits.insert(id.clone(), credit).is_some() {
                problems.push(format!(
                    "{label}: `{id}` is credited by more than one override"
                ));
            }
        }
    }
    credits
}

fn expand_id_spec(spec: &str, index: &Index) -> Result<Vec<String>, String> {
    let spec = spec.trim();
    let Some((a, b)) = spec.split_once("..") else {
        return if index.ids.contains_key(spec) {
            Ok(vec![spec.to_string()])
        } else {
            Err(format!("unknown requirement ID `{spec}`"))
        };
    };
    let (area, lo, _) = parse_id(a).ok_or_else(|| format!("bad ID `{a}`"))?;
    let hi = match parse_id(b) {
        Some((_, hi, _)) => hi,
        None => b
            .parse::<u32>()
            .map_err(|_| format!("bad range end `{b}`"))?,
    };
    if hi < lo {
        return Err(format!("empty range `{spec}`"));
    }
    let ids: Vec<String> = index
        .ids
        .values()
        .filter(|r| r.area == area && (lo..=hi).contains(&r.num))
        .map(|r| r.id.clone())
        .collect();
    if ids.is_empty() {
        Err(format!("range `{spec}` matches no requirement"))
    } else {
        Ok(ids)
    }
}

// ---------------------------------------------------------------------------------------------
// Evaluation and report
// ---------------------------------------------------------------------------------------------

struct Evaluated<'a> {
    req: &'a Requirement,
    status: Status,
    /// How the status was reached, for the report.
    evidence: String,
}

fn evaluate<'a>(
    index: &'a Index,
    refs: &BTreeMap<String, Refs>,
    credits: &BTreeMap<String, Credit>,
) -> Vec<Evaluated<'a>> {
    index
        .ids
        .values()
        .map(|req| {
            if let Some(c) = credits.get(&req.id) {
                return Evaluated {
                    req,
                    status: c.status,
                    evidence: format!("override: {}", c.note),
                };
            }
            match refs.get(&req.id) {
                Some(r) if !r.code.is_empty() && (!r.test.is_empty() || !r.scenario.is_empty()) => {
                    Evaluated {
                        req,
                        status: Status::Implemented,
                        evidence: "code + test".into(),
                    }
                }
                Some(_) => Evaluated {
                    req,
                    status: Status::Partial,
                    evidence: "referenced".into(),
                },
                None => Evaluated {
                    req,
                    status: Status::Missing,
                    evidence: String::new(),
                },
            }
        })
        .collect()
}

#[derive(Default, Clone, Copy)]
struct Tally {
    implemented: u32,
    partial: u32,
    missing: u32,
}

impl Tally {
    fn add(&mut self, s: Status) {
        match s {
            Status::Implemented => self.implemented += 1,
            Status::Partial => self.partial += 1,
            Status::Missing => self.missing += 1,
        }
    }

    fn total(self) -> u32 {
        self.implemented + self.partial + self.missing
    }

    fn pct(self) -> String {
        let t = self.total();
        if t == 0 {
            return "-".into();
        }
        format!("{:.1}%", 100.0 * f64::from(self.implemented) / f64::from(t))
    }

    fn row(self, label: &str) -> String {
        format!(
            "| {label} | {} | {} | {} | {} | {} |\n",
            self.total(),
            self.implemented,
            self.partial,
            self.missing,
            self.pct()
        )
    }
}

const TABLE_HEAD: &str =
    "| | Total | Implemented | Partial | Missing | % implemented |\n|---|---|---|---|---|---|\n";

fn md_cell(text: &str) -> String {
    text.replace('|', "\\|")
}

fn render_report(evals: &[Evaluated], credits: &BTreeMap<String, Credit>) -> String {
    let mut out = String::new();
    out.push_str(
        "# Spec coverage\n\n\
Generated by `cargo run -p xtask -- spec-coverage`; do not edit. Feature PRs need not commit \
changes to this file, it is regenerated on main and attached to every CI run.\n\n\
How a requirement gets its status:\n\n\
- **implemented**: its ID is referenced from code and from at least one test, scenario or scene, \
or `docs/specs/coverage.toml` credits it with a code path and a test path;\n\
- **partial**: referenced only from code, only from a test or scenario, or credited `partial`;\n\
- **missing**: not referenced.\n\n\
`% implemented` counts only `implemented`; partial rows are not rounded up.\n\n",
    );
    let mut total = Tally::default();
    let mut by_set: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut by_pri: BTreeMap<Priority, Tally> = BTreeMap::new();
    let mut by_set_pri: BTreeMap<(&str, Priority), Tally> = BTreeMap::new();
    let mut by_file: BTreeMap<&str, [Tally; 3]> = BTreeMap::new();
    for e in evals {
        let set = SETS
            .iter()
            .find(|(s, _)| *s == e.req.set())
            .map_or("?", |(s, _)| *s);
        total.add(e.status);
        by_set.entry(set).or_default().add(e.status);
        by_pri.entry(e.req.priority).or_default().add(e.status);
        by_set_pri
            .entry((set, e.req.priority))
            .or_default()
            .add(e.status);
        by_file.entry(&e.req.file).or_default()[e.req.priority as usize].add(e.status);
    }
    out.push_str("## Totals\n\n");
    out.push_str(TABLE_HEAD);
    for (set, name) in SETS {
        out.push_str(&by_set.get(set).copied().unwrap_or_default().row(name));
    }
    out.push_str(&total.row("**All**"));
    out.push_str("\n## By priority\n\n");
    out.push_str(TABLE_HEAD);
    for pri in Priority::ALL {
        out.push_str(
            &by_pri
                .get(&pri)
                .copied()
                .unwrap_or_default()
                .row(pri.name()),
        );
    }
    out.push_str("\n## By spec set and priority\n\n");
    out.push_str(TABLE_HEAD);
    for (set, name) in SETS {
        for pri in Priority::ALL {
            out.push_str(
                &by_set_pri
                    .get(&(set, pri))
                    .copied()
                    .unwrap_or_default()
                    .row(&format!("{name} {}", pri.name())),
            );
        }
    }
    out.push_str("\n## By file\n\nCells are implemented / partial / total.\n\n");
    out.push_str("| File | MUST | SHOULD | LATER | All |\n|---|---|---|---|---|\n");
    for (file, tallies) in &by_file {
        let mut all = Tally::default();
        let mut cells = String::new();
        for t in tallies {
            all.implemented += t.implemented;
            all.partial += t.partial;
            all.missing += t.missing;
            let _ = write!(cells, " {}/{}/{} |", t.implemented, t.partial, t.total());
        }
        let _ = writeln!(
            out,
            "| `{}` |{cells} {}/{}/{} |",
            file.trim_start_matches("docs/specs/"),
            all.implemented,
            all.partial,
            all.total()
        );
    }
    let credited: Vec<&Evaluated> = evals
        .iter()
        .filter(|e| e.evidence.starts_with("override"))
        .collect();
    let _ = write!(
        out,
        "\n{} requirements are credited by overrides in `docs/specs/coverage.toml`, {} by scanned references.\n",
        credited.len(),
        evals.iter().filter(|e| e.status != Status::Missing).count() - credited.len()
    );
    out.push_str("\n## Credited by override\n\n");
    if credits.is_empty() {
        out.push_str("None.\n");
    } else {
        out.push_str("| ID | Status | Code | Test | Note |\n|---|---|---|---|---|\n");
        let paths = |v: &[String]| {
            v.iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        for (id, c) in credits {
            let status = if c.status == Status::Implemented {
                "implemented"
            } else {
                "partial"
            };
            let _ = writeln!(
                out,
                "| {id} | {status} | {} | {} | {} |",
                paths(&c.code),
                paths(&c.test),
                md_cell(&c.note)
            );
        }
    }
    out.push_str("\n## Missing MUST requirements\n");
    let mut current = "";
    let mut any = false;
    for e in evals {
        if e.status != Status::Missing || e.req.priority != Priority::Must {
            continue;
        }
        any = true;
        if e.req.file != current {
            current = &e.req.file;
            let _ = write!(
                out,
                "\n### `{}`\n\n| ID | Requirement | Acceptance |\n|---|---|---|\n",
                current.trim_start_matches("docs/specs/")
            );
        }
        let _ = writeln!(
            out,
            "| {} | {} | {} |",
            e.req.id,
            md_cell(&e.req.title),
            md_cell(&e.req.accept)
        );
    }
    if !any {
        out.push_str("\nNone.\n");
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------------------------

pub fn run(root: &Path, args: impl Iterator<Item = String>) -> Result<()> {
    let mut check = false;
    let mut out_path: Option<PathBuf> = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--out" => out_path = Some(args.next().context("--out needs a file")?.into()),
            other => {
                bail!("unknown argument `{other}`; usage: spec-coverage [--check] [--out FILE]")
            }
        }
    }
    let mut problems = Vec::new();

    let mut spec_files = Vec::new();
    collect_files(&root.join(SPECS_DIR), &["md"], &mut spec_files);
    let mut reqs = Vec::new();
    for path in &spec_files {
        let rel_path = rel(root, path);
        if rel_path.ends_with("README.md") || rel_path == REPORT_FILE {
            continue;
        }
        let text = fs::read_to_string(path).with_context(|| format!("read {rel_path}"))?;
        parse_spec(&rel_path, &text, &mut reqs);
    }
    if reqs.is_empty() {
        bail!("no requirements found under {SPECS_DIR}");
    }
    let mut ids: BTreeMap<String, Requirement> = BTreeMap::new();
    for req in reqs {
        if let Some(prev) = ids.get(&req.id) {
            problems.push(format!(
                "{}:{}: duplicate requirement ID `{}` (first defined at {}:{})",
                req.file, req.line, req.id, prev.file, prev.line
            ));
        } else {
            ids.insert(req.id.clone(), req);
        }
    }
    let areas = ids.values().map(|r| r.area.clone()).collect();
    let index = Index { ids, areas };

    let refs = scan_references(root, &index, &mut problems);
    let credits = load_overrides(root, &index, &mut problems);
    let evals = evaluate(&index, &refs, &credits);
    let report = render_report(&evals, &credits);

    let mut tally = Tally::default();
    for e in &evals {
        tally.add(e.status);
    }
    println!(
        "{} requirements: {} implemented, {} partial, {} missing ({})",
        tally.total(),
        tally.implemented,
        tally.partial,
        tally.missing,
        tally.pct()
    );
    problems.sort();
    problems.dedup();
    for p in &problems {
        eprintln!("error: {p}");
    }
    if !problems.is_empty() {
        bail!("spec coverage found {} problem(s)", problems.len());
    }
    if check {
        println!("spec IDs, references and overrides valid");
        return Ok(());
    }
    let path = out_path.unwrap_or_else(|| root.join(REPORT_FILE));
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&path, report).with_context(|| format!("write {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Vec<Requirement> {
        let mut out = Vec::new();
        parse_spec("docs/specs/java/x.md", text, &mut out);
        out
    }

    #[test]
    fn parses_table_rows_by_header() {
        let r = parse(
            "| ID | Pri | Requirement | Acceptance |\n|---|---|---|---|\n| PLAT-DUMB-001 | MUST | One state. | Fresh project is dumb. |\n",
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "PLAT-DUMB-001");
        assert_eq!(r[0].priority, Priority::Must);
        assert_eq!(r[0].accept, "Fresh project is dumb.");
    }

    #[test]
    fn parses_bullets_and_bare_lines() {
        let r = parse(
            "- **JAVA-IMP-001** `MUST` Report it once.\n  *Accept:* markup range.\n- **JAVA-SPR-010** Bean model.\n\n`CI-DMN-001` SHOULD: registry\nis an extension point. Acceptance: it loads.\n\nprose\n",
        );
        let ids: Vec<_> = r.iter().map(|r| (r.id.as_str(), r.priority)).collect();
        assert_eq!(
            ids,
            [
                ("JAVA-IMP-001", Priority::Must),
                ("JAVA-SPR-010", Priority::Later),
                ("CI-DMN-001", Priority::Should)
            ]
        );
        assert_eq!(r[0].accept, "markup range.");
        assert_eq!(r[2].accept, "it loads.");
    }

    #[test]
    fn scans_ids_ranges_lists_and_test_names() {
        let toks =
            scan_text("// JAVA-IMP-070..076 and JAVA-ERR-012 / 144, plat_psi_051_x, NOCI-DMN-001");
        assert_eq!(toks.len(), 3);
        assert_eq!(toks[0].last, Some(76));
        assert_eq!(toks[1].extra, vec![144]);
        assert!(toks[2].lowercase);
        assert_eq!(toks[2].area, "PLAT-PSI");
    }

    #[test]
    fn unknown_ids_are_problems_but_legacy_areas_are_not() {
        let mut ids = BTreeMap::new();
        let mut reqs = Vec::new();
        parse_spec(
            "docs/specs/java/x.md",
            "- **JAVA-IMP-001** `MUST` a\n",
            &mut reqs,
        );
        for r in reqs {
            ids.insert(r.id.clone(), r);
        }
        let index = Index {
            areas: ["JAVA-IMP".to_string()].into(),
            ids,
        };
        let mut problems = Vec::new();
        for tok in scan_text("JAVA-IMP-001 JAVA-IMP-002 JAVA-NAV-011 JAVA-IPM-001 ci_zzz_001") {
            index.resolve(&tok, "f.rs", &mut problems);
        }
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    #[test]
    fn test_regions_and_paths() {
        assert!(is_test_path("crates/a/tests/x.rs"));
        assert!(!is_test_path("crates/a/src/x.rs"));
        assert_eq!(test_region_start("a\n#[cfg(test)]\nmod t {}"), Some(2));
    }
}
