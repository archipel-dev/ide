#![allow(dead_code, unused_imports)]
//! The executable Java acceptance cases of `docs/specs/java/cases/*.cases.md`, engine layer
//! (no `IdeState`, hermetic: libraries are our own fixture jars, built by
//! `tools/java-fixtures/build.sh`).
//!
//! Same shape as `crates/archipel-ide-core/tests/editor_cases.rs`: every `###` block is parsed
//! (the parser is shared with the editor cases), run and compared with its `after` block;
//! the test fails when fewer cases pass than [`PASS_BASELINE`] (raise it when cases start to
//! pass, never lower it). `JAVA_CASES_ONLY=JAVA-CMP` runs the cases whose id starts with the text,
//! `JAVA_CASES_VERBOSE=1` prints every failing case, `JAVA_CASES_REPORT=1` (set by
//! `cargo run -p xtask -- java-cases`) writes `docs/parity/java-cases.md`.
//!
//! Supported keys: `complete [count N]` and `complete, choose "<row>"` (see the header of
//! `06-completion.cases.md`), `inspect` for the error cases of the `02`, `04` and `05` spec files
//! (the native resolve highlights of the file, compared with the `problem:` lines) and
//! `Alt+Enter, choose "Import class"[, choose "<fqn>"]` (the native import fix). The catalog
//! inspections of the `09` files and multi-module cases are reported as not runnable until
//! their engine exists.

mod common;

use archipel_ide_java::complete::{self, CompletionInput};
use archipel_ide_java::diagnostics::{JavaSettings, apply_command, collect_detailed};
use archipel_ide_java::import_edit::ImportSettings;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Cases that must keep passing (never lower; raise when more pass).
const PASS_BASELINE: usize = 82;

include!("support/case_format.rs");

fn java_cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/specs/java/cases")
}

#[derive(Debug, Clone)]
enum Outcome {
    Pass,
    Fail(String),
    NotRunnable(&'static str),
}

struct Row {
    case: Case,
    outcome: Outcome,
}

/// One file of a case: project-relative path and text (caret markup removed).
struct CaseFile {
    path: String,
    marked: Marked,
}

/// Splits the `before` text at `--- path` lines. Without such a line the whole text is one file.
fn case_files(before: &str) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = Vec::new();
    for line in before.lines() {
        if let Some(path) = line.strip_prefix("--- ") {
            files.push((path.trim().to_string(), String::new()));
        } else if let Some((_, text)) = files.last_mut() {
            text.push_str(line);
            text.push('\n');
        } else {
            files.push(("src/main/java/Main.java".to_string(), format!("{line}\n")));
        }
    }
    for (_, text) in &mut files {
        while text.ends_with("\n\n") {
            text.pop();
        }
    }
    files
}

/// Library fixture sets by the names of `settings: library=...`; the bool is test scope.
fn library_fixture(name: &str) -> Option<(&'static str, bool)> {
    match name {
        "junit5" => Some(("junit5", true)),
        "junit4" => Some(("junit4", true)),
        "hamcrest" => Some(("hamcrest", true)),
        "jetbrains-annotations" => Some(("annotations", false)),
        _ => None,
    }
}

/// Spec files whose `inspect` and `Alt+Enter` cases the native engine answers (the error catalog,
/// resolve and import rules); the `09` inspection catalog is another engine.
fn engine_covers(case: &Case) -> bool {
    case.id.starts_with("JAVA-ERR")
        || case.id.starts_with("JAVA-RES")
        || case.id.starts_with("JAVA-IMP")
}

