//! The UI renders; it does not reach into other subsystems' state.
//!
//! Phase 5 of the platform refactor (`docs/architecture/platform-refactor-plan.md`) moves every
//! read of `state.<field>` in `archipel-ide-ui` behind a `DataContext` key, a service accessor
//! or an action (`docs/architecture/platform-kernel.md`, "DataContext and actions"). This test
//! counts the reads that are left and fails when the count rises above
//! [`SUBSYSTEM_READ_BASELINE`], so the UI can only get cleaner.
//!
//! A second ratchet counts the calls of the *transitional handles* (`state.ai()`,
//! `state.agents_mut()`, ...: the `handles!` list in `archipel-ide-core/src/views.rs`). A handle
//! hands out a whole subsystem, so a call is a read that has not yet become a `DataContext` key,
//! a narrower accessor or a service. [`HANDLE_BASELINE`] is the work that is left.
//!
//! - Removed reads? Lower the baseline to the new count (the failure message after your change
//!   says so).
//! - Need data from a subsystem? Add a `DataKey` and answer it in
//!   `archipel_ide_core::ide_actions::IdeDataProvider`, or give the subsystem an accessor or a
//!   service, instead of reading `state.<field>`.
//!
//! What counts: `state.<field>` where `<field>` is a `pub` field of `IdeState` (read from
//! `archipel-ide-core/src/lib.rs`, so the list cannot go stale), written `state.`, `app.state.` or
//! `self.state.`. Method calls such as `state.settings()` are accessors and do not count.
//! An inline `#[cfg(test)] mod x {` and everything after it, and files declared by `#[cfg(test)] mod x;`, are test code and are not counted.
//! Comments are skipped. Method chains broken over several lines count as one line.

// A baseline may reach zero, which makes "at least the baseline" trivially true.
#![allow(clippy::absurd_extreme_comparisons)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The number of direct reads the UI may still have. Lower it when reads go; never raise it.
///
/// It was 0 while the counter stopped at the first `#[cfg(test)]` of a file (`src/lib.rs` has one
/// at line 96, so nearly all of it was skipped) and could not see `app\n    .state\n    .field`
/// broken over lines. Counting everything, 86 reads were there all along, most of them in
/// `src/lib.rs` and `src/toolwindows.rs`.
const SUBSYSTEM_READ_BASELINE: usize = 86;

/// The number of calls of whole-subsystem handles the UI may still make. Lower it when calls go
/// (a call becomes a `DataContext` key, a narrower accessor or a service call); never raise it.
const HANDLE_BASELINE: usize = 112;

/// The `pub` fields of `pub struct IdeState`.
fn ide_state_fields() -> Vec<String> {
    let source = include_str!("../../archipel-ide-core/src/lib.rs");
    let start = source
        .find("pub struct IdeState {")
        .expect("core lib.rs declares `pub struct IdeState`");
    let body = &source[start..];
    let end = body.find("\n}").expect("the struct ends with a brace");
    body[..end]
        .lines()
        .skip(1)
        .filter_map(|line| {
            let field = line.strip_prefix("    pub ")?;
            let (name, _) = field.split_once(':')?;
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                .then(|| name.to_owned())
        })
        .collect()
}

/// The names of the transitional handles: the `name, name_mut: Type;` entries of the `handles!`
/// block in the core crate's `views.rs`.
fn handle_names() -> Vec<String> {
    let source = include_str!("../../archipel-ide-core/src/views.rs");
    let start = source
        .find("handles! {")
        .expect("views.rs has a handles! block");
    let body = &source[start..];
    let end = body.find("\n}").expect("the block ends");
    body[..end]
        .lines()
        .filter_map(|line| {
            let (names, _) = line.strip_prefix("    ")?.split_once(": ")?;
            let (shared, exclusive) = names.split_once(", ")?;
            (exclusive == format!("{shared}_mut")).then(|| shared.to_owned())
        })
        .collect()
}