fn run_case(case: &Case) -> Outcome {
    if case.language != "java" {
        return Outcome::NotRunnable("not a Java case");
    }
    let keys = case.keys.trim();
    if keys.starts_with("inspect") && engine_covers(case) {
        return run_inspect(case);
    }
    if keys.starts_with("Alt+Enter") && engine_covers(case) {
        return run_alt_enter(case);
    }
    if !keys.starts_with("complete") {
        return if keys.starts_with("inspect") {
            Outcome::NotRunnable("inspections (no native catalog yet)")
        } else {
            Outcome::NotRunnable("quick fixes (Alt+Enter) are not run by the engine layer yet")
        };
    }
    let Some(before) = &case.before else {
        return Outcome::Fail("no before block".into());
    };
    if before.starts_with("modules:") {
        return Outcome::NotRunnable("multi-module cases");
    }
    let mut libs: Vec<(
        String,
        std::path::PathBuf,
        archipel_ide_projectmodel::DependencyScope,
    )> = Vec::new();
    for (key, value) in &case.settings {
        if key == "library" {
            for name in value.split(',').map(str::trim) {
                let Some((set, test)) = library_fixture(name) else {
                    return Outcome::NotRunnable("library without a fixture jar");
                };
                libs.push((
                    name.to_string(),
                    common::fixture_jar(set),
                    if test {
                        archipel_ide_projectmodel::DependencyScope::Test
                    } else {
                        archipel_ide_projectmodel::DependencyScope::Compile
                    },
                ));
            }
        }
    }
    let files = case_files(before);
    if files.is_empty() {
        return Outcome::Fail("empty before block".into());
    }
    let parsed: Vec<CaseFile> = files
        .iter()
        .map(|(path, text)| CaseFile {
            path: path.clone(),
            marked: parse_marked(text),
        })
        .collect();
    let caret_file = parsed
        .iter()
        .find(|f| !f.marked.selections.is_empty())
        .unwrap_or(&parsed[0]);
    let refs: Vec<(&str, &str)> = parsed
        .iter()
        .map(|f| (f.path.as_str(), f.marked.text.as_str()))
        .collect();
    let libs_ref: Vec<(
        &str,
        std::path::PathBuf,
        archipel_ide_projectmodel::DependencyScope,
    )> = libs
        .iter()
        .map(|(n, p, s)| (n.as_str(), p.clone(), *s))
        .collect();
    let proj = common::project(&refs, &libs_ref);
    let Some(&(_, caret_char)) = caret_file.marked.selections.first() else {
        return Outcome::Fail("no <caret> in the before block".into());
    };
    let text = &caret_file.marked.text;
    let offset = text
        .char_indices()
        .nth(caret_char)
        .map_or(text.len(), |(b, _)| b);
    // `complete [count N]`
    let mut count = 1u32;
    let first = keys.split(',').next().unwrap_or("").trim();
    if let Some(rest) = first.strip_prefix("complete")
        && let Some(n) = rest
            .trim()
            .strip_prefix("[count ")
            .and_then(|r| r.strip_suffix(']'))
    {
        count = n.trim().parse().unwrap_or(1);
    }
    let settings = ImportSettings::default();
    let path = proj.path(&caret_file.path);
    let input = CompletionInput {
        project: &proj.project,
        settings: &settings,
        path: &path,
        text,
        offset,
        count,
    };
    let completion = match complete::complete(&input) {
        Ok(c) => c,
        Err(e) => return Outcome::Fail(format!("{e}")),
    };
    let Some(expected) = &case.after else {
        return Outcome::Fail("no after block".into());
    };
    let rows: Vec<String> = completion
        .as_ref()
        .map(|c| c.items.iter().map(|i| i.label()).collect())
        .unwrap_or_default();
    if let Some(choose) = keys.split_once("choose ").map(|(_, r)| unescape(r.trim())) {
        let Some(c) = &completion else {
            return Outcome::Fail("nothing to choose from".into());
        };
        let Some(item) = c.items.iter().find(|i| i.label() == choose) else {
            return Outcome::Fail(format!("no row `{choose}` in {rows:?}"));
        };
        let ins = match complete::insertion(&input, &item.key) {
            Ok(Some(i)) => i,
            Ok(None) => return Outcome::Fail("the class is gone from the index".into()),
            Err(e) => return Outcome::Fail(format!("{e}")),
        };
        // Apply the prefix replacement and the import edit to the original text.
        let mut edits: Vec<(usize, usize, String)> = ins
            .edits
            .iter()
            .map(|e| {
                (
                    e.range.start as usize,
                    e.range.end as usize,
                    e.new_text.clone(),
                )
            })
            .collect();
        // A snippet marks the caret with `$0` (inside the parentheses of a call).
        let (ins_text, caret_in_text) = if ins.snippet {
            let raw = ins.text.replace("\\$", "\u{1}");
            match raw.find("$0") {
                Some(i) => (
                    format!("{}{}", &raw[..i], &raw[i + 2..]).replace('\u{1}', "$"),
                    Some(raw[..i].replace('\u{1}', "$").len()),
                ),
                None => (ins.text.clone(), None),
            }
        } else {
            (ins.text.clone(), None)
        };
        edits.push((c.prefix_start, offset, ins_text.clone()));
        edits.sort_by_key(|(s, e, _)| (*s, *e));
        let mut out = text.clone();
        for (s, e, t) in edits.iter().rev() {
            out.replace_range(*s..*e, t);
        }
        // The caret follows the inserted name; edits before it shift it.
        let mut shift: isize = 0;
        for (s, e, t) in &edits {
            if *e <= c.prefix_start && (*s, *e) != (c.prefix_start, offset) {
                shift += t.len() as isize - (*e - *s) as isize;
            }
        }
        let caret =
            (c.prefix_start as isize + shift) as usize + caret_in_text.unwrap_or(ins_text.len());
        let actual_marked = {
            let mut o = out.clone();
            o.insert_str(caret, "<caret>");
            o
        };
        let want = expected.trim_end();
        let want_text = parse_marked(want).text;
        if out.trim_end() != want_text.trim_end() || !actual_marked.trim_end().eq(want) {
            return Outcome::Fail(format!("expected:\n{want}\nactual:\n{actual_marked}"));
        }
        return Outcome::Pass;
    }
    let info: Vec<RowInfo> = completion
        .as_ref()
        .map(|c| {
            c.items
                .iter()
                .map(|i| RowInfo {
                    label: i.label(),
                    name: i.name.clone(),
                    signature: i.signature.clone(),
                    ty: i.ty.clone(),
                    deprecated: i.deprecated,
                })
                .collect()
        })
        .unwrap_or_default();
    judge_list_with(expected, &rows, &info)
}

struct Setup {
    proj: common::Proj,
    files: Vec<CaseFile>,
}

/// The project of a case: its files and the fixture libraries of `settings: library=...`.
fn setup(case: &Case) -> Result<Setup, Outcome> {
    let Some(before) = &case.before else {
        return Err(Outcome::Fail("no before block".into()));
    };
    if before.starts_with("modules:") {
        return Err(Outcome::NotRunnable("multi-module cases"));
    }
    let mut libs: Vec<(
        String,
        std::path::PathBuf,
        archipel_ide_projectmodel::DependencyScope,
    )> = Vec::new();
    for (key, value) in &case.settings {
        if key == "library" {
            for name in value.split(',').map(str::trim) {
                let Some((set, test)) = library_fixture(name) else {
                    return Err(Outcome::NotRunnable("library without a fixture jar"));
                };
                libs.push((
                    name.to_string(),
                    common::fixture_jar(set),
                    if test {
                        archipel_ide_projectmodel::DependencyScope::Test
                    } else {
                        archipel_ide_projectmodel::DependencyScope::Compile
                    },
                ));
            }
        }
    }
    let files: Vec<CaseFile> = case_files(before)
        .iter()
        .map(|(path, text)| CaseFile {
            path: path.clone(),
            marked: parse_marked(text),
        })
        .collect();
    if files.is_empty() {
        return Err(Outcome::Fail("empty before block".into()));
    }
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|f| (f.path.as_str(), f.marked.text.as_str()))
        .collect();
    let libs_ref: Vec<(
        &str,
        std::path::PathBuf,
        archipel_ide_projectmodel::DependencyScope,
    )> = libs
        .iter()
        .map(|(n, p, s)| (n.as_str(), p.clone(), *s))
        .collect();
    let proj = common::project(&refs, &libs_ref);
    Ok(Setup { proj, files })
}

fn level_name(s: archipel_ide_analysis::Severity) -> &'static str {
    if s >= archipel_ide_analysis::Severity::ERROR {
        "error"
    } else if s >= archipel_ide_analysis::Severity::WARNING {
        "warning"
    } else {
        "weak warning"
    }
}