fn source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("src is readable") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            source_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The fields read as `state.<field>` in one line (not method calls).
fn reads_in_line<'a>(line: &'a str, fields: &'a [String]) -> Vec<&'a str> {
    let code = line.split("//").next().unwrap_or(line);
    let mut found = Vec::new();
    let mut rest = code;
    let mut offset = 0;
    while let Some(at) = rest.find("state.") {
        let start = offset + at;
        let before = code[..start].chars().next_back();
        let after = &code[start + "state.".len()..];
        offset = start + "state.".len();
        rest = &code[offset..];
        if before.is_some_and(is_ident) {
            continue; // `last_state.`, `ide_state.`
        }
        let name: String = after.chars().take_while(|c| is_ident(*c)).collect();
        let is_call = after[name.len()..].starts_with('(');
        if !is_call && let Some(field) = fields.iter().find(|f| **f == name) {
            found.push(field.as_str());
        }
    }
    found
}

/// How often `state.<handle>()` or `state.<handle>_mut()` is called in one line.
fn handle_calls_in_line(line: &str, handles: &[String]) -> usize {
    let code = line.split("//").next().unwrap_or(line);
    let mut calls = 0;
    let mut rest = code;
    let mut offset = 0;
    while let Some(at) = rest.find("state.") {
        let start = offset + at;
        let before = code[..start].chars().next_back();
        let after = &code[start + "state.".len()..];
        offset = start + "state.".len();
        rest = &code[offset..];
        if before.is_some_and(is_ident) {
            continue;
        }
        let name: String = after.chars().take_while(|c| is_ident(*c)).collect();
        let is_call = after[name.len()..].starts_with('(');
        let base = name.strip_suffix("_mut").unwrap_or(&name);
        if is_call && handles.iter().any(|handle| handle == base) {
            calls += 1;
        }
    }
    calls
}

/// The code of a source file the ratchets look at: everything before an inline test module
/// (`#[cfg(test)]` followed by `mod name {`), with method chains that rustfmt broke over several
/// lines (`app\n    .state\n    .android_tooling()`) joined into one line, so a read cannot hide
/// behind a line break.
fn production_code(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<String> = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            let next = lines.get(at + 1).map_or("", |next| next.trim_start());
            if next.starts_with("mod ") && next.trim_end().ends_with('{') {
                break;
            }
        }
        if trimmed.starts_with('.')
            && !trimmed.starts_with("..")
            && let Some(previous) = out.last_mut()
        {
            previous.push_str(trimmed);
            continue;
        }
        out.push((*line).to_owned());
    }
    out
}

/// Files that only exist for tests: `#[cfg(test)] mod name;` declares `name.rs` (or `name/`).
fn test_only_files(files: &[PathBuf]) -> Vec<PathBuf> {
    let mut names = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(file).expect("source is readable");
        let lines: Vec<&str> = text.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            if line.trim() == "#[cfg(test)]"
                && let Some(next) = lines.get(at + 1)
                && let Some(name) = next
                    .trim()
                    .strip_prefix("mod ")
                    .and_then(|rest| rest.strip_suffix(';'))
            {
                names.push(name.trim().to_owned());
            }
        }
    }
    files
        .iter()
        .filter(|file| {
            file.components().any(|c| {
                let part = c.as_os_str().to_string_lossy();
                names
                    .iter()
                    .any(|name| *part == **name || *part == format!("{name}.rs"))
            })
        })
        .cloned()
        .collect()
}

/// Reads per field and file.
fn count_reads() -> (usize, BTreeMap<String, usize>, BTreeMap<String, usize>) {
    let fields = ide_state_fields();
    let mut files = Vec::new();
    source_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let mut total = 0;
    let mut per_field = BTreeMap::new();
    let mut per_file = BTreeMap::new();
    let test_only = test_only_files(&files);
    for file in files.iter().filter(|file| !test_only.contains(file)) {
        let text = std::fs::read_to_string(file).expect("source is readable");
        let name = file
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(file)
            .display()
            .to_string();
        for line in production_code(&text) {
            for field in reads_in_line(&line, &fields) {
                total += 1;
                *per_field.entry(field.to_owned()).or_default() += 1;
                *per_file.entry(name.clone()).or_default() += 1;
            }
        }
    }
    (total, per_field, per_file)
}