/// `keys: inspect`: the highlights of the caret file (the first one without a caret), as the
/// daemon produces them for the unsaved text, against the `problem:` lines of `after`.
fn run_inspect(case: &Case) -> Outcome {
    let Some(expected) = &case.after else {
        return Outcome::Fail("no after block".into());
    };
    let wanted: Vec<&str> = expected
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let runnable = wanted.iter().all(|l| {
        *l == "no problem reported"
            || (l.starts_with("problem: error ") && l.contains("\"Cannot resolve"))
    });
    if !runnable {
        return Outcome::NotRunnable("inspections other than the resolve errors");
    }
    let setup = match setup(case) {
        Ok(s) => s,
        Err(o) => return o,
    };
    let file = setup
        .files
        .iter()
        .find(|f| !f.marked.selections.is_empty())
        .unwrap_or(&setup.files[0]);
    let text = &file.marked.text;
    let tree = archipel_ide_java::source::parse(text).expect("parse");
    let collected = collect_detailed(
        &setup.proj.project,
        &JavaSettings::default(),
        &setup.proj.path(&file.path),
        text,
        &tree,
    );
    let mut infos = collected.infos;
    infos.sort_by_key(|i| (i.range.start, i.range.end));
    let actual: Vec<String> = infos
        .iter()
        .map(|i| {
            format!(
                "problem: {} `{}` \"{}\"",
                level_name(i.severity),
                &text[i.range.start as usize..i.range.end as usize],
                i.message.as_deref().unwrap_or("")
            )
        })
        .collect();
    let want: Vec<String> = wanted
        .iter()
        .filter(|l| **l != "no problem reported")
        .map(|l| (*l).to_string())
        .collect();
    if actual == want {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("expected {want:?}\nactual   {actual:?}"))
    }
}

/// `Alt+Enter, choose "Import class"[, choose "<fqn>"]`.
fn run_alt_enter(case: &Case) -> Outcome {
    let Some(expected) = &case.after else {
        return Outcome::Fail("no after block".into());
    };
    let choices: Vec<String> = case
        .keys
        .split("choose ")
        .skip(1)
        .map(|r| unescape(r.trim().trim_end_matches(',').trim()))
        .collect();
    let negative = expected.trim().starts_with("no quick fix");
    if !negative && choices.first().map(String::as_str) != Some("Import class") {
        return Outcome::NotRunnable("quick fixes other than Import class");
    }
    let setup = match setup(case) {
        Ok(s) => s,
        Err(o) => return o,
    };
    let Some(file) = setup.files.iter().find(|f| !f.marked.selections.is_empty()) else {
        return Outcome::Fail("no <caret> in the before block".into());
    };
    let text = &file.marked.text;
    let caret_char = file.marked.selections[0].0;
    let offset = text
        .char_indices()
        .nth(caret_char)
        .map_or(text.len(), |(b, _)| b) as u32;
    let tree = archipel_ide_java::source::parse(text).expect("parse");
    let collected = collect_detailed(
        &setup.proj.project,
        &JavaSettings::default(),
        &setup.proj.path(&file.path),
        text,
        &tree,
    );
    let fix = collected
        .class_fixes
        .iter()
        .find(|(r, _)| r.start <= offset && offset <= r.end)
        .map(|(_, f)| f.clone());
    if negative {
        return match fix {
            None => Outcome::Pass,
            Some(f) => Outcome::Fail(format!(
                "`Import class` is offered: {:?}",
                f.candidates
                    .iter()
                    .map(|c| c.fqn().to_string())
                    .collect::<Vec<_>>()
            )),
        };
    }
    let Some(fix) = fix else {
        return Outcome::Fail("no `Import class` at the caret".into());
    };
    let index = match choices.get(1) {
        Some(fqn) => match fix.candidates.iter().position(|c| c.fqn() == fqn) {
            Some(i) => i,
            None => {
                return Outcome::Fail(format!(
                    "no row `{fqn}` in {:?}",
                    fix.candidates
                        .iter()
                        .map(|c| c.fqn().to_string())
                        .collect::<Vec<_>>()
                ));
            }
        },
        None => 0,
    };
    let command = fix.apply(
        archipel_ide_analysis::FileId(1),
        archipel_ide_analysis::FileRev(1),
        text,
        index,
    );
    let Some(out) = apply_command(&command, archipel_ide_analysis::FileId(1), text) else {
        return Outcome::Fail("the fix changes nothing".into());
    };
    let want = parse_marked(expected).text;
    if out.trim_end() == want.trim_end() {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("expected:\n{want}\nactual:\n{out}"))
    }
}

/// The popup rows with their details, for the `tail:`, `type:` and `struck:` assertions.
struct RowInfo {
    label: String,
    name: String,
    signature: String,
    ty: String,
    deprecated: bool,
}

fn judge_list(expected: &str, rows: &[String]) -> Outcome {
    judge_list_with(expected, rows, &[])
}

fn judge_list_with(expected: &str, rows: &[String], info: &[RowInfo]) -> Outcome {
    let find = |name: &str| {
        info.iter()
            .find(|r| r.label == name.trim())
            .or_else(|| info.iter().find(|r| r.name == name.trim()))
    };
    let has = |label: &str| {
        rows.iter()
            .any(|r| r == label || r.split(" (").next() == Some(label))
    };
    for line in expected.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(label) = line.strip_prefix("not item: ") {
            if has(label.trim()) {
                return Outcome::Fail(format!("`{label}` is offered: {rows:?}"));
            }
        } else if let Some(label) = line.strip_prefix("item: ") {
            if !rows.iter().any(|r| r == label.trim()) {
                return Outcome::Fail(format!("`{label}` is not offered: {rows:?}"));
            }
        } else if let Some(pair) = line.strip_prefix("rank: ") {
            let Some((a, b)) = pair.split_once(" < ") else {
                return Outcome::Fail(format!("bad rank line `{line}`"));
            };
            let pos = |l: &str| rows.iter().position(|r| r == l.trim());
            match (pos(a), pos(b)) {
                (Some(x), Some(y)) if x < y => {}
                (x, y) => {
                    return Outcome::Fail(format!(
                        "`{a}` ({x:?}) is not ranked above `{b}` ({y:?}): {rows:?}"
                    ));
                }
            }
        } else if let Some(label) = line.strip_prefix("once: ") {
            let n = rows.iter().filter(|r| r.as_str() == label.trim()).count();
            if n != 1 {
                return Outcome::Fail(format!("`{label}` is offered {n} times: {rows:?}"));
            }
        } else if let Some(rest) = line.strip_prefix("tail: ") {
            // `tail: <name> = <parameter list>`
            let Some((name, want)) = rest.split_once(" = ") else {
                return Outcome::Fail(format!("bad tail line `{line}`"));
            };
            match find(name) {
                Some(r) if r.signature == want.trim() => {}
                Some(r) => {
                    return Outcome::Fail(format!(
                        "`{name}` has tail `{}`, not `{}`",
                        r.signature,
                        want.trim()
                    ));
                }
                None => return Outcome::Fail(format!("`{name}` is not offered: {rows:?}")),
            }
        } else if let Some(rest) = line.strip_prefix("type: ") {
            // `type: <name> = <type>`
            let Some((name, want)) = rest.split_once(" = ") else {
                return Outcome::Fail(format!("bad type line `{line}`"));
            };
            match find(name) {
                Some(r) if r.ty == want.trim() => {}
                Some(r) => {
                    return Outcome::Fail(format!(
                        "`{name}` has type `{}`, not `{}`",
                        r.ty,
                        want.trim()
                    ));
                }
                None => return Outcome::Fail(format!("`{name}` is not offered: {rows:?}")),
            }
        } else if let Some(name) = line.strip_prefix("struck: ") {
            match find(name) {
                Some(r) if r.deprecated => {}
                Some(_) => return Outcome::Fail(format!("`{name}` is not struck through")),
                None => return Outcome::Fail(format!("`{name}` is not offered: {rows:?}")),
            }
        } else if let Some(name) = line.strip_prefix("not struck: ") {
            match find(name) {
                Some(r) if !r.deprecated => {}
                Some(_) => return Outcome::Fail(format!("`{name}` is struck through")),
                None => return Outcome::Fail(format!("`{name}` is not offered: {rows:?}")),
            }
        } else if line == "no items" {
            if !rows.is_empty() {
                return Outcome::Fail(format!("expected no items: {rows:?}"));
            }
        } else {
            return Outcome::Fail(format!("unknown assertion `{line}`"));
        }
    }
    Outcome::Pass
}

fn short(reason: &str) -> String {
    let first = reason.lines().next().unwrap_or_default();
    let first: String = first.chars().take(110).collect();
    first.replace('|', "\\|")
}