#[test]
fn the_reader_finds_fields_and_counts_only_reads() {
    let fields = ide_state_fields();
    for known in ["documents", "lsp", "status", "project"] {
        assert!(fields.iter().any(|f| f == known), "{known} in {fields:?}");
    }
    // `pub(crate)` fields are not reachable from the UI crate.
    assert!(!fields.iter().any(|f| f == "platform"));
    assert_eq!(
        reads_in_line("let a = app.state.documents.len();", &fields),
        ["documents"]
    );
    assert_eq!(
        reads_in_line(
            "self.state.lsp.is_busy() && state.status.is_empty()",
            &fields
        ),
        ["lsp", "status"]
    );
    assert!(reads_in_line("app.state.settings().read()", &fields).is_empty());
    assert!(reads_in_line("let last_state.documents = 1;", &fields).is_empty());
    assert!(reads_in_line("// state.documents is commented out", &fields).is_empty());
}

#[test]
fn the_ui_only_reads_less_of_other_subsystems() {
    let (total, per_field, per_file) = count_reads();
    let mut fields: Vec<_> = per_field.iter().collect();
    fields.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    let mut files: Vec<_> = per_file.iter().collect();
    files.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    assert!(
        total <= SUBSYSTEM_READ_BASELINE,
        "archipel-ide-ui reads `state.<field>` {total} times, the baseline is \
         {SUBSYSTEM_READ_BASELINE}. Do not add reads of other subsystems' state: ask for a \
         DataContext key (archipel_ide_core::ide_actions), a service accessor or an action \
         (docs/architecture/platform-kernel.md, \"DataContext and actions\"). By field: \
         {fields:?}. By file (top 10): {:?}",
        &files[..files.len().min(10)]
    );
    assert!(
        total >= SUBSYSTEM_READ_BASELINE,
        "archipel-ide-ui reads `state.<field>` {total} times, fewer than the baseline \
         {SUBSYSTEM_READ_BASELINE}: lower SUBSYSTEM_READ_BASELINE in \
         tests/no_subsystem_reads.rs to {total} so it cannot grow back."
    );
}

fn count_handle_calls() -> usize {
    let handles = handle_names();
    let mut files = Vec::new();
    source_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let test_only = test_only_files(&files);
    files
        .iter()
        .filter(|file| !test_only.contains(file))
        .map(|file| {
            production_code(&std::fs::read_to_string(file).expect("source is readable"))
                .iter()
                .map(|line| handle_calls_in_line(line, &handles))
                .sum::<usize>()
        })
        .sum()
}

#[test]
fn the_handle_reader_finds_handles_and_counts_calls() {
    let handles = handle_names();
    for known in ["ai", "lsp", "tool_windows"] {
        assert!(handles.iter().any(|h| h == known), "{known} in {handles:?}");
    }
    assert_eq!(handle_calls_in_line("app.state.lsp().servers", &handles), 1);
    assert_eq!(
        handle_calls_in_line("app.state.lsp_mut().x(); state.ai().y()", &handles),
        2
    );
    assert_eq!(handle_calls_in_line("app.state.status()", &handles), 0);
    assert_eq!(handle_calls_in_line("app.lsp().x", &handles), 0);
}

#[test]
fn whole_subsystem_handles_are_only_used_less() {
    let calls = count_handle_calls();
    assert!(
        calls <= HANDLE_BASELINE,
        "archipel-ide-ui calls whole-subsystem handles {calls} times, the baseline is \
         {HANDLE_BASELINE}. Prefer a DataContext key (archipel_ide_core::ide_actions) or a \
         narrow accessor over handing the whole subsystem to the UI."
    );
    assert!(
        calls >= HANDLE_BASELINE,
        "archipel-ide-ui calls whole-subsystem handles {calls} times, fewer than the baseline \
         {HANDLE_BASELINE}: lower HANDLE_BASELINE in tests/no_subsystem_reads.rs to {calls}."
    );
}