fn report(rows: &[Row]) -> String {
    let mut files: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
    for row in rows {
        let e = files.entry(row.case.file.as_str()).or_default();
        match row.outcome {
            Outcome::Pass => e[0] += 1,
            Outcome::Fail(_) => e[1] += 1,
            Outcome::NotRunnable(_) => e[2] += 1,
        }
    }
    let total = |i: usize| files.values().map(|v| v[i]).sum::<usize>();
    let mut out = String::new();
    out.push_str("# Java acceptance cases\n\n");
    out.push_str(
        "Generated by `cargo run -p xtask -- java-cases` (the test `crates/archipel-ide-java/tests/java_cases.rs`)\n\
         from `docs/specs/java/cases/*.cases.md`. Do not edit by hand.\n\n",
    );
    let _ = writeln!(
        out,
        "**{} pass, {} fail, {} not runnable yet** of {} cases.\n",
        total(0),
        total(1),
        total(2),
        rows.len()
    );
    out.push_str("| Spec file | Pass | Fail | Not runnable | Total |\n|---|---:|---:|---:|---:|\n");
    for (file, v) in &files {
        let _ = writeln!(
            out,
            "| {file} | {} | {} | {} | {} |",
            v[0],
            v[1],
            v[2],
            v[0] + v[1] + v[2]
        );
    }
    let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows {
        if let Outcome::NotRunnable(reason) = row.outcome {
            *reasons.entry(reason).or_default() += 1;
        }
    }
    if !reasons.is_empty() {
        out.push_str("\nNot runnable yet:\n\n");
        for (reason, count) in &reasons {
            let _ = writeln!(out, "- {reason}: {count}");
        }
    }
    let mut groups: BTreeMap<String, [usize; 3]> = BTreeMap::new();
    for row in rows {
        let prefix: Vec<&str> = row.case.id.split('-').take(2).collect();
        let e = groups.entry(prefix.join("-")).or_default();
        match row.outcome {
            Outcome::Pass => e[0] += 1,
            Outcome::Fail(_) => e[1] += 1,
            Outcome::NotRunnable(_) => e[2] += 1,
        }
    }
    out.push_str("\n## By requirement group\n\n| Group | Pass | Fail | Not runnable |\n|---|---:|---:|---:|\n");
    for (group, v) in &groups {
        let _ = writeln!(out, "| {group} | {} | {} | {} |", v[0], v[1], v[2]);
    }
    // Only the files that have runnable cases get a per-case table (the 1528 inspection cases
    // would drown it).
    let mut current = "";
    for row in rows {
        if matches!(row.outcome, Outcome::NotRunnable(_)) {
            continue;
        }
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
            Outcome::NotRunnable(reason) => ("not runnable", (*reason).to_owned()),
        };
        let _ = writeln!(out, "| {} | {} | {} |", row.case.name(), status, note);
    }
    out
}

#[test]
fn java_cases_do_not_regress() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(java_cases_dir())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".cases.md"))
        .collect();
    paths.sort();
    let only = std::env::var("JAVA_CASES_ONLY").ok();
    let verbose = std::env::var("JAVA_CASES_VERBOSE").is_ok();
    let mut rows = Vec::new();
    for path in &paths {
        for case in parse_file(path) {
            if only
                .as_ref()
                .is_some_and(|prefixes| !prefixes.split(',').any(|p| case.id.starts_with(p)))
            {
                continue;
            }
            let outcome =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_case(&case)))
                    .unwrap_or_else(|p| {
                        let message = p
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                            .unwrap_or_default();
                        Outcome::Fail(format!("panic: {message}"))
                    });
            if verbose && let Outcome::Fail(reason) = &outcome {
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
            rows.len() >= 1500,
            "cases were dropped: parsed {}",
            rows.len()
        );
        if std::env::var("JAVA_CASES_REPORT").is_ok() {
            let target =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/parity/java-cases.md");
            std::fs::write(target, report(&rows)).unwrap();
        }
        assert!(
            passing >= PASS_BASELINE,
            "only {passing} Java cases pass, the baseline is {PASS_BASELINE}; run \
             `JAVA_CASES_VERBOSE=1 cargo test -p archipel-ide-java --test java_cases` to see why"
        );
        if passing > PASS_BASELINE {
            eprintln!("{passing} Java cases pass: raise PASS_BASELINE from {PASS_BASELINE}");
        }
    } else {
        eprintln!("{passing} of {} selected cases pass", rows.len());
    }
}
